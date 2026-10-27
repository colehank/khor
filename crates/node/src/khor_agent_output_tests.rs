//! Isolated fake sidecar exercises the real failed-event and saved-receipt path.
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

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

fn drain_cancellation_owners(root: &std::path::Path) {
    // Terminal fixture scopes must outlive their finite cancellation owners.
    let by = Instant::now() + Duration::from_secs(6);
    let cleanup = loop {
        let result = crate::remote_cancel::release_test_root(root);
        if result.is_ok() || Instant::now() >= by {
            break result;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(cleanup.is_ok(), "owned cancellation cleanup: {cleanup:?}");
}

#[test]
fn failed_sidecar_receipts_redact_once_and_leave_empty_credentials_readable() {
    for (key, error, expected) in [
        (
            "",
            "OAuth 网络失败：请稍后重试🔑",
            "OAuth 网络失败：请稍后重试🔑",
        ),
        (
            "fixture-api-key",
            "fixture-api-key / fixture-api-key：网络失败",
            "[redacted] / [redacted]：网络失败",
        ),
        (
            "测试密钥🔑",
            "凭证测试密钥🔑失败；重试测试密钥🔑",
            "凭证[redacted]失败；重试[redacted]",
        ),
        // This fixture catches applying redaction to an already redacted error.
        ("redact", "failed: redact", "failed: [redacted]"),
    ] {
        let scratch = Scratch(std::env::temp_dir().join(format!(
            "khor-output-test-{}",
            crate::link::fresh_hex().unwrap()
        )));
        fs::create_dir(&scratch.0).unwrap();
        let sidecar = scratch.0.join("sidecar");
        let event = json!({"type":"failed","error":error}).to_string();
        // Fixtures contain no apostrophes; no shell interpolation of secrets.
        fs::write(
            &sidecar,
            format!("#!/bin/sh\nread -r start\nprintf '%s\\n' '{event}'\n"),
        )
        .unwrap();
        fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o700)).unwrap();
        let mut child = Worker(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "khor_agent::output_tests::failed_sidecar_child",
                    "--ignored",
                ])
                .env("KHOR_AGENT_SIDECAR", &sidecar)
                .env("KHOR_OUTPUT_TEST_ROOT", scratch.0.join("store"))
                .env("KHOR_OUTPUT_TEST_KEY", key)
                .env("KHOR_OUTPUT_TEST_EXPECTED", expected)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "fake sidecar test timed out");
            std::thread::sleep(Duration::from_millis(10));
        };
        use std::io::Read;
        let mut stdout = String::new();
        let mut stderr = String::new();
        child
            .0
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        child
            .0
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(status.success(), "key fixture {key:?}: {stdout}\n{stderr}");
    }
}

