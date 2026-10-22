//! Read-only classification of native Codex command lines. Never launches a process.
use std::ffi::OsString;

/// A value option is skipped before deciding whether a positional word
/// is a subcommand: `-m exec` is a model, not the noninteractive command.
pub(crate) fn interactive(args: &[OsString]) -> bool {
    let mut value = false;
    let mut positional = false;
    let mut command = None;
    for arg in args {
        if value {
            value = false;
            continue;
        }
        let s = arg.to_string_lossy();
        if positional {
            continue;
        }
        if s == "--" {
            positional = true;
            continue;
        }
        if matches!(
            s.as_ref(),
            "-h" | "--help" | "-V" | "--version" | "--remote"
        ) || s.starts_with("--remote=")
        {
            return false;
        }
        if takes_value(&s) {
            value = true;
            continue;
        }
        if s.starts_with('-') {
            continue;
        }
        if command.is_none() {
            command = Some(s.into_owned());
        }
    }
    !matches!(
        command.as_deref(),
        Some(
            "agents"
                | "exec"
                | "e"
                | "review"
                | "login"
                | "logout"
                | "mcp"
                | "mcp-server"
                | "plugin"
                | "app-server"
                | "remote-control"
                | "app"
                | "completion"
                | "update"
                | "doctor"
                | "sandbox"
                | "debug"
                | "apply"
                | "a"
                | "queue"
                | "archive"
                | "delete"
                | "migrate-rollouts"
                | "unarchive"
                | "cloud"
                | "exec-server"
                | "features"
                | "help"
        )
    )
}

fn takes_value(s: &str) -> bool {
    matches!(
        s,
        "-c" | "--config"
            | "-m"
            | "--model"
            | "-p"
            | "--profile"
            | "-s"
            | "--sandbox"
            | "-C"
            | "--cd"
            | "-a"
            | "--ask-for-approval"
            | "-i"
            | "--image"
            | "--local-provider"
            | "--add-dir"
            | "--enable"
            | "--disable"
            | "--remote-auth-token-env"
    )
}

/// An explicit resume UUID identifies the selected thread without a title or
/// birth/cwd guess. A free-form initial prompt cannot establish this contract.
pub(crate) fn resuming(args: &[OsString], thread: &str) -> bool {
    let mut values = args.iter();
    let mut resume = false;
    let mut positional = false;
    while let Some(arg) = values.next() {
        let value = arg.to_string_lossy();
        if !positional && value == "--" {
            if !resume {
                return false;
            }
            positional = true;
            continue;
        }
        if !positional && value == "--last" {
            return false;
        }
        if !positional && takes_value(&value) {
            values.next();
            continue;
        }
        if !positional && value.starts_with('-') {
            continue;
        }
        if !resume {
            if value != "resume" {
                return false;
            }
            resume = true;
        } else {
            return value == thread;
        }
    }
    false
}

/// Feature detection is bounded and happens before closing the GUI. Older
/// CLIs without a shared daemon keep their normal resume spelling.
pub(crate) fn supports_no_daemon(bin: &std::path::Path, path: &str) -> bool {
    use std::io::Read;
    let Ok(mut child) = std::process::Command::new(bin)
        .env("PATH", path)
        .arg("--help")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut text = String::new();
                if let Some(stdout) = child.stdout.take() {
                    let _ = stdout.take(65536).read_to_string(&mut text);
                }
                return status.success()
                    && text.split_whitespace().any(|word| word == "--no-daemon");
            }
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

/// Extract CLI config overrides without mistaking option values or prompt
/// text for configuration. Every reader of native title selection uses this.
pub(crate) fn config_values(args: &[OsString]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let Some(arg) = arg.to_str() else { continue };
        if arg == "--" {
            break;
        }
        if arg == "-c" || arg == "--config" {
            if let Some(value) = args.next().and_then(|v| v.to_str()) {
                out.push(value);
            }
        } else if let Some(value) = arg
            .strip_prefix("--config=")
            .or_else(|| arg.strip_prefix("-c").filter(|s| !s.is_empty()))
        {
            out.push(value);
        } else if takes_value(arg) {
            args.next();
        }
    }
    out
}

