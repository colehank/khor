//! Comparison work follows actual route facts, not a process-global clock.
use super::*;

fn comparison_epoch(f: &Publication, target: &str, picked: &[String]) -> MissEpoch {
    f.node
        .hop_miss_epoch(
            target,
            picked,
            MissObservation::new(f.node.wires.hop_epoch()),
        )
        .unwrap()
}

fn claim(f: &Publication, target: &str, epoch: &MissEpoch, middle: &str, comparison: bool) -> bool {
    begin_middle(
        f.node.device_str(),
        target,
        epoch,
        middle,
        comparison,
        tokio::time::Instant::now() + HOP_WITHIN,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn new_actual_middle_facts_do_not_wait_for_unchanged_comparison_work() {
    let mut f = Publication::new().await;
    let first = comparison_epoch(&f, &f.target.id, &[f.middle.id.clone()]);
    assert!(claim(&f, &f.target.id, &first, &f.middle.id, true));
    let same = comparison_epoch(&f, &f.target.id, &[f.middle.id.clone()]);
    assert!(!claim(&f, &f.target.id, &same, &f.middle.id, true));
    let mut second = ReachableMiddle::new(&f, true).await;
    let second_id = second.ep.id().to_string();
    let fresh = comparison_epoch(&f, &f.target.id, &[f.middle.id.clone(), second_id.clone()]);
    assert!(!claim(&f, &f.target.id, &first, &second_id, true));
    assert!(!claim(&f, &f.target.id, &fresh, &f.middle.id, true));
    assert!(!begin_middle(
        f.node.device_str(),
        &f.target.id,
        &fresh,
        &second_id,
        true,
        tokio::time::Instant::now() - Duration::from_millis(1),
    ));
    assert!(claim(&f, &f.target.id, &fresh, &second_id, true));
    assert!(!claim(&f, &f.target.id, &fresh, &second_id, true));
    // Another destination cannot consume or inherit this destination's work.
    let other_target = comparison_epoch(&f, &second_id, &[f.middle.id.clone()]);
    assert!(claim(&f, &second_id, &other_target, &f.middle.id, true));
    // A foreground recovery is bounded by its failure and caller budget,
    // not by the periodic improvement-comparison allowance.
    assert!(claim(&f, &f.target.id, &fresh, &f.middle.id, false));
    f.node.wires.forgive_all();
    let network = comparison_epoch(&f, &f.target.id, &[f.middle.id.clone(), second_id.clone()]);
    assert!(claim(&f, &f.target.id, &network, &f.middle.id, true));
    assert!(!claim(&f, &f.target.id, &fresh, &second_id, true));
    assert!(f.outer_local.close_reason().is_none() && second.local.close_reason().is_none());
    for task in [&mut f.middle_task, &mut f.echo_task, &mut second.task] {
        stop(task).await;
    }
    tokio::join!(
        f.ep.close(),
        f.target_ep.close(),
        f.middle_ep.close(),
        second.ep.close()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_carried_success_preserves_the_unchanged_comparison_allowance() {
    let mut f = Publication::new().await;
    let first = comparison_epoch(&f, &f.target.id, &[f.middle.id.clone()]);
    assert!(claim(&f, &f.target.id, &first, &f.middle.id, true));
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut opening = f.opening();
    let serial = tokio::time::timeout(Duration::from_secs(2), offered)
        .await
        .unwrap()
        .unwrap();
    let remote = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    release.send(()).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), opening.0.take().unwrap())
            .await
            .unwrap()
            .unwrap()
    );
    let conn = tokio::time::timeout(
        Duration::from_secs(5),
        f.ep.connect(
            iroh::EndpointAddr::from_parts(f.target_ep.id(), [transport_addr(&f.target_ep.id())]),
            INNER_ALPN,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(echo(&conn).await);
    assert_eq!(
        khor_net::path::classify(&conn),
        khor_net::path::PathKind::Carried
    );
    f.node.wires.adopt(&f.target.id, Proto::Main, conn.clone());
    let current = comparison_epoch(&f, &f.target.id, &[f.middle.id.clone()]);
    let success_kept_allowance = !claim(&f, &f.target.id, &current, &f.middle.id, true);
    let old_cannot_mark = !claim(&f, &f.target.id, &first, &f.middle.id, true);
    conn.close(0u32.into(), b"comparison context fixture complete");
    f.close(serial).await;
    tokio::time::timeout(Duration::from_secs(2), remote.ended())
        .await
        .unwrap();
    assert!(success_kept_allowance && old_cannot_mark);
}
