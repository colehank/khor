//! Run the scoped launcher through the actual CLI binary, with a fake
//! Codex executable. No model, network, user store or Codex config.
#![cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::{Command, Output};

struct Fixture {
    dir: PathBuf,
    shim: PathBuf,
    real: PathBuf,
}
impl Fixture {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("khor-codex-launch-{name}-{}", std::process::id()));
        let shim = dir.join("shim");
        let real = dir.join("real");
        for path in [&shim, &real] {
            std::fs::create_dir_all(path).unwrap();
        }
        symlink(env!("CARGO_BIN_EXE_khor"), shim.join("codex")).unwrap();
        let file = real.join("codex");
        std::fs::write(&file, "#!/bin/sh\nif [ \"$1\" = --help ]; then printf '  --no-daemon\\n'; exit 0; fi\nprintf '%s\\0' \"$@\"\nexit 23\n").unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self { dir, shim, real }
    }
    fn run(&self, args: &[&str], scoped: bool) -> Output {
        let mut command = Command::new(if scoped {
            self.shim.join("codex")
        } else {
            self.real.join("codex")
        });
        command
            .args(args)
            .env(
                "PATH",
                std::env::join_paths([&self.shim, &self.real]).unwrap(),
            )
            .env("KHOR_HOME", &self.dir);
        if scoped {
            command.env("KHOR_CODEX_SHIM_DIR", &self.shim);
        } else {
            command.env_remove("KHOR_CODEX_SHIM_DIR");
        }
        command.output().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
fn words(output: &Output) -> Vec<String> {
    assert_eq!(
        output.status.code(),
        Some(23),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .strip_prefix(b"\x1b]2;\x07")
        .unwrap_or(&output.stdout)
        .split(|b| *b == 0)
        .filter(|b| !b.is_empty())
        .map(|b| String::from_utf8(b.to_vec()).unwrap())
        .collect()
}
#[test]
fn interactive_launch_adds_native_thread_title_and_clears_the_previous_title() {
    let f = Fixture::new("interactive");
    for args in [
        vec!["hello"],
        vec!["resume", "--last"],
        vec!["fork", "previous"],
        vec!["--no-daemon", "resume", "previous"],
    ] {
        let result = f.run(&args, true);
        assert!(
            result.stdout.starts_with(b"\x1b]2;\x07"),
            "old pane title must clear before Codex starts"
        );
        let output = words(&result);
        assert_eq!(output.iter().filter(|s| *s == "--no-daemon").count(), 1);
        assert_eq!(
            output
                .iter()
                .filter(|s| *s == "tui.terminal_title=[\"thread-id\"]")
                .count(),
            1
        );
        assert!(
            output
                .iter()
                .all(|s| !s.contains("hooks.") && !s.contains("bypass"))
        );
        assert_eq!(words(&f.run(&args, false)), args);
    }
}
#[test]
fn administrative_remote_and_noninteractive_commands_are_verbatim() {
    let f = Fixture::new("passthrough");
    for args in [
        vec!["exec", "hello"],
        vec!["app-server"],
        vec!["--version"],
        vec!["resume", "--help"],
        vec!["--remote", "unix:///tmp/fixture.sock"],
        vec!["-c", "model='fixture'", "exec", "hello"],
    ] {
        let result = f.run(&args, true);
        assert!(!result.stdout.starts_with(b"\x1b]2;\x07"));
        assert_eq!(words(&result), args);
    }
}

#[test]
fn unsupported_or_failed_capability_probe_leaves_legacy_codex_untouched() {
    let f = Fixture::new("legacy");
    for (help, status) in [("Usage: codex [OPTIONS]", 0), ("  --no-daemon", 1)] {
        std::fs::write(f.real.join("codex"), format!(
            "#!/bin/sh\nif [ \"$1\" = --help ]; then printf '%s\\n' '{help}'; exit {status}; fi\nprintf '%s\\0' \"$@\"\nexit 23\n"
        )).unwrap();
        let args = ["resume", "--last"];
        let result = f.run(&args, true);
        assert!(!result.stdout.starts_with(b"\x1b]2;\x07"));
        assert_eq!(words(&result), args);
    }
}

#[test]
fn caller_cli_title_remains_exact_and_prompt_text_is_not_a_flag() {
    let f = Fixture::new("overrides");
    let args = ["-c", "tui.terminal_title=[\"project\"]", "resume", "--last"];
    let output = words(&f.run(&args, true));
    assert_eq!(output, [&["--no-daemon"][..], &args].concat());
    let output = words(&f.run(&["--", "--no-daemon"], true));
    assert_eq!(output.first().unwrap(), "--no-daemon");
    assert_eq!(&output[output.len() - 2..], ["--", "--no-daemon"]);
}

#[test]
fn shell_integration_restores_scoped_path_after_user_startup() {
    let f = Fixture::new("shell");
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../node/shell/khor.bash");
    std::fs::write(
        f.dir.join(".bashrc"),
        format!("export PATH='{}:/usr/bin:/bin'\n", f.real.display()),
    )
    .unwrap();
    let out = Command::new("/bin/bash")
        .args([
            "--init-file",
            script.to_str().unwrap(),
            "-ic",
            "command -v codex",
        ])
        .env("HOME", &f.dir)
        .env("KHOR_CODEX_SHIM_DIR", &f.shim)
        .env_remove("TMUX")
        .env_remove("_khor_loaded")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        f.shim.join("codex").to_str().unwrap()
    );
}
