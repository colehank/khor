//! The persistent host through the real binary: the opener exits, the
//! host lives on (reparented to init), the six-word shell face moves
//! with the foreground process group, attachers replay-then-stream over
//! the socket, the ending is recorded, and close kills exactly one
//! session. Controls: wrong cookie refused by name, a live close leaves
//! no process behind. Every wait has a deadline.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::host::{self, ClientOp, HostFile};

fn khor(home: &Path) -> Command {
    // The subprocess resolves its CLI language from the environment. This
    // test process otherwise keeps the catalog's unresolved Chinese default,
    // so an English Idle/Interrupted row looked like a lifecycle timeout.
    khor_catalog::lang::set(khor_catalog::lang::from_env());
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION");
    c
}

fn sessions(home: &Path) -> String {
    let out = khor(home).arg("sessions").output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn wait_until(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("timed out waiting for: {what}");
}

/// Kills a pid when it goes out of scope, however it goes.
///
/// A test that deliberately makes a process outlive its host is a test
/// that leaks one when it fails — and this one failed on purpose during
/// its own red check, leaving a `sh` that ignores SIGHUP running under
/// init with a `sleep` under it. An assertion panics past its cleanup;
/// a `Drop` does not.
struct Reaper(u32);

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-9", &self.0.to_string()])
            .stderr(std::process::Stdio::null())
            .status();
    }
}

fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn open_detached(home: &Path, args: &[&str]) -> (String, PathBuf) {
    let out = khor(home)
        .arg("open")
        .arg("-d")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let dir = home
        .join(".khor")
        .join("sessions")
        .join(sid.replace('/', "-"));
    assert!(
        dir.join("host.json").exists(),
        "the host should have checked in"
    );
    (sid, dir)
}

/// Reads from the attach stream until `needle` shows up (or panics).
fn read_until(stream: &mut std::net::TcpStream, needle: &str, secs: u64) -> String {
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut seen = Vec::new();
    let mut buf = [0u8; 4096];
    while Instant::now() < deadline {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => seen.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
        if String::from_utf8_lossy(&seen).contains(needle) {
            return String::from_utf8_lossy(&seen).into_owned();
        }
    }
    panic!(
        "never saw {needle:?} in: {}",
        String::from_utf8_lossy(&seen)
    );
}

