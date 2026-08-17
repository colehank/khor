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
        return Err(msg::TUNNEL_TARGET_TOO_LONG.into());
    }
    send.write_all(&(bytes.len() as u16).to_be_bytes())
        .await
        .map_err(|e| msg::tunnel_bad_handshake(e))?;
    send.write_all(bytes).await.map_err(|e| msg::tunnel_bad_handshake(e))?;
    Ok(())
}

/// Reads the target header on the exit side. An oversized length is
/// refused before the bytes are read, so a lie about the length cannot
/// make us buffer past the cap.
async fn read_target(recv: &mut RecvStream) -> Result<String, String> {
    let mut len = [0u8; 2];
    recv.read_exact(&mut len).await.map_err(|e| msg::tunnel_bad_handshake(e))?;
    let len = u16::from_be_bytes(len) as usize;
    if len > TARGET_MAX {
        return Err(msg::TUNNEL_TARGET_TOO_LONG.into());
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await.map_err(|e| msg::tunnel_bad_handshake(e))?;
    String::from_utf8(buf).map_err(|e| msg::tunnel_bad_handshake(e))
}

/// A held tunnel to one exit: the endpoint that dialed it and the live
/// connection. Streams are opened from `conn` via [`dial`] and borrow it,
/// so a caller keeps the `Borrow` alive for as long as any stream runs.
/// One `Borrow` is one lease — many streams share it (docs/NET.md 出口
/// 是一层：一台出口一个 lease，多消费者共享).
pub struct Borrow {
    _ep: iroh::Endpoint,
    conn: Connection,
}

impl Borrow {
    pub fn new(ep: iroh::Endpoint, conn: Connection) -> Self {
        Borrow { _ep: ep, conn }
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
pub async fn dial(
    conn: &Connection,
    target: &str,
) -> Result<(SendStream, RecvStream), String> {
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    write_target(&mut send, target).await?;
    let mut status = [0u8; 1];
    recv.read_exact(&mut status).await.map_err(|e| msg::tunnel_bad_handshake(e))?;
    match status[0] {
        OK => Ok((send, recv)),
        REFUSED => Err(msg::NOT_PAIRED.into()),
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
/// stream, and propagates each half-close: when the dialer stops sending,
/// the target sees EOF (we `shutdown` its write side); when the target
/// stops, the dialer's read side is finished. Neither direction waits on
/// the other — a one-way stream (a long download) must not stall because
/// the request direction went quiet.
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
