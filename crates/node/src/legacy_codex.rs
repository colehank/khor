//! Upgrade compatibility for already-open shells carrying the retired Codex shim.
//! New terminals never install a shim. Old aliases forward verbatim and cannot
//! accidentally enter khor's GUI dispatch when tmux drops the environment hint.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
pub const SHIM_ENV: &str = "KHOR_CODEX_SHIM_DIR";
const SHIM_NAME: &str = if cfg!(windows) { "codex.exe" } else { "codex" };

fn without_dir(path: &OsStr, dir: &Path) -> OsString {
    std::env::join_paths(std::env::split_paths(path).filter(|p| p != dir))
        .unwrap_or_else(|_| path.to_owned())
}

/// Only remove the retired path belonging to this khor store.
pub(crate) fn clean_path(khor: &Path, path: &str) -> String {
    without_dir(OsStr::new(path), &khor.join("codex-bin"))
        .to_string_lossy()
        .into_owned()
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
pub fn main_if_shim() {
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
    // tmux can retain PATH while dropping KHOR_CODEX_SHIM_DIR. The invoked
    // executable is the identity; an inherited environment hint is not.
    // Never fall through to the GUI/CLI when invoked as `codex`.
    let outcome =
        invoked_shim(&arg0).and_then(|dir| launch(&dir, std::env::args_os().skip(1).collect()));
    match outcome {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(127);
        }
    }
}

fn invoked_shim(arg0: &OsStr) -> Result<PathBuf, String> {
    let own = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|e| format!("Cannot identify khor's Codex launcher: {e}"))?;
    let path = Path::new(arg0);
    let mut candidates = vec![path.to_owned()];
    if path.parent().is_none_or(|p| p.as_os_str().is_empty()) {
        candidates = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join(path))
            .collect();
    }
    if let Some(dir) = std::env::var_os(SHIM_ENV) {
        candidates.push(PathBuf::from(dir).join(SHIM_NAME));
    }
    for candidate in candidates {
        if std::fs::canonicalize(&candidate).ok().as_ref() == Some(&own) {
            // Resolve the directory, not the file symlink into khor-gui.
            return std::fs::canonicalize(candidate.parent().unwrap_or(Path::new(".")))
                .map_err(|e| format!("Cannot locate khor's Codex launcher: {e}"));
        }
    }
    Err("Cannot locate khor's Codex launcher in PATH".into())
}

fn launch(dir: &Path, args: Vec<OsString>) -> Result<i32, String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let real =
        real_program(&path, dir).ok_or("Codex was not found outside khor's terminal launcher")?;
    let mut command = std::process::Command::new(real);
    // A compatibility path for shells opened before the launcher was retired.
    // Preserve argv, cwd, stdio and Codex's daemon/config defaults.
    command
        .args(args)
        .env("PATH", without_dir(&path, dir))
        .env_remove(SHIM_ENV);
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

    #[test]
    fn new_terminals_remove_only_their_retired_path() {
        let root = std::env::temp_dir().join("khor-path-retirement/.khor");
        let ordinary = std::env::temp_dir().join("user-owned/codex-bin");
        let retired = root.join("codex-bin");
        let path = std::env::join_paths([&ordinary, &retired, &ordinary]).unwrap();
        let clean = clean_path(&root, &path.to_string_lossy());
        assert_eq!(
            std::env::split_paths(&clean).collect::<Vec<_>>(),
            vec![ordinary.clone(), ordinary]
        );
        assert!(
            !root.exists(),
            "cleaning PATH never installs a new launcher"
        );
    }
}
