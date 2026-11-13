//! A real refused dial completing after a fresh incoming connection.

use super::*;
use std::future::Future;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::Poll;

#[derive(Clone, Copy)]
enum Contact {
    SamePeer,
    OtherPeer,
    None,
}

async fn endpoint() -> iroh::Endpoint {
    let ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .bind()
        .await
        .unwrap();
    let addr = ep.addr();
    assert!(!addr.addrs.is_empty());
    assert!(addr.addrs.iter().all(|addr| match addr {
        iroh::TransportAddr::Ip(addr) => addr.ip().is_loopback(),
        _ => false,
    }));
    ep
}

async fn echo(conn: &iroh::endpoint::Connection) {
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    send.write_all(b"peer-contact").await.unwrap();
    send.finish().unwrap();
    assert_eq!(recv.read_to_end(64).await.unwrap(), b"peer-contact");
}

async fn answer_echo(conn: &iroh::endpoint::Connection) {
    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
    let bytes = recv.read_to_end(64).await.unwrap();
    assert_eq!(bytes, b"peer-contact");
    send.write_all(&bytes).await.unwrap();
    send.finish().unwrap();
}

async fn ask(node: &Node, ep: &iroh::Endpoint, peer: &DeviceInfo, early: bool) -> bool {
    let conn = if early {
        match node.ask_early(ep, peer, &Request::Sessions, None).await {
            Early::Failed(DialFailure::Asleep(_)) => return false,
            Early::NotTaken => node.wires.live(&peer.id, wires::Proto::Main).unwrap(),
            _ => panic!("the cold handshake must either be refused or publish its wire"),
        }
    } else {
        match node
            .line_to_inner(ep, peer, wires::Proto::Main, None, false)
            .await
        {
            Err(DialFailure::Asleep(_)) => return false,
            Ok(line) => line.conn,
            _ => panic!("the actual loopback dial must be refused or succeed"),
        }
    };
    echo(&conn).await;
    true
}

async fn observe(early: bool, contact: Contact) {
    let server = endpoint().await;
    let other = endpoint().await;
    let (_root, node, mut peer, initial_ep) = fixture_for(server.id().to_string()).await;
    initial_ep.close().await;
    let ep = endpoint().await;
    peer.addrs = server
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
            _ => None,
        })
        .collect();
    assert!(!peer.addrs.is_empty());
    let node = Arc::new(node);
    let dials = Arc::new(AtomicUsize::new(0));
    let mut serving = Aborts(Some(tokio::spawn({
        let server = server.clone();
        let dials = dials.clone();
        async move {
            let incoming = server.accept().await.unwrap();
            dials.fetch_add(1, Ordering::SeqCst);
            incoming.refuse();
            let incoming = server.accept().await.unwrap();
            dials.fetch_add(1, Ordering::SeqCst);
            let conn = incoming.accept().unwrap().await.unwrap();
            answer_echo(&conn).await;
            conn.closed().await;
        }
    })));
    let (ready, release) = node.wires.gate_asleep_dial_for_test();
    let mut owner = Aborts(Some(tokio::spawn({
        let (node, ep, peer) = (node.clone(), ep.clone(), peer.clone());
        async move { ask(&node, &ep, &peer, early).await }
    })));
    tokio::time::timeout(Duration::from_secs(5), ready)
        .await
        .unwrap()
        .unwrap();
    // Poll the actual second caller while the first still owns the dial
    // mutex; it must be a waiter, not a later independent attempt.
    let mut queued = Box::pin(ask(&node, &ep, &peer, early));
    std::future::poll_fn(|cx| {
        assert!(queued.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let contact_wires = match contact {
        Contact::None => None,
        Contact::SamePeer | Contact::OtherPeer => {
            let caller = match contact {
                Contact::SamePeer => &server,
                Contact::OtherPeer => &other,
                Contact::None => unreachable!(),
            };
            let (outbound, inbound) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(caller.connect(ep.addr(), ALPN), async {
                    ep.accept().await.unwrap().accept().unwrap().await
                })
            })
            .await
            .unwrap();
            let (outbound, inbound) = (outbound.unwrap(), inbound.unwrap());
            assert_eq!(inbound.remote_id(), caller.id());
            node.wires.serving(&inbound);
            tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(echo(&outbound), answer_echo(&inbound));
            })
            .await
            .unwrap();
            assert!(!node.wires.left_alone(&peer.id));
            Some((outbound, inbound))
        }
    };
    release.send(()).unwrap();
    let owner_answered = tokio::time::timeout(Duration::from_secs(5), owner.0.as_mut().unwrap())
        .await
        .unwrap()
        .unwrap();
    let asleep_after_old_result = node.wires.left_alone(&peer.id);
    let queued_answered = tokio::time::timeout(Duration::from_secs(5), &mut queued)
        .await
        .unwrap();
    let dial_count = dials.load(Ordering::SeqCst);
    let down = node.wires.link(&peer.id).down;
    let backoff = node.wires.backing_off(&peer.id);
    drop(queued);
    drop(contact_wires);
    serving.0.as_ref().unwrap().abort();
    match serving.0.as_mut().unwrap().await {
        Ok(()) => {}
        Err(error) => assert!(error.is_cancelled(), "the scratch server must not panic"),
    }
    ep.close().await;
    server.close().await;
    other.close().await;

    assert!(!owner_answered, "the old caller keeps its actual refusal");
    assert_eq!(down, None);
    assert_eq!(backoff, None);
    if matches!(contact, Contact::SamePeer) {
        assert!(
            !asleep_after_old_result,
            "old asleep dial must not undo real contact; queued_answered={queued_answered}, dials={dial_count}"
        );
        assert!(queued_answered, "the waiter must not reuse stale Asleep");
        assert_eq!(dial_count, 2);
    } else {
        assert!(asleep_after_old_result);
        assert!(!queued_answered);
        assert_eq!(
            dial_count, 1,
            "unchanged peer stays quiet and shares refusal"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_dial_cannot_overwrite_fresh_peer_contact() {
    observe(false, Contact::SamePeer).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn early_dial_cannot_overwrite_fresh_peer_contact() {
    observe(true, Contact::SamePeer).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_refusal_without_fresh_contact_is_shared() {
    observe(false, Contact::None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn early_refusal_without_fresh_contact_is_shared() {
    observe(true, Contact::None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_refusal_is_not_invalidated_by_another_peer() {
    observe(false, Contact::OtherPeer).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn early_refusal_is_not_invalidated_by_another_peer() {
    observe(true, Contact::OtherPeer).await;
}
