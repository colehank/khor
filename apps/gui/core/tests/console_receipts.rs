//! Independent regressions for console receipt ownership and replay.
use khor_gui_core::console;
use khor_node::{CommandRecord, Node};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "khor-receipt-review-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(p.join(".khor")).unwrap();
        fs::write(
            p.join(".khor/relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn record(node: &Node, token: &str) -> CommandRecord {
    CommandRecord {
        id: format!("cmd-{}-{token}", &node.device_str()[..16]),
        line: "/version".into(),
        owner: node.device_str().into(),
        owner_name: node.name().into(),
        status: "running".into(),
        started_at_ms: 1,
        ended_at_ms: None,
        stdout: String::new(),
        stderr: String::new(),
        data_json: None,
        session: None,
        error: None,
    }
}

#[test]
fn concurrent_delivery_has_one_crdt_receipt_per_submission() {
    let scratch = Scratch::new("concurrent");
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let receipt = record(&node, "concurrent-submission-123456");
    let barrier = Arc::new(Barrier::new(16));
    let workers: Vec<_> = (0..16)
        .map(|_| {
            let root = scratch.0.clone();
            let receipt = receipt.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let node = Node::open(root).unwrap();
                barrier.wait();
                node.record_command(&receipt).unwrap();
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let log = node.log(khor_node::wide::BROADCAST).unwrap();
    assert_eq!(
        log.messages.iter().filter(|m| m.id == receipt.id).count(),
        1,
        "concurrent delivery must update one receipt, never append duplicate CRDT list rows"
    );
}

#[test]
fn a_reused_current_pid_does_not_keep_an_old_executor_running() {
    let scratch = Scratch::new("pid-reused");
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let receipt = record(&node, "pid-reused-submission-123456");
    fs::create_dir_all(scratch.0.join(".khor/console")).unwrap();
    let stored = serde_json::json!({
        "fingerprint": "fixture", "pid": std::process::id(),
        "process_started": 1, "private": false, "record": receipt,
    });
    fs::write(
        scratch
            .0
            .join(".khor/console")
            .join(format!("{}.json", receipt.id)),
        serde_json::to_vec(&stored).unwrap(),
    )
    .unwrap();
    node.record_command(&receipt).unwrap();
    let mut log = node.log(khor_node::wide::BROADCAST).unwrap();
    console::reconcile(&scratch.0, &node, &mut log.messages);
    let actual = log
        .messages
        .iter()
        .find(|m| m.id == receipt.id)
        .unwrap()
        .command
        .as_ref()
        .unwrap();
    assert_eq!(
        actual.status, "interrupted",
        "PID equality is insufficient: the process creation time belongs to a different executor"
    );
}

#[test]
fn stale_running_delivery_cannot_replace_a_terminal_receipt() {
    let scratch = Scratch::new("stale-after-done");
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let stale = record(&node, "stale-delivery-submission-123456");
    node.record_command(&stale).unwrap();
    let mut done = stale.clone();
    done.status = "done".into();
    done.ended_at_ms = Some(done.started_at_ms + 1);
    done.stdout = "completed exactly once".into();
    node.record_command(&done).unwrap();
    node.record_command(&stale).unwrap();
    let log = node.log(khor_node::wide::BROADCAST).unwrap();
    let records: Vec<_> = log.messages.iter().filter(|m| m.id == done.id).collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].command.as_ref(), Some(&done));
}

#[test]
fn concurrent_submit_retries_execute_a_non_idempotent_write_once() {
    let scratch = Scratch::new("run-retries");
    let file = scratch.0.join("only-once.txt");
    let line = format!("/touch '{}'", file.display());
    // Bootstrap once so the test concerns submissions, not device creation.
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let root = scratch.0.clone();
            let line = line.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                console::run(&root, &line, "run-retry-submission-123456", None).unwrap()
            })
        })
        .collect();
    let ids: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert!(ids.iter().all(|id| id == &ids[0]));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let log = node.log(khor_node::wide::BROADCAST).unwrap();
        let records: Vec<_> = log.messages.iter().filter(|m| m.id == ids[0]).collect();
        assert_eq!(records.len(), 1);
        let record = records[0].command.as_ref().unwrap();
        if record.status != "running" {
            assert_eq!(
                record.status, "done",
                "touch refuses a second execution: {:?}",
                record.error
            );
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "submission must complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(file.is_file());
    let before = fs::metadata(&file).unwrap().modified().unwrap();
    assert_eq!(
        console::run(&scratch.0, &line, "run-retry-submission-123456", None).unwrap(),
        ids[0]
    );
    assert_eq!(fs::metadata(file).unwrap().modified().unwrap(), before);
}

