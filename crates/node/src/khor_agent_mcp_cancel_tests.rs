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

#[test]
fn stopping_during_a_real_mcp_request_retires_the_executor_after_pi_exits() {
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
    let output = File::create(scratch.0.join("worker.log")).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "khor_agent::mcp_cancel_tests::mcp_cancel_worker",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("KHOR_AGENT_SIDECAR", sidecar)
            .env("KHOR_TEST_MCP_EXECUTABLE", executable)
            .env("KHOR_MCP_CANCEL_ROOT", &scratch.0)
            .env("KHOR_PATH", "/usr/bin:/bin")
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap(),
    );
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

async fn exercise(node: &Node, scratch: &Path, cleanup: &mut Cleanup) -> Result<(), String> {
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
    node.khor_agent_here(Action::Stop { run: run.clone() })
        .await?;
    for _ in 1..16 {
        // Repeated Stop must share the same local owner; a finished run may
        // already have retired by the last call.
        let _ = node
            .khor_agent_here(Action::Stop { run: run.clone() })
            .await;
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
    let outcome = exercise(&node, &scratch, &mut cleanup).await;
    // Preserve the product assertion, but retire the held socket and observe
    // actual executor release before failing. Never leave its 300s call behind.
    let cleanup_result = cleanup.finish();
    println!("explicit IPC release+fixture join and execute/map/claim drain: {cleanup_result:?}");
    assert!(
        cleanup_result.is_ok(),
        "{cleanup_result:?}; primary={outcome:?}"
    );
    assert!(outcome.is_ok(), "{outcome:?}");
}
