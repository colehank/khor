//! A command typed into a freshly-created shell. The target chooses its own
//! shell and home; the host holds the command until a terminal can answer it.
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellStart {
    pub token: String,
    pub title: String,
    pub cwd: String,
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    /// The host supplies terminal replies while no human viewer is attached.
    #[serde(default)]
    pub headless: bool,
    /// Structured CLI arguments are quoted only on the machine owning this shell.
    #[serde(default)]
    pub argv: Option<Vec<String>>,
    /// Optional trailing field. Keep the existing terminal frame byte-for-byte
    /// when absent. A reader that predates the field refuses a longer frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<WindowsShell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowsShell {
    PowerShell,
    Cmd,
}

impl WindowsShell {
    pub(crate) fn from_program(program: &str) -> Option<Self> {
        match program_name(program).as_str() {
            "pwsh" | "powershell" => Some(Self::PowerShell),
            "cmd" => Some(Self::Cmd),
            _ => None,
        }
    }

    /// Render one complete path argument for the shell on the session host.
    pub fn quote_attachment(self, path: &str) -> Result<String, String> {
        let path = path
            .strip_prefix(r"\\?\")
            .filter(|local| {
                let bytes = local.as_bytes();
                bytes.first().is_some_and(u8::is_ascii_alphabetic)
                    && bytes.get(1..3) == Some(b":\\")
            })
            .unwrap_or(path);
        if path.chars().any(char::is_control) {
            return Err("The path contains a character the terminal cannot paste".into());
        }
        match self {
            Self::PowerShell => Ok(quote_powershell(path)),
            Self::Cmd if path.contains(['%', '!', '"']) => {
                Err("CMD cannot safely paste a path containing %, ! or a quotation mark".into())
            }
            Self::Cmd => Ok(quote_cmd(path)),
        }
    }

    pub(crate) fn resolve(self) -> Result<String, String> {
        #[cfg(windows)]
        {
            Ok(match self {
                Self::PowerShell => windows_powershell(crate::Node::user_path())
                    .unwrap_or_else(|| "powershell.exe".into()),
                Self::Cmd => {
                    windows_system_program("System32/cmd.exe").unwrap_or_else(|| "cmd.exe".into())
                }
            })
        }
        #[cfg(not(windows))]
        {
            Err(khor_catalog::msg::launch_command_invalid().into())
        }
    }
}

fn program_name(program: &str) -> String {
    let name = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim_start_matches('-')
        .to_ascii_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_owned()
}

fn quote_powershell(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('\'');
    for ch in value.chars() {
        quoted.push(ch);
        if matches!(ch, '\'' | '\u{2018}'..='\u{201b}') {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

fn quote_cmd(value: &str) -> String {
    // For a Windows executable's argv, double any backslashes immediately
    // before the closing quote so they remain part of the argument.
    let trailing = value.chars().rev().take_while(|c| *c == '\\').count();
    format!("\"{}{}\"", value, "\\".repeat(trailing))
}

#[cfg(windows)]
fn windows_system_program(relative: &str) -> Option<String> {
    let file = PathBuf::from(std::env::var_os("SystemRoot")?).join(relative);
    file.is_file().then(|| file.to_string_lossy().into_owned())
}

#[cfg(windows)]
fn windows_powershell(path: &str) -> Option<String> {
    crate::find_program("pwsh.exe", path)
        .or_else(|| windows_system_program("System32/WindowsPowerShell/v1.0/powershell.exe"))
}

impl ShellStart {
    pub fn validate(&self) -> Result<(), String> {
        if !crate::once::valid(&self.token) {
            return Err(khor_catalog::msg::create_token_bad().into());
        }
        if self.command.len() > 32 * 1024
            || self.command.contains('\0')
            || self.argv.as_ref().is_some_and(|argv| {
                argv.is_empty()
                    || !self.command.trim().is_empty()
                    || argv
                        .iter()
                        .map(|arg| arg.len().saturating_add(1))
                        .sum::<usize>()
                        > 32 * 1024
                    || argv.iter().any(|arg| arg.contains('\0'))
            })
        {
            return Err(khor_catalog::msg::launch_command_invalid().into());
        }
        Ok(())
    }
}

/// The target user's shell. An explicit Khor selection is authoritative;
/// an inherited, stale SHELL is only a candidate, followed by the account.
pub fn default_shell() -> String {
    if let Some(shell) = std::env::var("KHOR_SHELL")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return shell;
    }
    let path = std::env::var("PATH").unwrap_or_default();
    let available = |shell: &str| crate::find_program(shell, &path);
    #[cfg(unix)]
    {
        if let Some(shell) = std::env::var("SHELL").ok().and_then(|s| available(&s)) {
            return shell;
        }
        if let Some(shell) = account_shell().and_then(|s| available(&s)) {
            return shell;
        }
        #[cfg(target_os = "macos")]
        if let Some(shell) = available("/bin/zsh") {
            return shell;
        }
        "/bin/sh".into()
    }
    #[cfg(windows)]
    {
        if let Some(shell) = windows_powershell(&path) {
            return shell;
        }
        std::env::var("COMSPEC")
            .ok()
            .and_then(|s| available(&s))
            .unwrap_or_else(|| "cmd.exe".into())
    }
    #[cfg(not(any(unix, windows)))]
    {
        "/bin/sh".into()
    }
}

#[cfg(unix)]
fn account_shell() -> Option<String> {
    let mut size = 16 * 1024;
    loop {
        let mut buffer = vec![0u8; size];
        // getpwuid_r writes pointers into buffer, which stays alive until the
        // shell has been copied. Unlike getpwuid it has no shared static data.
        let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
        let mut found = std::ptr::null_mut();
        let result = unsafe {
            libc::getpwuid_r(
                libc::geteuid(),
                &mut entry,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut found,
            )
        };
        if result == libc::ERANGE && size < 256 * 1024 {
            size *= 2;
            continue;
        }
        if result != 0 || found.is_null() || entry.pw_shell.is_null() {
            return None;
        }
        return unsafe { std::ffi::CStr::from_ptr(entry.pw_shell) }
            .to_str()
            .ok()
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
    }
}

#[derive(Clone, Copy)]
enum Platform {
    Mac,
    Unix,
    Windows,
}

/// A new interactive terminal, not a command string to be shell-expanded.
/// Login startup is the macOS convention; Linux normally inherits its login
/// session and reads interactive rc files. Native Windows shells own Profiles.
pub fn command(shell: &str) -> Vec<String> {
    let platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Mac
    } else {
        Platform::Unix
    };
    command_for(
        shell,
        platform,
        std::env::var("KHOR_SHELL_LOGIN").ok().as_deref(),
    )
}

fn command_for(shell: &str, platform: Platform, policy: Option<&str>) -> Vec<String> {
    let name = PathBuf::from(shell)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .trim_start_matches('-')
        .to_ascii_lowercase();
    let mut cmd = vec![shell.to_owned()];
    if matches!(platform, Platform::Windows) {
        return cmd;
    }
    let login = match policy {
        Some("always") => true,
        Some("never") => false,
        _ => matches!(platform, Platform::Mac),
    };
    // Other programs and custom shells retain their own argument conventions.
    if matches!(
        name.as_str(),
        "zsh" | "bash" | "fish" | "sh" | "dash" | "ksh"
    ) {
        if login {
            cmd.push("-l".into());
        }
        cmd.push("-i".into());
    }
    cmd
}

/// Literal startup text is unchanged. Structured argv comes from a CLI, so its
/// target shell must quote it before it is typed into the persistent terminal.
pub fn startup_command(
    shell: &str,
    command: &str,
    argv: Option<&[String]>,
) -> Result<String, String> {
    let Some(argv) = argv else {
        return Ok(command.to_owned());
    };
    if argv.is_empty() || !command.trim().is_empty() || argv.iter().any(|arg| arg.contains('\0')) {
        return Err("Invalid structured terminal startup command".into());
    }
    let name = program_name(shell);
    let posix = |arg: &str| format!("'{}'", arg.replace('\'', "'\\''"));
    let rendered = match name.as_str() {
        "pwsh" | "powershell" => format!("& {}", argv.iter().map(|s| quote_powershell(s)).collect::<Vec<_>>().join(" ")),
        "cmd" => {
            if argv.iter().any(|arg| arg.chars().any(|c| "%!^&|<>\r\n\"".contains(c))) {
                return Err("cmd structured startup cannot contain expansion or control characters; provide an explicit shell command instead".into());
            }
            let quoted = argv.iter().enumerate().map(|(at, arg)| {
                if at == 0 && !arg.is_empty() && !arg.chars().any(char::is_whitespace) { return arg.clone(); }
                quote_cmd(arg)
            }).collect::<Vec<_>>();
            quoted.join(" ")
        }
        "zsh" | "bash" | "sh" | "dash" | "ksh" | "fish" => argv.iter().map(|s| posix(s)).collect::<Vec<_>>().join(" "),
        _ => return Err("The selected shell has no structured startup argument adapter; provide a literal shell command".into()),
    };
    if rendered.len() > 32 * 1024 {
        return Err("Quoted terminal startup command exceeds its input budget".into());
    }
    Ok(rendered)
}

pub(crate) fn hosted_command(cmd: &[String]) -> Vec<String> {
    if cmd.len() == 1 {
        command(&cmd[0])
    } else {
        cmd.to_vec()
    }
}

/// Native Windows shell parsers can classify a verbatim local disk path as
/// remote, unlike the filesystem APIs that produced its canonical spelling.
/// Project only paths handed to those shells; durable identities stay intact.
pub(crate) fn native_shell_path<'a>(program: &Path, path: &'a Path) -> Cow<'a, Path> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let native = program
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                let name = name.to_ascii_lowercase();
                matches!(
                    name.as_str(),
                    "cmd" | "cmd.exe" | "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe"
                )
            });
        let mut components = path.components();
        if native
            && let Some(Component::Prefix(prefix)) = components.next()
            && let Prefix::VerbatimDisk(drive) = prefix.kind()
        {
            let mut local = PathBuf::from(format!("{}:", char::from(drive)));
            local.push(components.as_path());
            return Cow::Owned(local);
        }
    }
    #[cfg(not(windows))]
    let _ = program;
    Cow::Borrowed(path)
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
/// Explicit input stays interactive while only the automatic startup waits.
pub(crate) struct StartupInput<W> {
    writer: W,
    pending: Option<Vec<u8>>,
    shell_ready: bool,
    terminal_ready: bool,
    revision: u64,
    user_line: bool,
    startup_line: bool,
    accepted: VecDeque<(String, [u8; 32], Result<u64, String>)>,
    receipt_dir: Option<PathBuf>,
}

