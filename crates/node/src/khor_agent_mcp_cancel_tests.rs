//! A held real IPC reply distinguishes Pi reap from Rust executor retirement.
use super::*;
use std::{
    io::Read,
    net::{Shutdown, TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    sync::atomic::AtomicBool,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.join("runtime.closed").is_file() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn stopping_during_a_real_mcp_request_retires_the_executor_after_pi_exits() {
    exercise_fixture(None);
}

#[test]
#[ignore = "isolated metadata startup acceptance: requires verified actual CLI"]
fn zero_metadata_startup_adapter_preserves_actual_stop_admission() {
    exercise_fixture(Some(0));
}

#[test]
#[ignore = "isolated metadata startup acceptance: requires verified actual CLI"]
fn slow_metadata_startup_cannot_block_actual_stop_admission() {
    exercise_fixture(Some(2000));
}

fn exercise_fixture(startup_delay_ms: Option<u64>) {
    let executable = std::env::var_os("KHOR_TEST_MCP_EXECUTABLE")
        .expect("this regression requires the exact newly built Khor CLI worker executable");
    assert!(Path::new(&executable).is_absolute() && Path::new(&executable).is_file());
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-mcp-cancel-{}",
        crate::link::fresh_hex().unwrap()
    )));
    fs::create_dir(&scratch.0).unwrap();
    let python = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|path| path.join("python3"))
        .find(|path| path.is_file())
        .expect("python3 is required by this owned local fixture");
    let python = fs::canonicalize(python).unwrap();
    let fixture = scratch.0.join("mcp-cancel.py");
    fs::write(&fixture, include_str!("fixtures/mcp-cancel.py")).unwrap();
    let quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"));
    let sidecar = scratch.0.join("sidecar");
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
    fs::write(scratch.0.join("sidecar.mode"), "blocked-mcp").unwrap();
    let worker_executable = if let Some(delay_ms) = startup_delay_ms {
        let adapter = scratch.0.join("metadata-start.py");
        fs::write(&adapter, include_str!("fixtures/mcp-metadata-start.py")).unwrap();
        let entry = scratch.0.join("metadata-start");
        fs::write(
            &entry,
            format!(
                "#!/bin/sh\nexec {} -I -S {} {} {} {} {} \"$@\"\n",
                quote(&python),
                quote(&adapter),
                quote(Path::new(&executable)),
                quote(&scratch.0.join("store")),
                quote(&scratch.0.join("metadata.events")),
                delay_ms,
            ),
        )
        .unwrap();
        fs::set_permissions(&entry, fs::Permissions::from_mode(0o700)).unwrap();
        entry.into_os_string()
    } else {
        executable
    };
    let output = File::create(scratch.0.join("worker.log")).unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "khor_agent::mcp_cancel_tests::mcp_cancel_worker",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("KHOR_AGENT_SIDECAR", sidecar)
        .env("KHOR_TEST_MCP_EXECUTABLE", worker_executable)
        .env("KHOR_MCP_CANCEL_ROOT", &scratch.0)
        .env("KHOR_PATH", "/usr/bin:/bin")
        .stdout(output.try_clone().unwrap())
        .stderr(output);
    if let Some(delay_ms) = startup_delay_ms {
        command.env("KHOR_MCP_METADATA_DELAY_MS", delay_ms.to_string());
    } else {
        command.env_remove("KHOR_MCP_METADATA_DELAY_MS");
    }
    let mut worker = Worker(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(40);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "owned MCP cancellation worker exceeded 40s: {}",
            fs::read_to_string(scratch.0.join("worker.log")).unwrap_or_default()
        );
        thread::sleep(Duration::from_millis(10));
    };
    let log = fs::read_to_string(scratch.0.join("worker.log")).unwrap();
    println!("{log}");
    assert!(
        status.success(),
        "owned MCP cancellation worker failed: {status}"
    );
}

