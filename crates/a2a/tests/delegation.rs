use khor_a2a::{
    AgentTarget, DeviceId, Error, Execution, ExecutionFuture, Executor, SessionId, SessionRef,
    Store, TaskRef, dispatch,
    protocol::{self, Artifact, Message, Part, Role, TaskState},
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

const HOME: DeviceId = DeviceId([1; 32]);
const CALLER: DeviceId = DeviceId([2; 32]);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("khor-a2a-{}", protocol::new_task_id())))
    }
    fn store(&self) -> Store {
        Store::open(self.0.join("a2a"), HOME).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn target() -> AgentTarget {
    AgentTarget {
        home: HOME,
        agent: "fixture-agent".into(),
    }
}
fn input(text: &str) -> Message {
    Message::new(Role::User, vec![Part::text(text)])
}
fn session() -> SessionRef {
    SessionRef {
        home: HOME,
        id: SessionId("fixture-session".into()),
    }
}
fn accept(store: &Store, message: &Message) -> khor_a2a::Submission {
    store.accept(CALLER, &target(), message, None).unwrap()
}
fn artifact(text: &str) -> Artifact {
    Artifact {
        artifact_id: "report".into(),
        name: Some("Report".into()),
        description: None,
        parts: vec![Part::text(text), Part::data(json!({"passed":true}))],
        metadata: None,
        extensions: None,
    }
}

struct ReportExecutor;
impl Executor for ReportExecutor {
    fn execute<'a>(&'a self, execution: &'a mut Execution) -> ExecutionFuture<'a> {
        Box::pin(async move {
            execution.bind_session(session())?;
            if execution.message().text() == Some("ask") {
                execution.update(
                    TaskState::InputRequired,
                    Some(Message::new(
                        Role::Agent,
                        vec![Part::text("Which directory?")],
                    )),
                    None,
                )?;
            } else {
                execution.update(
                    TaskState::Completed,
                    None,
                    Some(artifact("Report is ready")),
                )?;
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn question_answer_and_new_task_share_a_session_but_keep_separate_results() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let first = input("ask");
    let accepted = accept(&store, &first);
    let id = &accepted.record.task.id;
    let waiting = dispatch(store.clone(), id, &first.message_id, &ReportExecutor)
        .await
        .unwrap();
    assert_eq!(waiting.task.status.state, TaskState::InputRequired);
    assert_eq!(waiting.context_session, Some(session()));

    let mut answer = input("/work/project");
    answer.task_id = Some(id.clone()); // Context is inferred from the task.
    accept(&store, &answer);
    let done = dispatch(store.clone(), id, &answer.message_id, &ReportExecutor)
        .await
        .unwrap();
    assert_eq!(done.task.status.state, TaskState::Completed);
    assert!(
        done.task
            .history
            .as_ref()
            .unwrap()
            .iter()
            .any(|m| m.text() == Some("Which directory?"))
    );
    assert_eq!(
        done.task.artifacts.as_ref().unwrap()[0].artifact_id,
        "report"
    );
    assert_eq!(accept(&store, &first).record, done);
    assert!(!accept(&store, &answer).accepted);
    assert!(matches!(
        store.request_cancel(CALLER, &target(), id),
        Err(Error::NotCancelable(_))
    ));

    let mut followup = input("Review the next change");
    followup.context_id = Some(done.task.context_id.clone());
    let next = accept(&store, &followup).record;
    assert_ne!(next.task.id, done.task.id);
    assert_eq!(next.context_session, Some(session()));
    assert!(
        next.task.artifacts.is_none(),
        "outputs belong to a task, not its context"
    );
    assert_eq!(next.task.history.as_ref().unwrap().len(), 1);
    assert!(matches!(
        store.accept(CALLER, &target(), &answer_with_new_id(answer), None),
        Err(Error::Terminal)
    ));
}
fn answer_with_new_id(mut message: Message) -> Message {
    message.message_id = protocol::new_message_id();
    message
}

#[test]
fn caller_target_and_context_are_checked_without_disclosing_another_callers_tasks() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let message = input("work");
    let accepted = accept(&store, &message).record;
    let stranger = DeviceId([3; 32]);
    assert!(matches!(
        store.get(stranger, &target(), &accepted.task.id),
        Err(Error::NotFound(_))
    ));
    assert!(
        store
            .list(stranger, &target(), None, None, 10)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .request_cancel(stranger, &target(), &accepted.task.id)
            .is_err()
    );
    let other_agent = AgentTarget {
        agent: "another-agent".into(),
        ..target()
    };
    assert!(matches!(
        store.get(CALLER, &other_agent, &accepted.task.id),
        Err(Error::NotFound(_))
    ));
    let mut followup = input("followup");
    followup.context_id = Some(accepted.task.context_id.clone());
    assert!(matches!(
        store.accept(stranger, &target(), &followup, None),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        store.accept(CALLER, &other_agent, &followup, None),
        Err(Error::NotFound(_))
    ));
    let wrong_home = AgentTarget {
        home: stranger,
        ..target()
    };
    assert!(matches!(
        store.accept(CALLER, &wrong_home, &message, None),
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn concurrent_delivery_and_retry_execute_a_side_effect_once() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let message = input("perform a side effect");
    let barrier = Arc::new(Barrier::new(6));
    let threads: Vec<_> = (0..6)
        .map(|_| {
            let store = store.clone();
            let message = message.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                accept(&store, &message)
            })
        })
        .collect();
    let receipts: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|r| r.accepted).count(), 1);
    assert!(
        receipts
            .iter()
            .all(|r| r.record.task.id == receipts[0].record.task.id)
    );
    struct Once(AtomicUsize);
    impl Executor for Once {
        fn execute<'a>(&'a self, execution: &'a mut Execution) -> ExecutionFuture<'a> {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                execution.update(TaskState::Completed, None, None)
            })
        }
    }
    let executor = Once(AtomicUsize::new(0));
    let id = &receipts[0].record.task.id;
    dispatch(store.clone(), id, &message.message_id, &executor)
        .await
        .unwrap();
    assert!(
        dispatch(store.clone(), id, &message.message_id, &executor)
            .await
            .is_err()
    );
    assert!(!accept(&store, &message).accepted);
    assert_eq!(executor.0.load(Ordering::SeqCst), 1);
}

