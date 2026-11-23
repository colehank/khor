//! The processes the tests need, on every platform.
//!
//! Several fixtures spawned `sleep` and `true` by bare name. Those files
//! exist on a Unix PATH and nowhere on Windows, so the test died with
//! "program not found" before it reached the thing it asserts. The
//! stand-ins below are the same two processes on each platform; they are
//! test-only and never linked into a shipped binary.

use std::process::Command;

/// A process that stays alive for roughly `seconds`, then exits 0.
///
/// `ping` is the sleep Windows ships without a console: `timeout` refuses
/// redirected input, and a PowerShell start is a second of startup for a
/// fixture that only needs a live pid.
pub(crate) fn sleeper(seconds: u64) -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("ping");
        command.args(["-n", &(seconds + 1).to_string(), "127.0.0.1"]);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("sleep");
        command.arg(seconds.to_string());
        command
    }
}

/// A process that does nothing and exits successfully.
pub(crate) fn true_like() -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.args(["/C", "exit", "0"]);
        command
    }
    #[cfg(not(windows))]
    {
        Command::new("true")
    }
}
