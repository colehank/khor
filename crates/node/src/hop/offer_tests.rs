//! Actual open_hop ownership before its carried address is offered.

use super::*;
use std::sync::Arc;

struct Owned<T>(Option<tokio::task::JoinHandle<T>>);

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}

async fn endpoint() -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![khor_net::endpoint::TUNNEL_ALPN.to_vec()])
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .bind()
        .await
        .unwrap()
}

struct Fixture {
    _root: tempfile::TempDir,
    node: Arc<Node>,
    ep: iroh::Endpoint,
    middle_ep: iroh::Endpoint,
    middle: crate::DeviceInfo,
    target: crate::DeviceInfo,
    local: iroh::endpoint::Connection,
    remote: iroh::endpoint::Connection,
    first_stopped: tokio::sync::oneshot::Receiver<bool>,
    stop_server: tokio::sync::oneshot::Sender<()>,
    serving: Owned<()>,
}

impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".khor")).unwrap();
        std::fs::write(
            root.path().join(".khor/relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        let node = Arc::new(Node::open_hosting(root.path().to_path_buf(), "offer", false).unwrap());
        let ep = endpoint().await;
        let middle_ep = endpoint().await;
        let target_id = iroh::SecretKey::generate().public().to_string();
        {
            let loaded = node.devices_loaded().unwrap();
            loaded
                .doc
                .upsert(&middle_ep.id().to_string(), "middle", &[])
                .unwrap();
            loaded.doc.upsert(&target_id, "target", &[]).unwrap();
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
        }
        let rows = node.devices().unwrap();
        let middle = rows
            .iter()
            .find(|d| d.id == middle_ep.id().to_string())
            .unwrap()
            .clone();
        let target = rows.iter().find(|d| d.id == target_id).unwrap().clone();
        let (local, remote) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(
                ep.connect(middle_ep.addr(), khor_net::endpoint::TUNNEL_ALPN),
                async { middle_ep.accept().await.unwrap().accept().unwrap().await }
            )
        })
        .await
        .unwrap();
        let (local, remote) = (local.unwrap(), remote.unwrap());
        node.wires.adopt(&middle.id, Proto::Tunnel, local.clone());
        let (stop_server, mut stop) = tokio::sync::oneshot::channel();
        let (first_stopped, stopped) = tokio::sync::oneshot::channel();
        let serving = Owned(Some(tokio::spawn({
            let conn = remote.clone();
            let target = Ask::To(target_id.parse().unwrap()).words();
            async move {
                let mut tasks = tokio::task::JoinSet::new();
                let mut first_stopped = Some(first_stopped);
                loop {
                    tokio::select! {
                        _ = &mut stop => break,
                        stream = conn.accept_bi() => {
                            let Ok((mut send, mut recv)) = stream else { break };
                            let expected = target.clone();
                            let report = first_stopped.take();
                            tasks.spawn(async move {
                                let target = crate::tunnel::read_target(&mut recv).await.unwrap();
                                assert_eq!(target, expected);
                                // Script only the middle's tunnel admission. No
                                // onward route or business success is claimed.
                                send.write_all(&[crate::tunnel::OK]).await.unwrap();
                                let stopped = matches!(send.stopped().await, Ok(Some(_)));
                                if let Some(report) = report {
                                    let _ = report.send(stopped);
                                }
                                drop(recv);
                            });
                        }
                        joined = tasks.join_next(), if !tasks.is_empty() => {
                            joined.unwrap().unwrap();
                        }
                    }
                }
                tasks.abort_all();
                while let Some(joined) = tasks.join_next().await {
                    if let Err(error) = joined {
                        assert!(
                            error.is_cancelled(),
                            "scratch tunnel handler must not panic"
                        );
                    }
                }
            }
        })));
        Self {
            _root: root,
            node,
            ep,
            middle_ep,
            middle,
            target,
            local,
            remote,
            first_stopped: stopped,
            stop_server,
            serving,
        }
    }

    fn opening(&self) -> Owned<bool> {
        let (node, ep, target, middle) = (
            self.node.clone(),
            self.ep.clone(),
            self.target.clone(),
            self.middle.clone(),
        );
        Owned(Some(tokio::spawn(async move {
            let picked = vec![middle.id.clone()];
            node.open_hop(&ep, &target, &[middle], &picked).await
        })))
    }

    async fn close(mut self) {
        self.node.carry.detach(
            &self.target.id.parse().unwrap(),
            Carrier::Hop {
                via: self.middle.id.parse().unwrap(),
            },
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while asking(self.node.device_str(), &self.target.id) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let _ = self.stop_server.send(());
        tokio::time::timeout(Duration::from_secs(2), self.serving.0.as_mut().unwrap())
            .await
            .unwrap()
            .unwrap();
        self.ep.close().await;
        self.middle_ep.close().await;
    }
}

