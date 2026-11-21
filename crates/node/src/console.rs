//! Starting a console program from a process that has no console.
//!
//! A Windows GUI-subsystem process has no console of its own. When it
//! starts a console program — `khor.exe`, `ssh`, the editor's CLI —
//! Windows allocates a brand new console for that child and **shows it**,
//! so pressing a button in the app flashes a black window the person did
//! not ask for. `hidden` asks Windows for a console with no window.
//!
//! **Returns a `Command` instead of taking one**, because
//! `CommandExt::creation_flags` is a setter that *replaces* the whole flag
//! word: a caller that later discovers it also needs `DETACHED_PROCESS`
//! would silently drop this flag by composing them in the wrong order.
//! Handing back a ready command keeps the flag out of every caller's way.
//! Callers that must set flags of their own (a detached host, a durable
//! worker) compose them together at the creation site instead — see
//! `mcp_worker::prepare_durable_child`, which is where that hazard lives.

/// A command for `program` that will not open a console window.
#[cfg(windows)]
pub(crate) fn hidden(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new(program);
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    command
}

/// A command for `program`. Everywhere else a child has no console window
/// to open, so this is the ordinary spawn.
#[cfg(not(windows))]
pub(crate) fn hidden(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    std::process::Command::new(program)
}
