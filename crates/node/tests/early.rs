//! **What may ride 0-RTT, and what waits for the handshake** (round
//! NET-3). A paired key redials alpha with the TLS session of an earlier
//! connection and sends a request as early data. Early data can be
//! replayed by anyone who captured it, and a replay never completes the
//! handshake (it lacks the keys), so the rule the serve keeps is: a
//! request that may not ride early (`Request::rides_early`) is held until
//! the handshake completes, and dropped if it never does.
//!
//! Proved with a write: a directory asked for in early data by a client
//! that then abandons the handshake is **not** made. The control is the
//! same early request with the handshake allowed to finish, which is.

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_net::endpoint::{self, ALPN};
use khor_node::Node;
use khor_node::proto::{self, Request, Response};
use tokio::time::timeout;

mod util;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-early-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// **A road that can stop coming back**: what the client sends always
/// reaches `to`; what comes back reaches the client only while `back` is
/// set. Cleared, the client never sees the server's handshake and never
/// finishes it — exactly as a replay of captured early data never does.
/// Its address.
async fn road(
    to: std::net::SocketAddr,
    back: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> std::net::SocketAddr {
    let sock = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let at = sock.local_addr().unwrap();
    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        let mut client: Option<std::net::SocketAddr> = None;
        while let Ok((n, from)) = sock.recv_from(&mut buf).await {
            if client.is_none() || client == Some(from) {
                client = Some(from);
                let _ = sock.send_to(&buf[..n], to).await;
            } else if back.load(std::sync::atomic::Ordering::SeqCst)
                && let Some(c) = client
            {
                let _ = sock.send_to(&buf[..n], c).await;
            }
        }
    });
    at
}

/// A client endpoint that knows only what it is told: no relay, no
/// local discovery, so the road above is its one way to alpha.
async fn bare(key: iroh::SecretKey) -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(key)
        .relay_mode(iroh::RelayMode::Disabled)
        .bind()
        .await
        .unwrap()
}

/// Sends `req` as early data on a resumed connection. With `finish`,
/// lets the handshake complete and returns the answer; without, leaves
/// it (the road is one way) and gives the early data a moment to land.
async fn early(ep: &iroh::Endpoint, to: iroh::EndpointAddr, req: &Request, finish: bool) -> Option<Response> {
    let connecting = ep.connect_with_opts(to, ALPN, Default::default()).await.unwrap();
    let early = connecting
        .into_0rtt()
        .unwrap_or_else(|_| panic!("a ticket from the first connection makes 0-RTT possible"));
    let (mut send, mut recv) = early.open_bi().await.unwrap();
    send.write_all(&proto::encode(req).unwrap()).await.unwrap();
    send.finish().unwrap();
    if !finish {
        tokio::time::sleep(Duration::from_secs(1)).await;
        early.close(0u32.into(), b"abandoned");
        return None;
    }
    match early.handshake_completed().await.unwrap() {
        iroh::endpoint::ZeroRttStatus::Accepted(_) => {}
        iroh::endpoint::ZeroRttStatus::Rejected(_) => panic!("alpha rejected the early data"),
    }
    let bytes = recv.read_to_end(proto::MAX_FRAME).await.unwrap();
    Some(proto::decode::<Response>(&bytes).unwrap())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_early_write_waits_for_a_handshake_that_a_replay_never_completes() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(60), b.pair(&a.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let alpha = b
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .unwrap();
    // beta's own key: the scratch identity this test made.
    let key = khor_net::identity::load_or_create(&rb.join(".khor").join("identity.key")).unwrap();
    let direct: std::net::SocketAddr = alpha
        .addrs
        .iter()
        .filter_map(|a| a.parse::<std::net::SocketAddr>().ok())
        .find(|a| a.ip().is_loopback())
        .or_else(|| alpha.addrs.iter().filter_map(|a| a.parse().ok()).next())
        .expect("alpha has a direct address");
    let back = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let at = road(direct, back.clone()).await;
    let to = || endpoint::dial_addr(&alpha.id, &[at.to_string()], &[]).unwrap();

    // A first, ordinary connection: admitted, answered, and the ticket.
    let ep = bare(key.clone()).await;
    let conn = ep.connect(to(), ALPN).await.unwrap();
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    let listing = Request::Ls {
        path: ra.display().to_string(),
    };
    send.write_all(&proto::encode(&listing).unwrap()).await.unwrap();
    send.finish().unwrap();
    let first = proto::decode::<Response>(&recv.read_to_end(proto::MAX_FRAME).await.unwrap()).unwrap();
    assert!(matches!(first, Response::Dir { .. }), "the paired key is answered: {first:?}");
    // The session ticket follows the handshake in its own packet.
    tokio::time::sleep(Duration::from_millis(500)).await;
    conn.close(0u32.into(), b"done");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // An early write whose handshake cannot finish: never made.
    back.store(false, std::sync::atomic::Ordering::SeqCst);
    let never = ra.join("made-by-a-replay");
    let abandoned = early(
        &ep,
        to(),
        &Request::Mkdir {
            path: never.display().to_string(),
        },
        false,
    )
    .await;
    assert!(abandoned.is_none());
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(!never.exists(), "an early write ran without a handshake");

    // The control: the same early write, the handshake finished — made.
    // A road of its own: the one above knows the first client's port.
    let back = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let at = road(direct, back).await;
    let to = || endpoint::dial_addr(&alpha.id, &[at.to_string()], &[]).unwrap();
    let ep = bare(key.clone()).await;
    let conn = ep.connect(to(), ALPN).await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    conn.close(0u32.into(), b"ticket");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let made = ra.join("made-after-the-handshake");
    let answer = early(
        &ep,
        to(),
        &Request::Mkdir {
            path: made.display().to_string(),
        },
        true,
    )
    .await;
    assert!(made.is_dir(), "the early write did not run after the handshake: {answer:?}");
    ep.close().await;
    serve_a.abort();
}
