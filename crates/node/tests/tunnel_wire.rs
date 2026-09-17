//! Borrowing a machine's network over a real wire (docs/NET.md 借网,
//! 账本: 网络类改动必须真连,还要有对照组). One serve owns alpha's key.
//! A plain TCP echo server stands in for "a site alpha's network can
//! reach". beta, paired, tunnels through alpha to that echo server and
//! gets its own bytes back — proving the pipe carries payload to a real
//! target on alpha's side and returns it. The control half: an unpaired
//! key speaking the same tunnel ALPN is refused with the status byte,
//! against the very serve beta just tunnelled through — so the refusal
//! is the pairing gate, not a dead endpoint.

//!
//! # The clock here is issue #73's, not this file's subject
//!
//! Every deadline below is sized to swallow one full first-touch stall
//! (#73: 20-40s per process on this machine — the comment in
//! `wait_for_endpoint_file` has the mechanism). **Uniformly**, because
//! which call pays it is not deterministic: measured 2026-08-21, a bind
//! cost 21ms and a dial 1.1s while the first real sync cost 37s. The
//! runtime is multi-thread for the other half of #73: that stall is
//! *synchronous*, and on a current-thread runtime it freezes tokio's
//! clock, so every timeout in the file silently stretches with it and a
//! real hang stops looking like one.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use khor_node::proto::MAX_FRAME;
use khor_node::tunnel;
use khor_node::Node;
use tokio::time::timeout;

mod util;
use util::raw_tunnel;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-tun-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90s, not 10 (issue #73, sampled 2026-08-20; every real-connection
    // test in this directory timed out for it on 2026-08-21). On a Mac
    // operated over ssh, a freshly compiled test binary is a new face to
    // macOS: its first network-stack touch (interface enumeration,
    // SystemConfiguration) hangs **20-40s per process**, waiting on an
    // authorization prompt nobody can ever click.
    //
    // What the budgets in this file are distinguishing themselves from:
    // a real hang, which is unbounded. 90s against a measured 40s
    // ceiling is more than double, deliberately — a timeout that never
    // fires costs nothing when the test passes, and a flaky red costs a
    // person a trip to find out it was the machine.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// A TCP echo server: every byte in comes straight back out. Stands in
/// for a target the exit machine's network can reach. Returns its bound
/// address; the task lives until the test process ends.
async fn echo_server() -> String {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else { break };
            tokio::spawn(async move {
                let (mut r, mut w) = sock.into_split();
                let _ = tokio::io::copy(&mut r, &mut w).await;
            });
        }
    });
    addr
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_paired_machine_borrows_the_far_network_and_an_unpaired_key_is_refused() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let echo = echo_server().await;

    // The tunnel carries a payload bigger than one buffer, taken back
    // byte-identical — a single write-and-read cannot prove the splice
    // survives a stream that fills and drains.
    let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let borrow = timeout(Duration::from_secs(60), b.tunnel_to("alpha"))
        .await
        .expect("dialling the tunnel must not hang")
        .unwrap();
    let (mut send, mut recv) = timeout(Duration::from_secs(60), borrow.open(&echo))
        .await
        .expect("opening the pipe must not hang")
        .expect("a paired open to a reachable target must succeed");
    send.write_all(&payload).await.unwrap();
    send.finish().unwrap();
    let back = timeout(Duration::from_secs(60), recv.read_to_end(MAX_FRAME))
        .await
        .expect("reading the echo must not hang")
        .unwrap();
    assert_eq!(back, payload, "every byte, echoed through alpha's network");

    // A second pipe on the same lease reaches a dead port: the exit's
    // network answers, not ours, and the status byte says so.
    let err = timeout(Duration::from_secs(60), borrow.open("127.0.0.1:1"))
        .await
        .expect("must not hang")
        .unwrap_err();
    assert!(
        err.contains("127.0.0.1:1"),
        "the no-route refusal names the target: {err}"
    );

    // The control: an unpaired key, same tunnel ALPN, same wire. Proven
    // against the live serve beta just tunnelled through, so REFUSED here
    // is the pairing gate and not a dead endpoint.
    let alpha_info = b
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .expect("pairing put alpha in beta's table");
    let status = timeout(
        Duration::from_secs(60),
        raw_tunnel(iroh::SecretKey::generate(), &alpha_info.id, &alpha_info.addrs, &echo),
    )
    .await
    .expect("must not hang")
    .unwrap();
    assert_eq!(status, tunnel::REFUSED, "an unpaired tunnel dial must be refused");

    serve_a.abort();
    let _ = fs::remove_dir_all(&ra);
    let _ = fs::remove_dir_all(&rb);
}

