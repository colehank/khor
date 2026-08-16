//! Pinned sessions: one flag per session id, replicated everywhere
//! (docs/NET.md — the same decision class as [`crate::seen`]).
//!
//! Shape: `pins : Map<session_id, bool>` (per-key LWW).
//!
//! # Why this replicates instead of sitting in a client preference
//!
//! A pin is a property of **the thing pinned**, not of the screen looking
//! at it. Kept per client, two phones hold two answers and both are
//! right — there is no way to say which one counts. Replicated, "this
//! session matters" is one fact the whole network agrees on: pin on the
//! phone, the desktop list has it pinned on the next pump.
//!
//! # Why not the same table as machine pins
//!
//! Machine pins ride the device table, keyed by device id; these are
//! keyed by session id. Folding both into one map compiles fine — the
//! keys are strings either way — and the cost is that from then on
//! nobody can say what a row in that table pins. One kind of landing,
//! one table, one key.
//!
//! # Why [`PinDoc::set`] is not monotonic, unlike [`crate::seen`]
//!
//! Seen refuses a lower watermark, because a badge that bounces back to
//! unread is the one thing the badge rules forbid. A pin has no such
//! ordering: **unpinning is a thing the user does on purpose**, and a
//! rule that only ever let the flag rise would make it impossible. So
//! this is plain LWW, and two devices that concurrently pin and unpin
//! settle on one of the two — which is the point of replicating it at
//! all. Both sides end up saying the same thing; a per-client store is
//! what leaves them disagreeing forever.
//!
//! # A pin outlives the row it points at, and that is fine
//!
//! Only the key is stored, never a copy of the session. A pinned session
//! whose process is gone (or whose machine left the network) simply has
//! no row to pin: readers look the flag up **by the ids they are already
//! showing**, so a stale pin paints nothing and raises nothing. Storing
//! the row alongside would mean the list could show a session that no
//! longer exists.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory (same reasoning as
/// `chat::REL_DIR`).
pub const REL_DIR: &str = ".khor/pins";

pub fn pins_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const PINS: &str = "pins";

pub struct PinDoc {
    inner: LoroDoc,
}

impl PinDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    /// Whether this session is pinned. Never pinned and explicitly
    /// unpinned read the same, on purpose: there is nothing a reader
    /// would do differently, and the difference would have to travel.
    pub fn pinned(&self, session: &str) -> bool {
        matches!(
            self.inner.get_map(PINS).get(session).and_then(|v| v.into_value().ok()),
            Some(LoroValue::Bool(true))
        )
    }

    /// Pins or unpins. Unchanged means unwritten: this is reachable from
    /// a click, and rewriting the same value inflates the version vector
    /// for nothing (same rule as `DeviceDoc::upsert`).
    pub fn set(&self, session: &str, on: bool) -> Result<(), String> {
        if self.pinned(session) == on {
            return Ok(());
        }
        self.inner
            .get_map(PINS)
            .insert(session, on)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every session id currently pinned, sorted. For the CLI and for
    /// tests — the list itself asks by id, one row at a time.
    pub fn all(&self) -> Vec<String> {
        let map = self.inner.get_map(PINS);
        let mut out: Vec<String> = map
            .keys()
            .filter(|k| self.pinned(k))
            .map(|k| k.to_string())
            .collect();
        out.sort();
        out
    }
}

impl Doc for PinDoc {
    fn open(peer: u64) -> Result<Self, String> {
        PinDoc::new(peer)
    }

    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }

    fn version(&self) -> VersionVector {
        self.inner.oplog_vv()
    }

    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        glue::changes_since(&self.inner, theirs)
    }

    fn snapshot(&self) -> Result<Vec<u8>, String> {
        glue::snapshot(&self.inner)
    }

    fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        glue::merge(&self.inner, bytes)
    }

    /// Pinned sessions, not keys ever written: an unpinned key is not an
    /// item, and the sync line that prints this would otherwise count
    /// things the user removed.
    fn items(&self) -> usize {
        self.all().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pin_round_trips_and_defaults_to_unpinned() {
        let d = PinDoc::new(1).unwrap();
        assert!(!d.pinned("tui/abc"), "never pinned = not pinned");
        d.set("tui/abc", true).unwrap();
        assert!(d.pinned("tui/abc"));
        assert_eq!(d.all(), vec!["tui/abc".to_string()]);
        assert_eq!(d.items(), 1);
    }

    /// The promise: pin on one device, pinned on the other. This is the
    /// whole reason it is a document instead of a preference file.
    #[test]
    fn pinning_on_one_device_shows_up_on_the_other_after_merge() {
        let phone = PinDoc::new(1).unwrap();
        let desk = PinDoc::new(2).unwrap();
        phone.set("chat/mac", true).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap()).unwrap();
        assert!(desk.pinned("chat/mac"), "a pin made on the phone must reach the desk");
    }

    /// Unpinning has to travel too — a flag that could only ever rise
    /// would leave the user unable to take a pin back from anywhere but
    /// the device that set it. This is why the setter is not monotonic
    /// the way `seen::mark` is.
    #[test]
    fn unpinning_travels_as_well_as_pinning() {
        let a = PinDoc::new(1).unwrap();
        let b = PinDoc::new(2).unwrap();
        a.set("tui/x", true).unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap()).unwrap();
        assert!(b.pinned("tui/x"), "control: it arrived pinned");

        b.set("tui/x", false).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        assert!(!a.pinned("tui/x"), "the removal must reach the other side too");
        assert!(a.all().is_empty(), "an unpinned session is not on the list");
    }

    /// Two devices disagreeing settle on one answer on both sides. What
    /// matters is not which one wins — it is that they agree, which a
    /// per-client store never does.
    #[test]
    fn a_concurrent_pin_and_unpin_settle_the_same_way_on_both_sides() {
        let a = PinDoc::new(1).unwrap();
        let b = PinDoc::new(2).unwrap();
        a.set("tui/y", true).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();

        a.set("tui/y", false).unwrap();
        b.set("tui/y", true).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();
        assert_eq!(a.pinned("tui/y"), b.pinned("tui/y"), "both sides must land on one answer");
    }

    /// A no-change write grows nothing: this sits behind a button, and a
    /// double click must not cost a block.
    #[test]
    fn setting_the_same_value_writes_nothing() {
        let d = PinDoc::new(1).unwrap();
        d.set("tui/z", true).unwrap();
        let before = d.version();
        d.set("tui/z", true).unwrap();
        assert_eq!(d.version(), before, "no change must mean no new ops");
        // …and the same for the unpinned side of the flag
        let fresh = PinDoc::new(3).unwrap();
        let empty = fresh.version();
        fresh.set("tui/never", false).unwrap();
        assert_eq!(fresh.version(), empty, "unpinning what was never pinned writes nothing");
    }

    /// Pins are keyed by session id and nothing else — two sessions are
    /// two independent flags, so pinning one must not move the other.
    #[test]
    fn each_session_carries_its_own_flag() {
        let d = PinDoc::new(1).unwrap();
        d.set("tui/one", true).unwrap();
        assert!(!d.pinned("tui/two"), "pinning one session must not pin another");
        d.set("tui/two", true).unwrap();
        d.set("tui/one", false).unwrap();
        assert_eq!(d.all(), vec!["tui/two".to_string()]);
    }
}