#[test]
#[ignore = "isolated child of failed-sidecar regression"]
fn failed_sidecar_child() {
    let root = PathBuf::from(std::env::var_os("KHOR_OUTPUT_TEST_ROOT").unwrap());
    let key = std::env::var("KHOR_OUTPUT_TEST_KEY").unwrap();
    let expected = std::env::var("KHOR_OUTPUT_TEST_EXPECTED").unwrap();
    let node = Node::open_as(root.clone(), "output-fixture").unwrap();
    let _claim = claim(&root).unwrap();
    let active = Arc::new(Running {
        id: "agent-output-fixture".into(),
        input: Mutex::new(None),
        process: Arc::default(),
        worker: Mutex::new(None),
        path: OnceLock::new(),
        stopped: AtomicBool::new(false),
        approval: Mutex::new(None),
    });
    let saved = Saved {
        fingerprint: "fixture".into(),
        record: CommandRecord {
            id: active.id.clone(),
            owner: node.device_str().into(),
            started_at_ms: now(),
            data_json: Some(json!({"existingFixtureMetadata":{"token":"kept"}}).to_string()),
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
    execute(
        root.clone(),
        saved,
        active.clone(),
        json!({"type":"start"}),
        &key,
    );
    drain_cancellation_owners(&root);
    let receipt: Saved =
        serde_json::from_slice(&fs::read(file(&root, &active.id)).unwrap()).unwrap();
    assert_eq!(receipt.record.status, "failed");
    let data: Value = serde_json::from_str(receipt.record.data_json.as_deref().unwrap()).unwrap();
    assert_eq!(data["existingFixtureMetadata"]["token"], "kept");
    // Cancellation truth lives in private receipts, never in competing Saved
    // state. Absence here is not a claim about remote confirmation.
    assert!(data.get("remoteCancellation").is_none());
    assert_eq!(receipt.record.error.as_deref(), Some(expected.as_str()));
    assert_eq!(receipt.record.error.unwrap().len(), expected.len());
}

#[test]
fn oversized_sidecar_events_are_rejected_before_the_writer_finishes() {
    for newline in [false, true] {
        let scratch = Scratch(std::env::temp_dir().join(format!(
            "khor-pipe-limit-{}",
            crate::link::fresh_hex().unwrap()
        )));
        fs::create_dir(&scratch.0).unwrap();
        let sidecar = scratch.0.join("sidecar");
        let finished = scratch.0.join("writer-finished");
        let script = format!(
            "#!/bin/sh\nset -e\nread -r start\nchunk={}\ni=0\nwhile [ \"$i\" -lt 1024 ]; do\n printf '%s' \"$chunk\"\n i=$((i+1))\ndone\n{}printf '%s' reached >{}\n",
            shell_literal(&"x".repeat(8192)),
            if newline { "printf '\\n'\n" } else { "" },
            shell_literal(finished.to_str().unwrap()),
        );
        fs::write(&sidecar, script).unwrap();
        fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o700)).unwrap();
        let mut worker = Worker(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "khor_agent::output_tests::failed_sidecar_child",
                    "--ignored",
                ])
                .env("KHOR_AGENT_SIDECAR", &sidecar)
                .env("KHOR_OUTPUT_TEST_ROOT", scratch.0.join("store"))
                .env("KHOR_OUTPUT_TEST_KEY", "")
                .env("KHOR_OUTPUT_TEST_EXPECTED", msg::agent_arguments())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = worker.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "oversized pipe fixture timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        use std::io::Read;
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
        assert!(status.success(), "{stdout}\n{stderr}");
        assert!(
            !finished.exists(),
            "the executor consumed the entire oversized event"
        );
    }
}

fn streamed_case(name: &str) -> Value {
    let (key, channel, ending, chunks, checkpoint) = match name {
        "stdout-done" => (
            "fixture-split-key",
            "stdout",
            "done",
            vec![
                "hello fixture-".into(),
                "split-key / fixture-split-key!".into(),
            ],
            false,
        ),
        "stdout-failed" => (
            "redact",
            "stdout",
            "failed",
            vec!["hello red".into(), "act / red".into()],
            false,
        ),
        "message-done" => (
            "凭证🔑",
            "message",
            "done",
            vec!["hello 凭".into(), "证🔑 / 凭".into()],
            false,
        ),
        "message-cancelled" => (
            "凭证🔑",
            "message",
            "cancelled",
            vec!["hello 凭".into()],
            true,
        ),
        "thought-eof" => (
            "ababa",
            "thought",
            "eof",
            vec!["hello ab".into(), "abababa / ab".into()],
            false,
        ),
        "compaction-done" => (
            "fixture-key",
            "compaction",
            "done",
            vec!["fixture-".into(), "key / fixture-k".into()],
            false,
        ),
        "cap-done" => (
            "fixture-split-key",
            "stdout",
            "done",
            vec![
                "界".repeat(128 * 1024 / 3),
                "fixture-".into(),
                "split-key ending".into(),
            ],
            false,
        ),
        "checkpoint" => (
            "fixture-split-key",
            "message",
            "done",
            vec!["hello fixture-".into(), "split-key!".into()],
            true,
        ),
        _ => panic!("unknown stream fixture"),
    };
    json!({"name":name,"key":key,"channel":channel,"ending":ending,"chunks":chunks,"checkpoint":checkpoint})
}

