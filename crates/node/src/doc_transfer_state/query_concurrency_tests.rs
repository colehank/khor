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
}
#[derive(Serialize, Deserialize)]
struct Proof {
    pid: u32,
    parent: u32,
    root: PathBuf,
    phase: String,
}
static BARRIER: OnceLock<Mutex<Option<Control>>> = OnceLock::new();

pub(crate) fn root_acquired(context: &Context) {
    let Some(barrier) = BARRIER.get() else { return };
    let control = {
        let mut slot = barrier.lock().unwrap();
        if !slot.as_ref().is_some_and(|c| c.root == context.root) {
            return;
        }
        slot.take().unwrap()
    };
    publish(
        &control.marker,
        &Proof {
            pid: std::process::id(),
            // The controller verifies this actual child PID and an unreaped Child
            // handle before it allows any CLI input. This is not a persisted PID lease.
            parent: unsafe { libc::getppid() } as u32,
            root: context.root.clone(),
            phase: "devices-apply-root-acquired".into(),
        },
    );
    let until = Instant::now() + HOLD;
    while !control.release.is_file() {
        assert!(
            Instant::now() < until,
            "owned root barrier release deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
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
                "preserving owned query fixture: child not reaped: {}",
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
        Self {
            process: Owned::spawn(cmd, fixture),
            out,
            err,
            started: Instant::now(),
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
        let pid = self.process.child.id();
        let born = birth(pid, fixture, "query-before", cli);
        let gate = root.join(".khor/trust-mutation/gate");
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
                "query must open the exact root gate before release"
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
        panic!("bounded root-gate descriptor probes exhausted");
    }
}

#[test]
#[ignore = "one owned cross-process diagnostic; requires pinned private CLI via KHOR_QUERY_CLI"]
fn ordinary_query_waits_for_devices_apply_and_preserves_ids() {
    let cli =
        PathBuf::from(std::env::var_os("KHOR_QUERY_CLI").expect("pinned private CLI required"));
    assert!(cli.is_absolute() && cli.is_file());
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
    let control = Control {
        root: target.root.canonicalize().unwrap(),
        owner: std::process::id(),
        source: source.local,
        packet,
        marker: fixture.root.join("root-held.json"),
        release: fixture.root.join("release.json"),
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
    *BARRIER.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(control);
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
