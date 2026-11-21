//! Where the app's own words go when there is no console to say them on.
//!
//! A Windows GUI-subsystem process has no standard streams, so `eprintln!`
//! goes nowhere. That is the right default for a shipped app — a console
//! window for every launch is not — but it leaves the one bad day with no
//! evidence: the window did not appear and nothing says why.
//!
//! [`attach`] points stdout and stderr at a log file instead, so the same
//! lines a developer reads in a terminal land where the person using the
//! app can find them. `KHOR_LOG_FILE` chooses the file; without it the
//! log sits under the app's own directory in the local app data folder.
//! `KHOR_GUI_CONSOLE=1` asks for a real console window instead, which is
//! what a development build launched from a terminal wants and what the
//! one-off debugging session reaches for.
//!
//! Called first thing in `main`, before anything can write: the handle
//! swap only works while the standard handles are still untouched.

/// Points the standard streams somewhere a person can read, or leaves
/// them alone when a console already exists.
pub fn attach() {
    #[cfg(windows)]
    windows::attach();
}

#[cfg(not(windows))]
#[allow(dead_code)]
mod windows_absent {
    // macOS and Linux keep their streams: launched from a terminal they
    // are that terminal, and a bundled app's stderr is already captured
    // by the system log. Doing nothing here is the honest behaviour.
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;

    type Handle = *mut c_void;

    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;

    /// A few megabytes is weeks of ordinary use; the cap keeps a chatty
    /// development build from filling a disk nobody is watching.
    const LOG_LIMIT: u64 = 8 * 1024 * 1024;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleWindow() -> Handle;
        fn AllocConsole() -> i32;
        fn GetStdHandle(handle: u32) -> Handle;
        fn SetStdHandle(handle: u32, value: Handle) -> i32;
    }

    /// Whether this process has a standard stream at all — a console, or a
    /// **pipe its parent handed it**.
    fn stream_present(which: u32) -> bool {
        let handle = unsafe { GetStdHandle(which) };
        !handle.is_null() && handle as isize != -1
    }

    pub(super) fn attach() {
        let wanted = std::env::var_os("KHOR_GUI_CONSOLE").is_some_and(|v| {
            let said = v.to_string_lossy();
            !(said.is_empty() || said == "0" || said.eq_ignore_ascii_case("false"))
        });
        let already_has_one = unsafe { !GetConsoleWindow().is_null() };
        if already_has_one {
            // Launched from a terminal (or a debugger): those streams are
            // the ones the person is watching. Never take them away.
            return;
        }
        if wanted && unsafe { AllocConsole() } != 0 {
            // A fresh console: the standard handles the process never had
            // are now bound to it, so `eprintln!` reaches the new window
            // without this module touching a handle.
            return;
        }
        // **No console is not the same as no streams.** This same binary is
        // re-executed as `_host`, `_ghost`, `_cagent`, `_codexagent` and the
        // MCP worker (`main_if_host` in `khor_gui::run`); those get pipes
        // from their parent and speak a protocol on them. Redirecting here
        // would write the handshake into a log file that nobody reads, and
        // the parent would wait forever — found by the independent review
        // before it shipped. A real window launched from Explorer has no
        // handles at all, which is what this asks.
        if stream_present(STD_OUTPUT_HANDLE) || stream_present(STD_ERROR_HANDLE) {
            return;
        }
        let Some(path) = log_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        rotate(&path);
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        else {
            return;
        };
        let _ = writeln!(
            file,
            "[gui] {} start {} ({})",
            stamp(),
            env!("CARGO_PKG_VERSION"),
            std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "?".into())
        );
        let _ = file.flush();
        let handle: Handle = file.as_raw_handle() as Handle;
        unsafe {
            SetStdHandle(STD_OUTPUT_HANDLE, handle);
            SetStdHandle(STD_ERROR_HANDLE, handle);
        }
        // The handle is the process's now; the `File` only existed to
        // produce it. Forgetting keeps it open for the life of the app.
        std::mem::forget(file);
    }

    fn log_path() -> Option<PathBuf> {
        if let Some(chosen) = std::env::var_os("KHOR_LOG_FILE") {
            return Some(PathBuf::from(chosen));
        }
        let base = std::env::var_os("KHOR_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Khor")))?;
        Some(base.join("logs").join("gui.log"))
    }

    /// One older copy, dropped the moment it is replaced: the previous
    /// run's lines are what a report needs, and the run before that is
    /// noise that would otherwise fill the disk one cap at a time.
    fn rotate(path: &std::path::Path) {
        let over = std::fs::metadata(path)
            .map(|m| m.len() > LOG_LIMIT)
            .unwrap_or(false);
        if !over {
            return;
        }
        let _ = std::fs::remove_file(path.with_extension("log.1"));
        let _ = std::fs::rename(path, path.with_extension("log.1"));
    }

    /// Local time as `YYYY-MM-DD HH:MM:SS`. No date crate is a dependency
    /// of this crate, and a log line needs the wall clock the person will
    /// compare against their own clock, not UTC.
    fn stamp() -> String {
        #[repr(C)]
        struct SystemTime {
            year: u16,
            month: u16,
            day_of_week: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            milliseconds: u16,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetLocalTime(out: *mut SystemTime);
        }
        let mut t = SystemTime {
            year: 0,
            month: 0,
            day_of_week: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            milliseconds: 0,
        };
        unsafe { GetLocalTime(&mut t) };
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            t.year, t.month, t.day, t.hour, t.minute, t.second
        )
    }
}
