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
