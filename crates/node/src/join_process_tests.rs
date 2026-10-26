//! Real local subprocess regressions for the actual join::run wrapper.

use super::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Child};

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

fn exercise(mode: &str) {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-join-process-{}",
        crate::link::fresh_hex().unwrap()
    )));
    fs::create_dir(&scratch.0).unwrap();
    let fixture = scratch.0.join("fixture.py");
    fs::write(&fixture, include_str!("fixtures/join-process.py")).unwrap();
    let adapter = scratch.0.join("ssh");
    fs::write(
        &adapter,
        "#!/bin/sh\nexec python3 -I -S \"$Q045_FIXTURE\"\n",
    )
    .unwrap();
    fs::set_permissions(&adapter, fs::Permissions::from_mode(0o700)).unwrap();
    let mut paths = vec![scratch.0.clone()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let output = fs::File::create(scratch.0.join("worker.log")).unwrap();
    let (_, module) = module_path!().split_once("::").unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("{module}::owned_ssh_worker"),
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("Q045_MODE", mode)
            .env("Q045_FIXTURE", fixture)
            .env("Q045_FIXTURE_PID", scratch.0.join("pid"))
            .current_dir(&scratch.0)
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(6);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "owned SSH worker exceeded outer watchdog: {}",
            fs::read_to_string(scratch.0.join("worker.log")).unwrap_or_default()
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    // Even a worker panic before run cleanup must allow the fixture's independent
    // three-second alarm to retire it before the parent returns. Never signal a
    // PID read from a stale file after the owning worker has been reaped.
    if let Ok(pid) = fs::read_to_string(scratch.0.join("pid")) {
        let pid: i32 = pid.parse().unwrap();
        let cleanup_deadline = Instant::now() + Duration::from_secs(4);
        loop {
            let exists = unsafe { libc::kill(pid, 0) };
            if exists == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                break;
            }
            assert!(
                Instant::now() < cleanup_deadline,
                "owned fixture {pid} outlived its alarm"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let worker_log = fs::read_to_string(scratch.0.join("worker.log")).unwrap();
    println!("owned SSH mode={mode}, status={status}:\n{worker_log}");
    assert!(status.success(), "owned SSH {mode} failed");
}

#[test]
fn ssh_deadline_includes_blocked_input() {
    exercise("blocked-input");
}

#[test]
fn ssh_drains_both_outputs_before_waiting_for_exit() {
    exercise("dual-output");
}

#[test]
fn ssh_normal_input_and_diagnostic_are_preserved() {
    exercise("normal");
}

#[test]
fn ssh_stdout_budget_is_an_error_and_reaps_the_child() {
    exercise("stdout-limit");
}
#[test]
fn ssh_stderr_keeps_a_bounded_marked_tail() {
    exercise("stderr-tail");
}
#[test]
fn ssh_continuous_stderr_obeys_the_original_deadline() {
    exercise("stderr-flood");
}
#[test]
fn ssh_refusal_preserves_diagnostic_after_broken_input() {
    exercise("early-refusal");
}
#[test]
fn ssh_success_with_incomplete_input_is_an_error() {
    exercise("early-success");
}
#[test]
fn ssh_closed_pipes_do_not_substitute_for_process_exit() {
    exercise("closed-io");
}
#[test]
fn ssh_private_cancellation_joins_io_and_reaps() {
    exercise("cancel");
}
#[test]
fn ssh_timeout_leaves_an_existing_independent_process_alive() {
    exercise("existing-master");
}

#[test]
fn finite_ssh_flags_keep_the_road_and_do_not_create_a_master() {
    let to = Dest::new("host-alias", Some("user@jump-alias"))
        .on_port(Some(2222))
        .with_identity(Some("/owned/key with spaces"))
        .with_password(true);
    let original = to.road();
    let command = to.finite_ssh_command();
    let args: Vec<_> = command
        .get_args()
        .map(|value| value.to_str().unwrap())
        .collect();
    for option in [
        "IgnoreUnknown=ForkAfterAuthentication",
        "ControlMaster=no",
        "ControlPersist=no",
        "ForkAfterAuthentication=no",
    ] {
        assert!(args.contains(&option));
        let proxy = args
            .iter()
            .find(|arg| arg.starts_with("ProxyCommand="))
            .unwrap();
        assert!(proxy.contains(&format!("-o {option}")));
        assert!(!original.iter().any(|arg| arg.contains(option)));
    }
    for option in [
        "Port=2222",
        "/owned/key with spaces",
        "IdentitiesOnly=yes",
        "BatchMode=no",
    ] {
        assert!(args.contains(&option));
    }
    assert!(!args.iter().any(|arg| arg.starts_with("ControlPath=")));
    let ignore = args
        .iter()
        .position(|arg| *arg == "IgnoreUnknown=ForkAfterAuthentication")
        .unwrap();
    let fork = args
        .iter()
        .position(|arg| *arg == "ForkAfterAuthentication=no")
        .unwrap();
    assert!(
        ignore < fork,
        "the exception must precede the otherwise unknown option"
    );
    let proxy = args
        .iter()
        .find(|arg| arg.starts_with("ProxyCommand="))
        .unwrap();
    assert!(proxy.contains("-W %h:%p 'user@jump-alias'"));
    assert!(
        proxy.find("IgnoreUnknown=ForkAfterAuthentication").unwrap()
            < proxy.find("ForkAfterAuthentication=no").unwrap()
    );
    let env: Vec<_> = command.get_envs().collect();
    assert!(
        env.iter()
            .any(|(key, value)| *key == "PATH" && value.is_some())
    );
    assert!(env.iter().any(|(key, value)| *key == "SSH_ASKPASS_REQUIRE"
        && *value == Some(std::ffi::OsStr::new("force"))));
}

#[test]
#[ignore = "dedicated worker invoked by the parent process tests"]
fn owned_ssh_worker() {
    let mode = std::env::var("Q045_MODE").expect("parent owns fixture mode");
    let input = vec![
        b'i';
        if mode == "closed-io" {
            0
        } else if matches!(
            mode.as_str(),
            "blocked-input" | "early-refusal" | "early-success" | "cancel" | "existing-master"
        ) {
            8 * 1024 * 1024
        } else {
            8192
        }
    ];
    let wait = if matches!(
        mode.as_str(),
        "blocked-input" | "stderr-flood" | "existing-master"
    ) {
        Duration::from_millis(200)
    } else {
        Duration::from_secs(1)
    };
    let mut sentinel = if mode == "existing-master" {
        Some(Worker(
            Command::new("python3")
                .args([
                    "-I",
                    "-S",
                    "-c",
                    "import signal,time; signal.alarm(5); time.sleep(4)",
                ])
                .spawn()
                .unwrap(),
        ))
    } else {
        None
    };
    let start = Instant::now();
    let result = if mode == "cancel" {
        let control = std::sync::Arc::new(crate::agent_process::Control::default());
        std::thread::scope(|scope| {
            let cancel = control.clone();
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                cancel.shutdown();
            });
            let output = process::run_with_control(
                Dest::new("owned-fixture.invalid", None).finite_ssh_command(),
                &input,
                Duration::from_secs(2),
                control,
            );
            output.map(|output| (output.status.success(), String::new(), String::new()))
        })
    } else {
        run(
            &Dest::new("owned-fixture.invalid", None),
            "fixture-only",
            &input,
            wait,
        )
    };
    let elapsed = start.elapsed();
    let pid: i32 = fs::read_to_string(std::env::var_os("Q045_FIXTURE_PID").unwrap())
        .unwrap()
        .parse()
        .unwrap();
    let exists = unsafe { libc::kill(pid, 0) };
    let exit_error = std::io::Error::last_os_error().raw_os_error();
    assert_eq!(exists, -1, "run returned before fixture {pid} was reaped");
    assert_eq!(exit_error, Some(libc::ESRCH));
    println!("owned fixture mode={mode} pid={pid}: immediate ESRCH after run; elapsed={elapsed:?}");
    if let Some(sentinel) = &mut sentinel {
        assert!(
            sentinel.0.try_wait().unwrap().is_none(),
            "timeout touched an independently owned process"
        );
    }
    if matches!(mode.as_str(), "stdout-limit" | "early-success") {
        let error = result.unwrap_err();
        assert!(
            error.contains(if mode == "stdout-limit" {
                "capture budget"
            } else {
                "not fully written"
            }),
            "{error}"
        );
        return;
    }
    if mode == "cancel" {
        assert!(
            elapsed < Duration::from_secs(1),
            "cancel did not join promptly: {elapsed:?}"
        );
        assert!(
            !matches!(result, Ok((true, _, _))),
            "cancelled child reported success"
        );
        return;
    }
    let (ok, out, err) = result.unwrap();
    match mode.as_str() {
        "blocked-input" | "existing-master" => {
            assert!(
                elapsed < Duration::from_secs(1),
                "200ms deadline excluded input time: elapsed={elapsed:?}, ok={ok}"
            );
            assert!(!ok, "a timed-out command cannot report success");
        }
        "dual-output" => {
            assert!(
                ok,
                "pipe pressure made a finite command time out: {elapsed:?}, stderr bytes={}",
                err.len()
            );
            assert_eq!(out.len(), 128 * 1024);
            assert_eq!(err.len(), 128 * 1024);
            assert!(out.bytes().all(|byte| byte == b'x'));
            assert!(err.bytes().all(|byte| byte == b'y'));
        }
        "normal" => {
            assert!(ok, "{err}");
            assert_eq!(out, "count=8192\n");
            assert_eq!(err, "fixture diagnostic\n");
        }
        "stderr-tail" => {
            assert!(ok, "{err}");
            assert_eq!(out, "complete\n");
            assert!(err.starts_with("[SSH stderr truncated"));
            assert!(err.ends_with("LAST-DIAGNOSTIC\n"));
            assert!(err.len() <= process::STDERR_LIMIT + 100);
        }
        "stderr-flood" => {
            assert!(!ok);
            assert!(elapsed < Duration::from_secs(1));
            assert!(err.starts_with("[SSH stderr truncated"));
            assert!(err.len() <= process::STDERR_LIMIT + 256);
        }
        "early-refusal" => {
            assert!(!ok);
            assert_eq!(err, "Permission denied (owned fixture)\n");
        }
        "closed-io" => {
            assert!(ok, "{err}");
            assert!(
                elapsed >= Duration::from_millis(300),
                "EOF masqueraded as process exit"
            );
        }
        _ => panic!("unknown parent fixture mode"),
    }
}
