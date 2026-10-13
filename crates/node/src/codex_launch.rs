//! A Codex launcher inherited only by terminals hosted by khor. Its
//! private PATH entry does not touch Codex's home, config or a user's
//! shell files. The real process replaces the launcher on Unix.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

pub const SHIM_ENV: &str = "KHOR_CODEX_SHIM_DIR";
const SHIM_NAME: &str = if cfg!(windows) { "codex.exe" } else { "codex" };

pub struct Scope {
    pub path: OsString,
    pub dir: PathBuf,
    pub root: PathBuf,
}

/// `khor` is the existing `.khor` directory, not the user's home.
pub fn install(khor: &Path, path: &OsStr) -> Result<Scope, String> {
    let root = khor.parent().ok_or("khor terminal has no store root")?;
    let dir = khor.join("codex-bin");
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    install_at(&dir, &exe).map_err(|e| format!("Cannot install khor's Codex launcher: {e}"))?;
    let path = scoped_path(&dir, path)?;
    Ok(Scope {
        path,
        dir,
        root: root.to_owned(),
    })
}

fn scoped_path(dir: &Path, path: &OsStr) -> Result<OsString, String> {
    std::env::join_paths(
        std::iter::once(dir.to_owned()).chain(std::env::split_paths(path).filter(|p| p != dir)),
    )
    .map_err(|e| e.to_string())
}

fn install_at(dir: &Path, exe: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let shim = dir.join(SHIM_NAME);
    #[cfg(unix)]
    if std::fs::read_link(&shim).ok().as_deref() == Some(exe) {
        return Ok(());
    }
    #[cfg(windows)]
    if let (Ok(a), Ok(b)) = (std::fs::metadata(&shim), std::fs::metadata(exe)) {
        if a.len() == b.len() && a.modified().ok() == b.modified().ok() {
            return Ok(());
        }
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = dir.join(format!(".codex-{}-{unique}", std::process::id()));
    #[cfg(unix)]
    std::os::unix::fs::symlink(exe, &temp)?;
    #[cfg(windows)]
    if std::fs::hard_link(exe, &temp).is_err() {
        std::fs::copy(exe, &temp)?;
    }
    #[cfg(windows)]
    if shim.exists() {
        std::fs::remove_file(&shim)?;
    }
    let result = std::fs::rename(&temp, &shim);
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// A value option is skipped before deciding whether a positional word
/// is a subcommand: `-m exec` is a model, not the noninteractive command.
pub(crate) fn interactive(args: &[OsString]) -> bool {
    invocation(args).0
}
pub(crate) fn no_daemon(args: &[OsString]) -> bool {
    invocation(args).1
}

fn invocation(args: &[OsString]) -> (bool, bool) {
    let mut foreground = false;
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
            return (false, foreground);
        }
        if takes_value(&s) {
            value = true;
            continue;
        }
        if s == "--no-daemon" {
            foreground = true;
        }
        if s.starts_with('-') {
            continue;
        }
        if command.is_none() {
            command = Some(s.into_owned());
        }
    }
    (
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
        ),
        foreground,
    )
}

fn real_program(path: &OsStr, shim_dir: &Path) -> Option<PathBuf> {
    let shim = shim_dir.join(SHIM_NAME);
    let own = std::fs::canonicalize(&shim).ok();
    for dir in std::env::split_paths(path).filter(|d| d != shim_dir) {
        for name in if cfg!(windows) {
            &["codex.exe", "codex.cmd", "codex.bat", "codex"][..]
        } else {
            &["codex"][..]
        } {
            let candidate = dir.join(name);
            if own.is_some() && std::fs::canonicalize(&candidate).ok() == own {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&candidate) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            return Some(candidate);
        }
    }
    None
}

/// Every executable that can host a terminal calls this before ordinary
/// dispatch. A normal khor invocation, or an external terminal, is untouched.
pub fn main_if_shim(root: &Path) {
    let Some(dir) = std::env::var_os(SHIM_ENV).map(PathBuf::from) else {
        return;
    };
    let Some(arg0) = std::env::args_os().next() else {
        return;
    };
    if Path::new(&arg0)
        .file_name()
        .and_then(OsStr::to_str)
        .is_none_or(|n| !n.eq_ignore_ascii_case(SHIM_NAME))
    {
        return;
    }
    let outcome = launch(root, &dir, std::env::args_os().skip(1).collect());
    match outcome {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(127);
        }
    }
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

/// Older installations still use the legacy adaptor. Ask the executable
/// itself before adding a flag they cannot parse; help exits before config,
/// authentication or a model session is loaded. Failure is a passthrough.
fn supports_no_daemon(program: &Path) -> bool {
    let Ok(output) = std::process::Command::new(program)
        .arg("--help")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
    else {
        return false;
    };
    output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.split_whitespace().next() == Some("--no-daemon"))
}

fn launch(root: &Path, dir: &Path, args: Vec<OsString>) -> Result<i32, String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let real =
        real_program(&path, dir).ok_or("Codex was not found outside khor's terminal launcher")?;
    let scoped = interactive(&args) && supports_no_daemon(&real);
    let mut command = std::process::Command::new(real);
    if scoped {
        // The flag is deliberately before the user's `--` prompt separator.
        // Ignore occurrences used as option values or prompt text.
        if !no_daemon(&args) {
            command.arg("--no-daemon");
        }
        if !caller_terminal_title(&args) {
            command.args(["-c", "tui.terminal_title=[\"thread-id\"]"]);
        }
        // A new process must never inherit the previous Codex pane's title
        // before its own first native title arrives (including /resume).
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(b"\x1b]2;\x07")
            .and_then(|_| stdout.flush())
            .map_err(|e| e.to_string())?;
    }
    command.args(args).env("KHOR_HOME", root);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec().to_string())
    }
    #[cfg(not(unix))]
    {
        command
            .status()
            .map(|s| s.code().unwrap_or(1))
            .map_err(|e| e.to_string())
    }
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
    fn foreground_flag_must_be_an_option_not_a_value_or_prompt() {
        assert!(no_daemon(&args(&["resume", "--no-daemon", "last"])));
        assert!(!no_daemon(&args(&["--", "--no-daemon"])));
        assert!(!no_daemon(&args(&["--model", "--no-daemon"])));
        assert!(!no_daemon(&args(&["-c", "--no-daemon"])));
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
    #[test]
    fn scoped_path_deduplicates_and_lookup_skips_itself() {
        let dir = std::env::temp_dir().join(format!("khor-codex-launch-{}", std::process::id()));
        let shim = dir.join("shim");
        let real = dir.join("real");
        std::fs::create_dir_all(&real).unwrap();
        install_at(&shim, &std::env::current_exe().unwrap()).unwrap();
        let target = real.join(SHIM_NAME);
        std::fs::write(&target, b"fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let path = std::env::join_paths([shim.clone(), real]).unwrap();
        let once = scoped_path(&shim, &path).unwrap();
        assert_eq!(
            std::env::split_paths(&once).filter(|p| p == &shim).count(),
            1
        );
        assert_eq!(real_program(&once, &shim), Some(target));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
