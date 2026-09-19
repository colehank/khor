//! Borrowing a machine's network (docs/NET.md 借网): a paired peer opens
//! a byte pipe through this machine to some `host:port` this machine can
//! reach. One exit is one layer, and a layer does not know its consumers
//! — the same tunnel carries a browser window's bytes and a session's
//! bytes alike (docs/NET.md 出口是一层).
//!
//! **Why a second ALPN and not another `Request` variant.** The main
//! `khor/0` protocol is strict request-reply: the server reads a whole
//! frame to FIN, answers, closes the stream. A tunnel is the opposite —
//! a long-lived duplex stream that never sees FIN until the far side
//! hangs up — so it cannot live inside a `read_to_end` handler. It gets
//! its own ALPN ([`endpoint::TUNNEL_ALPN`]); the old road is untouched.
//!
//! **The per-stream handshake.** A dialer opens a bi stream and writes a
//! length-prefixed target, and the exit answers with one status byte
//! before any payload flows. The byte is not decoration: without it a
//! refusal is a bare connection reset, which reads exactly like a dead
//! endpoint — and the ledger's control-group rule needs the refusal to
//! be legible as *the gate*, distinct from "the machine is gone". So a
//! not-paired dial gets [`REFUSED`] and an unreachable target gets
//! [`NO_ROUTE`], each a fact the caller can name.

use khor_catalog::msg;
use tokio::io::AsyncWriteExt;

use iroh::endpoint::{Connection, RecvStream, SendStream};

/// A dialer that asks for more than this in one target header is
/// malformed, not ambitious — a real `host:port` is well under it, and
/// the cap keeps a hostile peer from making us buffer.
const TARGET_MAX: usize = 256;

/// The exit's one-byte answer, sent before any payload.
pub const OK: u8 = 0;
/// The dialer is not in this machine's device table (docs/NET.md 入网).
pub const REFUSED: u8 = 1;
/// The target is well-formed but the exit could not reach it.
pub const NO_ROUTE: u8 = 2;

/// How long the exit waits on the TCP connect to the target before
/// calling it unreachable. Long enough for a real far host, short
/// enough that a black hole does not pin the stream.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Writes the target header a dialer sends first: a `u16` length then the
/// `host:port` bytes.
async fn write_target(send: &mut SendStream, target: &str) -> Result<(), String> {
    let bytes = target.as_bytes();
    if bytes.len() > TARGET_MAX {
        return Err(msg::tunnel_target_too_long().into());
    }
    send.write_all(&(bytes.len() as u16).to_be_bytes())
        .await
        .map_err(msg::tunnel_bad_handshake)?;
    send.write_all(bytes)
        .await
        .map_err(msg::tunnel_bad_handshake)?;
    Ok(())
}

