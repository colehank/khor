//! Total preparation budget, including the post-admission offer phase.

use super::*;
use tokio::time::Instant;

fn opening_before(f: &Fixture, by: Instant) -> Owned<bool> {
    let (node, ep, target, middle) = (
        f.node.clone(),
        f.ep.clone(),
        f.target.clone(),
        f.middle.clone(),
    );
    Owned(Some(tokio::spawn(crate::deadline::scope(by, async move {
        let picked = vec![middle.id.clone()];
        node.open_hop(&ep, &target, &[middle], &picked).await
    }))))
}

async fn observe_held_offer(caller: Option<Duration>) {
    let mut f = Fixture::new().await;
    let (entered, release) = f.node.wires.gate_hop_offer_for_test();
    let began = Instant::now();
    let within = caller.unwrap_or(HOP_WITHIN).min(HOP_WITHIN);
    let mut opening = match caller {
        Some(within) => opening_before(&f, began + within),
        None => f.opening(),
    };
    let serial = ready(entered).await;
    assert!(serial > 0);
    assert!(f.node.carry.has(&f.target.id.parse().unwrap()));
    let answer = match tokio::time::timeout_at(
        began + within + Duration::from_millis(300),
        opening.0.as_mut().unwrap(),
    )
    .await
    {
        Ok(joined) => Some(joined.unwrap()),
        Err(_) => {
            opening.0.as_ref().unwrap().abort();
            let _ = opening.0.as_mut().unwrap().await;
            None
        }
    };
    let elapsed = began.elapsed();
    let orphaned = f.node.carry.has(&f.target.id.parse().unwrap());
    let charged = missed_lately(f.node.device_str(), &f.target.id);
    let still_asking = asking(f.node.device_str(), &f.target.id);
    let stopped = matches!(
        tokio::time::timeout(Duration::from_secs(1), &mut f.first_stopped).await,
        Ok(Ok(true))
    );
    let wire_open = f.local.close_reason().is_none() && f.remote.close_reason().is_none();
    let gate_cancelled = release.is_closed();
    f.close().await;

    assert_eq!(
        answer,
        Some(false),
        "the complete preparation must return within its original budget: elapsed={elapsed:?}"
    );
    assert!(!orphaned && !still_asking && stopped && gate_cancelled);
    assert!(wire_open, "expiry stops the stream, not the middle's wire");
    assert!(
        !charged,
        "post-admission expiry is not an unreachable middle"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_original_hop_budget_includes_waiting_to_offer() {
    observe_held_offer(None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_earlier_caller_deadline_bounds_the_complete_preparation() {
    observe_held_offer(Some(Duration::from_secs(1))).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ready_offer_inside_the_callers_budget_still_succeeds() {
    let f = Fixture::new().await;
    let (entered, release) = f.node.wires.gate_hop_offer_for_test();
    let mut opening = opening_before(&f, Instant::now() + Duration::from_secs(1));
    let _serial = ready(entered).await;
    release.send(()).unwrap();
    assert!(finish(&mut opening).await);
    assert!(f.node.carry.has(&f.target.id.parse().unwrap()));
    assert!(asking(f.node.device_str(), &f.target.id));
    f.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_caller_expiring_before_admission_is_not_a_middle_miss() {
    let mut f = Fixture::new().await;
    f.local.close(0u32.into(), b"fresh tunnel budget control");
    tokio::time::timeout(Duration::from_secs(5), f.remote.closed())
        .await
        .unwrap();
    f.middle.addrs = f
        .middle_ep
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
            _ => None,
        })
        .collect();
    assert!(!f.middle.addrs.is_empty());
    let (arrived, saw_incoming) = tokio::sync::oneshot::channel();
    let (release, held) = tokio::sync::oneshot::channel();
    let mut holding = Owned(Some(tokio::spawn({
        let ep = f.middle_ep.clone();
        async move {
            let incoming = ep.accept().await.unwrap();
            arrived.send(()).unwrap();
            let _ = held.await;
            incoming.ignore();
        }
    })));
    let began = Instant::now();
    let by = began + Duration::from_secs(1);
    let mut opening = opening_before(&f, by);
    tokio::time::timeout_at(by, saw_incoming)
        .await
        .unwrap()
        .unwrap();
    let answer =
        tokio::time::timeout_at(by + Duration::from_millis(300), opening.0.as_mut().unwrap())
            .await
            .unwrap()
            .unwrap();
    let charged = missed_lately(f.node.device_str(), &f.target.id);
    let down = f.node.wires.link(&f.middle.id).down;
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), holding.0.as_mut().unwrap())
        .await
        .unwrap()
        .unwrap();
    f.close().await;

    assert!(!answer);
    assert_eq!(down, None);
    assert!(
        !charged,
        "a caller's deadline must not suppress this middle"
    );
}
