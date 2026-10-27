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

// ── the kept serve's heartbeat (net-dial B1) ────────────────────────
//
// The high-water mark above is a number somebody has to ask for, and
// on a multi-threaded runtime it only moves when every worker is
// blocked — by which time nobody can ask. turing, 2026-10-20: 22:50:15
// to 00:18:02 and again 00:53:42 to 01:44:36 it answered nothing and
// wrote nothing, and was not dead, so its keeper — which only restarts
// a serve that exits (`crate::keeper`) — had nothing to do. Its deaf
// watch could not help either: it runs on the very loop that stopped.
//
// So the kept serve's loop beats ([`beat`]) and a plain thread outside
// the runtime ([`guard`]) watches the beats. A loop that has not beaten
// for [`STUCK_AFTER`] is stuck, and the serve leaves with [`STUCK_EXIT`]
// for its keeper to log and start again.

/// When the serve's loop last beat, in ms of [`since_start`]; 0 never.
static BEAT_MS: AtomicU64 = AtomicU64::new(0);

/// How long a kept serve's loop may go without a beat before it is
/// taken for stuck: **ten minutes**.
///
/// Not shorter, because the slow thing on the machines that showed this
/// is a home on NFS, and a hard NFS mount does not fail a call, it
/// retries it: one RPC waits 60 s, then twice more with backoff, a few
/// minutes before the kernel even says 「server not responding」 — and a
/// machine whose home came back after such a blip is a machine that
/// should keep its serve. Ten minutes is more than three of those and
/// five of [`crate::link`]'s deafness (`DEAF_AFTER`, two minutes).
/// Not longer, because it is the length of the outage it leaves: turing's
/// were 87 and 51 minutes.
pub const STUCK_AFTER: Duration = Duration::from_secs(600);

/// How often the guard looks.
const LOOK_EVERY: Duration = Duration::from_secs(15);

/// A kept serve's exit code when its loop was stuck: news for the
/// keeper's log (「serve 死了（退出码 87）」), a restart like any other.
pub const STUCK_EXIT: i32 = 87;

fn since_start() -> Duration {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed()
}

/// The loop is alive: called on every beat of the serve's loop, after
/// it has touched what a stuck serve was found stuck on.
pub fn beat() {
    BEAT_MS.store((since_start().as_millis() as u64).max(1), Ordering::Relaxed);
}

/// How long since the last [`beat`] at `now` (a [`since_start`]
/// reading); `None` before the first.
fn quiet_at(now: Duration) -> Option<Duration> {
    match BEAT_MS.load(Ordering::Relaxed) {
        0 => None,
        ms => Some(now.saturating_sub(Duration::from_millis(ms))),
    }
}

/// Watches the beats from a thread of its own — not a task: a runtime
/// whose workers are all held cannot run the thing that would notice.
/// Calls `stuck` once with the silence when the loop has not beaten for
/// `after`; the serve passes [`leave_stuck`].
pub fn guard(after: Duration, every: Duration, stuck: impl FnOnce(Duration) + Send + 'static) {
    let _ = std::thread::Builder::new()
        .name("khor-guard".into())
        .spawn(move || {
            loop {
                std::thread::sleep(every);
                if let Some(quiet) = quiet_at(since_start())
                    && quiet >= after
                {
                    stuck(quiet);
                    return;
                }
            }
        });
}

/// [`guard`] as the kept serve arms it.
pub fn guard_serve() {
    guard(STUCK_AFTER, LOOK_EVERY, leave_stuck);
}

/// Says why, if saying it does not itself get stuck, and leaves.
///
/// The sentence goes to stderr, which for a kept serve is `serve.log` —
/// on turing a file on the same NFS that may be what holds the loop. So
/// it is written from a thread of its own and given five seconds; then
/// `_exit`, which runs nothing that could wait on anything. (A thread
/// parked in the kernel on a dead NFS server still holds the process
/// until that call returns; nothing in a process can cut that short.)
fn leave_stuck(quiet: Duration) {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new().spawn(move || {
        let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
        eprintln!(
            "{}",
            khor_catalog::msg::serve_stuck(&stamp, quiet.as_secs() / 60)
        );
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(Duration::from_secs(5));
    unsafe { libc::_exit(STUCK_EXIT) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A loop that stops beating is found, one that beats is not**: the
    /// guard on milliseconds, the same code the serve arms on minutes.
    /// One test, because the beat is one process-global, as the serve
    /// has one loop.
    #[test]
    fn a_loop_that_stops_beating_is_found_and_one_that_beats_is_not() {
        beat();
        let (tx, rx) = std::sync::mpsc::channel();
        guard(
            Duration::from_millis(300),
            Duration::from_millis(20),
            move |q| {
                let _ = tx.send(q);
            },
        );
        // Beating: nothing for a second, three times the limit.
        let until = Instant::now() + Duration::from_secs(1);
        while Instant::now() < until {
            beat();
            assert!(
                rx.try_recv().is_err(),
                "a loop that beats is never taken for stuck"
            );
            std::thread::sleep(Duration::from_millis(30));
        }
        // Stopped: found within the limit and a look or two.
        let t = Instant::now();
        let quiet = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("a loop that stopped is found");
        assert!(
            quiet >= Duration::from_millis(300),
            "not before its limit: {quiet:?}"
        );
        assert!(
            t.elapsed() < Duration::from_secs(1),
            "and not long after: {:?}",
            t.elapsed()
        );
    }
}
