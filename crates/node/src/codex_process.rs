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

/// Extract CLI config overrides without mistaking option values or prompt
/// text for configuration. Every reader of native title selection uses this.
fn config_values(args: &[OsString]) -> Vec<&str> {
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
pub(crate) fn selection_title(args: &[OsString]) -> bool {
    let mut selected = false;
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
