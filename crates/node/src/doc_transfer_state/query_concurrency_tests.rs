//! One cross-process diagnostic, not a synchronization policy or a retry loop.
//! Only the exact owned helper installs this root-scoped native-lock barrier.
use crate::doc_transfer_state::{Context, State};
use crate::proto::{DocChunk, DocTransferState};
use crate::{Node, list::Arrange};
use khor_core::SessionId;
use khor_sync::devices::{DeviceDoc, devices_dir};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const WORKER: &str = "link::documents::query_concurrency_tests::owned_devices_step_worker";
const WORKER_ENV: &str = "KHOR_QUERY_CONCURRENCY_CONTROL";
const BY: Duration = Duration::from_secs(3);
const HOLD: Duration = Duration::from_secs(10);
const MAX_FILE: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Control {
    root: PathBuf,
    owner: u32,
    source: [u8; 32],
    packet: DocChunk,
    marker: PathBuf,
    release: PathBuf,
    publication: Option<Publication>,
}
#[derive(Serialize, Deserialize)]
struct Publication {
    marker: PathBuf,
    release: PathBuf,
}
#[derive(Serialize, Deserialize)]
struct Proof {
    pid: u32,
    parent: u32,
    root: PathBuf,
    phase: String,
}
enum Phase {
    Root(Control),
    Publication {
        root: PathBuf,
        local: [u8; 32],
        paths: Publication,
    },
}
static BARRIER: OnceLock<Mutex<Option<Phase>>> = OnceLock::new();

