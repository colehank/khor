//! A real new first leg is new evidence even when the middle's key is unchanged.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AddressChange {
    None,
    V4,
    V6,
}

async fn dual_publication() -> Publication {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    f.ep.close().await;
    f.ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(f.node.key.clone())
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .bind_addr("[::1]:0")
        .unwrap()
        .add_custom_transport(f.node.carry.transport())
        .path_selector(Arc::new(RoadOrder::default()))
        .alpns(vec![
            INNER_ALPN.to_vec(),
            khor_net::endpoint::TUNNEL_ALPN.to_vec(),
        ])
        .bind()
        .await
        .unwrap();
    let (local, remote) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            f.ep.connect(f.middle_ep.addr(), khor_net::endpoint::TUNNEL_ALPN),
            async { f.middle_ep.accept().await.unwrap().accept().unwrap().await }
        )
    })
    .await
    .unwrap();
    f.outer_local = local.unwrap();
    f.outer_remote = remote.unwrap();
    f.node
        .wires
        .adopt(&f.middle.id, Proto::Tunnel, f.outer_local.clone());
    f
}

async fn observe_address(change: AddressChange) {
    let mut f = dual_publication().await;
    set_hint(&f, &f.middle.id, 5);
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
    let mut first = preparing(&f);
    completed(&mut first).await;
    assert_eq!(asks.load(Ordering::SeqCst), 1);
    let old_wire = f.outer_local.stable_id();
    let old_addresses = f.middle_ep.addr().addrs;
    let mut replacement = None;
    let mut accepted = None;
    if change != AddressChange::None {
        stop(&mut f.middle_task).await;
        f.outer_local
            .close(0u32.into(), b"old middle address retired");
        f.outer_remote
            .close(0u32.into(), b"old middle address retired");
        let ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .secret_key(f.middle_ep.secret_key().clone())
            .relay_mode(iroh::RelayMode::Disabled)
            .clear_ip_transports()
            .bind_addr(if change == AddressChange::V6 {
                "[::1]:0"
            } else {
                "127.0.0.1:0"
            })
            .unwrap()
            .alpns(vec![khor_net::endpoint::TUNNEL_ALPN.to_vec()])
            .bind()
            .await
            .unwrap();
        assert_eq!(ep.id(), f.middle_ep.id());
        let addresses = ep.addr().addrs;
        assert_ne!(addresses, old_addresses);
        let (local, remote) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(
                f.ep.connect(ep.addr(), khor_net::endpoint::TUNNEL_ALPN),
                async { ep.accept().await.unwrap().accept().unwrap().await }
            )
        })
        .await
        .unwrap();
        let (local, remote) = (local.unwrap(), remote.unwrap());
        assert_ne!(local.stable_id(), old_wire);
        assert!(local.paths().iter().any(|p| {
            p.is_selected()
                && addresses.contains(p.remote_addr())
                && matches!(p.remote_addr(), iroh::TransportAddr::Ip(ip)
                    if ip.is_ipv6() == (change == AddressChange::V6))
        }));
        assert_eq!(
            khor_net::path::classify(&local),
            khor_net::path::PathKind::Direct
        );
        f.node
            .wires
            .adopt(&f.middle.id, Proto::Tunnel, local.clone());
        f.outer_local = local;
        f.outer_remote = remote;
        let (attached, ready) = tokio::sync::oneshot::channel();
        accepted = Some(ready);
        f.middle_task = Owned(Some(tokio::spawn({
            let conn = f.outer_remote.clone();
            let links = f.target_links.clone();
            let asks = asks.clone();
            let (me, via, to) = (f.ep.id(), ep.id(), f.target_ep.id());
            async move {
                let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                assert_eq!(
                    crate::tunnel::read_target(&mut recv).await.unwrap(),
                    Ask::To(to).words()
                );
                asks.fetch_add(1, Ordering::SeqCst);
                send.write_all(&[crate::tunnel::OK]).await.unwrap();
                attached
                    .send(links.attach(me, Carrier::Hop { via }, recv, send))
                    .unwrap();
            }
        })));
        replacement = Some(ep);
    }
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut next = preparing(&f);
    let serial = tokio::time::timeout(Duration::from_secs(2), offered)
        .await
        .ok()
        .and_then(Result::ok);
    let mut attached = None;
    let mut answered = false;
    if serial.is_some() {
        attached = Some(
            tokio::time::timeout(Duration::from_secs(2), accepted.as_mut().unwrap())
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
        conn.close(0u32.into(), b"address fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    } else {
        drop(release);
    }
    let count = asks.load(Ordering::SeqCst);
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    for task in [&mut first, &mut next, &mut f.middle_task, &mut f.echo_task] {
        stop(task).await;
    }
    if let Some(serial) = serial {
        let via = f.middle_ep.id();
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
    if let Some(ep) = replacement {
        ep.close().await;
    }
    tokio::join!(f.ep.close(), f.target_ep.close(), f.middle_ep.close());
    assert!(outer_open);
    let changed = change != AddressChange::None;
    assert_eq!(serial.is_some() && answered, changed);
    assert_eq!(count, if changed { 2 } else { 1 });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_real_new_ipv4_first_leg_with_the_same_middle_key_can_recover() {
    observe_address(AddressChange::V4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_real_ipv6_first_leg_with_the_same_middle_key_can_recover() {
    observe_address(AddressChange::V6).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unchanged_real_first_leg_keeps_the_cooldown() {
    observe_address(AddressChange::None).await;
}
