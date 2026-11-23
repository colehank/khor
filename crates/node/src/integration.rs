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
//! - PowerShell loads its native Profiles before `-NoExit -Command` loads
//!   `khor.ps1`; cmd keeps its native AutoRun and reads `PROMPT` from its environment.
//!
//! `KHOR_SHELL_INTEGRATION=0` in the environment khor runs in turns it off.

use std::path::{Path, PathBuf};

const FILES: &[(&str, &str)] = &[
    ("zsh/.zshenv", include_str!("../shell/zsh/.zshenv")),
    ("khor.zsh", include_str!("../shell/khor.zsh")),
    ("khor.bash", include_str!("../shell/khor.bash")),
    ("khor.bash-login", include_str!("../shell/khor.bash-login")),
    ("khor.sh", include_str!("../shell/khor.sh")),
    (
        "fish-data/fish/vendor_conf.d/khor.fish",
        include_str!("../shell/khor.fish"),
    ),
    ("khor.ps1", include_str!("../shell/khor.ps1")),
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
    let program = Path::new(cmd.first()?)
        .file_name()?
        .to_str()?
        .to_ascii_lowercase();
    let program = program.strip_suffix(".exe").unwrap_or(&program);
    let args = &cmd[1..];
    let native_dir = crate::shell_start::native_shell_path(Path::new(&cmd[0]), dir);
    let dir = native_dir.as_ref();
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
        "bash" | "-bash"
            if args
                .iter()
                .all(|a| matches!(a.as_str(), "-i" | "-l" | "--login"))
                && args.iter().any(|a| a == "-l" || a == "--login") =>
        {
            let boot = dir
                .join("khor.bash-login")
                .display()
                .to_string()
                .replace('\'', "'\\''");
            let mut env = vec![
                ("PROMPT_COMMAND".into(), format!("builtin source '{boot}'")),
                (
                    "KHOR_BASH_INTEGRATION".into(),
                    dir.join("khor.bash").display().to_string(),
                ),
            ];
            if let Some(original) = getenv("PROMPT_COMMAND") {
                env.push(("KHOR_ORIG_PROMPT_COMMAND".into(), original));
            }
            Some(Plan {
                args: vec![
                    "--login".into(),
                    "--noprofile".into(),
                    "--norc".into(),
                    "-i".into(),
                ],
                env,
            })
        }
        "bash" | "-bash" if args.iter().all(|a| a == "-i") => Some(Plan {
            args: std::iter::once("--init-file".into())
                .chain(std::iter::once(format!("{d}/khor.bash")))
                .chain(args.iter().cloned())
                .collect(),
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
        "sh" | "dash" | "ksh" if args.iter().all(|a| matches!(a.as_str(), "-i" | "-l")) => {
            let mut env = vec![("ENV".into(), dir.join("khor.sh").display().to_string())];
            if let Some(original) = getenv("ENV") {
                env.push(("KHOR_ORIG_ENV".into(), original));
            }
            Some(Plan {
                args: args.to_vec(),
                env,
            })
        }
        "pwsh" | "powershell" if args.is_empty() => Some(Plan {
            args: vec![
                "-NoLogo".into(),
                "-NoExit".into(),
                "-Command".into(),
                format!(
                    "try {{ . '{}' }} finally {{ [Console]::Write([string][char]27 + ']133;B' + [char]7) }}",
                    dir.join("khor.ps1")
                        .display()
                        .to_string()
                        .replace('\'', "''")
                ),
            ],
            env: Vec::new(),
        }),
        "cmd" if args.is_empty() => {
            // CMD expands this prompt itself. Keep file paths and quoting out
            // of its /k parser, including when the store is under a Unicode home.
            let original = getenv("PROMPT")
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "$P$G".into());
            let end = r"$e]133;B$e\";
            let prompt = if original.ends_with(end) {
                original
            } else {
                format!(r"$e]133;D$e\$e]133;A$e\$e]9;9;$P$e\{original}{end}")
            };
            Some(Plan {
                args: Vec::new(),
                env: vec![("PROMPT".into(), prompt)],
            })
        }
        _ => None,
    }
}