pub(crate) fn root_acquired(context: &Context) {
    let Some(barrier) = BARRIER.get() else { return };
    let control = {
        let mut slot = barrier.lock().unwrap();
        if !matches!(slot.as_ref(), Some(Phase::Root(c)) if c.root == context.root) {
            return;
        }
        let Some(Phase::Root(control)) = slot.take() else {
            unreachable!()
        };
        control
    };
    publish(
        &control.marker,
        &Proof {
            pid: std::process::id(),
            parent: unsafe { libc::getppid() } as u32,
            root: context.root.clone(),
            phase: "devices-apply-root-acquired".into(),
        },
    );
    if let Some(paths) = control.publication {
        // Same owner, next physical stage: the RootGate must leave Context.apply
        // before this source's later real Journal.save can reach the hook below.
        *barrier.lock().unwrap() = Some(Phase::Publication {
            root: context.root.clone(),
            local: context.local,
            paths,
        });
    } else {
        wait_release(&control.release);
    }
}
fn wait_release(path: &Path) {
    let until = Instant::now() + HOLD;
    while !path.is_file() {
        assert!(
            Instant::now() < until,
            "owned stage barrier release deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[derive(Serialize, Deserialize)]
struct PublicationProof {
    pid: u32,
    parent: u32,
    root: PathBuf,
    phase: String,
    root_free: bool,
    metadata_held: bool,
    needs_recovery: bool,
    next_generation: u64,
    next_bytes: usize,
    next_blake3: String,
    ordinary_wait_ms: u128,
}
pub(crate) fn publication_ready(
    directory: &Path,
    metadata_held: bool,
    wait: Duration,
    image: (&[u8], &[u8]),
) {
    let Some(barrier) = BARRIER.get() else { return };
    let (root, local, paths) = {
        let mut slot = barrier.lock().unwrap();
        if !matches!(slot.as_ref(), Some(Phase::Publication { root, .. }) if root.join(".khor/doc-transfer-state") == directory)
        {
            return;
        }
        let Some(Phase::Publication { root, local, paths }) = slot.take() else {
            unreachable!()
        };
        (root, local, paths)
    };
    assert!(
        metadata_held,
        "actual writer must still own its metadata Gate"
    );
    let next = directory.join("registry.next");
    let metadata = fs::symlink_metadata(&next).unwrap();
    assert!(metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= MAX_FILE);
    let bytes = fs::read(&next).unwrap();
    let (header, body) = image;
    assert_eq!(header.len(), 56);
    assert_eq!(&header[..8], b"khor-d01");
    assert_eq!(bytes.len(), header.len() + body.len());
    assert_eq!(&bytes[..56], header);
    assert_eq!(
        &bytes[56..],
        body,
        "proof is this actual State.save image, not a hand-written next file"
    );
    assert_eq!(
        u64::from_be_bytes(header[16..24].try_into().unwrap()),
        body.len() as u64
    );
    let mut checksum = blake3::Hasher::new();
    checksum.update(&header[..24]);
    checksum.update(body);
    assert_eq!(&header[24..56], checksum.finalize().as_bytes());
    // This is a successful zero-wait physical OS fence acquisition on the
    // SAME source, not a promise inferred from the import's completion log.
    let root_gate = crate::doc_transfer_state::RootPermit::try_acquire(&root)
        .unwrap()
        .attach()
        .expect("Context.apply has released the real root fence");
    drop(root_gate);
    let needs_recovery = crate::doc_transfer_state::pairing::needs_recovery(&root, local).unwrap();
    assert!(
        needs_recovery,
        "real next publication must exercise the conservative constructor branch"
    );
    publish(
        &paths.marker,
        &PublicationProof {
            pid: std::process::id(),
            parent: unsafe { libc::getppid() } as u32,
            root,
            phase: "journal-next-synced-before-rename".into(),
            root_free: true,
            metadata_held,
            needs_recovery,
            next_generation: u64::from_be_bytes(header[8..16].try_into().unwrap()),
            next_bytes: bytes.len(),
            next_blake3: blake3::hash(&bytes).to_hex().to_string(),
            ordinary_wait_ms: wait.as_millis(),
        },
    );
    wait_release(&paths.release);
}

fn publish(path: &Path, value: &impl Serialize) {
    let bytes = serde_json::to_vec(value).unwrap();
    assert!(bytes.len() as u64 <= MAX_FILE);
    assert!(!path.exists());
    let next = path.with_extension("new");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&next)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    drop(file);
    fs::rename(next, path).unwrap();
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    let metadata = fs::symlink_metadata(path).unwrap();
    assert!(metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= MAX_FILE);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
struct Fixture {
    root: PathBuf,
    children_done: Arc<AtomicBool>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-query-sync-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root: root.canonicalize().unwrap(),
            children_done: Arc::new(AtomicBool::new(true)),
        }
    }
    fn node(&self, label: &str) -> Node {
        let root = self.root.join(label);
        fs::create_dir_all(root.join(".khor")).unwrap();
        fs::write(
            root.join(".khor/relays.json"),
            br#"{"relays":[],"only_mine":true}"#,
        )
        .unwrap();
        Node::open_hosting(root, label, false).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.children_done.load(Ordering::SeqCst) {
            let _ = fs::remove_dir_all(&self.root);
        } else {
            eprintln!(
                "preserving owned query fixture: parent cleanup not confirmed: {}",
                self.root.display()
            );
        }
    }
}
struct Owned {
    child: Child,
    done: bool,
    children_done: Arc<AtomicBool>,
}
impl Owned {
    fn spawn(mut command: Command, fixture: &Fixture) -> Self {
        let child = command.spawn().unwrap();
        fixture.children_done.store(false, Ordering::SeqCst);
        eprintln!(
            "[QUERY-OWNED] pid={} parent={}",
            child.id(),
            std::process::id()
        );
        Self {
            child,
            done: false,
            children_done: fixture.children_done.clone(),
        }
    }
    fn wait(&mut self, by: Duration) -> ExitStatus {
        let until = Instant::now() + by;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                self.done = true;
                // The fixture runs children strictly serially except for this
                // one background step. Its dedicated handle remains authoritative.
                return status;
            }
            assert!(
                Instant::now() < until,
                "owned query process deadline: {}",
                self.child.id()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
            let until = Instant::now() + Duration::from_secs(2);
            while Instant::now() < until {
                if self.child.try_wait().is_ok_and(|s| s.is_some()) {
                    self.done = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        // false is retained until the parent has explicitly reaped ALL children.
        if !self.done {
            self.children_done.store(false, Ordering::SeqCst);
        }
    }
}
struct Release(PathBuf);
impl Drop for Release {
    fn drop(&mut self) {
        if !self.0.exists() {
            publish(&self.0, &true);
        }
    }
}
fn birth(pid: u32, fixture: &Fixture, label: &str, expected_exe: &Path) -> String {
    let path = fixture.root.join(format!("{label}.identity"));
    let mut cmd = Command::new("/bin/ps");
    cmd.args(["-ww", "-p", &pid.to_string(), "-o", "lstart=,comm="])
        .stdin(Stdio::null())
        .stdout(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .unwrap(),
        )
        .stderr(Stdio::null());
    assert!(Owned::spawn(cmd, fixture).wait(BY).success());
    let identity = output(&path).trim().to_owned();
    assert!(!identity.is_empty() && identity.len() <= 4096);
    assert!(
        identity.ends_with(expected_exe.to_str().unwrap()),
        "owned actual executable identity: {identity}"
    );
    identity
}
fn ids(node: &Node) -> Vec<String> {
    node.sessions_arranged(Arrange::Recent)
        .unwrap()
        .into_iter()
        .map(|r| r.view.session.id.0)
        .collect()
}
fn output(path: &Path) -> String {
    let metadata = fs::metadata(path).unwrap();
    assert!(metadata.len() <= MAX_FILE);
    fs::read_to_string(path).unwrap()
}
struct Query {
    process: Owned,
    out: PathBuf,
    err: PathBuf,
    started: Instant,
}
impl Query {
    fn start(cli: &Path, fixture: &Fixture, root: &Path, stage: &str) -> Self {
        let out = fixture.root.join(format!("{stage}.stdout"));
        let err = fixture.root.join(format!("{stage}.stderr"));
        let mut cmd = Command::new(cli);
        cmd.args(["sessions", "--all"])
            .env("KHOR_HOME", root)
            .env("KHOR_NAME", "receiver")
            .env("HOME", root)
            .env("XDG_CONFIG_HOME", root.join(".config"))
            .env("KHOR_NO_MDNS", "1")
            .env("KHOR_WEB_PORT", "0")
            .env("KHOR_TMUX_SOCKET", "query-sync-private-unused")
            .env("TMUX_TMPDIR", fixture.root.join("tmux-unused"))
            .env_remove("KHOR_SESSION")
            .env_remove("KHOR_RELAY")
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .stdin(Stdio::null())
            .stdout(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&out)
                    .unwrap(),
            )
            .stderr(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&err)
                    .unwrap(),
            );
        // Include process creation and any parent scheduling delay. This
        // observation is used to exclude the actual ordinary admission budget.
        let started = Instant::now();
        let process = Owned::spawn(cmd, fixture);
        Self {
            process,
            out,
            err,
            started,
        }
    }
    fn finish(mut self) -> (ExitStatus, String, String, u128) {
        let status = self.process.wait(BY);
        (
            status,
            output(&self.out),
            output(&self.err),
            self.started.elapsed().as_millis(),
        )
    }
    fn entered_gate(&mut self, cli: &Path, fixture: &Fixture, root: &Path) -> String {
        self.entered_file(cli, fixture, &root.join(".khor/trust-mutation/gate"))
    }
    fn entered_file(&mut self, cli: &Path, fixture: &Fixture, gate: &Path) -> String {
        let pid = self.process.child.id();
        let born = birth(pid, fixture, "query-before", cli);
        let until = Instant::now() + BY;
        for index in 0..64 {
            if let Some(status) = self.process.child.try_wait().unwrap() {
                self.process.done = true;
                panic!(
                    "query exited before release: {status}, stderr={}",
                    output(&self.err)
                );
            }
            assert!(
                Instant::now() < until,
                "query must open the exact gate before release: {}",
                gate.display()
            );
            let out = fixture.root.join(format!("gate-fd-{index}.stdout"));
            let err = fixture.root.join(format!("gate-fd-{index}.stderr"));
            let mut cmd = Command::new("/usr/sbin/lsof");
            cmd.args(["-nP", "-a", "-p", &pid.to_string(), "-Fn", "--"])
                .arg(&gate)
                .stdin(Stdio::null())
                .stdout(
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .open(&out)
                        .unwrap(),
                )
                .stderr(
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .open(&err)
                        .unwrap(),
                );
            let status = Owned::spawn(cmd, fixture).wait(BY);
            let descriptors = output(&out);
            let why = output(&err);
            if status.success()
                && descriptors
                    .lines()
                    .any(|line| line == format!("n{}", gate.display()))
            {
                assert_eq!(
                    birth(pid, fixture, "query-held", cli),
                    born,
                    "query PID/executable changed"
                );
                assert!(
                    self.process.child.try_wait().unwrap().is_none(),
                    "query ended before the proven release boundary"
                );
                return descriptors;
            }
            assert_eq!(
                status.code(),
                Some(1),
                "unexpected gate probe failure: {why}"
            );
            assert!(
                why.is_empty(),
                "gate observation error must not masquerade as pending: {why}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!(
            "bounded exact-gate descriptor probes exhausted: {}",
            gate.display()
        );
    }
}

struct PreparedFixture {
    fixture: Fixture,
    expected: Vec<String>,
    baseline: Vec<String>,
    target: Context,
    source: Context,
    packet: DocChunk,
}
impl PreparedFixture {
    fn new() -> Self {
        let fixture = Fixture::new();
        let receiver = fixture.node("receiver");
        let expected: Vec<_> = (0..10).map(|i| format!("tui/query-{i:02}")).collect();
        for id in &expected {
            let id = SessionId(id.clone());
            receiver
                .live
                .register(&id, "tui", "private query row", None, None)
                .unwrap();
            receiver.roster_new(&id).unwrap();
        }
        let baseline = ids(&receiver);
        for id in &expected {
            assert!(baseline.contains(id), "baseline missing {id}");
        }
        let target = receiver.doc_context().unwrap();
        let donor = fixture.node("donor-image");
        let source = donor.doc_context().unwrap();
        // A small real devices image, no Pair/RPC/endpoint or external source.
        let mut loaded =
            khor_sync::store::load_pile::<DeviceDoc>(&devices_dir(&source.root), source.writer)
                .unwrap();
        loaded
            .doc
            .upsert(
                &khor_net::trust::id_hex(&source.local),
                "donor-updated",
                &[],
            )
            .unwrap();
        loaded.store.flush(&loaded.doc).unwrap();
        drop(loaded);
        let have = target.version("devices").unwrap();
        let packet = State::open(&source)
            .unwrap()
            .offer(&source, target.local, "devices", &have)
            .unwrap()
            .expect("real differing devices image");
        assert!(
            packet.bytes.len() < 64 * 1024 && packet.offset == 0,
            "single bounded real devices image"
        );
        Self {
            fixture,
            expected,
            baseline,
            target,
            source,
            packet,
        }
    }
}

#[test]
#[ignore = "one owned cross-process diagnostic; requires pinned private CLI via KHOR_QUERY_CLI"]
fn ordinary_query_waits_for_devices_apply_and_preserves_ids() {
    let cli =
        PathBuf::from(std::env::var_os("KHOR_QUERY_CLI").expect("pinned private CLI required"));
    assert!(cli.is_absolute() && cli.is_file());
    let PreparedFixture {
        fixture,
        expected,
        baseline,
        target,
        source,
        packet,
    } = PreparedFixture::new();
    let control = Control {
        root: target.root.canonicalize().unwrap(),
        owner: std::process::id(),
        source: source.local,
        packet,
        marker: fixture.root.join("root-held.json"),
        release: fixture.root.join("release.json"),
        publication: None,
    };
    let control_path = fixture.root.join("control.json");
    publish(&control_path, &control);
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--ignored",
        "--exact",
        WORKER,
        "--nocapture",
        "--test-threads=1",
    ])
    .env(WORKER_ENV, &control_path)
    .stdin(Stdio::null());
    let mut worker = Owned::spawn(cmd, &fixture);
    // This guard is installed before any assertion after spawn. Failure also
    // releases the actual production lock before scoped child cleanup.
    let release = Release(control.release.clone());
    let born = birth(
        worker.child.id(),
        &fixture,
        "worker-before",
        &std::env::current_exe().unwrap(),
    );
    let until = Instant::now() + BY;
    while !control.marker.is_file() {
        assert!(
            worker.child.try_wait().unwrap().is_none(),
            "worker exited before actual root gate checkpoint"
        );
        assert!(
            Instant::now() < until,
            "real devices root checkpoint deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let proof: Proof = read(&control.marker);
    assert_eq!(proof.pid, worker.child.id());
    assert_eq!(proof.parent, std::process::id());
    assert_eq!(proof.root, control.root);
    assert_eq!(proof.phase, "devices-apply-root-acquired");
    assert!(
        worker.child.try_wait().unwrap().is_none(),
        "owned worker PID must still be unreaped and live"
    );
    assert_eq!(
        birth(
            worker.child.id(),
            &fixture,
            "worker-held",
            &std::env::current_exe().unwrap()
        ),
        born,
        "worker birth/executable must remain pinned before CLI input"
    );
    // Start this ONE query while the real root fence is held. A real FD
    // observation, not a fixed sleep, determines when releasing is valid.
    let mut waiting = Query::start(&cli, &fixture, &target.root, "waiting");
    let gate_descriptors = waiting.entered_gate(&cli, &fixture, &target.root);
    // These are real typed primitives, NOT a text->Busy conversion. They are
    // diagnostics of the constructor's OS gate and metadata gate separately.
    let root_error = crate::doc_transfer_state::RootPermit::try_acquire(&target.root)
        .unwrap()
        .attach()
        .err()
        .expect("control is still zero-wait at the held cross-process root gate");
    let metadata_error = State::open(&target)
        .err()
        .expect("cross-process metadata gate held");
    assert_eq!(root_error.kind, crate::proto::DocTransferFailure::Busy);
    assert_eq!(metadata_error.kind, crate::proto::DocTransferFailure::Busy);
    drop(release);
    let held = waiting.finish();
    assert!(
        held.0.success(),
        "ordinary query must wait rather than fail Busy: {}",
        held.2
    );
    assert!(
        worker.wait(BY).success(),
        "actual devices import must finish after release"
    );
    let after = Query::start(&cli, &fixture, &target.root, "released").finish();
    assert!(after.0.success(), "released CLI: {}", after.2);
    for id in &expected {
        assert!(held.1.contains(id), "waiting query lost {id}");
        assert!(after.1.contains(id), "released CLI lost {id}");
    }
    let reopened = Node::open_hosting(target.root.clone(), "receiver", false).unwrap();
    let final_ids = ids(&reopened);
    for id in &baseline {
        assert!(final_ids.contains(id), "query/sync lost original {id}");
    }
    fixture.children_done.store(true, Ordering::SeqCst);
    eprintln!(
        "[N-QUERY] {}",
        serde_json::json!({"held_exit":held.0.code(),"held_stdout":held.1,"held_stderr":held.2,"held_ms":held.3,"released_exit":after.0.code(),"released_stdout":after.1,"released_stderr":after.2,"released_ms":after.3,"baseline_ids":baseline,"final_ids":final_ids,"original_ids":expected,"root_failure_kind":root_error.kind,"metadata_failure_kind":metadata_error.kind,"phase":proof.phase,"receiver":target.root,"worker_pid":proof.pid,"worker_birth_executable":born,"query_gate_descriptors":gate_descriptors,"constructor_contention":true,"full_network_sync":false})
    );
}

#[test]
#[ignore = "one owned metadata-publication phase diagnostic; requires pinned actual1507 CLI"]
fn metadata_publication_separates_recovery_gate_from_root_admission() {
    let cli =
        PathBuf::from(std::env::var_os("KHOR_QUERY_CLI").expect("pinned actual1507 CLI required"));
    assert!(cli.is_absolute() && cli.is_file());
    let PreparedFixture {
        fixture,
        expected,
        baseline,
        target,
        source,
        packet,
    } = PreparedFixture::new();
    let marker = fixture.root.join("publication-held.json");
    let released = fixture.root.join("publication-release.json");
    let control = Control {
        root: target.root.canonicalize().unwrap(),
        owner: std::process::id(),
        source: source.local,
        packet,
        marker: fixture.root.join("root-reached.json"),
        release: fixture.root.join("unused-root-release.json"),
        publication: Some(Publication {
            marker: marker.clone(),
            release: released.clone(),
        }),
    };
    let control_path = fixture.root.join("control.json");
    publish(&control_path, &control);
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--ignored",
        "--exact",
        WORKER,
        "--nocapture",
        "--test-threads=1",
    ])
    .env(WORKER_ENV, &control_path)
    .stdin(Stdio::null());
    let mut worker = Owned::spawn(cmd, &fixture);
    let release = Release(released);
    let born = birth(
        worker.child.id(),
        &fixture,
        "worker-before",
        &std::env::current_exe().unwrap(),
    );
    let until = Instant::now() + BY;
    while !marker.is_file() {
        assert!(
            worker.child.try_wait().unwrap().is_none(),
            "devices worker ended before real next publication"
        );
        assert!(
            Instant::now() < until,
            "real Journal.next publication checkpoint deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let proof: PublicationProof = read(&marker);
    assert_eq!(proof.pid, worker.child.id());
    assert_eq!(proof.parent, std::process::id());
    assert_eq!(proof.root, control.root);
    assert_eq!(proof.phase, "journal-next-synced-before-rename");
    assert!(proof.root_free && proof.metadata_held && proof.needs_recovery);
    assert_eq!(
        birth(
            worker.child.id(),
            &fixture,
            "worker-held",
            &std::env::current_exe().unwrap()
        ),
        born
    );
    assert!(worker.child.try_wait().unwrap().is_none());
    let next = target.root.join(".khor/doc-transfer-state/registry.next");
    assert!(fs::metadata(&next).unwrap().len() <= MAX_FILE);
    let next_bytes = fs::read(&next).unwrap();
    assert_eq!(next_bytes.len(), proof.next_bytes);
    assert_eq!(
        blake3::hash(&next_bytes).to_hex().to_string(),
        proof.next_blake3
    );
    let root_free = crate::doc_transfer_state::RootPermit::try_acquire(&target.root)
        .unwrap()
        .attach()
        .expect("parent independently acquires real root OS gate");
    drop(root_free);
    assert!(
        crate::doc_transfer_state::pairing::needs_recovery(&target.root, target.local).unwrap()
    );
    let metadata = State::open(&target)
        .err()
        .expect("real metadata gate is held by publication worker");
    assert_eq!(metadata.kind, crate::proto::DocTransferFailure::Busy);
    let held = Query::start(&cli, &fixture, &target.root, "publication").finish();
    // Under this known source-aligned branch, root admission is free and the
    // ENTIRE CLI finished below its original root wait deadline. Not an mtime
    // inference: the pinned next and metadata owner remain blocked throughout.
    assert!(
        !held.0.success(),
        "diagnostic requires the actual publication-time failure"
    );
    assert!(
        held.2.contains("document transfer state busy"),
        "preserve original actual CLI error: {}",
        held.2
    );
    assert!(
        held.1.is_empty(),
        "failure cannot be reported as successful empty rows"
    );
    assert!(
        held.3 < proof.ordinary_wait_ms,
        "cannot exclude a root deadline: {}ms",
        held.3
    );
    assert!(
        worker.child.try_wait().unwrap().is_none(),
        "metadata owner must still be in the proved checkpoint"
    );
    assert_eq!(
        fs::read(&next).unwrap(),
        next_bytes,
        "same real unpublished image remains held"
    );
    let root_free = crate::doc_transfer_state::RootPermit::try_acquire(&target.root)
        .unwrap()
        .attach()
        .expect("root remains independently available after actual Busy");
    drop(root_free);
    drop(release);
    assert!(
        worker.wait(BY).success(),
        "actual import finishes after publication release"
    );
    let after = Query::start(&cli, &fixture, &target.root, "released").finish();
    assert!(after.0.success(), "released actual CLI: {}", after.2);
    for id in &expected {
        assert!(after.1.contains(id), "released query lost {id}");
    }
    let final_ids = ids(&Node::open_hosting(target.root.clone(), "receiver", false).unwrap());
    for id in &baseline {
        assert!(final_ids.contains(id), "original row disappeared: {id}");
    }
    fixture.children_done.store(true, Ordering::SeqCst);
    eprintln!(
        "[N-METADATA-QUERY] {}",
        serde_json::json!({
            "phase_proof":proof,"worker_birth_executable":born,"held_exit":held.0.code(),"held_ms":held.3,"held_stdout":held.1,"held_stderr":held.2,
            "metadata_failure_kind":metadata.kind,"released_exit":after.0.code(),"released_ms":after.3,"released_stdout":after.1,"released_stderr":after.2,
            "original_ids":expected,"baseline_ids":baseline,"final_ids":final_ids,"constructor_recovery_branch":true,"product_fixed":false
        })
    );
}

#[test]
#[ignore = "one owned metadata-publication waiting positive; requires pinned current CLI"]
fn ordinary_query_waits_for_metadata_publication_and_preserves_ids() {
    let cli =
        PathBuf::from(std::env::var_os("KHOR_QUERY_CLI").expect("pinned current CLI required"));
    assert!(cli.is_absolute() && cli.is_file());
    let PreparedFixture {
        fixture,
        expected,
        baseline,
        target,
        source,
        packet,
    } = PreparedFixture::new();
    let marker = fixture.root.join("publication-held.json");
    let released = fixture.root.join("publication-release.json");
    let control = Control {
        root: target.root.canonicalize().unwrap(),
        owner: std::process::id(),
        source: source.local,
        packet,
        marker: fixture.root.join("root-reached.json"),
        release: fixture.root.join("unused-root-release.json"),
        publication: Some(Publication {
            marker: marker.clone(),
            release: released.clone(),
        }),
    };
    let control_path = fixture.root.join("control.json");
    publish(&control_path, &control);
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--ignored",
        "--exact",
        WORKER,
        "--nocapture",
        "--test-threads=1",
    ])
    .env(WORKER_ENV, &control_path)
    .stdin(Stdio::null());
    let mut worker = Owned::spawn(cmd, &fixture);
    let release = Release(released);
    let born = birth(
        worker.child.id(),
        &fixture,
        "worker-before",
        &std::env::current_exe().unwrap(),
    );
    let until = Instant::now() + BY;
    while !marker.is_file() {
        assert!(
            worker.child.try_wait().unwrap().is_none(),
            "devices worker ended before real next publication"
        );
        assert!(
            Instant::now() < until,
            "real Journal.next publication checkpoint deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let proof: PublicationProof = read(&marker);
    assert_eq!(proof.pid, worker.child.id());
    assert_eq!(proof.parent, std::process::id());
    assert_eq!(proof.root, control.root);
    assert_eq!(proof.phase, "journal-next-synced-before-rename");
    assert!(proof.root_free && proof.metadata_held && proof.needs_recovery);
    assert_eq!(
        birth(
            worker.child.id(),
            &fixture,
            "worker-held",
            &std::env::current_exe().unwrap()
        ),
        born
    );
    assert!(worker.child.try_wait().unwrap().is_none());
    let next = target.root.join(".khor/doc-transfer-state/registry.next");
    assert!(fs::metadata(&next).unwrap().len() <= MAX_FILE);
    let next_bytes = fs::read(&next).unwrap();
    assert_eq!(next_bytes.len(), proof.next_bytes);
    assert_eq!(
        blake3::hash(&next_bytes).to_hex().to_string(),
        proof.next_blake3
    );
    let root_free = crate::doc_transfer_state::RootPermit::try_acquire(&target.root)
        .unwrap()
        .attach()
        .expect("parent independently acquires real root OS gate");
    drop(root_free);
    assert!(
        crate::doc_transfer_state::pairing::needs_recovery(&target.root, target.local).unwrap()
    );
    let metadata = State::open(&target)
        .err()
        .expect("real metadata gate is held by publication worker");
    assert_eq!(metadata.kind, crate::proto::DocTransferFailure::Busy);
    let mut waiting = Query::start(&cli, &fixture, &target.root, "publication");
    let gate_descriptors = waiting.entered_file(
        &cli,
        &fixture,
        &target.root.join(".khor/doc-transfer-state/gate"),
    );
    assert!(worker.child.try_wait().unwrap().is_none());
    assert_eq!(fs::read(&next).unwrap(), next_bytes);
    // The actual waiting CLI must have released its root fence before waiting
    // for this metadata owner. This independent physical try proves handoff.
    let root_free = crate::doc_transfer_state::RootPermit::try_acquire(&target.root)
        .unwrap()
        .attach()
        .expect("waiting CLI released root before metadata wait");
    drop(root_free);
    drop(release);
    let held = waiting.finish();
    assert!(held.0.success(), "metadata waiting CLI: {}", held.2);
    assert!(
        worker.wait(BY).success(),
        "actual import finishes after publication release"
    );
    let after = Query::start(&cli, &fixture, &target.root, "released").finish();
    assert!(after.0.success(), "released actual CLI: {}", after.2);
    for id in &expected {
        assert!(held.1.contains(id), "metadata waiting query lost {id}");
        assert!(after.1.contains(id), "released query lost {id}");
    }
    let final_ids = ids(&Node::open_hosting(target.root.clone(), "receiver", false).unwrap());
    for id in &baseline {
        assert!(final_ids.contains(id), "original row disappeared: {id}");
    }
    fixture.children_done.store(true, Ordering::SeqCst);
    eprintln!(
        "[N-METADATA-WAIT] {}",
        serde_json::json!({
            "phase_proof":proof,"worker_birth_executable":born,"held_exit":held.0.code(),"held_ms":held.3,"held_stdout":held.1,"held_stderr":held.2,
            "metadata_failure_kind":metadata.kind,"released_exit":after.0.code(),"released_ms":after.3,"released_stdout":after.1,"released_stderr":after.2,
            "original_ids":expected,"baseline_ids":baseline,"final_ids":final_ids,"constructor_recovery_branch":true,"query_metadata_descriptors":gate_descriptors,"current_cli_positive":true
        })
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "owned devices step helper only, not an independent parent case"]
async fn owned_devices_step_worker() {
    let path = PathBuf::from(std::env::var_os(WORKER_ENV).expect("owned control only"));
    let control: Control = read(&path);
    assert_eq!(unsafe { libc::getppid() } as u32, control.owner);
    assert_eq!(control.root.canonicalize().unwrap(), control.root);
    let node = Node::open_hosting(control.root.clone(), "receiver", false).unwrap();
    let context = node.doc_context().unwrap();
    let packet = control.packet.clone();
    let remote = control.source;
    *BARRIER.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(Phase::Root(control));
    let result = crate::link::documents::on_worker(context, move |context| {
        let mut state = State::open(context)?;
        state.receive(context, remote, &packet, true)
    })
    .await
    .unwrap();
    assert!(
        matches!(result, DocTransferState::Complete { .. }),
        "single real devices import incomplete: {result:?}"
    );
    eprintln!("[N-DEVICES-STEP] complete=true pid={}", std::process::id());
}