fn shell_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn stream_sidecar(case: &Value, directory: &Path) -> PathBuf {
    let path = directory.join("sidecar");
    let key = case["key"].as_str().unwrap();
    let split = key.char_indices().nth(1).map_or(key.len(), |(at, _)| at);
    let mut script = format!(
        "#!/bin/sh\nread -r start\nprintf '%s' {} >&2\n",
        shell_literal(&key[..split])
    );
    let emit = |script: &mut String, event: Value| {
        script.push_str(&format!(
            "printf '%s\\n' {}\n",
            shell_literal(&event.to_string())
        ));
    };
    if case["channel"] == "compaction" {
        emit(
            &mut script,
            json!({"type":"update","update":{"sessionUpdate":"compaction_update","compactionId":"c","status":"in_progress"}}),
        );
    }
    if case["checkpoint"] == true {
        script.push_str("sleep 0.25\n");
    }
    for (index, chunk) in case["chunks"].as_array().unwrap().iter().enumerate() {
        let event = match case["channel"].as_str().unwrap() {
            "stdout" => json!({"type":"text","delta":chunk}),
            "message" => {
                json!({"type":"update","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":chunk}}})
            }
            "thought" => {
                json!({"type":"update","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":chunk}}})
            }
            "compaction" => {
                json!({"type":"update","update":{"sessionUpdate":"compaction_summary_chunk","compactionId":"c","content":{"type":"text","text":chunk}}})
            }
            _ => unreachable!(),
        };
        emit(&mut script, event);
        if index == 0 && case["checkpoint"] == true {
            let first = shell_literal(directory.join("first-sent").to_str().unwrap());
            let gate = shell_literal(directory.join("continue").to_str().unwrap());
            script.push_str(&format!("touch {first}\ni=0\nwhile [ ! -f {gate} ]; do\n i=$((i+1))\n [ \"$i\" -lt 1000 ] || exit 1\n sleep 0.01\ndone\n"));
        }
        emit(
            &mut script,
            json!({"type":"settings","settings":{"configOptions":[]}}),
        );
    }
    script.push_str(&format!(
        "printf '%s' {} >&2\n",
        shell_literal(&key[split..])
    ));
    if case["channel"] == "compaction" {
        emit(
            &mut script,
            json!({"type":"update","update":{"sessionUpdate":"compaction_update","compactionId":"c","status":"completed"}}),
        );
    }
    match case["ending"].as_str().unwrap() {
        "failed" => emit(
            &mut script,
            json!({"type":"failed","error":format!("failed: {key}")}),
        ),
        "eof" => {}
        _ => emit(&mut script, json!({"type":"done"})),
    }
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
fn streamed_receipts_withhold_matching_tails_until_safe_and_flush_all_exit_paths() {
    for name in [
        "stdout-done",
        "stdout-failed",
        "message-done",
        "message-cancelled",
        "thought-eof",
        "compaction-done",
        "cap-done",
        "checkpoint",
    ] {
        let scratch = Scratch(std::env::temp_dir().join(format!(
            "khor-stream-test-{}",
            crate::link::fresh_hex().unwrap()
        )));
        fs::create_dir(&scratch.0).unwrap();
        let case = streamed_case(name);
        let sidecar = stream_sidecar(&case, &scratch.0);
        let fixture = scratch.0.join("case.json");
        fs::write(&fixture, case.to_string()).unwrap();
        let mut worker = Worker(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "khor_agent::output_tests::streamed_sidecar_child",
                    "--ignored",
                ])
                .env("KHOR_AGENT_SIDECAR", &sidecar)
                .env("KHOR_STREAM_TEST_CASE", &fixture)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = worker.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "stream fixture {name} timed out");
            std::thread::sleep(Duration::from_millis(10));
        };
        use std::io::Read;
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
        assert!(
            status.success(),
            "stream fixture {name}: {stdout}\n{stderr}"
        );
    }
}

