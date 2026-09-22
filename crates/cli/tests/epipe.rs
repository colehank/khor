//! `khor devices | head -1` must not panic (SHELL.md 第五轮 第 4 条).
//!
//! Rust ignores SIGPIPE, so a write to a pipe whose reader has gone
//! comes back as an error — and `println!` turns that error into a
//! panic: `failed printing to stdout: Broken pipe`, exit 101, a stack
//! hint on the terminal, for a person who only wanted the first line.
//!
//! The reader is closed **before** khor starts, so the very first write
//! meets the broken pipe; nothing here depends on timing.

use std::fs;
use std::process::{Command, Stdio};

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_listing_into_a_closed_pipe_leaves_quietly() {
    let home = Scratch(std::env::temp_dir().join(format!("khor-epipe-{}", std::process::id())));
    let _ = fs::remove_dir_all(&home.0);
    fs::create_dir_all(&home.0).unwrap();
    for verb in [&["devices"][..], &["config"][..], &["version"][..]] {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let out = Command::new(env!("CARGO_BIN_EXE_khor"))
            .args(verb)
            .env("KHOR_HOME", &home.0)
            .env("KHOR_NAME", "box")
            .env_remove("KHOR_SESSION")
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(
            !said.contains("panicked") && out.status.code() != Some(101),
            "`khor {}` into a closed pipe must leave quietly, not panic: {:?} {said}",
            verb.join(" "),
            out.status
        );
    }
}
