//! The keeper: what stands between `khor serve` and dying silently.
//!
//! # Why a supervisor, and why khor's own
//!
//! hinton's serve died at 08:12 and the machine read as "offline" for
//! eleven hours — the box itself was fine (ping 0.13ms, sshd up), nothing
//! restarted the serve, and nothing even *knew*. Its log held six lines
//! of "在听" and not one word about a death: the process was killed with
//! a signal, and a process killed with a signal writes nothing. **Only
//! something standing outside it can say how it died** — that is the
//! whole argument for a supervisor, and it is also why "死前说一句"
//! cannot be done from the inside.
//!
//! systemd user units and launchd do this properly, and on a machine
//! that has them the install may grow that later. But khor's machines
//! include shared HPC boxes where systemd lingering needs an admin and
//! cron is disabled — the same machines that made the install story
//! "one file, no root". So the keeper ships inside that one file.
//!
//! # The shape
//!
//! `khor serve` is the keeper; the actual serve is the same binary
//! re-run with [`INNER_ENV`] set. The keeper spawns it, waits, writes
//! one line about every death, and starts it again — with backoff, so a
//! serve that dies at birth (bad config, taken port) does not become a
//! hot loop. A **clean** exit ends the keeper too: intentional shutdown
//! is not a failure to repair.
//!
//! Respawning re-executes [`crate::self_exe`], so after an upgrade
//! replaces the binary on disk, the very next restart runs the new
//! version — the keeper is how an upgraded serve comes up without
//! anybody remembering to bounce it.
//!
//! # Signals
//!
//! The pid file written by the installer holds the **keeper's** pid, so
//! `kill $(cat serve.pid)` must stop everything: the keeper forwards
//! SIGTERM/SIGINT to the child and exits without respawning. Ctrl-C in a
//! foreground `khor serve` reaches both (same process group) and takes
//! the same path.

use std::sync::atomic::{AtomicI32, Ordering};

use khor_catalog::msg;

/// Set on the child: "you are the serve, not the keeper". An env var
/// rather than a hidden verb so `ps` shows the same honest `khor serve`
/// for both processes, parent over child.
pub const INNER_ENV: &str = "KHOR_SERVE_KEPT";

/// Whether this process is the kept serve itself.
pub fn is_inner() -> bool {
    std::env::var_os(INNER_ENV).is_some()
}

/// A child that lived at least this long was up for real, and its death
/// is news rather than a birth defect: the backoff resets.
const LIVED: std::time::Duration = std::time::Duration::from_secs(60);

/// The longest the keeper waits between respawns of a serve that keeps
/// dying young. Long enough not to hammer whatever is killing it, short
/// enough that a machine recovers within a minute of the cause clearing.
const BACKOFF_MAX: u64 = 60;

/// The kept child's pid, for the signal handler — which may run at any
/// instant and can touch nothing but an atomic.
static CHILD: AtomicI32 = AtomicI32::new(0);

/// Forwards the signal to the child and leaves. `_exit` because a
/// signal handler must not unwind, and there is nothing to clean up —
/// the child owns every resource worth the name.
extern "C" fn forward(sig: libc::c_int) {
    let child = CHILD.load(Ordering::SeqCst);
    if child > 0 {
        unsafe {
            libc::kill(child, sig);
        }
    }
    unsafe { libc::_exit(128 + sig) }
}

/// Runs the serve under the keeper, forever. Returns only when the serve
/// ends cleanly (exit 0) or a child cannot even be spawned.
pub fn keep() -> Result<(), String> {
    unsafe {
        libc::signal(libc::SIGTERM, forward as libc::sighandler_t);
        libc::signal(libc::SIGINT, forward as libc::sighandler_t);
    }
    let mut backoff = 1u64;
    loop {
        let exe = crate::self_exe()?;
        let mut child = std::process::Command::new(&exe)
            .args(std::env::args_os().skip(1))
            .env(INNER_ENV, "1")
            .spawn()
            .map_err(msg::host_wont_start)?;
        CHILD.store(child.id() as i32, Ordering::SeqCst);
        let born = std::time::Instant::now();
        let status = child.wait().map_err(|e| e.to_string())?;
        CHILD.store(0, Ordering::SeqCst);
        if status.success() {
            return Ok(());
        }
        // How it died, from outside — the one vantage point that still
        // works when the death was a signal.
        let how = {
            use std::os::unix::process::ExitStatusExt;
            match (status.code(), status.signal()) {
                (Some(code), _) => msg::died_with_code(code),
                (None, Some(sig)) => msg::died_by_signal(sig),
                (None, None) => msg::DIED_UNREADABLY.to_owned(),
            }
        };
        if born.elapsed() >= LIVED {
            backoff = 1;
        }
        // 报故障要带时间戳: this line lands in a log somebody reads
        // hours later, next to lines from other lifetimes.
        let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
        eprintln!("{}", msg::serve_died_restarting(&stamp, &how, backoff));
        std::thread::sleep(std::time::Duration::from_secs(backoff));
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
}