impl<W: Write> StartupInput<W> {
    pub(crate) fn new(writer: W, startup: Option<&str>) -> Self {
        Self::with_shell_readiness(writer, startup, false)
    }

    pub(crate) fn with_shell_readiness(writer: W, startup: Option<&str>, wait_shell: bool) -> Self {
        Self {
            writer,
            pending: startup.map(input).filter(|bytes| !bytes.is_empty()),
            shell_ready: !wait_shell,
            terminal_ready: false,
            revision: 0,
            user_line: false,
            startup_line: false,
            accepted: VecDeque::new(),
            receipt_dir: None,
        }
    }

    pub(crate) fn with_receipts(mut self, dir: PathBuf) -> Self {
        self.receipt_dir = Some(dir);
        self
    }

    pub(crate) fn ready(&mut self) -> io::Result<()> {
        self.terminal_ready = true;
        self.release()
    }

    pub(crate) fn shell_ready(&mut self) -> io::Result<()> {
        self.shell_ready = true;
        self.release()
    }

    fn release(&mut self) -> io::Result<()> {
        if self.shell_ready && self.terminal_ready && !self.user_line && !self.startup_line {
            if let Some(bytes) = self.pending.take() {
                self.writer.write_all(&bytes)?;
                self.writer.flush()?;
            }
        }
        Ok(())
    }