/// Reads the target header on the exit side. An oversized length is
/// refused before the bytes are read, so a lie about the length cannot
/// make us buffer past the cap.
async fn read_target(recv: &mut RecvStream) -> Result<String, String> {
    let mut len = [0u8; 2];
    recv.read_exact(&mut len)
        .await
        .map_err(msg::tunnel_bad_handshake)?;
    let len = u16::from_be_bytes(len) as usize;
    if len > TARGET_MAX {
        return Err(msg::tunnel_target_too_long().into());
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .map_err(msg::tunnel_bad_handshake)?;
    String::from_utf8(buf).map_err(msg::tunnel_bad_handshake)
}

/// A held tunnel to one exit: the live connection, and — for a one-shot
/// caller — the endpoint it dialed from. Streams are opened from `conn`
/// via [`dial`] and borrow it, so a caller keeps the `Borrow` alive for
/// as long as any stream runs. One `Borrow` is one lease — many streams
/// share it (docs/NET.md 出口是一层：一台出口一个 lease，多消费者共享).
///
/// The endpoint is `Option` because who owns it differs by caller: a
/// one-shot CLI/test dials a fresh endpoint the `Borrow` must keep alive;
/// the resident serve dials on its own long-lived endpoint, which outlives
/// any borrow and must not be owned here.
pub struct Borrow {
    _ep: Option<iroh::Endpoint>,
    conn: Connection,
}

impl Borrow {
    /// A borrow that owns the endpoint it dialed from (CLI/test).
    pub fn new(ep: iroh::Endpoint, conn: Connection) -> Self {
        Borrow {
            _ep: Some(ep),
            conn,
        }
    }

    /// A borrow dialed on an endpoint someone else keeps alive (the
    /// resident serve's).
    pub fn on_shared(conn: Connection) -> Self {
        Borrow { _ep: None, conn }
    }

    /// Opens one pipe through this lease to `target`.
    pub async fn open(&self, target: &str) -> Result<(SendStream, RecvStream), String> {
        dial(&self.conn, target).await
    }
}

/// The dialer half: connect the tunnel ALPN, ask for `target`, and return
/// the stream halves once the exit says [`OK`]. A refusal or an
/// unreachable target comes back as an `Err` naming which — never as a
/// bare stream that then mysteriously carries nothing.
///
/// The caller owns `ep`; the returned streams borrow the connection, so
/// the caller keeps `conn` alive for as long as it splices bytes.
pub async fn dial(conn: &Connection, target: &str) -> Result<(SendStream, RecvStream), String> {
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    write_target(&mut send, target).await?;
    let mut status = [0u8; 1];
    recv.read_exact(&mut status)
        .await
        .map_err(msg::tunnel_bad_handshake)?;
    match status[0] {
        OK => Ok((send, recv)),
        REFUSED => Err(msg::not_paired().into()),
        NO_ROUTE => Err(msg::tunnel_no_route(target)),
        other => Err(msg::tunnel_bad_status(other)),
    }
}

/// The exit half of one accepted bi stream: read the target, gate on
/// pairing, connect, answer with the status byte, then splice bytes both
/// ways until either side hangs up. `paired` is asked per stream so the
/// verdict rides the stream that carries it.
pub async fn serve_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    paired: bool,
) -> Result<(), String> {
    let target = read_target(&mut recv).await?;
    if !paired {
        let _ = send.write_all(&[REFUSED]).await;
        let _ = send.finish();
        return Ok(());
    }
    let tcp = match tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::TcpStream::connect(&target))
        .await
    {
        Ok(Ok(tcp)) => tcp,
        // Both "connect errored" and "connect timed out" are the same
        // fact to the dialer: the exit's network could not reach there.
        _ => {
            let _ = send.write_all(&[NO_ROUTE]).await;
            let _ = send.finish();
            return Ok(());
        }
    };
    send.write_all(&[OK]).await.map_err(|e| e.to_string())?;
    splice(send, recv, tcp).await;
    Ok(())
}

/// Copies bytes both ways between the QUIC stream halves and the TCP
/// stream, and propagates each half-close: when the QUIC peer stops
/// sending, the TCP side sees EOF (we `shutdown` its write side); when
/// the TCP side stops, the QUIC send is finished. Neither direction waits
/// on the other — a one-way stream (a long download) must not stall
/// because the request direction went quiet.
///
/// The same code serves both ends: `recv` always drains to `tcp` and
/// `tcp` always feeds `send`, whether `tcp` is the target (exit side) or
/// the browser (proxy side) — the roles are symmetric.
async fn splice(mut send: SendStream, mut recv: RecvStream, tcp: tokio::net::TcpStream) {
    let (mut tr, mut tw) = tcp.into_split();
    let up = async {
        let _ = tokio::io::copy(&mut recv, &mut tw).await;
        let _ = tw.shutdown().await;
    };
    let down = async {
        let _ = tokio::io::copy(&mut tr, &mut send).await;
        let _ = send.finish();
    };
    tokio::join!(up, down);
}

/// The largest request head we will buffer before giving up. A real one
/// is a request line plus a few short headers; anything past this is a
/// client that will never send the blank line, not a big ask. It is also
/// about what an ordinary origin server accepts, so a head this proxy
/// refuses is one the server behind it would have refused too.
const HEAD_MAX: usize = 8 * 1024;

/// The local HTTP proxy half of a borrow (docs/NET.md 借网): one lease
/// behind a proxy port. A browser (or an `HTTPS_PROXY`/`HTTP_PROXY`
/// honouring tool) points here; every connection it makes becomes a
/// stream through the exit to the host it named. Runs until the listener
/// is dropped.
///
/// **Two shapes, and they are not equally blind.**
///
/// `CONNECT host:port` is a byte pipe: the client speaks its own protocol
/// through it (in practice TLS) and khor never sees inside. That is what
/// an https page uses and what a walled site needs.
///
/// An absolute-form request (`GET http://host/path HTTP/1.1`) is what a
/// **plain-http** page sends a proxy, and it cannot be blind — the target
/// is inside the request, so the proxy has to read the head to know where
/// to send it, and it rewrites that head into the origin form a server
/// expects. Campus services and the self-hosted things people actually
/// borrow a network to reach (a notebook, a dashboard, a router) are
/// largely plain http, so refusing this shape refused the main case.
///
/// **What that costs is worth saying out loud rather than discovering:**
/// for an `http://` page khor sees the request head in the clear. There
/// is no version of a plain-http proxy that does not — the alternative is
/// not privacy, it is the page not loading. `https` keeps the old
/// property untouched, because it keeps going through CONNECT.
pub async fn serve_proxy(
    borrow: std::sync::Arc<Borrow>,
    listener: tokio::net::TcpListener,
    activity: Activity,
) -> Result<(), String> {
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    loop {
        let Ok((sock, _)) = listener.accept().await else {
            return Ok(());
        };
        let borrow = borrow.clone();
        let count = count.clone();
        let activity = activity.clone();
        tokio::spawn(async move {
            let _ = proxy_one(&borrow, sock, count, activity).await;
        });
    }
}

