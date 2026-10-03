//! A far terminal or conversation that **attaches again** when its pipe
//! ends, instead of ending (docs/IOS.md 第三步).
//!
//! A face attached to a session on another machine reads it through a
//! pipe the resident binds on its endpoint (`Node::reach`). On a phone
//! that pipe dies every time the app is suspended: the endpoint goes
//! deaf, and `khor_node::link::resume` gives the resident new ears on
//! the way back — which closes the old endpoint and every pipe on it.
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
//! restarted. The face says 正在重新连接 meanwhile (`TermChunk::rejoining`).
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

/// Waits `beat`, or until the next [`kick`].
fn wait(beat: Duration) {
    let (count, woke) = kicks();
    let guard = count
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let seen = *guard;
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
pub fn again<T>(
    root: &Path,
    machine: &str,
    id: &SessionId,
    left: &AtomicBool,
    attach: impl Fn(&str, &str) -> Result<T, String>,
) -> Option<T> {
    let mut beat = FIRST;
    loop {
        if left.load(Ordering::Relaxed) {
            return None;
        }
        match rejoin_once(root, machine, id) {
            Ok((addr, cookie)) => {
                // A pipe that was bound but whose handshake fails is the
                // same as not reached: the far host is not answering
                // through it yet.
                if let Ok(conn) = attach(&addr, &cookie) {
                    return Some(conn);
                }
            }
            Err(Rejoin::Gone(_)) => return None,
            Err(Rejoin::Unreached(_)) => {}
        }
        wait(beat);
        beat = (beat * 2).min(LONGEST);
    }
}

/// **Back to the foreground** (docs/IOS.md 第三步): the resident on
/// `root` gets new ears, every far attachment waiting to rejoin tries at
/// once on them, and the round that follows is answered — so the caller
/// can tell the face its lists are worth reading again. `None` when no
/// resumable resident holds that home in this process.
pub async fn resume(root: &Path) -> Option<Result<Vec<(String, Result<String, String>)>, String>> {
    let mut resuming = khor_node::link::resume(root)?;
    let ears = resuming.ears().await;
    kick();
    Some(match ears {
        Ok(()) => resuming.synced().await,
        Err(why) => Err(why),
    })
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

    /// An attachment nobody holds is not asked for again.
    #[test]
    fn a_left_attachment_is_not_asked_again() {
        let left = Arc::new(AtomicBool::new(true));
        let root = std::env::temp_dir().join("khor-rejoin-left-never-opened");
        let got: Option<()> = again(&root, "far", &SessionId("x".into()), &left, |_, _| Ok(()));
        assert!(got.is_none());
        assert!(!root.exists(), "nothing was opened for it");
    }
}
