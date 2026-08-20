//! How long the serve's reactor was unavailable — a high-water mark.
//!
//! # Why this exists, and what it replaced
//!
//! The vitals acceptance guards one thing: **a request handler must not
//! block the reactor**. It was written after one did — sampling a real
//! machine takes 1.9 s, and calling that inline in the handler stalled
//! QUIC long enough that a twenty-second sync timed out. So the test
//! measured the sync's wall clock and called anything over twenty
//! seconds a stall.
//!
//! **A wall clock cannot tell a block from a wait.** It measures the
//! sum of both, so anything slow reads as the bug: on this machine a
//! first-touch authorization stall (#73, 20-40 s, nothing to do with
//! khor) landed on that very sync and looked exactly like a handler
//! blocking the reactor. Measured 2026-08-21: bind 21 ms, dial 1.1 s,
//! the guarded sync 37 s — and no instrument in the tree could say
//! which of the two the 37 s was. **The guard's twenty seconds had
//! stopped being able to tell its own bug from the machine's.**
//!
//! So this measures the property itself instead of a consequence: a
//! ticker on the serve's runtime, and the worst gap between its ticks.
//! A **synchronous block** starves it and shows up; an `await`, however
//! long, does not — which is exactly the line the old measurement could
//! not draw.
//!
//! # It only sees anything on a single-threaded runtime
//!
//! On a multi-threaded runtime another worker picks the ticker up and
//! the stall is invisible. That is not a flaw here — it is why the
//! vitals acceptance runs on `#[tokio::test]`'s default single thread,
//! a judgment older than this module. **This design keeps that judgment
//! rather than overturning it**: the cheapest reactor to stall is still
//! the one with a single thread, and now there is something watching it.
//!
//! # Process-global, on purpose
//!
//! The thing being measured is a runtime's thread, not a `Node` — and
//! the test that reads it holds a different `Node` instance from the
//! one running `serve`. One process, one reactor, one number.
//!
//! # It lives in the shipped binary
//!
//! Not behind `cfg(test)`: a guard that watches something other than
//! the code people run is not watching the code people run. The cost is
//! one timer task and one atomic. It is also useful on its own — a
//! resident record of how long this serve has ever been stuck.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// How often the watch checks in. Short enough that a stall worth a
/// word is many ticks long, cheap enough to leave running.
const TICK: Duration = Duration::from_millis(50);

/// The worst gap seen, in milliseconds beyond the tick itself.
static WORST_MS: AtomicU64 = AtomicU64::new(0);

/// Starts the watch on the current runtime. Called once by `serve`;
/// calling it twice would just mean two tickers agreeing.
pub fn watch() {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        // A stall makes several ticks come due at once. Delay, not
        // Burst: bursting would fire them back to back with no time
        // between, and every gap after the first would read as zero —
        // the stall would erase its own evidence.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await;
        // `std::time::Instant`, not tokio's: tokio's clock is the thing
        // a synchronous block freezes, so measuring the freeze with it
        // would always read zero.
        let mut last = Instant::now();
        loop {
            ticker.tick().await;
            let now = Instant::now();
            let gap = now.duration_since(last);
            last = now;
            let over = gap.saturating_sub(TICK).as_millis() as u64;
            WORST_MS.fetch_max(over, Ordering::Relaxed);
        }
    });
}

/// The longest the reactor has been unavailable since the last
/// [`forget`], in milliseconds.
pub fn worst_stall_ms() -> u64 {
    WORST_MS.load(Ordering::Relaxed)
}

/// Starts a fresh measurement window. A caller that wants to ask about
/// one stretch of time has to say where it begins — otherwise the
/// answer includes every stall since the process started, which for a
/// test is every stall its own setup caused.
pub fn forget() {
    WORST_MS.store(0, Ordering::Relaxed);
}
