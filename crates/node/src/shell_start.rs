//! A command typed into a freshly-created shell. The target chooses its own
//! shell and home; the host queues the command once, before any face attaches.
use serde::{Deserialize, Serialize};
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
}