    pub(crate) fn readiness(&self) -> (bool, bool) {
        (self.shell_ready, self.pending.is_none())
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// A bounded, idempotent automation input transaction. The observed input
    /// revision and an unfinished human line are checked under the PTY lock.
    #[cfg(test)]
    fn automated_input(&mut self, token: &str, expected: u64, bytes: &[u8]) -> Result<u64, String> {
        self.automated_input_in_generation("test-host", token, expected, bytes)
    }

    pub(crate) fn automated_input_in_generation(
        &mut self,
        generation: &str,
        token: &str,
        expected: u64,
        bytes: &[u8],
    ) -> Result<u64, String> {
        if !crate::once::valid(token) || bytes.len() > 32 * 1024 {
            return Err("Invalid terminal input request".into());
        }
        let mut fingerprint = blake3::Hasher::new();
        fingerprint.update(&(generation.len() as u64).to_be_bytes());
        fingerprint.update(generation.as_bytes());
        fingerprint.update(&expected.to_be_bytes());
        fingerprint.update(bytes);
        let fingerprint = *fingerprint.finalize().as_bytes();
        if let Some((_, prior, result)) = self.accepted.iter().find(|(key, _, _)| key == token) {
            return if prior == &fingerprint {
                result.clone()
            } else {
                Err("Terminal input token was already used for different input".into())
            };
        }
        let receipt = self
            .receipt_dir
            .as_ref()
            .map(|dir| dir.join(format!("{}.json", blake3::hash(token.as_bytes()).to_hex())));
        if let Some(path) = &receipt {
            let stored = std::fs::File::open(path).and_then(|file| {
                let mut bytes = Vec::new();
                file.take(4097).read_to_end(&mut bytes)?;
                Ok(bytes)
            });
            match stored {
                Ok(bytes) if bytes.len() <= 4096 => {
                    let (prior, result): ([u8; 32], Result<u64, String>) =
                        serde_json::from_slice(&bytes).map_err(|_| {
                            "Terminal input receipt is unreadable; its token will not be replayed"
                                .to_owned()
                        })?;
                    return if prior == fingerprint {
                        result
                    } else {
                        Err("Terminal input token was already used for different input".into())
                    };
                }
                Ok(_) => return Err("Terminal input receipt exceeds its budget".into()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        if expected != self.revision {
            return Err(
                "Terminal input changed; read its current state before sending input".into(),
            );
        }
        if self.user_line {
            return Err("The user is editing a terminal line".into());
        }
        let revision = self.revision.wrapping_add(1);
        if let Some(path) = &receipt {
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            let reserved: Result<u64, String> =
                Err("Terminal input outcome is uncertain; its token will not be replayed".into());
            crate::link::write_private(
                path,
                &serde_json::to_vec(&(fingerprint, reserved)).map_err(|e| e.to_string())?,
            )?;
        }
        self.revision = revision;
        self.operator_input(bytes);
        // A write may partially succeed. Reserve its token before the write so
        // retrying an uncertain outcome can never execute the bytes twice.
        self.accepted.push_back((
            token.into(),
            fingerprint,
            Err("Terminal input write did not complete".into()),
        ));
        if self.accepted.len() > 128 {
            self.accepted.pop_front();
        }
        let result = self
            .writer
            .write_all(bytes)
            .and_then(|()| self.writer.flush())
            .map(|()| revision)
            .map_err(|e| e.to_string());
        if result.is_err() && self.pending.is_some() {
            // An uncertain partial write cannot be treated as a completed
            // profile answer or an empty editable line.
            self.startup_line = true;
        }
        self.accepted.back_mut().unwrap().2 = result.clone();
        if let Some(path) = &receipt {
            // The durable reservation remains a non-replayable unknown if
            // writing its final outcome fails after the PTY write.
            crate::link::write_private(
                path,
                &serde_json::to_vec(&(fingerprint, &result)).map_err(|e| e.to_string())?,
            )?;
        }
        result
    }

    pub(crate) fn write_classified(
        &mut self,
        bytes: &[u8],
        keys: &[u8],
        _replies: &[u8],
    ) -> io::Result<()> {
        self.user_input(keys);
        let result = self
            .writer
            .write_all(bytes)
            .and_then(|()| self.writer.flush());
        if result.is_err() && !keys.is_empty() {
            self.user_line = true;
            if self.pending.is_some() {
                self.startup_line = true;
            }
        }
        result
    }

    /// Replies to startup terminal queries bypass commands waiting for readiness.
    pub(crate) fn reply(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    fn user_input(&mut self, bytes: &[u8]) {
        self.operator_input(bytes);
        if !bytes.is_empty() {
            self.revision = self.revision.wrapping_add(1);
        }
        for byte in bytes {
            match byte {
                b'\r' | b'\n' | 3 => self.user_line = false,
                // Completion, history search, cursor motion and editing may
                // change text without a printable byte. Replies were separated
                // by the answer ledger before reaching this method.
                _ => self.user_line = true,
            }
        }
    }

    fn operator_input(&mut self, bytes: &[u8]) {
        if self.pending.is_some() && !bytes.is_empty() {
            // Profile questions and authentication prompts accept explicit
            // input immediately. Automatic input must wait for a fresh prompt
            // after that interaction, even when an older B was already seen.
            self.shell_ready = false;
            for byte in bytes {
                self.startup_line = !matches!(byte, b'\r' | b'\n' | 3);
            }
        }
    }
}

impl<W: Write> Write for StartupInput<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.writer.write(bytes)?;
        self.user_input(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Prompt marks are parsed across PTY read boundaries. B follows the prompt
/// after the shell's native startup files and hooks have finished.
#[derive(Default)]
pub(crate) struct PromptReady {
    state: u8,
    body: Vec<u8>,
    ready: bool,
}

impl PromptReady {
    pub(crate) fn take_ready(&mut self) -> bool {
        std::mem::take(&mut self.ready)
    }

    pub(crate) fn feed(&mut self, bytes: &[u8]) -> bool {
        for &byte in bytes {
            match (self.state, byte) {
                (0, 0x1b) => self.state = 1,
                (1, b']') => {
                    self.state = 2;
                    self.body.clear();
                }
                (1, 0x1b) => {}
                (1, _) => self.state = 0,
                (2, 7) | (3, b'\\') => {
                    self.ready |= self.body.starts_with(b"133;B")
                        && matches!(self.body.get(5), None | Some(&b';'));
                    self.state = 0;
                }
                (2, 0x1b) => self.state = 3,
                (2 | 3, _) if self.body.len() < 512 => {
                    self.body.push(byte);
                    self.state = 2;
                }
                (2 | 3, _) => self.state = 0,
                _ => {}
            }
        }
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_shell_arguments_do_not_cross_native_boundaries() {
        assert_eq!(
            command_for("/bin/zsh", Platform::Mac, None),
            ["/bin/zsh", "-l", "-i"]
        );
        assert_eq!(
            command_for("/bin/bash", Platform::Unix, None),
            ["/bin/bash", "-i"]
        );
        assert_eq!(
            command_for("/bin/bash", Platform::Unix, Some("always")),
            ["/bin/bash", "-l", "-i"]
        );
        assert_eq!(
            command_for("/bin/zsh", Platform::Mac, Some("never")),
            ["/bin/zsh", "-i"]
        );
        assert_eq!(
            command_for("pwsh.exe", Platform::Windows, Some("always")),
            ["pwsh.exe"]
        );
        assert_eq!(
            command_for("/custom/shell", Platform::Mac, None),
            ["/custom/shell"]
        );
        assert_eq!(
            hosted_command(&["/bin/zsh".into(), "-f".into()]),
            ["/bin/zsh", "-f"]
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_shell_paths_project_only_verbatim_local_disks() {
        use std::path::Path;
        let canonical = Path::new(r"\\?\C:\Users\中文 用户\khor");
        let local = Path::new(r"C:\Users\中文 用户\khor");
        for shell in ["cmd", "CMD.EXE", "powershell.exe", "PowerShell", "pwsh.exe"] {
            assert_eq!(
                native_shell_path(Path::new(shell), canonical).as_ref(),
                local
            );
        }
        assert_eq!(
            native_shell_path(Path::new("native-tool.exe"), canonical).as_ref(),
            canonical
        );
        for path in [
            r"C:\Users\中文 用户\khor",
            r"\\server\share\khor",
            r"\\?\UNC\server\share\khor",
            r"\\?\Volume{1234}\khor",
            r"\\.\C:\khor",
        ] {
            let path = Path::new(path);
            assert_eq!(
                native_shell_path(Path::new("powershell.exe"), path).as_ref(),
                path
            );
        }
    }

    #[test]
    fn structured_startup_uses_the_target_shells_quoting() {
        let argv = vec!["tool".into(), "a b".into(), "x'y".into(), "".into()];
        assert_eq!(
            startup_command("/bin/zsh", "", Some(&argv)).unwrap(),
            "'tool' 'a b' 'x'\\''y' ''"
        );
        assert_eq!(
            startup_command("pwsh.exe", "", Some(&argv)).unwrap(),
            "& 'tool' 'a b' 'x''y' ''"
        );
        assert_eq!(
            startup_command("cmd.exe", "", Some(&["tool".into(), "a b".into()])).unwrap(),
            "tool \"a b\""
        );
        assert!(startup_command("cmd.exe", "", Some(&["echo".into(), "%PATH%".into()])).is_err());
        assert!(startup_command("unknown-shell", "", Some(&argv)).is_err());
        assert_eq!(
            startup_command("unknown-shell", "literal | command", None).unwrap(),
            "literal | command"
        );
    }

    #[test]
    fn attachment_paths_are_literal_arguments_in_the_host_shell() {
        let path = r"C:\资料\John's notes & plan.txt";
        assert_eq!(
            WindowsShell::from_program(r"C:\Windows\System32\CMD.EXE"),
            Some(WindowsShell::Cmd)
        );
        assert_eq!(
            WindowsShell::from_program("/opt/microsoft/powershell/7/pwsh"),
            Some(WindowsShell::PowerShell)
        );
        assert_eq!(WindowsShell::from_program("bash"), None);
        assert_eq!(
            WindowsShell::Cmd.quote_attachment(path).unwrap(),
            format!("\"{path}\"")
        );
        assert_eq!(
            WindowsShell::Cmd
                .quote_attachment(&format!(r"\\?\{path}"))
                .unwrap(),
            format!("\"{path}\"")
        );
        assert_eq!(
            WindowsShell::PowerShell.quote_attachment(path).unwrap(),
            r"'C:\资料\John''s notes & plan.txt'"
        );
        assert_eq!(
            WindowsShell::PowerShell.quote_attachment("‘a’").unwrap(),
            "'‘‘a’’'"
        );
        for path in ["%PATH%", "!PATH!", "\"quoted\"", "two\nlines"] {
            assert!(WindowsShell::Cmd.quote_attachment(path).is_err());
        }
    }

    #[test]
    fn input_tokens_outlive_the_memory_cache_and_cannot_be_rebound() {
        let dir = crate::testdir::TempTree::fresh("terminal-input-receipts");
        let mut input = StartupInput::new(Vec::new(), None).with_receipts(dir.path());
        input.ready().unwrap();
        for n in 0..140 {
            let key = format!("input-request-{n:06}");
            assert_eq!(input.automated_input(&key, n, b"echo test\r"), Ok(n + 1));
        }
        let before = input.writer.len();
        assert_eq!(
            input.automated_input("input-request-000000", 0, b"echo test\r"),
            Ok(1)
        );
        assert!(
            input
                .automated_input("input-request-000000", 140, b"echo changed\r")
                .is_err()
        );
        assert_eq!(input.writer.len(), before);
    }

    #[test]
    fn durable_input_receipts_cannot_acknowledge_another_host_generation() {
        let dir = crate::testdir::TempTree::fresh("terminal-generation-receipts");
        let mut old = StartupInput::new(Vec::new(), None).with_receipts(dir.path());
        old.ready().unwrap();
        assert_eq!(
            old.automated_input_in_generation("old-host", "input-generation-01", 0, b"echo old\r"),
            Ok(1)
        );
        let mut replacement = StartupInput::new(Vec::new(), None).with_receipts(dir.path());
        replacement.ready().unwrap();
        assert!(
            replacement
                .automated_input_in_generation("new-host", "input-generation-01", 0, b"echo old\r")
                .is_err()
        );
        assert!(replacement.writer.is_empty());
        assert_eq!(replacement.revision(), 0);
    }

    #[test]
    fn shell_and_terminal_readiness_are_both_required_in_either_order() {
        for shell_first in [true, false] {
            let mut input = StartupInput::with_shell_readiness(Vec::new(), Some("run"), true);
            if shell_first {
                input.shell_ready().unwrap();
            } else {
                input.ready().unwrap();
            }
            assert!(input.writer.is_empty());
            if shell_first {
                input.ready().unwrap();
            } else {
                input.shell_ready().unwrap();
            }
            assert_eq!(input.writer, b"run\r");
            input.ready().unwrap();
            input.shell_ready().unwrap();
            assert_eq!(input.writer, b"run\r");
        }
    }

    #[test]
    fn prompt_readiness_survives_every_read_boundary() {
        for mark in [
            b"\x1b]133;B\x07".as_slice(),
            b"\x1b]133;B;click_events=1\x1b\\",
        ] {
            let mut prompt = PromptReady::default();
            for byte in &mark[..mark.len() - 1] {
                assert!(!prompt.feed(&[*byte]));
            }
            assert!(prompt.feed(&mark[mark.len() - 1..]));
            assert!(prompt.take_ready());
            assert!(!prompt.feed(b"ordinary output"));
            assert!(!prompt.take_ready());
            assert!(prompt.feed(mark));
            assert!(prompt.take_ready());
        }
        assert!(!PromptReady::default().feed(b"\x1b]133;BLAH\x07"));
    }

    #[test]
    fn automated_input_is_atomic_idempotent_and_respects_user_editing() {
        let mut input = StartupInput::new(Vec::new(), None);
        input.ready().unwrap();
        assert_eq!(
            input.automated_input("input-first-0001", 0, b"echo one\r"),
            Ok(1)
        );
        assert_eq!(
            input.automated_input("input-first-0001", 0, b"echo one\r"),
            Ok(1)
        );
        assert_eq!(input.writer, b"echo one\r");
        assert!(
            input
                .automated_input("input-first-0001", 0, b"echo other\r")
                .is_err()
        );
        input.write_all(b"user-prefix").unwrap();
        assert!(
            input
                .automated_input("input-stale-0001", 1, b"echo two\r")
                .is_err()
        );
        assert!(
            input
                .automated_input("input-editing-01", input.revision(), b"echo two\r")
                .is_err()
        );
        input.write_all(b"\x03").unwrap();
        let revision = input.revision();
        assert_eq!(
            input.automated_input("input-next-00001", revision, b"echo two\r"),
            Ok(revision + 1)
        );
    }

    #[test]
    fn completion_and_history_keys_protect_the_humans_unfinished_line() {
        for key in [9, 18, 16, 21, 27, 127, 4] {
            let mut input = StartupInput::new(Vec::new(), None);
            input.ready().unwrap();
            input.write_all(&[key]).unwrap();
            assert!(
                input
                    .automated_input("input-editing-001", input.revision(), b"echo robot\r")
                    .is_err()
            );
            input.write_all(&[3]).unwrap();
            let revision = input.revision();
            assert_eq!(
                input.automated_input("input-after-clear", revision, b"echo robot\r"),
                Ok(revision + 1)
            );
        }
    }

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
    fn profile_answers_reach_the_shell_before_automatic_startup() {
        let mut input = StartupInput::with_shell_readiness(Vec::new(), Some("startup"), true);
        input.ready().unwrap();
        input.write_classified(b"n\r", b"n\r", b"").unwrap();
        assert_eq!(input.writer, b"n\r");
        assert_eq!(input.readiness(), (false, false));
        input.shell_ready().unwrap();
        input.ready().unwrap();
        input.shell_ready().unwrap();
        input.write_all(b"later\r").unwrap();
        assert_eq!(input.writer, b"n\rstartup\rlater\r");
    }

    #[test]
    fn an_operator_answer_invalidates_readiness_until_a_fresh_prompt() {
        for control in [false, true] {
            let mut input = StartupInput::with_shell_readiness(Vec::new(), Some("startup"), true);
            input.shell_ready().unwrap();
            if control {
                assert_eq!(
                    input.automated_input("profile-answer-001", 0, b"n\r"),
                    Ok(1)
                );
            } else {
                input.write_all(b"n\r").unwrap();
            }
            input.ready().unwrap();
            assert_eq!(input.writer, b"n\r");
            assert_eq!(input.readiness(), (false, false));
            input.shell_ready().unwrap();
            assert_eq!(input.writer, b"n\rstartup\r");
        }
    }

    #[test]
    fn prompt_redraws_do_not_append_startup_to_a_partial_operator_line() {
        for control in [false, true] {
            let mut input = StartupInput::with_shell_readiness(Vec::new(), Some("startup"), true);
            input.ready().unwrap();
            if control {
                assert_eq!(
                    input.automated_input("profile-partial-01", 0, b"partial"),
                    Ok(1)
                );
            } else {
                input.write_classified(b"partial", b"partial", b"").unwrap();
            }
            input.shell_ready().unwrap();
            assert_eq!(input.writer, b"partial");
            if control {
                assert_eq!(input.automated_input("profile-finish-001", 1, b"\r"), Ok(2));
            } else {
                // A redraw must not clear the human editing fence either.
                assert!(
                    input
                        .automated_input("profile-interfere", 1, b"robot\r")
                        .is_err()
                );
                input.write_classified(b"\r", b"\r", b"").unwrap();
            }
            input.ready().unwrap();
            assert_eq!(input.writer, b"partial\r");
            input.shell_ready().unwrap();
            assert_eq!(input.writer, b"partial\rstartup\r");
        }
    }

    #[test]
    fn pre_ready_control_input_keeps_cas_and_durable_idempotence() {
        let dir = crate::testdir::TempTree::fresh("profile-answer-receipts");
        let mut input = StartupInput::with_shell_readiness(Vec::new(), Some("startup"), true)
            .with_receipts(dir.path());
        assert_eq!(
            input.automated_input("profile-answer-001", 0, b"n\r"),
            Ok(1)
        );
        assert_eq!(
            input.automated_input("profile-answer-001", 0, b"n\r"),
            Ok(1)
        );
        assert!(
            input
                .automated_input("profile-answer-001", 0, b"y\r")
                .is_err()
        );
        assert!(
            input
                .automated_input("profile-stale-0001", 0, b"n\r")
                .is_err()
        );
        assert_eq!(input.writer, b"n\r");
        // A host reconstructed against the durable token cannot replay it.
        let mut retry = StartupInput::with_shell_readiness(Vec::new(), Some("startup"), true)
            .with_receipts(dir.path());
        assert_eq!(
            retry.automated_input("profile-answer-001", 0, b"n\r"),
            Ok(1)
        );
        assert!(retry.writer.is_empty());
        input.ready().unwrap();
        input.shell_ready().unwrap();
        assert_eq!(input.writer, b"n\rstartup\r");
    }

    #[test]
    fn an_uncertain_operator_write_cannot_release_automatic_startup() {
        #[derive(Default)]
        struct PartialWriter {
            bytes: Vec<u8>,
            fail: bool,
        }
        impl Write for PartialWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.fail {
                    return Err(io::Error::other("write stopped"));
                }
                self.bytes.extend_from_slice(&bytes[..1]);
                self.fail = true;
                Ok(1)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for control in [false, true] {
            let mut input =
                StartupInput::with_shell_readiness(PartialWriter::default(), Some("startup"), true);
            input.ready().unwrap();
            if control {
                assert!(
                    input
                        .automated_input("profile-answer-001", 0, b"n\r")
                        .is_err()
                );
            } else {
                assert!(input.write_classified(b"n\r", b"n\r", b"").is_err());
            }
            input.shell_ready().unwrap();
            assert_eq!(input.writer.bytes, b"n");
            assert_eq!(input.readiness(), (true, false));
        }
    }

    #[test]
    fn ordinary_and_empty_shells_accept_profile_answers_before_readiness() {
        for startup in [None, Some(" \n")] {
            let mut input = StartupInput::with_shell_readiness(Vec::new(), startup, true);
            input.write_all(b"immediate\r").unwrap();
            assert_eq!(input.writer, b"immediate\r");
            assert_eq!(
                input.automated_input("profile-answer-001", 1, b"n\r"),
                Ok(2)
            );
            assert_eq!(input.writer, b"immediate\rn\r");
        }
    }

    #[test]
    fn classified_terminal_replies_do_not_invalidate_a_ready_prompt() {
        let mut input = StartupInput::with_shell_readiness(Vec::new(), Some("startup"), true);
        input.shell_ready().unwrap();
        input.write_classified(b"\x1b[0n", b"", b"\x1b[0n").unwrap();
        assert_eq!(input.revision(), 0);
        input.ready().unwrap();
        assert_eq!(input.writer, b"\x1b[0nstartup\r");
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
