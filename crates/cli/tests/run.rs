//! The wrapper and the hook door through the real binary — no serve, no
//! network: the registry must work cold. The child's ending decides the
//! word; Claude payloads drive an observed session over stdin.

use std::io::Write;
use std::process::{Command, Stdio};

fn khor(home: &std::path::Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home).env("KHOR_NAME", "box").env_remove("KHOR_SESSION");
    c
}

fn listed(home: &std::path::Path) -> String {
    let out = khor(home).arg("sessions").output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_wrapped_command_and_a_hooked_agent_both_land_in_the_list() {
    let home = std::env::temp_dir().join(format!("khor-cli-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();

    // A failing command: the child's code passes through the wrapper,
    // and the row sinks to 失败.
    let out = khor(&home).args(["run", "--", "sh", "-c", "exit 3"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3), "the wrapper must hand back the child's code");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("session: shell/"), "the id goes to stderr: {stderr}");
    let text = listed(&home);
    assert!(
        text.lines().any(|l| l.contains("shell/") && l.contains("failed")),
        "a non-zero ending must list as failed:\n{text}"
    );

    // A clean command waits to be looked at; seen settles it.
    let out = khor(&home).args(["run", "--title", "quick", "--", "true"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = listed(&home);
    let row = text
        .lines()
        .find(|l| l.contains("quick"))
        .expect("the clean run should be listed");
    assert!(row.contains("done") && row.contains("未读 1"), "{row}");
    let id = row.split('\t').next().unwrap().to_owned();
    assert!(khor(&home).args(["seen", &id]).status().unwrap().success());
    let text = listed(&home);
    assert!(
        text.lines().any(|l| l.contains("quick") && l.contains("idle")),
        "looked at = 空闲:\n{text}"
    );

    // Claude hook payloads over stdin, the observed path: no wrapper,
    // no pid, the words still move.
    let feed = |event: &str| {
        let mut c = khor(&home)
            .args(["state", "--hook"])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let payload = format!(
            r#"{{"session_id":"cafe1234","cwd":"/tmp/proj","hook_event_name":"{event}"}}"#
        );
        c.stdin.take().unwrap().write_all(payload.as_bytes()).unwrap();
        let out = c.wait_with_output().unwrap();
        assert!(out.status.success(), "{event}: {}", String::from_utf8_lossy(&out.stderr));
    };
    feed("SessionStart");
    feed("UserPromptSubmit");
    let text = listed(&home);
    assert!(
        text.lines().any(|l| l.contains("tui/cafe1234") && l.contains("busy") && l.contains("proj")),
        "the observed agent should list busy:\n{text}"
    );
    feed("Stop");
    let text = listed(&home);
    assert!(
        text.lines().any(|l| l.contains("tui/cafe1234") && l.contains("done")),
        "the turn's end should list done:\n{text}"
    );

    // Control: a word outside the six is refused by name, and a report
    // with no session to report to is too.
    let out = khor(&home).args(["state", "sleepy", "--session", "tui/cafe1234"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("六词"));
    let out = khor(&home).args(["state", "busy"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("KHOR_SESSION"));

    let _ = std::fs::remove_dir_all(&home);
}
