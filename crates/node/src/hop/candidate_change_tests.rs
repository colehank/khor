//! A tunnel miss must be reconsidered when a new reachable middle appears.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CandidateChange {
    None,
    CostOnly,
    AfterRefusal,
    WhilePending,
    NewerFailureBeforeOldReply,
}

fn set_hint(f: &Publication, middle: &str, ms: u16) {
    let loaded = f.node.devices_loaded().unwrap();
    let road = Road::own(&f.target.id, RoadKind::Direct, None, Some(ms));
    loaded
        .doc
        .set_road(middle, &f.target.id, &serde_json::to_string(&road).unwrap())
        .unwrap();
    let mut store = loaded.store;
    store.flush(&loaded.doc).unwrap();
}

async fn reject(stream: (SendStream, RecvStream), to: EndpointId, asks: &AtomicUsize) {
    let (mut send, mut recv) = stream;
    assert_eq!(
        crate::tunnel::read_target(&mut recv).await.unwrap(),
        Ask::To(to).words()
    );
    asks.fetch_add(1, Ordering::SeqCst);
    send.write_all(&[crate::tunnel::NO_ROUTE]).await.unwrap();
    send.finish().unwrap();
}

struct ReachableMiddle {
    ep: iroh::Endpoint,
    local: iroh::endpoint::Connection,
    incoming: iroh::endpoint::Connection,
    task: Owned<()>,
    attached: tokio::sync::oneshot::Receiver<Attached>,
    asks: Arc<AtomicUsize>,
}

impl ReachableMiddle {
    async fn new(f: &Publication, refuses: bool) -> Self {
        let ep = endpoint().await;
        let id = ep.id().to_string();
        {
            let loaded = f.node.devices_loaded().unwrap();
            loaded.doc.upsert(&id, "new-middle", &[]).unwrap();
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
        }
        set_hint(f, &id, 1);
        let (local, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(
                f.ep.connect(ep.addr(), khor_net::endpoint::TUNNEL_ALPN),
                async { ep.accept().await.unwrap().accept().unwrap().await }
            )
        })
        .await
        .unwrap();
        let (local, incoming) = (local.unwrap(), incoming.unwrap());
        assert_eq!(
            khor_net::path::classify(&local),
            khor_net::path::PathKind::Direct
        );
        f.node.wires.adopt(&id, Proto::Tunnel, local.clone());
        let asks = Arc::new(AtomicUsize::new(0));
        let (attached, ready) = tokio::sync::oneshot::channel();
        let task = Owned(Some(tokio::spawn({
            let conn = incoming.clone();
            let links = f.target_links.clone();
            let asks = asks.clone();
            let (me, via, to) = (f.ep.id(), ep.id(), f.target_ep.id());
            async move {
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    if refuses {
                        reject((send, recv), to, &asks).await;
                        continue;
                    }
                    assert_eq!(
                        crate::tunnel::read_target(&mut recv).await.unwrap(),
                        Ask::To(to).words()
                    );
                    asks.fetch_add(1, Ordering::SeqCst);
                    send.write_all(&[crate::tunnel::OK]).await.unwrap();
                    let up = links.attach(me, Carrier::Hop { via }, recv, send);
                    attached.send(up).unwrap();
                    return;
                }
            }
        })));
        Self {
            ep,
            local,
            incoming,
            task,
            attached: ready,
            asks,
        }
    }
}