/// One listener and one real Handoff. The release channel is owned by the test,
/// so unwinding closes the held reply and joins the server before returning.
struct HeldIpc {
    port: u16,
    stop: Arc<AtomicBool>,
    release: Option<mpsc::SyncSender<()>>,
    entered: mpsc::Receiver<Result<(), String>>,
    thread: Option<JoinHandle<Result<(), String>>>,
}
impl HeldIpc {
    fn new(cookie: String) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let (release, released) = mpsc::sync_channel(1);
        let (enter, entered) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || {
            let result = (|| -> Result<TcpStream, String> {
                let by = Instant::now() + Duration::from_secs(5);
                let mut stream = loop {
                    if stopping.load(Ordering::Acquire) {
                        return Err("fixture released before request".into());
                    }
                    match listener.accept() {
                        Ok((stream, peer)) => {
                            if !peer.ip().is_loopback() {
                                return Err("non-loopback fixture peer".into());
                            }
                            break stream;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= by {
                                return Err("no actual IPC request arrived".into());
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => return Err(error.to_string()),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_millis(100)))
                    .map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                let mut chunk = [0; 8192];
                loop {
                    if stopping.load(Ordering::Acquire) {
                        return Err("fixture released while reading request".into());
                    }
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(count) => {
                            if bytes.len() + count > crate::proto::MAX_FRAME {
                                return Err("oversized IPC fixture request".into());
                            }
                            bytes.extend_from_slice(&chunk[..count]);
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock
                                    | std::io::ErrorKind::TimedOut
                                    | std::io::ErrorKind::Interrupted
                            ) =>
                        {
                            if Instant::now() >= by {
                                return Err("IPC request did not half-close".into());
                            }
                        }
                        Err(error) => return Err(error.to_string()),
                    }
                }
                let handoff: crate::ipc::Handoff = crate::proto::decode(&bytes)?;
                if handoff.cookie != cookie {
                    return Err("wrong owned IPC cookie".into());
                }
                match handoff.op {
                    crate::ipc::Op::Delegation {
                        machine,
                        action: crate::delegation::Action::Get { agent, task },
                    } if machine.is_empty() && agent == "fixture" && task == "owned-mcp-task" => {}
                    other => {
                        return Err(format!(
                            "expected actual Delegation/Get, received {other:?}"
                        ));
                    }
                }
                Ok(stream)
            })();
            match result {
                Ok(stream) => {
                    let _ = enter.send(Ok(()));
                    // This is a private gate, not the production 300s wait.
                    // The parent always releases it, even after the 13s failure.
                    let _ = released.recv_timeout(Duration::from_secs(25));
                    let _ = stream.shutdown(Shutdown::Both);
                    Ok(())
                }
                Err(error) => {
                    let _ = enter.send(Err(error.clone()));
                    Err(error)
                }
            }
        });
        Self {
            port,
            stop,
            release: Some(release),
            entered,
            thread: Some(thread),
        }
    }
    fn release_and_join(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        if let Some(release) = self.release.take() {
            let _ = release.try_send(());
        }
        match self.thread.take() {
            Some(thread) => thread
                .join()
                .map_err(|_| "owned IPC fixture panicked".to_owned())?,
            None => Ok(()),
        }
    }
}
impl Drop for HeldIpc {
    fn drop(&mut self) {
        let _ = self.release_and_join();
    }
}

