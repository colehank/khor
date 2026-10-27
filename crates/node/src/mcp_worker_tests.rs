use super::*;
use crate::{Node, testdir::TempTree};
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

fn native_cli() -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os("KHOR_TEST_MCP_EXECUTABLE")
            .expect("set the exact freshly built native CLI path"),
    );
    assert!(path.is_absolute() && path.is_file(), "{path:?}");
    path
}
async fn configured(name: &str) -> (TempTree, Node) {
    native_cli();
    let scratch = TempTree::fresh(name);
    let node = Node::open_as(scratch.join("store"), "mcp-worker-fixture").unwrap();
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
    node.khor_agent_here(crate::khor_agent::Action::Configure {
        account: account.id,
        model: "fixture".into(),
    })
    .await
    .unwrap();
    (scratch, node)
}
fn tool(node: &Node, name: &str, args: Value, authorized: bool) -> Worker {
    Worker::spawn(
        Request {
            root: node.root().into(),
            operation: Operation::Tool {
                run: "worker-parent-fixture".into(),
                authorized,
                name: name.into(),
                args,
            },
        },
        Arc::default(),
        Duration::from_secs(15),
        None,
    )
    .unwrap()
}
fn finish(worker: &mut Worker) -> Result<String, String> {
    loop {
        if let Some(answer) = worker.poll() {
            return answer;
        }
        thread::sleep(Duration::from_millis(5));
    }
}
fn absent(pid: u32) -> bool {
    (unsafe { libc::kill(pid as i32, 0) }) == -1
        && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

#[test]
fn total_encoded_budget_returns_an_error_instead_of_partial_success() {
    // A text fits its own budget while JSON escaping exceeds the wire budget.
    // The actual encoder must reject the whole result before emitting bytes.
    let mut bytes = Vec::new();
    write_result(&mut bytes, Ok("\0".repeat(OUTPUT_LIMIT / 6 + 1))).unwrap();
    let answer: Result<String, String> = serde_json::from_slice(&bytes).unwrap();
    assert!(answer.unwrap_err().contains("encoded result exceeded"));
    assert!(bytes.len() < 1024);
    // A valid inner result is insufficient: the final request id/envelope
    // must fit the same total budget before it reaches Pi's stdin.
    let mut frame = json!({"id":"i".repeat(64 * 1024),"result":null});
    frame["result"] = Value::String("\0".repeat(OUTPUT_LIMIT / 6 - 32));
    assert!(encode_message(&frame).is_err());
}

#[test]
#[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn native_worker_rejects_trailing_or_oversize_input_before_opening_any_store() {
    let scratch = TempTree::fresh("worker-malformed");
    let root = scratch.join("must-not-exist");
    let request = serde_json::to_vec(&Request {
        root: root.clone(),
        operation: Operation::CancelStep {
            run: "fixture".into(),
        },
    })
    .unwrap();
    for bytes in [
        [request.clone(), request].concat(),
        vec![b' '; INPUT_LIMIT + 1],
    ] {
        let mut child = Command::new(native_cli())
            .arg("_mcp_worker")
            .env("KHOR_HOME", scratch.join("unrelated"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let _ = child.stdin.take().unwrap().write_all(&bytes);
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let answer: Result<String, String> = serde_json::from_slice(&output.stdout).unwrap();
        assert!(answer.is_err());
        assert!(
            output.stderr.is_empty(),
            "worker must not echo rejected input"
        );
        assert!(!root.exists());
        assert!(!scratch.join("unrelated").exists());
    }
}

#[tokio::test]
#[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn real_worker_authorization_and_original_parent_identity_survive_the_process_boundary() {
    let (scratch, node) = configured("worker-parent").await;
    let mut read = tool(&node, "devices", json!({}), false);
    let pid = read.session.as_ref().unwrap().pid();
    assert!(finish(&mut read).unwrap().contains("mcp-worker-fixture"));
    assert!(absent(pid), "a result is delivered only after worker reap");
    let args =
        json!({"agent":"not-configured-fixture", "prompt":"fixture", "request":"owned-message"});
    let mut rejected = tool(&node, "delegate", args.clone(), false);
    assert!(finish(&mut rejected).is_err());
    assert!(!node.root().join(".khor/agent/children").exists());
    let mut approved = tool(&node, "delegate", args, true);
    // The target is intentionally unconfigured: no model or host is launched.
    assert!(finish(&mut approved).is_err());
    assert!(
        !node.root().join(".khor/agent/children").exists(),
        "a refused binding has no side effect to track"
    );
    assert!(!node.root().join(".khor/agent/messages").exists());
    let agent = "registered-parent-fixture";
    node.register_agent(
        agent,
        &[scratch
            .join("must-not-exist")
            .to_string_lossy()
            .into_owned()],
    )
    .unwrap();
    let args = json!({"agent":agent, "prompt":"fixture", "request":"owned-message"});
    let mut unauthorized = tool(&node, "delegate", args.clone(), false);
    assert!(finish(&mut unauthorized).is_err());
    assert!(!node.root().join(".khor/agent/children").exists());
    let mut authorized = tool(&node, "delegate", args.clone(), true);
    let result = finish(&mut authorized).unwrap();
    let crate::delegation::Answer::Task { record } = serde_json::from_str(&result).unwrap() else {
        panic!("actual registered Send returned no Task");
    };
    assert_eq!(
        record.task.status.state,
        crate::delegation::protocol::TaskState::Failed,
        "missing command must fail the real accepted task before a host starts"
    );
    let parent = crate::delegation::TaskRef {
        home: node.device(),
        id: "worker-parent-fixture".into(),
    };
    assert_eq!(record.parent.as_ref(), Some(&parent));
    let crate::delegation::Answer::Binding { binding } = node
        .delegation_here(
            node.device(),
            crate::delegation::Action::ResolveBinding {
                agent: agent.into(),
                task: Some(record.task.id.clone()),
            },
        )
        .unwrap()
    else {
        panic!("missing actual owned Binding");
    };
    let key = blake3::hash(&serde_json::to_vec(&(&binding, "owned-message")).unwrap())
        .to_hex()
        .to_string();
    let actual: crate::delegation::TaskRef = serde_json::from_slice(
        &fs::read(
            node.root()
                .join(".khor/agent/messages")
                .join(format!("{key}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(actual, parent);
    let later_run = "later-worker-parent-fixture";
    let mut later = Worker::spawn(
        Request {
            root: node.root().clone(),
            operation: Operation::Tool {
                run: later_run.into(),
                authorized: true,
                name: "delegate".into(),
                args,
            },
        },
        Arc::default(),
        Duration::from_secs(15),
        None,
    )
    .unwrap();
    let later_result = finish(&mut later).unwrap();
    let crate::delegation::Answer::Task {
        record: later_record,
    } = serde_json::from_str(&later_result).unwrap()
    else {
        panic!("later real Send returned no Task");
    };
    assert_eq!(later_record.task.id, record.task.id);
    assert_eq!(later_record.parent.as_ref(), Some(&parent));
    for run in ["worker-parent-fixture", later_run] {
        let directory = crate::delegation::children_dir(node.root(), run);
        let files: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], directory.join(format!("{key}.json")));
        let link: crate::delegation::ChildLink =
            serde_json::from_slice(&fs::read(&files[0]).unwrap()).unwrap();
        assert_eq!(link.binding, binding);
        assert_eq!(link.task.as_deref(), Some(record.task.id.as_str()));
    }
    let owned = node
        .delegation_store()
        .unwrap()
        .get(node.device(), &record.target, &record.task.id)
        .unwrap();
    assert_eq!(owned.parent, Some(parent));
}

struct ChildOwner(std::process::Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct AgentProbe(Option<JoinHandle<Result<(), String>>>);
impl Drop for AgentProbe {
    fn drop(&mut self) {
        if let Some(thread) = self.0.take() {
            let _ = thread.join();
        }
    }
}

#[tokio::test]
#[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn real_worker_keeps_ssh_broker_and_network_policy_without_inheriting_credentials() {
    if let Some(root) = std::env::var_os("KHOR_WORKER_ENV_FIXTURE") {
        environment_fixture(Path::new(&root)).await;
        return;
    }
    native_cli();
    let scratch = TempTree::fresh("worker-environment-driver");
    for name in ["home", "ssh", "secrets"] {
        fs::create_dir(scratch.join(name)).unwrap();
    }
    fs::write(scratch.join("extra-ca.pem"), b"").unwrap();
    let output = File::create(scratch.join("environment.log")).unwrap();
    let mut child=ChildOwner(Command::new(std::env::current_exe().unwrap())
        .args(["--exact","mcp_worker::tests::real_worker_keeps_ssh_broker_and_network_policy_without_inheriting_credentials","--ignored","--nocapture","--test-threads=1"])
        .env("KHOR_WORKER_ENV_FIXTURE",scratch.path()).env("HOME",scratch.join("home"))
        .env("SSH_AUTH_SOCK",scratch.join("agent.sock"))
        .env("KHOR_RELAY","http://127.0.0.1:9").env("KHOR_RELAY_ONLY","1")
        .env("KHOR_NO_MDNS","1").env("KHOR_PUBLIC_RELAYS","")
        .env("KHOR_BIND_IP","127.0.0.1").env("KHOR_NETLAB","1").env("KHOR_SPARE_ROAD","off")
        .env("KHOR_RELAY_CA",scratch.join("extra-ca.pem")).env("KHOR_RELAY_SUCCESSOR","")
        .env("KHOR_SSH_DIR",scratch.join("ssh")).env("KHOR_SECRET_DIR",scratch.join("secrets"))
        .env("OPENAI_API_KEY","synthetic-must-not-inherit")
        .env("SSH_ASKPASS","must-not-inherit").env("KHOR_ASKPASS","must-not-inherit")
        .stdout(output.try_clone().unwrap()).stderr(output).spawn().unwrap());
    let by = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < by,
            "environment worker exceeded cleanup fuse"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let log = fs::read_to_string(scratch.join("environment.log")).unwrap();
    println!("{log}");
    assert!(
        status.success(),
        "actual environment fixture failed: {status}"
    );
}
async fn environment_fixture(root: &Path) {
    use std::os::unix::net::UnixListener;
    let listener = UnixListener::bind(root.join("agent.sock")).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut probe = AgentProbe(Some(thread::spawn(move || {
        let by = Instant::now() + Duration::from_secs(8);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= by {
                        return Err("actual ssh-add never reached owned broker".into());
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        let mut request = [0; 5];
        stream.read_exact(&mut request).map_err(|e| e.to_string())?;
        if request != [0, 0, 0, 1, 11] {
            return Err("expected actual OpenSSH identities request".into());
        }
        stream
            .write_all(&[0, 0, 0, 5, 12, 0, 0, 0, 0])
            .map_err(|e| e.to_string())
    })));
    let (_tree, node) = configured("worker-environment-child").await;
    node.set_relays(
        &[khor_net::endpoint::Relay::parse("http://127.0.0.1:9").unwrap()],
        true,
    )
    .unwrap();
    let marker = root.join("environment-ok");
    let mut host = ShellHost {
        release: root.join("release"),
        pid: None,
    };
    let checks = [
        ("SSH_AUTH_SOCK", root.join("agent.sock")),
        ("KHOR_RELAY_CA", root.join("extra-ca.pem")),
        ("KHOR_SSH_DIR", root.join("ssh")),
        ("KHOR_SECRET_DIR", root.join("secrets")),
    ]
    .into_iter()
    .map(|(name, path)| format!("[ \"${{{name}-}}\" = {} ]", quote(&path)))
    .collect::<Vec<_>>()
    .join(" && ");
    let command = format!(
        "/usr/bin/ssh-add -l >/dev/null 2>&1; status=$?; [ $status -eq 1 ] && {checks} && [ \"${{KHOR_RELAY-}}\" = http://127.0.0.1:9 ] && [ \"${{KHOR_RELAY_ONLY-}}\" = 1 ] && [ \"${{KHOR_NO_MDNS-}}\" = 1 ] && [ -z \"${{OPENAI_API_KEY+x}}${{SSH_ASKPASS+x}}${{KHOR_ASKPASS+x}}\" ] && printf ok > {}; i=0; while [ ! -f {} ] && [ $i -lt 400 ]; do sleep .05; i=$((i+1)); done",
        quote(&marker),
        quote(&host.release)
    );
    let mut worker = tool(
        &node,
        "open",
        json!({"machine":node.name(),"command":command,"cwd":root}),
        true,
    );
    let id = crate::SessionId(finish(&mut worker).unwrap());
    let live = crate::live::LiveKind::new(node.root().into(), node.device());
    host.pid = Some(
        crate::host::read_host_file(&live.dir_of(&id).unwrap())
            .unwrap()
            .host_pid,
    );
    probe.0.take().unwrap().join().unwrap().unwrap();
    let by = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < by {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(fs::read(&marker).unwrap(), b"ok");
}

#[tokio::test]
#[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn a_real_a2a_ledger_lock_cannot_keep_a_cancelled_worker_or_its_io_alive() {
    let (_scratch, node) = configured("worker-ledger-lock").await;
    let agent = "a2a:held-fixture";
    let task = "held-task";
    let directory = node
        .root()
        .join(".khor/a2a/http")
        .join(blake3::hash(agent.as_bytes()).to_hex().as_str());
    fs::create_dir_all(&directory).unwrap();
    let ledger = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("ledger.lock"))
        .unwrap();
    ledger.lock().unwrap();
    let mut worker = tool(
        &node,
        "task_status",
        json!({"agent":agent,"task":task}),
        false,
    );
    let pid = worker.session.as_ref().unwrap().pid();
    let key = blake3::hash(&serde_json::to_vec(&(node.device(), task)).unwrap())
        .to_hex()
        .to_string();
    let gate = directory.join(format!("task-{key}.lock"));
    let by = Instant::now() + Duration::from_secs(5);
    loop {
        // Product refresh acquires this gate immediately before transaction's
        // blocking ledger.lock. Its live lock is an observation-only barrier.
        if File::open(&gate).is_ok_and(|gate| gate.try_lock().is_err()) {
            break;
        }
        assert!(
            worker.poll().is_none(),
            "worker returned before reaching the held lock"
        );
        assert!(Instant::now() < by, "actual A2A task gate was not acquired");
        thread::sleep(Duration::from_millis(5));
    }
    let stopped = Instant::now();
    worker.cancel_and_join().unwrap();
    assert!(stopped.elapsed() < Duration::from_secs(1));
    assert!(
        absent(pid),
        "the blocked Rust worker must actually be reaped"
    );
    let mut next = tool(&node, "devices", json!({}), false);
    assert!(
        finish(&mut next).is_ok(),
        "next work must finish while ledger remains held"
    );
    drop(ledger);
}

#[tokio::test]
#[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn cancellation_registered_before_os_spawn_still_reaps_the_actual_worker() {
    let (_scratch, node) = configured("worker-stop-before-spawn").await;
    let control = Arc::new(Control::default());
    control.terminate_now();
    let started = Instant::now();
    let mut worker = Worker::spawn(
        Request {
            root: node.root().into(),
            operation: Operation::Tool {
                run: "stopped-before-spawn".into(),
                authorized: false,
                name: "devices".into(),
                args: json!({}),
            },
        },
        control,
        Duration::from_secs(5),
        None,
    )
    .unwrap();
    let pid = worker.session.as_ref().unwrap().pid();
    assert!(finish(&mut worker).is_err());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(absent(pid));
}

struct DelegatedHosts(PathBuf);
impl Drop for DelegatedHosts {
    fn drop(&mut self) {
        let Ok(node) = Node::open(self.0.clone()) else {
            return;
        };
        let Ok(store) = node.delegation_store() else {
            return;
        };
        let target = crate::delegation::AgentTarget {
            home: node.device(),
            agent: "fixture".into(),
        };
        let live = crate::live::LiveKind::new(self.0.clone(), node.device());
        for record in store
            .list(node.device(), &target, None, None, 100)
            .unwrap_or_default()
        {
            let Some(dir) = record.context_session.and_then(|s| live.dir_of(&s.id)) else {
                continue;
            };
            let Ok(host) = crate::host::read_host_file(&dir) else {
                continue;
            };
            if let Ok(mut socket) = std::net::TcpStream::connect_timeout(
                &std::net::SocketAddr::from(([127, 0, 0, 1], host.port)),
                Duration::from_secs(1),
            ) {
                let _ = socket.set_read_timeout(Some(Duration::from_secs(1)));
                let _ = socket.set_write_timeout(Some(Duration::from_secs(1)));
                let _ = crate::host::write_frame(
                    &mut socket,
                    &crate::host::Hello {
                        cookie: host.cookie,
                        cols: 0,
                        rows: 0,
                    },
                );
                if crate::host::read_frame::<crate::host::Welcome>(&mut socket).is_ok() {
                    let _ = crate::host::write_frame(&mut socket, &crate::gui_host::GuiOp::Close);
                }
            }
            let by = Instant::now() + Duration::from_secs(3);
            while !absent(host.host_pid) && Instant::now() < by {
                thread::sleep(Duration::from_millis(10));
            }
            if !absent(host.host_pid) {
                unsafe {
                    libc::kill(host.host_pid as i32, libc::SIGTERM);
                }
            }
        }
    }
}

/// Executes the actual incoming Answer through the real detached ACP host,
/// then withholds only its IPC receipt. No MCP dispatcher or tool is replaced.
struct AnswerReceiptGate {
    port: u16,
    applied: mpsc::Receiver<Result<(), String>>,
    release: mpsc::SyncSender<()>,
    thread: Option<JoinHandle<Result<(), String>>>,
}
impl AnswerReceiptGate {
    fn new(root: PathBuf, cookie: String) -> Self {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let (entered, applied) = mpsc::sync_channel(1);
        let (release, released) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || {
            let result = (|| -> Result<std::net::TcpStream, String> {
                let node = Node::open(root)?;
                let by = Instant::now() + Duration::from_secs(8);
                loop {
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, peer)) => {
                                if !peer.ip().is_loopback() {
                                    return Err("non-loopback Answer peer".into());
                                }
                                break stream;
                            }
                            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                                if Instant::now() >= by {
                                    return Err("Answer IPC did not arrive".into());
                                }
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(error) => return Err(error.to_string()),
                        }
                    };
                    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .map_err(|e| e.to_string())?;
                    let mut bytes = Vec::new();
                    (&mut stream)
                        .take((crate::proto::MAX_FRAME + 1) as u64)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    if bytes.len() > crate::proto::MAX_FRAME {
                        return Err("Answer IPC exceeded frame budget".into());
                    }
                    let handoff: crate::ipc::Handoff = crate::proto::decode(&bytes)?;
                    if handoff.cookie != cookie {
                        return Err("wrong owned Answer cookie".into());
                    }
                    let crate::ipc::Op::Delegation { machine, action } = handoff.op else {
                        return Err("expected actual Delegation".into());
                    };
                    if matches!(&action, crate::delegation::Action::ResolveBinding { agent, task: Some(_) } if agent == "fixture")
                    {
                        if machine != node.device_str() {
                            return Err("expected stable resolution home".into());
                        }
                        let answer = node.delegation_here(node.device(), action)?;
                        let bytes =
                            crate::proto::encode(&crate::ipc::Reply::Delegation { answer })?;
                        stream.write_all(&bytes).map_err(|e| e.to_string())?;
                        stream
                            .shutdown(std::net::Shutdown::Write)
                            .map_err(|e| e.to_string())?;
                        continue;
                    }
                    if machine != node.device_str()
                        || !matches!(&action,crate::delegation::Action::Bound { binding, action: crate::delegation::BoundAction::Answer { option, .. } } if binding.home == node.device() && binding.caller == node.device() && binding.agent=="fixture" && option.as_deref()==Some("go"))
                    {
                        return Err("expected actual authenticated bound fixture Answer".into());
                    }
                    let crate::delegation::Answer::Task { record } =
                        node.delegation_here(node.device(), action)?
                    else {
                        return Err("Answer returned no task".into());
                    };
                    if crate::delegation::approval(&record).is_some() {
                        return Err("actual ACP permission was not resolved".into());
                    }
                    break Ok(stream);
                }
            })();
            match result {
                Ok(stream) => {
                    let _ = entered.send(Ok(()));
                    let _ = released.recv_timeout(Duration::from_secs(10));
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    Ok(())
                }
                Err(error) => {
                    let _ = entered.send(Err(error.clone()));
                    Err(error)
                }
            }
        });
        Self {
            port,
            applied,
            release,
            thread: Some(thread),
        }
    }
}
impl Drop for AnswerReceiptGate {
    fn drop(&mut self) {
        let _ = self.release.try_send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[tokio::test]
#[ignore = "requires freshly built native CLI and KHOR_TEST_ACP_STUB"]
async fn answering_an_existing_task_keeps_a_durable_link_when_cancelled_after_actual_resume() {
    use std::os::unix::fs::PermissionsExt;
    let (_scratch, node) = configured("worker-answer-before-receipt").await;
    let stub = PathBuf::from(
        std::env::var_os("KHOR_TEST_ACP_STUB").expect("set freshly built acp-stub path"),
    );
    node.register_agent(
        "fixture",
        &[
            stub.to_string_lossy().into_owned(),
            "--unique-session".into(),
        ],
    )
    .unwrap();
    let _hosts = DelegatedHosts(node.root().into());
    let mut delegate = tool(
        &node,
        "delegate",
        json!({"agent":"fixture","request":"answer-window-message","prompt":"fixture"}),
        true,
    );
    let crate::delegation::Answer::Task { record } =
        serde_json::from_str(&finish(&mut delegate).unwrap()).unwrap()
    else {
        panic!("no delegated task");
    };
    let store = node.delegation_store().unwrap();
    let target = crate::delegation::AgentTarget {
        home: node.device(),
        agent: "fixture".into(),
    };
    let by = Instant::now() + Duration::from_secs(5);
    let ask = loop {
        let current = store.get(node.device(), &target, &record.task.id).unwrap();
        if let Some(ask) = crate::delegation::approval(&current).and_then(|v| v["ask"].as_u64()) {
            break ask;
        }
        assert!(Instant::now() < by, "actual ACP permission did not arrive");
        thread::sleep(Duration::from_millis(10));
    };
    let cookie = crate::link::fresh_hex().unwrap();
    let gate = AnswerReceiptGate::new(node.root().into(), cookie.clone());
    fs::write(node.root().join(".khor/endpoint.json"),serde_json::to_vec(&json!({
        "id":node.device_str(),"addrs":[],"pid":std::process::id(),"ipc_port":gate.port,"ipc_cookie":cookie
    })).unwrap()).unwrap();
    fs::set_permissions(
        node.root().join(".khor/endpoint.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    // A new parent run must not inherit the earlier delegate's existing link.
    let run = "answer-parent-fixture";
    let directory = node
        .root()
        .join(".khor/agent/children")
        .join(blake3::hash(run.as_bytes()).to_hex().as_str());
    assert!(!directory.exists());
    let mut answer=Worker::spawn(Request {root:node.root().into(),operation:Operation::Tool {
        run:run.into(),authorized:true,name:"task_answer".into(),args:json!({"agent":"fixture","task":record.task.id,"ask":ask.to_string(),"option":"go"})
    }},Arc::default(),Duration::from_secs(15),None).unwrap();
    let pid = answer.session.as_ref().unwrap().pid();
    gate.applied
        .recv_timeout(Duration::from_secs(8))
        .expect("actual Answer never reached held receipt")
        .unwrap();
    let links: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect();
    assert_eq!(links.len(), 1);
    let crate::delegation::Answer::Binding { binding } = node
        .delegation_here(
            node.device(),
            crate::delegation::Action::ResolveBinding {
                agent: "fixture".into(),
                task: Some(record.task.id.clone()),
            },
        )
        .unwrap()
    else {
        panic!("missing actual Answer binding");
    };
    let expected_link = directory.join(
        crate::delegation::child_key(&binding, &format!("task:{}", record.task.id)).unwrap(),
    );
    assert_eq!(links[0], expected_link);
    let before = fs::read(&expected_link).unwrap();
    let link: Value = serde_json::from_slice(&before).unwrap();
    assert_eq!(link["task"], record.task.id);
    assert_eq!(link["message"], format!("task:{}", record.task.id));
    answer.cancel_and_join().unwrap();
    assert!(absent(pid));
    assert_eq!(
        fs::read(&links[0]).unwrap(),
        before,
        "unknown/resumed work must remain traceable after worker reap"
    );
    drop(gate);
}

#[tokio::test]
#[ignore = "requires freshly built native CLI and KHOR_TEST_ACP_STUB"]
async fn a_real_delegated_acp_host_survives_the_worker_and_keeps_its_parent() {
    let (_scratch, node) = configured("worker-durable-ghost").await;
    let stub = PathBuf::from(
        std::env::var_os("KHOR_TEST_ACP_STUB").expect("set freshly built acp-stub path"),
    );
    assert!(stub.is_absolute() && stub.is_file());
    node.register_agent(
        "fixture",
        &[
            stub.to_string_lossy().into_owned(),
            "--unique-session".into(),
        ],
    )
    .unwrap();
    let hosts = DelegatedHosts(node.root().into());
    let mut worker = tool(
        &node,
        "delegate",
        json!({"agent":"fixture","request":"durable-fixture-message","prompt":"fixture"}),
        true,
    );
    let pid = worker.session.as_ref().unwrap().pid();
    let answer: crate::delegation::Answer =
        serde_json::from_str(&finish(&mut worker).unwrap()).unwrap();
    let crate::delegation::Answer::Task { record } = answer else {
        panic!("worker returned no task");
    };
    assert_eq!(
        record.parent,
        Some(crate::delegation::TaskRef {
            home: node.device(),
            id: "worker-parent-fixture".into()
        })
    );
    let context = record
        .context_session
        .as_ref()
        .expect("actual detached ACP host");
    let live = crate::live::LiveKind::new(node.root().into(), node.device());
    let host = crate::host::read_host_file(&live.dir_of(&context.id).unwrap()).unwrap();
    assert!(absent(pid));
    assert!(
        !absent(host.host_pid),
        "durable _ghost must outlive its MCP worker"
    );
    drop(hosts);
    let by = Instant::now() + Duration::from_secs(3);
    while !absent(host.host_pid) && Instant::now() < by {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(absent(host.host_pid), "owned delegated host did not retire");
}

struct ShellHost {
    release: PathBuf,
    pid: Option<u32>,
}
impl Drop for ShellHost {
    fn drop(&mut self) {
        let _ = fs::write(&self.release, b"release");
        if let Some(pid) = self.pid {
            let by = Instant::now() + Duration::from_secs(3);
            while !absent(pid) && Instant::now() < by {
                thread::sleep(Duration::from_millis(10));
            }
            if !absent(pid) {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
            }
        }
    }
}
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[tokio::test]
#[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn a_real_durable_shell_host_survives_its_successful_worker() {
    let (scratch, node) = configured("worker-durable-host").await;
    let mut host = ShellHost {
        release: scratch.join("release"),
        pid: None,
    };
    let command = format!(
        "i=0; while [ ! -f {} ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done",
        quote(&host.release)
    );
    let mut worker = tool(
        &node,
        "open",
        json!({"machine":node.name(),"command":command,"cwd":scratch.path()}),
        true,
    );
    let worker_pid = worker.session.as_ref().unwrap().pid();
    let id = crate::SessionId(finish(&mut worker).unwrap());
    let live = crate::live::LiveKind::new(node.root().into(), node.device());
    let dir = live.dir_of(&id).expect("real worker-created shell session");
    let file = crate::host::read_host_file(&dir).unwrap();
    host.pid = Some(file.host_pid);
    assert!(absent(worker_pid));
    assert!(
        !absent(file.host_pid),
        "durable _host must survive worker group cleanup"
    );
    assert!(!absent(file.child_pid), "durable shell must remain active");
    drop(host);
    let by = Instant::now() + Duration::from_secs(3);
    while !absent(file.host_pid) && Instant::now() < by {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        absent(file.host_pid),
        "owned host did not exit after its release gate"
    );
}
