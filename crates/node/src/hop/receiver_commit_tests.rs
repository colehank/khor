//! Admission alone cannot replace a carried link that already carries traffic.
use super::*;

async fn admitted_from(
    f: &Publication,
    local: &iroh::endpoint::Connection,
    remote: &iroh::endpoint::Connection,
    via: EndpointId,
) -> (Owned<()>, SendStream, RecvStream) {
    let (mut send, mut recv) = remote.open_bi().await.unwrap();
    // A real request is parsed at the same boundary as the resident.
    let target = Ask::From(f.target_ep.id()).words();
    send.write_all(&(target.len() as u16).to_be_bytes())
        .await
        .unwrap();
    send.write_all(target.as_bytes()).await.unwrap();
    let (incoming_send, mut incoming_recv) =
        tokio::time::timeout(Duration::from_secs(2), local.accept_bi())
            .await
            .unwrap()
            .unwrap();
    let words = crate::tunnel::read_target(&mut incoming_recv)
        .await
        .unwrap();
    let ask = Ask::of(&words).unwrap();
    let task = Owned(Some(tokio::spawn({
        let (node, ep, weak, from) = (
            f.node.clone(),
            Arc::new(f.ep.clone()),
            local.weak_handle(),
            via.to_string(),
        );
        async move {
            node.serve_hop(ep, &from, weak, ask, incoming_send, incoming_recv, true)
                .await;
        }
    })));
    let mut status = [0];
    tokio::time::timeout(Duration::from_secs(2), recv.read_exact(&mut status))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status[0], OK);
    (task, send, recv)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_admitted_loser_without_payload_cannot_replace_a_working_receiver_link() {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    stop(&mut f.echo_task).await;
    let mut later = ReachableMiddle::new(&f, true).await;
    stop(&mut later.task).await;
    {
        let loaded = f.node.devices_loaded().unwrap();
        f.node
            .enroll_after_pair(&loaded.doc, &[(&f.target.id, "target")])
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    assert!(f.node.admit(&f.target.id).is_ok());
    f.echo_task = Owned(Some(tokio::spawn({
        let (ep, node, id) = (f.ep.clone(), f.node.clone(), f.target.id.clone());
        async move {
            let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
            node.wires.adopt(&id, Proto::Main, conn.clone());
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
    let first_via = f.middle_ep.id();
    let (mut first, send, recv) =
        admitted_from(&f, &f.outer_local, &f.outer_remote, first_via).await;
    let attached = f
        .target_links
        .attach(f.ep.id(), Carrier::Hop { via: first_via }, recv, send);
    let conn = tokio::time::timeout(
        Duration::from_secs(5),
        f.target_ep.connect(
            iroh::EndpointAddr::from_parts(f.ep.id(), [transport_addr(&f.ep.id())]),
            INNER_ALPN,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        khor_net::path::classify(&conn),
        khor_net::path::PathKind::Carried
    );
    assert!(echo(&conn).await);
    let (mut uncommitted, send, recv) =
        admitted_from(&f, &later.local, &later.incoming, later.ep.id()).await;
    let still_first =
        f.node.carry.carrier(&f.target_ep.id()) == Some(Carrier::Hop { via: first_via });
    let while_prepared = echo(&conn).await;
    drop((send, recv));
    tokio::time::timeout(Duration::from_secs(2), uncommitted.0.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    let after_cancel = echo(&conn).await;
    let kept_first =
        f.node.carry.carrier(&f.target_ep.id()) == Some(Carrier::Hop { via: first_via });
    let outer_open = f.outer_local.close_reason().is_none()
        && later.local.close_reason().is_none()
        && later.incoming.close_reason().is_none();
    conn.close(0u32.into(), b"receiver commit fixture complete");
    f.node
        .carry
        .detach(&f.target_ep.id(), Carrier::Hop { via: first_via });
    f.target_links.detach_attached(
        &f.ep.id(),
        Carrier::Hop { via: first_via },
        attached.serial(),
    );
    tokio::time::timeout(Duration::from_secs(2), attached.ended())
        .await
        .unwrap();
    for task in [&mut first, &mut f.echo_task] {
        stop(task).await;
    }
    tokio::join!(
        f.ep.close(),
        f.target_ep.close(),
        f.middle_ep.close(),
        later.ep.close()
    );
    assert!(outer_open);
    assert!(
        still_first && while_prepared && after_cancel && kept_first,
        "a status-only loser must not publish over or cancel the carrier that completed actual TLS and traffic"
    );
}
