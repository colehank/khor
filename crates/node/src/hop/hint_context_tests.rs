//! Effective new dialing hints are evidence; cosmetic source edits are not.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HintChange {
    TargetV4,
    TargetV6,
    MiddleV4,
    MiddleV6,
    TargetOrder,
    MiddleOrder,
}

fn write_hints(f: &mut Publication, target: bool, addrs: Vec<String>) {
    let id = if target {
        f.target.id.clone()
    } else {
        f.middle.id.clone()
    };
    let loaded = f.node.devices_loaded().unwrap();
    let name = loaded.doc.get(&id).unwrap().name;
    loaded.doc.upsert(&id, &name, &addrs).unwrap();
    let mut store = loaded.store;
    store.flush(&loaded.doc).unwrap();
    if target {
        f.target.addrs = addrs;
    }
}

async fn observe_hint(change: HintChange) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    set_hint(&f, &f.middle.id, 5);
    let target = matches!(
        change,
        HintChange::TargetV4 | HintChange::TargetV6 | HintChange::TargetOrder
    );
    let v6 = matches!(change, HintChange::TargetV6 | HintChange::MiddleV6);
    let cosmetic = matches!(change, HintChange::TargetOrder | HintChange::MiddleOrder);
    let key = if target {
        f.target_ep.secret_key().clone()
    } else {
        f.middle_ep.secret_key().clone()
    };
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(key)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(if v6 { "[::1]:0" } else { "127.0.0.1:0" })
        .unwrap()
        .alpns(vec![khor_net::endpoint::TUNNEL_ALPN.to_vec()]);
    if cosmetic {
        builder = builder.bind_addr("[::1]:0").unwrap();
    }
    let alias = builder.bind().await.unwrap();
    assert_eq!(
        alias.id().to_string(),
        if target {
            f.target.id.clone()
        } else {
            f.middle.id.clone()
        }
    );
    let hints: Vec<_> = alias
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(ip) => Some(ip.to_string()),
            _ => None,
        })
        .collect();
    assert!(!hints.is_empty());
    // Authenticate the real newly published endpoint with a separate probe;
    // never replace the initiator's original first-leg registry or wire.
    let probe = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr(if v6 { "[::1]:0" } else { "127.0.0.1:0" })
        .unwrap()
        .alpns(vec![khor_net::endpoint::TUNNEL_ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    let (local, remote) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            probe.connect(alias.addr(), khor_net::endpoint::TUNNEL_ALPN),
            async { alias.accept().await.unwrap().accept().unwrap().await },
        )
    })
    .await
    .unwrap();
    let (local, remote) = (local.unwrap(), remote.unwrap());
    assert_eq!(local.remote_id(), alias.id());
    assert!(local.paths().iter().any(|path| path.is_selected()
        && matches!(path.remote_addr(), iroh::TransportAddr::Ip(ip) if ip.is_ipv6() == v6)));
    local.close(0u32.into(), b"hint precondition authenticated");
    remote.close(0u32.into(), b"hint precondition authenticated");
    probe.close().await;
    if cosmetic {
        write_hints(&mut f, target, hints.clone());
    }
    let network = f.node.wires.hop_epoch();
    let wire = f.outer_local.stable_id();
    let asks = Arc::new(AtomicUsize::new(0));
    f.middle_task = Owned(Some(tokio::spawn({
        let (conn, to, asks) = (f.outer_remote.clone(), f.target_ep.id(), asks.clone());
        async move {
            while let Ok(stream) = conn.accept_bi().await {
                reject(stream, to, &asks).await;
            }
        }
    })));
    let mut first = preparing(&f);
    completed(&mut first).await;
    assert_eq!(asks.load(Ordering::SeqCst), 1);
    let mut published = hints.clone();
    if cosmetic {
        published.reverse();
        published.extend(hints);
        published.push("not a transport hint".to_owned());
        published.push("[invalid]:port".to_owned());
    }
    write_hints(&mut f, target, published);
    let mut accepted = None;
    if !cosmetic {
        stop(&mut f.middle_task).await;
        let (attached, ready) = tokio::sync::oneshot::channel();
        accepted = Some(ready);
        f.middle_task = Owned(Some(tokio::spawn({
            let (conn, links, asks) =
                (f.outer_remote.clone(), f.target_links.clone(), asks.clone());
            let (me, via, to) = (f.ep.id(), f.middle_ep.id(), f.target_ep.id());
            async move {
                let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                assert_eq!(
                    crate::tunnel::read_target(&mut recv).await.unwrap(),
                    Ask::To(to).words()
                );
                asks.fetch_add(1, Ordering::SeqCst);
                send.write_all(&[OK]).await.unwrap();
                attached
                    .send(links.attach(me, Carrier::Hop { via }, recv, send))
                    .unwrap();
            }
        })));
    }
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    assert_eq!(wire, f.outer_local.stable_id());
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
        conn.close(0u32.into(), b"hint fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    } else {
        drop(release);
    }
    let count = asks.load(Ordering::SeqCst);
    let same_outer = wire == f.outer_local.stable_id() && f.outer_local.close_reason().is_none();
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
    tokio::join!(
        f.ep.close(),
        f.target_ep.close(),
        f.middle_ep.close(),
        alias.close()
    );
    assert!(same_outer);
    assert_eq!(
        serial.is_some() && answered,
        !cosmetic,
        "new effective hints must admit real carried traffic without replacing the first leg; cosmetic edits keep cooldown"
    );
    assert_eq!(count, if cosmetic { 1 } else { 2 });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn new_target_ipv4_hint_can_recover_on_the_same_first_leg() {
    observe_hint(HintChange::TargetV4).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn new_target_ipv6_hint_can_recover_on_the_same_first_leg() {
    observe_hint(HintChange::TargetV6).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn new_middle_ipv4_hint_can_recover_on_the_same_first_leg() {
    observe_hint(HintChange::MiddleV4).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn new_middle_ipv6_hint_can_recover_on_the_same_first_leg() {
    observe_hint(HintChange::MiddleV6).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cosmetic_target_hints_keep_the_existing_cooldown() {
    observe_hint(HintChange::TargetOrder).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cosmetic_middle_hints_keep_the_existing_cooldown() {
    observe_hint(HintChange::MiddleOrder).await;
}
