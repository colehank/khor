//! New pending consumers participate in the normal watcher's actual release.

use super::*;

fn answering(conn: iroh::endpoint::Connection) -> Owned<()> {
    Owned(Some(tokio::spawn(async move {
        while let Ok((mut send, mut recv)) = conn.accept_bi().await {
            let Ok(bytes) = recv.read_to_end(64).await else {
                break;
            };
            if send.write_all(&bytes).await.is_err() {
                break;
            }
            let _ = send.finish();
        }
    })))
}

async fn stop(task: &mut Owned<()>) {
    if let Some(task) = task.0.take() {
        task.abort();
        if let Err(error) = task.await {
            assert!(error.is_cancelled());
        }
    }
}

async fn observe_release(successor: bool) {
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
    assert_eq!(
        khor_net::path::classify(&outgoing),
        khor_net::path::PathKind::Direct
    );
    let (offered, release_offer) = f.node.wires.gate_hop_offered_for_test();
    let (judged, release_judgment, reviewed) = f.node.wires.gate_normal_release_for_test();
    let mut opening = f.opening();
    let serial = ready(offered).await;
    let remote = tokio::time::timeout(Duration::from_secs(2), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    release_offer.send(()).unwrap();
    assert!(finish(&mut opening).await);
    // Use the real two five-second looks, without advancing a fake clock
    // or weakening the existing measured comparison and dwell contract.
    assert_eq!(
        tokio::time::timeout(LOOK_EVERY * 3, judged)
            .await
            .unwrap()
            .unwrap(),
        serial
    );
    assert_eq!(
        f.node.wires.compare_carrier(&f.target.id),
        CarrierComparison::Unused
    );
    let pending = successor.then(|| f.node.wires.pending_dial(&f.target.id));
    let carried = if successor {
        let (client, incoming) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(
                f.target_ep.connect(
                    iroh::EndpointAddr::from_parts(f.ep.id(), [transport_addr(&f.ep.id())]),
                    INNER_ALPN,
                ),
                async { f.ep.accept().await.unwrap().accept().unwrap().await }
            )
        })
        .await
        .unwrap();
        let (client, incoming) = (client.unwrap(), incoming.unwrap());
        let answer = answering(incoming.clone());
        assert!(echo(&client).await);
        let facts = khor_net::path::path_facts(&incoming);
        assert!(!facts.is_empty() && facts.iter().all(|path| path.carried));
        assert_eq!(
            f.node.wires.compare_carrier(&f.target.id),
            CarrierComparison::Unused,
            "the new carried wire is genuinely not registered yet"
        );
        Some((client, incoming, answer))
    } else {
        None
    };
    release_judgment.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), reviewed)
        .await
        .unwrap()
        .unwrap();
    let kept = f.node.carry.has(&f.target_ep.id());
    let answered = match &carried {
        Some((client, _, _)) => echo(client).await,
        None => false,
    };
    if let Some((_, incoming, _)) = &carried {
        f.node.wires.serving(incoming);
    }
    drop(pending);
    let direct_echo = echo(&outgoing).await;
    let outer_open =
        f.outer_local.close_reason().is_none() && f.outer_remote.close_reason().is_none();
    if let Some((_, _, mut answer)) = carried {
        stop(&mut answer).await;
    }
    stop(&mut direct_answer).await;
    f.close(serial).await;
    direct.close().await;
    tokio::time::timeout(Duration::from_secs(2), remote.ended())
        .await
        .unwrap();
    assert!(direct_echo && outer_open);
    assert_eq!(
        kept, successor,
        "normal release must include the new caller"
    );
    assert_eq!(answered, successor);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_pending_carried_wire_survives_the_normal_release_decision() {
    observe_release(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_normal_release_without_a_pending_consumer_still_reclaims() {
    observe_release(false).await;
}

#[path = "unused_tests.rs"]
mod unused;
