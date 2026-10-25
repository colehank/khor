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
    let _node = Node::open_as(root.clone(), "output-fixture").unwrap();
    let _claim = claim(&root).unwrap();
    let active = Arc::new(Running {
        id: "agent-output-fixture".into(),
        input: Mutex::new(None),
        stopped: AtomicBool::new(false),
        approval: Mutex::new(None),
    });
    let saved = Saved {
        fingerprint: "fixture".into(),
        record: CommandRecord {
            id: active.id.clone(),
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
    execute(
        root.clone(),
        saved,
        active.clone(),
        json!({"type":"start"}),
        &key,
    );
    let receipt: Saved =
        serde_json::from_slice(&fs::read(file(&root, &active.id)).unwrap()).unwrap();
    assert_eq!(receipt.record.status, "failed");
    assert_eq!(receipt.record.error.as_deref(), Some(expected.as_str()));
    assert_eq!(receipt.record.error.unwrap().len(), expected.len());
}
