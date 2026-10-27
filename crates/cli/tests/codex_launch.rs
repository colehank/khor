//! Retire old installed launchers through the actual CLI binary, with a fake
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
            .env("KHOR_HOME", &self.dir)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE");
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
fn retired_launcher_preserves_interactive_arguments_and_stdout() {
    let f = Fixture::new("interactive");
    for args in [
        vec!["hello"],
        vec!["resume", "--last"],
        vec!["fork", "previous"],
        vec!["--no-daemon", "resume", "previous"],
        vec!["-c", "tui.terminal_title=[\"project\"]", "resume", "--last"],
        vec!["--", "--no-daemon"],
    ] {
        let result = f.run(&args, true);
        assert!(!result.stdout.starts_with(b"\x1b]2;\x07"));
        assert_eq!(words(&result), args);
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
fn retirement_does_not_depend_on_codex_version_support() {
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
fn shell_integration_preserves_user_path() {
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
        f.real.join("codex").to_str().unwrap()
    );
}

#[test]
fn launcher_survives_missing_tmux_environment_marker() {
    let f = Fixture::new("missing-marker");
    let result = Command::new(f.shim.join("codex"))
        .args(["resume", "--last"])
        .env("PATH", std::env::join_paths([&f.shim, &f.real]).unwrap())
        .env("KHOR_HOME", &f.dir)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .env_remove("KHOR_CODEX_SHIM_DIR")
        .output()
        .unwrap();
    assert_eq!(words(&result), ["resume", "--last"]);
}

#[test]
fn existing_tmux_server_without_marker_launches_codex_on_a_pty() {
    if Command::new("tmux").arg("-V").output().is_err() {
        return;
    }
    let f = Fixture::new("tmux-pty");
    let socket = format!("/tmp/khor-launch-{}.sock", std::process::id());
    struct Server(String);
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = Command::new("tmux")
                .args(["-S", &self.0, "kill-server"])
                .output();
        }
    }
    let _server = Server(socket.clone());
    let tmux = |args: &[&str]| {
        let out = Command::new("tmux")
            .args(["-S", &socket])
            .args(args)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .env_remove("KHOR_CODEX_SHIM_DIR")
            .env("SHELL", "/bin/sh")
            .env("HOME", &f.dir)
            .env("CODEX_HOME", f.dir.join("codex-home"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    // The server predates the launcher; it cannot pass the marker to panes.
    tmux(&[
        "-f",
        "/dev/null",
        "new-session",
        "-d",
        "-s",
        "seed",
        "sleep 60",
    ]);
    tmux(&["set-option", "-g", "mouse", "off"]);
    tmux(&["set-option", "-g", "default-shell", "/bin/sh"]);
    tmux(&["set-option", "-gw", "remain-on-exit", "on"]);
    let result = f.dir.join("result");
    std::fs::write(f.real.join("codex"), format!(
        "#!/bin/sh\nif [ \"$1\" = --help ]; then echo '  --no-daemon'; exit 0; fi\ntest -t 0 || exit 24\ntest -z \"$KHOR_CODEX_SHIM_DIR\" || exit 25\nprintf '%s\\0' \"$@\" > '{}'\nexit 23\n", result.display()
    )).unwrap();
    let mut paths = vec![f.shim.clone(), f.real.clone()];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let path = format!(
        "PATH={}",
        std::env::join_paths(paths).unwrap().to_string_lossy()
    );
    let home = format!("KHOR_HOME={}", f.dir.display());
    tmux(&[
        "new-session",
        "-d",
        "-s",
        "probe",
        "-e",
        &path,
        "-e",
        &home,
        &format!(
            "/usr/bin/env '{}' '{}' '{}'",
            path,
            home,
            f.shim.join("codex").display()
        ),
    ]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let status = tmux(&[
            "display-message",
            "-p",
            "-t",
            "probe",
            "#{pane_dead_status}",
        ]);
        if !status.is_empty() {
            assert_eq!(status, "23");
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "launcher hung: {}",
            tmux(&["capture-pane", "-p", "-t", "probe"])
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let bytes = std::fs::read(result).unwrap();
    assert_eq!(bytes, b"\0");
    assert_eq!(
        tmux(&["display-message", "-p", "-t", "probe", "#{mouse}"]),
        "0"
    );
    assert_eq!(tmux(&["show-option", "-gv", "mouse"]), "off");
    assert_eq!(
        tmux(&["display-message", "-p", "-t", "seed", "#{mouse}"]),
        "0"
    );
}

#[test]
fn bare_argv0_does_not_invent_environment_hints() {
    use std::os::unix::process::CommandExt;
    let mut f = Fixture::new("bare-argv0");
    f.shim = f.dir.join(".khor/codex-bin");
    std::fs::create_dir_all(&f.shim).unwrap();
    symlink(env!("CARGO_BIN_EXE_khor"), f.shim.join("codex")).unwrap();
    std::fs::write(
        f.real.join("codex"),
        "#!/bin/sh\nprintf '%s\\0' \"$KHOR_HOME\"\nexit 23\n",
    )
    .unwrap();
    let result = Command::new(f.shim.join("codex"))
        .arg0("codex")
        .arg("--version")
        .env("PATH", std::env::join_paths([&f.shim, &f.real]).unwrap())
        .env_remove("KHOR_CODEX_SHIM_DIR")
        .env_remove("KHOR_HOME")
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .output()
        .unwrap();
    assert_eq!(words(&result), Vec::<String>::new());
}
