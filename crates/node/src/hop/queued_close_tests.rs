//! The carrier changes after a close was enqueued to a paused real actor.

use super::*;
use futures::StreamExt as _;
use iroh::endpoint::transports::{PathSelection, PathSelectionContext, PathSelector};
use std::sync::{Condvar, Mutex};

#[derive(Debug, Default)]
struct Resume {
    released: Mutex<bool>,
    changed: Condvar,
}

impl Resume {
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

#[derive(Debug)]
struct Waiting {
    entered: tokio::sync::oneshot::Sender<()>,
    resume: Arc<Resume>,
}

struct Paused {
    entered: tokio::sync::oneshot::Receiver<()>,
    resume: Arc<Resume>,
}

impl Drop for Paused {
    fn drop(&mut self) {
        self.resume.release();
    }
}

#[derive(Debug, Default)]
struct PausingOrder {
    road: RoadOrder,
    waiting: Mutex<Option<Waiting>>,
}

impl PausingOrder {
    fn pause(&self) -> Paused {
        let (entered, ready) = tokio::sync::oneshot::channel();
        let resume = Arc::new(Resume::default());
        assert!(
            self.waiting
                .lock()
                .unwrap()
                .replace(Waiting {
                    entered,
                    resume: resume.clone(),
                })
                .is_none()
        );
        Paused {
            entered: ready,
            resume,
        }
    }
}

impl PathSelector for PausingOrder {
    fn select(&self, ctx: &PathSelectionContext<'_>) -> PathSelection {
        let waiting = self.waiting.lock().unwrap().take();
        if let Some(waiting) = waiting {
            let _ = waiting.entered.send(());
            let mut released = waiting.resume.released.lock().unwrap();
            while !*released {
                released = waiting.resume.changed.wait(released).unwrap();
            }
        }
        // Fault injection only pauses the actor. Its real selection,
        // measurements and gain rules are otherwise exactly RoadOrder.
        self.road.select(ctx)
    }
}

async fn with_order(order: Arc<PausingOrder>) -> Epoch {
    let mut f = Epoch::new().await;
    // Rebind only this scratch endpoint before any carried stream exists.
    // The original Epoch fixture stays byte-identical as the file prefix.
    f.answering.0.as_ref().unwrap().abort();
    if let Err(error) = f.answering.0.take().unwrap().await {
        assert!(error.is_cancelled());
    }
    f.ep.close().await;
    f.ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(f.node.key.clone())
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .add_custom_transport(f.node.carry.transport())
        .path_selector(order)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    *f.node.ears.lock().unwrap() = Some(Arc::new(f.ep.clone()));
    let addr = iroh::EndpointAddr::from_parts(
        f.remote.id(),
        f.remote
            .addr()
            .addrs
            .into_iter()
            .filter(|addr| matches!(addr, iroh::TransportAddr::Ip(_))),
    );
    let (conn, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(f.ep.connect(addr, ALPN), async {
            f.remote.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    f.conn = conn.unwrap();
    let incoming = incoming.unwrap();
    f.answering = Owned(Some(tokio::spawn(async move {
        while let Ok((mut send, mut recv)) = incoming.accept_bi().await {
            let Ok(bytes) = recv.read_to_end(64).await else {
                break;
            };
            if send.write_all(&bytes).await.is_err() {
                break;
            }
            let _ = send.finish();
        }
    })));
    assert!(echo(&f.conn).await);
    f
}

async fn observe_queued(replacement: bool) {
    let order = Arc::new(PausingOrder::default());
    let mut f = with_order(order.clone()).await;
    f.attach().await;
    f.end_stream().await;
    let mut paused = order.pause();
    tokio::time::timeout(Duration::from_secs(2), &mut paused.entered)
        .await
        .unwrap()
        .unwrap();
    let mut closing = Owned(Some(tokio::spawn({
        let (node, links, peer) = (Arc::downgrade(&f.node), f.node.carry.clone(), f.remote.id());
        async move { crate::carry::let_path_go(&node, &links, peer).await }
    })));
    // Both APIs complete after enqueue. The real remote actor is still
    // inside our one-shot selector callback and has not inspected close.
    tokio::time::timeout(Duration::from_secs(2), closing.0.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(!*paused.resume.released.lock().unwrap());
    assert!(!f.node.carry.has(&f.remote.id()));
    if replacement {
        let (near, far) = tokio::io::duplex(1 << 20);
        let (nr, nw) = tokio::io::split(near);
        let (fr, fw) = tokio::io::split(far);
        f.stream = Some((
            f.node.carry.attach(f.remote.id(), Carrier::Ssh, nr, nw),
            f.other.attach(f.ep.id(), Carrier::Ssh, fr, fw),
        ));
        assert!(f.node.carry.has(&f.remote.id()));
    }
    let mut events = f.conn.path_events();
    paused.resume.release();
    if replacement {
        f.ep.offer_addrs(iroh::EndpointAddr::from_parts(
            f.remote.id(),
            [transport_addr(&f.remote.id())],
        ))
        .await;
    }
    assert!(f.ep.remote_info(f.remote.id()).await.is_some());
    let closed = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = events.next().await {
            match event {
                iroh::endpoint::PathEvent::Closed {
                    remote_addr: iroh::TransportAddr::Custom(_),
                    ..
                } => return true,
                iroh::endpoint::PathEvent::Lagged { .. } => {
                    panic!("the actual path event stream must not lag")
                }
                _ => {}
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    let carried = khor_net::path::path_facts(&f.conn)
        .iter()
        .any(|path| path.carried);
    let answered = echo(&f.conn).await;
    let wire_open = f.conn.close_reason().is_none();
    f.close().await;

    assert!(answered && wire_open);
    if replacement {
        assert!(
            !closed && carried,
            "close was enqueued before the new stream, so the actor must recheck it: closed={closed}, carried={carried}"
        );
    } else {
        assert!(
            closed && !carried,
            "an absent carrier still closes its path"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_already_queued_close_rechecks_the_actual_replacement() {
    observe_queued(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_already_queued_close_without_replacement_still_closes() {
    observe_queued(false).await;
}
