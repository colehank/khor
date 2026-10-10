//! A far terminal or conversation that **attaches again** when its pipe
//! ends, instead of ending (docs/IOS.md 第三步).
//!
//! A face attached to a session on another machine reads it through a
//! pipe the resident binds on its endpoint (`Node::reach`). On a phone
//! that pipe dies every time the app is suspended: going to the
//! background lets every wire go, and `khor_node::link::resume` mends
//! the endpoint on the way back (new ears only when it went deaf).
//! Until this, the reader saw its socket end and told the face
//! 「这个终端结束了」, about a session still running over there.
//!
//! **The same road back, not a second one.** A pipe that ends is asked
//! for again with `Node::rejoin` — the reach every attach makes, on the
//! resident's current ears — and the far host pours its ring to the new
//! client, as it does for every attacher (`khor_node::host`). What the
//! face already does with an attachment that starts over is unchanged:
//! the terminal's feed starts a new life and the face's cursor falls off
//! its head (`TermChunk::reset` — reset and replay), the conversation's
//! frames do the same (`ChatBatch::lost` — clear and ask for history).
//!
//! **On everywhere, since round NET-FIX** ([`set_on`] can still turn it
//! off): it used to be the phone's alone, while a desktop's far terminal
//! that lost its pipe ended — and with wires that are now retired when
//! they go silent (`khor_node`'s wires), a desktop loses pipes for the
//! same reasons a phone does: a sleep, a network change, a far machine
//! restarted. The face says 正在重新连接 meanwhile (`TermChunk::stall`).
//!
//! **It stops on an answer, not on a clock.** `Rejoin::Gone` — the far
//! machine answered and the session is not there — ends the attachment,
//! and 「结束了」 is then the truth. `Rejoin::Unreached` is asked again:
//! at once after a [`kick`] (a resume's new ears), else after a beat that
//! doubles up to [`LONGEST`] — never in a loop that spins, and while the
//! app is suspended nothing runs at all.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use khor_node::link::Rejoin;
use khor_node::{Node, SessionId};

use crate::miss::{Miss, MissKind};

static ON: AtomicBool = AtomicBool::new(true);

/// The first wait after a rejoin that did not reach.
const FIRST: Duration = Duration::from_secs(1);
/// The longest wait between two tries; a [`kick`] cuts any wait short.
pub const LONGEST: Duration = Duration::from_secs(30);

/// Switches rejoining on or off for this process (on unless told).
pub fn set_on(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Whether a far attachment whose pipe ends asks again.
pub fn is_on() -> bool {
    ON.load(Ordering::Relaxed)
}

fn kicks() -> &'static (Mutex<u64>, Condvar) {
    static KICKS: OnceLock<(Mutex<u64>, Condvar)> = OnceLock::new();
    KICKS.get_or_init(|| (Mutex::new(0), Condvar::new()))
}

/// Cuts every rejoin's wait short: the resident has new ears, so a try
/// now can reach where the last one could not.
pub fn kick() {
    let (count, woke) = kicks();
    *count
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
    woke.notify_all();
}

/// How many [`kick`]s there have been, for [`wait_since`].
fn kicked() -> u64 {
    *kicks()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Waits `beat`, or until the next [`kick`].
#[cfg(test)]
fn wait(beat: Duration) {
    wait_since(kicked(), beat);
}

/// Waits `beat`, or until a [`kick`] after the count `seen` — at once
/// when one came already. **A kick is not lost to a try in flight**
/// (net-r): the resident's new ears land while a rejoin is still waiting
/// on the old ones, and a wait that counted kicks only from its own start
/// slept its whole beat after it (up to [`LONGEST`]) — the terminal that
/// stayed 「正在重新连接」 until the person left it and opened it again.
fn wait_since(seen: u64, beat: Duration) {
    let (count, woke) = kicks();
    let guard = count
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ = woke.wait_timeout_while(guard, beat, |now| *now == seen);
}

/// [`Node::rejoin`] from a reader's own thread — a plain thread, so a
/// runtime of its own is safe to build here (the reason `reach_blocking`
/// spawns one is that its caller may be async; this one never is).
fn rejoin_once(root: &Path, machine: &str, id: &SessionId) -> Result<(String, String), Rejoin> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Rejoin::Unreached(khor_catalog::msg::runtime_wont_start(e)))?;
    let n = Node::open(root.to_path_buf()).map_err(Rejoin::Unreached)?;
    rt.block_on(n.rejoin(machine, id))
}

/// Asks for the session again until it is reached, it is gone, or
/// nobody holds the attachment any more (`left`). `attach` turns an
/// address and cookie into the host connection (the terminal's and the
/// conversation's handshakes differ only in the size they send).
///
/// **Each try that does not reach is told, in parts** (`told`,
/// NOTICE-BRIEF): the machine not answering at all (`unreached`), or
/// answering with a pipe whose host does not (`host-lost`) — both read
/// as 「正在重新连接」 on the pill, and the words go under 详情.
pub fn again<T>(
    root: &Path,
    machine: &str,
    id: &SessionId,
    left: &AtomicBool,
    attach: impl Fn(&str, &str) -> Result<T, String>,
    told: impl Fn(Miss),
) -> Option<T> {
    again_with(|| rejoin_once(root, machine, id), machine, left, attach, told)
}

