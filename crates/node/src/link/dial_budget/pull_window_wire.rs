//! The real pull scheduler against a controlled, authenticated QUIC responder.
//! Server markers mean decoded RPCs, lifecycle markers mean decoded client
//! replies, and part-file lengths mean committed bytes. A responder is not the
//! production server/host-child cancellation contract, which remains separate.
use super::*;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

mod job_stop;
mod offer_stop;
mod stop_rpc;

const PATH: &str = "/pull-window-wire.bin";

#[derive(Clone, Copy, Debug)]
enum Held {
    First,
    Head,
    Tail,
    All,
}

#[derive(Default)]
struct Seen {
    issued: BTreeSet<u64>,
    delivered: BTreeSet<u64>,
    stopped: BTreeSet<u64>,
}

struct Fixture {
    root: tempfile::TempDir,
    node: Arc<Node>,
    ep: iroh::Endpoint,
    server: iroh::Endpoint,
    local: iroh::endpoint::Connection,
    remote: iroh::endpoint::Connection,
    serving: Aborts<()>,
    seen: Arc<Mutex<Seen>>,
    changed: Arc<Notify>,
    release: Arc<Notify>,
    bytes: Arc<Vec<u8>>,
    kind: Held,
}

impl Fixture {
    async fn new(kind: Held) -> Self {
        Self::build(kind, false).await
    }

    async fn build(kind: Held, offered: bool) -> Self {
        Self::with_options(kind, offered, false).await
    }

    async fn with_identity(kind: Held, node_identity: bool) -> Self {
        Self::with_options(kind, false, node_identity).await
    }

