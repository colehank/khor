//! A late old selection must not retire the newer context's cooldown.
use super::*;

async fn observe_snapshot(add_middle: bool) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    set_hint(&f, &f.middle.id, 500);
    let network = f.node.wires.hop_epoch();
    let asks = Arc::new(AtomicUsize::new(0));
    f.middle_task = Owned(Some(tokio::spawn({
        let conn = f.outer_remote.clone();
        let asks = asks.clone();
        let to = f.target_ep.id();
        async move {
            while let Ok(stream) = conn.accept_bi().await {
                reject(stream, to, &asks).await;
            }
        }
    })));
    let (selected, release) = f.node.wires.gate_hop_selection_for_test();
    let mut older = preparing(&f);
    tokio::time::timeout(Duration::from_secs(2), selected)
        .await
        .unwrap()
        .unwrap();
    let mut middle = if add_middle {
        Some(ReachableMiddle::new(&f, true).await)
    } else {
        None
    };
    let mut newer = preparing(&f);
    completed(&mut newer).await;
    let counts = || {
        (
            asks.load(Ordering::SeqCst),
            middle.as_ref().map_or(0, |m| m.asks.load(Ordering::SeqCst)),
        )
    };
    let after_newer = counts();
    release.send(()).unwrap();
    completed(&mut older).await;
    let after_older = counts();
    let mut next = preparing(&f);
    completed(&mut next).await;
    let after_next = counts();
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    let outer_open = f.outer_local.close_reason().is_none()
        && middle.as_ref().is_none_or(|m| {
            m.local.close_reason().is_none() && m.incoming.close_reason().is_none()
        });
    for task in [
        &mut older,
        &mut newer,
        &mut next,
        &mut f.middle_task,
        &mut f.echo_task,
    ] {
        stop(task).await;
    }
    if let Some(m) = middle.as_mut() {
        stop(&mut m.task).await;
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while asking(f.node.device_str(), &f.target.id) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    if let Some(m) = middle {
        m.ep.close().await;
    }
    tokio::join!(f.ep.close(), f.target_ep.close(), f.middle_ep.close());
    assert!(outer_open);
    assert_eq!(after_newer, (1, usize::from(add_middle)));
    assert_eq!(
        after_older, after_newer,
        "an old selection cannot reopen a route after a newer miss"
    );
    assert_eq!(
        after_next, after_newer,
        "an old selection cannot retire the newer context's cooldown"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_old_selection_cannot_replace_a_newer_candidates_completed_miss() {
    observe_snapshot(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_late_selection_of_the_same_candidates_keeps_the_cooldown() {
    observe_snapshot(false).await;
}

#[path = "candidate_success_tests.rs"]
mod success;
