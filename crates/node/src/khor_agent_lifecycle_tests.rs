use super::*;
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn test_name(name: &str) -> String {
    format!("{}::{name}", module_path!().split_once("::").unwrap().1)
}

fn isolated(case: &str) {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-lifecycle-{}",
        crate::link::fresh_hex().unwrap()
    )));
    fs::create_dir(&scratch.0).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &test_name("lifecycle_worker"),
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("KHOR_LIFECYCLE_CASE", case)
            .env("KHOR_LIFECYCLE_ROOT", &scratch.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(18);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < until,
            "lifecycle worker did not finish: {case}"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    worker
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    worker
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(status.success(), "{case}: {stdout}\n{stderr}");
}

#[test]
fn execution_unwind_closes_idle_input_even_with_an_active_clone() {
    isolated("unwind");
}

#[test]
fn a_stop_before_input_installation_writes_no_start_or_abort() {
    isolated("before-input");
}

struct RecordedPipe {
    bytes: Arc<Mutex<Vec<u8>>>,
    dropped: mpsc::Sender<()>,
}
impl Write for RecordedPipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for RecordedPipe {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}
fn recorded_input() -> (
    input::Input,
    std::thread::JoinHandle<()>,
    Arc<Mutex<Vec<u8>>>,
    mpsc::Receiver<()>,
) {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let (dropped, witness) = mpsc::channel();
    let (input, writer) = input::Input::spawn(Box::new(RecordedPipe {
        bytes: bytes.clone(),
        dropped,
    }))
    .unwrap();
    (input, writer, bytes, witness)
}

fn unwind_idle_input() {
    let active = Arc::new(Running::new("lifecycle-unwind-fixture".into()));
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        &test_name("lifecycle_sidecar"),
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]);
    let (child, pipes) = process::Session::spawn(command, active.process.clone()).unwrap();
    drop(pipes.stdin);
    let (input, writer, bytes, witness) = recorded_input();
    *active.input.lock().unwrap() = Some(input.clone());
    let mut execution = Execution {
        child,
        input,
        writer: Some(writer),
        reader: None,
        drain: None,
    };
    let (ready, started) = mpsc::channel();
    let (reader_done, read) = mpsc::channel();
    execution.reader = Some(std::thread::spawn(move || {
        let mut stdout = BufReader::new(pipes.stdout);
        loop {
            let mut line = String::new();
            if stdout.read_line(&mut line).unwrap() == 0 {
                return;
            }
            if let Some(at) = line.find("LIFECYCLE_READY ") {
                ready
                    .send(
                        line[at + "LIFECYCLE_READY ".len()..]
                            .trim()
                            .parse::<libc::pid_t>()
                            .unwrap(),
                    )
                    .unwrap();
                break;
            }
        }
        let mut tail = Vec::new();
        let _ = stdout.read_to_end(&mut tail);
        let _ = reader_done.send(());
    }));
    let (drain_done, drained) = mpsc::channel();
    execution.drain = Some(std::thread::spawn(move || {
        super::io::drain_stderr(pipes.stderr);
        let _ = drain_done.send(());
    }));
    let pid = started.recv_timeout(Duration::from_secs(3)).unwrap();
    let before = Instant::now();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _owned = execution;
        panic!("fixture failure with idle writer");
    }));
    assert!(panic.is_err());
    assert!(
        before.elapsed() < Duration::from_secs(3),
        "Execution Drop waited for grace or idle input"
    );
    witness.recv_timeout(Duration::from_secs(1)).unwrap();
    read.recv_timeout(Duration::from_secs(1)).unwrap();
    drained.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(bytes.lock().unwrap().is_empty());
    assert!(
        send(&active, &json!({"type":"start"})).is_err(),
        "the active clone must remain closed after unwind"
    );
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Execution Drop returned before reap"
    );
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}

fn stop_before_input(root: PathBuf) {
    let sidecar = root.join("sidecar");
    fs::write(
        &sidecar,
        "#!/bin/sh\nprintf started >\"$0.started\"\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o700)).unwrap();
    unsafe {
        std::env::set_var("KHOR_AGENT_SIDECAR", &sidecar);
    }
    let node = Node::open_as(root.join("store"), "lifecycle-fixture").unwrap();
    let active = Arc::new(Running::new(format!(
        "agent-{}-lifecycle-before-input-fixture",
        &node.device_str()[..16]
    )));
    active.request_stop();
    let saved = Saved {
        fingerprint: "fixture".into(),
        record: CommandRecord {
            id: active.id.clone(),
            owner: node.device_str().into(),
            owner_name: node.shown_name().into(),
            line: "fixture".into(),
            status: "running".into(),
            started_at_ms: now(),
            agent: Some(AgentTurn {
                model: "fixture".into(),
                approval: None,
                updated_at_ms: 0,
                updates: vec![],
                updates_truncated: false,
            }),
            ..Default::default()
        },
    };
    fs::create_dir_all(directory(node.root()).join("runs")).unwrap();
    save(node.root(), &saved).unwrap();
    execute(
        node.root().to_path_buf(),
        saved,
        active.clone(),
        json!({"type":"start"}),
        "fixture-key",
    );
    assert!(
        !root.join("sidecar.started").exists(),
        "a run stopped before input installation spawned the sidecar"
    );
    let receipt: Saved =
        serde_json::from_slice(&fs::read(file(node.root(), &active.id)).unwrap()).unwrap();
    assert_eq!(receipt.record.status, "cancelled");

    // Exercise the real writer's pre-start cancellation independently of the
    // executor's early-return guard, without reproducing its installation branch.
    let (input, writer, bytes, witness) = recorded_input();
    input.abort();
    assert!(input.send(b"{\"type\":\"start\"}\n".to_vec()).is_err());
    witness.recv_timeout(Duration::from_secs(1)).unwrap();
    writer.join().unwrap();
    assert!(
        bytes.lock().unwrap().is_empty(),
        "pre-start cancellation wrote abort or start bytes"
    );
}

#[test]
#[ignore = "isolated real Execution/Input lifecycle worker"]
fn lifecycle_worker() {
    match std::env::var("KHOR_LIFECYCLE_CASE").unwrap().as_str() {
        "unwind" => unwind_idle_input(),
        "before-input" => stop_before_input(PathBuf::from(
            std::env::var_os("KHOR_LIFECYCLE_ROOT").unwrap(),
        )),
        other => panic!("unknown lifecycle fixture {other}"),
    }
}

#[test]
#[ignore = "real process fixture with a self-exit fuse"]
fn lifecycle_sidecar() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(12));
        std::process::exit(124);
    });
    println!("LIFECYCLE_READY {}", std::process::id());
    std::io::stdout().flush().unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(100));
    }
}
