//! Native exact-input receipt integration. The CLI is a separate real worker;
//! this loopback IPC peer invokes the real authenticated-home Store handler.
//! Cross-machine authentication is covered separately by delegation_wire.
use super::*;

struct InputPeer {
    stop: Arc<AtomicBool>,
    owner: Option<JoinHandle<Result<usize, String>>>,
    endpoint: PathBuf,
    task_calls: Arc<Mutex<Vec<String>>>,
}
impl InputPeer {
    fn new(node: &Node) -> Self {
        Self::with_task_fallback(node, false)
    }
    fn with_task_fallback(node: &Node, allow_task: bool) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let cookie = crate::link::fresh_hex().unwrap();
        let endpoint = node.root().join(".khor/endpoint.json");
        crate::link::write_private(
            &endpoint,
            &serde_json::to_vec(&json!({
                "id":node.device_str(),"addrs":[],"pid":std::process::id(),
                "ipc_port":listener.local_addr().unwrap().port(),"ipc_cookie":cookie
            }))
            .unwrap(),
        )
        .unwrap();
        let root = node.root().clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let task_calls = Arc::new(Mutex::new(Vec::new()));
        let observed = task_calls.clone();
        let owner = thread::spawn(move || {
            let by = Instant::now() + Duration::from_secs(20);
            let mut count = 0;
            while !stopping.load(Ordering::Acquire) && Instant::now() < by {
                match listener.accept() {
                    Ok((mut stream, address)) => {
                        if !address.ip().is_loopback() {
                            return Err("nonloopback input peer".into());
                        }
                        // Apple accept can inherit the listener's O_NONBLOCK.
                        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .map_err(|e| e.to_string())?;
                        stream
                            .set_write_timeout(Some(Duration::from_secs(2)))
                            .map_err(|e| e.to_string())?;
                        let mut bytes = Vec::new();
                        (&mut stream)
                            .take((crate::proto::MAX_FRAME + 1) as u64)
                            .read_to_end(&mut bytes)
                            .map_err(|e| e.to_string())?;
                        if bytes.len() > crate::proto::MAX_FRAME {
                            return Err("oversize input IPC".into());
                        }
                        let handoff: crate::ipc::Handoff = crate::proto::decode(&bytes)?;
                        if handoff.cookie != cookie {
                            return Err("input peer cookie mismatch".into());
                        }
                        let node = Node::open(root.clone())?;
                        let crate::ipc::Op::Delegation { machine, action } = handoff.op else {
                            return Err("not input Delegation".into());
                        };
                        let (binding, task_call) = match &action {
                            DelegationAction::Bound {
                                binding,
                                action: BoundAction::CancelInput { .. },
                            } => (binding, None),
                            DelegationAction::Bound {
                                binding,
                                action: BoundAction::Cancel { task },
                            } if allow_task => (binding, Some(format!("Cancel:{task}"))),
                            DelegationAction::Bound {
                                binding,
                                action: BoundAction::Get { task },
                            } if allow_task => (binding, Some(format!("Get:{task}"))),
                            _ => return Err("input peer rejected non-exact-input operation".into()),
                        };
                        if machine != node.device_str()
                            || binding.home != node.device()
                            || binding.caller != node.device()
                            || binding.agent != "fixture"
                        {
                            return Err("input peer identity mismatch".into());
                        }
                        // The negative regression deliberately permits and
                        // records a bad task-wide RPC, then invokes the real
                        // handler. Old code must fail on its actual mutation,
                        // never because this peer refused the operation.
                        if let Some(call) = task_call {
                            observed.lock().unwrap().push(call);
                        }
                        let answer = node.delegation_here(node.device(), action)?;
                        count += 1;
                        stream
                            .write_all(&crate::proto::encode(&crate::ipc::Reply::Delegation {
                                answer,
                            })?)
                            .map_err(|e| e.to_string())?;
                        stream
                            .shutdown(Shutdown::Write)
                            .map_err(|e| e.to_string())?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
            Ok(count)
        });
        Self {
            stop,
            owner: Some(owner),
            endpoint,
            task_calls,
        }
    }
    fn finish(&mut self) -> Result<usize, String> {
        self.stop.store(true, Ordering::Release);
        let result = self.owner.take().map_or(Ok(0), |owner| {
            owner.join().map_err(|_| "input peer panicked".to_owned())?
        });
        if self.endpoint.exists() {
            fs::remove_file(&self.endpoint).map_err(|e| e.to_string())?;
        }
        result
    }
}
impl Drop for InputPeer {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn unknown_native_receipt_recovers_task_and_keeps_working_as_ack_until_terminal() {
    let (_tree, node, run) = fixture("cancel-input-recovery");
    let parent = TaskRef {
        home: node.device(),
        id: run.clone(),
    };
    let bound = binding(&node);
    let message = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("response lost after acceptance")],
    );
    delegation::track_input(
        node.root(),
        &parent,
        "diagnostic",
        &bound,
        &message.message_id,
        None,
    )
    .unwrap();
    let store = node.delegation_store().unwrap();
    let target = delegation::AgentTarget {
        home: node.device(),
        agent: "fixture".into(),
    };
    let accepted = store
        .accept(node.device(), &target, &message, Some(parent))
        .unwrap();
    let task = accepted.record.task.id;
    // Retain only the actual remote execution; never publish the accepted task
    // into the caller's link (the response boundary was lost).
    let mut execution = TaskExecution::claim(store.clone(), &task, &message.message_id).unwrap();
    let mut peer = InputPeer::new(&node);
    let before = persist(&node, &run);
    let first = worker(&node, Operation::CancelStep { run: run.clone() });
    let ack = status(&node, &run);
    let actual = store.get(node.device(), &target, &task).unwrap();
    let paths = receipt_paths(&node, &run);
    let recovered: Value = serde_json::from_slice(&fs::read(&paths[0]).unwrap()).unwrap();
    let link: delegation::ChildLink = remote_cancel::read(
        &delegation::children_dir(node.root(), &run)
            .join(delegation::child_key(&bound, &message.message_id).unwrap()),
        16 * 1024,
    )
    .unwrap();
    execution
        .update(protocol::TaskState::Canceled, None, None)
        .unwrap();
    update_receipt(&node, &run, |value| value["next_retry_at_ms"] = json!(0));
    let second = worker(&node, Operation::CancelStep { run: run.clone() });
    let terminal = status(&node, &run);
    let cleanup = peer.finish();
    assert_eq!(cleanup.unwrap(), 2);
    assert!(
        first.is_ok() && second.is_ok(),
        "first={first:?} second={second:?}"
    );
    assert_eq!(before["counts"]["Queued"], 1);
    assert_eq!(ack["counts"]["AckPending"], 1);
    assert_eq!(ack["all_known_children_canceled"], false);
    assert_eq!(actual.task.status.state, protocol::TaskState::Working);
    assert!(actual.cancellation_requested);
    assert_eq!(link.task.as_deref(), Some(task.as_str()));
    assert_eq!(recovered["task"], task);
    assert_eq!(recovered["input"]["message"], message.message_id);
    assert_eq!(terminal["counts"]["ConfirmedCanceled"], 1);
    assert_eq!(terminal["all_known_children_canceled"], true);
    assert_eq!(terminal["late_input_fence"], false);
    assert_eq!(
        store
            .list(node.device(), &target, None, None, 100)
            .unwrap()
            .len(),
        1
    );
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn unknown_native_absence_is_fenced_without_claiming_a_task_canceled() {
    let (_tree, node, run) = fixture("cancel-input-absence");
    let parent = TaskRef {
        home: node.device(),
        id: run.clone(),
    };
    let bound = binding(&node);
    let message = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("late input")],
    );
    delegation::track_input(
        node.root(),
        &parent,
        "diagnostic",
        &bound,
        &message.message_id,
        None,
    )
    .unwrap();
    let mut peer = InputPeer::new(&node);
    persist(&node, &run);
    let result = worker(&node, Operation::CancelStep { run: run.clone() });
    let snapshot = status(&node, &run);
    let cleanup = peer.finish();
    assert_eq!(cleanup.unwrap(), 1);
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(snapshot["counts"]["InputFenced"], 1);
    assert_eq!(snapshot["all_known_children_canceled"], false);
    assert_eq!(snapshot["late_input_fence"], true);
    let store = delegation::Store::open(node.root().join(".khor/a2a"), node.device()).unwrap();
    let target = delegation::AgentTarget {
        home: node.device(),
        agent: "fixture".into(),
    };
    assert!(matches!(
        store.accept(node.device(), &target, &message, Some(parent)),
        Err(khor_a2a::Error::InputFenced)
    ));
    assert!(
        store
            .list(node.device(), &target, None, None, 100)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn original_input_parent_has_one_durable_first_writer_and_rejects_bad_existing_records() {
    let (_tree, node, _) = fixture("cancel-input-parent-index");
    let bound = binding(&node);
    let message = "racing-tail-input";
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut owners = Vec::new();
    for id in ["first-tail", "second-tail"] {
        let (root, bound, barrier) = (node.root().clone(), bound.clone(), barrier.clone());
        let parent = TaskRef {
            home: node.device(),
            id: id.into(),
        };
        owners.push(thread::spawn(move || {
            barrier.wait();
            delegation::original_parent(&root, &parent, &bound, message)
        }));
    }
    let results: Vec<_> = owners
        .into_iter()
        .map(|owner| owner.join().unwrap())
        .collect();
    let winner = results
        .iter()
        .find_map(|result| result.as_ref().ok())
        .expect("no first-parent writer succeeded");
    for result in &results {
        match result {
            Ok(parent) => assert_eq!(parent, winner),
            Err(error) => assert_eq!(error, "Busy"),
        }
    }
    let path = node
        .root()
        .join(".khor/agent/messages")
        .join(delegation::child_key(&bound, message).unwrap());
    let persisted: TaskRef = remote_cancel::read(&path, 16 * 1024).unwrap();
    assert_eq!(&persisted, winner);
    assert_eq!(
        delegation::original_parent(
            node.root(),
            &TaskRef {
                home: node.device(),
                id: "third-tail".into()
            },
            &bound,
            message
        )
        .unwrap(),
        *winner
    );
    for bad in [
        TaskRef {
            home: khor_core::DeviceId([9; 32]),
            id: "foreign-home".into(),
        },
        TaskRef {
            home: node.device(),
            id: "bad\nparent".into(),
        },
        TaskRef {
            home: node.device(),
            id: String::new(),
        },
    ] {
        let bytes = serde_json::to_vec(&bad).unwrap();
        crate::link::write_private(&path, &bytes).unwrap();
        let parent = TaskRef {
            home: node.device(),
            id: "legitimate-tail".into(),
        };
        let result =
            delegation::track_input(node.root(), &parent, "diagnostic", &bound, message, None);
        assert_eq!(result.unwrap_err(), "Original parent identity mismatch");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!delegation::children_dir(node.root(), &parent.id).exists());
    }
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn recovered_input_scope_cannot_disappear_after_scan_complete_or_cancel_a_new_turn() {
    let (_tree, node, run) = fixture("cancel-input-scope-recovery");
    let parent = TaskRef {
        home: node.device(),
        id: run.clone(),
    };
    let bound = binding(&node);
    let store = node.delegation_store().unwrap();
    let target = delegation::AgentTarget {
        home: node.device(),
        agent: "fixture".into(),
    };
    let first = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("old input")],
    );
    delegation::track_input(
        node.root(),
        &parent,
        "diagnostic",
        &bound,
        &first.message_id,
        None,
    )
    .unwrap();
    let task = store
        .accept(node.device(), &target, &first, Some(parent.clone()))
        .unwrap()
        .record
        .task
        .id;
    let mut old = TaskExecution::claim(store.clone(), &task, &first.message_id).unwrap();
    old.update(protocol::TaskState::InputRequired, None, None)
        .unwrap();
    drop(old);
    let mut next = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("new input")],
    );
    next.task_id = Some(task.clone());
    store
        .accept(node.device(), &target, &next, Some(parent))
        .unwrap();
    let execution = TaskExecution::claim(store.clone(), &task, &next.message_id).unwrap();
    let current = store.get(node.device(), &target, &task).unwrap();
    let mut peer = InputPeer::new(&node);
    persist(&node, &run);
    let resolved = worker(&node, Operation::CancelStep { run: run.clone() });
    let superseded = status(&node, &run);
    let path = receipt_paths(&node, &run).remove(0);
    let mut corrupt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    corrupt.as_object_mut().unwrap().remove("input");
    let corrupt_bytes = serde_json::to_vec(&corrupt).unwrap();
    crate::link::write_private(&path, &corrupt_bytes).unwrap();
    let missing_scope = status(&node, &run);
    let scope_step = worker(&node, Operation::CancelStep { run: run.clone() });
    let preserved_corrupt = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let missing_receipt = status(&node, &run);
    let missing_step = worker(&node, Operation::CancelStep { run: run.clone() });
    let final_record = store.get(node.device(), &target, &task).unwrap();
    let cleanup = peer.finish();
    drop(execution);
    assert_eq!(
        cleanup.unwrap(),
        1,
        "corrupt/missing receipt must not issue a task Cancel"
    );
    assert!(resolved.is_ok() && scope_step.is_ok() && missing_step.is_ok());
    assert_eq!(superseded["scan_complete"], true);
    assert_eq!(superseded["counts"]["InputSuperseded"], 1);
    assert_eq!(superseded["all_known_children_canceled"], false);
    for snapshot in [missing_scope, missing_receipt] {
        assert!(snapshot["issue_count"].as_u64().unwrap() >= 1, "{snapshot}");
        assert_eq!(snapshot["all_known_children_canceled"], false);
        assert_eq!(snapshot["late_input_fence"], false);
    }
    assert_eq!(preserved_corrupt, corrupt_bytes);
    assert!(!path.exists());
    assert_eq!(final_record, current);
    assert_eq!(final_record.task.status.state, protocol::TaskState::Working);
    assert!(!final_record.cancellation_requested);
}