struct Cleanup {
    root: PathBuf,
    run: Option<String>,
    ipc: HeldIpc,
}
impl Cleanup {
    fn finish(&mut self) -> Result<(), String> {
        if let Some(active) = running()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&self.root)
            .filter(|active| Some(&active.id) == self.run.as_ref())
            .cloned()
        {
            active.request_stop();
        }
        let ipc = self.ipc.release_and_join();
        let by = Instant::now() + Duration::from_secs(5);
        while self.run.is_some() {
            let mapped = running()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .contains_key(&self.root);
            if !mapped && claim(&self.root).is_ok() {
                self.run = None;
                break;
            }
            if Instant::now() >= by {
                return Err("execute did not release map/claim after IPC gate release".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        ipc
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn absent(pid: i32) -> bool {
    (unsafe { libc::kill(pid, 0) }) == -1
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

async fn close_fixture_runtime(node: &Node) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            match Node::close_agent_runtime(node.root()).await? {
                RuntimeClose::Closed => return Ok::<(), String>(()),
                RuntimeClose::Busy => {
                    node.reconcile_agent()?;
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        }
    })
    .await
    .map_err(|_| "fixture runtime close exceeded its deadline".to_owned())?
}

fn first_fixture_run(node: &Node) -> String {
    format!(
        "agent-{}-mcp-cancel-first-fixture",
        &node.device_str()[..16]
    )
}

async fn observe_startup_persistence(node: &Node, diagnostics: &[Value]) -> Result<Value, String> {
    let Some(delay) = std::env::var_os("KHOR_MCP_METADATA_DELAY_MS") else {
        return Ok(Value::Null);
    };
    let first = diagnostics
        .iter()
        .find(|event| event["index"] == 0 && event["phase"] == "end")
        .ok_or("first actual Stop result was not recorded")?;
    if delay.to_str() == Some("2000")
        && first["call_elapsed_ms"]
            .as_u64()
            .is_some_and(|ms| ms < 1000)
        && (first["durability_state"] != "PersistenceUnknown"
            || first["durability_code"] != "MetadataOutcomeUnknown")
    {
        return Err("fast Stop did not report its still-pending metadata honestly".into());
    }
    let run = first_fixture_run(node);
    let started = Instant::now();
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let report = node
                .khor_agent_here(Action::CancellationStatus { run: run.clone() })
                .await?;
            let data = report.data.ok_or("actual Status returned no projection")?;
            if report.run.as_deref() != Some(run.as_str()) || data["run"] != run {
                return Err("actual Status returned a different run".into());
            }
            if data["durability"]["state"] == "DurablyQueued" {
                let as_of = data["as_of_ms"]
                    .as_i64()
                    .ok_or("actual Status returned no observation time")?;
                return Ok(json!({"durability_state":"DurablyQueued",
                    "as_of_ms":as_of,"observed_after_ms":started.elapsed().as_millis()}));
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| "actual Status did not observe persistence before its deadline".to_owned())?
}

fn startup_evidence(
    node: &Node,
    scratch: &Path,
    diagnostics: &[Value],
) -> Result<Vec<Value>, String> {
    let pid: i32 = fs::read_to_string(scratch.join("sidecar.pid"))
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|_| "invalid actual Pi PID")?;
    if !absent(pid) {
        return Err("actual Pi remains after runtime close".into());
    }
    let Some(delay_ms) = std::env::var_os("KHOR_MCP_METADATA_DELAY_MS") else {
        return Ok(Vec::new());
    };
    let delay_ms: u64 = delay_ms
        .to_str()
        .and_then(|value| value.parse().ok())
        .filter(|value| matches!(*value, 0 | 2000))
        .ok_or("invalid fixture startup delay")?;
    let path = scratch.join("metadata.events");
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 2048 {
        return Err("startup events exceeded their byte budget".into());
    }
    let events = fs::read_to_string(path)
        .map_err(|e| e.to_string())?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "invalid bounded startup event")?;
    if events.is_empty() || events.len() > 8 || events.len() % 2 != 0 {
        return Err("actual metadata Persist startup was not observed".into());
    }
    for pair in events.chunks_exact(2) {
        for event in pair {
            let object = event.as_object().ok_or("invalid startup event object")?;
            if object.len() != 4
                || object.keys().any(|key| {
                    !matches!(key.as_str(), "phase" | "pid" | "monotonic_ns" | "delay_ms")
                })
                || event["delay_ms"].as_u64() != Some(delay_ms)
            {
                return Err("startup event fields differ from the fixture".into());
            }
        }
        let pid = pair[0]["pid"]
            .as_i64()
            .and_then(|pid| i32::try_from(pid).ok())
            .filter(|pid| *pid > 0)
            .ok_or("invalid actual metadata PID")?;
        let start = pair[0]["monotonic_ns"].as_u64().ok_or("no gate time")?;
        let exec = pair[1]["monotonic_ns"].as_u64().ok_or("no exec time")?;
        if pair[0]["phase"] != "gate_enter"
            || pair[1]["phase"] != "pre_exec"
            || pair[0]["pid"] != pair[1]["pid"]
            || exec
                .checked_sub(start)
                .is_none_or(|ns| ns < delay_ms * 1_000_000)
            || !absent(pid)
        {
            return Err("actual startup delay or metadata reap was not proved".into());
        }
    }
    // The adapter never returns a reply. Require the actual native producer's
    // independently persisted intent after close, including on the timing red.
    let run = first_fixture_run(node);
    let path = node
        .root()
        .join(".khor/agent/cancellations")
        .join(blake3::hash(run.as_bytes()).to_hex().to_string())
        .join("intent.json");
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 4096 {
        return Err("actual intent exceeds its byte budget".into());
    }
    let intent: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|_| "invalid actual intent")?;
    if intent["version"] != 1
        || intent["run"] != run
        || intent["execution_home"] != serde_json::to_value(node.device()).unwrap()
    {
        return Err("actual native metadata intent identity differs".into());
    }
    let first = diagnostics
        .iter()
        .find(|event| event["index"] == 0 && event["phase"] == "end")
        .ok_or("first actual Stop result was not recorded")?;
    if first["durability_state"] == "DurablyQueued"
        && first["call_elapsed_ms"]
            .as_u64()
            .is_none_or(|ms| ms < delay_ms)
    {
        return Err("Stop claimed durable persistence before actual metadata startup".into());
    }
    Ok(events)
}