#[test]
fn message_conflicts_include_lineage_but_ignore_json_object_key_order() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let mut message = input("work");
    message.metadata = Some(serde_json::from_str(r#"{"one":1,"two":{"a":1,"b":2}}"#).unwrap());
    let parent = Some(TaskRef {
        home: CALLER,
        id: "parent-task".into(),
    });
    let accepted = store
        .accept(CALLER, &target(), &message, parent.clone())
        .unwrap();
    message.metadata = Some(serde_json::from_str(r#"{"two":{"b":2,"a":1},"one":1}"#).unwrap());
    assert!(
        !store
            .accept(CALLER, &target(), &message, parent)
            .unwrap()
            .accepted
    );
    assert!(matches!(
        store.accept(CALLER, &target(), &message, None),
        Err(Error::Conflict)
    ));
    message.parts = vec![Part::text("different task")];
    assert!(matches!(
        store.accept(CALLER, &target(), &message, accepted.record.parent),
        Err(Error::Conflict)
    ));
}

#[test]
fn one_context_holds_one_execution_and_session_bindings_cannot_be_reassigned() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let one = input("one");
    let first = accept(&store, &one).record;
    let mut two = input("two");
    two.context_id = Some(first.task.context_id.clone());
    let second = accept(&store, &two).record;
    let mut running = Execution::claim(store.clone(), &first.task.id, &one.message_id).unwrap();
    running.bind_session(session()).unwrap();
    assert!(
        store
            .get(CALLER, &target(), &second.task.id)
            .unwrap()
            .revision
            > second.revision,
        "a context session binding changes every affected task's visible version"
    );
    assert!(matches!(
        Execution::claim(store.clone(), &second.task.id, &two.message_id),
        Err(Error::Busy)
    ));
    // This second task has not started, even though another task owns the context.
    assert_eq!(
        store
            .request_cancel(CALLER, &target(), &second.task.id)
            .unwrap()
            .task
            .status
            .state,
        TaskState::Canceled
    );
    assert!(
        store.recover().unwrap().is_empty(),
        "a live worker must survive reconciliation"
    );
    assert!(
        running
            .bind_session(SessionRef {
                id: SessionId("different".into()),
                ..session()
            })
            .is_err()
    );
    running.update(TaskState::Completed, None, None).unwrap();
    assert!(running.update(TaskState::Working, None, None).is_err());
    drop(running);

    let third_message = input("independent context");
    let third = accept(&store, &third_message).record;
    let mut third_run =
        Execution::claim(store.clone(), &third.task.id, &third_message.message_id).unwrap();
    assert!(matches!(
        third_run.bind_session(session()),
        Err(Error::Conflict)
    ));
    assert!(
        third_run
            .bind_session(SessionRef {
                home: CALLER,
                id: SessionId("other".into())
            })
            .is_err()
    );
}

#[tokio::test]
async fn cancellation_is_intent_until_the_executor_confirms_it() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let early_message = input("cancel before starting");
    let early = accept(&store, &early_message).record;
    let canceled = store
        .request_cancel(CALLER, &target(), &early.task.id)
        .unwrap();
    assert_eq!(canceled.task.status.state, TaskState::Canceled);
    assert_eq!(
        store
            .request_cancel(CALLER, &target(), &early.task.id)
            .unwrap(),
        canceled
    );
    assert!(Execution::claim(store.clone(), &early.task.id, &early_message.message_id).is_err());

    let message = input("cancel while running");
    let receipt = accept(&store, &message).record;
    let mut run = Execution::claim(store.clone(), &receipt.task.id, &message.message_id).unwrap();
    let pending = store
        .request_cancel(CALLER, &target(), &receipt.task.id)
        .unwrap();
    assert_eq!(pending.task.status.state, TaskState::Working);
    assert!(pending.cancellation_requested && run.cancellation_requested().unwrap());
    run.update(TaskState::Canceled, None, None).unwrap();
    assert_eq!(
        store
            .get(CALLER, &target(), &receipt.task.id)
            .unwrap()
            .task
            .status
            .state,
        TaskState::Canceled
    );
    drop(run);

    // Work may finish before its cancellation takes effect. Report that outcome.
    let race_message = input("already finishing");
    let race = accept(&store, &race_message).record;
    let mut run = Execution::claim(store.clone(), &race.task.id, &race_message.message_id).unwrap();
    store
        .request_cancel(CALLER, &target(), &race.task.id)
        .unwrap();
    run.update(
        TaskState::Completed,
        None,
        Some(artifact("actually finished")),
    )
    .unwrap();
    assert_eq!(
        store
            .get(CALLER, &target(), &race.task.id)
            .unwrap()
            .task
            .status
            .state,
        TaskState::Completed
    );
}

#[tokio::test]
async fn failed_or_unfinished_adapters_produce_failed_receipts() {
    struct Broken(bool);
    impl Executor for Broken {
        fn execute<'a>(&'a self, _execution: &'a mut Execution) -> ExecutionFuture<'a> {
            Box::pin(async move {
                if self.0 {
                    Err(Error::Adapter("fixture failure".into()))
                } else {
                    Ok(())
                }
            })
        }
    }
    let scratch = Scratch::new();
    let store = scratch.store();
    for should_error in [false, true] {
        let message = input("broken adapter");
        let receipt = accept(&store, &message).record;
        let failed = dispatch(
            store.clone(),
            &receipt.task.id,
            &message.message_id,
            &Broken(should_error),
        )
        .await
        .unwrap();
        assert_eq!(failed.task.status.state, TaskState::Failed);
        assert!(failed.task.status.message.is_some());
    }
}

#[test]
fn protocol_json_is_v1_and_preserves_structured_and_file_parts() {
    let value = json!({
        "message": { "messageId":"fixture-v1", "role":"ROLE_USER",
            "parts":[{"text":"Inspect project"},{"data":{"directory":"/work"}},
                     {"url":"https://example.invalid/input.txt","mediaType":"text/plain","filename":"input.txt"}] },
        "configuration":{"returnImmediately":true}
    });
    let request: protocol::SendMessageRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&request).unwrap(), value);
    let scratch = Scratch::new();
    let task = accept(&scratch.store(), &request.message).record.task;
    let wire = serde_json::to_value(&task).unwrap();
    assert_eq!(wire["status"]["state"], "TASK_STATE_SUBMITTED");
    assert!(wire["contextId"].is_string());
    assert!(wire["status"]["timestamp"].as_str().unwrap().ends_with('Z'));
    assert_eq!(wire["history"][0]["parts"], value["message"]["parts"]);
    assert!(wire.get("caller").is_none() && wire.get("session").is_none());
}

