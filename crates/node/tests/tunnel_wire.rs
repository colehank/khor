//! Borrowing a machine's network over a real wire (docs/NET.md 借网,
//! 账本: 网络类改动必须真连,还要有对照组). One serve owns alpha's key.
//! A plain TCP echo server stands in for "a site alpha's network can
//! reach". beta, paired, tunnels through alpha to that echo server and
//! gets its own bytes back — proving the pipe carries payload to a real
//! target on alpha's side and returns it. The control half: an unpaired
//! key speaking the same tunnel ALPN is refused with the status byte,
//! against the very serve beta just tunnelled through — so the refusal
//! is the pairing gate, not a dead endpoint.

use std::fs;
use std::path::PathBuf;
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

async fn wait_for_endpoint_file(root: &PathBuf) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
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

#[tokio::test]
async fn a_paired_machine_borrows_the_far_network_and_an_unpaired_key_is_refused() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let echo = echo_server().await;

    // The tunnel carries a payload bigger than one buffer, taken back
    // byte-identical — a single write-and-read cannot prove the splice
    // survives a stream that fills and drains.
    let payload: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    let borrow = timeout(Duration::from_secs(15), b.tunnel_to("alpha"))
        .await
        .expect("dialling the tunnel must not hang")
        .unwrap();
    let (mut send, mut recv) = timeout(Duration::from_secs(15), borrow.open(&echo))
        .await
        .expect("opening the pipe must not hang")
        .expect("a paired open to a reachable target must succeed");
    send.write_all(&payload).await.unwrap();
    send.finish().unwrap();
    let back = timeout(Duration::from_secs(15), recv.read_to_end(MAX_FRAME))
        .await
        .expect("reading the echo must not hang")
        .unwrap();
    assert_eq!(back, payload, "every byte, echoed through alpha's network");

    // A second pipe on the same lease reaches a dead port: the exit's
    // network answers, not ours, and the status byte says so.
    let err = timeout(Duration::from_secs(15), borrow.open("127.0.0.1:1"))
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
        Duration::from_secs(15),
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

#[tokio::test]
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
    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let echo = echo_server().await;

    // beta stands up its local proxy in front of one lease to alpha.
    let borrow = timeout(Duration::from_secs(15), b.tunnel_to("alpha"))
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
    timeout(Duration::from_secs(15), client.read_exact(&mut back))
        .await
        .expect("the echo must not hang")
        .unwrap();
    assert_eq!(&back, b"ping through the exit", "bytes made the round trip");

    // A non-CONNECT method is refused, not tunnelled: the proxy only
    // speaks CONNECT (the plain-HTTP absolute-form GET is on the ledger).
    let mut plain = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
    plain.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n").await.unwrap();
    let refused = read_until(&mut plain, b"\r\n\r\n").await;
    assert!(
        String::from_utf8_lossy(&refused).starts_with("HTTP/1.1 405"),
        "a non-CONNECT method is refused: {}",
        String::from_utf8_lossy(&refused)
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

/// Waits up to ~4s for the borrow row to reach `want`, returning whether
/// it did. The activity callback writes the state synchronously as pipes
/// come and go, so this poll only rides out the tiny gap between the wire
/// event and the disk write — no system cycle to cover.
async fn wait_borrow_state(node: &Node, want: khor_core::State) -> bool {
    for _ in 0..40 {
        if let Ok(views) = node.sessions() {
            if let Some(v) = views.iter().find(|v| v.session.kind.0 == "borrow") {
                if v.session.state.state == want {
                    return true;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn the_serve_hosts_a_borrow_row_that_turns_busy_and_close_reaps_it() {
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
    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let echo = echo_server().await;

    // The borrow is hosted by beta's serve — b.borrow hands off, gets back
    // the session and the proxy address the serve bound.
    let (session, addr) = timeout(Duration::from_secs(20), b.borrow("alpha"))
        .await
        .expect("borrow must not hang")
        .unwrap();
    let proxy: std::net::SocketAddr = addr.parse().unwrap();

    // The row is there, and idle until something uses it.
    assert!(
        wait_borrow_state(&b, khor_core::State::Idle).await,
        "a fresh borrow row opens 空闲"
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
        wait_borrow_state(&b, khor_core::State::Busy).await,
        "a lease with a live pipe reads 忙碌"
    );
    drop(held);
    assert!(
        wait_borrow_state(&b, khor_core::State::Idle).await,
        "the lease falls back to 空闲 when the pipe closes"
    );

    // Closing the borrow removes its row; the serve reaps the proxy task
    // on its next tick.
    b.close(&khor_node::SessionId(session)).unwrap();
    let gone = {
        let mut gone = false;
        for _ in 0..40 {
            let has = b
                .sessions()
                .unwrap()
                .iter()
                .any(|v| v.session.kind.0 == "borrow");
            if !has {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        gone
    };
    assert!(gone, "close removes the borrow row");

    serve_a.abort();
    serve_b.abort();
    let _ = fs::remove_dir_all(&ra);
    let _ = fs::remove_dir_all(&rb);
}
