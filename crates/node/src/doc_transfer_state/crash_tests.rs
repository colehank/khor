//! Process-death acceptance at the real checked-flush/receipt boundary.
//! The only pause is installed by the ignored receiver worker for its own
//! root and exact immutable manifest. Normal builds have no checkpoint.
use super::*;
use crate::Node;
use khor_sync::pins::{PinDoc, pins_dir};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::{fs::OpenOptionsExt, process::ExitStatusExt},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::timeout;
#[path = "../../tests/util/mod.rs"]
mod util;

const BY: Duration = Duration::from_secs(90);
const WORKER_ENV: &str = "KHOR_DOC_CRASH_CASE";
const RECEIVER: &str = "doc_transfer_state::crash_tests::postflush_receiver_worker";
const VERIFIER: &str = "doc_transfer_state::crash_tests::fresh_importing_verifier_worker";
const PRIMARY: &str = "tui/crash-primary";
const CONCURRENT: &str = "tui/crash-local-concurrent";
const MAX_CONTROL_BYTES: u64 = 1024 * 1024;

struct Checkpoint {
    root: PathBuf,
    manifest: DocManifest,
    marker: PathBuf,
}
static CHECKPOINT: OnceLock<Mutex<Option<Checkpoint>>> = OnceLock::new();

/// Called only after Context::apply returned and its Loaded and Sink dropped,
/// while the committed journal still says Importing, before any Complete save.
pub(super) fn after_flush(context: &Context, manifest: &DocManifest, state: &State) {
    let Some(slot) = CHECKPOINT.get() else {
        return;
    };
    let checkpoint = {
        let mut slot = slot.lock().unwrap();
        if !slot
            .as_ref()
            .is_some_and(|c| c.root == context.root && c.manifest == *manifest)
        {
            return;
        }
        slot.take().unwrap()
    };
    assert!(state.records.incoming.iter().any(|r| matches!(r,
        Incoming::Importing { manifest: m, .. } if m == manifest)));
    assert!(!state.records.incoming.iter().any(|r| matches!(r,
        Incoming::Complete { proof, .. } if proof.manifest == *manifest)));
    let version = context.version("pins").unwrap();
    let descriptor = manifest.descriptor().unwrap();
    assert_eq!(version.epoch, descriptor.epoch);
    assert!(version.version.includes_vv(&descriptor.target));
    assert!(pinned(context, PRIMARY) && pinned(context, CONCURRENT));
    publish(
        &checkpoint.marker,
        &FlushMarker {
            pid: std::process::id(),
            manifest_id: manifest.id().unwrap(),
            epoch: version.epoch,
            version: version.encode().unwrap(),
            importing: true,
            primary_present: true,
            concurrent_present: true,
        },
    );
    // A finite watchdog makes an un-killed worker a failure, never a reply.
    let by = Instant::now() + BY;
    while Instant::now() < by {
        std::thread::park_timeout(Duration::from_millis(100));
    }
    panic!("owned receiver was not killed at the checked-flush checkpoint");
}

