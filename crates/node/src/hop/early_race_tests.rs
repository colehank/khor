//! Actual ask_on, with and without TLS resumption, races a stalled middle.
use super::*;

async fn observe(warm: bool) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    let direct = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(f.target_ep.secret_key().clone())
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .alpns(vec![INNER_ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    f.target.addrs = direct
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
            _ => None,
        })
        .collect();
    assert!(!f.target.addrs.is_empty());
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
    let (entered, arrived) = tokio::sync::oneshot::channel();
    let (stopped, stop_seen) = tokio::sync::oneshot::channel();
    f.middle_task = Owned(Some(tokio::spawn({
        let (conn, target) = (f.outer_remote.clone(), f.target_ep.id());
        async move {
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut recv).await.unwrap(),
                Ask::To(target).words()
            );
            entered.send(()).unwrap();
            // Do not admit or publish a carrier. Observe the real loser's
            // STOP_SENDING when the normal connection wins or expires.
            let _ = stopped.send(matches!(send.stopped().await, Ok(Some(_))));
        }
    })));
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (observed_early, mut was_early) = tokio::sync::oneshot::channel();
    let (allow_direct, accepting) = tokio::sync::oneshot::channel();
    let mut direct_task = Owned(Some(tokio::spawn({
        let ep = direct.clone();
        let requests = requests.clone();
        async move {
            let answer =
                crate::proto::encode(&crate::proto::Response::SessionRows { rows: vec![] })
                    .unwrap();
            if warm {
                let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
                let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                let bytes = recv.read_to_end(crate::proto::MAX_FRAME).await.unwrap();
                assert!(matches!(
                    crate::proto::question(&bytes).unwrap(),
                    crate::proto::Request::Sessions
                ));
                requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                send.write_all(&answer).await.unwrap();
                send.finish().unwrap();
                conn.closed().await;
            }
            let _ = accepting.await;
            let incoming = ep.accept().await.unwrap().accept().unwrap().into_0rtt();
            let (mut send, mut recv) = incoming.accept_bi().await.unwrap();
            let early = recv.is_0rtt();
            let bytes = recv.read_to_end(crate::proto::MAX_FRAME).await.unwrap();
            assert!(matches!(
                crate::proto::question(&bytes).unwrap(),
                crate::proto::Request::Sessions
            ));
            requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let conn = incoming.handshake_completed().await.unwrap();
            observed_early.send(early).unwrap();
            send.write_all(&answer).await.unwrap();
            send.finish().unwrap();
            conn.closed().await;
        }
    })));
    if warm {
        let conn = tokio::time::timeout(
            Duration::from_secs(2),
            f.ep.connect(direct.addr(), INNER_ALPN),
        )
        .await
        .unwrap()
        .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();
        send.write_all(&crate::proto::encode(&crate::proto::Request::Sessions).unwrap())
            .await
            .unwrap();
        send.finish().unwrap();
        let bytes = recv.read_to_end(crate::proto::MAX_FRAME).await.unwrap();
        assert!(matches!(
            crate::proto::answer(&bytes).unwrap(),
            crate::proto::Response::SessionRows { .. }
        ));
        conn.close(0u32.into(), b"warm request complete");
    }
    let mut resumption = f.node.wires.observe_early_dial_for_test();
    let by = Instant::now() + Duration::from_secs(1);
    let mut calling = Owned(Some(tokio::spawn({
        let (node, ep, target) = (f.node.clone(), f.ep.clone(), f.target.clone());
        async move {
            crate::deadline::scope(
                by,
                node.ask_on(&ep, &target, &crate::proto::Request::Sessions),
            )
            .await
        }
    })));
    tokio::time::timeout_at(by, arrived).await.unwrap().unwrap();
    allow_direct.send(()).unwrap();
    let result =
        tokio::time::timeout_at(by + Duration::from_millis(300), calling.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    let won = result.is_ok();
    let answered = matches!(&result, Ok(crate::proto::Response::SessionRows { .. }));
    let actual_early = resumption.try_recv().ok();
    // is_0rtt at accept time can already be false on a completed fast
    // handshake. The client TLS Accepted result is the resumption witness.
    let server_marker = was_early.try_recv().ok();
    eprintln!(
        "early calibration: warm={warm}, accepted={actual_early:?}, server_marker={server_marker:?}"
    );
    let held_direct = f
        .node
        .wires
        .live(&f.target.id, Proto::Main)
        .is_some_and(|conn| khor_net::path::classify(&conn) == khor_net::path::PathKind::Direct);
    let total = requests.load(std::sync::atomic::Ordering::SeqCst);
    let stopped = matches!(
        tokio::time::timeout(Duration::from_secs(1), stop_seen).await,
        Ok(Ok(true))
    );
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    let orphaned = f.node.carry.has(&f.target_ep.id()) || asking(f.node.device_str(), &f.target.id);
    stop(&mut direct_task).await;
    f.close(0).await;
    direct.close().await;

    assert!(stopped && outer_open && !orphaned);
    assert!(
        won && answered && held_direct,
        "the actual reading must finish inside the original caller budget while the middle is unadmitted: warm={warm}, won={won}, answered={answered}, held_direct={held_direct}, requests={total}"
    );
    assert_eq!(actual_early, Some(warm));
    assert_eq!(
        total,
        usize::from(warm) + 1,
        "no accepted reading may be replayed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unresumed_read_does_not_wait_for_slow_middle_preparation() {
    observe(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_resumed_read_does_not_wait_for_slow_middle_preparation() {
    observe(true).await;
}