    async fn with_options(kind: Held, offered: bool, node_identity: bool) -> Self {
        assert!(PULL_AHEAD >= 2);
        let server = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .relay_mode(iroh::RelayMode::Disabled)
            .alpns(vec![ALPN.to_vec()])
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")
            .unwrap()
            .bind()
            .await
            .unwrap();
        let (root, node, peer, initial) = fixture_for(server.id().to_string()).await;
        let ep = if node_identity {
            initial.close().await;
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
                .secret_key(node.key.clone())
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![ALPN.to_vec()])
                .clear_ip_transports()
                .bind_addr("127.0.0.1:0")
                .unwrap()
                .bind()
                .await
                .unwrap();
            assert_eq!(ep.id().to_string(), node.device_str());
            ep
        } else {
            initial
        };
        let node = Arc::new(node);
        let (local, remote) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(ep.connect(server.addr(), ALPN), async {
                server.accept().await.unwrap().accept().unwrap().await
            })
        })
        .await
        .unwrap();
        let (local, remote) = (local.unwrap(), remote.unwrap());
        node.wires
            .adopt(&peer.id, wires::Proto::Main, local.clone());
        let seen = Arc::new(Mutex::new(Seen::default()));
        let changed = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let bytes = Arc::new(
            (0..(12 * proto::SLICE + 1234))
                .map(|i| (i % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let serving = Aborts(Some(tokio::spawn({
            let peer = server.id().to_string();
            let (conn, seen, changed, release, bytes) = (
                remote.clone(),
                seen.clone(),
                changed.clone(),
                release.clone(),
                bytes.clone(),
            );
            async move {
                // JoinSet owns every responder, including a held stream; an
                // assertion or outer deadline cannot detach any of them.
                let mut children = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        stream = conn.accept_bi() => {
                            let Ok((mut send, mut recv)) = stream else { break };
                            let (seen, changed, release, bytes) =
                                (seen.clone(), changed.clone(), release.clone(), bytes.clone());
                            let peer = peer.clone();
                            children.spawn(async move {
                                let request = recv.read_to_end(MAX_FRAME).await.unwrap();
                                let question = proto::question(&request).unwrap();
                                let offer_request = matches!(&question, Request::Fetch { .. });
                                match question {
                                    Request::FetchPath { path, offset } | Request::Fetch { digest: path, offset } => {
                                        assert_eq!(offer_request, offered);
                                        if offered {
                                            assert_eq!(path, blake3::hash(&bytes).to_hex().to_string());
                                        } else {
                                            assert_eq!(path, PATH);
                                        }
                                        let index = offset / proto::SLICE;
                                        assert_eq!(offset % proto::SLICE, 0);
                                        assert!(seen.lock().unwrap().issued.insert(index), "duplicate RPC");
                                        eprintln!("[pull-window-wire] {}", serde_json::json!({
                                            "event":"issued", "peer":peer, "kind":format!("{kind:?}"), "index":index,
                                        }));
                                        changed.notify_one();
                                        let hold = match kind {
                                            Held::First => index == 0,
                                            Held::Head => index == 1,
                                            Held::Tail => index == PULL_AHEAD,
                                            Held::All => index > 0,
                                        };
                                        if hold {
                                            tokio::select! {
                                                _ = release.notified() => {},
                                                stopped = send.stopped() => {
                                                    assert!(matches!(stopped, Ok(Some(_))), "only stream STOP_SENDING counts");
                                                    seen.lock().unwrap().stopped.insert(index);
                                                    changed.notify_one();
                                                    return;
                                                }
                                            }
                                        }
                                        let start = offset as usize;
                                        let end = (start + proto::SLICE as usize).min(bytes.len());
                                        let reply = if offered {
                                            Response::Slice {
                                                total: bytes.len() as u64,
                                                bytes: bytes[start..end].to_vec().into(),
                                            }
                                        } else {
                                            Response::PathSlice {
                                                total: bytes.len() as u64,
                                                at_ms: 123,
                                                bytes: bytes[start..end].to_vec().into(),
                                            }
                                        };
                                        send.write_all(&proto::encode(&reply).unwrap()).await.unwrap();
                                        send.finish().unwrap();
                                        // ACKed data is labelled delivered, never called decoded.
                                        assert!(matches!(send.stopped().await, Ok(None)));
                                        seen.lock().unwrap().delivered.insert(index);
                                        changed.notify_one();
                                    }
                                    Request::Sessions => {
                                        send.write_all(&proto::encode(&Response::SessionRows { rows: vec![] }).unwrap())
                                            .await.unwrap();
                                        send.finish().unwrap();
                                    }
                                    other => panic!("unexpected request: {other:?}"),
                                }
                            });
                        }
                        child = children.join_next(), if !children.is_empty() => {
                            child.unwrap().unwrap();
                        }
                    }
                }
                children.shutdown().await;
            }
        })));
        Self {
            root,
            node,
            ep,
            server,
            local,
            remote,
            serving,
            seen,
            changed,
            release,
            bytes,
            kind,
        }
    }

    fn part(&self) -> std::path::PathBuf {
        self.root.path().join(".khor-pull-pull-window-wire.bin")
    }

    async fn wait(&self, predicate: impl Fn(&Seen) -> bool) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let changed = self.changed.notified();
                if predicate(&self.seen.lock().unwrap()) {
                    break;
                }
                changed.await;
            }
        })
        .await
        .expect("controlled RPC condition did not arrive");
    }

    fn snapshot(&self, event: &str) -> u64 {
        let committed = std::fs::metadata(self.part()).unwrap().len();
        let seen = self.seen.lock().unwrap();
        eprintln!(
            "[pull-window-wire] {}",
            serde_json::json!({
                "event":event, "kind":format!("{:?}",self.kind), "peer":self.server.id().to_string(),
                "width":PULL_AHEAD, "slice":proto::SLICE, "total":self.bytes.len(), "committed":committed,
                "issued":seen.issued, "delivered":seen.delivered, "stopped":seen.stopped,
                "wire":self.local.stable_id(),
            })
        );
        committed
    }

    async fn finish(&self, result: Result<(u64, std::path::PathBuf), String>) {
        let (moved, dest) = result.unwrap();
        assert_eq!(moved, self.bytes.len() as u64);
        assert_eq!(std::fs::read(dest).unwrap(), *self.bytes);
        assert!(!self.part().exists());
        assert!(!PullNote::path_of(&self.part()).exists());
        assert_eq!(self.seen.lock().unwrap().issued, (0..13).collect());
    }

    async fn close(mut self) {
        self.serving.0.as_ref().unwrap().abort();
        let _ = self.serving.0.take().unwrap().await;
        tokio::join!(self.ep.close(), self.server.close());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn slow_tail_reports_real_rpc_refill_and_committed_prefix() {
    let fixture = Fixture::new(Held::Tail).await;
    let expected = std::env::var("KHOR_PULL_CONTRACT").unwrap_or_else(|_| "rolling".into());
    assert!(matches!(expected.as_str(), "batch" | "rolling"));
    {
        let pull = fixture
            .node
            .pull_path_with(&fixture.ep, "peer", PATH, fixture.root.path());
        tokio::pin!(pull);
        let observed = async {
            fixture.wait(|seen| seen.issued.contains(&PULL_AHEAD)).await;
            fixture
                .wait(|seen| seen.delivered.len() >= PULL_AHEAD as usize)
                .await;
            let refilled = tokio::time::timeout(
                Duration::from_secs(3),
                fixture.wait(|seen| seen.issued.contains(&(PULL_AHEAD + 1))),
            )
            .await
            .is_ok();
            if refilled {
                tokio::time::timeout(Duration::from_secs(5), async {
                    while std::fs::metadata(fixture.part()).unwrap().len()
                        < PULL_AHEAD * proto::SLICE
                    {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await
                .expect("the ready prefix must actually commit before the held tail");
            }
            let committed = fixture.snapshot("tail_before_release");
            fixture.release.notify_one();
            (refilled, committed)
        };
        let (outcome, (refilled, committed)) =
            tokio::time::timeout(Duration::from_secs(20), async {
                tokio::join!(&mut pull, observed)
            })
            .await
            .unwrap();
        fixture.finish(outcome).await;
        assert_eq!(refilled, expected == "rolling");
        if refilled {
            assert!(committed >= 2 * proto::SLICE);
        } else {
            assert_eq!(committed, proto::SLICE);
        }
    }
    fixture.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn slow_head_bounds_real_issued_and_uncommitted_replies() {
    let fixture = Fixture::new(Held::Head).await;
    {
        let pull = fixture
            .node
            .pull_path_with(&fixture.ep, "peer", PATH, fixture.root.path());
        tokio::pin!(pull);
        let observed = async {
            fixture
                .wait(|seen| {
                    seen.issued.len() == PULL_AHEAD as usize + 1
                        && seen.delivered.len() == PULL_AHEAD as usize
                })
                .await;
            let excess = tokio::time::timeout(
                Duration::from_millis(250),
                fixture.wait(|seen| seen.issued.contains(&(PULL_AHEAD + 1))),
            )
            .await
            .is_ok();
            assert!(
                !excess,
                "completed but uncommitted responses still occupy slots"
            );
            assert_eq!(fixture.snapshot("head_before_release"), proto::SLICE);
            fixture.release.notify_one();
        };
        let (outcome, ()) = tokio::time::timeout(Duration::from_secs(20), async {
            tokio::join!(&mut pull, observed)
        })
        .await
        .unwrap();
        fixture.finish(outcome).await;
    }
    fixture.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropping_pull_stops_every_held_rpc_and_keeps_the_shared_wire() {
    let fixture = Fixture::new(Held::All).await;
    {
        let pull = fixture
            .node
            .pull_path_with(&fixture.ep, "peer", PATH, fixture.root.path());
        tokio::pin!(pull);
        tokio::select! {
            outcome = &mut pull => panic!("held pull completed: {outcome:?}"),
            _ = fixture.wait(|seen| seen.issued.len() == PULL_AHEAD as usize + 1) => {},
        }
        assert_eq!(fixture.snapshot("before_drop"), proto::SLICE);
    }
    fixture
        .wait(|seen| seen.stopped.len() == PULL_AHEAD as usize)
        .await;
    let peer = fixture
        .node
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "peer")
        .unwrap();
    let answer = tokio::time::timeout(
        Duration::from_secs(5),
        fixture.node.ask_on(&fixture.ep, &peer, &Request::Sessions),
    )
    .await
    .unwrap();
    assert!(matches!(answer, Ok(Response::SessionRows { .. })));
    assert!(fixture.remote.close_reason().is_none() && fixture.local.close_reason().is_none());
    assert_eq!(
        fixture
            .node
            .wires
            .live(&peer.id, wires::Proto::Main)
            .unwrap()
            .stable_id(),
        fixture.local.stable_id()
    );
    fixture.snapshot("after_drop_control");
    assert_eq!(
        fixture.seen.lock().unwrap().issued.len(),
        PULL_AHEAD as usize + 1
    );
    // A raw future drop is not the Job stop action or landing cleanup contract.
    fixture.close().await;
}