#[derive(Serialize, Deserialize)]
struct Ready {
    identity: String,
    invite: String,
}
#[derive(Serialize, Deserialize)]
struct Prepared {
    identity: String,
    have: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct FlushMarker {
    pid: u32,
    manifest_id: String,
    epoch: u32,
    version: Vec<u8>,
    importing: bool,
    primary_present: bool,
    concurrent_present: bool,
}
#[derive(Serialize, Deserialize)]
struct VerifyResult {
    pid: u32,
    importing_on_fresh_open: bool,
    equivalent_proof_stable: bool,
    concurrent_pin_and_epoch_preserved: bool,
    old_stage_released: bool,
    new_receiving_preserved: bool,
    late_write_missing: bool,
}

fn publish<T: Serialize>(path: &Path, value: &T) {
    let bytes = serde_json::to_vec(value).unwrap();
    assert!(bytes.len() as u64 <= MAX_CONTROL_BYTES);
    let temp = path.with_extension("publishing");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    drop(file);
    fs::rename(temp, path).unwrap();
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    assert!(fs::metadata(path).unwrap().len() <= MAX_CONTROL_BYTES);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
async fn wait_file(path: &Path) {
    timeout(BY, async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("owned worker must reach the actual checkpoint");
}
fn context(node: &Node) -> Context {
    Context {
        root: node.root().clone(),
        writer: node.writer_peer(),
        local: khor_net::trust::id_from_hex(node.device_str()).unwrap(),
    }
}
fn with_state<T>(context: &Context, run: impl FnOnce(&mut State) -> T) -> T {
    run(&mut State::open(context).unwrap())
}
fn pin(context: &Context, name: &str) {
    let mut loaded =
        khor_sync::store::load_pile::<PinDoc>(&pins_dir(&context.root), context.writer).unwrap();
    loaded.doc.set(name, true).unwrap();
    loaded.store.flush(&loaded.doc).unwrap();
}
fn pinned(context: &Context, name: &str) -> bool {
    let loaded =
        khor_sync::store::load_pile::<PinDoc>(&pins_dir(&context.root), context.writer).unwrap();
    loaded.doc.pinned(name)
}
fn proof(value: DocTransferState) -> DocProof {
    match value {
        DocTransferState::Complete { proof } => proof,
        other => panic!("expected real complete receipt: {other:?}"),
    }
}

struct Case {
    root: PathBuf,
    safe_to_remove: Arc<AtomicBool>,
}
impl Drop for Case {
    fn drop(&mut self) {
        // Preserve complete child stdout in the parent raw log before removing
        // the owned data; private control JSON (including the invite) is never logged.
        for name in ["receiver.log", "verifier.log"] {
            let path = self.root.join(name);
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(log) => eprintln!("[Q039-CRASH-WORKER-LOG] {name}\n{log}"),
                    Err(error) => {
                        self.safe_to_remove.store(false, Ordering::Release);
                        eprintln!("[Q039-CRASH-CLEANUP-FAIL] child log unavailable: {error}");
                    }
                }
            }
        }
        if self.safe_to_remove.load(Ordering::Acquire) {
            if let Err(error) = fs::remove_dir_all(&self.root) {
                eprintln!("[Q039-CRASH-CLEANUP-FAIL] {error}");
                assert!(
                    std::thread::panicking(),
                    "private crash data cleanup failed"
                );
            }
        } else {
            eprintln!("[Q039-CRASH-CLEANUP-FAIL] live owned worker; preserving private root");
        }
    }
}
struct Worker {
    child: Child,
    stopped: bool,
    safe_to_remove: Arc<AtomicBool>,
}
impl Worker {
    fn start(case: &Case, exact: &str, log: &str) -> Self {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(case.root.join(log))
            .unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                exact,
                "--nocapture",
                "--test-threads=1",
            ])
            .env(WORKER_ENV, &case.root)
            .stdin(Stdio::null())
            .stdout(file.try_clone().unwrap())
            .stderr(file)
            .spawn()
            .unwrap();
        Self {
            child,
            stopped: false,
            safe_to_remove: case.safe_to_remove.clone(),
        }
    }
    fn wait(&mut self, limit: Duration) -> ExitStatus {
        let by = Instant::now() + limit;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                self.stopped = true;
                return status;
            }
            assert!(Instant::now() < by, "owned child exceeded its finite wait");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn kill_at_checkpoint(&mut self) {
        self.child.kill().unwrap();
        let status = self.wait(Duration::from_secs(5));
        assert_eq!(
            status.signal(),
            Some(9),
            "actual owned receiver must die by SIGKILL"
        );
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if self.stopped {
            return;
        }
        let _ = self.child.kill();
        let by = Instant::now() + Duration::from_secs(5);
        while Instant::now() < by {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    self.stopped = true;
                    return;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(_) => break,
            }
        }
        self.safe_to_remove.store(false, Ordering::Release);
        eprintln!("[Q039-CRASH-CLEANUP-FAIL] owned worker not reaped");
    }
}

