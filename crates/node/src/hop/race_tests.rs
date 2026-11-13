//! A slow middle must not hold an actually available ordinary connection.

use super::*;
use tokio::time::Instant;

async fn stop(task: &mut Owned<()>) {
    if let Some(task) = task.0.take() {
        task.abort();
        if let Err(error) = task.await {
            assert!(error.is_cancelled());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_middle_waiting_for_admission_cannot_hold_the_fast_direct_wire() {
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
    let (allow_direct, accepting) = tokio::sync::oneshot::channel();
    let mut direct_task = Owned(Some(tokio::spawn({
        let ep = direct.clone();
        async move {
            let _ = accepting.await;
            let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
            while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                let Ok(bytes) = recv.read_to_end(64).await else {
                    break;
                };
                if send.write_all(&bytes).await.is_err() {
                    break;
                }
                let _ = send.finish();
            }
        }
    })));
    let by = Instant::now() + Duration::from_secs(1);
    let mut calling = Owned(Some(tokio::spawn({
        let (node, ep, target) = (f.node.clone(), f.ep.clone(), f.target.clone());
        async move {
            crate::deadline::scope(
                by,
                node.line_to_inner(&ep, &target, Proto::Main, None, true),
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
    let echoed = match &result {
        Ok(line) => {
            assert_eq!(
                khor_net::path::classify(&line.conn),
                khor_net::path::PathKind::Direct
            );
            echo(&line.conn).await
        }
        Err(_) => false,
    };
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
        won && echoed,
        "the available IP wire must win inside the caller's budget while the middle is still unadmitted: won={won}, echoed={echoed}"
    );
}

#[path = "early_race_tests.rs"]
mod early;