async fn exercise(
    node: &Node,
    scratch: &Path,
    cleanup: &mut Cleanup,
    diagnostics: &mut Vec<Value>,
) -> Result<(), String> {
    let run = node
        .khor_agent_here(Action::Prompt {
            token: "mcp-cancel-first-fixture".into(),
            text: "fixture only".into(),
        })
        .await?
        .run
        .ok_or("no first run")?;
    cleanup.run = Some(run.clone());
    cleanup
        .ipc
        .entered
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "setup: real MCP did not reach IPC barrier".to_owned())??;
    println!("barrier: decoded actual MCP task_status → IPC Delegation/Get; reply held");
    let pid: i32 = fs::read_to_string(scratch.join("sidecar.pid"))
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    if pid <= 0 {
        return Err("setup: invalid owned Pi PID".into());
    }
    let stopped = Instant::now();
    let sweeps_before = crate::mcp_worker::SWEEPS_STARTED.load(Ordering::SeqCst);
    // Bounded diagnostic only: no Status/Retry RPC or result-payload logging.
    let mut log_stop = |index: usize,
                        call_started: Instant,
                        sweep_at_start: usize,
                        result: Option<&Result<Report, String>>| {
        let data = result
            .and_then(|value| value.as_ref().ok())
            .and_then(|report| report.data.as_ref());
        let safe = |key: &str| {
            data.and_then(|value| value["durability"][key].as_str())
                .filter(|value| {
                    value.len() <= 64
                        && value
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                })
                .map(str::to_owned)
        };
        let sweeps = crate::mcp_worker::SWEEPS_STARTED.load(Ordering::SeqCst);
        if diagnostics.len() < 32 {
            diagnostics.push(json!({
                "index":index,"phase":if result.is_some() {"end"} else {"begin"},
                "elapsed_ms":stopped.elapsed().as_millis(),
                "call_elapsed_ms":call_started.elapsed().as_millis(),
                "result":result.map(|value| if value.is_ok() {"Ok"} else {"Err"}),
                "durability_state":safe("state"),"durability_code":safe("code"),
                "as_of_ms":data.and_then(|value| value["as_of_ms"].as_i64()),
                "sweeps_call_delta":sweeps.saturating_sub(sweep_at_start),
                "sweeps_total_delta":sweeps.saturating_sub(sweeps_before),
            }));
        }
    };
    let call_started = Instant::now();
    let sweep_at_start = crate::mcp_worker::SWEEPS_STARTED.load(Ordering::SeqCst);
    log_stop(0, call_started, sweep_at_start, None);
    let stop_result = node
        .khor_agent_here(Action::Stop { run: run.clone() })
        .await;
    log_stop(0, call_started, sweep_at_start, Some(&stop_result));
    stop_result?;
    for index in 1..16 {
        // Repeated Stop must share the same local owner; a finished run may
        // already have retired by the last call.
        let call_started = Instant::now();
        let sweep_at_start = crate::mcp_worker::SWEEPS_STARTED.load(Ordering::SeqCst);
        log_stop(index, call_started, sweep_at_start, None);
        let stop_result = node
            .khor_agent_here(Action::Stop { run: run.clone() })
            .await;
        log_stop(index, call_started, sweep_at_start, Some(&stop_result));
    }
    // Checked after map/claim retirement below, when the single sweep has been
    // admitted. No snapshot of a source-level flag substitutes for this count.
    if stopped.elapsed() >= Duration::from_secs(1) {
        return Err("Stop admission blocked".into());
    }
    let reaped_by = stopped + Duration::from_secs(11);
    while !absent(pid) {
        if Instant::now() >= reaped_by {
            return Err(format!(
                "Pi fixture {pid} was not reaped by its original grace"
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !scratch.join("sidecar.abort").is_file() {
        return Err("Pi did not observe actual abort".into());
    }
    println!(
        "Pi fixture pid={pid}: ESRCH after cooperative abort; elapsed={:?}",
        stopped.elapsed()
    );
    println!(
        "after Pi reap: map_present={}, claim_busy={}, held IPC reply not released",
        running()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains_key(&cleanup.root),
        claim(&cleanup.root).is_err()
    );
    let by = stopped + Duration::from_secs(13);
    loop {
        let mapped = running()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains_key(&cleanup.root);
        let claimed = claim(&cleanup.root).is_err();
        let saved: Saved = serde_json::from_slice(
            &fs::read(file(&cleanup.root, &run)).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if !mapped && !claimed && saved.record.status == "cancelled" {
            break;
        }
        if Instant::now() >= by {
            return Err(format!(
                "Pi already reaped, but Rust executor exceeded 13s while IPC reply stayed held: status={}, map_present={mapped}, claim_busy={claimed}, elapsed={:?}",
                saved.record.status,
                stopped.elapsed()
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    // Keep the first IPC gate held: a next prompt must not depend on the test
    // supplying a late reply to the cancelled operation.
    let sweeps_after = crate::mcp_worker::SWEEPS_STARTED.load(Ordering::SeqCst);
    if sweeps_after != sweeps_before + 1 {
        return Err(format!(
            "16 Stop calls admitted {} sweep owners, expected one",
            sweeps_after - sweeps_before
        ));
    }
    fs::write(scratch.join("sidecar.mode"), "finish").map_err(|e| e.to_string())?;
    let second = node
        .khor_agent_here(Action::Prompt {
            token: "mcp-cancel-next-fixture".into(),
            text: "next fixture".into(),
        })
        .await?
        .run
        .ok_or("no second run")?;
    cleanup.run = Some(second.clone());
    let by = Instant::now() + Duration::from_secs(3);
    loop {
        let saved: Saved = serde_json::from_slice(
            &fs::read(file(&cleanup.root, &second)).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let mapped = running()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains_key(&cleanup.root);
        if saved.record.status == "done" && !mapped && claim(&cleanup.root).is_ok() {
            break;
        }
        if Instant::now() >= by {
            return Err(format!(
                "next prompt did not finish: {}",
                saved.record.status
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    let second_pid: i32 = fs::read_to_string(scratch.join("sidecar.pid"))
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    if !absent(second_pid) {
        return Err("next prompt published completion before Pi reap".into());
    }
    println!("next prompt finished while original IPC gate held; pid={second_pid}: ESRCH");
    cleanup.ipc.release_and_join()?;
    let after_late_reply: Saved =
        serde_json::from_slice(&fs::read(file(&cleanup.root, &second)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if after_late_reply.record.status != "done" {
        return Err("old held IPC retirement changed the next prompt receipt".into());
    }
    cleanup.run = None;
    Ok(())
}

#[tokio::test]
#[ignore = "isolated actual MCP/IPC cancellation regression worker"]
async fn mcp_cancel_worker() {
    let scratch = PathBuf::from(std::env::var_os("KHOR_MCP_CANCEL_ROOT").unwrap());
    let node = Node::open_as(scratch.join("store"), "mcp-cancel-fixture").unwrap();
    let root = node.root().to_path_buf();
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
    .await
    .unwrap();
    let cookie = crate::link::fresh_hex().unwrap();
    let ipc = HeldIpc::new(cookie.clone());
    let endpoint = root.join(".khor/endpoint.json");
    fs::write(
        &endpoint,
        serde_json::to_vec(&json!({"id":node.device_str(),"addrs":[],
        "pid":std::process::id(),"ipc_port":ipc.port,"ipc_cookie":cookie}))
        .unwrap(),
    )
    .unwrap();
    fs::set_permissions(endpoint, fs::Permissions::from_mode(0o600)).unwrap();
    let mut cleanup = Cleanup {
        root,
        run: None,
        ipc,
    };
    let mut diagnostics = Vec::with_capacity(32);
    let outcome = exercise(&node, &scratch, &mut cleanup, &mut diagnostics).await;
    // Preserve the product assertion, but retire the held socket and observe
    // actual executor release before failing. Never leave its 300s call behind.
    let cleanup_result = cleanup.finish();
    let status_result = if cleanup_result.is_ok() {
        observe_startup_persistence(&node, &diagnostics).await
    } else {
        Err("actual Status observation requires completed IPC/map/claim cleanup".into())
    };
    let runtime_result = close_fixture_runtime(&node).await;
    let startup_result =
        if cleanup_result.is_ok() && status_result.is_ok() && runtime_result.is_ok() {
            startup_evidence(&node, &scratch, &diagnostics)
        } else {
            Err("startup evidence requires completed actual cleanup".into())
        };
    if let Ok(events) = &startup_result {
        for event in events {
            println!("metadata_startup_event {event}");
        }
        fs::write(scratch.join("runtime.closed"), "closed").unwrap();
    }
    println!("explicit IPC release+fixture join and execute/map/claim drain: {cleanup_result:?}");
    if let Ok(snapshot) = &status_result {
        println!("fresh_cancellation_status {snapshot}");
    }
    println!(
        "actual runtime close: {runtime_result:?}; startup evidence: {:?}",
        startup_result.as_ref().map(Vec::len)
    );
    for record in diagnostics {
        println!("stop_diagnostic {record}");
    }
    assert!(
        cleanup_result.is_ok(),
        "{cleanup_result:?}; primary={outcome:?}"
    );
    assert!(
        runtime_result.is_ok(),
        "{runtime_result:?}; primary={outcome:?}"
    );
    assert!(
        status_result.is_ok(),
        "{status_result:?}; primary={outcome:?}"
    );
    assert!(
        startup_result.is_ok(),
        "{startup_result:?}; primary={outcome:?}"
    );
    assert!(outcome.is_ok(), "{outcome:?}");
}
