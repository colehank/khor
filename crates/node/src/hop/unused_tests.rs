//! A normal publisher cannot leave an unused carrier after its wire goes away.

use super::*;

async fn two_looks(looks: &mut tokio::sync::mpsc::UnboundedReceiver<u64>, serial: u64) {
    tokio::time::timeout(LOOK_EVERY * 3, async {
        assert_eq!(looks.recv().await, Some(serial));
        assert_eq!(looks.recv().await, Some(serial));
    })
    .await
    .unwrap();
}

async fn observe_unused(pending: bool) {
    let mut f = Publication::new().await;
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
    let (outgoing, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(f.ep.connect(direct.addr(), INNER_ALPN), async {
            direct.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (outgoing, incoming) = (outgoing.unwrap(), incoming.unwrap());
    let mut direct_answer = answering(incoming);
    assert!(echo(&outgoing).await);
    f.node
        .wires
        .adopt(&f.target.id, Proto::Main, outgoing.clone());
    let (offered, release_offer) = f.node.wires.gate_hop_offered_for_test();
    let mut looks = f.node.wires.observe_hop_looks_for_test();
    let mut opening = f.opening();
    let serial = ready(offered).await;
    let remote = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    release_offer.send(()).unwrap();
    assert!(finish(&mut opening).await);
    // This new caller starts after normal publication's handoff. It is
    // not an owner the handoff already subscribed to at publication.
    let using = pending.then(|| f.node.wires.pending_dial(&f.target.id));
    f.node
        .wires
        .retire_peer(&f.target.id, crate::wires::Retired::Closed);
    assert!(outgoing.close_reason().is_some());
    assert!(f.node.wires.link(&f.target.id).road.is_none());
    assert_eq!(
        f.node.wires.compare_carrier(&f.target.id),
        CarrierComparison::Unused
    );
    two_looks(&mut looks, serial).await;
    let kept_during = f.node.carry.has(&f.target_ep.id());
    drop(using);
    if pending {
        two_looks(&mut looks, serial).await;
    }
    let kept_after = f.node.carry.has(&f.target_ep.id());
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    stop(&mut direct_answer).await;
    f.close(serial).await;
    direct.close().await;
    tokio::time::timeout(Duration::from_secs(2), remote.ended())
        .await
        .unwrap();
    assert!(outer_open);
    assert_eq!(
        kept_during, pending,
        "only a finite pending owner should retain the now-unused carrier"
    );
    assert!(!kept_after, "unused carrier retained");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_normal_prepared_carrier_is_reclaimed_after_its_only_wire_closes() {
    observe_unused(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_new_pending_owner_is_kept_until_it_ends_then_the_unused_carrier_releases() {
    observe_unused(true).await;
}
