//! Native binaries run console lifecycle sessions themselves. The hidden
//! entry point validates its arguments before opening a store or a window.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-cli-console-{}-{}",
            std::process::id(),
            khor_node::link::fresh_hex().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_console_rejects_non_lifecycle_and_malformed_arguments() {
    let scratch = Scratch::new();
    let root = scratch.0.to_str().unwrap();
    for args in [
        vec!["_console"],
        vec!["_console", root, "devices"],
        vec!["_console", "relative-root", "mcp"],
        vec!["_console", root, "serve", "--unknown"],
        vec!["_console", root, "mcp", "--unknown"],
        vec!["_console", root, "relay", "--unknown"],
        vec!["_console", root, "pipe", "--via", "lab", "host", "NaN"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_khor"))
            .args(&args)
            .env("KHOR_HOME", scratch.0.join("environment"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
        assert!(!scratch.0.join(".khor").exists(), "{args:?}");
        assert!(!scratch.0.join("environment").exists(), "{args:?}");
    }
}

#[test]
fn native_console_mcp_uses_its_explicit_store_after_changing_directory() {
    let scratch = Scratch::new();
    let requested = scratch.0.join("requested");
    let environment = scratch.0.join("environment");
    let working = scratch.0.join("working");
    std::fs::create_dir_all(&working).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("_console")
        .arg(&requested)
        .arg("mcp")
        .env("KHOR_HOME", &environment)
        .env("KHOR_NAME", "console-test")
        .env_remove("KHOR_SESSION")
        .current_dir(&working)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(
        b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"devices\",\"arguments\":{}}}\n"
    ).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["id"], 1);
    assert_ne!(response["result"]["isError"], true, "{response}");
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("console-test")
    );
    assert!(requested.join(".khor/identity.key").is_file());
    assert!(!environment.exists());
    assert!(!working.join(".khor").exists());
}
