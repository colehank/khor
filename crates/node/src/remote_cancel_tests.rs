//! R1 receipt tests use the real private files, finite CLI worker and owned
//! native Store/Execution. The separate R0 fixture supplies six real Pi Stops.
use super::*;
use crate::{
    delegation::{
        self, Action as DelegationAction, Answer as DelegationAnswer, Binding, BoundAction,
        Execution as TaskExecution, TaskRef, protocol,
    },
    mcp_worker::{Operation, Request, Worker},
    remote_cancel::{self, Metadata},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn fixture(tag: &str) -> (crate::testdir::TempTree, Node, String) {
    let tree = crate::testdir::TempTree::fresh(tag);
    let node = Node::open_as(tree.join("store"), "cancel-receipt-fixture").unwrap();
    node.register_agent("fixture", &["cancel-fixture-must-not-run".into()])
        .unwrap();
    let run = historical_run(&node);
    (tree, node, run)
}
fn historical_run(node: &Node) -> String {
    let run = format!("agent-fixture-{}", crate::link::fresh_hex().unwrap());
    fs::create_dir_all(file(node.root(), &run).parent().unwrap()).unwrap();
    save(
        node.root(),
        &Saved {
            fingerprint: "receipt-only-fixture".into(),
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
                    updates: Vec::new(),
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        },
    )
    .unwrap();
    run
}
async fn drain(node: &Node) -> Result<(), String> {
    let by = Instant::now() + Duration::from_secs(6);
    loop {
        let result = remote_cancel::release_test_root(node.root());
        if result.is_ok() || Instant::now() >= by {
            return result;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
fn absent(pid: u32) -> bool {
    (unsafe { libc::kill(pid as i32, 0) }) == -1
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

fn binding(node: &Node) -> Binding {
    let DelegationAnswer::Binding { binding } = node
        .delegation_here(
            node.device(),
            DelegationAction::ResolveBinding {
                agent: "fixture".into(),
                task: None,
            },
        )
        .unwrap()
    else {
        panic!("no actual native binding");
    };
    binding
}

fn child(node: &Node, run: &str) -> (TaskExecution, String) {
    let store = node.delegation_store().unwrap();
    let message = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("owned native receipt fixture")],
    );
    let parent = TaskRef {
        home: node.device(),
        id: run.into(),
    };
    let accepted = store
        .accept(
            node.device(),
            &delegation::AgentTarget {
                home: node.device(),
                agent: "fixture".into(),
            },
            &message,
            Some(parent.clone()),
        )
        .unwrap();
    let task = accepted.record.task.id;
    let mut execution = TaskExecution::claim(store, &task, &message.message_id).unwrap();
    execution
        .update(protocol::TaskState::Working, None, None)
        .unwrap();
    delegation::track_child(
        node.root(),
        &parent,
        "fixture diagnostic",
        &binding(node),
        &message.message_id,
        Some(&task),
    )
    .unwrap();
    (execution, task)
}
fn worker(node: &Node, operation: Operation) -> Result<String, String> {
    let mut worker = Worker::spawn(
        Request {
            root: node.root().clone(),
            operation,
        },
        Arc::default(),
        Duration::from_secs(5),
        None,
    )?;
    let pid = worker.pid().ok_or("worker PID missing")?;
    let result = loop {
        if let Some(result) = worker.poll() {
            break result;
        }
        thread::sleep(Duration::from_millis(5));
    };
    drop(worker);
    let absent = unsafe { libc::kill(pid as i32, 0) } == -1
        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH);
    if !absent {
        return Err("cleanup: finite native worker did not reap".into());
    }
    println!("actual CLI worker {pid}: reaped/ESRCH");
    result
}
fn persist(node: &Node, run: &str) -> Value {
    let raw = worker(
        node,
        Operation::CancellationMetadata {
            operation: Metadata::Persist {
                run: run.into(),
                reason: "Stop".into(),
            },
        },
    )
    .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        value["snapshot"]["durability"]["state"], "DurablyQueued",
        "{value}"
    );
    value["snapshot"].clone()
}
fn status(node: &Node, run: &str) -> Value {
    let raw = worker(
        node,
        Operation::CancellationMetadata {
            operation: Metadata::Status { run: run.into() },
        },
    )
    .unwrap();
    serde_json::from_str::<Value>(&raw).unwrap()["snapshot"].clone()
}
fn receipt_paths(node: &Node, run: &str) -> Vec<PathBuf> {
    let directory = node
        .root()
        .join(".khor/agent/cancellations")
        .join(blake3::hash(run.as_bytes()).to_hex().as_str())
        .join("children");
    let mut paths: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "json"))
        .collect();
    paths.sort();
    paths
}
fn update_receipt(node: &Node, run: &str, edit: impl Fn(&mut Value)) {
    for path in receipt_paths(node, run) {
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        crate::link::write_private(&path, &serde_json::to_vec(&value).unwrap()).unwrap();
    }
}
#[derive(Clone, Copy)]
enum Mode {
    Reply,
    DropCancelReply,
    HoldFirstCancel,
    HoldContinuedSend,
}
struct Peer {
    stop: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
    events: Arc<Mutex<Vec<(String, bool)>>>,
    thread: Option<JoinHandle<Result<(), String>>>,
    endpoint: PathBuf,
}
impl Peer {
    fn new(node: &Node, mode: Mode) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let cookie = crate::link::fresh_hex().unwrap();
        let endpoint = node.root().join(".khor/endpoint.json");
        crate::link::write_private(&endpoint, &serde_json::to_vec(&json!({"id":node.device_str(),"addrs":[],"pid":std::process::id(),"ipc_port":listener.local_addr().unwrap().port(),"ipc_cookie":cookie})).unwrap()).unwrap();
        fs::set_permissions(&endpoint, fs::Permissions::from_mode(0o600)).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let events = Arc::new(Mutex::new(Vec::new()));
        let (root, stopping, released, observed) = (
            node.root().clone(),
            stop.clone(),
            release.clone(),
            events.clone(),
        );
        let owner = thread::spawn(move || {
            let by = Instant::now() + Duration::from_secs(25);
            let mut handlers = Vec::new();
            let mut failure = None;
            while !stopping.load(Ordering::Acquire) && Instant::now() < by {
                match listener.accept() {
                    Ok((stream, address)) => {
                        if !address.ip().is_loopback() {
                            failure = Some("nonloopback fixture peer".to_owned());
                            break;
                        }
                        let (root, cookie, stop, release, events) = (
                            root.clone(),
                            cookie.clone(),
                            stopping.clone(),
                            released.clone(),
                            observed.clone(),
                        );
                        handlers.push(thread::spawn(move || {
                            respond(stream, &root, &cookie, mode, &stop, &release, &events)
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => {
                        failure = Some(e.to_string());
                        break;
                    }
                }
            }
            released.store(true, Ordering::Release);
            for handler in handlers {
                match handler.join() {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        failure.get_or_insert(e);
                    }
                    Err(_) => {
                        failure.get_or_insert("IPC handler panicked".into());
                    }
                }
            }
            failure.map_or(Ok(()), Err)
        });
        Self {
            stop,
            release,
            events,
            thread: Some(owner),
            endpoint,
        }
    }
    fn finish(&mut self) -> Result<(), String> {
        self.release.store(true, Ordering::Release);
        self.stop.store(true, Ordering::Release);
        let result = self.thread.take().map_or(Ok(()), |h| {
            h.join().map_err(|_| "IPC listener panicked".to_owned())?
        });
        let _ = fs::remove_file(&self.endpoint);
        result
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
fn respond(
    mut stream: TcpStream,
    root: &Path,
    cookie: &str,
    mode: Mode,
    stop: &AtomicBool,
    release: &AtomicBool,
    events: &Mutex<Vec<(String, bool)>>,
) -> Result<(), String> {
    // Accepted sockets can inherit the listener's nonblocking mode on macOS.
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    (&mut stream)
        .take((crate::proto::MAX_FRAME + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > crate::proto::MAX_FRAME {
        return Err("oversized IPC".into());
    }
    let handoff: crate::ipc::Handoff = crate::proto::decode(&bytes)?;
    if handoff.cookie != cookie {
        return Err("incorrect fixture cookie".into());
    }
    let crate::ipc::Op::Delegation { machine, action } = handoff.op else {
        return Err("not Delegation".into());
    };
    let node = Node::open(root.to_path_buf())?;
    let expected_agent = if matches!(mode, Mode::HoldContinuedSend) {
        "a2a:fixture"
    } else {
        "fixture"
    };
    if machine != node.device_str() {
        return Err("incorrect stable IPC home".into());
    }
    if matches!(mode, Mode::HoldContinuedSend)
        && matches!(&action, DelegationAction::ResolveBinding { agent, task: Some(_) } if agent == expected_agent)
    {
        let answer = node.delegation_here(node.device(), action)?;
        let bytes = crate::proto::encode(&crate::ipc::Reply::Delegation { answer })?;
        stream.write_all(&bytes).map_err(|e| e.to_string())?;
        let _ = stream.shutdown(Shutdown::Write);
        return Ok(());
    }
    let (task, cancel, continued_send) = match &action {
        DelegationAction::Bound {
            binding,
            action: BoundAction::Get { task },
        } if binding.home == node.device()
            && binding.caller == node.device()
            && binding.agent == expected_agent =>
        {
            (task.clone(), false, false)
        }
        DelegationAction::Bound {
            binding,
            action: BoundAction::Cancel { task },
        } if binding.home == node.device()
            && binding.caller == node.device()
            && binding.agent == expected_agent =>
        {
            (task.clone(), true, false)
        }
        DelegationAction::Bound {
            binding,
            action: BoundAction::Send { message, .. },
        } if matches!(mode, Mode::HoldContinuedSend)
            && binding.home == node.device()
            && binding.caller == node.device()
            && binding.agent == expected_agent =>
        {
            (
                message
                    .task_id
                    .clone()
                    .ok_or("continued input must name its actual task")?,
                false,
                true,
            )
        }
        _ => return Err("incorrect bound fixture operation".into()),
    };
    let answer = node.delegation_here(node.device(), action);
    if continued_send
        && !matches!(&answer, Ok(DelegationAnswer::Task { record }) if record.task.id == task && record.task.status.state == protocol::TaskState::Working)
    {
        return Err("continued Send did not actually resume the task".into());
    }
    let first = {
        let mut events = events.lock().unwrap();
        let first = events.is_empty();
        events.push((task, cancel));
        first
    };
    if cancel && matches!(mode, Mode::DropCancelReply) {
        return Ok(());
    }
    if first && cancel && matches!(mode, Mode::HoldFirstCancel) || continued_send {
        let by = Instant::now() + Duration::from_secs(8);
        while !release.load(Ordering::Acquire)
            && !stop.load(Ordering::Acquire)
            && Instant::now() < by
        {
            thread::sleep(Duration::from_millis(5));
        }
    }
    let reply = match answer {
        Ok(answer) => crate::ipc::Reply::Delegation { answer },
        Err(why) => crate::ipc::Reply::Refused { why },
    };
    let bytes = crate::proto::encode(&reply)?;
    // The finite owner may have timed out deliberately while this reply was held.
    if stream.write_all(&bytes).is_err()
        && matches!(mode, Mode::HoldFirstCancel | Mode::HoldContinuedSend)
    {
        return Ok(());
    }
    let _ = stream.shutdown(Shutdown::Write);
    Ok(())
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn working_cancel_reply_is_only_ack_and_fresh_get_confirms_actual_terminal() {
    let (_tree, node, run) = fixture("cancel-working-ack");
    let (mut execution, task) = child(&node, &run);
    let mut peer = Peer::new(&node, Mode::Reply);
    persist(&node, &run);
    let first = worker(&node, Operation::CancelStep { run: run.clone() });
    let ack = status(&node, &run);
    execution
        .update(protocol::TaskState::Canceled, None, None)
        .unwrap();
    update_receipt(&node, &run, |value| value["next_retry_at_ms"] = json!(0));
    let second = worker(&node, Operation::CancelStep { run: run.clone() });
    let confirmed = status(&node, &run);
    let events = peer.events.lock().unwrap().clone();
    let cleanup = peer.finish();
    assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
    assert!(first.is_ok() && second.is_ok());
    assert_eq!(ack["counts"]["AckPending"], 1);
    assert_eq!(ack["all_known_children_canceled"], false);
    assert_eq!(confirmed["counts"]["ConfirmedCanceled"], 1);
    assert_eq!(confirmed["all_known_children_canceled"], true);
    assert_eq!(events, vec![(task.clone(), true), (task, false)]);
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn lost_cancel_reply_and_restarted_inflight_use_observe_before_any_retry() {
    for restart in [false, true] {
        let (_tree, node, run) = fixture("cancel-unknown-restart");
        let (mut execution, task) = child(&node, &run);
        let mut peer = Peer::new(
            &node,
            if restart {
                Mode::Reply
            } else {
                Mode::DropCancelReply
            },
        );
        persist(&node, &run);
        let mut unknown_snapshot = None;
        if restart {
            update_receipt(&node, &run, |value| {
                value["phase"] = json!("InFlight");
                value["attempt_id"] = json!("lost-old-owner");
                value["attempt_no"] = json!(1);
            });
        } else {
            worker(&node, Operation::CancelStep { run: run.clone() }).unwrap();
            unknown_snapshot = Some(status(&node, &run));
        }
        execution
            .update(protocol::TaskState::Canceled, None, None)
            .unwrap();
        update_receipt(&node, &run, |value| value["next_retry_at_ms"] = json!(0));
        // Each operation is a distinct actual CLI: no old resident memory can
        // supply the missing state or turn a cancellation flag into an ACK.
        let observed = worker(
            &Node::open(node.root().clone()).unwrap(),
            Operation::CancelStep { run: run.clone() },
        );
        let snapshot = status(&node, &run);
        let events = peer.events.lock().unwrap().clone();
        let cleanup = peer.finish();
        assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
        assert!(observed.is_ok());
        if let Some(unknown) = unknown_snapshot {
            assert_eq!(unknown["counts"]["OutcomeUnknown"], 1);
            assert_eq!(unknown["all_known_children_canceled"], false);
        }
        assert_eq!(snapshot["counts"]["ConfirmedCanceled"], 1);
        assert_eq!(events.last(), Some(&(task.clone(), false)));
        assert_eq!(
            events.iter().filter(|(_, cancel)| *cancel).count(),
            usize::from(!restart)
        );
    }
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn metadata_lock_failure_still_requests_local_stop_and_history_can_retry() {
    let (_tree, node, run) = fixture("cancel-history-retry");
    let active = Arc::new(Running::new(run.clone()));
    running()
        .lock()
        .unwrap()
        .insert(node.root().clone(), active.clone());
    let directory = node.root().join(".khor/agent/cancellations");
    fs::create_dir_all(&directory).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .open(directory.join("metadata.lock"))
        .unwrap();
    lock.lock().unwrap();
    // The local stop request must already have happened when persistence fails.
    let report = node
        .khor_agent_here(Action::Stop { run: run.clone() })
        .await
        .unwrap();
    let requested = active.stopped.load(Ordering::SeqCst);
    running().lock().unwrap().remove(node.root());
    // Keep the real OS fault in place until the admitted producer reports it;
    // the fast Stop snapshot alone does not prove that it reached metadata.lock.
    let blocked_retry = node
        .khor_agent_here(Action::RetryCancellation { run: run.clone() })
        .await
        .unwrap();
    let intent_absent = !directory
        .join(blake3::hash(run.as_bytes()).to_hex().as_str())
        .join("intent.json")
        .try_exists()
        .unwrap();
    drop(lock);
    let retry = node
        .khor_agent_here(Action::RetryCancellation { run: run.clone() })
        .await
        .unwrap();
    let query = node
        .khor_agent_here(Action::CancellationStatus { run })
        .await
        .unwrap();
    let by = Instant::now() + Duration::from_secs(6);
    let cleanup = loop {
        let result = remote_cancel::release_test_root(node.root());
        if result.is_ok() || Instant::now() >= by {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
    assert!(requested);
    let initial = report.data.unwrap();
    assert!(
        (initial["durability"]["state"] == "NotPersisted"
            && initial["durability"]["code"] == "Busy")
            || (initial["durability"]["state"] == "PersistenceUnknown"
                && initial["durability"]["code"] == "MetadataOutcomeUnknown"),
        "initial Stop snapshot: {initial}"
    );
    let blocked = blocked_retry.data.unwrap();
    assert_eq!(blocked["durability"]["state"], "NotPersisted");
    assert_eq!(blocked["durability"]["code"], "Busy");
    assert!(intent_absent, "the locked producer must not publish an intent");
    assert_eq!(retry.data.unwrap()["durability"]["state"], "DurablyQueued");
    assert_eq!(query.data.unwrap()["durability"]["state"], "DurablyQueued");
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn one_slow_child_does_not_starve_its_sibling_and_pause_sends_nothing() {
    let (_tree, node, run) = fixture("cancel-child-fairness");
    let (_a, _) = child(&node, &run);
    let (_b, _) = child(&node, &run);
    let mut peer = Peer::new(&node, Mode::HoldFirstCancel);
    persist(&node, &run);
    let first = worker(&node, Operation::CancelStep { run: run.clone() });
    let second = worker(&node, Operation::CancelStep { run: run.clone() });
    let events = peer.events.lock().unwrap().clone();
    crate::pause::set_on(node.root(), true).unwrap();
    let paused = worker(&node, Operation::CancelStep { run: run.clone() });
    let unchanged = peer.events.lock().unwrap().clone();
    let cleanup = peer.finish();
    assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
    assert!(
        first.is_err(),
        "first held worker must exhaust its original budget"
    );
    assert!(second.is_ok() && paused.is_ok());
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|(_, cancel)| *cancel));
    assert_ne!(events[0].0, events[1].0);
    assert_eq!(events, unchanged);
}

#[test]
fn bound_native_identity_mismatch_is_rejected_before_any_task_side_effect() {
    let (_tree, node, run) = fixture("cancel-binding-denied");
    let (_execution, task) = child(&node, &run);
    let mut wrong = binding(&node);
    wrong.home = khor_core::DeviceId([9; 32]);
    assert!(
        node.delegation_here(
            node.device(),
            DelegationAction::Bound {
                binding: wrong,
                action: BoundAction::Cancel { task: task.clone() }
            }
        )
        .is_err()
    );
    let mut wrong = binding(&node);
    wrong.caller = khor_core::DeviceId([8; 32]);
    assert!(
        node.delegation_here(
            node.device(),
            DelegationAction::Bound {
                binding: wrong,
                action: BoundAction::Cancel { task: task.clone() }
            }
        )
        .is_err()
    );
    let record = node
        .delegation_store()
        .unwrap()
        .get(
            node.device(),
            &delegation::AgentTarget {
                home: node.device(),
                agent: "fixture".into(),
            },
            &task,
        )
        .unwrap();
    assert!(!record.cancellation_requested);
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn completed_is_terminal_other_and_unknown_input_never_replays_send() {
    let (_tree, node, run) = fixture("cancel-terminal-other");
    let (mut execution, task) = child(&node, &run);
    execution
        .update(protocol::TaskState::Completed, None, None)
        .unwrap();
    let parent = TaskRef {
        home: node.device(),
        id: run.clone(),
    };
    delegation::track_child(
        node.root(),
        &parent,
        "diagnostic",
        &binding(&node),
        "unknown-input-never-resend",
        None,
    )
    .unwrap();
    let mut peer = Peer::new(&node, Mode::Reply);
    persist(&node, &run);
    let first = worker(&node, Operation::CancelStep { run: run.clone() });
    let refused = status(&node, &run);
    update_receipt(&node, &run, |value| value["next_retry_at_ms"] = json!(0));
    let second = worker(&node, Operation::CancelStep { run: run.clone() });
    let snapshot = status(&node, &run);
    let events = peer.events.lock().unwrap().clone();
    let cleanup = peer.finish();
    assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
    assert!(first.is_ok() && second.is_ok());
    assert_eq!(refused["counts"]["OutcomeUnknown"], 1);
    assert_eq!(refused["all_known_children_canceled"], false);
    assert_eq!(snapshot["counts"]["TerminalOther"], 1);
    assert_eq!(snapshot["counts"]["NeedsAttention"], 1);
    assert_eq!(snapshot["all_known_children_canceled"], false);
    assert_eq!(events, vec![(task.clone(), true), (task, false)]);
}

struct HttpExecutor;
impl a2a_server::AgentExecutor for HttpExecutor {
    fn execute(
        &self,
        ctx: a2a_server::ExecutorContext,
    ) -> futures::stream::BoxStream<'static, Result<protocol::StreamResponse, protocol::A2AError>>
    {
        use futures::StreamExt;
        let awaiting_input = ctx.message.as_ref().is_some_and(|message| message.parts.iter().any(|part| matches!(&part.content, protocol::PartContent::Text(text) if text == "input-required")));
        let task = protocol::Task {
            id: ctx.task_id,
            context_id: ctx.context_id,
            status: protocol::TaskStatus {
                state: if awaiting_input {
                    protocol::TaskState::InputRequired
                } else {
                    protocol::TaskState::Working
                },
                message: None,
                timestamp: None,
            },
            history: ctx.message.map(|message| vec![message]),
            artifacts: None,
            metadata: None,
        };
        let first = futures::stream::iter([Ok(protocol::StreamResponse::Task(task))]);
        if awaiting_input {
            Box::pin(first)
        } else {
            Box::pin(first.chain(futures::stream::pending()))
        }
    }
    fn cancel(
        &self,
        _: a2a_server::ExecutorContext,
    ) -> futures::stream::BoxStream<'static, Result<protocol::StreamResponse, protocol::A2AError>>
    {
        Box::pin(futures::stream::empty())
    }
}
async fn http_service() -> (
    String,
    Arc<std::sync::atomic::AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", socket.local_addr().unwrap());
    let card = protocol::AgentCard {
        name: "Cancellation binding fixture".into(),
        description: "Owned official A2A server".into(),
        version: "1".into(),
        supported_interfaces: vec![protocol::AgentInterface {
            url: address.clone(),
            protocol_binding: protocol::TRANSPORT_PROTOCOL_JSONRPC.into(),
            protocol_version: "1.0".into(),
            tenant: None,
        }],
        capabilities: Default::default(),
        default_input_modes: vec!["text/plain".into()],
        default_output_modes: vec!["text/plain".into()],
        skills: vec![],
        provider: None,
        documentation_url: None,
        icon_url: None,
        security_schemes: None,
        security_requirements: None,
        signatures: None,
    };
    let handler = Arc::new(a2a_server::DefaultRequestHandler::new(
        HttpExecutor,
        a2a_server::InMemoryTaskStore::new(),
    ));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = calls.clone();
    let router = a2a_server::jsonrpc::jsonrpc_router(handler)
        .merge(a2a_server::agent_card::agent_card_router(Arc::new(
            a2a_server::StaticAgentCard::new(card),
        )))
        .layer(axum::middleware::from_fn(
            move |request: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let calls = counted.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    next.run(request).await
                }
            },
        ));
    let task = tokio::spawn(async move {
        axum::serve(socket, router).await.unwrap();
    });
    (address, calls, task)
}
async fn local_call(node: &Node, action: DelegationAction) -> Result<DelegationAnswer, String> {
    let root = node.root().clone();
    let caller = node.device();
    tokio::task::spawn_blocking(move || Node::open(root)?.delegation_here(caller, action))
        .await
        .map_err(|e| e.to_string())?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http_alias_change_does_not_rebind_owned_get_cancel_or_new_send() {
    let (_tree, node, _run) = fixture("cancel-http-bound-endpoint");
    let (a, a_calls, a_task) = http_service().await;
    let (b, b_calls, b_task) = http_service().await;
    let root = node.root().clone();
    let endpoint_a =
        tokio::task::spawn_blocking(move || Node::open(root)?.connect_a2a("fixture", &a, None))
            .await
            .unwrap()
            .unwrap();
    let DelegationAnswer::Binding { binding } = local_call(
        &node,
        DelegationAction::ResolveBinding {
            agent: "a2a:fixture".into(),
            task: None,
        },
    )
    .await
    .unwrap() else {
        panic!("no real HTTP binding");
    };
    let message = protocol::Message::new(protocol::Role::User, vec![protocol::Part::text("wait")]);
    let DelegationAnswer::Task { record } = local_call(
        &node,
        DelegationAction::Bound {
            binding: binding.clone(),
            action: BoundAction::Send {
                message,
                parent: None,
            },
        },
    )
    .await
    .unwrap() else {
        panic!("no owned HTTP task");
    };
    let root = node.root().clone();
    let endpoint_b =
        tokio::task::spawn_blocking(move || Node::open(root)?.connect_a2a("other", &b, None))
            .await
            .unwrap()
            .unwrap();
    let mut switched = serde_json::to_value(endpoint_b).unwrap();
    switched["name"] = json!("a2a:fixture");
    let alias = node
        .root()
        .join(".khor/a2a/http")
        .join(blake3::hash(b"a2a:fixture").to_hex().as_str())
        .join("endpoint.json");
    crate::link::write_private(&alias, &serde_json::to_vec(&switched).unwrap()).unwrap();
    let (before_a, before_b) = (
        a_calls.load(Ordering::SeqCst),
        b_calls.load(Ordering::SeqCst),
    );
    let new_message = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("must not reach B")],
    );
    let refused_send = local_call(
        &node,
        DelegationAction::Bound {
            binding: binding.clone(),
            action: BoundAction::Send {
                message: new_message,
                parent: None,
            },
        },
    )
    .await;
    let canceled = local_call(
        &node,
        DelegationAction::Bound {
            binding: binding.clone(),
            action: BoundAction::Cancel {
                task: record.task.id.clone(),
            },
        },
    )
    .await;
    let read = local_call(
        &node,
        DelegationAction::Bound {
            binding: binding.clone(),
            action: BoundAction::Get {
                task: record.task.id.clone(),
            },
        },
    )
    .await;
    let (after_a, after_b) = (
        a_calls.load(Ordering::SeqCst),
        b_calls.load(Ordering::SeqCst),
    );
    a_task.abort();
    b_task.abort();
    let _ = a_task.await;
    let _ = b_task.await;
    assert!(refused_send.is_err());
    assert!(
        canceled.is_ok() && read.is_ok(),
        "cancel={canceled:?} read={read:?}"
    );
    assert_eq!(
        after_a,
        before_a + 2,
        "actual original service receives Cancel and Get"
    );
    assert_eq!(after_b, before_b, "new alias receives no task RPC");
    assert!(
        !serde_json::to_string(&binding)
            .unwrap()
            .contains(&endpoint_a.card_url)
    );
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn late_attempt_reply_cannot_overwrite_a_new_receipt() {
    let (_tree, node, run) = fixture("cancel-stale-attempt");
    let (_execution, _) = child(&node, &run);
    let mut peer = Peer::new(&node, Mode::HoldFirstCancel);
    persist(&node, &run);
    let root = node.root().clone();
    let attempt_run = run.clone();
    let owner = thread::spawn(move || {
        worker(
            &Node::open(root).unwrap(),
            Operation::CancelStep { run: attempt_run },
        )
    });
    let by = Instant::now() + Duration::from_secs(3);
    while peer.events.lock().unwrap().is_empty() && Instant::now() < by {
        thread::sleep(Duration::from_millis(5));
    }
    let entered = !peer.events.lock().unwrap().is_empty();
    if entered {
        update_receipt(&node, &run, |value| {
            value["attempt_id"] = json!("newer-attempt-fence");
            value["phase"] = json!("Queued");
            value["revision"] = json!(1000);
        });
    }
    peer.release.store(true, Ordering::Release);
    let result = owner.join().unwrap();
    let bytes = fs::read(&receipt_paths(&node, &run)[0]).unwrap();
    let receipt: Value = serde_json::from_slice(&bytes).unwrap();
    let cleanup = peer.finish();
    assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
    assert!(entered);
    assert!(result.is_err());
    assert_eq!(receipt["attempt_id"], "newer-attempt-fence");
    assert_eq!(receipt["phase"], "Queued");
    assert_eq!(receipt["revision"], 1000);
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn discovery_commits_only_a_bounded_page_before_reporting_scan_complete() {
    let (_tree, node, run) = fixture("cancel-discovery-page");
    let mut executions = Vec::new();
    for _ in 0..40 {
        executions.push(child(&node, &run).0);
    }
    let mut peer = Peer::new(&node, Mode::Reply);
    let first = persist(&node, &run);
    let first_files = receipt_paths(&node, &run).len();
    let step = worker(&node, Operation::CancelStep { run: run.clone() });
    let second = status(&node, &run);
    let second_files = receipt_paths(&node, &run).len();
    let cleanup = peer.finish();
    assert!(cleanup.is_ok(), "cleanup={cleanup:?}");
    assert!(step.is_ok());
    assert_eq!(first_files, 32);
    assert_eq!(first["scan_complete"], false);
    assert_eq!(first["all_known_children_canceled"], false);
    assert_eq!(second_files, 40);
    assert_eq!(second["scan_complete"], true);
    assert_eq!(second["all_known_children_canceled"], false);
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn resting_lifecycle_can_persist_and_query_without_admitting_network() {
    let (_tree, node, run) = fixture("cancel-resting-admission");
    let (_execution, _) = child(&node, &run);
    let mut peer = Peer::new(&node, Mode::Reply);
    node.resting.store(true, Ordering::Relaxed);
    let persisted = node
        .khor_agent_here(Action::RetryCancellation { run: run.clone() })
        .await
        .unwrap();
    for _ in 0..8 {
        node.reconcile_agent().unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let quiet = peer.events.lock().unwrap().is_empty();
    node.resting.store(false, Ordering::Relaxed);
    let by = Instant::now() + Duration::from_secs(3);
    while peer.events.lock().unwrap().is_empty() && Instant::now() < by {
        node.reconcile_agent().unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let resumed = !peer.events.lock().unwrap().is_empty();
    let by = Instant::now() + Duration::from_secs(6);
    let retired = loop {
        let result = remote_cancel::release_test_root(node.root());
        if result.is_ok() || Instant::now() >= by {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    let cleanup = peer.finish();
    assert!(
        cleanup.is_ok() && retired.is_ok(),
        "peer={cleanup:?} workers={retired:?}"
    );
    assert_eq!(
        persisted.data.unwrap()["durability"]["state"],
        "DurablyQueued"
    );
    assert!(quiet);
    assert!(resumed);
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn a_bad_source_or_receipt_stays_unresolved_while_valid_siblings_cancel() {
    for bad_receipt in [false, true] {
        let (_tree, node, run) = fixture("cancel-poison-sibling");
        let mut executions = Vec::new();
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let (execution, task) = child(&node, &run);
            executions.push(execution);
            tasks.push(task);
        }
        let mut peer = Peer::new(&node, Mode::Reply);
        let bad = if bad_receipt {
            persist(&node, &run);
            receipt_paths(&node, &run)[0].clone()
        } else {
            let folder = delegation::children_dir(node.root(), &run);
            let mut paths: Vec<_> = fs::read_dir(folder)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
                .collect();
            assert_eq!(paths.len(), 3, "all three business links must exist");
            paths.sort();
            paths[0].clone()
        };
        let prior: Value = serde_json::from_slice(&fs::read(&bad).unwrap()).unwrap();
        let bad_task = prior["task"].as_str().unwrap().to_owned();
        let damaged = b"{deliberately damaged child bytes";
        crate::link::write_private(&bad, damaged).unwrap();
        if !bad_receipt {
            persist(&node, &run);
        }
        let by = Instant::now() + Duration::from_secs(8);
        while Instant::now() < by {
            node.reconcile_agent().unwrap();
            if peer
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, cancel)| *cancel)
                .count()
                >= 2
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        for execution in &mut executions {
            execution
                .update(protocol::TaskState::Canceled, None, None)
                .unwrap();
        }
        let by = Instant::now() + Duration::from_secs(5);
        let mut snapshot;
        loop {
            node.reconcile_agent().unwrap();
            snapshot = status(&node, &run);
            if snapshot["counts"]["ConfirmedCanceled"] == 2 || Instant::now() >= by {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let events = peer.events.lock().unwrap().clone();
        let preserved = fs::read(&bad).unwrap() == damaged;
        let retired = drain(&node).await;
        let cleanup = peer.finish();
        assert!(
            retired.is_ok() && cleanup.is_ok(),
            "workers={retired:?} peer={cleanup:?}"
        );
        let actual: std::collections::HashSet<_> = events
            .iter()
            .filter(|(_, cancel)| *cancel)
            .map(|(task, _)| task.clone())
            .collect();
        let expected: std::collections::HashSet<_> =
            tasks.into_iter().filter(|task| task != &bad_task).collect();
        assert_eq!(actual, expected);
        assert!(events.iter().all(|(task, _)| task != &bad_task));
        assert!(preserved);
        assert_eq!(snapshot["counts"]["ConfirmedCanceled"], 2);
        assert_eq!(snapshot["counts"]["NeedsAttention"], 1);
        assert_eq!(snapshot["issue_count"], 1);
        assert_eq!(snapshot["all_known_children_canceled"], false);
    }
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn pending_reference_recovers_only_by_its_hash_and_preserves_bad_bytes() {
    for oversized in [false, true] {
        let (_tree, node, first) = fixture("cancel-pending-reference");
        let second = historical_run(&node);
        let (mut first_execution, first_task) = child(&node, &first);
        let (mut second_execution, second_task) = child(&node, &second);
        persist(&node, &first);
        persist(&node, &second);
        first_execution
            .update(protocol::TaskState::Canceled, None, None)
            .unwrap();
        second_execution
            .update(protocol::TaskState::Canceled, None, None)
            .unwrap();
        let hash = blake3::hash(first.as_bytes()).to_hex().to_string();
        let root = node.root().join(".khor/agent/cancellations");
        let marker = root.join("pending").join(format!("{hash}.json"));
        let damaged = if oversized {
            vec![b'x'; 4097]
        } else {
            b"broken pending reference".to_vec()
        };
        crate::link::write_private(&marker, &damaged).unwrap();
        let mut peer = Peer::new(&node, Mode::Reply);
        let by = Instant::now() + Duration::from_secs(6);
        let (mut first_snapshot, mut second_snapshot);
        loop {
            node.reconcile_agent().unwrap();
            first_snapshot = status(&node, &first);
            second_snapshot = status(&node, &second);
            if first_snapshot["counts"]["ConfirmedCanceled"] == 1
                && second_snapshot["counts"]["ConfirmedCanceled"] == 1
                || Instant::now() >= by
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let by = Instant::now() + Duration::from_millis(1100);
        let starts = remote_cancel::test_step_starts(node.root());
        while Instant::now() < by {
            node.reconcile_agent().unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let later_starts = remote_cancel::test_step_starts(node.root());
        let issue: Value =
            serde_json::from_slice(&fs::read(root.join(&hash).join("pending-issue.json")).unwrap())
                .unwrap();
        let preserved = if oversized {
            fs::read(&marker).unwrap() == damaged
        } else {
            fs::read(root.join(&hash).join("pending-original.bin")).unwrap() == damaged
        };
        let events = peer.events.lock().unwrap().clone();
        let retired = drain(&node).await;
        let cleanup = peer.finish();
        assert!(
            retired.is_ok() && cleanup.is_ok(),
            "workers={retired:?} peer={cleanup:?}"
        );
        assert!(preserved);
        assert!(events.contains(&(first_task, true)) && events.contains(&(second_task, true)));
        assert_eq!(issue["resolved"], !oversized);
        assert_eq!(first_snapshot["all_known_children_canceled"], !oversized);
        assert_eq!(second_snapshot["all_known_children_canceled"], true);
        assert_eq!(
            starts, later_starts,
            "attention/finished markers must not respawn network workers"
        );
    }
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn retry_deadline_bounds_actual_worker_starts_without_starving_a_fresh_run() {
    let (_tree, node, first) = fixture("cancel-real-worker-backoff");
    let mut runs = vec![first];
    for _ in 0..3 {
        runs.push(historical_run(&node));
    }
    let mut executions = Vec::new();
    let mut tasks = Vec::new();
    for run in &runs {
        let (execution, task) = child(&node, run);
        executions.push(execution);
        tasks.push(task);
        persist(&node, run);
    }
    let mut peer = Peer::new(&node, Mode::Reply);
    let mut acknowledgements = Vec::new();
    for run in &runs {
        worker(&node, Operation::CancelStep { run: run.clone() }).unwrap();
        acknowledgements.push(status(&node, run));
    }
    let original_events = peer.events.lock().unwrap().clone();
    peer.events.lock().unwrap().clear();
    let due = now() + 6000;
    for run in &runs {
        update_receipt(&node, run, |receipt| {
            receipt["next_retry_at_ms"] = json!(due)
        });
    }
    let by = Instant::now() + Duration::from_secs(3);
    while Instant::now() < by {
        node.reconcile_agent().unwrap();
        let starts = remote_cancel::test_step_starts(node.root());
        if starts.len() == 4 && starts.iter().all(|(_, pid, _)| absent(*pid)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    node.reconcile_agent().unwrap();
    let initial = remote_cancel::test_step_starts(node.root());
    let quiet_until = Instant::now() + Duration::from_millis(700);
    while Instant::now() < quiet_until {
        node.reconcile_agent().unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let quiet = remote_cancel::test_step_starts(node.root());
    let fresh = historical_run(&node);
    let (fresh_execution, fresh_task) = child(&node, &fresh);
    executions.push(fresh_execution);
    let metadata_by = Instant::now() + Duration::from_secs(6);
    while !remote_cancel::test_reap_metadata() && Instant::now() < metadata_by {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    persist(&node, &fresh);
    let mut fresh_seen_at = None;
    let by = Instant::now() + Duration::from_secs(8);
    while Instant::now() < by {
        node.reconcile_agent().unwrap();
        let events = peer.events.lock().unwrap().clone();
        if fresh_seen_at.is_none() && events.contains(&(fresh_task.clone(), true)) {
            fresh_seen_at = Some(now());
        }
        if tasks
            .iter()
            .all(|task| events.contains(&(task.clone(), false)))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let starts = remote_cancel::test_step_starts(node.root());
    let events = peer.events.lock().unwrap().clone();
    let retired = drain(&node).await;
    let cleanup = peer.finish();
    assert!(
        retired.is_ok() && cleanup.is_ok(),
        "workers={retired:?} peer={cleanup:?}"
    );
    assert!(
        acknowledgements
            .iter()
            .all(|snapshot| snapshot["counts"]["AckPending"] == 1)
    );
    assert!(
        tasks
            .iter()
            .all(|task| original_events.contains(&(task.clone(), true)))
    );
    assert_eq!(initial.len(), 4);
    assert!(initial.iter().all(|(_, pid, _)| absent(*pid)));
    assert_eq!(
        initial, quiet,
        "actual OS process starts must stay stable before retry eligibility"
    );
    assert!(fresh_seen_at.is_some_and(|at| at < due));
    assert!(
        tasks
            .iter()
            .all(|task| events.contains(&(task.clone(), false)))
    );
    for run in &runs {
        let run_starts: Vec<_> = starts.iter().filter(|(id, _, _)| id == run).collect();
        assert!(run_starts.len() >= 2);
        assert!(run_starts.iter().skip(1).all(|(_, _, at)| *at >= due));
    }
    assert!(starts.iter().all(|(_, pid, _)| absent(*pid)));
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn committed_intent_with_invalid_reply_remains_unknown_until_real_status() {
    let (_tree, node, run) = fixture("cancel-invalid-metadata-reply");
    let active = Arc::new(Running::new(run.clone()));
    running()
        .lock()
        .unwrap()
        .insert(node.root().clone(), active.clone());
    let stopped = node
        .khor_agent_here(Action::Stop { run: run.clone() })
        .await
        .unwrap();
    let requested = active.stopped.load(Ordering::SeqCst);
    running().lock().unwrap().remove(node.root());
    // Await the actual producer before using its committed Status reply as the
    // decoder fixture. Stop may honestly return before that commit finishes.
    let retried = node
        .khor_agent_here(Action::RetryCancellation { run: run.clone() })
        .await
        .unwrap();
    // The actual metadata worker committed the intent. Inject only at the
    // production reply decoder, without fabricating a file or a successful ACK.
    let committed = worker(
        &node,
        Operation::CancellationMetadata {
            operation: Metadata::Status { run: run.clone() },
        },
    )
    .unwrap();
    let mut wrong_run: Value = serde_json::from_str(&committed).unwrap();
    wrong_run["snapshot"]["run"] = json!("different-run");
    let mut no_snapshot: Value = serde_json::from_str(&committed).unwrap();
    no_snapshot["snapshot"] = Value::Null;
    let mut fake_absent: Value = serde_json::from_str(&committed).unwrap();
    fake_absent["snapshot"] = json!({"run":run,"durability":{"state":"Absent"}});
    let payloads = [
        "{".to_owned(),
        no_snapshot.to_string(),
        wrong_run.to_string(),
        fake_absent.to_string(),
    ];
    let decoded: Vec<_> = payloads
        .into_iter()
        .map(|raw| {
            (
                remote_cancel::decode_metadata_result(&run, true, Ok(raw.clone())),
                remote_cancel::decode_metadata_result(&run, false, Ok(raw)),
            )
        })
        .collect();
    let actual = status(&node, &run);
    let retired = drain(&node).await;
    assert!(retired.is_ok(), "workers={retired:?}");
    assert!(requested);
    let initial = stopped.data.unwrap();
    assert!(
        initial["durability"]["state"] == "DurablyQueued"
            || (initial["durability"]["state"] == "PersistenceUnknown"
                && initial["durability"]["code"] == "MetadataOutcomeUnknown"),
        "initial Stop snapshot: {initial}"
    );
    assert_eq!(retried.data.unwrap()["durability"]["state"], "DurablyQueued");
    for (persist, query) in decoded {
        assert_eq!(persist["durability"]["state"], "PersistenceUnknown");
        assert_eq!(query["durability"]["state"], "Unavailable");
        assert_eq!(persist["run"], run);
    }
    assert_eq!(actual["durability"]["state"], "DurablyQueued");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn known_delegate_continuation_survives_lost_reply_and_stop_cancels_exact_task() {
    let (_tree, node, run) = fixture("cancel-known-delegate-continuation");
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
    drain(&node).await.unwrap();
    let old_run = historical_run(&node);
    let old_parent = TaskRef {
        home: node.device(),
        id: old_run,
    };
    let (address, _calls, http_owner) = http_service().await;
    let root = node.root().clone();
    tokio::task::spawn_blocking(move || Node::open(root)?.connect_a2a("fixture", &address, None))
        .await
        .unwrap()
        .unwrap();
    let DelegationAnswer::Binding { binding } = local_call(
        &node,
        DelegationAction::ResolveBinding {
            agent: "a2a:fixture".into(),
            task: None,
        },
    )
    .await
    .unwrap() else {
        panic!("no actual HTTP binding");
    };
    let initial = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("input-required")],
    );
    let DelegationAnswer::Task { record: original } = local_call(
        &node,
        DelegationAction::Bound {
            binding: binding.clone(),
            action: BoundAction::Send {
                message: initial,
                parent: Some(old_parent.clone()),
            },
        },
    )
    .await
    .unwrap() else {
        panic!("no actual accepted HTTP task");
    };
    let task = original.task.id.clone();
    let context = original.task.context_id.clone();
    let mut peer = Peer::new(&node, Mode::HoldContinuedSend);
    let mut saved: Saved =
        serde_json::from_slice(&fs::read(file(node.root(), &run)).unwrap()).unwrap();
    saved.record.status = "running".into();
    saved.record.ended_at_ms = None;
    save(node.root(), &saved).unwrap();
    // This fixture owns the standalone real MCP process/control. R0 separately
    // covers six actual Pi processes and the executor's terminal-save path.
    let active = Arc::new(Running::new(run.clone()));
    running()
        .lock()
        .unwrap()
        .insert(node.root().clone(), active.clone());
    struct Registered(PathBuf);
    impl Drop for Registered {
        fn drop(&mut self) {
            let active = running()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&self.0);
            if let Some(active) = active {
                active.request_stop();
            }
        }
    }
    let _registration = Registered(node.root().clone());
    let control = Arc::new(crate::agent_process::Control::default());
    assert!(active.register_worker(control.clone()));
    let request = "known-continuation-held-receipt";
    let mut owner = Worker::spawn(Request { root: node.root().clone(), operation: Operation::Tool { run: run.clone(), authorized: true, name: "delegate".into(), args: json!({"machine":node.device_str(),"agent":"a2a:fixture","prompt":"continue working","request":request,"task":task,"context":context}) } }, control.clone(), Duration::from_secs(10), None).unwrap();
    let pid = owner.pid().unwrap();
    let by = Instant::now() + Duration::from_secs(5);
    while !peer.events.lock().unwrap().contains(&(task.clone(), false)) && Instant::now() < by {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let applied = peer.events.lock().unwrap().contains(&(task.clone(), false));
    let links = delegation::children_dir(node.root(), &run);
    let before_link = fs::read(links.join(delegation::child_key(&binding, request).unwrap()))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let stopped = node
        .khor_agent_here(Action::Stop { run: run.clone() })
        .await
        .unwrap();
    let tool_result = loop {
        if let Some(result) = owner.poll() {
            break result;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    };
    drop(owner);
    let local_reaped = absent(pid);
    active.release_worker(&control);
    running().lock().unwrap().remove(node.root());
    saved.record.status = "cancelled".into();
    saved.record.ended_at_ms = Some(now());
    save(node.root(), &saved).unwrap();
    remote_cancel::local_retired(node.root().clone(), run.clone());
    let by = Instant::now() + Duration::from_secs(6);
    let mut snapshot;
    loop {
        node.reconcile_agent().unwrap();
        snapshot = status(&node, &run);
        if snapshot["counts"]["ConfirmedCanceled"] == 1 || Instant::now() >= by {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let current = local_call(
        &node,
        DelegationAction::Bound {
            binding: binding.clone(),
            action: BoundAction::Get { task: task.clone() },
        },
    )
    .await;
    let events = peer.events.lock().unwrap().clone();
    let retired = drain(&node).await;
    let cleanup = peer.finish();
    http_owner.abort();
    let _ = http_owner.await;
    assert!(
        retired.is_ok() && cleanup.is_ok(),
        "workers={retired:?} peer={cleanup:?}"
    );
    assert!(
        applied,
        "actual continued Send must apply before its receipt is held"
    );
    assert!(local_reaped && tool_result.is_err());
    assert_eq!(
        original.task.status.state,
        protocol::TaskState::InputRequired
    );
    assert_eq!(original.parent.as_ref(), Some(&old_parent));
    assert_eq!(
        before_link.unwrap()["task"],
        task,
        "known task is durable before the actual Send reply"
    );
    let initial = stopped.data.unwrap();
    assert!(
        initial["durability"]["state"] == "DurablyQueued"
            || (initial["durability"]["state"] == "PersistenceUnknown"
                && initial["durability"]["code"] == "MetadataOutcomeUnknown"),
        "initial Stop snapshot: {initial}"
    );
    assert!(events.contains(&(task.clone(), true)));
    assert_eq!(snapshot["durability"]["state"], "DurablyQueued");
    assert_eq!(snapshot["counts"]["ConfirmedCanceled"], 1);
    let DelegationAnswer::Task { record } = current.unwrap() else {
        panic!("no actual owned task after Stop");
    };
    assert_eq!(record.task.id, task);
    assert_eq!(record.task.status.state, protocol::TaskState::Canceled);
    assert_eq!(record.parent.as_ref(), Some(&old_parent));
}

#[tokio::test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
async fn retry_preserves_parseable_foreign_issue_identity_and_cancels_valid_receipts() {
    for wrong_field in ["version", "intent_id", "child_key"] {
        let (_tree, node, run) = fixture("cancel-foreign-issue-identity");
        let mut executions = Vec::new();
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let (execution, task) = child(&node, &run);
            executions.push(execution);
            tasks.push(task);
        }
        persist(&node, &run);
        let child = receipt_paths(&node, &run)[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let dir = node
            .root()
            .join(".khor/agent/cancellations")
            .join(blake3::hash(run.as_bytes()).to_hex().as_str());
        let intent: Value =
            serde_json::from_slice(&fs::read(dir.join("intent.json")).unwrap()).unwrap();
        let mut issue = json!({"version":1,"intent_id":intent["intent_id"],"child_key":child,"code":"UnknownIssueFixture","resolved":false});
        issue[wrong_field] = if wrong_field == "version" {
            json!(2)
        } else {
            json!("foreign-identity")
        };
        let raw = serde_json::to_vec(&issue).unwrap();
        let folder = dir.join("issues");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("{}.json", blake3::hash(child.as_bytes()).to_hex()));
        crate::link::write_private(&path, &raw).unwrap();
        let retried = worker(
            &node,
            Operation::CancellationMetadata {
                operation: Metadata::Persist {
                    run: run.clone(),
                    reason: "Retry".into(),
                },
            },
        );
        for execution in &mut executions {
            execution
                .update(protocol::TaskState::Canceled, None, None)
                .unwrap();
        }
        let mut peer = Peer::new(&node, Mode::Reply);
        let by = Instant::now() + Duration::from_secs(6);
        let mut snapshot;
        loop {
            node.reconcile_agent().unwrap();
            snapshot = status(&node, &run);
            if snapshot["counts"]["ConfirmedCanceled"] == 3 || Instant::now() >= by {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let preserved = fs::read(&path).unwrap() == raw;
        let events = peer.events.lock().unwrap().clone();
        let retired = drain(&node).await;
        let cleanup = peer.finish();
        assert!(
            retired.is_ok() && cleanup.is_ok(),
            "workers={retired:?} peer={cleanup:?}"
        );
        assert!(retried.is_ok());
        assert!(
            preserved,
            "the parseable but foreign {wrong_field} issue must retain its original bytes"
        );
        assert!(
            tasks
                .iter()
                .all(|task| events.contains(&(task.clone(), true)))
        );
        assert_eq!(snapshot["counts"]["ConfirmedCanceled"], 3);
        assert_eq!(snapshot["counts"]["NeedsAttention"], 1);
        assert_eq!(snapshot["issue_count"], 1);
        assert_eq!(snapshot["all_known_children_canceled"], false);
        println!(
            "foreign issue {wrong_field}: preserved; three actual owned Task Cancels; unresolved=1"
        );
    }
}

#[path = "remote_cancel_input_tests.rs"]
mod input_tests;
