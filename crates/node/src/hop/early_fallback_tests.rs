//! A stale IP now belongs to a different identity; a trusted middle can recover.

use super::*;

async fn stop(task: &mut Owned<()>) {
    if let Some(task) = task.0.take() {
        task.abort();
        if let Err(error) = task.await {
            assert!(error.is_cancelled());
        }
    }
}

async fn observe(permit: bool) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    stop(&mut f.echo_task).await;
    let wrong = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .alpns(vec![INNER_ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    assert_ne!(wrong.id(), f.target_ep.id());
    f.target.addrs = wrong
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
            _ => None,
        })
        .collect();
    assert!(!f.target.addrs.is_empty());
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    f.echo_task = Owned(Some(tokio::spawn({
        let ep = f.target_ep.clone();
        let requests = requests.clone();
        async move {
            let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
            while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                let Ok(bytes) = recv.read_to_end(crate::proto::MAX_FRAME).await else {
                    break;
                };
                assert!(matches!(
                    crate::proto::question(&bytes).unwrap(),
                    crate::proto::Request::Sessions
                ));
                requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let answer =
                    crate::proto::encode(&crate::proto::Response::SessionRows { rows: vec![] })
                        .unwrap();
                if send.write_all(&answer).await.is_err() {
                    break;
                }
                let _ = send.finish();
            }
        }
    })));
    let (entered, arrived) = tokio::sync::oneshot::channel();
    let (admit, admission) = tokio::sync::oneshot::channel::<bool>();
    f.middle_task = Owned(Some(tokio::spawn({
        let conn = f.outer_remote.clone();
        let links = f.target_links.clone();
        let (me, via, target) = (f.ep.id(), f.middle_ep.id(), f.target_ep.id());
        async move {
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut recv).await.unwrap(),
                Ask::To(target).words()
            );
            entered.send(()).unwrap();
            if admission.await.unwrap_or(false) {
                if send.write_all(&[crate::tunnel::OK]).await.is_ok() {
                    links.attach(me, Carrier::Hop { via }, recv, send);
                }
            } else {
                let _ = send.write_all(&[crate::tunnel::NO_ROUTE]).await;
                let _ = send.finish();
            }
        }
    })));
    let (allow_wrong, accepting) = tokio::sync::oneshot::channel();
    let mut wrong_task = Owned(Some(tokio::spawn({
        let ep = wrong.clone();
        async move {
            let _ = accepting.await;
            loop {
                let incoming = ep.accept().await.unwrap().accept().unwrap();
                if let Ok(conn) = incoming.await {
                    conn.closed().await;
                }
            }
        }
    })));
    let failed = f.node.wires.observe_early_failure_for_test();
    let (mut offered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut release = Some(release);
    let by = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut calling = tokio::spawn({
        let (node, ep, target) = (f.node.clone(), f.ep.clone(), f.target.clone());
        async move {
            crate::deadline::scope(
                by,
                node.ask_on(&ep, &target, &crate::proto::Request::Sessions),
            )
            .await
        }
    });
    tokio::time::timeout_at(by, arrived).await.unwrap().unwrap();
    allow_wrong.send(()).unwrap();
    let failure = tokio::time::timeout_at(by, failed)
        .await
        .ok()
        .and_then(Result::ok);
    // A client-side, completed identity failure precedes the middle's answer.
    let _ = admit.send(permit);
    let mut serial = 0;
    let result = tokio::select! {
        biased;
        result = &mut calling => Some(result.unwrap()),
        offer = &mut offered => {
            serial = offer.unwrap();
            let _ = release.take().unwrap().send(());
            tokio::time::timeout_at(by + Duration::from_millis(300), &mut calling)
                .await.ok().map(Result::unwrap)
        }
        () = tokio::time::sleep_until(by + Duration::from_millis(300)) => None,
    };
    if result.is_none() {
        calling.abort();
        let _ = calling.await;
    }
    drop(release);
    let answered = matches!(result, Some(Ok(crate::proto::Response::SessionRows { .. })));
    let carried = f
        .node
        .wires
        .live(&f.target.id, Proto::Main)
        .is_some_and(|conn| khor_net::path::classify(&conn) == khor_net::path::PathKind::Carried);
    let total = requests.load(std::sync::atomic::Ordering::SeqCst);
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    stop(&mut wrong_task).await;
    f.close(serial).await;
    wrong.close().await;
    assert!(outer_open);
    assert!(
        failure
            .as_ref()
            .is_some_and(|why| why.starts_with("Refused")),
        "a real client identity failure must precede admission: {failure:?}"
    );
    assert_eq!(
        answered, permit,
        "one stale address must leave the same caller a chance to use its trusted middle: permit={permit}, answered={answered}, carried={carried}, requests={total}"
    );
    assert_eq!(carried, permit);
    assert_eq!(total, usize::from(permit));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_client_identity_failure_can_fall_back_to_the_later_peer_relay() {
    observe(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_later_middle_refusing_the_route_still_fails_without_a_request() {
    observe(false).await;
}

#[path = "hint_guard_tests.rs"]
mod hint_guard;
