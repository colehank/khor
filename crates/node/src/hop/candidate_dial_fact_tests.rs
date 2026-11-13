//! A tunnel created by the failing attempt is not a new external route.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refusal_on_a_newly_dialled_tunnel_keeps_its_cooldown() {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    f.outer_local
        .close(0u32.into(), b"fixture starts with main only");
    f.outer_remote
        .close(0u32.into(), b"fixture starts with main only");
    let middle = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(f.middle_ep.secret_key().clone())
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .alpns(vec![
            INNER_ALPN.to_vec(),
            khor_net::endpoint::TUNNEL_ALPN.to_vec(),
        ])
        .bind()
        .await
        .unwrap();
    let hints: Vec<_> = middle
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(ip) => Some(ip.to_string()),
            _ => None,
        })
        .collect();
    {
        let loaded = f.node.devices_loaded().unwrap();
        loaded.doc.upsert(&f.middle.id, "middle", &hints).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    set_hint(&f, &f.middle.id, 5);
    let (main, remote) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(f.ep.connect(middle.addr(), INNER_ALPN), async {
            middle.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (main, remote) = (main.unwrap(), remote.unwrap());
    f.node.wires.adopt(&f.middle.id, Proto::Main, main.clone());
    assert!(f.node.wires.live(&f.middle.id, Proto::Tunnel).is_none());
    let network = f.node.wires.hop_epoch();
    let asks = Arc::new(AtomicUsize::new(0));
    f.middle_task = Owned(Some(tokio::spawn({
        let ep = middle.clone();
        let asks = asks.clone();
        let to = f.target_ep.id();
        async move {
            let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
            while let Ok(stream) = conn.accept_bi().await {
                reject(stream, to, &asks).await;
            }
        }
    })));
    let mut first = preparing(&f);
    completed(&mut first).await;
    assert_eq!(asks.load(Ordering::SeqCst), 1);
    let tunnel = f.node.wires.live(&f.middle.id, Proto::Tunnel).unwrap();
    assert_ne!(main.stable_id(), tunnel.stable_id());
    let mut next = preparing(&f);
    completed(&mut next).await;
    let count = asks.load(Ordering::SeqCst);
    let current = f.node.wires.live(&f.middle.id, Proto::Tunnel).unwrap();
    let same_tunnel = current.stable_id() == tunnel.stable_id();
    assert!(network.ptr_eq(&f.node.wires.hop_epoch()));
    let open = main.close_reason().is_none() && tunnel.close_reason().is_none();
    for task in [&mut first, &mut next, &mut f.middle_task, &mut f.echo_task] {
        stop(task).await;
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while asking(f.node.device_str(), &f.target.id) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    main.close(0u32.into(), b"dial fact fixture complete");
    remote.close(0u32.into(), b"dial fact fixture complete");
    middle.close().await;
    tokio::join!(f.ep.close(), f.target_ep.close(), f.middle_ep.close());
    assert!(open && same_tunnel);
    assert_eq!(
        count, 1,
        "this attempt's own dial must not forgive its refusal"
    );
}