async fn ready(receiver: tokio::sync::oneshot::Receiver<u64>) -> u64 {
    tokio::time::timeout(Duration::from_secs(5), receiver)
        .await
        .unwrap()
        .unwrap()
}

async fn cancel(task: &mut Owned<bool>) {
    task.0.as_ref().unwrap().abort();
    let error = task.0.as_mut().unwrap().await.unwrap_err();
    assert!(error.is_cancelled());
}

async fn finish(task: &mut Owned<bool>) -> bool {
    tokio::time::timeout(Duration::from_secs(5), task.0.as_mut().unwrap())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_before_offer_reclaims_the_actual_hop_stream() {
    let mut f = Fixture::new().await;
    let (entered, _release) = f.node.wires.gate_hop_offer_for_test();
    let mut opening = f.opening();
    let serial = ready(entered).await;
    assert!(serial > 0);
    assert!(f.node.carry.has(&f.target.id.parse().unwrap()));
    assert!(asking(f.node.device_str(), &f.target.id));
    cancel(&mut opening).await;
    let orphaned = f.node.carry.has(&f.target.id.parse().unwrap());
    let still_asking = asking(f.node.device_str(), &f.target.id);
    let stopped = matches!(
        tokio::time::timeout(Duration::from_secs(1), &mut f.first_stopped).await,
        Ok(Ok(true))
    );
    let wire_open = f.local.close_reason().is_none() && f.remote.close_reason().is_none();
    f.close().await;
    assert!(!still_asking);
    assert!(
        wire_open,
        "stream cancellation keeps the middle's connection"
    );
    assert!(
        !orphaned && stopped,
        "cancelled pre-offer owner must reclaim its attachment and stream: orphaned={orphaned}, stopped={stopped}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_completed_offer_hands_the_hop_to_its_watcher() {
    let f = Fixture::new().await;
    let (entered, release) = f.node.wires.gate_hop_offer_for_test();
    let mut opening = f.opening();
    let _serial = ready(entered).await;
    release.send(()).unwrap();
    assert!(finish(&mut opening).await);
    assert!(f.node.carry.has(&f.target.id.parse().unwrap()));
    assert!(asking(f.node.device_str(), &f.target.id));
    assert!(f.local.close_reason().is_none());
    f.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_old_preparation_preserves_the_new_attachment() {
    let f = Fixture::new().await;
    let (entered_old, _release_old) = f.node.wires.gate_hop_offer_for_test();
    let mut old = f.opening();
    let serial_old = ready(entered_old).await;
    // This fixture owns the replacement's actual second tunnel stream;
    // no competing-hop ID arbitration or new watcher is being simulated.
    let words = Ask::To(f.target.id.parse().unwrap()).words();
    let (send, recv) = tokio::time::timeout(
        Duration::from_secs(5),
        crate::tunnel::dial(&f.local, &words),
    )
    .await
    .unwrap()
    .unwrap();
    let replacement = f.node.carry.attach(
        f.target.id.parse().unwrap(),
        Carrier::Hop {
            via: f.middle.id.parse().unwrap(),
        },
        recv,
        send,
    );
    assert_ne!(serial_old, replacement.serial());
    let ended = replacement.ended();
    tokio::pin!(ended);
    cancel(&mut old).await;
    assert!(f.node.carry.has(&f.target.id.parse().unwrap()));
    assert!(!asking(f.node.device_str(), &f.target.id));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut ended)
            .await
            .is_err()
    );
    assert!(f.local.close_reason().is_none());
    f.close().await;
    tokio::time::timeout(Duration::from_secs(2), &mut ended)
        .await
        .unwrap();
}

#[path = "prepare_budget_tests.rs"]
mod prepare_budget;

#[path = "published_tests.rs"]
mod published;

#[path = "close_offer_tests.rs"]
mod close_offer;
