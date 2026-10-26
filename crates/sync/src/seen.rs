//! Read state: one watermark per session, replicated everywhere
//! (docs/NET.md: seen is a decision — clear on the phone, the
//! desktop badge clears too).
//!
//! Shape: `seen : Map<session_id, watermark_ms>` retains broad CLI marks.
//! `snapshots : Map<[session_id, message_id, at_ms], true>` acknowledges only
//! versions actually rendered by the GUI. Immutable version keys merge as a
//! union: a stale receipt cannot erase a newer one, and equal timestamps on
//! different messages stay independent. A new version of the same message
//! requires a new receipt. GUI snapshots never advance the broad watermark.
//!
//! **Whose decision is it — the person's or the machine's?** The
//! person's. One watermark per session, replicated, so a look on the
//! phone clears the desk; there is deliberately no per-device copy. That
//! is what lets "telling implies looking" live in here: a line's content
//! *is* the line, so the person who typed it has seen it on every
//! machine they own (`khor_node`'s `tests/pairing.rs` pins exactly that).
//!
//! The two readings coincide almost everywhere, and the one place they
//! do not is worth naming here, because the answer is not obvious from
//! either side. A **file** message carries no content — it carries a
//! summary of bytes that have not arrived anywhere yet, and *"it arrived
//! here"* is a fact about a machine. So offering a file is not looking
//! at one, on any seat, and `khor_node::Node::send` marks nothing (it
//! used to mark, and the offering machine cleared the badge on the
//! machine the file was for). Anything a *seat* knows — this machine
//! wrote that, this machine holds those bytes — belongs where it is
//! derived, never in this document: what is written here travels to
//! every machine and speaks for all of them.
//!
//! **That divergence closes again once the bytes land**, and knowing
//! where it closes is what keeps this document from growing a
//! per-machine half. Before the landing, the thing wanted is an
//! approval, and it is already asked for by a signal that owes this
//! document nothing (待批, from the seat whose approval it is). After
//! the landing there is only a file sitting on a disk — a fact, not a
//! demand — and facts are the person's to have seen, from any machine.
//! So a watermark that cannot say "everything before T *except* that
//! file" is not missing anything: the badge counts what wants you, and
//! a landed file in a conversation you have since spoken in wants
//! nothing. Making it want something would take a mark that does not
//! replicate, and a badge that clears only by walking to one particular
//! machine is a badge that is never zero.
//!
//! A second map rides the same document: `closed : Map<session_id,
//! watermark_ms>` — how far a conversation has been *put away*, not
//! merely looked at (docs/SESSION.md 动作: close 对话 = 收起那一行).
//! Same clock, same monotonic rule, one degree stronger — which is why
//! it lives here rather than in a document of its own: the two are one
//! family of decisions about one row, and a new doc would be a new name
//! on the wire that every older peer refuses whole, while a new map in
//! this one replicates through them untouched (loro merges maps it was
//! never asked to read).

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
const CLOSED: &str = "closed";
const SNAPSHOTS: &str = "snapshots";

/// One rendered message version; `at_ms` is the message's sender timestamp.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct SeenMessage {
    pub id: String,
    #[ts(type = "number")]
    pub at_ms: i64,
}

fn version_key(session: &str, message: &str, at_ms: i64) -> String {
    // JSON tuple encoding makes embedded slashes and separators unambiguous.
    serde_json::to_string(&(session, message, at_ms)).expect("string tuple serializes")
}

pub struct SeenDoc {
    inner: LoroDoc,
}