#[test]
#[ignore = "requires exact fresh CLI via KHOR_TEST_MCP_EXECUTABLE"]
fn normal_send_reply_and_retry_keep_input_scope_and_never_cancel_a_new_turn() {
    let (_tree, node, run) = fixture("cancel-input-normal-reply");
    let parent = TaskRef {
        home: node.device(),
        id: run.clone(),
    };
    let binding = binding(&node);
    let first = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("first input")],
    );
    delegation::track_input(
        node.root(),
        &parent,
        "diagnostic",
        &binding,
        &first.message_id,
        None,
    )
    .unwrap();
    let links = delegation::children_dir(node.root(), &run);
    let link_path = links.join(delegation::child_key(&binding, &first.message_id).unwrap());
    // Real short publication contention must defer discovery, not poison this
    // child or skip its cursor. Keep all behavior assertions until after the
    // bad-RPC opportunity, so old code reaches its real wrong cancellation.
    let publication = remote_cancel::gate(&links.join("links.lock")).unwrap();
    let deferred = persist(&node, &run);
    let deferred_progress: Value = remote_cancel::read(
        &node
            .root()
            .join(".khor/agent/cancellations")
            .join(blake3::hash(run.as_bytes()).to_hex().as_str())
            .join("progress.json"),
        4096,
    )
    .unwrap();
    drop(publication);
    let resumed = persist(&node, &run); // No Retry reset: the saved cursor must retry this child.
    let retry = || {
        worker(
            &node,
            Operation::CancellationMetadata {
                operation: Metadata::Persist {
                    run: run.clone(),
                    reason: "Retry".into(),
                },
            },
        )
        .unwrap()
    };
    let store = node.delegation_store().unwrap();
    let target = delegation::AgentTarget {
        home: node.device(),
        agent: "fixture".into(),
    };
    let accepted = store
        .accept(node.device(), &target, &first, Some(parent.clone()))
        .unwrap();
    let task = accepted.record.task.id.clone();
    // This is the same production publication used after a normal validated
    // Send reply (mcp.rs), not the CancelInput recovery publication.
    delegation::track_child(
        node.root(),
        &parent,
        "diagnostic",
        &binding,
        &first.message_id,
        Some(&accepted.record.task.id),
    )
    .unwrap();
    retry(); // Real Retry discovery promotes receipt.task from the normal link.
    let path = receipt_paths(&node, &run).remove(0);
    let ready: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let source_before = fs::read(&link_path).unwrap();
    let source: Value = serde_json::from_slice(&source_before).unwrap();
    let mut old = TaskExecution::claim(store.clone(), &task, &first.message_id).unwrap();
    old.update(protocol::TaskState::InputRequired, None, None)
        .unwrap();
    drop(old);
    let mut next = protocol::Message::new(
        protocol::Role::User,
        vec![protocol::Part::text("newer turn")],
    );
    next.task_id = Some(task.clone());
    store
        .accept(node.device(), &target, &next, Some(parent))
        .unwrap();
    let execution = TaskExecution::claim(store.clone(), &task, &next.message_id).unwrap();
    let current = store.get(node.device(), &target, &task).unwrap();
    let mut damaged = ready.clone();
    damaged.as_object_mut().unwrap().remove("input");
    let damaged_bytes = serde_json::to_vec(&damaged).unwrap();
    crate::link::write_private(&path, &damaged_bytes).unwrap();
    let mut peer = InputPeer::with_task_fallback(&node, true);
    let missing_scope = status(&node, &run);
    let stepped = worker(&node, Operation::CancelStep { run: run.clone() });
    let after_step = store.get(node.device(), &target, &task).unwrap();
    let preserved = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let missing_receipt = status(&node, &run);
    let missing_step = worker(&node, Operation::CancelStep { run: run.clone() });
    let after_missing = store.get(node.device(), &target, &task).unwrap();
    let source_after = fs::read(&link_path).unwrap();
    let task_calls = peer.task_calls.lock().unwrap().clone();
    let cleanup = peer.finish();
    drop(execution);
    assert!(cleanup.is_ok(), "peer cleanup={cleanup:?}");
    assert_eq!(
        after_step, current,
        "old input Stop must not mutate the newer turn; actual task RPCs={task_calls:?}"
    );
    assert_eq!(after_missing, current);
    assert!(
        task_calls.is_empty(),
        "task-wide fallback was actually delivered: {task_calls:?}"
    );
    assert_eq!(cleanup.unwrap(), 0);
    assert!(stepped.is_ok() && missing_step.is_ok());
    assert_eq!(deferred["scan_complete"], false);
    assert_eq!(deferred["issue_count"], 0);
    assert_eq!(deferred_progress["scan_cursor"], Value::Null);
    assert_eq!(resumed["scan_complete"], true);
    assert_eq!(resumed["issue_count"], 0);
    assert_eq!(ready["task"], task);
    assert_eq!(ready["input"]["message"], first.message_id);
    assert_eq!(source["input_scoped"], true);
    for snapshot in [missing_scope, missing_receipt] {
        assert!(snapshot["issue_count"].as_u64().unwrap() >= 1, "{snapshot}");
        assert_eq!(snapshot["all_known_children_canceled"], false);
        assert_eq!(snapshot["late_input_fence"], false);
    }
    assert_eq!(preserved, damaged_bytes);
    assert_eq!(source_after, source_before);
    assert!(!path.exists());
}