async fn observe(change: CandidateChange) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    set_hint(&f, &f.middle.id, 500);
    let network = f.node.wires.hop_epoch();
    let asks = Arc::new(AtomicUsize::new(0));
    let (entered, first_ask) = tokio::sync::oneshot::channel();
    let (refuse, refusal) = tokio::sync::oneshot::channel();
    f.middle_task = Owned(Some(tokio::spawn({
        let conn = f.outer_remote.clone();
        let asks = asks.clone();
        let to = f.target_ep.id();
        async move {
            let (mut first_send, mut first_recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut first_recv).await.unwrap(),
                Ask::To(to).words()
            );
            asks.fetch_add(1, Ordering::SeqCst);
            entered.send(()).unwrap();
            let mut refusal = refusal;
            loop {
                tokio::select! {
                    _ = &mut refusal => {
                        first_send.write_all(&[crate::tunnel::NO_ROUTE]).await.unwrap();
                        first_send.finish().unwrap();
                        break;
                    }
                    stream = conn.accept_bi() => {
                        reject(stream.unwrap(), to, &asks).await;
                    }
                }
            }
            while let Ok(stream) = conn.accept_bi().await {
                reject(stream, to, &asks).await;
            }
        }
    })));
    let mut first = preparing(&f);
    tokio::time::timeout(Duration::from_secs(2), first_ask)
        .await
        .unwrap()
        .unwrap();
    let mut middle = None;
    if matches!(
        change,
        CandidateChange::WhilePending | CandidateChange::NewerFailureBeforeOldReply
    ) {
        middle = Some(
            ReachableMiddle::new(&f, change == CandidateChange::NewerFailureBeforeOldReply).await,
        );
    }
    if change == CandidateChange::NewerFailureBeforeOldReply {
        let mut newer = preparing(&f);
        completed(&mut newer).await;
        assert_eq!(middle.as_ref().unwrap().asks.load(Ordering::SeqCst), 1);
        assert_eq!(asks.load(Ordering::SeqCst), 2);
    }
    refuse.send(()).unwrap();
    completed(&mut first).await;
    if change == CandidateChange::AfterRefusal {
        middle = Some(ReachableMiddle::new(&f, false).await);
    }
    if change == CandidateChange::CostOnly {
        set_hint(&f, &f.middle.id, 501);
    }
    assert!(
        network.ptr_eq(&f.node.wires.hop_epoch()),
        "the local network did not change"
    );
    let before = (
        asks.load(Ordering::SeqCst),
        middle.as_ref().map_or(0, |m| m.asks.load(Ordering::SeqCst)),
    );
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut next = preparing(&f);
    let serial = tokio::time::timeout(Duration::from_secs(2), offered)
        .await
        .ok()
        .and_then(Result::ok);
    let mut attached = None;
    let mut inner = None;
    let mut answered = false;
    if serial.is_some() {
        let m = middle.as_mut().unwrap();
        attached = Some(
            tokio::time::timeout(Duration::from_secs(2), &mut m.attached)
                .await
                .unwrap()
                .unwrap(),
        );
        release.send(()).unwrap();
        completed(&mut next).await;
        let conn = tokio::time::timeout(
            Duration::from_secs(5),
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
        answered = echo(&conn).await;
        assert_eq!(
            khor_net::path::classify(&conn),
            khor_net::path::PathKind::Carried
        );
        f.node.wires.adopt(&f.target.id, Proto::Main, conn.clone());
        inner = Some(conn);
    } else {
        drop(release);
    }
    let after = (
        asks.load(Ordering::SeqCst),
        middle.as_ref().map_or(0, |m| m.asks.load(Ordering::SeqCst)),
    );
    let outer_open = f.outer_local.close_reason().is_none()
        && middle.as_ref().is_none_or(|m| {
            m.local.close_reason().is_none() && m.incoming.close_reason().is_none()
        });
    if let Some(conn) = inner {
        conn.close(0u32.into(), b"candidate change fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
    for task in [&mut first, &mut next, &mut f.middle_task, &mut f.echo_task] {
        stop(task).await;
    }
    if let Some(m) = middle.as_mut() {
        stop(&mut m.task).await;
        if let Some(serial) = serial {
            let via = m.ep.id();
            f.node
                .carry
                .detach_attached(&f.target_ep.id(), Carrier::Hop { via }, serial);
            f.target_links.detach(&f.ep.id(), Carrier::Hop { via });
        }
    }
    if let Some(attached) = attached {
        tokio::time::timeout(Duration::from_secs(2), attached.ended())
            .await
            .unwrap();
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
    let recover = matches!(
        change,
        CandidateChange::AfterRefusal | CandidateChange::WhilePending
    );
    assert_eq!(
        serial.is_some() && answered,
        recover,
        "new reachable candidates must get admission within the existing miss window"
    );
    if recover {
        assert_eq!(after, (1, 1));
    } else {
        assert_eq!(
            after, before,
            "the current context's cooldown still suppresses repeated asks"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_live_middle_can_recover_after_a_real_previous_refusal() {
    observe(CandidateChange::AfterRefusal).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_middle_appearing_during_an_old_ask_is_not_suppressed_by_its_refusal() {
    observe(CandidateChange::WhilePending).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unchanged_candidates_keep_the_original_cooldown() {
    observe(CandidateChange::None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cost_change_alone_keeps_the_original_cooldown() {
    observe(CandidateChange::CostOnly).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_older_refusal_cannot_replace_the_new_contexts_completed_failure() {
    observe(CandidateChange::NewerFailureBeforeOldReply).await;
}

#[path = "candidate_snapshot_tests.rs"]
mod snapshot;

#[path = "candidate_address_tests.rs"]
mod address;

#[path = "candidate_dial_fact_tests.rs"]
mod dial_fact;

#[path = "candidate_admission_tests.rs"]
mod admission;

#[path = "receiver_commit_tests.rs"]
mod receiver_commit;

#[path = "prepared_race_tests.rs"]
mod prepared_race;

#[path = "comparison_context_tests.rs"]
mod comparison_context;

#[path = "hint_context_tests.rs"]
mod hint_context;

#[path = "late_success_tests.rs"]
mod late_success;