// If setup panics while public sync is still active, abort it and preserve
// its root rather than deleting data before the runtime has joined its IO.
struct ActiveSync {
    task: tokio::task::JoinHandle<Result<Vec<(String, Result<String, String>)>, String>>,
    joined: bool,
    safe_to_remove: Arc<AtomicBool>,
}
impl Drop for ActiveSync {
    fn drop(&mut self) {
        if !self.joined {
            self.task.abort();
            self.safe_to_remove.store(false, Ordering::Release);
            eprintln!("[Q039-CRASH-CLEANUP-FAIL] public sync not joined; preserving root");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "isolated native Node process-death acceptance; run --ignored --exact"]
async fn a_real_postflush_sigkill_recovers_equivalent_receipt_and_exact_ack_gc() {
    let root = std::env::temp_dir().join(format!(
        "khor-doc-crash-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let case = Case {
        root: fs::canonicalize(root).unwrap(),
        safe_to_remove: Arc::new(AtomicBool::new(true)),
    };
    for side in ["source", "receiver"] {
        util::hermetic(&case.root.join(side));
    }
    let mut receiver = Worker::start(&case, RECEIVER, "receiver.log");
    wait_file(&case.root.join("ready.json")).await;
    let ready: Ready = read(&case.root.join("ready.json"));
    let source =
        Arc::new(Node::open_hosting(case.root.join("source"), "crash-source", false).unwrap());
    timeout(BY, source.pair(&ready.invite))
        .await
        .unwrap()
        .unwrap();
    let source_context = context(&source);
    let remote = khor_net::trust::id_from_hex(&ready.identity).unwrap();
    // Retire setup receipts through the real public sync before pinning the
    // one immutable target that will be interrupted.
    for _ in 0..6 {
        let rows = timeout(BY, source.sync_now()).await.unwrap().unwrap();
        assert!(
            rows.iter().all(|(_, result)| result.is_ok()),
            "setup public sync must succeed"
        );
        if with_state(&source_context, |state| {
            state.pending_docs(remote).unwrap().is_empty()
        }) {
            break;
        }
    }
    with_state(&source_context, |state| {
        assert!(
            state.pending_docs(remote).unwrap().is_empty(),
            "setup receipts must settle before arming the crash"
        );
    });
    pin(&source_context, PRIMARY);
    publish(&case.root.join("prepare.json"), &true);
    wait_file(&case.root.join("prepared.json")).await;
    let prepared: Prepared = read(&case.root.join("prepared.json"));
    assert_eq!(prepared.identity, ready.identity);
    let have = DocumentVersion::decode(&prepared.have).unwrap();
    let chunk = with_state(&source_context, |state| {
        state
            .offer(&source_context, remote, "pins", &have)
            .unwrap()
            .unwrap()
    });
    assert!(chunk.manifest.total_bytes <= khor_sync::transfer::CHUNK_BYTES as u64);
    publish(&case.root.join("manifest.json"), &chunk.manifest);
    wait_file(&case.root.join("armed.json")).await;
    let armed: String = read(&case.root.join("armed.json"));
    assert_eq!(armed, chunk.manifest.id().unwrap());
    assert!(
        !source.root().join(".khor/endpoint.json").exists(),
        "source must use the real one-shot public Node, not another resident"
    );
    let active_source = source.clone();
    let mut running = ActiveSync {
        task: tokio::spawn(async move { active_source.sync_now().await }),
        joined: false,
        safe_to_remove: case.safe_to_remove.clone(),
    };
    wait_file(&case.root.join("flushed.json")).await;
    let marker: FlushMarker = read(&case.root.join("flushed.json"));
    assert_eq!(marker.pid, receiver.child.id());
    assert_eq!(marker.manifest_id, chunk.manifest.id().unwrap());
    let disk = DocumentVersion::decode(&marker.version).unwrap();
    assert_eq!(disk.epoch, marker.epoch);
    assert_eq!(disk.epoch, chunk.manifest.descriptor().unwrap().epoch);
    assert!(
        disk.version
            .includes_vv(&chunk.manifest.descriptor().unwrap().target)
    );
    assert!(marker.importing && marker.primary_present && marker.concurrent_present);
    receiver.kill_at_checkpoint();
    // Its reply could not be constructed: Complete persistence follows the
    // killed checkpoint. Check actual sender state rather than inventing ACKs.
    let _first = timeout(BY, &mut running.task).await.unwrap().unwrap();
    running.joined = true;
    with_state(&source_context, |state| {
        assert!(
            state.records.outgoing.iter().any(|r| matches!(r,
            Outgoing::Ready(m) if *m == chunk.manifest)),
            "first actual request must not retire its plan as Applied"
        );
        assert!(
            state
                .blobs
                .retained_manifests()
                .unwrap()
                .contains(&chunk.manifest.core().unwrap())
        );
    });
    let mut verifier = Worker::start(&case, VERIFIER, "verifier.log");
    let status = verifier.wait(BY);
    assert!(
        status.success(),
        "fresh verifier process must pass its actual recovery oracles"
    );
    let result: VerifyResult = read(&case.root.join("verified.json"));
    assert_eq!(result.pid, verifier.child.id());
    assert!(
        result.importing_on_fresh_open
            && result.equivalent_proof_stable
            && result.concurrent_pin_and_epoch_preserved
            && result.old_stage_released
            && result.new_receiving_preserved
            && result.late_write_missing
    );
    eprintln!(
        "[Q039-CRASH] actual_sigkill=9 first_no_applied=true fresh_importing=true equivalent_stable=true concurrent_epoch=true exact_gc=true new_id_preserved=true late_write_missing=true"
    );
    let owned_root = case.root.clone();
    drop(verifier);
    drop(receiver);
    drop(source);
    drop(case);
    assert!(!owned_root.exists());
    eprintln!("[Q039-CRASH-CLEANUP] all_owned_children_reaped=true private_nodes_removed=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "actual receiver child, not a separate acceptance parent"]
async fn postflush_receiver_worker() {
    let case = PathBuf::from(
        std::env::var_os(WORKER_ENV).expect("only the owned parent starts this child"),
    );
    util::off_the_segment();
    util::own_tmux();
    let server = Node::open_hosting(case.join("receiver"), "crash-receiver", false).unwrap();
    let task = tokio::spawn(async move { server.serve_resumable().await });
    wait_file(&case.join("receiver/.khor/endpoint.json")).await;
    let node = Node::open_hosting(case.join("receiver"), "crash-receiver", false).unwrap();
    publish(
        &case.join("ready.json"),
        &Ready {
            identity: node.device_str().into(),
            invite: node.invite().unwrap(),
        },
    );
    wait_file(&case.join("prepare.json")).await;
    let ctx = context(&node);
    pin(&ctx, CONCURRENT);
    publish(
        &case.join("prepared.json"),
        &Prepared {
            identity: node.device_str().into(),
            have: ctx.version("pins").unwrap().encode().unwrap(),
        },
    );
    wait_file(&case.join("manifest.json")).await;
    let manifest: DocManifest = read(&case.join("manifest.json"));
    assert_eq!(manifest.recipient, ctx.local);
    let slot = CHECKPOINT.get_or_init(|| Mutex::new(None));
    assert!(
        slot.lock()
            .unwrap()
            .replace(Checkpoint {
                root: ctx.root.clone(),
                manifest: manifest.clone(),
                marker: case.join("flushed.json")
            })
            .is_none()
    );
    publish(&case.join("armed.json"), &manifest.id().unwrap());
    timeout(BY, task)
        .await
        .expect("receiver must reach checkpoint before its watchdog")
        .unwrap()
        .unwrap();
    panic!("receiver exited normally instead of actual SIGKILL");
}

#[test]
#[ignore = "fresh verifier child, not a separate acceptance parent"]
fn fresh_importing_verifier_worker() {
    let case = PathBuf::from(
        std::env::var_os(WORKER_ENV).expect("only the owned parent starts this child"),
    );
    let source_node = Node::open_hosting(case.join("source"), "crash-source", false).unwrap();
    let receiver_node = Node::open_hosting(case.join("receiver"), "crash-receiver", false).unwrap();
    let source = context(&source_node);
    let receiver = context(&receiver_node);
    let manifest: DocManifest = read(&case.join("manifest.json"));
    let descriptor = manifest.descriptor().unwrap();
    assert_eq!(manifest.source, source.local);
    assert_eq!(manifest.recipient, receiver.local);
    let current = receiver.version("pins").unwrap();
    assert_eq!(current.epoch, descriptor.epoch);
    assert!(
        current.version.includes_vv(&descriptor.base)
            && current.version.includes_vv(&descriptor.target)
    );
    assert!(pinned(&receiver, PRIMARY) && pinned(&receiver, CONCURRENT));
    let late = with_state(&source, |s| {
        s.read_outgoing(&source, receiver.local, &manifest, 0)
            .unwrap()
    });
    let recovered =
        with_state(&receiver, |s| {
            assert!(
                s.records.incoming.iter().any(
                    |r| matches!(r, Incoming::Importing { manifest: m, .. } if m == &manifest)
                )
            );
            assert!(!s.records.incoming.iter().any(
                |r| matches!(r, Incoming::Complete { proof, .. } if proof.manifest == manifest)
            ));
            assert!(
                s.blobs
                    .retained_manifests()
                    .unwrap()
                    .contains(&manifest.core().unwrap())
            );
            let p = proof(s.status(&receiver, source.local, &manifest).unwrap());
            assert_eq!(p.kind, DocProofKind::AlreadyHaveEquivalent);
            assert_eq!(
                proof(s.status(&receiver, source.local, &manifest).unwrap()),
                p
            );
            assert_eq!(
                proof(s.receive(&receiver, source.local, &late, true).unwrap()),
                p
            );
            let notices = s.notice_ids(&receiver).unwrap();
            assert_eq!(
                notices
                    .iter()
                    .filter(|n| n.id == manifest.id().unwrap())
                    .count(),
                1
            );
            s.mark_notified(&manifest.id().unwrap()).unwrap();
            p
        });
    with_state(&receiver, |s| {
        assert_eq!(
            proof(s.status(&receiver, source.local, &manifest).unwrap()),
            recovered
        );
        assert!(
            !s.notice_ids(&receiver)
                .unwrap()
                .iter()
                .any(|n| n.id == manifest.id().unwrap())
        );
    });
    let ack = with_state(&source, |s| {
        s.accept(&source, receiver.local, &recovered, false)
            .unwrap()
    });
    // A real new generation is left incomplete before ACKing the old one.
    {
        let mut loaded =
            khor_sync::store::load_pile::<PinDoc>(&pins_dir(&source.root), source.writer).unwrap();
        for i in 0u64..6000 {
            loaded
                .doc
                .set(
                    &format!("tui/{i}/{}", blake3::hash(&i.to_be_bytes()).to_hex()),
                    true,
                )
                .unwrap();
        }
        loaded.store.flush(&loaded.doc).unwrap();
    }
    let have = receiver.version("pins").unwrap();
    let next = with_state(&source, |s| {
        s.offer(&source, receiver.local, "pins", &have)
            .unwrap()
            .unwrap()
    });
    assert_ne!(next.manifest, manifest);
    assert!(next.manifest.total_bytes > khor_sync::transfer::CHUNK_BYTES as u64);
    with_state(&receiver, |s| {
        assert!(
            matches!(s.receive(&receiver, source.local, &next, true).unwrap(), DocTransferState::Stored { next_offset } if next_offset == khor_sync::transfer::CHUNK_BYTES as u64)
        )
    });
    with_state(&receiver, |s| {
        assert!(s.acknowledge([0; 32], &ack).is_err());
        let mut wrong = ack.clone();
        wrong.proof_hash[0] ^= 1;
        assert_eq!(
            s.acknowledge(source.local, &wrong).unwrap(),
            DocAckResult::Reconfirm {
                proof: recovered.clone()
            }
        );
    });
    // Reopen before the correct ACK: an invalid ACK must not have deleted
    // either real stage while merely returning an apparently valid Reconfirm.
    with_state(&receiver, |s| {
        assert!(s.records.incoming.iter().any(|r| matches!(r,
            Incoming::Complete { proof, notify: false, retired_pen: None }
                if proof == &recovered)));
        assert!(s.records.incoming.iter().any(|r| matches!(r,
            Incoming::Receiving(m) if m == &next.manifest)));
        {
            let old = s
                .blobs
                .open_existing(&manifest.core().unwrap())
                .unwrap()
                .expect("wrong ACK must retain the actual old complete stage");
            assert!(matches!(
                old.progress().unwrap(),
                Progress::VerifiedComplete { .. }
            ));
        }
        {
            let new = s
                .blobs
                .open_existing(&next.manifest.core().unwrap())
                .unwrap()
                .expect("wrong ACK must retain the actual new receiving stage");
            assert!(
                matches!(new.progress().unwrap(), Progress::Stored { next_offset }
                if next_offset == khor_sync::transfer::CHUNK_BYTES as u64)
            );
        }
    });
    let released = with_state(&receiver, |s| s.acknowledge(source.local, &ack).unwrap());
    assert_eq!(
        released,
        DocAckResult::Released {
            acknowledgement: ack.clone()
        }
    );
    with_state(&source, |s| {
        s.acknowledged(&source, receiver.local, &released).unwrap()
    });
    with_state(&receiver, |s| {
        assert!(!s.records.incoming.iter().any(|r| r.manifest() == &manifest));
        assert!(
            s.records
                .incoming
                .iter()
                .any(|r| matches!(r, Incoming::Receiving(m) if m == &next.manifest))
        );
        let retained = s.blobs.retained_manifests().unwrap();
        assert!(!retained.contains(&manifest.core().unwrap()));
        assert!(retained.contains(&next.manifest.core().unwrap()));
        assert_eq!(
            s.receive(&receiver, source.local, &late, false).unwrap(),
            DocTransferState::Missing
        );
        assert_eq!(
            s.acknowledge(source.local, &ack).unwrap(),
            DocAckResult::Absent {
                acknowledgement: ack.clone()
            }
        );
        assert_eq!(s.blobs.retained_manifests().unwrap(), retained);
    });
    with_state(&source, |s| {
        assert!(!s.records.outgoing.iter().any(|r| r.manifest() == &manifest));
        assert!(
            s.records
                .outgoing
                .iter()
                .any(|r| matches!(r, Outgoing::Ready(m) if m == &next.manifest))
        );
        assert!(
            s.blobs
                .retained_manifests()
                .unwrap()
                .contains(&next.manifest.core().unwrap())
        );
    });
    publish(
        &case.join("verified.json"),
        &VerifyResult {
            pid: std::process::id(),
            importing_on_fresh_open: true,
            equivalent_proof_stable: true,
            concurrent_pin_and_epoch_preserved: true,
            old_stage_released: true,
            new_receiving_preserved: true,
            late_write_missing: true,
        },
    );
}