impl SeenDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self {
            inner: glue::with_peer(peer)?,
        })
    }

    /// The session's watermark; 0 = never seen.
    pub fn watermark(&self, session: &str) -> i64 {
        match self
            .inner
            .get_map(SEEN)
            .get(session)
            .and_then(|v| v.into_value().ok())
        {
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

    /// A GUI receipt covers exactly this version, not other messages with
    /// the same timestamp and not later updates to this message.
    pub fn message_seen(&self, session: &str, message: &str, at_ms: i64) -> bool {
        self.inner
            .get_map(SNAPSHOTS)
            .get(&version_key(session, message, at_ms))
            .is_some()
    }

    pub fn mark_message(&self, session: &str, message: &str, at_ms: i64) -> Result<(), String> {
        if self.message_seen(session, message, at_ms) {
            return Ok(());
        }
        self.inner
            .get_map(SNAPSHOTS)
            .insert(&version_key(session, message, at_ms), true)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// How far this conversation has been put away, or `None` — never
    /// closed. `Some(0)` and `None` are different answers on purpose: a
    /// conversation with no messages yet closes at watermark 0, and
    /// folding that into "never closed" would make an empty row the one
    /// row that cannot be put away.
    pub fn closed_watermark(&self, session: &str) -> Option<i64> {
        match self
            .inner
            .get_map(CLOSED)
            .get(session)
            .and_then(|v| v.into_value().ok())
        {
            Some(LoroValue::I64(n)) => Some(n),
            _ => None,
        }
    }

    /// Puts the conversation away up to `at_ms` on the senders' clocks.
    /// The row stops being derived until a message stamps past the mark
    /// (`khor_node` ChatKind rows) — nothing is deleted, and the next
    /// message brings the row back on every device at once. Monotonic
    /// like [`Self::mark`], and for the same reason: a stale mark from
    /// a late merge must not re-hide what a newer message revived.
    pub fn put_away(&self, session: &str, at_ms: i64) -> Result<(), String> {
        if self.closed_watermark(session).is_some_and(|w| at_ms <= w) {
            return Ok(());
        }
        self.inner
            .get_map(CLOSED)
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

    fn merge_report(&self, bytes: &[u8]) -> Result<crate::store::MergeReport, String> {
        glue::merge_report(&self.inner, bytes)
    }

    fn items(&self) -> usize {
        self.inner.get_map(SEEN).keys().count() + self.inner.get_map(SNAPSHOTS).keys().count()
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
        desk.merge(&phone.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(
            desk.watermark("chat/mac"),
            500,
            "seen on the phone must clear on the desk"
        );
    }

    /// `Some(0)` and `None` are different answers: an empty conversation
    /// closes at watermark 0, and that close must be readable back.
    #[test]
    fn putting_away_at_zero_is_not_never_closed() {
        let d = SeenDoc::new(1).unwrap();
        assert_eq!(d.closed_watermark("chat/mac"), None, "never closed = None");
        d.put_away("chat/mac", 0).unwrap();
        assert_eq!(d.closed_watermark("chat/mac"), Some(0));
    }

    /// Monotonic like the seen mark, and op-free when it is a no-op —
    /// a stale put-away from a late merge must not re-hide a row a
    /// newer message revived.
    #[test]
    fn a_lower_put_away_never_lowers_the_closed_watermark() {
        let d = SeenDoc::new(1).unwrap();
        d.put_away("chat/mac", 100).unwrap();
        let before = d.version();
        d.put_away("chat/mac", 50).unwrap();
        assert_eq!(d.closed_watermark("chat/mac"), Some(100));
        assert_eq!(d.version(), before, "a lower put-away must not create ops");
    }

    /// The promise: close on the phone, the desktop's row folds too.
    #[test]
    fn putting_away_on_one_device_reaches_the_other_after_merge() {
        let phone = SeenDoc::new(1).unwrap();
        let desk = SeenDoc::new(2).unwrap();
        phone.put_away("chat/mac", 500).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(desk.closed_watermark("chat/mac"), Some(500));
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
        assert!(
            wa == 300 || wa == 700,
            "the settled value must be one of the marks: {wa}"
        );
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn exact_receipts_do_not_cover_equal_clock_neighbors_or_new_versions() {
        let seen = SeenDoc::new(31).unwrap();
        seen.mark_message("wide/all", "first", 100).unwrap();
        assert!(seen.message_seen("wide/all", "first", 100));
        assert!(!seen.message_seen("wide/all", "later", 100));
        assert!(!seen.message_seen("wide/all", "first", 101));
        assert_eq!(seen.watermark("wide/all"), 0);
        let version = seen.version();
        seen.mark_message("wide/all", "first", 100).unwrap();
        assert_eq!(
            seen.version(),
            version,
            "repeated rendering creates no CRDT operations"
        );
    }

    #[test]
    fn concurrent_old_and_new_version_receipts_merge_without_regression() {
        let phone = SeenDoc::new(32).unwrap();
        let desk = SeenDoc::new(33).unwrap();
        phone.mark_message("wide/all", "same", 200).unwrap();
        desk.mark_message("wide/all", "same", 100).unwrap();
        desk.mark_message("wide/all", "another", 200).unwrap();
        phone
            .merge(&desk.changes_since(&phone.version()).unwrap())
            .unwrap();
        desk.merge(&phone.changes_since(&desk.version()).unwrap())
            .unwrap();
        for seen in [&phone, &desk] {
            assert!(seen.message_seen("wide/all", "same", 100));
            assert!(seen.message_seen("wide/all", "same", 200));
            assert!(seen.message_seen("wide/all", "another", 200));
            assert!(!seen.message_seen("wide/all", "same", 201));
        }
    }
}