#[test]
fn a_failed_seen_ack_cannot_strand_a_running_submission() {
    let scratch = Scratch::new("seen-failure");
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let seen = scratch.0.join(".khor/seen");
    if seen.is_dir() {
        fs::remove_dir_all(&seen).unwrap();
    }
    fs::write(&seen, "not a directory").unwrap();
    let token = "seen-failure-submission-123456";
    let id = format!("cmd-{}-{token}", &node.device_str()[..16]);
    let file = scratch.0.join("accepted-write.txt");
    let line = format!("/touch '{}'", file.display());
    let answer = console::run(&scratch.0, &line, token, None);
    let claim = scratch.0.join(".khor/console").join(format!("{id}.json"));
    if answer.is_err() {
        let stored: serde_json::Value = serde_json::from_slice(&fs::read(claim).unwrap()).unwrap();
        assert_ne!(
            stored["record"]["status"], "running",
            "a failed pre-execution step must finish its durable receipt before returning an error"
        );
    } else {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let stored: serde_json::Value =
                serde_json::from_slice(&fs::read(&claim).unwrap()).unwrap();
            if stored["record"]["status"] != "running" {
                assert_eq!(stored["record"]["status"], "done");
                assert!(file.is_file());
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

#[test]
fn reading_an_unclaimed_synced_receipt_does_not_execute_its_line() {
    let scratch = Scratch::new("no-replay");
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let target = scratch.0.join("must-never-exist.txt");
    let mut receipt = record(&node, "unclaimed-receipt-123456");
    receipt.line = format!("/touch '{}'", target.display());
    node.record_command(&receipt).unwrap();
    let log = khor_gui_core::machine_log(&scratch.0, khor_node::wide::BROADCAST).unwrap();
    assert!(log.messages.iter().any(|m| m.id == receipt.id));
    assert!(!target.exists());
    assert!(
        !scratch.0.join(".khor/console").exists(),
        "reading an imported receipt must not create an execution claim"
    );
}

#[test]
fn private_inputs_and_output_do_not_enter_replicated_receipts() {
    let scratch = Scratch::new("private-wire");
    let node = Node::open_as(scratch.0.clone(), "receipt-review").unwrap();
    let id = console::run(
        &scratch.0,
        "/version",
        "private-wire-submission-123456",
        Some("fake-secret-input-123456"),
    )
    .unwrap();
    let path = scratch.0.join(".khor/console").join(format!("{id}.json"));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let stored: serde_json::Value = loop {
        let stored: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if stored["record"]["status"] != "running" {
            break stored;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert_eq!(stored["record"]["status"], "done");
    assert!(
        !fs::read_to_string(&path)
            .unwrap()
            .contains("fake-secret-input-123456")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let log = node.log(khor_node::wide::BROADCAST).unwrap();
    let wire = log.messages.iter().find(|m| m.id == id).unwrap();
    let public = wire.command.as_ref().unwrap();
    assert_eq!(public.line, "/version …");
    assert_eq!(wire.body, khor_node::MsgBody::Text("/version …".into()));
    assert_ne!(public.stdout, stored["record"]["stdout"].as_str().unwrap());
    assert!(public.stderr.is_empty());
    assert!(public.data_json.is_none());
    assert!(
        !serde_json::to_string(public)
            .unwrap()
            .contains("fake-secret-input-123456")
    );
}
