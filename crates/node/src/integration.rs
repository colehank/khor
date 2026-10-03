//! **Shell integration: how a hosted shell is taught to mark its commands**
//! (roadmap 1.1) without touching the person's own startup files.
//!
//! The scripts live in `crates/node/shell/` and are written, as this build
//! has them, under the khor home's `shell/` whenever a host starts a shell
//! (rewritten only when they differ). Each shell loads them its own way,
//! following what VS Code and Kitty do:
//!
//! - **zsh**: `ZDOTDIR` points at khor's directory, whose `.zshenv` puts the
//!   person's `ZDOTDIR` back first thing and sources their `.zshenv`
//!   (Kitty's `shell-integration/zsh/.zshenv`); the hooks go in at the
//!   first prompt, after their `.zshrc`.
//! - **bash**: `bash --init-file khor.bash`, which reads `~/.bashrc` as bash
//!   would have and then adds the hooks (VS Code's
//!   `shellIntegration-bash.sh`). Only for a plain interactive bash: one
//!   started with arguments is left as asked.
//! - **fish**: khor's directory first in `XDG_DATA_DIRS`, so fish reads
//!   `fish/vendor_conf.d/khor.fish`, which restores the variable (Kitty's
//!   way).
//! - PowerShell (`khor.ps1`) and cmd (`khor.cmd`) are written but nothing
//!   starts them yet: khor does not host on Windows (docs/SHELL-INTEGRATION.md).
//!
//! `KHOR_SHELL_INTEGRATION=0` in the environment khor runs in turns it off.

use std::path::{Path, PathBuf};

const FILES: &[(&str, &str)] = &[
    ("zsh/.zshenv", include_str!("../shell/zsh/.zshenv")),
    ("khor.zsh", include_str!("../shell/khor.zsh")),
    ("khor.bash", include_str!("../shell/khor.bash")),
    (
        "fish-data/fish/vendor_conf.d/khor.fish",
        include_str!("../shell/khor.fish"),
    ),
    ("khor.ps1", include_str!("../shell/khor.ps1")),
    ("khor.cmd", include_str!("../shell/khor.cmd")),
];

/// How to start a shell so it loads khor's integration.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// The arguments after the program.
    pub args: Vec<String>,
    /// Environment to set for the shell.
    pub env: Vec<(String, String)>,
}

/// Where the scripts are written: `<khor home>/shell`.
pub fn dir(khor: &Path) -> PathBuf {
    khor.join("shell")
}

/// Writes the scripts this build carries under `dir`, each only when it
/// differs from what is there (through a temporary file, so a shell
/// starting meanwhile never reads half of one).
pub fn write_scripts(dir: &Path) -> std::io::Result<()> {
    for (name, text) in FILES {
        let path = dir.join(name);
        if std::fs::read(&path).ok().as_deref() == Some(text.as_bytes()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &path)?;
    }
    Ok(())
}

/// How `cmd` should start to load the scripts in `dir` — `None` for a
/// program that is not a shell khor integrates, a shell started with
/// arguments it would not understand, or integration turned off.
/// `getenv` reads the environment the host itself runs in.
pub fn plan(dir: &Path, cmd: &[String], getenv: impl Fn(&str) -> Option<String>) -> Option<Plan> {
    if getenv("KHOR_SHELL_INTEGRATION").as_deref() == Some("0") {
        return None;
    }
    let program = Path::new(cmd.first()?).file_name()?.to_str()?;
    let args = &cmd[1..];
    let d = dir.display().to_string();
    match program {
        "zsh" | "-zsh" => Some(Plan {
            args: args.to_vec(),
            env: vec![
                ("ZDOTDIR".into(), format!("{d}/zsh")),
                (
                    "KHOR_ORIG_ZDOTDIR".into(),
                    getenv("ZDOTDIR").unwrap_or_default(),
                ),
                ("KHOR_SHELL_DIR".into(), d),
            ],
        }),
        "bash" | "-bash" if args.iter().all(|a| a == "-i") => Some(Plan {
            args: vec!["--init-file".into(), format!("{d}/khor.bash")],
            env: Vec::new(),
        }),
        "fish" | "-fish" => {
            let orig = getenv("XDG_DATA_DIRS");
            let rest = orig
                .clone()
                .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
            Some(Plan {
                args: args.to_vec(),
                env: vec![
                    ("XDG_DATA_DIRS".into(), format!("{d}/fish-data:{rest}")),
                    ("KHOR_ORIG_XDG_DATA_DIRS".into(), orig.unwrap_or_default()),
                ],
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| pairs.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn each_shell_is_started_its_own_way_and_others_are_left_alone() {
        let d = Path::new("/k/shell");
        let zsh = plan(d, &["/bin/zsh".into()], env(&[("ZDOTDIR", "/me/z")])).unwrap();
        assert!(zsh.env.contains(&("ZDOTDIR".into(), "/k/shell/zsh".into())));
        assert!(
            zsh.env
                .contains(&("KHOR_ORIG_ZDOTDIR".into(), "/me/z".into()))
        );
        let bash = plan(d, &["/bin/bash".into()], env(&[])).unwrap();
        assert_eq!(bash.args, vec!["--init-file", "/k/shell/khor.bash"]);
        assert_eq!(
            plan(d, &["/bin/bash".into(), "-c".into(), "x".into()], env(&[])),
            None
        );
        let fish = plan(d, &["/opt/homebrew/bin/fish".into()], env(&[])).unwrap();
        assert!(fish.env.contains(&(
            "XDG_DATA_DIRS".into(),
            "/k/shell/fish-data:/usr/local/share:/usr/share".into()
        )));
        assert_eq!(plan(d, &["/usr/bin/vim".into()], env(&[])), None);
        assert_eq!(
            plan(
                d,
                &["/bin/zsh".into()],
                env(&[("KHOR_SHELL_INTEGRATION", "0")])
            ),
            None
        );
    }

    #[test]
    fn the_scripts_are_written_once_and_kept() {
        let d = std::env::temp_dir().join(format!("khor-integ-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        write_scripts(&d).unwrap();
        let zshenv = d.join("zsh/.zshenv");
        let before = std::fs::metadata(&zshenv).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        write_scripts(&d).unwrap();
        assert_eq!(
            std::fs::metadata(&zshenv).unwrap().modified().unwrap(),
            before,
            "unchanged files are not rewritten"
        );
        assert!(d.join("fish-data/fish/vendor_conf.d/khor.fish").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
