//! Where a process spent its first moments, when somebody asks it to say.
//!
//! The session host is started detached with all three streams on null, so
//! a slow start has nowhere to report itself: the opener sees one number —
//! "until host.json appeared" — and every cause inside it looks the same
//! from outside. Measured 2026-11-17 on Windows: **4.4 s, every run**, with
//! a bare process start of 55 ms, so it was neither the disk being cold
//! nor the machine being slow.
//!
//! `KHOR_STARTUP_TRACE=<path>` makes every process append its marks to that
//! file: `pid<TAB>elapsed_ms<TAB>name`. Off unless asked, one `Instant` and
//! one environment lookup otherwise, and no sentence in any language —
//! this is a developer's instrument, not something a person reads.

use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

/// The moment this process began — called first thing, so the marks are
/// measured from a point the person's wait also started at.
pub fn begin() {
    START.get_or_init(Instant::now);
    mark("begin");
}

/// Appends one line, when tracing is on and this process has a start.
pub fn mark(name: &str) {
    let Some(path) = std::env::var_os("KHOR_STARTUP_TRACE") else {
        return;
    };
    let elapsed = START.get_or_init(Instant::now).elapsed().as_millis();
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(file, "{}\t{elapsed}\t{name}", std::process::id());
}
