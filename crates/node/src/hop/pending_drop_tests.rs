//! A pending caller can arrive during the opening future's synchronous Drop.

use super::*;

async fn observe_drop(successor: bool) {
    let mut f = Publication::new().await;
    let (offered, release_offer) = f.node.wires.gate_hop_offered_for_test();
    let mut opening = f.opening();
    let serial = ready(offered).await;
    let remote = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    let (dropping, release_drop) = f.node.wires.gate_pending_drop_for_test();
    opening.0.as_ref().unwrap().abort();
    assert_eq!(ready(dropping).await, serial);
    // The opening future has already observed no consumer. A fresh caller
    // now owns its finite lease before the actual carrier withdrawal.
    let next = successor.then(|| f.node.wires.pending_dial(&f.target.id));
    let conn = if successor {
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
    release_drop.send(()).unwrap();
    cancel(&mut opening).await;
    drop(release_offer);
    let kept = f.node.carry.has(&f.target_ep.id());
    let answered = match conn.as_ref() {
        Some(conn) => echo(conn).await,
        None => false,
    };
    if let Some(conn) = conn.filter(|_| kept) {
        f.node.wires.adopt(&f.target.id, Proto::Main, conn);
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
        "Drop must include the newly pending caller"
    );
    assert_eq!(answered, successor);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_pending_caller_survives_the_opening_future_drop() {
    observe_drop(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_opening_future_drop_without_a_consumer_still_releases() {
    observe_drop(false).await;
}