#[test]
#[ignore = "isolated child of streaming-sidecar regression"]
fn streamed_sidecar_child() {
    let fixture = PathBuf::from(std::env::var_os("KHOR_STREAM_TEST_CASE").unwrap());
    let directory = fixture.parent().unwrap();
    let case: Value = serde_json::from_slice(&fs::read(&fixture).unwrap()).unwrap();
    let key = case["key"].as_str().unwrap().to_owned();
    let root = directory.join("store");
    let node = Node::open_as(root.clone(), "stream-fixture").unwrap();
    let _claim = claim(&root).unwrap();
    let active = Arc::new(Running {
        id: "agent-stream-fixture".into(),
        input: Mutex::new(None),
        process: Arc::default(),
        worker: Mutex::new(None),
        path: OnceLock::new(),
        stopped: AtomicBool::new(false),
        approval: Mutex::new(None),
    });
    let saved = Saved {
        fingerprint: "stream-fixture".into(),
        record: CommandRecord {
            id: active.id.clone(),
            owner: node.device_str().into(),
            started_at_ms: now(),
            status: "running".into(),
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
    let execution_root = root.clone();
    let execution_active = active.clone();
    let execution_key = key.clone();
    let execution = std::thread::spawn(move || {
        execute(
            execution_root,
            saved,
            execution_active,
            json!({"type":"start"}),
            &execution_key,
        )
    });
    let checkpoint = if case["checkpoint"] == true {
        let deadline = Instant::now() + Duration::from_secs(5);
        let receipt = loop {
            if directory.join("first-sent").exists() {
                if let Ok(bytes) = fs::read(file(&root, &active.id)) {
                    if let Ok(receipt) = serde_json::from_slice::<Saved>(&bytes) {
                        break Some(receipt);
                    }
                }
            }
            if Instant::now() >= deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if case["ending"] == "cancelled" {
            active.request_stop();
        }
        fs::write(directory.join("continue"), b"").unwrap();
        receipt
    } else {
        None
    };
    execution.join().unwrap();
    drain_cancellation_owners(&root);
    if case["checkpoint"] == true {
        let checkpoint = checkpoint.expect("sidecar must persist a live snapshot before release");
        assert_eq!(
            checkpoint.record.stdout, "hello ",
            "raw key prefixes must stay out of the live receipt"
        );
        let updates = &checkpoint.record.agent.unwrap().updates;
        let live_text: String = updates
            .iter()
            .filter_map(|s| serde_json::from_str::<Value>(s).ok())
            .filter(|v| v["sessionUpdate"] == "agent_message_chunk")
            .filter_map(|v| v["content"]["text"].as_str().map(str::to_owned))
            .collect();
        assert_eq!(live_text, "hello ");
    }
    let receipt: Saved =
        serde_json::from_slice(&fs::read(file(&root, &active.id)).unwrap()).unwrap();
    let raw: String = case["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let mut display = raw.replace(&key, "[redacted]");
    let full_display = display.clone();
    if display.len() > 128 * 1024 {
        let mut cut = 128 * 1024;
        while !display.is_char_boundary(cut) {
            cut -= 1;
        }
        display.truncate(cut);
        display.push('\n');
        display.push_str(msg::console_output_cut());
    }
    let expected_stdout = if matches!(case["channel"].as_str(), Some("stdout" | "message")) {
        display.as_str()
    } else {
        ""
    };
    assert!(
        receipt.record.stdout == expected_stdout,
        "stdout differs for {} (actual {} bytes, expected {})",
        case["name"],
        receipt.record.stdout.len(),
        expected_stdout.len()
    );
    let ending = case["ending"].as_str().unwrap();
    assert_eq!(
        receipt.record.status,
        match ending {
            "cancelled" => "cancelled",
            "failed" | "eof" => "failed",
            _ => "done",
        }
    );
    if ending == "failed" {
        assert_eq!(receipt.record.error.as_deref(), Some("failed: [redacted]"));
    } else if ending == "eof" {
        assert_eq!(receipt.record.error.as_deref(), Some(msg::agent_failed()));
    } else {
        assert!(receipt.record.error.is_none());
    }
    assert_eq!(
        receipt.record.stderr,
        if ending == "cancelled" {
            msg::agent_stopped()
        } else {
            ""
        },
        "sidecar stderr remains drained, never durable output"
    );
    let updates: Vec<Value> = receipt
        .record
        .agent
        .unwrap()
        .updates
        .iter()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let snapshot_text: String = match case["channel"].as_str().unwrap() {
        "message" | "thought" => {
            let kind = if case["channel"] == "message" {
                "agent_message_chunk"
            } else {
                "agent_thought_chunk"
            };
            updates
                .iter()
                .filter(|v| v["sessionUpdate"] == kind)
                .filter_map(|v| v["content"]["text"].as_str())
                .collect()
        }
        "compaction" => updates
            .iter()
            .filter(|v| v["compactionId"] == "c")
            .filter_map(|v| v["summary"].as_array())
            .flatten()
            .filter_map(|v| v["text"].as_str())
            .collect(),
        _ => String::new(),
    };
    if case["channel"] != "stdout" {
        assert_eq!(snapshot_text, full_display);
    }
}
