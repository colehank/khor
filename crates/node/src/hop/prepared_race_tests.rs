//! Actual concurrent admissions, ciphertext commit, loser cleanup and stale hints.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Prepared,
    DoubleReady,
    StaleHint,
}

async fn observe_prepared(mode: Mode) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    let mut second = ReachableMiddle::new(&f, true).await;
    stop(&mut second.task).await;
    set_hint(&f, &f.middle.id, 1);
    set_hint(&f, &second.ep.id().to_string(), 10000);
    f.target.hop_staged = true;
    {
        let loaded = f.node.devices_loaded().unwrap();
        loaded.doc.set_hop_staged(&f.target.id, true).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    let network = f.node.wires.hop_epoch();
    let ready_asks = Arc::new(AtomicUsize::new(0));
    let legacy_asks = Arc::new(AtomicUsize::new(0));
    let committed = Arc::new(AtomicUsize::new(0));
    let cancelled = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let (attachments, mut attached) = tokio::sync::mpsc::unbounded_channel();
    for (first, conn, via) in [
        (true, f.outer_remote.clone(), f.middle_ep.id()),
        (false, second.incoming.clone(), second.ep.id()),
    ] {
        let task = Owned(Some(tokio::spawn({
            let (links, me, to) = (f.target_links.clone(), f.ep.id(), f.target_ep.id());
            let (ready_asks, legacy_asks, committed, cancelled, barrier, attachments) = (
                ready_asks.clone(),
                legacy_asks.clone(),
                committed.clone(),
                cancelled.clone(),
                barrier.clone(),
                attachments.clone(),
            );
            async move {
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let words = crate::tunnel::read_target(&mut recv).await.unwrap();
                    let prepared = words == Ask::PrepareTo(to).words();
                    assert!(prepared || words == Ask::To(to).words());
                    if prepared {
                        ready_asks.fetch_add(1, Ordering::SeqCst);
                        if mode == Mode::StaleHint {
                            send.write_all(&[NO_ROUTE]).await.unwrap();
                            send.finish().unwrap();
                            continue;
                        }
                        barrier.wait().await;
                    } else {
                        legacy_asks.fetch_add(1, Ordering::SeqCst);
                    }
                    if first && mode != Mode::DoubleReady {
                        if matches!(send.stopped().await, Ok(Some(_))) {
                            cancelled.fetch_add(1, Ordering::SeqCst);
                        }
                        return;
                    }
                    if send.write_all(&[OK]).await.is_err() {
                        if matches!(send.stopped().await, Ok(Some(_))) {
                            cancelled.fetch_add(1, Ordering::SeqCst);
                        }
                        return;
                    }
                    let Some(prefix) = stream_trace::first_frame(&mut recv).await else {
                        if matches!(send.stopped().await, Ok(Some(_))) {
                            cancelled.fetch_add(1, Ordering::SeqCst);
                        }
                        return;
                    };
                    committed.fetch_add(1, Ordering::SeqCst);
                    let up = stream_trace::attach_prefixed(
                        &links,
                        me,
                        Carrier::Hop { via },
                        None,
                        "prepared-fixture",
                        (send, recv),
                        prefix,
                    );
                    attachments.send(up).unwrap();
                    return;
                }
            }
        })));
        if first {
            f.middle_task = task;
        } else {
            second.task = task;
        }
    }
    drop(attachments);
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let start = tokio::time::Instant::now();
    let mut opening = preparing(&f);
    let serial = tokio::time::timeout(HOP_WITHIN + Duration::from_secs(1), offered)
        .await
        .unwrap()
        .unwrap();
    let preparation_elapsed = start.elapsed();
    release.send(()).unwrap();
    completed(&mut opening).await;
    let via = match f.node.carry.carrier(&f.target_ep.id()).unwrap() {
        Carrier::Hop { via } => via,
        _ => panic!("the candidate must publish a hop"),
    };
    if mode != Mode::DoubleReady {
        assert_eq!(via, second.ep.id());
    }
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
    assert_eq!(
        khor_net::path::classify(&conn),
        khor_net::path::PathKind::Carried
    );
    let answered = echo(&conn).await;
    f.node.wires.adopt(&f.target.id, Proto::Main, conn.clone());
    let target_attachment = tokio::time::timeout(Duration::from_secs(2), attached.recv())
        .await
        .unwrap()
        .unwrap();
    for task in [&mut f.middle_task, &mut second.task] {
        tokio::time::timeout(Duration::from_secs(2), task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
    let counts = (
        ready_asks.load(Ordering::SeqCst),
        legacy_asks.load(Ordering::SeqCst),
        committed.load(Ordering::SeqCst),
        cancelled.load(Ordering::SeqCst),
    );
    let extra_attachment = attached.try_recv().is_ok();
    let outer_open = f.outer_local.close_reason().is_none()
        && second.local.close_reason().is_none()
        && second.incoming.close_reason().is_none();
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    conn.close(0u32.into(), b"prepared race fixture complete");
    tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    f.node
        .carry
        .detach_attached(&f.target_ep.id(), Carrier::Hop { via }, serial);
    f.target_links
        .detach_attached(&f.ep.id(), Carrier::Hop { via }, target_attachment.serial());
    tokio::time::timeout(Duration::from_secs(2), target_attachment.ended())
        .await
        .unwrap();
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
        second.ep.close()
    );
    assert!(answered && outer_open && !extra_attachment);
    assert_eq!(
        counts,
        if mode == Mode::StaleHint {
            (2, 2, 1, 1)
        } else {
            (2, 0, 1, 1)
        }
    );
    assert!(preparation_elapsed < HOP_WITHIN);
    if mode != Mode::StaleHint {
        assert!(
            preparation_elapsed < HOP_WITHIN / 2,
            "prepared peers must race rather than wait for the legacy first-slot deadline"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_prepared_second_middle_can_win_while_the_first_status_is_pending() {
    observe_prepared(Mode::Prepared).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_ready_middles_commit_only_one_actual_ciphertext_transport() {
    observe_prepared(Mode::DoubleReady).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stale_support_hint_still_uses_bounded_legacy_admission() {
    observe_prepared(Mode::StaleHint).await;
}
