//! Source-only close regressions: native workers and real file/claim barriers.
//! Factory failure is explicitly controlled; no business RPC result is injected.
use super::*;
use crate::remote_cancel::{self, Metadata};
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    sync::Condvar,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Gate {
    entered: AtomicBool,
    released: Mutex<bool>,
    wake: Condvar,
    expired: AtomicBool,
}
impl Gate {
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
}
#[derive(Clone)]
struct Hook {
    point: &'static str,
    gate: Arc<Gate>,
    fail_spawn: bool,
    starts: Arc<Mutex<Vec<u32>>>,
}
fn hooks() -> &'static Mutex<HashMap<PathBuf, Hook>> {
    static HOOKS: OnceLock<Mutex<HashMap<PathBuf, Hook>>> = OnceLock::new();
    HOOKS.get_or_init(Default::default)
}
struct ObservedRoot {
    root: PathBuf,
    hook: Hook,
}
impl ObservedRoot {
    fn new(root: &Path, point: &'static str, fail_spawn: bool) -> Self {
        let hook = Hook {
            point,
            gate: Arc::default(),
            fail_spawn,
            starts: Arc::default(),
        };
        let mut registered = hooks().lock().unwrap();
        assert!(registered.len() < 16);
        assert!(registered.insert(root.into(), hook.clone()).is_none());
        Self {
            root: root.into(),
            hook,
        }
    }
    fn pids(&self) -> Vec<u32> {
        self.hook.starts.lock().unwrap().clone()
    }
    fn point(&mut self, point: &'static str) -> Arc<Gate> {
        let previous = self.hook.gate.clone();
        self.hook.gate = Arc::default();
        self.hook.point = point;
        hooks()
            .lock()
            .unwrap()
            .insert(self.root.clone(), self.hook.clone());
        previous
    }
}
impl Drop for ObservedRoot {
    fn drop(&mut self) {
        self.hook.gate.release();
        hooks().lock().unwrap().remove(&self.root);
    }
}
pub(crate) fn fail_spawn(root: &Path) -> bool {
    hooks()
        .lock()
        .unwrap()
        .get(root)
        .is_some_and(|hook| hook.fail_spawn)
}
pub(crate) fn producer_point(root: &Path, point: &str) {
    let hook = hooks().lock().unwrap().get(root).cloned();
    if let Some(hook) = hook.filter(|hook| hook.point == point) {
        hook.gate.entered.store(true, Ordering::SeqCst);
        let (released, timeout) = hook
            .gate
            .wake
            .wait_timeout_while(
                hook.gate.released.lock().unwrap(),
                Duration::from_secs(4),
                |released| !*released,
            )
            .unwrap();
        drop(released);
        hook.gate
            .expired
            .store(timeout.timed_out(), Ordering::SeqCst);
    }
}
pub(crate) fn owner_started(root: &Path, pid: u32) {
    if let Some(hook) = hooks().lock().unwrap().get(root) {
        let mut starts = hook.starts.lock().unwrap();
        assert!(starts.len() < 128);
        starts.push(pid);
    }
}
async fn until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    true
}
fn historical(node: &Node) -> String {
    let run = format!("agent-close-{}", crate::link::fresh_hex().unwrap());
    let saved = Saved {
        fingerprint: "close-native-fixture".into(),
        record: CommandRecord {
            id: run.clone(),
            owner: node.device_str().into(),
            status: "cancelled".into(),
            started_at_ms: now() - 10,
            ended_at_ms: Some(now()),
            agent: Some(AgentTurn {
                model: "fixture".into(),
                approval: None,
                updated_at_ms: now(),
                updates: vec![],
                updates_truncated: false,
            }),
            ..Default::default()
        },
    };
    let claim = claim(node.root()).unwrap();
    save(node.root(), &saved).unwrap();
    drop(claim);
    run
}
fn no_process(pid: u32) -> bool {
    (unsafe { libc::kill(pid as i32, 0) == -1 })
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}
fn assert_retired(observed: &ObservedRoot) {
    for pid in observed.pids() {
        println!("close native owner PID {pid}: ESRCH={}", no_process(pid));
        assert!(no_process(pid));
    }
}
async fn close_after_progress(node: &Node) -> RuntimeClose {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match Node::close_agent_runtime(node.root()).await.unwrap() {
            RuntimeClose::Closed => return RuntimeClose::Closed,
            RuntimeClose::Busy => {
                assert!(Instant::now() < deadline, "runtime never quiesced");
                node.reconcile_agent().unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    }
}

/// FIFO read is an actual native valid_run/read_to_end barrier, not a fake
/// metadata response. The original real Saved bytes are restored before close.
struct HeldRun {
    path: PathBuf,
    original: Vec<u8>,
    writer: Option<File>,
    restored: bool,
}
impl HeldRun {
    fn new(node: &Node, run: &str) -> Self {
        let path = file(node.root(), run);
        let original = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        Self {
            path,
            original,
            writer: None,
            restored: false,
        }
    }
    fn connected(&mut self) -> bool {
        if self.writer.is_none() {
            match OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&self.path)
            {
                Ok(writer) => self.writer = Some(writer),
                Err(error) if error.raw_os_error() == Some(libc::ENXIO) => return false,
                Err(error) => panic!("FIFO connection failed: {error}"),
            }
        }
        true
    }
    fn release(&mut self) {
        if self.restored {
            return;
        }
        // The native reader already owns the FIFO inode. Replace the pathname
        // first so subsequent real snapshot reads see the ordinary Saved file.
        fs::remove_file(&self.path).unwrap();
        fs::write(&self.path, &self.original).unwrap();
        if let Some(mut writer) = self.writer.take() {
            writer.write_all(&self.original).unwrap();
        }
        self.restored = true;
    }
}
impl Drop for HeldRun {
    fn drop(&mut self) {
        if !self.restored {
            self.release();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated native acceptance: scripts/run-node-acceptance.py"]
async fn cold_close_is_no_create_and_closed_roots_have_a_hard_quota() {
    let tree = crate::testdir::TempTree::fresh("agent-close-cold");
    let mut roots = Vec::new();
    for index in 0..255 {
        let root = tree.join(format!("cold-{index}"));
        Node::acquire_agent_runtime(&root).unwrap();
        // Capacity-only fixture: defer these Roots' ordinary scans, rather
        // than resetting the live scheduler or creating unrelated workers.
        remote_cancel::test_defer_scan(&root);
        assert!(!root.exists());
        roots.push(root);
    }
    let cold = tree.join("reserved-close");
    let reserved = ObservedRoot::new(&cold, "close-reserved", false);
    let closing_root = cold.clone();
    let closing = tokio::spawn(async move { Node::close_agent_runtime(&closing_root).await });
    let entered = until(|| reserved.hook.gate.entered.load(Ordering::SeqCst)).await;
    let while_reserved_count = remote_cancel::test_root_count();
    let denied = tree.join("concurrent-pump-and-status");
    let pumped = denied.clone();
    let pump = std::thread::spawn(move || remote_cancel::pump(&pumped, true));
    let acquired = tree.join("concurrent-acquire");
    let acquire = std::thread::spawn(move || Node::acquire_agent_runtime(&acquired));
    let status = remote_cancel::request(&denied, "capacity-fixture-run", false, true, false).await;
    pump.join().unwrap();
    let acquire_result = acquire.join().unwrap();
    let after_competitors_count = remote_cancel::test_root_count();
    reserved.hook.gate.release();
    let closed = closing.await.unwrap();
    let after_closed_count = remote_cancel::test_root_count();
    for root in &roots {
        assert_eq!(
            Node::close_agent_runtime(root).await.unwrap(),
            RuntimeClose::Closed
        );
        assert!(!root.exists());
    }
    roots.push(cold);
    assert_retired(&reserved);
    assert!(entered && !reserved.hook.gate.expired.load(Ordering::SeqCst));
    assert_eq!(closed.unwrap(), RuntimeClose::Closed);
    assert_eq!(while_reserved_count, 256);
    assert_eq!(after_competitors_count, 256);
    assert_eq!(after_closed_count, 256);
    assert_eq!(acquire_result.unwrap_err(), "ResourceLimit");
    assert_eq!(status["durability"]["code"], "ResourceLimit");
    assert!(!denied.exists());
    assert!(roots.iter().all(|root| !root.exists()));
    let overflow = tree.join("overflow");
    assert_eq!(
        Node::close_agent_runtime(&overflow).await.unwrap(),
        RuntimeClose::Busy
    );
    assert!(!overflow.exists());
    remote_cancel::pump(&roots[0], true);
    assert!(remote_cancel::test_close_state(&roots[0]).1);
    assert!(!roots[0].exists());
    Node::acquire_agent_runtime(&roots[0]).unwrap();
    assert_eq!(
        Node::close_agent_runtime(&overflow).await.unwrap(),
        RuntimeClose::Busy
    );
    assert!(!overflow.exists());
    assert!(!remote_cancel::test_close_state(&roots[0]).1);
    assert_eq!(
        Node::close_agent_runtime(&roots[0]).await.unwrap(),
        RuntimeClose::Closed
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated native acceptance: scripts/run-node-acceptance.py"]
async fn native_metadata_join_fences_passive_reopen_and_preserves_other_root_progress() {
    let tree = crate::testdir::TempTree::fresh("agent-close-native");
    let node = Node::open_as(tree.join("held"), "close-held").unwrap();
    let other = Node::open_as(tree.join("other"), "close-other").unwrap();
    let run = historical(&node);
    let other_run = historical(&other);
    let observed = ObservedRoot::new(node.root(), "", false);
    let other_observed = ObservedRoot::new(other.root(), "", false);
    let other_initial = remote_cancel::metadata(
        other.root(),
        Metadata::Persist {
            run: other_run.clone(),
            reason: "Stop".into(),
        },
    )
    .unwrap();
    assert!(other_initial.contains("DurablyQueued"));
    let mut held = HeldRun::new(&node, &run);
    let request_root = node.root().clone();
    let request_run = run.clone();
    let admitted = remote_cancel::request(&request_root, &request_run, true, true, false).await;
    // Stop now returns while its real worker owns the FIFO. An explicit Retry
    // joins that same owner and still waits for its original native result.
    let mut requested = Box::pin(remote_cancel::request(
        &request_root,
        &request_run,
        true,
        true,
        true,
    ));
    let requested_pending = futures::poll!(&mut requested).is_pending();
    let connected = until(|| held.connected()).await;
    let closing_root = node.root().clone();
    let closing = tokio::spawn(async move { Node::close_agent_runtime(&closing_root).await });
    let fenced = until(|| remote_cancel::test_close_state(node.root()).0).await;
    let held_close_pending = !closing.is_finished() && node.root().exists();
    let refused = remote_cancel::request(node.root(), &run, true, true, true).await;

    let busy = remote_cancel::request(other.root(), &other_run, false, true, false).await;

    // A genuine already-durable Step in another Root uses a remaining network
    // lane while this Root's metadata is held. No worker result is fabricated.
    remote_cancel::test_resume_step(other.root(), &other_run);
    let other_progressed = until(|| {
        remote_cancel::pump(other.root(), true);
        !other_observed.pids().is_empty()
            && other_observed.pids().iter().all(|pid| no_process(*pid))
            && remote_cancel::test_close_state(other.root()).2 == 0
    })
    .await;
    let quotas = remote_cancel::test_close_state(node.root());
    remote_cancel::local_retired(other.root().clone(), other_run.clone());
    let queued_retirement = remote_cancel::test_close_state(other.root()).4;
    let unpersisted_busy = Node::close_agent_runtime(other.root()).await.unwrap();
    let retirement_retained = remote_cancel::test_close_state(other.root()).4;
    held.release();
    let closed = closing.await.unwrap().unwrap();
    let actual = requested.await;
    // Reap the other Root before any primary outcome assertions or fixture drop.
    let other_closed = close_after_progress(&other).await;
    assert_retired(&observed);
    assert_retired(&other_observed);
    assert!(connected && requested_pending && fenced && held_close_pending && other_progressed);
    assert_eq!(admitted["durability"]["state"], "PersistenceUnknown");
    assert_eq!(admitted["durability"]["code"], "MetadataOutcomeUnknown");
    assert_eq!(refused["durability"]["code"], "RuntimeClosing");
    assert_eq!(busy["durability"]["code"], "Busy");
    assert!(quotas.2 <= 4 && quotas.3);
    assert_eq!(unpersisted_busy, RuntimeClose::Busy);
    assert_eq!(queued_retirement, 1);
    assert_eq!(retirement_retained, 1);
    assert_eq!(closed, RuntimeClose::Closed);
    assert_eq!(other_closed, RuntimeClose::Closed);
    assert_eq!(actual["durability"]["state"], "DurablyQueued");
    let starts = observed.pids();
    node.khor_agent_here(Action::Read).await.unwrap();
    node.reconcile_agent().unwrap();
    remote_cancel::pump(node.root(), true);
    let passive = remote_cancel::request(node.root(), &run, false, true, false).await;
    assert_eq!(passive["durability"]["code"], "RuntimeClosed");
    assert_eq!(observed.pids(), starts);
    Node::acquire_agent_runtime(node.root()).unwrap();
    let resumed = remote_cancel::request(node.root(), &run, false, true, false).await;
    assert_eq!(resumed["durability"]["state"], "DurablyQueued");
    assert!(observed.pids().len() > starts.len());
    let pending = node
        .root()
        .join(".khor/agent/cancellations/pending")
        .join(format!("{}.json", blake3::hash(run.as_bytes()).to_hex()));
    let preserved_pending = pending.is_file();
    let resumed_step = until(|| {
        node.reconcile_agent().unwrap();
        !pending.exists()
    })
    .await;
    assert_eq!(close_after_progress(&node).await, RuntimeClose::Closed);
    assert_retired(&observed);
    assert!(preserved_pending && resumed_step);
    assert!(!remote_cancel::test_step_starts(node.root()).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated native acceptance: scripts/run-node-acceptance.py"]
async fn canceling_a_close_waiter_keeps_finite_owner_and_quota_until_real_join() {
    let tree = crate::testdir::TempTree::fresh("agent-close-dropped-waiter");
    let node = Node::open_as(tree.join("store"), "close-drop").unwrap();
    let run = historical(&node);
    let observed = ObservedRoot::new(node.root(), "", false);
    let mut held = HeldRun::new(&node, &run);
    let request_root = node.root().clone();
    let request_run = run.clone();
    let admitted = remote_cancel::request(&request_root, &request_run, true, true, false).await;
    let mut request = Box::pin(remote_cancel::request(
        &request_root,
        &request_run,
        true,
        true,
        true,
    ));
    let request_pending = futures::poll!(&mut request).is_pending();
    let connected = until(|| held.connected()).await;
    let close_root = node.root().clone();
    let close = tokio::spawn(async move { Node::close_agent_runtime(&close_root).await });
    let fenced = until(|| remote_cancel::test_close_state(node.root()).0).await;
    close.abort();
    let _ = close.await;
    let while_held = Node::close_agent_runtime(node.root()).await.unwrap();
    let quota = remote_cancel::test_close_state(node.root());
    held.release();
    let actual = request.await;
    let finished = until(|| !remote_cancel::test_close_state(node.root()).0).await;
    let closed = Node::close_agent_runtime(node.root()).await.unwrap();
    assert_retired(&observed);
    assert!(connected && request_pending && fenced && finished);
    assert_eq!(admitted["durability"]["state"], "PersistenceUnknown");
    assert_eq!(admitted["durability"]["code"], "MetadataOutcomeUnknown");
    assert_eq!(while_held, RuntimeClose::Busy);
    assert!(quota.0 && quota.3);
    assert_eq!(actual["durability"]["state"], "DurablyQueued");
    assert_eq!(closed, RuntimeClose::Closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated native acceptance: scripts/run-node-acceptance.py"]
async fn a_joining_step_keeps_its_network_slot_until_the_actual_native_owner_exits() {
    let tree = crate::testdir::TempTree::fresh("agent-close-four-step-slots");
    let nodes: Vec<_> = (0..5)
        .map(|index| Node::open_as(tree.join(format!("root-{index}")), "close-slots").unwrap())
        .collect();
    let observed: Vec<_> = nodes
        .iter()
        .map(|node| ObservedRoot::new(node.root(), "", false))
        .collect();
    let runs: Vec<_> = nodes.iter().map(historical).collect();
    for (node, run) in nodes.iter().zip(&runs) {
        let actual = remote_cancel::metadata(
            node.root(),
            Metadata::Persist {
                run: run.clone(),
                reason: "Stop".into(),
            },
        )
        .unwrap();
        assert!(actual.contains("DurablyQueued"));
    }
    let mut held: Vec<_> = nodes
        .iter()
        .zip(&runs)
        .map(|(node, run)| HeldRun::new(node, run))
        .collect();
    let mut connected = true;
    for index in 0..4 {
        remote_cancel::test_resume_step(nodes[index].root(), &runs[index]);
        connected &= until(|| held[index].connected()).await;
    }
    let closing_root = nodes[0].root().clone();
    let closing = tokio::spawn(async move { Node::close_agent_runtime(&closing_root).await });
    let fenced = until(|| remote_cancel::test_close_state(nodes[0].root()).0).await;
    remote_cancel::test_ready_durable_step(nodes[4].root(), &runs[4]);
    let reserved_count = remote_cancel::test_close_state(nodes[0].root()).2;
    let fifth_not_started = observed[4].pids().is_empty();
    let closing_not_finished = !closing.is_finished();
    held[1].release();
    let fifth_connected = until(|| {
        remote_cancel::pump(nodes[4].root(), true);
        held[4].connected()
    })
    .await;
    let after_replacement_count = remote_cancel::test_close_state(nodes[0].root()).2;
    for run in &mut held {
        run.release();
    }
    let first_closed = closing.await.unwrap();
    let mut others_closed = Vec::new();
    for node in &nodes[1..] {
        others_closed.push(close_after_progress(node).await);
    }
    for owner in &observed {
        assert_retired(owner);
    }
    assert!(connected && fenced && fifth_connected && fifth_not_started && closing_not_finished);
    assert_eq!(reserved_count, 4);
    assert_eq!(after_replacement_count, 4);
    assert_eq!(first_closed.unwrap(), RuntimeClose::Closed);
    assert!(
        others_closed
            .iter()
            .all(|result| *result == RuntimeClose::Closed)
    );
    assert_eq!(observed[4].pids().len(), 1);
}

fn configured(tree: &crate::testdir::TempTree) -> Node {
    let node = Node::open_as(tree.join("store"), "close-producer").unwrap();
    let account = node
        .register_account(
            "fixture",
            crate::Provider::Newapi,
            crate::AccountKind::Bill,
            Some("http://127.0.0.1:9"),
            "close-fixture-key",
            None,
        )
        .unwrap();
    node.chat
        .set_agent_config(&AgentConfig {
            home: node.device_str().into(),
            account: account.id,
            model: "fixture".into(),
            options: Default::default(),
        })
        .unwrap();
    node
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated native acceptance: scripts/run-node-acceptance.py"]
async fn controlled_spawn_failure_keeps_claim_through_the_actual_final_receipt_save() {
    let tree = crate::testdir::TempTree::fresh("agent-close-factory-failure");
    let node = configured(&tree);
    let observed = ObservedRoot::new(node.root(), "spawn-failure-save", true);
    let root = node.root().clone();
    let prompt = std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                Node::open(root)
                    .unwrap()
                    .khor_agent_here(Action::Prompt {
                        token: "close-spawn-failure-1234".into(),
                        text: "controlled factory error".into(),
                    })
                    .await
            })
    });
    let entered = until(|| observed.hook.gate.entered.load(Ordering::SeqCst)).await;
    let while_saving = Node::close_agent_runtime(node.root()).await.unwrap();
    let claim_busy = matches!(close_claim(node.root(), false).unwrap(), CloseClaim::Busy);
    observed.hook.gate.release();
    let report = prompt.join().unwrap().unwrap();
    let record: Saved = serde_json::from_slice(
        &fs::read(file(node.root(), report.run.as_deref().unwrap())).unwrap(),
    )
    .unwrap();
    let closed = Node::close_agent_runtime(node.root()).await.unwrap();
    assert_retired(&observed);
    assert!(entered);
    assert_eq!(while_saving, RuntimeClose::Busy);
    assert!(claim_busy);
    assert_eq!(record.record.status, "failed");
    assert!(
        record
            .record
            .error
            .unwrap()
            .contains("controlled executor factory failure")
    );
    assert_eq!(closed, RuntimeClose::Closed);
    assert!(!observed.hook.gate.expired.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "isolated native acceptance: scripts/run-node-acceptance.py"]
async fn running_token_retry_is_idempotent_and_its_record_writer_blocks_close() {
    let tree = crate::testdir::TempTree::fresh("agent-close-token-retry");
    let node = configured(&tree);
    let mut observed = ObservedRoot::new(node.root(), "executor-start", false);
    let token = "close-same-token-fixture-1234";
    let text = "real executor input held before startup";
    let first = node
        .khor_agent_here(Action::Prompt {
            token: token.into(),
            text: text.into(),
        })
        .await
        .unwrap();
    let executor_entered = until(|| observed.hook.gate.entered.load(Ordering::SeqCst)).await;
    let executor_gate = observed.point("retry-record");
    let root = node.root().clone();
    let retry = std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                Node::open(root)
                    .unwrap()
                    .khor_agent_here(Action::Prompt {
                        token: token.into(),
                        text: text.into(),
                    })
                    .await
            })
    });
    let retry_entered = until(|| observed.hook.gate.entered.load(Ordering::SeqCst)).await;
    let while_recording = Node::close_agent_runtime(node.root()).await.unwrap();
    let running_claim_busy = matches!(close_claim(node.root(), false).unwrap(), CloseClaim::Busy);
    observed.hook.gate.release();
    let second = retry.join().unwrap();
    executor_gate.release();
    // The actual executor now performs its ordinary request to the unreachable
    // fixture endpoint and retires normally. Its result is never synthesized.
    let closed = close_after_progress(&node).await;
    let actual_terminal: Saved = serde_json::from_slice(
        &fs::read(file(node.root(), first.run.as_deref().unwrap())).unwrap(),
    )
    .unwrap();
    let running_retry_gate = observed.point("retry-record");
    let completed_claim_free = matches!(
        close_claim(node.root(), false).unwrap(),
        CloseClaim::Held(_)
    );
    let completed_root = node.root().clone();
    let completed_retry = std::thread::spawn(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                Node::open(completed_root)
                    .unwrap()
                    .khor_agent_here(Action::Prompt {
                        token: token.into(),
                        text: text.into(),
                    })
                    .await
            })
    });
    let completed_entered = until(|| observed.hook.gate.entered.load(Ordering::SeqCst)).await;
    let while_completed_recording = Node::close_agent_runtime(node.root()).await.unwrap();
    // Prove that the completed-token fast path is fenced by configuration
    // admission independently of the already-finished executor's claim.
    let completed_claim_still_free = matches!(
        close_claim(node.root(), false).unwrap(),
        CloseClaim::Held(_)
    );
    observed.hook.gate.release();
    let third = completed_retry.join().unwrap();
    let completed_closed = close_after_progress(&node).await;
    assert_retired(&observed);
    assert!(executor_entered && retry_entered);
    assert_eq!(while_recording, RuntimeClose::Busy);
    assert!(running_claim_busy);
    assert_eq!(second.unwrap().run, first.run);
    assert_eq!(closed, RuntimeClose::Closed);
    assert!(actual_terminal.record.ended_at_ms.is_some());
    assert!(completed_entered && completed_claim_free && completed_claim_still_free);
    assert_eq!(while_completed_recording, RuntimeClose::Busy);
    assert_eq!(third.unwrap().run, first.run);
    assert_eq!(completed_closed, RuntimeClose::Closed);
    assert!(!executor_gate.expired.load(Ordering::SeqCst));
    assert!(!running_retry_gate.expired.load(Ordering::SeqCst));
    assert!(!observed.hook.gate.expired.load(Ordering::SeqCst));
}