/// Reads from a socket until `needle` appears, returning all bytes read.
async fn read_until(sock: &mut tokio::net::TcpStream, needle: &[u8]) -> Vec<u8> {
    use tokio::io::AsyncReadExt;
    let mut got = Vec::new();
    let mut byte = [0u8; 1];
    while sock.read(&mut byte).await.unwrap() == 1 {
        got.push(byte[0]);
        if got.ends_with(needle) {
            break;
        }
    }
    got
}

/// A plain-`http` origin: reads one request head per connection, hands it
/// back through the channel, and answers a fixed page. Stands in for the
/// notebook, dashboard or router somebody borrows a network to reach —
/// the things that are still `http://` and are the reason the proxy
/// learned this shape at all.
async fn http_server() -> (String, tokio::sync::mpsc::UnboundedReceiver<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { break };
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut head = Vec::new();
                let mut byte = [0u8; 1];
                while sock.read(&mut byte).await.unwrap_or(0) == 1 {
                    head.push(byte[0]);
                    if head.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let _ = tx.send(String::from_utf8_lossy(&head).into_owned());
                let _ = sock
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                    )
                    .await;
            });
        }
    });
    (addr, rx)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_local_proxy_connects_a_client_through_the_borrowed_network() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let ra = root("pa");
    let rb = root("pb");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let echo = echo_server().await;

    // beta stands up its local proxy in front of one lease to alpha.
    let borrow = timeout(Duration::from_secs(60), b.tunnel_to("alpha"))
        .await
        .expect("dialling the tunnel must not hang")
        .unwrap();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let idle: tunnel::Activity = std::sync::Arc::new(|_busy| {});
    tokio::spawn(async move {
        let _ = tunnel::serve_proxy(std::sync::Arc::new(borrow), listener, idle).await;
    });

    // A client speaks HTTP CONNECT to the proxy, then raw bytes to the
    // echo server through the established tunnel — exactly what a browser
    // does for an HTTPS site, minus the TLS the proxy never sees.
    let mut client = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
    client
        .write_all(format!("CONNECT {echo} HTTP/1.1\r\nHost: {echo}\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let status = read_until(&mut client, b"\r\n\r\n").await;
    assert!(
        String::from_utf8_lossy(&status).starts_with("HTTP/1.1 200"),
        "the proxy establishes the tunnel: {}",
        String::from_utf8_lossy(&status)
    );
    client.write_all(b"ping through the exit").await.unwrap();
    let mut back = [0u8; 21];
    timeout(Duration::from_secs(60), client.read_exact(&mut back))
        .await
        .expect("the echo must not hang")
        .unwrap();
    assert_eq!(&back, b"ping through the exit", "bytes made the round trip");

    // **Origin form is refused even now that plain http is relayed**, and
    // it is a different case rather than a leftover: `GET /` names a path
    // and no host, so there is nothing to route it to. Somebody has
    // pointed a plain client at this port as if it were the web server.
    let mut plain = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
    plain.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n").await.unwrap();
    let refused = read_until(&mut plain, b"\r\n\r\n").await;
    assert!(
        String::from_utf8_lossy(&refused).starts_with("HTTP/1.1 405"),
        "a non-CONNECT method is refused: {}",
        String::from_utf8_lossy(&refused)
    );

    // **The other shape: a plain-http page.** The client puts the whole
    // URL in the request line, so the proxy has to read the head to know
    // where to send it, and rewrites it into the origin form a server
    // expects. This is the case campus services and self-hosted things
    // actually are, and refusing it refused the main reason to borrow.
    let (site, mut asked) = http_server().await;
    let mut page = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
    page.write_all(
        format!(
            "GET http://{site}/wiki?q=1 HTTP/1.1\r\nHost: {site}\r\n\
             Proxy-Connection: keep-alive\r\nX-Mine: yes\r\n\r\n"
        )
        .as_bytes(),
    )
    .await
    .unwrap();
    let answer = timeout(Duration::from_secs(60), read_until(&mut page, b"\r\n\r\n"))
        .await
        .expect("the origin's answer must not hang");
    assert!(
        String::from_utf8_lossy(&answer).starts_with("HTTP/1.1 200 OK"),
        "the origin's own response comes back, not one the proxy invented: {}",
        String::from_utf8_lossy(&answer)
    );
    let mut body = [0u8; 5];
    timeout(Duration::from_secs(60), page.read_exact(&mut body))
        .await
        .expect("the body must not hang")
        .unwrap();
    assert_eq!(&body, b"hello", "and its body arrives through the exit");

    let got = timeout(Duration::from_secs(60), asked.recv())
        .await
        .expect("the origin must be reached")
        .expect("the origin must have read a head");
    assert!(
        got.starts_with("GET /wiki?q=1 HTTP/1.1\r\n"),
        "the request line arrives in origin form, path only: {got:?}"
    );
    assert!(got.contains("X-Mine: yes"), "the client's own headers are kept: {got:?}");
    assert!(
        !got.to_ascii_lowercase().contains("proxy-connection"),
        "a header addressed to the proxy is not forwarded to the server: {got:?}"
    );

    // **https in absolute form is refused, not relayed.** Sending it on
    // would put the request in the clear at a port waiting for a TLS
    // handshake; the way to an https page through here is CONNECT, which
    // is what every browser already does.
    let mut wrong = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
    wrong.write_all(b"GET https://example.invalid/ HTTP/1.1\r\nHost: x\r\n\r\n").await.unwrap();
    let refused_tls = read_until(&mut wrong, b"\r\n\r\n").await;
    assert!(
        String::from_utf8_lossy(&refused_tls).starts_with("HTTP/1.1 405"),
        "an absolute-form https request is refused: {}",
        String::from_utf8_lossy(&refused_tls)
    );

    serve_a.abort();
    let _ = fs::remove_dir_all(&ra);
    let _ = fs::remove_dir_all(&rb);
}

/// Opens a tunnel through a proxy and returns the still-open client
/// socket, so the caller can hold the pipe (to watch the row go 忙碌).
async fn connect_through(proxy: std::net::SocketAddr, target: &str) -> tokio::net::TcpStream {
    use tokio::io::AsyncWriteExt;
    let mut client = tokio::net::TcpStream::connect(proxy).await.unwrap();
    client
        .write_all(format!("CONNECT {target} HTTP/1.1\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let status = read_until(&mut client, b"\r\n\r\n").await;
    assert!(
        String::from_utf8_lossy(&status).starts_with("HTTP/1.1 200"),
        "proxy established: {}",
        String::from_utf8_lossy(&status)
    );
    client
}

/// Waits up to ~4s for **that** borrow's row to reach `want`, returning
/// whether it did. The activity callback writes the state synchronously
/// as pipes come and go, so this poll only rides out the tiny gap between
/// the wire event and the disk write — no system cycle to cover.
///
/// **By id, not by kind.** It used to take the first row whose kind was
/// "borrow", which was unambiguous while a borrow row was the only thing
/// a borrow could be. It is not any more: a page's borrow is deliberately
/// rowless, and a teardown that wrongly gave it a row made this helper
/// read that row instead — so a torn-down build failed here, on 忙碌,
/// about a borrow this line was never asking about.
async fn wait_borrow_state(node: &Node, id: &str, want: khor_core::State) -> bool {
    for _ in 0..40 {
        if let Ok(views) = node.sessions()
            && let Some(v) = views.iter().find(|v| v.session.id.0 == id)
                && v.session.state.state == want {
                    return true;
                }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_borrow_is_one_per_machine_and_only_the_hand_held_kind_wears_a_row() {
    let ra = root("sa");
    let rb = root("sb");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    let sb = Node::open_as(rb.clone(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&ra).await;
    wait_for_endpoint_file(&rb).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    // Routed through beta's serve (both hold their keys), like every verb.
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let echo = echo_server().await;

    // The borrow is hosted by beta's serve — b.borrow hands off, gets back
    // the session and the proxy address the serve bound.
    let (session, addr) = timeout(Duration::from_secs(60), b.borrow("alpha"))
        .await
        .expect("borrow must not hang")
        .unwrap();
    let proxy: std::net::SocketAddr = addr.parse().unwrap();

    // The row is there, and idle until something uses it.
    assert!(
        wait_borrow_state(&b, &session, khor_core::State::Idle).await,
        "a fresh borrow row opens 空闲"
    );

    // **One borrow per exit machine.** Asking again for a machine already
    // borrowed is answered with the borrow that is open — not a second
    // lease, a second port and a second row. Before this, every page
    // opened through a machine spent all three, so three pages left three
    // rows nobody could tell apart.
    let (again, again_addr) = timeout(Duration::from_secs(60), b.borrow("alpha"))
        .await
        .expect("a second borrow of a held machine must not hang")
        .unwrap();
    assert_eq!(again, session, "a second borrow of one machine is the first one");
    assert_eq!(again_addr, addr, "and it answers where the first one listens");
    assert_eq!(
        b.sessions().unwrap().iter().filter(|v| v.session.kind.0 == "borrow").count(),
        1,
        "so the session list gains no second row"
    );

    // **A page's borrow takes no row at all** (docs/NET.md 可达性,
    // 2026-08-17 定形: 一层不占会话列表). The tab that opened it is its
    // face and owns its lifetime, so a row of its own would be a session
    // in the list that nothing treats as a session. Only the hand-held
    // borrow above is one, because that one has neither.
    let (unlisted, unlisted_addr) = timeout(Duration::from_secs(60), b.borrow_unlisted("alpha"))
        .await
        .expect("an unlisted borrow must not hang")
        .unwrap();
    // The two kinds are not shared even on one machine, so closing the
    // row a person is looking at cannot pull the network out from under
    // a page. Both are alive right now, so the ports really must differ.
    assert_ne!(unlisted, session, "the hand-held borrow is not handed to a page");
    assert_ne!(unlisted_addr, addr, "each holds its own proxy port");

    // It reuses per machine the way the listed kind does.
    let (unlisted_again, _) = timeout(Duration::from_secs(60), b.borrow_unlisted("alpha"))
        .await
        .expect("a second unlisted borrow must not hang")
        .unwrap();
    assert_eq!(unlisted_again, unlisted, "one unlisted borrow per machine too");

    // Borrowing your own network is refused in words, not left to dial
    // itself and time out.
    let err = timeout(Duration::from_secs(10), b.borrow("beta"))
        .await
        .expect("a self-borrow must answer fast, not dial")
        .unwrap_err();
    assert!(
        err.contains(khor_catalog::msg::BORROW_SELF),
        "borrowing this machine is refused: {err}"
    );

    // Hold a pipe open through the proxy: the row must turn 忙碌 while
    // bytes can flow, and back to 空闲 once the pipe closes.
    let mut held = connect_through(proxy, &echo).await;
    {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        held.write_all(b"live").await.unwrap();
        let mut back = [0u8; 4];
        timeout(Duration::from_secs(10), held.read_exact(&mut back))
            .await
            .expect("echo must not hang")
            .unwrap();
        assert_eq!(&back, b"live", "the held pipe carries bytes");
    }
    assert!(
        wait_borrow_state(&b, &session, khor_core::State::Busy).await,
        "a lease with a live pipe reads 忙碌"
    );
    drop(held);
    assert!(
        wait_borrow_state(&b, &session, khor_core::State::Idle).await,
        "the lease falls back to 空闲 when the pipe closes"
    );

    // Closing the borrow removes its row; the serve reaps the proxy task
    // on its next tick.
    b.close(&khor_node::SessionId(session.clone())).unwrap();
    let gone = {
        let mut gone = false;
        for _ in 0..40 {
            // By id, for `wait_borrow_state`'s reason: "is there any
            // borrow row" and "is this borrow's row gone" stopped being
            // the same question the day a borrow could be rowless.
            let has = b.sessions().unwrap().iter().any(|v| v.session.id.0 == session);
            if !has {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        gone
    };
    assert!(gone, "close removes the borrow row");

    // ⚠️ **This block has to stay last, and the reason is the fixture's,
    // not the subject's.** It is the only place in this test that lets
    // real time pass, and once a few seconds go by, beta's own row in its
    // device table is named by this machine's hostname rather than by the
    // `open_as` name — so `resolve("beta")` stops finding it and every
    // assertion above that names a machine fails with 机器不存在, pointing
    // at a subject it has nothing to do with. Measured 2026-09-09 by
    // moving only this block: with it earlier the self-borrow refusal
    // above failed reading `网里现在有: Mac-mini、alpha`; moved here, all
    // green, nothing else changed. **What renames that row was not
    // chased** — it is a fixture question, and this test is about borrows.
    //
    // **And it survives the serve's tick — the half with no row to be
    // judged by.** `reap_borrows` asks the registry whether a borrow's
    // row is still claimed; an unlisted borrow has no row, so asking
    // would answer "gone" about something that was never there and every
    // page's network would die within one tick, seconds after it opened.
    tokio::time::sleep(khor_node::link::SYNC_EVERY + Duration::from_secs(1)).await;
    let (after_tick, _) = timeout(Duration::from_secs(60), b.borrow_unlisted("alpha"))
        .await
        .expect("an unlisted borrow must not hang after a tick")
        .unwrap();
    assert_eq!(after_tick, unlisted, "a tick does not reap a borrow that has no row");

    // **Zero rows, with a page's borrow alive.** Asserted here rather
    // than beside the hand-held one because here it can fail on its own:
    // up there the two borrows share a machine, so a flag flipped the
    // wrong way makes them the *same* borrow and the count stays 1 —
    // true, and proving nothing. Down here the hand-held row is closed,
    // so anything the unlisted road registers has to show up.
    assert_eq!(
        b.sessions().unwrap().iter().filter(|v| v.session.kind.0 == "borrow").count(),
        0,
        "a live page borrow puts nothing in the session list"
    );

    // Dropping it by hand is what ends it — there is no row to close.
    // Asking again then has to dial a fresh one.
    timeout(Duration::from_secs(10), b.drop_borrow(&unlisted))
        .await
        .expect("dropping a borrow must not hang")
        .unwrap();
    let (regrown, _) = timeout(Duration::from_secs(60), b.borrow_unlisted("alpha"))
        .await
        .expect("borrowing after a drop must not hang")
        .unwrap();
    assert_ne!(regrown, unlisted, "a dropped borrow is not handed back as a live one");
    timeout(Duration::from_secs(10), b.drop_borrow(&regrown))
        .await
        .expect("dropping a borrow must not hang")
        .unwrap();


    // **The other arm of the reuse: it is gated on the row still being
    // claimed, not merely on the machine matching.** After a close there
    // is nothing left to hand back — the proxy task was aborted and the
    // port with it — so this must dial a fresh lease rather than answer
    // with the session it just reaped. Without this arm, dropping the
    // liveness half of that test would leave every assertion above green
    // while a borrow handed out a dead port.
    let (after, _) = timeout(Duration::from_secs(60), b.borrow("alpha"))
        .await
        .expect("borrowing again after a close must not hang")
        .unwrap();
    assert_ne!(after, session, "a closed borrow is not handed back as a live one");
    b.close(&khor_node::SessionId(after)).unwrap();

    serve_a.abort();
    serve_b.abort();
    let _ = fs::remove_dir_all(&ra);
    let _ = fs::remove_dir_all(&rb);
}