/// A local port that pipes to **one** fixed target on the far machine.
///
/// The borrow proxy speaks HTTP CONNECT because a browser picks a fresh
/// destination on every request. A terminal has exactly one for the life
/// of the pipe — the session's host socket — so there is nothing to
/// parse and nothing to choose, and putting a CONNECT parser in front of
/// it would mean the attach client had to speak proxy.
///
/// Same tunnel, same exit, same status byte: what changes is only who
/// decides the target, and here it is decided once by the caller.
pub async fn serve_fixed(
    borrow: std::sync::Arc<Borrow>,
    listener: tokio::net::TcpListener,
    target: String,
) -> Result<(), String> {
    loop {
        let Ok((sock, _)) = listener.accept().await else {
            return Ok(());
        };
        let borrow = borrow.clone();
        let target = target.clone();
        tokio::spawn(async move {
            match borrow.open(&target).await {
                Ok((send, recv)) => splice(send, recv, sock).await,
                // Nothing can be *said* to a plain TCP client — there is
                // no status byte in the terminal protocol — so dropping
                // is all there is. It is logged because the difference
                // between "the exit refused" and "the target is gone"
                // is invisible from the other side, and one of them is
                // a bug.
                Err(why) => eprintln!("{}", msg::tunnel_pipe_failed(&target, why)),
            }
        });
    }
}

/// Called as the lease crosses between having pipes and having none, so a
/// borrow row can read 忙碌 while bytes flow and 空闲 while the port waits
/// (docs/SESSION.md 六词). `true` = now busy, `false` = now idle.
pub type Activity = std::sync::Arc<dyn Fn(bool) + Send + Sync>;

/// Increments the live-pipe count on creation and decrements on drop,
/// firing `activity` only at the 0↔1 boundary so a busy lease is not
/// rewritten on every stream. Held across a splice, so the count is
/// exactly the number of pipes actually carrying bytes.
struct PipeGuard {
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    activity: Activity,
}

impl PipeGuard {
    fn open(count: std::sync::Arc<std::sync::atomic::AtomicUsize>, activity: Activity) -> Self {
        use std::sync::atomic::Ordering::SeqCst;
        if count.fetch_add(1, SeqCst) == 0 {
            (activity)(true);
        }
        PipeGuard { count, activity }
    }
}

impl Drop for PipeGuard {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering::SeqCst;
        if self.count.fetch_sub(1, SeqCst) == 1 {
            (self.activity)(false);
        }
    }
}

/// What one client connection is asking the proxy for.
#[derive(Debug, PartialEq)]
enum ProxyAsk {
    /// `CONNECT host:port`: a byte pipe. The client speaks its own
    /// protocol through it and nothing here reads a byte of it.
    Tunnel { target: String },
    /// An absolute-form request (`GET http://host/path HTTP/1.1`): where
    /// to send it, and the head rewritten into the origin form a server
    /// expects. Plain http only — see [`absolute_http`].
    Relay { target: String, head: Vec<u8> },
}

/// One browser connection: read its request head, open the tunnel, and
/// splice. What goes out first differs by shape — the proxy's own status
/// line for a CONNECT, the rewritten request for a relay.
///
/// **A relay gets no status line from the proxy.** The client asked for a
/// page, and the answer to that is the origin's response; a `200` written
/// here would be this proxy answering a question only the far server can
/// answer, and the real response would arrive behind it as garbage. A
/// CONNECT is the opposite: there `200` means "the pipe is up", which is
/// a fact the proxy is the only one holding.
async fn proxy_one(
    borrow: &Borrow,
    mut sock: tokio::net::TcpStream,
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    activity: Activity,
) -> Result<(), String> {
    use tokio::io::AsyncReadExt;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    // Read only the request head (to the blank line); everything after it
    // — tunnelled bytes, or a request body — belongs to `splice`, not to
    // this parse.
    loop {
        let n = sock.read(&mut byte).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
        if head.len() > HEAD_MAX {
            return Ok(());
        }
    }
    let Some(ask) = read_ask(&head) else {
        let _ = sock
            .write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")
            .await;
        return Ok(());
    };
    let target = match &ask {
        ProxyAsk::Tunnel { target } | ProxyAsk::Relay { target, .. } => target.clone(),
    };
    match borrow.open(&target).await {
        Ok((mut send, recv)) => {
            match ask {
                ProxyAsk::Tunnel { .. } => {
                    sock.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                        .await
                        .map_err(|e| e.to_string())?;
                }
                ProxyAsk::Relay { head, .. } => {
                    send.write_all(&head).await.map_err(|e| e.to_string())?;
                }
            }
            let _pipe = PipeGuard::open(count, activity);
            splice(send, recv, sock).await;
            Ok(())
        }
        Err(_) => {
            let _ = sock.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            Ok(())
        }
    }
}

