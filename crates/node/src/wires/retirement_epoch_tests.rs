//! Retirement of an old wire must commit before a replacement publishes.
use super::*;
use iroh::endpoint::presets;
use iroh::{Endpoint, RelayMode, SecretKey};

const ALPN: &[u8] = b"khor/retirement-epoch-test";

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
    .expect("a real independent wire")
}

async fn echo(client: &Connection, server: &Connection) {
    tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            async {
                let (mut tx, mut rx) = client.open_bi().await.unwrap();
                tx.write_all(b"new").await.unwrap();
                tx.finish().unwrap();
                assert_eq!(rx.read_to_end(3).await.unwrap(), b"new");
            },
            async {
                let (mut tx, mut rx) = server.accept_bi().await.unwrap();
                let bytes = rx.read_to_end(3).await.unwrap();
                tx.write_all(&bytes).await.unwrap();
                tx.finish().unwrap();
            },
        )
    })
    .await
    .expect("replacement actually answers while old retirement is held");
}

async fn suspend(client: &Connection, server: &Connection) {
    client.close(0u32.into(), Retired::Suspended.reason());
    let why = tokio::time::timeout(Duration::from_secs(5), server.closed())
        .await
        .expect("the peer's old resting close is received");
    assert!(resting_close(&why));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retiring_old_wire_cannot_mark_a_published_replacement_asleep() {
    let (near, far) = (endpoint().await, endpoint().await);
    let peer = far.id().to_string();
    let wires = Arc::new(Wires::default());
    let (old_client, old_server) = connect(&far, &near).await;
    wires.adopt(&peer, Proto::Main, old_server.clone());
    suspend(&old_client, &old_server).await;

    let (entered, arrived) = tokio::sync::oneshot::channel();
    let (released, release) = std::sync::mpsc::channel();
    *wires.retirement_test_gate.lock().unwrap() = Some(RetirementTestGate { entered, release });
    let retiring = {
        let wires = wires.clone();
        let peer = peer.clone();
        tokio::task::spawn_blocking(move || {
            wires.retire(&peer, Proto::Main, &old_server, Retired::Asleep)
        })
    };
    tokio::time::timeout(Duration::from_secs(5), arrived)
        .await
        .expect("retire released the slot before its completion")
        .unwrap();
    let (new_client, new_server) = connect(&far, &near).await;
    wires.adopt(&peer, Proto::Main, new_server.clone());
    echo(&new_client, &new_server).await;
    assert!(!wires.left_alone(&peer), "new adoption clears old state");
    released.send(()).unwrap();
    assert!(retiring.await.unwrap());
    assert!(
        !wires.left_alone(&peer),
        "finishing old retirement must not undo a published live replacement"
    );
    assert_eq!(
        wires.live(&peer, Proto::Main).unwrap().stable_id(),
        new_server.stable_id()
    );
    echo(&new_client, &new_server).await;
    near.close().await;
    far.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retiring_the_current_suspended_wire_without_replacement_stays_quiet() {
    let (near, far) = (endpoint().await, endpoint().await);
    let peer = far.id().to_string();
    let wires = Wires::default();
    let (client, server) = connect(&far, &near).await;
    wires.adopt(&peer, Proto::Main, server.clone());
    suspend(&client, &server).await;
    assert!(wires.retire(&peer, Proto::Main, &server, Retired::Asleep));
    assert!(wires.left_alone(&peer));
    assert_eq!(wires.link(&peer).down, None);
    near.close().await;
    far.close().await;
}
