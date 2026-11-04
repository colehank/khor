//! The wrapper and the hook door through the real binary — no serve, no
//! network: the registry must work cold. The child's ending decides the
//! word; Claude payloads drive an observed session over stdin.

use std::process::{Command, Stdio};

fn khor(home: &std::path::Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION");
    c
}

fn listed(home: &std::path::Path) -> String {
    let out = khor(home).arg("sessions").output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn retired_hook_exits_without_waiting_for_stdin_or_opening_a_node() {
    let home = std::env::temp_dir().join(format!("khor-retired-hook-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let mut child = khor(&home)
        .args(["state", "--hook"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let opened_node = home.join(".khor").exists();
    let mut stderr = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut stderr).unwrap();
    std::fs::remove_dir_all(&home).unwrap();
    let status = status.unwrap_or_else(|| {
        panic!("retired hook did not exit; opened_node={opened_node}; {stderr}")
    });
    assert!(status.success(), "retired hook exited with {status}: {stderr}");
    assert!(
        !opened_node,
        "retired hook opened the production state path"
    );
}

#[test]
fn wrapped_commands_report_exit_and_read_state_without_hooks() {
    let home = std::env::temp_dir().join(format!("khor-cli-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    // Compare the same localized words the child selects at startup.
    khor_node::config::apply_lang(&home);

    // A failing command: the child's code passes through the wrapper,
    // and the row sinks to 失败.
    let out = khor(&home)
        .args(["run", "--", "sh", "-c", "exit 3"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "the wrapper must hand back the child's code"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("session: shell/"),
        "the id goes to stderr: {stderr}"
    );
    let text = listed(&home);
    assert!(
        text.lines()
            .any(|l| l.contains("shell/") && l.contains(khor_catalog::state::word("failed"))),
        "a non-zero ending must list as failed:\n{text}"
    );

    // A clean command waits to be looked at; seen settles it.
    let out = khor(&home)
        .args(["run", "--title", "quick", "--", "true"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = listed(&home);
    let row = text
        .lines()
        .find(|l| l.contains("quick"))
        .expect("the clean run should be listed");
    assert!(
        row.contains(khor_catalog::state::word("done"))
            && row.contains(&khor_catalog::cli::unread(1)),
        "{row}"
    );
    let id = row.split('\t').next().unwrap().to_owned();
    assert!(khor(&home).args(["seen", &id]).status().unwrap().success());
    let text = listed(&home);
    assert!(
        text.lines()
            .any(|l| l.contains("quick") && l.contains(khor_catalog::state::word("idle"))),
        "looked at = 空闲:\n{text}"
    );

    // Control: a word outside the six is refused by name, and a report
    // with no session to report to is too.
    let out = khor(&home)
        .args(["state", "sleepy", "--session", "tui/cafe1234"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains(&khor_catalog::msg::not_a_state_word("sleepy"))
    );
    let out = khor(&home).args(["state", "busy"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("KHOR_SESSION"));

    let _ = std::fs::remove_dir_all(&home);
}