/// Reads a request head into what it is asking for, or `None` for
/// anything this proxy will not route — which it answers with `405`
/// rather than guessing.
///
/// **An origin-form request (`GET / HTTP/1.1`) is one of those**, and
/// deliberately: it names a path and no host, so there is nothing to
/// route it to. Somebody has pointed a plain client at this port as if it
/// were the web server itself.
fn read_ask(head: &[u8]) -> Option<ProxyAsk> {
    let text = std::str::from_utf8(head).ok()?;
    let (first, rest) = text.split_once("\r\n")?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?;
    let asked = parts.next()?;
    let version = parts.next().unwrap_or("HTTP/1.1");
    if method.eq_ignore_ascii_case("CONNECT") {
        // A CONNECT authority is always host:port; a bare host would leave
        // the exit's TCP connect without a port and is not a thing
        // browsers send.
        return asked.contains(':').then(|| ProxyAsk::Tunnel {
            target: asked.to_owned(),
        });
    }
    let (target, host, path) = absolute_http(asked)?;
    Some(ProxyAsk::Relay {
        head: origin_form(method, &path, version, &host, rest),
        target,
    })
}

/// Splits an absolute `http` URI into the target to dial (`host:port`,
/// port defaulted to 80), the authority as written (what a missing `Host`
/// header should say), and the path to ask the origin for.
///
/// **`http` only.** A client sending `https://…` in absolute form is
/// asking the proxy to do TLS on its behalf, which this one does not do;
/// answering would send the request in the clear at a port expecting a
/// handshake. The way to an https page through here is CONNECT, which is
/// what every browser already does.
///
/// **Userinfo is refused rather than forwarded.** `http://user:pw@host/`
/// carries a credential, and passing it on would make this proxy a thing
/// that copies credentials into somebody else's connection.
fn absolute_http(uri: &str) -> Option<(String, String, String)> {
    let (scheme, rest) = uri.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") {
        return None;
    }
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    // An IPv6 literal is full of colons that are not the port separator,
    // so the port is only the one after the closing bracket.
    let has_port = match authority.rfind(']') {
        Some(close) => authority[close + 1..].starts_with(':'),
        None => authority.contains(':'),
    };
    let target = if has_port {
        authority.to_owned()
    } else {
        format!("{authority}:80")
    };
    Some((target, authority.to_owned(), path.to_owned()))
}

/// Rewrites an absolute-form head into origin form: the request line
/// carries the path alone, and the headers meant for this proxy are
/// dropped rather than passed along.
///
/// **The dropped ones are hop-by-hop**: they are addressed to whoever
/// handles this connection, and forwarding them puts this proxy's own
/// connection handling into somebody else's. `Proxy-Authorization` is
/// dropped for the reason userinfo is refused — it is a credential for
/// the hop, not for the server.
///
/// **`Connection: close`, and that is a real limit, not a formality.**
/// Keep-alive would let the client send a second request for a *different*
/// host down a socket already committed to one tunnel stream, and this
/// proxy would send it to the first host. Closing after one response makes
/// the client open another connection instead — which costs one more
/// stream on a QUIC connection that is already up, and is the honest
/// version of the limit rather than a wrong answer.
fn origin_form(method: &str, path: &str, version: &str, host: &str, rest: &str) -> Vec<u8> {
    let mut out = format!("{method} {path} {version}\r\n");
    let mut had_host = false;
    for line in rest.split("\r\n") {
        if line.is_empty() {
            continue;
        }
        let name = line.split(':').next().unwrap_or("");
        if name.eq_ignore_ascii_case("proxy-connection")
            || name.eq_ignore_ascii_case("proxy-authorization")
            || name.eq_ignore_ascii_case("connection")
            || name.eq_ignore_ascii_case("keep-alive")
        {
            continue;
        }
        if name.eq_ignore_ascii_case("host") {
            had_host = true;
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    if !had_host {
        // HTTP/1.0 clients need not send one and every modern server wants
        // it; the authority in the absolute URI is exactly the answer.
        out.push_str("Host: ");
        out.push_str(host);
        out.push_str("\r\n");
    }
    out.push_str("Connection: close\r\n\r\n");
    out.into_bytes()
}
