//! R0: real six-run Stop admission, with four actual cancellation IPC replies held.
//! The controlled peer is not an ACP executor or proof of remote terminal state.
use super::*;
use crate::delegation::{AgentTarget, Answer, Execution, Record, Store, TaskRef, protocol};
use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

type Event = Result<(String, bool), String>; // exact task, Cancel rather than Get
type Records = Arc<Mutex<HashMap<String, (usize, Record)>>>;

fn absent(pid: i32) -> bool {
    (unsafe { libc::kill(pid, 0) }) == -1
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}
fn wait_for(
    mut condition: impl FnMut() -> Result<bool, String>,
    seconds: u64,
) -> Result<(), String> {
    let by = Instant::now() + Duration::from_secs(seconds);
    loop {
        if condition()? {
            return Ok(());
        }
        if Instant::now() >= by {
            return Err("bounded observation expired".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}
fn pids(path: &Path) -> Result<Vec<i32>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    fs::read_to_string(path)
        .map_err(|e| e.to_string())?
        .lines()
        .map(|line| {
            line.parse::<i32>()
                .ok()
                .filter(|pid| *pid > 1)
                .ok_or_else(|| "setup: invalid owned PID log".into())
        })
        .collect()
}

// Observe only this isolated libtest's direct, exact fresh-CLI worker children.
// This is process retirement evidence, never a substitute for Cancel IPC arrival.
fn worker_pids() -> Result<Vec<i32>, String> {
    worker_pids_of(std::process::id())
}
fn worker_pids_of(parent: u32) -> Result<Vec<i32>, String> {
    let executable = std::env::var("KHOR_TEST_MCP_EXECUTABLE").map_err(|e| e.to_string())?;
    let output = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid=,command="])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("setup: owned process observation failed".into());
    }
    let expected = format!("{executable} _mcp_worker");
    let mut found = Vec::new();
    for line in String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .lines()
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() >= 3
            && fields[1].parse::<u32>().ok() == Some(parent)
            && fields[2..].join(" ") == expected
        {
            found.push(fields[0].parse().map_err(|_| "setup: invalid ps PID")?);
        }
    }
    Ok(found)
}

// Last-resort outer cleanup also covers a fixture panic/wall fuse. Never kill
// an arbitrary logged PID: require the exact owned command and private PGID.
fn retire_owned(pid: i32, command: &str) -> Result<(), String> {
    if absent(pid) {
        return Ok(());
    }
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .map_err(|e| e.to_string())?;
    let actual = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    if actual.trim() != command || unsafe { libc::getpgid(pid) } != pid {
        return Err(format!("cleanup: refused changed PID/group identity {pid}"));
    }
    println!("outer fallback: terminate exact owned process group {pid}");
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    wait_for(|| Ok(absent(pid)), 3)
}

