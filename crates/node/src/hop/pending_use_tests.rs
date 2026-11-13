//! A completed handshake can still belong to its dial before registration.

use super::*;

async fn observe_pending(register: bool, other_peer: bool) {
    let mut f = Publication::new().await;
    let peer = if other_peer {
        f.middle.id.clone()
    } else {
        f.target.id.clone()
    };
    let pending = f.node.wires.pending_dial(&peer);
    let (entered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut opening = f.opening();
    let serial = ready(entered).await;
    let remote = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    let conn = if register {
        let conn = tokio::time::timeout(
            Duration::from_secs(2),
            f.ep.connect(
                iroh::EndpointAddr::from_parts(
                    f.target_ep.id(),
                    [transport_addr(&f.target_ep.id())],
                ),
                INNER_ALPN,
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(echo(&conn).await);
        assert!(f.node.wires.live(&f.target.id, Proto::Main).is_none());
        Some(conn)
    } else {
        None
    };
    cancel(&mut opening).await;
    drop(release);
    let kept_while_pending = f.node.carry.has(&f.target_ep.id());
    let owned_while_pending = asking(f.node.device_str(), &f.target.id);
    if let Some(conn) = &conn {
        // This is the caller's adoption, after the opening future stopped.
        f.node.wires.adopt(&f.target.id, Proto::Main, conn.clone());
    }
    drop(pending);
    let answered = match &conn {
        Some(conn) => echo(conn).await,
        None => true,
    };
    if conn.is_none() {
        tokio::time::timeout(Duration::from_secs(2), async {
            while f.node.carry.has(&f.target_ep.id()) || asking(f.node.device_str(), &f.target.id) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    f.close(serial).await;
    tokio::time::timeout(Duration::from_secs(2), remote.ended())
        .await
        .unwrap();
    assert!(outer_open);
    assert!(
        answered,
        "the completed handshake must survive until its caller adopts it"
    );
    assert_eq!(kept_while_pending, !other_peer);
    assert_eq!(owned_while_pending, !other_peer);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_pending_dial_keeps_its_completed_unregistered_carried_wire() {
    observe_pending(true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_the_last_pending_dial_reclaims_an_unused_carrier() {
    observe_pending(false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn another_peers_pending_dial_cannot_keep_an_unused_carrier() {
    observe_pending(false, true).await;
}

#[path = "pending_successor_tests.rs"]
mod successor;