/// Known integrations finish their first prompt with OSC 133;B. The native
/// login Bash bootstrap sources its login files exactly once in that shell.
pub(crate) fn marks_prompt(cmd: &[String]) -> bool {
    let Some(program) = cmd
        .first()
        .and_then(|s| Path::new(s).file_name())
        .and_then(|s| s.to_str())
    else {
        return false;
    };
    let program = program.to_ascii_lowercase();
    matches!(
        program.trim_start_matches('-').trim_end_matches(".exe"),
        "zsh" | "bash" | "fish" | "sh" | "dash" | "ksh" | "pwsh" | "powershell" | "cmd"
    )
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
    fn native_profiles_and_explicit_arguments_survive_integration() {
        let d = Path::new("/k/shell's dir");
        let ps = plan(d, &["pwsh.exe".into()], env(&[])).unwrap();
        assert_eq!(&ps.args[..3], ["-NoLogo", "-NoExit", "-Command"]);
        assert!(ps.args[3].contains("shell''s dir"));
        assert!(ps.args[3].contains("finally"));
        assert!(!ps.args.iter().any(|a| a == "-NoProfile"));
        let cmd = plan(d, &["cmd.exe".into()], env(&[])).unwrap();
        assert!(cmd.args.is_empty());
        assert_eq!(
            cmd.env,
            [(
                "PROMPT".into(),
                r"$e]133;D$e\$e]133;A$e\$e]9;9;$P$e\$P$G$e]133;B$e\".into()
            )]
        );
        let bash = plan(d, &["/bin/bash".into(), "-l".into(), "-i".into()], env(&[])).unwrap();
        assert_eq!(bash.args, ["--login", "--noprofile", "--norc", "-i"]);
        assert!(
            bash.env
                .iter()
                .any(|(k, v)| k == "PROMPT_COMMAND" && v.contains("khor.bash-login"))
        );
        assert_eq!(
            plan(
                d,
                &["pwsh.exe".into(), "-File".into(), "script.ps1".into()],
                env(&[])
            ),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_hooks_use_local_disk_paths() {
        let dir = Path::new(r"\\?\C:\Users\中文 user's dir\.khor\shell");
        let ps = plan(dir, &["powershell.exe".into()], env(&[])).unwrap();
        assert!(ps.args[3].contains(r"C:\Users\中文 user''s dir\.khor\shell\khor.ps1"));
        assert!(!ps.args[3].contains(r"\\?\"));
        assert_eq!(&ps.args[..3], ["-NoLogo", "-NoExit", "-Command"]);
        let unc = Path::new(r"\\?\UNC\server\share\shell");
        let ps = plan(unc, &["pwsh.exe".into()], env(&[])).unwrap();
        assert!(ps.args[3].contains(r"\\?\UNC\server\share\shell\khor.ps1"));
    }

    #[test]
    fn cmd_uses_its_native_prompt_without_rewrapping_an_inherited_integration() {
        let dir = Path::new("/ignored/shell");
        let command = ["cmd.exe".into()];
        let first = plan(dir, &command, env(&[("PROMPT", "Custom $P$G")])).unwrap();
        assert!(first.args.is_empty());
        assert_eq!(
            first.env[0].1,
            r"$e]133;D$e\$e]133;A$e\$e]9;9;$P$e\Custom $P$G$e]133;B$e\"
        );
        assert_eq!(
            plan(dir, &command, env(&[("PROMPT", &first.env[0].1)])),
            Some(first)
        );
    }

    #[cfg(windows)]
    #[test]
    fn cmd_accepts_its_integrated_prompt_and_input_through_conpty() {
        use portable_pty::{CommandBuilder, PtySize, native_pty_system};
        use std::io::{Read, Write};
        use std::time::{Duration, Instant};

        let root = crate::testdir::TempTree::fresh("cmd-native-prompt");
        let cwd = root.join("Khor 验收 user's %20 dir");
        std::fs::create_dir(&cwd).unwrap();
        let executable = crate::shell_start::WindowsShell::Cmd.resolve().unwrap();
        let plan = plan(&cwd, &[executable.clone()], env(&[("PROMPT", "Khor $P$G")])).unwrap();
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut command = CommandBuilder::new(executable);
        command.cwd(&cwd);
        command.args(plan.args);
        for (name, value) in plan.env {
            command.env(name, value);
        }
        let mut child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut chunk = [0; 4096];
            while let Ok(size) = reader.read(&mut chunk) {
                if size == 0 || tx.send(chunk[..size].to_vec()).is_err() {
                    break;
                }
            }
        });

        let mut output = Vec::new();
        let mut prompt = crate::shell_start::PromptReady::default();
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut typed = false;
        let expected = b"KHOR_CMD_NATIVE_OK";
        while let Ok(chunk) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            output.extend_from_slice(&chunk);
            if !typed && prompt.feed(&chunk) {
                if writer.write_all(b"echo KHOR_CMD_NATIVE^_OK\r").is_err() {
                    break;
                }
                typed = true;
            }
            if output.windows(expected.len()).any(|word| word == expected) {
                break;
            }
        }
        let _ = child.kill();
        assert!(
            typed && output.windows(expected.len()).any(|word| word == expected),
            "CMD did not render the native prompt and echo the command: {:?}",
            String::from_utf8_lossy(&output)
        );
        let mut ledger = crate::commands::Ledger::default();
        ledger.feed(&output, 0);
        let doing = ledger.doing();
        let reported = std::fs::canonicalize(doing.cwd.expect("CMD working directory mark"));
        assert_eq!(reported.unwrap(), cwd.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn login_profile_answers_and_theme_widgets_precede_automatic_startup() {
        use portable_pty::{CommandBuilder, PtySize, native_pty_system};
        use std::io::Read;
        use std::time::{Duration, Instant};
        for shell in ["/bin/zsh", "/bin/bash"] {
            if !Path::new(shell).exists() {
                continue;
            }
            let root = crate::testdir::TempTree::fresh("shell-login-profile");
            let home = root.join("home");
            let scripts = root.join("integration");
            std::fs::create_dir_all(&home).unwrap();
            std::fs::write(
                home.join(".zprofile"),
                "export KHOR_PROFILE_TEST=profile-loaded\n",
            )
            .unwrap();
            std::fs::write(
                home.join(".zshrc"),
                r#"
builtin printf 'RC-QUESTION [Y/n]'
read REPLY
[[ "$REPLY" == n ]] || exit 19
export KHOR_RC_TEST=rc-loaded
function zle-line-init() { builtin printf 'USER-ZLE-INIT'; }
zle -N zle-line-init
function khor_test_theme_line_init() { builtin printf 'THEME-ZLE-INIT'; }
function khor_test_theme_precmd() {
  PS1='profile-theme> '
  precmd_functions=(${precmd_functions:#khor_test_theme_precmd} khor_test_theme_precmd)
  builtin autoload -Uz add-zle-hook-widget
  add-zle-hook-widget line-init khor_test_theme_line_init
}
precmd_functions+=(khor_test_theme_precmd)
"#,
            )
            .unwrap();
            std::fs::write(home.join(".bash_profile"), "shopt -q login_shell || exit 9\nexport KHOR_PROFILE_TEST=profile-loaded\n. \"$HOME/.bashrc\"\n").unwrap();
            std::fs::write(home.join(".bashrc"), "printf 'RC-QUESTION [Y/n]'\nread -r reply\n[ \"$reply\" = n ] || exit 19\nexport KHOR_RC_TEST=rc-loaded\n").unwrap();
            write_scripts(&scripts).unwrap();
            let cmd = vec![shell.into(), "-l".into(), "-i".into()];
            let plan = plan(&scripts, &cmd, env(&[])).unwrap();
            let pair = native_pty_system()
                .openpty(PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .unwrap();
            let mut builder = CommandBuilder::new(shell);
            builder.args(plan.args);
            builder.env("HOME", home.as_os_str());
            builder.env("PATH", "/usr/bin:/bin");
            builder.env("TERM", "xterm-256color");
            builder.env_remove("ZDOTDIR");
            for (key, value) in plan.env {
                builder.env(key, value);
            }
            let mut child = pair.slave.spawn_command(builder).unwrap();
            drop(pair.slave);
            let mut reader = pair.master.try_clone_reader().unwrap();
            let mut writer = crate::shell_start::StartupInput::with_shell_readiness(
                pair.master.take_writer().unwrap(),
                Some("printf 'PROFILE-RESULT:%s:%s\\n' \"$KHOR_PROFILE_TEST\" \"$KHOR_RC_TEST\""),
                true,
            );
            writer.ready().unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            let reading = std::thread::spawn(move || {
                let mut buffer = [0; 4096];
                while let Ok(size) = reader.read(&mut buffer) {
                    if size == 0 || tx.send(buffer[..size].to_vec()).is_err() {
                        break;
                    }
                }
            });
            let result: Result<(), String> = (|| {
                let deadline = Instant::now() + Duration::from_secs(5);
                let mut prompt = crate::shell_start::PromptReady::default();
                let mut output = Vec::new();
                let mut answered = false;
                let mut prompts = 0;
                while Instant::now() < deadline {
                    let bytes = rx
                        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                        .map_err(|e| e.to_string())?;
                    output.extend_from_slice(&bytes);
                    if !answered
                        && output
                            .windows(b"RC-QUESTION [Y/n]".len())
                            .any(|w| w == b"RC-QUESTION [Y/n]")
                    {
                        if writer.readiness() != (false, false) {
                            return Err("automatic startup bypassed the profile question".into());
                        }
                        writer
                            .write_classified(b"n\r", b"n\r", b"")
                            .map_err(|e| e.to_string())?;
                        answered = true;
                    }
                    if prompt.feed(&bytes) {
                        prompt.take_ready();
                        prompts += 1;
                        writer.shell_ready().map_err(|e| e.to_string())?;
                    }
                    let result = b"PROFILE-RESULT:profile-loaded:rc-loaded";
                    let results = output
                        .windows(result.len())
                        .filter(|w| *w == result)
                        .count();
                    if results > 1 {
                        return Err("automatic startup ran more than once".into());
                    }
                    // A second editor entry must announce readiness too. The
                    // first B can be invalidated by explicit profile input.
                    if results == 1 && prompts >= 2 {
                        if !answered {
                            return Err("the profile question was never answered".into());
                        }
                        if shell.ends_with("zsh") {
                            for widget in [b"USER-ZLE-INIT".as_slice(), b"THEME-ZLE-INIT"] {
                                if !output.windows(widget.len()).any(|w| w == widget) {
                                    return Err(
                                        "readiness replaced an existing line-init widget".into()
                                    );
                                }
                            }
                        }
                        return Ok(());
                    }
                }
                Err("startup input did not see login and interactive configuration".into())
            })();
            let _ = child.kill();
            let _ = child.wait();
            drop(writer);
            drop(pair.master);
            drop(rx);
            let _ = reading.join();
            drop(root);
            assert!(result.is_ok(), "{result:?}");
        }
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

    #[cfg(unix)]
    #[test]
    fn posix_readiness_marks_survive_long_readline_prompts() {
        use portable_pty::{CommandBuilder, PtySize, native_pty_system};
        use std::io::Read;
        use std::time::{Duration, Instant};
        for shell in ["/bin/sh", "/bin/dash"] {
            if !Path::new(shell).exists() {
                continue;
            }
            for width in [74, 77] {
                let root = crate::testdir::TempTree::fresh("posix-long-prompt");
                let home = root.join("home");
                let scripts = root.join("integration");
                std::fs::create_dir(&home).unwrap();
                let prompt = format!("{}$ ", "x".repeat(width - 2));
                std::fs::write(home.join(".profile"), format!("PS1='{prompt}'\n")).unwrap();
                write_scripts(&scripts).unwrap();
                let plan = plan(
                    &scripts,
                    &[shell.into(), "-l".into(), "-i".into()],
                    env(&[]),
                )
                .unwrap();
                let pair = native_pty_system()
                    .openpty(PtySize {
                        rows: 24,
                        cols: 80,
                        pixel_width: 0,
                        pixel_height: 0,
                    })
                    .unwrap();
                let mut builder = CommandBuilder::new(shell);
                builder.args(plan.args);
                builder.env("HOME", home.as_os_str());
                builder.env("PATH", "/usr/bin:/bin");
                builder.env("TERM", "xterm-256color");
                builder.env_remove("BASH_VERSION");
                builder.env_remove("ENV");
                for (key, value) in plan.env {
                    builder.env(key, value);
                }
                let mut child = pair.slave.spawn_command(builder).unwrap();
                drop(pair.slave);
                let mut reader = pair.master.try_clone_reader().unwrap();
                let mut writer = crate::shell_start::StartupInput::with_shell_readiness(
                    pair.master.take_writer().unwrap(),
                    Some("printf 'PROMPT-%s-%s\\n' READY RESULT"),
                    true,
                );
                writer.ready().unwrap();
                let (tx, rx) = std::sync::mpsc::channel();
                let reading = std::thread::spawn(move || {
                    let mut buffer = [0; 4096];
                    while let Ok(size) = reader.read(&mut buffer) {
                        if size == 0 || tx.send(buffer[..size].to_vec()).is_err() {
                            break;
                        }
                    }
                });
                let result: Result<(), String> = (|| {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut readiness = crate::shell_start::PromptReady::default();
                    let mut output = Vec::new();
                    let mut prompts = 0;
                    loop {
                        let bytes = rx
                            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                            .map_err(|error| error.to_string())?;
                        output.extend_from_slice(&bytes);
                        if readiness.feed(&bytes) {
                            readiness.take_ready();
                            prompts += 1;
                            writer.shell_ready().map_err(|error| error.to_string())?;
                        }
                        let marker = b"PROMPT-READY-RESULT";
                        let results = output
                            .windows(marker.len())
                            .filter(|w| *w == marker)
                            .count();
                        if results > 1 {
                            return Err("automatic startup repeated after prompt redraw".into());
                        }
                        if results == 1 && prompts >= 2 {
                            if !output.windows(prompt.len()).any(|w| w == prompt.as_bytes()) {
                                return Err(
                                    "the test shell did not load the long profile prompt".into()
                                );
                            }
                            if output.windows(2).any(|w| w == b"\\[" || w == b"\\]") {
                                return Err("readline delimiters leaked into the terminal".into());
                            }
                            return Ok(());
                        }
                    }
                })();
                let _ = child.kill();
                let _ = child.wait();
                drop(writer);
                drop(pair.master);
                reading.join().unwrap();
                assert!(result.is_ok(), "{shell} width={width}: {result:?}");
            }
        }
    }
}
