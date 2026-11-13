//! A target-wide miss must not hide an untried or unblocked middle.
use super::*;

async fn observe_admission(first_hangs: bool) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    set_hint(&f, &f.middle.id, 1);
    let network = f.node.wires.hop_epoch();
    let asks = Arc::new(AtomicUsize::new(0));
    let (stopped, cancelled) = tokio::sync::oneshot::channel();
    f.middle_task = Owned(Some(tokio::spawn({
        let conn = f.outer_remote.clone();
        let asks = asks.clone();
        let to = f.target_ep.id();
        async move {
            if first_hangs {
                let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                assert_eq!(
                    crate::tunnel::read_target(&mut recv).await.unwrap(),
                    Ask::To(to).words()
                );
                asks.fetch_add(1, Ordering::SeqCst);
                let _recv = recv;
                let _ = stopped.send(matches!(send.stopped().await, Ok(Some(_))));
            } else {
                drop(stopped);
                while let Ok(stream) = conn.accept_bi().await {
                    reject(stream, to, &asks).await;
                }
            }
        }
    })));
    let mut refuser = if first_hangs {
        None
    } else {
        Some(ReachableMiddle::new(&f, true).await)
    };
    if let Some(m) = &refuser {
        set_hint(&f, &m.ep.id().to_string(), 1000);
    }
    let mut good = ReachableMiddle::new(&f, false).await;
    set_hint(&f, &good.ep.id().to_string(), 10000);
    let mut earlier = None;
    if !first_hangs {
        let mut first = preparing(&f);
        completed(&mut first).await;
        assert_eq!(asks.load(Ordering::SeqCst), 1);
        assert_eq!(refuser.as_ref().unwrap().asks.load(Ordering::SeqCst), 1);
        assert_eq!(good.asks.load(Ordering::SeqCst), 0);
        earlier = Some(first);
    }
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let start = tokio::time::Instant::now();
    let mut next = preparing(&f);
    let serial = tokio::time::timeout(HOP_WITHIN + Duration::from_secs(1), offered)
        .await
        .ok()
        .and_then(Result::ok);
    let preparation_elapsed = start.elapsed();
    let mut attached = None;
    let mut answered = false;
    if serial.is_some() {
        attached = Some(
            tokio::time::timeout(Duration::from_secs(2), &mut good.attached)
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
        conn.close(0u32.into(), b"admission fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    } else {
        drop(release);
    }
    let loser_cancelled = if first_hangs {
        tokio::time::timeout(Duration::from_secs(2), cancelled)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
    } else {
        true
    };
    let counts = (
        asks.load(Ordering::SeqCst),
        refuser
            .as_ref()
            .map_or(0, |m| m.asks.load(Ordering::SeqCst)),
        good.asks.load(Ordering::SeqCst),
    );
    let outer_open = f.outer_local.close_reason().is_none()
        && f.outer_remote.close_reason().is_none()
        && good.local.close_reason().is_none()
        && good.incoming.close_reason().is_none();
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    for task in [
        &mut next,
        &mut f.middle_task,
        &mut f.echo_task,
        &mut good.task,
    ] {
        stop(task).await;
    }
    if let Some(task) = earlier.as_mut() {
        stop(task).await;
    }
    if let Some(m) = refuser.as_mut() {
        stop(&mut m.task).await;
    }
    if let Some(serial) = serial {
        let via = good.ep.id();
        f.node
            .carry
            .detach_attached(&f.target_ep.id(), Carrier::Hop { via }, serial);
        f.target_links.detach(&f.ep.id(), Carrier::Hop { via });
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
    if let Some(m) = refuser {
        m.ep.close().await;
    }
    good.ep.close().await;
    tokio::join!(f.ep.close(), f.target_ep.close(), f.middle_ep.close());
    assert!(outer_open);
    assert!(
        serial.is_some() && answered,
        "an untried or unblocked middle must be able to restore actual carried traffic"
    );
    assert!(
        preparation_elapsed < HOP_WITHIN,
        "a blocked first middle must not consume the entire shared hop budget"
    );
    assert!(
        loser_cancelled,
        "the losing request stream must observe cancellation"
    );
    assert_eq!(counts, if first_hangs { (1, 0, 1) } else { (1, 1, 1) });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_refusals_do_not_cool_an_untried_third_middle() {
    observe_admission(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_blocked_first_middle_cannot_starve_a_ready_second_middle() {
    observe_admission(true).await;
}