struct Peer {
    port: u16,
    records: Records,
    released: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
    events: mpsc::Receiver<Event>,
    thread: Option<JoinHandle<Result<(), String>>>,
}
impl Peer {
    fn new(cookie: String) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let records: Records = Arc::default();
        let released = Arc::new(AtomicBool::new(false));
        let stopping = Arc::new(AtomicBool::new(false));
        let (send, events) = mpsc::channel();
        let (rows, release, stop) = (records.clone(), released.clone(), stopping.clone());
        let thread = thread::spawn(move || {
            let by = Instant::now() + Duration::from_secs(40);
            let mut handlers = Vec::new();
            while !stop.load(Ordering::Acquire) && Instant::now() < by {
                match listener.accept() {
                    Ok((stream, address)) => {
                        if !address.ip().is_loopback() {
                            return Err("setup: non-loopback peer".into());
                        }
                        let (rows, release, stop, cookie, send) = (
                            rows.clone(),
                            release.clone(),
                            stop.clone(),
                            cookie.clone(),
                            send.clone(),
                        );
                        handlers.push(thread::spawn(move || {
                            if let Err(error) = reply(stream, &cookie, rows, release, stop, &send) {
                                let _ = send.send(Err(error));
                            }
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            release.store(true, Ordering::Release);
            for handler in handlers {
                handler.join().map_err(|_| "setup: IPC handler panicked")?;
            }
            Ok(())
        });
        Ok(Self {
            port,
            records,
            released,
            stopping,
            events,
            thread: Some(thread),
        })
    }
    fn observe(&self, cancels: &mut HashSet<String>) -> Result<(), String> {
        while let Ok(event) = self.events.try_recv() {
            let (task, cancel) = event?;
            println!(
                "actual cookie IPC: {} task={task}",
                if cancel { "Cancel" } else { "Get" }
            );
            if cancel {
                cancels.insert(task);
            }
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<(), String> {
        self.released.store(true, Ordering::Release);
        self.stopping.store(true, Ordering::Release);
        self.thread.take().map_or(Ok(()), |t| {
            t.join()
                .map_err(|_| "setup: IPC listener panicked".to_owned())?
        })
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn reply(
    mut stream: TcpStream,
    cookie: &str,
    rows: Records,
    released: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
    events: &mpsc::Sender<Event>,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    let by = Instant::now() + Duration::from_secs(3);
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        if stopping.load(Ordering::Acquire) || Instant::now() >= by {
            return Err("setup: IPC request failed to half-close".into());
        }
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if bytes.len() + count > crate::proto::MAX_FRAME {
                    return Err("setup: oversized fixture IPC".into());
                }
                bytes.extend_from_slice(&buffer[..count]);
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let handoff: crate::ipc::Handoff = crate::proto::decode(&bytes)?;
    if handoff.cookie != cookie {
        return Err("setup: incorrect private IPC cookie".into());
    }
    let (machine, agent, task, cancel) = match handoff.op {
        crate::ipc::Op::Delegation {
            machine,
            action: crate::delegation::Action::Get { agent, task },
        } => (machine, agent, task, false),
        crate::ipc::Op::Delegation {
            machine,
            action: crate::delegation::Action::Cancel { agent, task },
        } => (machine, agent, task, true),
        other => return Err(format!("setup: unexpected handoff {other:?}")),
    };
    if !machine.is_empty() || agent != "fixture" {
        return Err("setup: changed target identity".into());
    }
    let (index, mut record) = rows
        .lock()
        .unwrap()
        .get(&task)
        .cloned()
        .ok_or("setup: unknown exact task")?;
    events
        .send(Ok((task.clone(), cancel)))
        .map_err(|e| e.to_string())?;
    if cancel {
        let by = Instant::now() + Duration::from_secs(15);
        while index < 4 && !released.load(Ordering::Acquire) && !stopping.load(Ordering::Acquire) {
            if Instant::now() >= by {
                return Err("setup: held Cancel fixture fuse expired".into());
            }
            thread::sleep(Duration::from_millis(5));
        }
        // Controlled peer delivery response only, not an actual ACP execution receipt.
        record.task.status.state = protocol::TaskState::Canceled;
    }
    let bytes = crate::proto::encode(&crate::ipc::Reply::Delegation {
        answer: Answer::Task {
            record: Box::new(record),
        },
    })?;
    stream.write_all(&bytes).map_err(|e| e.to_string())?;
    stream.shutdown(Shutdown::Write).map_err(|e| e.to_string())
}

fn local_retired(root: &Path, run: &str) -> Result<bool, String> {
    let saved: Saved =
        serde_json::from_slice(&fs::read(file(root, run)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    Ok(saved.record.status == "cancelled"
        && !running().lock().unwrap().contains_key(root)
        && claim(root).is_ok())
}

fn exercise(node: &Node, scratch: &Path, peer: &Peer) -> Result<(), String> {
    let store = Store::open(scratch.join("controlled-peer-store"), node.device())
        .map_err(|e| e.to_string())?;
    let mut tasks = Vec::new();
    let mut cancels = HashSet::new();
    let mut native_workers = HashSet::new();
    let mut held_since = None;
    for index in 0..6 {
        let ready = scratch.join("ready");
        if ready.exists() {
            fs::remove_file(&ready).map_err(|e| e.to_string())?;
        }
        let run = node
            .khor_agent_here(Action::Prompt {
                token: format!("remote-cancel-r0-{index}"),
                text: "owned fixture".into(),
            })?
            .run
            .ok_or("setup: Prompt returned no run")?;
        wait_for(|| Ok(ready.is_file()), 3)
            .map_err(|e| format!("setup: Pi readiness {index}: {e}"))?;
        let pid: i32 = fs::read_to_string(&ready)
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let parent = TaskRef {
            home: node.device(),
            id: run.clone(),
        };
        let message = protocol::Message::new(
            protocol::Role::User,
            vec![protocol::Part::text("R0 controlled peer")],
        );
        let accepted = store
            .accept(
                node.device(),
                &AgentTarget {
                    home: node.device(),
                    agent: "fixture".into(),
                },
                &message,
                Some(parent.clone()),
            )
            .map_err(|e| e.to_string())?;
        let task = accepted.record.task.id;
        let mut execution = Execution::claim(store.clone(), &task, &message.message_id)
            .map_err(|e| e.to_string())?;
        execution
            .update(protocol::TaskState::Working, None, None)
            .map_err(|e| e.to_string())?;
        peer.records
            .lock()
            .unwrap()
            .insert(task.clone(), (index, execution.record().clone()));
        drop(execution);
        crate::delegation::track_child(
            node.root(),
            &parent,
            "",
            "fixture",
            &message.message_id,
            Some(&task),
        )?;
        println!(
            "run {index}: actual Prompt ready pid={pid}; production durable track_child task={task}"
        );
        node.khor_agent_here(Action::Stop { run: run.clone() })?;
        wait_for(|| Ok(local_retired(node.root(), &run)? && absent(pid)), 3)
            .map_err(|e| format!("setup: run {index} local retirement: {e}"))?;
        if !pids(&scratch.join("pi-aborts"))?.contains(&pid) {
            return Err(format!("setup: Pi {pid} did not receive actual abort"));
        }
        let saved: Saved =
            serde_json::from_slice(&fs::read(file(node.root(), &run)).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let data: Value = serde_json::from_str(
            saved
                .record
                .data_json
                .as_deref()
                .ok_or("setup: no cancellation metadata")?,
        )
        .map_err(|e| e.to_string())?;
        println!(
            "run {index}: Pi ESRCH; cancelled; map=false claim=free; sweep={}",
            data["remoteCancellation"]["sweep"]
        );
        if index < 4 {
            wait_for(
                || {
                    peer.observe(&mut cancels)?;
                    Ok(cancels.contains(&task))
                },
                2,
            )
            .map_err(|e| format!("setup: run {index} never entered held Cancel: {e}"))?;
            if held_since.is_none() {
                held_since = Some(Instant::now());
            }
            native_workers.extend(worker_pids()?);
        }
        tasks.push(task);
    }
    if held_since.unwrap().elapsed() >= Duration::from_secs(4)
        || worker_pids()?.len() != 4
        || native_workers.len() != 4
    {
        return Err(
            "setup: four real worker hold window was lost before six local retirements".into(),
        );
    }
    println!(
        "barrier: all six local runs retired; four exact CLI workers live; first four actual Cancel replies still held"
    );
    peer.released.store(true, Ordering::Release);
    wait_for(|| Ok(native_workers.iter().all(|pid| absent(*pid))), 3)
        .map_err(|e| format!("setup: released original worker processes did not reap: {e}"))?;
    println!("barrier: original four real CLI worker PIDs ESRCH: {native_workers:?}");
    // Run the existing startup reconciliation and ordinary Read lifecycle only.
    // No new Stop/Prompt, direct scheduler call, or global Vec/counter oracle.
    // Eight seconds also exceeds all original five-second finite owner budgets.
    let by = Instant::now() + Duration::from_secs(8);
    loop {
        node.reconcile_agent()?;
        node.khor_agent_here(Action::Read)?;
        peer.observe(&mut cancels)?;
        if cancels.contains(&tasks[4]) && cancels.contains(&tasks[5]) {
            break;
        }
        if Instant::now() >= by {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    peer.observe(&mut cancels)?;
    let missed: Vec<_> = tasks[4..]
        .iter()
        .filter(|task| !cancels.contains(*task))
        .collect();
    println!(
        "normal lifecycle delivery: actual Cancel count={}, missed later tasks={missed:?}",
        cancels.len()
    );
    if !missed.is_empty() {
        return Err(format!(
            "BUSINESS RED R0: after four owners retired and normal Read/startup reconciliation, later cancellations never reached actual IPC: {missed:?}"
        ));
    }
    Ok(())
}

fn cleanup(node: &Node, scratch: &Path, peer: &mut Peer) -> Result<(), String> {
    peer.released.store(true, Ordering::Release);
    if let Some(active) = running().lock().unwrap().get(node.root()).cloned() {
        active.request_stop();
    }
    let drain = wait_for(
        || Ok(!running().lock().unwrap().contains_key(node.root()) && claim(node.root()).is_ok()),
        12,
    );
    let joined = peer.finish();
    let reaped = wait_for(
        || Ok(pids(&scratch.join("pi-pids"))?.into_iter().all(absent) && worker_pids()?.is_empty()),
        7,
    );
    println!(
        "cleanup: IPC handlers/listener joined={joined:?}; map/claim drain={drain:?}; all owned Pi/CLI ESRCH={reaped:?}"
    );
    drain?;
    joined?;
    reaped
}

#[test]
#[ignore = "private process for real six-run remote cancellation admission regression"]
fn remote_cancel_worker() {
    let scratch = PathBuf::from(std::env::var_os("KHOR_REMOTE_CANCEL_ROOT").unwrap());
    let node = Node::open_as(scratch.join("store"), "remote-cancel-r0").unwrap();
    let account = node
        .register_account(
            "fixture",
            crate::Provider::Newapi,
            crate::AccountKind::Bill,
            Some("http://127.0.0.1:1"),
            "synthetic-unused-fixture-key",
            None,
        )
        .unwrap();
    node.khor_agent_here(Action::Configure {
        account: account.id,
        model: "fixture".into(),
    })
    .unwrap();
    let cookie = crate::link::fresh_hex().unwrap();
    let mut peer = Peer::new(cookie.clone()).unwrap();
    let endpoint = node.root().join(".khor/endpoint.json");
    fs::write(
        &endpoint,
        serde_json::to_vec(&json!({"id":node.device_str(), "addrs":[],
        "pid":std::process::id(), "ipc_port":peer.port, "ipc_cookie":cookie}))
        .unwrap(),
    )
    .unwrap();
    fs::set_permissions(endpoint, fs::Permissions::from_mode(0o600)).unwrap();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        exercise(&node, &scratch, &peer)
    }))
    .unwrap_or_else(|_| Err("setup: exercise panicked; explicit cleanup still required".into()));
    let retired = cleanup(&node, &scratch, &mut peer);
    assert!(retired.is_ok(), "cleanup={retired:?}; primary={outcome:?}");
    assert!(outcome.is_ok(), "{outcome:?}");
}

#[test]
fn six_stopped_runs_eventually_deliver_remote_cancel_after_four_held_owners_retire() {
    let executable = std::env::var_os("KHOR_TEST_MCP_EXECUTABLE")
        .expect("requires exact CLI built from this candidate source, not an installed release");
    assert!(Path::new(&executable).is_absolute() && Path::new(&executable).is_file());
    let scratch = std::env::temp_dir().join(format!("r0-{}", crate::link::fresh_hex().unwrap()));
    fs::create_dir(&scratch).unwrap();
    let python = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("python3"))
        .find(|path| path.is_file())
        .expect("python3 fixture runtime");
    let python = fs::canonicalize(python).unwrap();
    let fixture = scratch.join("remote-cancel.py");
    fs::write(&fixture, include_str!("fixtures/remote-cancel.py")).unwrap();
    let quote = |p: &Path| format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"));
    let sidecar = scratch.join("sidecar");
    fs::write(
        &sidecar,
        format!(
            "#!/bin/sh\nexec {} -I -S {}\n",
            quote(&python),
            quote(&fixture)
        ),
    )
    .unwrap();
    fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o700)).unwrap();
    let output = File::create(scratch.join("worker.log")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "khor_agent::remote_cancel_tests::remote_cancel_worker",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("KHOR_TEST_MCP_EXECUTABLE", executable)
        .env("KHOR_REMOTE_CANCEL_ROOT", &scratch)
        .env("KHOR_AGENT_SIDECAR", sidecar)
        .env("KHOR_PATH", "/usr/bin:/bin")
        .process_group(0)
        .stdout(output.try_clone().unwrap())
        .stderr(output);
    let mut child = child.spawn().unwrap();
    let by = Instant::now() + Duration::from_secs(55);
    let mut next_observation = Instant::now();
    let mut observed_workers = HashSet::new();
    let status = loop {
        if Instant::now() >= next_observation {
            observed_workers.extend(worker_pids_of(child.id()).unwrap_or_default());
            next_observation = Instant::now() + Duration::from_millis(250);
        }
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= by {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let log = fs::read_to_string(scratch.join("worker.log")).unwrap_or_default();
    println!("{log}");
    let pi_command = format!("{} -I -S {}", python.display(), fixture.display());
    let pi_pids = pids(&scratch.join("pi-pids")).unwrap();
    let mut cleanup_errors = Vec::new();
    for pid in &pi_pids {
        if let Err(error) = retire_owned(*pid, &pi_command) {
            cleanup_errors.push(error);
        }
    }
    let native_command = format!(
        "{} _mcp_worker",
        Path::new(&std::env::var_os("KHOR_TEST_MCP_EXECUTABLE").unwrap()).display()
    );
    for pid in &observed_workers {
        if let Err(error) = retire_owned(*pid, &native_command) {
            cleanup_errors.push(error);
        }
    }
    let pi_gone = pi_pids.into_iter().all(absent);
    let native_gone = observed_workers.into_iter().all(absent);
    println!(
        "outer fixture child wait={status:?}; Pi ESRCH={pi_gone}; native ESRCH={native_gone}; cleanup={cleanup_errors:?}"
    );
    fs::remove_dir_all(&scratch).unwrap();
    assert!(pi_gone, "owned Pi remained after fixture child wait");
    assert!(
        native_gone && cleanup_errors.is_empty(),
        "owned native cleanup: {cleanup_errors:?}"
    );
    assert!(
        status.is_some_and(|s| s.success()),
        "real remote cancellation fixture failed: {status:?}"
    );
}
