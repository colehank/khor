//! `khor quota --why` through the real binary (docs/SHELL.md 第九轮):
//! each vendor's chain, and what every strategy did. A scratch `$HOME`
//! holds a fake Codex login and no Claude one, and a loopback server
//! stands in for the usage endpoint through the test door — **no real
//! login is read and no real endpoint is asked**. The second run is the
//! disk cache: a new process, and the endpoint is not asked again.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn usage_server() -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let body = r#"{"plan_type":"plus","rate_limit":{"allowed":true,"limit_reached":false,"primary_window":{"used_percent":9,"limit_window_seconds":18000,"reset_after_seconds":3600},"secondary_window":{"used_percent":31,"limit_window_seconds":604800,"reset_after_seconds":86400}},"credits":{"has_credits":false,"unlimited":false,"balance":"0"}}"#;
    std::thread::spawn(move || {
        for c in l.incoming() {
            let Ok(mut c) = c else { continue };
            let mut buf = [0u8; 4096];
            let n = c.read(&mut buf).unwrap_or(0);
            log.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase());
            let _ = write!(
                c,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (port, seen)
}

fn why(root: &Path, port: u16) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(["quota", "--why"])
        .env("KHOR_HOME", root.join("store"))
        .env("KHOR_NAME", "box")
        .env("HOME", root.join("home"))
        .env("LANG", "en_US.UTF-8")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("CODEX_HOME")
        .env(
            "KHOR_CODEX_USAGE_URL",
            format!("http://127.0.0.1:{port}/wham/usage"),
        )
        .env(
            "KHOR_CLAUDE_USAGE_URL",
            format!("http://127.0.0.1:{port}/claude"),
        )
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn why_names_every_strategy_and_the_second_process_asks_nobody() {
    let root = std::env::temp_dir().join(format!("khor-quota-why-{}", std::process::id()));
    let _s = Scratch(root.clone());
    let _ = std::fs::remove_dir_all(&root);
    let codex = root.join("home").join(".codex");
    std::fs::create_dir_all(&codex).unwrap();
    std::fs::write(
        codex.join("auth.json"),
        r#"{"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"access_token":"fake-token","account_id":"acc-fake"}}"#,
    )
    .unwrap();
    let (port, seen) = usage_server();

    let first = why(&root, port);
    let line = |said: &str, strategy: &str| {
        said.lines()
            .find(|l| l.trim_start().starts_with(strategy))
            .unwrap_or("")
            .to_owned()
    };
    assert!(
        first.contains("Claude Code") && first.contains("Codex"),
        "{first}"
    );
    assert!(
        line(&first, "oauth-usage").contains("can't run on this machine"),
        "no Claude login in this home: {first}"
    );
    assert!(
        line(&first, "wham-usage").contains("its reading was used"),
        "{first}"
    );
    {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1, "one ask: {seen:?}");
        assert!(
            seen[0].contains("authorization: bearer fake-token"),
            "{}",
            seen[0]
        );
        assert!(
            seen[0].contains("chatgpt-account-id: acc-fake"),
            "{}",
            seen[0]
        );
    }

    let second = why(&root, port);
    assert!(
        line(&second, "wham-usage").contains("not asked"),
        "a fresh reading on this machine answers a new process: {second}"
    );
    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "the second process asked again"
    );
}