#[test]
fn bounds_and_corruption_refuse_without_replacing_valid_records() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let message = input("preserved");
    let receipt = accept(&store, &message).record;
    let path = scratch.0.join("a2a/tasks.json");
    let before = fs::read(&path).unwrap();
    assert!(
        store
            .accept(CALLER, &target(), &input(&"x".repeat(129 * 1024)), None)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let mut run = Execution::claim(store.clone(), &receipt.task.id, &message.message_id).unwrap();
    let before = fs::read(&path).unwrap();
    assert!(
        run.update(
            TaskState::Completed,
            None,
            Some(artifact(&"x".repeat(5 * 1024 * 1024)))
        )
        .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    run.update(TaskState::Completed, None, None).unwrap();
    drop(run);
    let saved = fs::read(&path).unwrap();
    let mut newer: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    newer["version"] = json!(2);
    fs::write(&path, serde_json::to_vec(&newer).unwrap()).unwrap();
    let newer_bytes = fs::read(&path).unwrap();
    assert!(matches!(store.recover(), Err(Error::Corrupt(_))));
    assert_eq!(fs::read(&path).unwrap(), newer_bytes);
    fs::write(&path, b"broken-json").unwrap();
    assert!(
        store
            .accept(CALLER, &target(), &input("do not replace it"), None)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), b"broken-json");
}

