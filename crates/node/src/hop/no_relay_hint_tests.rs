//! Exercise missing roadbook hints after either endpoint reports no relay.
//! Publication supplies real Tunnel admission, inner TLS and echo.
use super::*;
use iroh::Watcher;

#[derive(Clone, Copy)]
enum Hint {
    Missing,
    Direct,
    Recursive,
}

async fn observe(local_no_relay: bool, target_no_relay: bool, hint: Hint) {
    let mut f = Publication::new().await;
    if local_no_relay {
        assert!(
            f.ep.home_relay_status()
                .get()
                .iter()
                .all(|r| !r.is_connected())
        );
        let now = std::time::Instant::now();
        let dark = crate::egress::Seen {
            home: false,
            answered: false,
        };
        let mut egress = f.node.egress.lock().unwrap();
        // Feed the established absence window through the normal state logic.
        assert_eq!(egress.saw(dark, now - crate::egress::ENTER_AFTER), None);
        assert_eq!(egress.saw(dark, now), Some(true));
    }
    f.target.no_relay = target_no_relay;
    {
        let loaded = f.node.devices_loaded().unwrap();
        loaded
            .doc
            .set_no_relay(&f.target.id, target_no_relay)
            .unwrap();
        if !matches!(hint, Hint::Missing) {
            let mut road = Road::own(&f.target.id, RoadKind::Direct, None, Some(50));
            if matches!(hint, Hint::Recursive) {
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
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    assert_eq!(f.node.no_relay(), local_no_relay);
    assert_eq!(
        khor_net::path::classify(&f.outer_local),
        khor_net::path::PathKind::Direct
    );
    assert!(f.node.wires.live(&f.target.id, Proto::Main).is_none());
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
    let mut answered = false;
    if let Some(serial) = serial {
        let attached = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
            .await
            .unwrap()
            .unwrap();
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
        conn.close(0u32.into(), b"no relay hint fixture complete");
        f.close(serial).await;
        tokio::time::timeout(Duration::from_secs(2), attached.ended())
            .await
            .unwrap();
    } else {
        drop(release);
        for task in [&mut preparing, &mut f.middle_task, &mut f.echo_task] {
            stop(task).await;
        }
        tokio::join!(f.ep.close(), f.middle_ep.close(), f.target_ep.close());
    }
    let eligible = !matches!(hint, Hint::Recursive);
    assert_eq!(
        serial.is_some(),
        eligible,
        "bounded admission with local_no_relay={local_no_relay}, target_no_relay={target_no_relay}"
    );
    assert_eq!(
        answered, eligible,
        "admitted carrier must actually complete inner TLS and echo"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_caller_without_relay_tries_a_missing_second_leg_hint() {
    observe(true, false, Hint::Missing).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_target_without_relay_allows_a_missing_second_leg_hint() {
    observe(false, true, Hint::Missing).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_peers_without_relay_try_the_reachable_middle() {
    observe(true, true, Hint::Missing).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_known_direct_second_leg_still_works_without_local_relay() {
    observe(true, false, Hint::Direct).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_recursive_second_leg_stays_ineligible_without_target_relay() {
    observe(false, true, Hint::Recursive).await;
}
