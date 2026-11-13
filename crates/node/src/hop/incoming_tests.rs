//! A live incoming carried-only wire may have no selected marker.

use super::*;

/// Deliberately withhold a preference for carried-only traffic. The
/// independent direct connection and the actual outer tunnel retain their
/// normal selection; no global environment or transport is mocked.
#[derive(Debug, Default)]
struct WithheldTargetSelection {
    normal: RoadOrder,
}

impl iroh::endpoint::transports::PathSelector for WithheldTargetSelection {
    fn select(
        &self,
        ctx: &iroh::endpoint::transports::PathSelectionContext<'_>,
    ) -> iroh::endpoint::transports::PathSelection {
        use iroh::endpoint::transports::FourTuple;
        if ctx
            .paths()
            .any(|path| matches!(path.network_path(), FourTuple::Custom { .. }))
        {
            iroh::endpoint::transports::PathSelection::none()
        } else {
            iroh::endpoint::transports::PathSelector::select(&self.normal, ctx)
        }
    }
}

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
    let Some(task) = task.0.take() else {
        return;
    };
    task.abort();
    if let Err(error) = task.await {
        assert!(error.is_cancelled());
    }
}

async fn observe_incoming(cancelled: bool) {
    let mut f = Publication::new_selecting(Arc::new(WithheldTargetSelection::default())).await;
    // Two fresh endpoints share only B's ephemeral identity. Their actual
    // independent connections give A one IP tuple and one carried-only
    // tuple, as in the original shared-carrier Wires regression.
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
    // A separate endpoint with A's same ephemeral identity supplies the
    // independent selected IP connection. Wires must preserve an incoming
    // carried consumer even when its preferred-route observation is absent.
    let direct_client = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(f.ep.secret_key().clone())
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .alpns(vec![INNER_ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    assert_eq!(direct_client.id(), f.ep.id());
    let (outgoing, direct_incoming) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(direct_client.connect(direct.addr(), INNER_ALPN), async {
            direct.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (outgoing, direct_incoming) = (outgoing.unwrap(), direct_incoming.unwrap());
    let mut direct_answer = answering(direct_incoming);
    assert!(echo(&outgoing).await);
    f.node
        .wires
        .adopt(&f.target.id, Proto::Main, outgoing.clone());
    assert_eq!(
        khor_net::path::classify(&outgoing),
        khor_net::path::PathKind::Direct
    );
    assert!(
        outgoing
            .paths()
            .iter()
            .any(|path| { path.id() == iroh::endpoint::PathId::ZERO && path.is_ip() })
    );

    let (entered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut opening = f.opening();
    let serial = ready(entered).await;
    let remote_attached = tokio::time::timeout(Duration::from_secs(5), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    let (client, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
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
    let mut carried_answer = answering(incoming.clone());
    assert!(echo(&client).await);
    f.node.wires.serving(&incoming);
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let facts = khor_net::path::path_facts(&incoming);
            if !facts.is_empty()
                && facts.iter().all(|path| path.carried && !path.selected)
                && khor_net::path::classify(&outgoing) == khor_net::path::PathKind::Direct
                && khor_net::path::classify(&client) == khor_net::path::PathKind::Carried
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("real incoming Custom-only and outgoing IP prerequisites");

    if cancelled {
        cancel(&mut opening).await;
        drop(release);
    } else {
        release.send(()).unwrap();
        assert!(finish(&mut opening).await);
    }
    let kept = f.node.carry.has(&f.target_ep.id());
    let owned = asking(f.node.device_str(), &f.target.id);
    let carried_echo = echo(&client).await;
    let direct_echo = echo(&outgoing).await;
    stop(&mut carried_answer).await;
    stop(&mut direct_answer).await;
    f.close(serial).await;
    tokio::join!(direct_client.close(), direct.close());
    tokio::time::timeout(Duration::from_secs(2), remote_attached.ended())
        .await
        .unwrap();

    assert!(direct_echo, "the independent IP connection stays usable");
    assert!(
        kept && owned && carried_echo,
        "the real incoming wire uses the carrier without a selected marker: kept={kept}, owned={owned}, echoed={carried_echo}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_publication_keeps_a_live_incoming_carried_only_wire() {
    observe_incoming(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn normal_publication_keeps_a_live_incoming_carried_only_wire() {
    observe_incoming(false).await;
}
