//! Admission alone cannot replace a carried link that already carries traffic.
use super::*;

async fn admitted_from(
    f: &Publication,
    local: &iroh::endpoint::Connection,
    remote: &iroh::endpoint::Connection,
    via: EndpointId,
    switching: bool,
) -> (Owned<()>, SendStream, RecvStream) {
    let (mut send, mut recv) = remote.open_bi().await.unwrap();
    // A real request is parsed at the same boundary as the resident.
    let target = if switching {
        Ask::SwitchFrom(f.target_ep.id())
    } else {
        Ask::From(f.target_ep.id())
    }
    .words();
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
    if switching {
        let probe = [0xf4; SWITCH_PROBE_BYTES];
        send.write_all(&probe).await.unwrap();
        let mut reply = [0; SWITCH_PROBE_BYTES];
        tokio::time::timeout(Duration::from_secs(2), recv.read_exact(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(probe, reply);
    }
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
        admitted_from(&f, &f.outer_local, &f.outer_remote, first_via, false).await;
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
    for switching in [false, true] {
        let (mut uncommitted, send, recv) =
            admitted_from(&f, &later.local, &later.incoming, later.ep.id(), switching).await;
        assert_eq!(
            f.node.carry.carrier(&f.target_ep.id()),
            Some(Carrier::Hop { via: first_via })
        );
        assert!(echo(&conn).await);
        drop((send, recv));
        tokio::time::timeout(Duration::from_secs(2), uncommitted.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(echo(&conn).await);
        assert_eq!(
            f.node.carry.carrier(&f.target_ep.id()),
            Some(Carrier::Hop { via: first_via })
        );
    }
    let next_via = later.ep.id();
    let (mut second, send, recv) =
        admitted_from(&f, &later.local, &later.incoming, next_via, true).await;
    let next_carrier = Carrier::Hop { via: next_via };
    let replacement = f.target_links.attach(f.ep.id(), next_carrier, recv, send);
    assert!(echo(&conn).await);
    assert_eq!(f.node.carry.carrier(&f.target_ep.id()), Some(next_carrier));
    let outer_open = f.outer_local.close_reason().is_none()
        && later.local.close_reason().is_none()
        && later.incoming.close_reason().is_none();
    conn.close(0u32.into(), b"receiver commit fixture complete");
    f.node.carry.detach(&f.target_ep.id(), next_carrier);
    f.target_links
        .detach_attached(&f.ep.id(), next_carrier, replacement.serial());
    tokio::time::timeout(Duration::from_secs(2), attached.ended())
        .await
        .unwrap();
    for task in [&mut first, &mut second, &mut f.echo_task] {
        stop(task).await;
    }
    tokio::join!(
        f.ep.close(),
        f.target_ep.close(),
        f.middle_ep.close(),
        later.ep.close()
    );
    assert!(outer_open);
}
