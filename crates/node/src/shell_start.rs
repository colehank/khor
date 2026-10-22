//! A command typed into a freshly-created shell. The target chooses its own
//! shell and home; the host holds the command until a terminal can answer it.
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellStart {
    pub token: String,
    pub title: String,
    pub cwd: String,
    pub command: String,
    pub cols: u16,
    pub rows: u16,
}

impl ShellStart {
    pub fn validate(&self) -> Result<(), String> {
        if !crate::once::valid(&self.token) {
            return Err(khor_catalog::msg::create_token_bad().into());
        }
        if self.command.len() > 32 * 1024 || self.command.contains('\0') {
            return Err(khor_catalog::msg::launch_command_invalid().into());
        }
        Ok(())
    }
}

pub fn default_shell() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}

pub fn directory(typed: &str) -> Result<PathBuf, String> {
    let home = || std::env::home_dir().ok_or_else(|| khor_catalog::msg::no_such_dir("~"));
    let path = match typed.trim() {
        "" | "~" => home()?,
        d if d.starts_with("~/") || d.starts_with("~\\") => home()?.join(&d[2..]),
        d => PathBuf::from(d),
    };
    if !path.is_dir() {
        return Err(khor_catalog::msg::no_such_dir(path.display()));
    }
    Ok(path)
}

/// Terminal input, not a shell argument or a shell script wrapper. Definitions,
/// exported variables and cd therefore stay in this very interactive shell.
pub fn input(command: &str) -> Vec<u8> {
    if command.trim().is_empty() {
        return Vec::new();
    }
    let mut bytes = command
        .replace("\r\n", "\n")
        .replace('\n', "\r")
        .into_bytes();
    if !bytes.ends_with(b"\r") {
        bytes.push(b'\r');
    }
    bytes
}

/// One host owns the startup, even when several viewers attach or retry.
/// Early keys follow it; ordinary hosts without startup input write directly.
pub(crate) struct StartupInput<W> {
    writer: W,
    pending: Option<Vec<u8>>,
}

impl<W: Write> StartupInput<W> {
    pub(crate) fn new(writer: W, startup: Option<&str>) -> Self {
        Self { writer, pending: startup.map(input).filter(|bytes| !bytes.is_empty()) }
    }

    pub(crate) fn ready(&mut self) -> io::Result<()> {
        if let Some(bytes) = self.pending.take() {
            self.writer.write_all(&bytes)?;
            self.writer.flush()?;
        }
        Ok(())
    }

    /// A shell can already have queried its terminal during startup.
    /// Its answers must not be queued behind a command it has not read.
    pub(crate) fn reply(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }
}

impl<W: Write> Write for StartupInput<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(pending) = &mut self.pending {
            // The host frame budget also bounds keys pasted while opening.
            if pending.len().saturating_add(bytes.len()) > 1 << 20 {
                return Err(io::Error::other("terminal startup input exceeds 1 MiB"));
            }
            pending.extend_from_slice(bytes);
            Ok(bytes.len())
        } else {
            self.writer.write(bytes)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_some() { Ok(()) } else { self.writer.flush() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_is_literal_terminal_input_not_an_argv_rewrite() {
        assert_eq!(
            input("printf '%s' \"$HOME\"\ncd 'a b'"),
            b"printf '%s' \"$HOME\"\rcd 'a b'\r"
        );
        assert_eq!(input("echo a\r\necho b\n"), b"echo a\recho b\r");
        assert!(input(" \n").is_empty());
    }

    #[test]
    fn startup_waits_for_readiness_once_and_precedes_early_keys() {
        let mut input = StartupInput::new(Vec::new(), Some("export CHECK=kept"));
        input.write_all(b"echo $CHECK\r").unwrap();
        input.flush().unwrap();
        assert!(input.writer.is_empty());
        input.ready().unwrap();
        input.ready().unwrap();
        input.write_all(b"later\r").unwrap();
        assert_eq!(input.writer, b"export CHECK=kept\recho $CHECK\rlater\r");
    }

    #[test]
    fn ordinary_and_empty_shells_do_not_wait_for_a_viewer() {
        for startup in [None, Some(" \n")] {
            let mut input = StartupInput::new(Vec::new(), startup);
            input.write_all(b"immediate\r").unwrap();
            assert_eq!(input.writer, b"immediate\r");
        }
    }

    #[test]
    fn early_input_is_bounded() {
        let mut input = StartupInput::new(Vec::new(), Some("command"));
        assert!(input.write_all(&vec![b'x'; 1 << 20]).is_err());
        input.ready().unwrap();
        assert_eq!(input.writer, b"command\r");
    }

    #[test]
    fn shell_replies_do_not_wait_behind_pending_startup() {
        let mut input = StartupInput::new(Vec::new(), Some("command"));
        input.reply(b"answer").unwrap();
        assert_eq!(input.writer, b"answer");
        input.ready().unwrap();
        assert_eq!(input.writer, b"answercommand\r");
    }
}
