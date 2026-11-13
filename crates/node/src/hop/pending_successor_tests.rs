//! A new caller can take over after the previous pending dial finishes.

use super::*;

async fn observe_successor(successor: bool) {
    let mut f = Publication::new().await;
    let first = f.node.wires.pending_dial(&f.target.id);
    let (entered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut opening = f.opening();
    let serial = ready(entered).await;
    let remote = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    let conn = tokio::time::timeout(
        Duration::from_secs(2),
        f.ep.connect(
            iroh::EndpointAddr::from_parts(f.target_ep.id(), [transport_addr(&f.target_ep.id())]),
            INNER_ALPN,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(echo(&conn).await);
    assert!(f.node.wires.live(&f.target.id, Proto::Main).is_none());
    let (drained, reconsider, reviewed) = f.node.wires.gate_pending_carrier_for_test();
    cancel(&mut opening).await;
    drop(release);
    drop(first);
    assert_eq!(ready(drained).await, serial);
    // The old owner is gone. The old watcher's cleanup decision has not
    // happened yet; this is an actual new caller, not a cloned old lease.
    let next = successor.then(|| f.node.wires.pending_dial(&f.target.id));
    reconsider.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), reviewed)
        .await
        .unwrap()
        .unwrap();
    let kept = f.node.carry.has(&f.target_ep.id());
    let answered = if successor { echo(&conn).await } else { false };
    if successor && kept {
        f.node.wires.adopt(&f.target.id, Proto::Main, conn.clone());
    }
    drop(next);
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    f.close(serial).await;
    tokio::time::timeout(Duration::from_secs(2), remote.ended())
        .await
        .unwrap();
    assert!(outer_open);
    assert_eq!(
        kept, successor,
        "the cleanup decision must include the new pending caller"
    );
    assert_eq!(answered, successor);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_successor_pending_dial_keeps_the_existing_actual_carried_wire() {
    observe_successor(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_finished_pending_dial_without_a_successor_still_releases() {
    observe_successor(false).await;
}
