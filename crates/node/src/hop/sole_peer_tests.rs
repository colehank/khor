//! Candidate admission through the sole currently reachable neighbor.
//!
//! The first Tunnel, admission, inner TLS and echo are real. The target
//! transport is adapted by Publication's scripted middle, so the reported
//! second leg is deliberately a hint, not a measured public-relay leg.

use super::*;

async fn stop(task: &mut Owned<()>) {
    if let Some(task) = task.0.take() {
        task.abort();
        if let Err(error) = task.await {
            assert!(error.is_cancelled());
        }
    }
}

async fn observe_hint(kind: Option<RoadKind>, historical_direct: bool, recursive: bool) {
    let mut f = Publication::new().await;
    assert!(f.node.hop_on() && f.node.carrying().is_some());
    assert!(!f.node.no_relay() && !f.target.no_relay);
    {
        let loaded = f.node.devices_loaded().unwrap();
        if let Some(kind) = kind {
            let mut road = Road::own(&f.target.id, kind, None, Some(50));
            if recursive {
                road.via = Some(iroh::SecretKey::generate().public().to_string());
            }
            loaded
                .doc
                .set_road(
                    &f.middle.id,
                    &f.target.id,
                    &serde_json::to_string(&road).unwrap(),
                )
                .unwrap();
        }
        if historical_direct {
            let me = f.node.device_str();
            loaded.doc.upsert(me, "phone", &[]).unwrap();
            let road = Road::own(&f.target.id, RoadKind::Direct, None, Some(1));
            loaded
                .doc
                .set_road(me, &f.target.id, &serde_json::to_string(&road).unwrap())
                .unwrap();
        }
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    assert_eq!(
        khor_net::path::classify(&f.outer_local),
        khor_net::path::PathKind::Direct
    );
    assert!(f.node.wires.live(&f.target.id, Proto::Main).is_none());
    assert!(f.node.wires.live(&f.target.id, Proto::Tunnel).is_none());
    assert!(!f.node.carry.has(&f.target_ep.id()));
    let (entered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut preparing = Owned(Some(tokio::spawn({
        let (node, ep, target) = (f.node.clone(), f.ep.clone(), f.target.clone());
        async move { node.hop_ahead(&ep, &target).await }
    })));
    let serial = tokio::time::timeout(Duration::from_secs(2), entered)
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
        tokio::time::timeout(Duration::from_secs(2), preparing.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
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
    if let Some(conn) = inner {
        // Close the inner connection while its owned carrier still works.
        conn.close(0u32.into(), b"sole-peer fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
    for task in [&mut preparing, &mut f.middle_task, &mut f.echo_task] {
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

    if recursive {
        assert!(
            serial.is_none() && !answered,
            "a recursive second leg is ineligible"
        );
    } else {
        assert!(
            serial.is_some() && answered,
            "the sole live first leg must get a bounded actual hop attempt: second_hint={kind:?}, historical_direct={historical_direct}, admitted={}, echoed={answered}",
            serial.is_some()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_sole_direct_neighbor_can_report_a_relay_second_leg() {
    observe_hint(Some(RoadKind::Relay), false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_missing_second_leg_hint_does_not_prevent_a_bounded_attempt() {
    observe_hint(None, false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_historical_direct_target_does_not_hide_a_reachable_middle() {
    observe_hint(Some(RoadKind::Direct), true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_known_direct_second_leg_still_connects_and_echoes() {
    observe_hint(Some(RoadKind::Direct), false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_second_leg_already_forwarded_by_another_peer_stays_ineligible() {
    observe_hint(Some(RoadKind::Direct), false, true).await;
}

#[path = "incoming_middle_tests.rs"]
mod incoming_middle;

#[path = "miss_epoch_tests.rs"]
mod miss_epoch;

#[path = "no_relay_hint_tests.rs"]
mod no_relay_hint;
