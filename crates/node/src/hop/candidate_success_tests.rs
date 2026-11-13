//! Successful publication must not make an old selection current again.
use super::*;

async fn observe_success(release_carrier: bool) {
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
    let (selected, release_old) = f.node.wires.gate_hop_selection_for_test();
    let mut older = preparing(&f);
    tokio::time::timeout(Duration::from_secs(2), selected)
        .await
        .unwrap()
        .unwrap();
    let mut middle = ReachableMiddle::new(&f, false).await;
    let (offered, release_new) = f.node.wires.gate_hop_offered_for_test();
    let mut newer = preparing(&f);
    let serial = tokio::time::timeout(Duration::from_secs(2), offered)
        .await
        .unwrap()
        .unwrap();
    let mut attached = Some(
        tokio::time::timeout(Duration::from_secs(2), &mut middle.attached)
            .await
            .unwrap()
            .unwrap(),
    );
    release_new.send(()).unwrap();
    completed(&mut newer).await;
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
    let via = middle.ep.id();
    let before = (
        asks.load(Ordering::SeqCst),
        middle.asks.load(Ordering::SeqCst),
    );
    if release_carrier {
        conn.close(0u32.into(), b"successful consumer complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        f.node
            .carry
            .detach_attached(&f.target_ep.id(), Carrier::Hop { via }, serial);
        f.target_links.detach(&f.ep.id(), Carrier::Hop { via });
        tokio::time::timeout(Duration::from_secs(2), attached.take().unwrap().ended())
            .await
            .unwrap();
        assert!(!f.node.carry.has(&f.target_ep.id()));
    }
    release_old.send(()).unwrap();
    completed(&mut older).await;
    let after = (
        asks.load(Ordering::SeqCst),
        middle.asks.load(Ordering::SeqCst),
    );
    let still_answered = release_carrier || echo(&conn).await;
    let outer_open = f.outer_local.close_reason().is_none()
        && middle.local.close_reason().is_none()
        && middle.incoming.close_reason().is_none();
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    if !release_carrier {
        conn.close(0u32.into(), b"success snapshot fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        f.node
            .carry
            .detach_attached(&f.target_ep.id(), Carrier::Hop { via }, serial);
        f.target_links.detach(&f.ep.id(), Carrier::Hop { via });
        tokio::time::timeout(Duration::from_secs(2), attached.take().unwrap().ended())
            .await
            .unwrap();
    }
    for task in [&mut older, &mut newer, &mut f.middle_task, &mut middle.task] {
        stop(task).await;
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while asking(f.node.device_str(), &f.target.id) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    middle.ep.close().await;
    tokio::join!(f.ep.close(), f.target_ep.close(), f.middle_ep.close());
    assert!(outer_open && still_answered);
    assert_eq!(before, (0, 1));
    assert_eq!(
        after, before,
        "a successful newer route must not let an old selection ask again"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_successful_new_route_does_not_reopen_an_old_selection() {
    observe_success(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_completed_successful_consumer_does_not_make_the_old_selection_current() {
    observe_success(true).await;
}
