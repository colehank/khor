//! A miss belongs to the network on which the actual tunnel ask began.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    None,
    AfterRefusal,
    WhilePending,
}

fn preparing(f: &Publication) -> Owned<()> {
    Owned(Some(tokio::spawn({
        let (node, ep, target) = (f.node.clone(), f.ep.clone(), f.target.clone());
        async move { node.hop_ahead(&ep, &target).await }
    })))
}

async fn completed(task: &mut Owned<()>) {
    tokio::time::timeout(Duration::from_secs(2), task.0.take().unwrap())
        .await
        .unwrap()
        .unwrap();
}

async fn observe_epoch(change: Change) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    {
        let loaded = f.node.devices_loaded().unwrap();
        let road = Road::own(&f.target.id, RoadKind::Direct, None, Some(5));
        loaded
            .doc
            .set_road(
                &f.middle.id,
                &f.target.id,
                &serde_json::to_string(&road).unwrap(),
            )
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    let asks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (entered, first_ask) = tokio::sync::oneshot::channel();
    let (refuse, refusal) = tokio::sync::oneshot::channel();
    let (attached, attached_target) = tokio::sync::oneshot::channel();
    f.attached_target = attached_target;
    f.middle_task = Owned(Some(tokio::spawn({
        let conn = f.outer_remote.clone();
        let links = f.target_links.clone();
        let asks = asks.clone();
        let (me, via, to) = (f.ep.id(), f.middle_ep.id(), f.target_ep.id());
        async move {
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut recv).await.unwrap(),
                Ask::To(to).words()
            );
            asks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            entered.send(()).unwrap();
            let _ = refusal.await;
            send.write_all(&[crate::tunnel::NO_ROUTE]).await.unwrap();
            send.finish().unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut recv).await.unwrap(),
                Ask::To(to).words()
            );
            asks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            send.write_all(&[crate::tunnel::OK]).await.unwrap();
            let up = links.attach(me, Carrier::Hop { via }, recv, send);
            let _ = attached.send(up);
        }
    })));
    let mut first = preparing(&f);
    tokio::time::timeout(Duration::from_secs(2), first_ask)
        .await
        .unwrap()
        .unwrap();
    if change == Change::WhilePending {
        // This is the real Wires entry used by Path/Wake/resume. The old
        // ask has already entered, but its NO_ROUTE has not happened.
        f.node.wires.network_changed();
    }
    refuse.send(()).unwrap();
    completed(&mut first).await;
    assert_eq!(asks.load(std::sync::atomic::Ordering::SeqCst), 1);
    if change == Change::AfterRefusal {
        // The Addrs arm forgives existing waits without retiring healthy
        // wires. The original tunnel remains the same throughout.
        f.node.wires.forgive_all();
    }
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut next = preparing(&f);
    let serial = tokio::time::timeout(Duration::from_secs(2), offered)
        .await
        .ok()
        .and_then(Result::ok);
    let mut admitted = None;
    let mut inner = None;
    let mut answered = false;
    if serial.is_some() {
        admitted = Some(
            tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
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
    let actual_asks = asks.load(std::sync::atomic::Ordering::SeqCst);
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    if let Some(conn) = inner {
        conn.close(0u32.into(), b"miss epoch fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
    for task in [&mut first, &mut next, &mut f.middle_task, &mut f.echo_task] {
        stop(task).await;
    }
    if let Some(serial) = serial {
        f.node.carry.detach_attached(
            &f.target_ep.id(),
            Carrier::Hop {
                via: f.middle_ep.id(),
            },
            serial,
        );
    }
    f.target_links.detach(
        &f.ep.id(),
        Carrier::Hop {
            via: f.middle_ep.id(),
        },
    );
    if let Some(attached) = admitted {
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
    tokio::join!(f.ep.close(), f.target_ep.close(), f.middle_ep.close());
    let changed = change != Change::None;
    assert!(outer_open);
    assert_eq!(
        serial.is_some() && answered,
        changed,
        "only the current network may impose its tunnel miss cooldown"
    );
    assert_eq!(actual_asks, if changed { 2 } else { 1 });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_address_change_forgives_a_real_previous_hop_miss() {
    observe_epoch(Change::AfterRefusal).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_old_hop_refusal_completing_after_network_change_cannot_suppress_the_new_network() {
    observe_epoch(Change::WhilePending).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_a_network_change_the_real_hop_miss_still_waits() {
    observe_epoch(Change::None).await;
}

#[path = "candidate_change_tests.rs"]
mod candidate_change;
