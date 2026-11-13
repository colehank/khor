//! A currently reachable middle may only have an incoming authenticated wire.

use super::*;

async fn observe_incoming_middle(available: bool) {
    let mut f = Publication::new().await;
    stop(&mut f.middle_task).await;
    f.node
        .wires
        .retire_peer(&f.middle.id, crate::wires::Retired::Closed);
    assert!(f.outer_local.close_reason().is_some());
    let (client, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            f.middle_ep
                .connect(f.ep.addr(), khor_net::endpoint::TUNNEL_ALPN),
            async { f.ep.accept().await.unwrap().accept().unwrap().await }
        )
    })
    .await
    .unwrap();
    let (client, incoming) = (client.unwrap(), incoming.unwrap());
    let mut reverse_answer = Owned(Some(tokio::spawn({
        let conn = incoming.clone();
        async move {
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
    assert!(echo(&client).await);
    assert_eq!(
        khor_net::path::classify(&incoming),
        khor_net::path::PathKind::Direct
    );
    f.node.wires.serving(&incoming);
    assert!(f.node.wires.live(&f.middle.id, Proto::Tunnel).is_none());
    assert!(f.node.wires.live(&f.middle.id, Proto::Main).is_none());
    assert!(f.node.wires.link(&f.middle.id).road.is_none());
    assert!(f.middle.addrs.is_empty());
    if !available {
        client.close(0u32.into(), b"incoming middle control ended");
        incoming.close(0u32.into(), b"incoming middle control ended");
    }
    {
        let loaded = f.node.devices_loaded().unwrap();
        let road = Road::own(&f.target.id, RoadKind::Direct, None, Some(5));
        loaded
            .doc
            .set_road(
                &f.middle.id,
                &f.target.id,
                &serde_json::to_string(&road).unwrap(),
            )
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    let (attached, attached_target) = tokio::sync::oneshot::channel();
    f.attached_target = attached_target;
    f.middle_task = Owned(Some(tokio::spawn({
        let ep = f.middle_ep.clone();
        let links = f.target_links.clone();
        let (me, via, to) = (f.ep.id(), f.middle_ep.id(), f.target_ep.id());
        async move {
            // A reverse authenticated wire proves a road, but does not
            // imply the peer serves client requests on its outgoing wire.
            // The node still has to establish its actual Tunnel protocol.
            let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
            assert_eq!(conn.remote_id(), me);
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();
            assert_eq!(
                crate::tunnel::read_target(&mut recv).await.unwrap(),
                Ask::To(to).words()
            );
            send.write_all(&[crate::tunnel::OK]).await.unwrap();
            let up = links.attach(me, Carrier::Hop { via }, recv, send);
            let _ = attached.send(up);
        }
    })));
    let (offered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut preparing = Owned(Some(tokio::spawn({
        let (node, ep, target) = (f.node.clone(), f.ep.clone(), f.target.clone());
        async move { node.hop_ahead(&ep, &target).await }
    })));
    let serial = tokio::time::timeout(Duration::from_secs(2), offered)
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
    let reverse_open = client.close_reason().is_none() && incoming.close_reason().is_none();
    if let Some(conn) = inner {
        conn.close(0u32.into(), b"incoming middle fixture complete");
        tokio::time::timeout(Duration::from_secs(2), f.echo_task.0.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
    for task in [
        &mut preparing,
        &mut f.middle_task,
        &mut f.echo_task,
        &mut reverse_answer,
    ] {
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
    assert_eq!(reverse_open, available);
    assert_eq!(
        serial.is_some() && answered,
        available,
        "a live incoming first leg must be eligible; a closed one must not"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_sole_incoming_middle_can_prepare_an_actual_tunnel_and_carried_wire() {
    observe_incoming_middle(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_closed_incoming_wire_does_not_make_a_middle_reachable() {
    observe_incoming_middle(false).await;
}
