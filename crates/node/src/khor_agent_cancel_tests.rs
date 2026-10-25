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
fn an_uncooperative_sidecar_has_a_bounded_stop_and_releases_the_next_prompt() {
    run_fixture(
        "abort-ignored",
        r#"#!/bin/sh
read -r start
printf '%s' "$$" >"$0.pid"
printf '%s\n' '{"type":"text","delta":"ready"}'
printf '%s' ready >"$0.ready"
while IFS= read -r message; do
 case "$message" in *abort*) printf '%s' received >"$0.abort";; esac
done
"#,
    );
}

#[test]
fn a_sidecar_that_never_reads_stdin_cannot_block_stop() {
    run_fixture(
        "blocked-input",
        r#"#!/bin/sh
printf '%s' "$$" >"$0.pid"
printf '%s' ready >"$0.ready"
ticks=0
while [ "$ticks" -lt 25 ]; do /bin/sleep 1; ticks=$((ticks + 1)); done
"#,
    );
}

fn run_fixture(scenario: &str, script: &str) {
    let scratch = Scratch(
        std::env::temp_dir().join(format!("khor-cancel-{}", crate::link::fresh_hex().unwrap())),
    );
    fs::create_dir(&scratch.0).unwrap();
    let sidecar = scratch.0.join("sidecar");
    fs::write(&sidecar, script).unwrap();
    fs::set_permissions(&sidecar, fs::Permissions::from_mode(0o700)).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "khor_agent::cancel_tests::uncooperative_stop_child",
                "--ignored",
            ])
            .env("KHOR_AGENT_SIDECAR", &sidecar)
            .env("KHOR_CANCEL_SCENARIO", scenario)
            .env("KHOR_CANCEL_TEST_ROOT", &scratch.0)
            .env("KHOR_PATH", "/usr/bin:/bin")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(25);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "stop worker failed to finish");
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
}

#[test]
#[ignore = "isolated Prompt/Stop home-handler regression worker"]
fn uncooperative_stop_child() {
    let blocked = std::env::var("KHOR_CANCEL_SCENARIO").unwrap() == "blocked-input";
    let scratch = PathBuf::from(std::env::var_os("KHOR_CANCEL_TEST_ROOT").unwrap());
    let root = scratch.join("store");
    let node = Node::open_as(root.clone(), "cancel-fixture").unwrap();
    let root = node.root().to_path_buf();
    let account = node
        .register_account(
            "fixture",
            crate::Provider::Newapi,
            crate::AccountKind::Bill,
            Some("http://127.0.0.1:1"),
            "fixture-key",
            None,
        )
        .unwrap();
    node.khor_agent_here(Action::Configure {
        account: account.id,
        model: "fixture".into(),
    })
    .unwrap();
    let run = node
        .khor_agent_here(Action::Prompt {
            token: "cancel-first-fixture-token".into(),
            text: if blocked {
                "x".repeat(128 * 1024)
            } else {
                "fixture".into()
            },
        })
        .unwrap()
        .run
        .unwrap();
    let ready = scratch.join("sidecar.ready");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "the fake sidecar did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    let pid: libc::pid_t = fs::read_to_string(scratch.join("sidecar.pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(pid > 0);
    let stopped = Instant::now();
    node.khor_agent_here(Action::Stop { run: run.clone() })
        .unwrap();
    assert!(
        stopped.elapsed() < Duration::from_secs(1),
        "Stop blocked behind stdin"
    );
    let abort = scratch.join("sidecar.abort");
    let deadline = Instant::now() + Duration::from_secs(2);
    while !blocked && !abort.exists() {
        assert!(
            Instant::now() < deadline,
            "the sidecar did not receive the real abort message"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    loop {
        let receipt: Saved = serde_json::from_slice(&fs::read(file(&root, &run)).unwrap()).unwrap();
        if receipt.record.status == "cancelled" {
            assert_eq!(
                unsafe { libc::kill(pid, 0) },
                -1,
                "cancelled published before child exit"
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
            if !running().lock().unwrap().contains_key(&root) {
                break;
            }
        }
        assert!(
            stopped.elapsed() < Duration::from_secs(15),
            "abort was received but the executor did not release: {}",
            receipt.record.status
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // Signal zero only probes existence. A zombie still owns its PID, so
    // this also rejects declaring cancellation before the child is reaped.
    let exists = unsafe { libc::kill(pid, 0) };
    assert!(
        exists == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH),
        "cancelled was published before the sidecar was reaped"
    );
    // The reading fixture exits on worker EOF; the blocked fixture has a
    // 25-second self-exit fuse. Neither failure cleanup targets reused PIDs.
    // A second prompt proves the executor claim was actually released.
    fs::write(
        scratch.join("sidecar"),
        "#!/bin/sh\nread -r start\nprintf '%s\\n' '{\"type\":\"done\"}'\n",
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let second = loop {
        match node.khor_agent_here(Action::Prompt {
            token: "after-stop-fixture-token".into(),
            text: "next fixture".into(),
        }) {
            Ok(report) => break report.run.unwrap(),
            Err(error) if error == msg::agent_busy() => {
                assert!(
                    Instant::now() < deadline,
                    "the executor claim was not released"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("the next prompt failed: {error}"),
        }
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let receipt: Saved =
            serde_json::from_slice(&fs::read(file(&root, &second)).unwrap()).unwrap();
        if receipt.record.status == "done" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the next prompt could not finish: {} {:?}",
            receipt.record.status,
            receipt.record.error
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