#[test]
fn records_are_local_private_and_bound_to_their_home() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let receipt = accept(&store, &input("work")).record;
    let wrong_home = Store::open(scratch.0.join("a2a"), CALLER).unwrap();
    assert!(matches!(wrong_home.recover(), Err(Error::Corrupt(_))));
    assert_eq!(
        store
            .list(CALLER, &target(), None, None, 1)
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&receipt)
    );
    assert!(
        store
            .list(CALLER, &target(), None, Some(&receipt.task.id), 1)
            .unwrap()
            .is_empty()
    );
    assert!(store.list(CALLER, &target(), None, None, 0).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(scratch.0.join("a2a/tasks.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(scratch.0.join("a2a"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}

#[test]
fn a_large_working_record_still_has_room_for_a_durable_crash_outcome() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let message = input("large report in progress");
    let receipt = accept(&store, &message).record;
    let mut execution =
        Execution::claim(store.clone(), &receipt.task.id, &message.message_id).unwrap();
    execution
        .update(
            TaskState::Working,
            None,
            Some(artifact(&"x".repeat(4 * 1024 * 1024 - 8192))),
        )
        .unwrap();
    drop(execution);
    let recovered = store.recover().unwrap();
    assert_eq!(recovered[0].task.status.state, TaskState::Failed);
    assert!(recovered[0].task.status.message.is_some());
    assert!(
        recovered[0].task.artifacts.is_some(),
        "recovery must preserve partial output"
    );
}

#[test]
fn an_orphaned_context_must_be_reconciled_before_another_task_starts() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let first_message = input("first worker");
    let first = accept(&store, &first_message).record;
    let mut second_message = input("second worker");
    second_message.context_id = Some(first.task.context_id.clone());
    let second = accept(&store, &second_message).record;
    let first_execution =
        Execution::claim(store.clone(), &first.task.id, &first_message.message_id).unwrap();
    drop(first_execution); // The runtime lost the worker without a terminal event.
    assert!(matches!(
        Execution::claim(store.clone(), &second.task.id, &second_message.message_id),
        Err(Error::Busy)
    ));
    assert_eq!(store.recover().unwrap()[0].task.id, first.task.id);
    let mut second_execution =
        Execution::claim(store.clone(), &second.task.id, &second_message.message_id).unwrap();
    second_execution
        .update(TaskState::Completed, None, None)
        .unwrap();
    assert_eq!(
        store
            .get(CALLER, &target(), &first.task.id)
            .unwrap()
            .task
            .status
            .state,
        TaskState::Failed
    );
}

#[test]
fn protocol_errors_keep_their_meaning_and_hide_storage_details() {
    assert_eq!(
        Error::NotFound("missing".into()).protocol_error().code,
        protocol::error_code::TASK_NOT_FOUND
    );
    assert_eq!(
        Error::NotCancelable("done".into()).protocol_error().code,
        protocol::error_code::TASK_NOT_CANCELABLE
    );
    assert_eq!(
        Error::Conflict.protocol_error().code,
        protocol::error_code::INVALID_PARAMS
    );
    let error = Error::Corrupt("private-path-and-diagnostic".into()).protocol_error();
    assert_eq!(error.code, protocol::error_code::INTERNAL_ERROR);
    assert!(!error.message.contains("private-path"));
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_crashed_process_is_failed_without_replaying_and_a_live_lease_is_respected() {
    let scratch = Scratch::new();
    let store = scratch.store();
    let message = input("do not replay after a crash");
    let receipt = accept(&store, &message).record;
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "child_executor", "--nocapture"])
            .env("KHOR_A2A_TEST_ROOT", &scratch.0)
            .env("KHOR_A2A_TEST_TASK", &receipt.task.id)
            .env("KHOR_A2A_TEST_MESSAGE", &message.message_id)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !scratch.0.join("ready").exists() {
        assert!(Instant::now() < deadline, "child failed to claim execution");
        assert!(child.0.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(store.recover().unwrap().is_empty());
    assert!(matches!(
        Execution::claim(store.clone(), &receipt.task.id, &message.message_id),
        Err(Error::Busy)
    ));
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let restarted = scratch.store();
    let recovered = restarted.recover().unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].task.status.state, TaskState::Failed);
    assert_eq!(recovered[0].context_session, Some(session()));
    assert!(!accept(&restarted, &message).accepted);
    assert!(restarted.recover().unwrap().is_empty());
    assert!(Execution::claim(restarted, &receipt.task.id, &message.message_id).is_err());
}

#[test]
#[ignore = "subprocess fixture, invoked only by the crash test"]
fn child_executor() {
    let root = PathBuf::from(std::env::var_os("KHOR_A2A_TEST_ROOT").expect("fixture root"));
    let store = Store::open(root.join("a2a"), HOME).unwrap();
    let mut execution = Execution::claim(
        store,
        &std::env::var("KHOR_A2A_TEST_TASK").unwrap(),
        &std::env::var("KHOR_A2A_TEST_MESSAGE").unwrap(),
    )
    .unwrap();
    execution.bind_session(session()).unwrap();
    fs::write(root.join("ready"), b"claimed").unwrap();
    loop {
        std::thread::park_timeout(Duration::from_secs(1));
    }
}
