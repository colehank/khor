//! Real-relay acceptance (docs/NET.md 中继/验收): with no direct road
//! and no reachable discovery, a dial must still arrive — through the
//! Khor relay tier. Network-dependent (needs the aliyun relay), so
//! ignored by default: `cargo test -p khor-net --test relay -- --ignored`.

use std::time::Duration;

use khor_net::endpoint::{self, ALPN, KHOR_RELAYS};
use tokio::time::timeout;

/// The relay client speaks HTTP; a machine-wide proxy would carry the
/// probe and make "reachable" a lie about the wrong network.
fn clear_proxies() {
    for k in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        unsafe { std::env::remove_var(k) };
    }
}

#[tokio::test]
#[ignore = "真连阿里云中继,网络相关——手动跑"]
async fn a_relay_only_dial_connects_and_a_ghost_peer_does_not() {
    clear_proxies();
    let relays: Vec<String> = KHOR_RELAYS.iter().map(|s| s.to_string()).collect();

    let server = endpoint::bind(iroh::SecretKey::generate(), &relays).await.unwrap();
    let server_id = server.addr().id.to_string();
    // Until the server actually sits on a relay, the client's relay road
    // points at a house nobody entered.
    timeout(Duration::from_secs(20), async {
        while !server
            .addr()
            .addrs
            .iter()
            .any(|a| matches!(a, iroh::TransportAddr::Relay(_)))
        {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("the server should reach a relay within 20s");

    let echo = tokio::spawn(async move {
        let incoming = server.accept().await.expect("accept ended early");
        let conn = incoming.await.expect("handshake failed");
        let (mut send, mut recv) = conn.accept_bi().await.expect("no stream");
        let msg = recv.read_to_end(1024).await.expect("read failed");
        send.write_all(&msg).await.expect("write failed");
        send.finish().expect("finish failed");
        tokio::time::sleep(Duration::from_secs(3)).await;
    });

    // The dial carries the relay road only — no IPs.
    let client = endpoint::bind(iroh::SecretKey::generate(), &relays).await.unwrap();
    let addr = endpoint::dial_addr(&server_id, &[], &relays).unwrap();
    let conn = timeout(Duration::from_secs(20), client.connect(addr, ALPN))
        .await
        .expect("a relay dial must not hang")
        .expect("the relay road should carry the connection");
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    send.write_all(b"over the relay").await.unwrap();
    send.finish().unwrap();
    let back = recv.read_to_end(1024).await.unwrap();
    assert_eq!(back, b"over the relay", "the echo should come back verbatim");

    // Control: the relay works, but a peer that is not there stays not
    // there — a ghost id must time out, not "connect".
    let ghost = iroh::SecretKey::generate().public().to_string();
    let addr = endpoint::dial_addr(&ghost, &[], &relays).unwrap();
    let refused = timeout(Duration::from_secs(8), client.connect(addr, ALPN)).await;
    assert!(
        refused.is_err() || refused.unwrap().is_err(),
        "a ghost peer must not connect through a healthy relay"
    );

    echo.abort();
    client.close().await;
}