/// **The host says what terminal it is; it does not pass one on.**
///
/// A 持久 session outlives its opener and takes attachers with
/// different terminals, so "whatever `TERM` the opener had" is not even
/// a coherent answer — and on a machine where khor installed itself and
/// runs `serve` as a daemon it is `unknown`, which is not a terminfo
/// entry. Measured on turing: every tmux bridge stood up there died at
/// birth (`open terminal failed: missing or unsuitable terminal:
/// unknown`), and the person clicking the row read 读不到帧:
/// Connection reset.
///
/// The opener is given a bogus `TERM` on purpose. Without it this test
/// passes on any developer's machine — the ambient `TERM` there is
/// already `xterm-256color`, so an implementation that simply inherits
/// would look correct exactly where nobody is affected, and stay broken
/// exactly where everybody is.
#[test]
fn the_child_is_told_what_terminal_it_is_in_not_what_the_opener_had() {
    let home = std::env::temp_dir().join(format!("khor-term-{}", std::process::id()));
    let out = khor(&home)
        .env("TERM", "khor-not-a-real-terminfo-entry")
        .env("NO_COLOR", "1")
        .env("FORCE_COLOR", "0")
        .env("CLICOLOR", "0")
        .env("CLICOLOR_FORCE", "0")
        .args([
            "open",
            "-d",
            "--",
            "sh",
            "-c",
            "printf 'TERM=[%s] COLORTERM=[%s] NO_COLOR=[%s] FORCE_COLOR=[%s] CLICOLOR=[%s] CLICOLOR_FORCE=[%s] END\\n' \"$TERM\" \"$COLORTERM\" \"${NO_COLOR-unset}\" \"${FORCE_COLOR-unset}\" \"${CLICOLOR-unset}\" \"${CLICOLOR_FORCE-unset}\"; sleep 30",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let dir = home
        .join(".khor")
        .join("sessions")
        .join(sid.replace('/', "-"));

    // The `sleep` outlives an assertion that panics; a Drop does not.
    let hf: HostFile =
        serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    let _reap = Reaper(hf.child_pid);

    let mut s = host::connect(&dir, 100, 30).unwrap();
    let seen = read_until(&mut s, " END", 10);
    assert!(
        seen.contains("TERM=[xterm-256color]"),
        "the child must be told khor's own screen model, not the opener's: {seen}"
    );
    for field in ["COLORTERM=[truecolor]", "NO_COLOR=[unset]", "FORCE_COLOR=[unset]", "CLICOLOR=[unset]", "CLICOLOR_FORCE=[unset]"] {
        assert!(seen.contains(field), "PTY must own its color capability, missing {field}: {seen}");
    }
    assert!(
        khor(&home)
            .args(["close", &sid])
            .status()
            .unwrap()
            .success()
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_hosted_session_outlives_its_opener_and_walks_the_shell_face() {
    let home = std::env::temp_dir().join(format!("khor-open-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();

    let (sid, dir) = open_detached(&home, &["--title", "work", "--", "sh"]);
    assert!(sid.starts_with("shell/"), "{sid}");

    // Truly detached: the opener is gone (its command returned), and the
    // host has been reparented to init.
    let hf: HostFile =
        serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    let ppid = Command::new("ps")
        .args(["-o", "ppid=", "-p", &hf.host_pid.to_string()])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ppid.stdout).trim(),
        "1",
        "the host must not hang off anyone"
    );

    // The shell reaches its prompt: no foreground job = 空闲.
    wait_until("the prompt to read as idle", 15, || {
        sessions(&home)
            .lines()
            .any(|l| l.contains(&sid) && l.contains(khor_catalog::state::word("idle")))
    });

    // Attach over the socket: type into the pty, see the output.
    let mut s = host::connect(&dir, 100, 30).unwrap();
    host::write_frame(&mut s, &ClientOp::Input(b"echo khor-marker-ok\n".to_vec())).unwrap();
    read_until(&mut s, "khor-marker-ok", 10);

    // A foreground job flips the face to 忙碌 and back.
    host::write_frame(&mut s, &ClientOp::Input(b"sleep 3\n".to_vec())).unwrap();
    wait_until("the sleep to read as busy", 10, || {
        sessions(&home)
            .lines()
            .any(|l| l.contains(&sid) && l.contains(khor_catalog::state::word("busy")))
    });
    wait_until("the prompt to read as idle again", 10, || {
        sessions(&home)
            .lines()
            .any(|l| l.contains(&sid) && l.contains(khor_catalog::state::word("idle")))
    });

    // A late attacher gets the past from the replay ring without asking.
    let mut late = host::connect(&dir, 100, 30).unwrap();
    read_until(&mut late, "khor-marker-ok", 5);
    drop(late);

    // Control: the port alone opens nothing.
    let mut bad = std::net::TcpStream::connect(("127.0.0.1", hf.port)).unwrap();
    host::write_frame(
        &mut bad,
        &host::Hello {
            cookie: "not-the-cookie".into(),
            cols: 80,
            rows: 24,
        },
    )
    .unwrap();
    let w: host::Welcome = host::read_frame(&mut bad).unwrap();
    assert!(
        !w.ok && w.why == khor_catalog::msg::wrong_cookie(),
        "{}",
        w.why
    );

    // The ending is recorded even with nobody attached: exit 7 = 失败,
    // the row stays (sinks), the host leaves.
    host::write_frame(&mut s, &ClientOp::Input(b"exit 7\n".to_vec())).unwrap();
    wait_until("the ending to read as failed", 10, || {
        sessions(&home)
            .lines()
            .any(|l| l.contains(&sid) && l.contains(khor_catalog::state::word("failed")))
    });
    wait_until("the host to leave", 10, || !pid_alive(hf.host_pid));
    assert!(dir.exists(), "a settled row stays listed until closed");

    // Close removes the settled row.
    assert!(
        khor(&home)
            .args(["close", &sid])
            .status()
            .unwrap()
            .success()
    );
    assert!(!dir.exists());
    assert!(!sessions(&home).contains(&sid));

    let _ = std::fs::remove_dir_all(&home);
}

/// **A host that dies while its child lives must not read as 失败.**
///
/// The registry records the host's pid, and the missing-ending rule
/// turns a dead recorded pid into 失败 — which for a hosted session
/// means khor's own plumbing dying is reported as the user's work
/// failing. It is 中断: the process is there, and 中断 vs 失败 is
/// exactly the question of whether it is (`live::face` derives it).
///
/// Real, not simulated: a real host, a real child, `kill -9` on the
/// host only, the word read back out of the real binary.
///
/// **The child has to ignore SIGHUP or there is nothing to test.**
/// Measured on this machine: killing the host closes the pty master, and
/// the kernel hangs up the foreground group — a plain `sleep 300` dies
/// with it, and so does an interactive `zsh`. Only a child that ignores
/// the signal outlives its host, which is both why this hole is narrow
/// and why the test has to say `trap "" HUP` out loud.
#[test]
fn a_host_that_dies_while_its_child_lives_is_not_a_failure() {
    let home = std::env::temp_dir().join(format!("khor-orphan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();

    let (sid, dir) = open_detached(&home, &["--", "sh", "-c", r#"trap "" HUP; sleep 300"#]);
    let hf: HostFile =
        serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    // Whatever happens below, this child does not outlive the test: it
    // ignores SIGHUP, so nothing else would take it away.
    let _reap = Reaper(hf.child_pid);
    assert!(
        pid_alive(hf.host_pid) && pid_alive(hf.child_pid),
        "control: both are up"
    );

    // The host alone.
    Command::new("kill")
        .args(["-9", &hf.host_pid.to_string()])
        .status()
        .unwrap();
    wait_until("the host to be gone", 5, || !pid_alive(hf.host_pid));
    assert!(
        pid_alive(hf.child_pid),
        "the child ignores SIGHUP, so it is still working"
    );

    wait_until("the row to say 中断", 10, || {
        sessions(&home)
            .lines()
            .any(|l| l.contains(&sid) && l.contains(khor_catalog::state::word("errored")))
    });
    assert!(
        !sessions(&home).contains(khor_catalog::state::word("failed")),
        "失败 would tell the user to start over on work that is still running"
    );

    // The control group: now the child goes too, and the ordinary
    // missing-ending rule applies again. Without this half, a `face`
    // that simply never said 失败 would pass the assertion above.
    Command::new("kill")
        .args(["-9", &hf.child_pid.to_string()])
        .status()
        .unwrap();
    wait_until("the child to be gone", 5, || !pid_alive(hf.child_pid));
    wait_until("the row to say 失败", 10, || {
        sessions(&home)
            .lines()
            .any(|l| l.contains(&sid) && l.contains(khor_catalog::state::word("failed")))
    });

    assert!(
        khor(&home)
            .args(["close", &sid])
            .status()
            .unwrap()
            .success()
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn closing_a_live_hosted_session_leaves_no_process_behind() {
    let home = std::env::temp_dir().join(format!("khor-open-close-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();

    let (sid, dir) = open_detached(&home, &["--", "sh"]);
    let hf: HostFile =
        serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    assert!(pid_alive(hf.host_pid) && pid_alive(hf.child_pid));

    assert!(
        khor(&home)
            .args(["close", &sid])
            .status()
            .unwrap()
            .success()
    );
    assert!(!dir.exists(), "closing forgets the session");
    wait_until("the child to be gone", 5, || !pid_alive(hf.child_pid));
    wait_until("the host to be gone", 5, || !pid_alive(hf.host_pid));

    let _ = std::fs::remove_dir_all(&home);
}

/// **The person is told what khor could not find, not how long it
/// waited.**
///
/// 2026-08-31, on the machine this is written on: `khor serve` is a
/// launchd agent, so it searches `/usr/bin:/bin:/usr/sbin:/sbin` —
/// and `tmux`, which every tui row's terminal is a client of, lives in
/// `/opt/homebrew/bin`. The host died at exec in about a millisecond,
/// its three streams were on `/dev/null` because that is what makes it
/// outlive its opener, and so the one thing the person saw was the
/// opener's timeout: 宿主没在 5 秒内就位.
///
/// **A sentence about speed, for something that never waited.** Read
/// straight, it says the machine is slow or the session is wedged, and
/// both send somebody to look at a thing that is not sick — the same
/// shape as the `npx` refusal that blamed a working agent for not
/// speaking ACP.
///
/// The `PATH`s are pinned on purpose, both of them: with an ambient
/// `PATH` this passes on any developer's machine, where the program is
/// found and no message is ever produced, and would stay broken exactly
/// where everybody is affected. `KHOR_PATH` is pinned too because
/// otherwise the login-shell repair finds the program and there is
/// nothing to report.
#[test]
fn a_host_that_cannot_find_its_program_says_so_instead_of_timing_out() {
    let home = std::env::temp_dir().join(format!("khor-nopath-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let bare = "/usr/bin:/bin:/usr/sbin:/sbin";
    let out = khor(&home)
        .env("PATH", bare)
        .env("KHOR_PATH", bare)
        .args(["open", "-d", "--", "khor-no-such-program", "--hello"])
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "it cannot have opened: {said}");
    assert!(
        said.contains("khor-no-such-program"),
        "the refusal must name the program khor could not find: {said}"
    );
    assert!(
        said.contains("/usr/sbin"),
        "and where it looked, which is the other half of the next step: {said}"
    );
    assert!(
        !said.contains("5 秒"),
        "and it must not be reported as a wait — it never waited: {said}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// The control the sentence above cannot supply: **the same command,
/// the same binary, the same everything, and only `KHOR_PATH`
/// differing.** Without this the test above passes for an
/// implementation that merely says a nicer sentence and still cannot
/// start anything — a message is not a repair.
#[test]
fn the_same_command_starts_once_khor_is_told_where_to_look() {
    let home = std::env::temp_dir().join(format!("khor-onpath-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    // A real program, reachable only through a directory that is not on
    // `PATH` — which is the whole of what launchd does to `khor serve`.
    let prog = bin.join("khor-test-only-here");
    std::fs::write(&prog, "#!/bin/sh\nsleep 30\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let bare = "/usr/bin:/bin:/usr/sbin:/sbin";

    let blind = khor(&home)
        .env("PATH", bare)
        .env("KHOR_PATH", bare)
        .args(["open", "-d", "--", "khor-test-only-here"])
        .output()
        .unwrap();
    assert!(
        !blind.status.success(),
        "the control arm has to actually fail"
    );

    let told = khor(&home)
        .env("PATH", bare)
        .env("KHOR_PATH", format!("{}:{bare}", bin.display()))
        .args(["open", "-d", "--", "khor-test-only-here"])
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&told.stdout),
        String::from_utf8_lossy(&told.stderr)
    );
    assert!(
        told.status.success(),
        "only KHOR_PATH differs, and it must now start: {said}"
    );

    let sid = String::from_utf8_lossy(&told.stdout).trim().to_owned();
    let dir = home
        .join(".khor")
        .join("sessions")
        .join(sid.replace('/', "-"));
    let hf: HostFile =
        serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    let _reap = Reaper(hf.child_pid);
    let _ = khor(&home).args(["close", &sid]).output();
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_private_host_does_not_inherit_an_unrelated_native_agent_store() {
    let home = std::env::temp_dir().join(format!("khor-host-vendor-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let out = khor(&home)
        .env("CODEX_HOME", "/unrelated-codex")
        .env("CLAUDE_CONFIG_DIR", "/unrelated-claude")
        .args(["open", "-d", "--", "sh", "-c",
            "printf 'codex=%s;claude=%s;done\\n' \"$CODEX_HOME\" \"$CLAUDE_CONFIG_DIR\"; sleep 30"])
        .output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let sid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let dir = home.join(".khor/sessions").join(sid.replace('/', "-"));
    let hf = host::read_host_file(&dir).unwrap();
    let _reap = Reaper(hf.host_pid);
    let mut stream = host::connect_host_versioned(&dir, 80, 24).unwrap().stream;
    let shown = read_until(&mut stream, ";done", 10);
    assert!(shown.contains(&format!("codex={};claude={};done", home.join(".codex").display(), home.join(".claude").display())), "wrong store: {shown}");
    assert!(khor(&home).args(["close", &sid]).status().unwrap().success());
    let _ = std::fs::remove_dir_all(home);
}
