//! A successful old ask is not a commit after its route context expired.
use super::*;

async fn observe_late_success(release_consumer: bool) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    set_hint(&f, &f.middle.id, 500);
    let network = f.node.wires.hop_epoch();
    let (entered, old_entered) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let (finished, old_target) = tokio::sync::oneshot::channel();
    f.middle_task = Owned(Some(tokio::spawn({
        let (conn, links) = (f.outer_remote.clone(), f.target_links.clone());
        let (me, via, to) = (f.ep.id(), f.middle_ep.id(), f.target_ep.id());
        async move {
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut recv).await.unwrap(),
                Ask::To(to).words()
            );
            entered.send(()).unwrap();
            released.await.unwrap();
            send.write_all(&[OK]).await.unwrap();
            // Use the production receiver's complete-frame commit, so a
            // status-only rejected loser cannot damage the newer target.
            let attached = stream_trace::first_frame(&mut recv).await.map(|prefix| {
                stream_trace::attach_prefixed(
                    &links,
                    me,
                    Carrier::Hop { via },
                    None,
                    "late-success-fixture",
                    (send, recv),
                    prefix,
                )
            });
            let _ = finished.send(attached);
        }
    })));
    let start = tokio::time::Instant::now();
    let mut older = f.opening();
    tokio::time::timeout(Duration::from_secs(2), old_entered)
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
    assert_eq!(
        f.node.carry.carrier(&f.target_ep.id()),
        Some(Carrier::Hop { via })
    );
    if release_consumer {
        conn.close(0u32.into(), b"newer successful consumer complete");
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
    assert!(
        start.elapsed() < HOP_WITHIN - Duration::from_millis(500),
        "the old successful reply must still be inside its original operation budget"
    );
    release.send(()).unwrap();
    let stale_committed = tokio::time::timeout(Duration::from_secs(2), older.0.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    let carrier_after = f.node.carry.carrier(&f.target_ep.id());
    let still_answered = release_consumer || echo(&conn).await;
    let outer_open = f.outer_local.close_reason().is_none()
        && middle.local.close_reason().is_none()
        && middle.incoming.close_reason().is_none();
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    // Remove only a stale attachment if the baseline wrongly published it.
    let old_via = f.middle_ep.id();
    f.node
        .carry
        .detach(&f.target_ep.id(), Carrier::Hop { via: old_via });
    let old_attached = tokio::time::timeout(Duration::from_secs(2), old_target)
        .await
        .unwrap()
        .unwrap();
    if let Some(old_attached) = old_attached {
        f.target_links.detach_attached(
            &f.ep.id(),
            Carrier::Hop { via: old_via },
            old_attached.serial(),
        );
        tokio::time::timeout(Duration::from_secs(2), old_attached.ended())
            .await
            .unwrap();
    }
    if !release_consumer {
        conn.close(0u32.into(), b"late success fixture complete");
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
    for task in [&mut newer, &mut f.middle_task, &mut middle.task] {
        stop(task).await;
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while asking(f.node.device_str(), &f.target.id) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::join!(
        f.ep.close(),
        f.target_ep.close(),
        f.middle_ep.close(),
        middle.ep.close()
    );
    assert!(outer_open && still_answered);
    assert!(
        !stale_committed,
        "an OK from an expired ask context must not commit a carrier"
    );
    assert_eq!(
        carrier_after,
        if release_consumer {
            None
        } else {
            Some(Carrier::Hop { via })
        }
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_old_successful_ask_cannot_replace_a_newer_live_consumer() {
    observe_late_success(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_old_successful_ask_cannot_publish_after_a_newer_consumer_finished() {
    observe_late_success(true).await;
}
