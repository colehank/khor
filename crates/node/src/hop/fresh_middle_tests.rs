//! Historical low RTT must not spend the recovery budget ahead of a live peer.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_live_neighbor_is_not_hidden_by_two_fast_cached_middles() {
    let mut f = Publication::new().await;
    let mut cached = Vec::new();
    let mut tasks = Vec::new();
    let mut attempts = Vec::new();
    for _ in 0..2 {
        let ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .relay_mode(iroh::RelayMode::Disabled)
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")
            .unwrap()
            .alpns(vec![khor_net::endpoint::TUNNEL_ALPN.to_vec()])
            .bind()
            .await
            .unwrap();
        let addrs: Vec<String> = ep
            .addr()
            .addrs
            .iter()
            .filter_map(|a| match a {
                iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
                _ => None,
            })
            .collect();
        assert!(!addrs.is_empty());
        {
            let loaded = f.node.devices_loaded().unwrap();
            let (me, middle, target) = (f.node.device_str(), ep.id().to_string(), &f.target.id);
            loaded.doc.upsert(me, "caller", &[]).unwrap();
            loaded.doc.upsert(&middle, "cached", &addrs).unwrap();
            for (from, to) in [(me, middle.as_str()), (middle.as_str(), target.as_str())] {
                let road = Road::own(to, RoadKind::Direct, None, Some(1));
                loaded
                    .doc
                    .set_road(from, to, &serde_json::to_string(&road).unwrap())
                    .unwrap();
            }
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
        }
        let (attempted, observed) = tokio::sync::oneshot::channel();
        tasks.push(Owned(Some(tokio::spawn({
            let ep = ep.clone();
            async move {
                let incoming = ep.accept().await.unwrap();
                let _ = attempted.send(());
                // The formerly fast peer is no longer answering. Keep the
                // real Incoming unadmitted until the caller has finished.
                std::future::pending::<()>().await;
                drop(incoming);
            }
        }))));
        attempts.push(observed);
        cached.push(ep);
    }
    {
        let loaded = f.node.devices_loaded().unwrap();
        let road = Road::own(&f.target.id, RoadKind::Relay, None, Some(50));
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
    assert!(f.node.wires.live(&f.middle.id, Proto::Tunnel).is_some());
    let (mut offered, release) = f.node.wires.gate_hop_offered_for_test();
    let by = tokio::time::Instant::now() + Duration::from_secs(2);
    let result = crate::deadline::scope(
        by,
        f.node
            .line_to_inner(&f.ep, &f.target, Proto::Main, None, true),
    )
    .await;
    let won = result.is_ok();
    let answered = match &result {
        Ok(line) => {
            assert_eq!(
                khor_net::path::classify(&line.conn),
                khor_net::path::PathKind::Carried
            );
            echo(&line.conn).await
        }
        Err(_) => false,
    };
    let tried_cached = attempts.iter_mut().any(|seen| seen.try_recv().is_ok());
    let serial = offered.try_recv().unwrap_or(0);
    drop(release);
    if serial == 0 {
        if let Some(task) = f.middle_task.0.take() {
            task.abort();
            if let Err(error) = task.await {
                assert!(error.is_cancelled());
            }
        }
        f.middle_task = Owned(Some(tokio::spawn(async {})));
    }
    for task in &mut tasks {
        if let Some(task) = task.0.take() {
            task.abort();
            if let Err(error) = task.await {
                assert!(error.is_cancelled());
            }
        }
    }
    f.close(serial).await;
    for ep in cached {
        ep.close().await;
    }
    assert!(
        won && answered,
        "the actually live neighbor must rescue this caller inside its unchanged budget: won={won}, echo={answered}, tried_cached={tried_cached}"
    );
}
