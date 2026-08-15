//! Read state: one watermark per session, replicated everywhere
//! (docs/NET.md: seen is a decision — clear on the phone, the
//! desktop badge clears too).
//!
//! Shape: `seen : Map<session_id, watermark_ms>` (per-key LWW). A
//! watermark, not a count and not a set of message ids: a count miscounts
//! under concurrent inserts and an LWW collision bounces read back to
//! unread; an id set grows without bound, and the CRDT rule is small
//! decision-class data only (docs/NET.md). "Seen" means: everything
//! up to that instant on the *senders'* clocks — so callers mark with the
//! newest message's `at`, never the local clock: a sender running ahead
//! would otherwise stay unread after being looked at, and a badge that
//! cannot reach zero is the one thing the badge rules forbid.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory (same reasoning as
/// `chat::REL_DIR`).
pub const REL_DIR: &str = ".khor/seen";

pub fn seen_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const SEEN: &str = "seen";

pub struct SeenDoc {
    inner: LoroDoc,
}

impl SeenDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    /// The session's watermark; 0 = never seen.
    pub fn watermark(&self, session: &str) -> i64 {
        match self.inner.get_map(SEEN).get(session).and_then(|v| v.into_value().ok()) {
            Some(LoroValue::I64(n)) => n,
            _ => 0,
        }
    }

    /// Raises the watermark. Monotonic: a lower mark writes nothing, so a
    /// stale local mark can never bounce read back to unread — and a
    /// no-op mark must not grow the version vector.
    pub fn mark(&self, session: &str, at_ms: i64) -> Result<(), String> {
        if at_ms <= self.watermark(session) {
            return Ok(());
        }
        self.inner
            .get_map(SEEN)
            .insert(session, at_ms)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }
}

impl Doc for SeenDoc {
    fn open(peer: u64) -> Result<Self, String> {
        SeenDoc::new(peer)
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

    fn items(&self) -> usize {
        self.inner.get_map(SEEN).keys().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watermark_round_trips_and_defaults_to_zero() {
        let d = SeenDoc::new(1).unwrap();
        assert_eq!(d.watermark("chat/mac"), 0, "never marked = zero");
        d.mark("chat/mac", 1_700_000_000_000).unwrap();
        assert_eq!(d.watermark("chat/mac"), 1_700_000_000_000);
        assert_eq!(d.items(), 1);
    }

    /// Monotonic: a stale mark can never bounce read back to unread, and
    /// it must not create ops either.
    #[test]
    fn a_lower_mark_never_lowers_the_watermark() {
        let d = SeenDoc::new(1).unwrap();
        d.mark("chat/mac", 100).unwrap();
        let before = d.version();
        d.mark("chat/mac", 50).unwrap();
        assert_eq!(d.watermark("chat/mac"), 100);
        assert_eq!(d.version(), before, "a lower mark must not create ops");
    }

    /// The promise itself: clear on the phone, the desktop clears too.
    #[test]
    fn marking_seen_on_one_device_clears_on_the_other_after_merge() {
        let phone = SeenDoc::new(1).unwrap();
        let desk = SeenDoc::new(2).unwrap();
        phone.mark("chat/mac", 500).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap()).unwrap();
        assert_eq!(desk.watermark("chat/mac"), 500, "seen on the phone must clear on the desk");
    }

    /// Two devices mark concurrently: whichever LWW picks, the watermark
    /// stays at one of the two marks — never below both.
    #[test]
    fn concurrent_marks_settle_on_one_of_them() {
        let a = SeenDoc::new(1).unwrap();
        let b = SeenDoc::new(2).unwrap();
        a.mark("chat/mac", 300).unwrap();
        b.mark("chat/mac", 700).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();
        let (wa, wb) = (a.watermark("chat/mac"), b.watermark("chat/mac"));
        assert_eq!(wa, wb, "both sides must converge on one value");
        assert!(wa == 300 || wa == 700, "the settled value must be one of the marks: {wa}");
    }
}