/// [`again`] with the reach itself given (`try_once`), so a test can
/// stand in for the far machine.
fn again_with<T>(
    mut try_once: impl FnMut() -> Result<(String, String), Rejoin>,
    machine: &str,
    left: &AtomicBool,
    attach: impl Fn(&str, &str) -> Result<T, String>,
    told: impl Fn(Miss),
) -> Option<T> {
    let mut beat = FIRST;
    loop {
        if left.load(Ordering::Relaxed) {
            return None;
        }
        // Counted before the try: a kick while it runs is the next try's.
        let seen = kicked();
        match try_once() {
            Ok((addr, cookie)) => {
                // A pipe that was bound but whose handshake fails is the
                // same as not reached: the far host is not answering
                // through it yet.
                match attach(&addr, &cookie) {
                    Ok(conn) => return Some(conn),
                    Err(why) => told(Miss::new(MissKind::HostLost, Some(machine), why)),
                }
            }
            Err(Rejoin::Gone(_)) => return None,
            Err(Rejoin::Unreached(why)) => {
                let mut miss = crate::miss::read(&why, Some(machine));
                miss.kind = MissKind::Unreached;
                told(miss);
            }
        }
        wait_since(seen, beat);
        // After a kick the ladder starts again: new ears are a new road.
        beat = if kicked() != seen { FIRST } else { (beat * 2).min(LONGEST) };
    }
}

/// **Back to the foreground** (docs/IOS.md 第三步): the resident on
/// `root` gets new ears, every far attachment waiting to rejoin tries at
/// once on them, and the round that follows is answered — so the caller
/// can tell the face its lists are worth reading again. `None` when no
/// resumable resident holds that home in this process.
pub async fn resume(root: &Path) -> Option<Result<Vec<(String, Result<String, String>)>, String>> {
    Some(resumed(khor_node::link::resume(root)?).await)
}

/// The rest of a [`resume`] already asked for — for a caller that has to
/// send the ask itself, in order with its other asks to the resident
/// (the phone's lifecycle worker), and wait for the rest elsewhere.
pub async fn resumed(
    mut resuming: khor_node::link::Resuming,
) -> Result<Vec<(String, Result<String, String>)>, String> {
    mended(&mut resuming).await?;
    resuming.synced().await
}

/// The first half of [`resumed`]: the resident answering again, and every
/// far attachment waiting to rejoin told so ([`kick`]). The caller can say
/// so to the face at once, then follow the round machine by machine
/// (`Resuming::answered`) rather than waiting on the slowest (net-r).
pub async fn mended(resuming: &mut khor_node::link::Resuming) -> Result<(), String> {
    let ears = resuming.ears().await;
    kick();
    ears
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    /// A kick ends a wait at once; without one the wait is the beat.
    #[test]
    fn a_kick_cuts_the_wait_short() {
        let begun = Instant::now();
        let waiter = std::thread::spawn(|| wait(Duration::from_secs(20)));
        std::thread::sleep(Duration::from_millis(100));
        kick();
        waiter.join().unwrap();
        assert!(begun.elapsed() < Duration::from_secs(5), "the kick woke it");
        let begun = Instant::now();
        wait(Duration::from_millis(150));
        assert!(
            begun.elapsed() >= Duration::from_millis(150),
            "no kick: the whole beat"
        );
    }

    /// **A kick that lands while a try is in flight cuts the wait after
    /// it** (net-r). Three tries miss; the third is where the resident's
    /// new ears come (a kick), and the fourth must follow at once rather
    /// than after the 4 s the ladder had reached. Before the fix the gap
    /// was the whole beat.
    #[test]
    fn a_kick_during_a_try_is_not_lost() {
        let left = AtomicBool::new(false);
        let tries = Mutex::new(Vec::<Instant>::new());
        let got = again_with(
            || {
                let mut t = tries.lock().unwrap();
                t.push(Instant::now());
                match t.len() {
                    3 => {
                        kick();
                        Err(Rejoin::Unreached("on the old ears".into()))
                    }
                    4 => Ok(("addr".into(), "cookie".into())),
                    _ => Err(Rejoin::Unreached("not yet".into())),
                }
            },
            "far",
            &left,
            |_, _| Ok(()),
            |_| {},
        );
        assert!(got.is_some());
        let t = tries.lock().unwrap();
        let gap = t[3] - t[2];
        assert!(gap < Duration::from_millis(500), "the try after the kick came {gap:?} later");
    }

    /// An attachment nobody holds is not asked for again.
    #[test]
    fn a_left_attachment_is_not_asked_again() {
        let left = Arc::new(AtomicBool::new(true));
        let root = std::env::temp_dir().join("khor-rejoin-left-never-opened");
        let got: Option<()> = again(&root, "far", &SessionId("x".into()), &left, |_, _| Ok(()), |_| {});
        assert!(got.is_none());
        assert!(!root.exists(), "nothing was opened for it");
    }
}