fn title_key(value: &str) -> Option<(bool, &str)> {
    let (key, value) = value.split_once('=')?;
    let parts: Vec<_> = key
        .split('.')
        .map(|part| part.trim().trim_matches(['\'', '"']))
        .collect();
    match parts.as_slice() {
        ["tui"] => Some((false, value)),
        ["tui", "terminal_title"] => Some((true, value)),
        _ => None,
    }
}

#[cfg(test)]
fn caller_terminal_title(args: &[OsString]) -> bool {
    config_values(args).iter().any(|v| title_key(v).is_some())
}

/// Only the effective, unambiguous native ID title opts a process into
/// title-based selection. Later whole-table/custom overrides invalidate it.
#[cfg(test)]
pub(crate) fn selection_title(args: &[OsString]) -> bool {
    selection_title_or(args, false)
}

/// Explicit overrides always win over the native configuration file.
pub(crate) fn selection_title_or(args: &[OsString], configured: bool) -> bool {
    let mut selected = configured;
    for value in config_values(args) {
        if let Some((exact, value)) = title_key(value) {
            selected = exact
                && serde_json::from_str::<Vec<String>>(value)
                    .is_ok_and(|items| items == ["thread-id"]);
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }
    #[test]
    fn an_explicit_resume_id_cannot_be_confused_with_a_prompt_or_option_value() {
        assert!(resuming(
            &args(&["--no-daemon", "resume", "thread"]),
            "thread"
        ));
        assert!(resuming(
            &args(&["resume", "-m", "model", "thread"]),
            "thread"
        ));
        for words in [
            vec!["a prompt", "resume", "thread"],
            vec!["-m", "resume", "thread"],
            vec!["resume", "--last"],
            vec!["--", "resume", "thread"],
            vec!["resume", "other"],
        ] {
            assert!(!resuming(&args(&words), "thread"), "{words:?}");
        }
    }
    #[test]
    fn scopes_only_interactive_invocations() {
        for words in [
            vec![],
            vec!["resume", "--last"],
            vec!["fork", "abc"],
            vec!["-m", "exec", "hello"],
            vec!["-c", "hooks.enabled=true", "resume", "abc"],
            vec!["--", "help"],
        ] {
            assert!(interactive(&args(&words)), "{words:?}");
        }
        for words in [
            vec!["exec", "hello"],
            vec!["-c", "model='x'", "app-server"],
            vec!["resume", "--help"],
            vec!["--version"],
            vec!["--remote", "unix:///tmp/socket"],
            vec!["--remote=ws://host"],
            vec!["help"],
            vec!["queue", "hello"],
            vec!["migrate-rollouts"],
        ] {
            assert!(!interactive(&args(&words)), "{words:?}");
        }
    }
    #[test]
    fn preserves_caller_title_and_recognizes_only_the_effective_native_id() {
        let native = "tui.terminal_title=[\"thread-id\"]";
        for words in [
            vec!["-c", native],
            vec!["--config=tui.terminal_title=[]"],
            vec!["-ctui.terminal_title=[]"],
            vec!["--config", " 'tui' = {}"],
        ] {
            assert!(caller_terminal_title(&args(&words)), "{words:?}");
        }
        assert!(!caller_terminal_title(&args(&["--model", "-c", native])));
        assert!(!caller_terminal_title(&args(&["--", "-c", native])));
        assert!(selection_title(&args(&["-c", native])));
        assert!(selection_title(&args(&[
            "--config=tui.terminal_title=[\"thread-id\"]"
        ])));
        assert!(!selection_title(&args(&[
            "-c",
            native,
            "-c",
            "tui.terminal_title=[]"
        ])));
        assert!(!selection_title(&args(&["-c", native, "-c", "tui={}"])));
        assert!(!selection_title(&args(&["--", "-c", native])));
        assert!(!selection_title(&args(&["-m", "-c", native])));
        assert!(selection_title(&args(&["-c", "tui={}", "-c", native])));
    }
}
