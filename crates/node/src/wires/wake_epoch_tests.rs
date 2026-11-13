//! Fresh traffic must outrank a previous connection's delayed resting close.
use super::*;
use iroh::endpoint::presets;
use iroh::{Endpoint, RelayMode, SecretKey};

const ALPN: &[u8] = b"khor/wake-epoch-test";

async fn endpoint() -> Endpoint {
    Endpoint::builder(presets::Minimal)
        .secret_key(SecretKey::generate())
        .relay_mode(RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap()
}

async fn connect(far: &Endpoint, near: &Endpoint) -> (Connection, Connection) {
    tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            async { far.connect(near.addr(), ALPN).await.unwrap() },
            async { near.accept().await.unwrap().await.unwrap() },
        )
    })
    .await
    .expect("a real independent connection")
}

async fn echo(client: &Connection, server: &Connection) {
    tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            async {
                let (mut tx, mut rx) = client.open_bi().await.unwrap();
                tx.write_all(b"new epoch").await.unwrap();
                tx.finish().unwrap();
                assert_eq!(rx.read_to_end(9).await.unwrap(), b"new epoch");
            },
            async {
                let (mut tx, mut rx) = server.accept_bi().await.unwrap();
                let bytes = rx.read_to_end(9).await.unwrap();
                tx.write_all(&bytes).await.unwrap();
                tx.finish().unwrap();
            },
        )
    })
    .await
    .expect("the new connection answers actual stream data");
}

async fn suspended(client: &Connection, server: &Connection) {
    client.close(0u32.into(), Retired::Suspended.reason());
    let why = tokio::time::timeout(Duration::from_secs(5), server.closed())
        .await
        .expect("the old resting close is actually received");
    assert!(resting_close(&why));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_connection_cannot_be_marked_asleep_by_an_older_close() {
    let (near, far) = (endpoint().await, endpoint().await);
    let peer = far.id().to_string();
    let wires = Wires::default();
    let (old_client, old_server) = connect(&far, &near).await;
    wires.serving(&old_server);
    let (new_client, new_server) = connect(&far, &near).await;
    assert_ne!(old_server.stable_id(), new_server.stable_id());
    wires.serving(&new_server);

    suspended(&old_client, &old_server).await;
    let before = heard_count(&new_server);
    echo(&new_client, &new_server).await;
    assert!(heard_count(&new_server) > before, "fresh reception is real");
    wires.tick();
    assert!(
        !wires.left_alone(&peer),
        "the old suspended wire must not overwrite fresh reachability"
    );
    assert!(new_server.close_reason().is_none());
    echo(&new_client, &new_server).await;
    near.close().await;
    far.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn all_connections_really_suspended_still_leave_the_peer_alone() {
    let (near, far) = (endpoint().await, endpoint().await);
    let peer = far.id().to_string();
    let wires = Wires::default();
    let (old_client, old_server) = connect(&far, &near).await;
    wires.serving(&old_server);
    let (new_client, new_server) = connect(&far, &near).await;
    wires.serving(&new_server);
    suspended(&old_client, &old_server).await;
    suspended(&new_client, &new_server).await;
    wires.tick();
    assert!(wires.left_alone(&peer), "true background remains quiet");
    assert_eq!(wires.link(&peer).down, None, "resting is not unreachable");
    near.close().await;
    far.close().await;
}
