//! User-given session names: one override per session id, replicated
//! everywhere (docs/NET.md — the same decision class as [`crate::pins`]
//! and [`crate::seen`]).
//!
//! Shape: `titles : Map<session_id, string>` (per-key LWW).
//!
//! # An override, never the title itself
//!
//! Rows keep deriving their own titles underneath — a discovered agent's
//! is recomputed from the vendor's status file on every sweep, a
//! khor-opened row carries the title it was opened with. This document
//! holds only what a person typed over that, and readers apply it at the
//! surface (`Node::sessions`). Two things fall out of that split:
//! the derived name cannot fight the override (the next sweep rewrites
//! meta, not this table), and taking the override back costs nothing —
//! the derived name was flowing the whole time and simply shows again.
//!
//! # Why the empty string is the eraser
//!
//! Clearing a name is a decision that must travel like any other, or the
//! device that cleared it keeps seeing the fallback while every other
//! device keeps the old override. So clearing writes `""` rather than
//! deleting the key — the same LWW register carries both directions, the
//! way a pin and an unpin share one flag ([`crate::pins`] on why the
//! setter must not be monotonic). Readers treat empty and absent as one
//! answer: no override. Nothing a reader would do differs between the
//! two, and the difference would otherwise have to travel.
//!
//! # The key is the bare session id
//!
//! The same network-wide bet as pins and seen, spelled out in
//! [`crate::pins`]'s module head: ids are unique enough, a collision
//! costs one visible wrong name and the user can rename it away, and the
//! fix if it ever matters is in how ids are minted — not in this key.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory (same reasoning as
/// `chat::REL_DIR`).
pub const REL_DIR: &str = ".khor/titles";

pub fn titles_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const TITLES: &str = "titles";

pub struct TitleDoc {
    inner: LoroDoc,
}

impl TitleDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    /// The user's name for this session, or `None` when the derived
    /// title stands. Empty and never-named read the same, on purpose
    /// (module head).
    pub fn title_of(&self, session: &str) -> Option<String> {
        match self.inner.get_map(TITLES).get(session).and_then(|v| v.into_value().ok()) {
            Some(LoroValue::String(s)) if !s.is_empty() => Some(s.to_string()),
            _ => None,
        }
    }

    /// Names or, with an empty (or all-whitespace) title, un-names.
    /// Unchanged means unwritten: this is reachable from a keystroke,
    /// and rewriting the same value inflates the version vector for
    /// nothing (same rule as `PinDoc::set`).
    pub fn set(&self, session: &str, title: &str) -> Result<(), String> {
        let title = title.trim();
        if self.title_of(session).as_deref().unwrap_or("") == title {
            return Ok(());
        }
        self.inner
            .get_map(TITLES)
            .insert(session, title)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }
}

impl Doc for TitleDoc {
    fn open(peer: u64) -> Result<Self, String> {
        TitleDoc::new(peer)
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

    /// Overrides in force, not keys ever written: a cleared name is not
    /// an item, and the sync line that prints this would otherwise count
    /// names the user took back.
    fn items(&self) -> usize {
        let map = self.inner.get_map(TITLES);
        map.keys().filter(|k| self.title_of(k).is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_round_trips_and_defaults_to_none() {
        let d = TitleDoc::new(1).unwrap();
        assert_eq!(d.title_of("tui/abc"), None, "never named = no override");
        d.set("tui/abc", "  paying the rent  ").unwrap();
        assert_eq!(d.title_of("tui/abc").as_deref(), Some("paying the rent"), "trimmed on the way in");
        assert_eq!(d.items(), 1);
    }

    /// The promise: name it on the phone, the desk shows that name. This
    /// is the whole reason it is a document instead of a preference file.
    #[test]
    fn renaming_on_one_device_shows_up_on_the_other_after_merge() {
        let phone = TitleDoc::new(1).unwrap();
        let desk = TitleDoc::new(2).unwrap();
        phone.set("chat/mac", "the mac").unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap()).unwrap();
        assert_eq!(desk.title_of("chat/mac").as_deref(), Some("the mac"));
    }

    /// Clearing has to travel too, or the device that cleared keeps the
    /// fallback while every other device keeps the old override. This is
    /// why clearing writes `""` instead of deleting nothing.
    #[test]
    fn clearing_a_name_travels_and_reads_as_no_override() {
        let a = TitleDoc::new(1).unwrap();
        let b = TitleDoc::new(2).unwrap();
        a.set("tui/x", "my name").unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap()).unwrap();
        assert_eq!(b.title_of("tui/x").as_deref(), Some("my name"), "control: it arrived named");

        b.set("tui/x", "").unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        assert_eq!(a.title_of("tui/x"), None, "the clearing must reach the other side too");
        assert_eq!(a.items(), 0, "a cleared name is not an item");
    }

    /// Two devices disagreeing settle on one answer on both sides. Which
    /// one wins matters less than that they agree — a per-client store
    /// never does.
    #[test]
    fn concurrent_renames_settle_the_same_way_on_both_sides() {
        let a = TitleDoc::new(1).unwrap();
        let b = TitleDoc::new(2).unwrap();
        a.set("tui/y", "from a").unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();

        a.set("tui/y", "a again").unwrap();
        b.set("tui/y", "b instead").unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();
        assert_eq!(a.title_of("tui/y"), b.title_of("tui/y"), "both sides must land on one answer");
        let settled = a.title_of("tui/y").unwrap();
        assert!(
            settled == "a again" || settled == "b instead",
            "the settled name must be one of the two: {settled}"
        );
    }

    /// A no-change write grows nothing: this sits behind a keystroke,
    /// and re-submitting the same name must not cost a block.
    #[test]
    fn setting_the_same_name_writes_nothing() {
        let d = TitleDoc::new(1).unwrap();
        d.set("tui/z", "kept").unwrap();
        let before = d.version();
        d.set("tui/z", "kept").unwrap();
        assert_eq!(d.version(), before, "no change must mean no new ops");
        // …and clearing what was never named writes nothing either.
        let fresh = TitleDoc::new(3).unwrap();
        let empty = fresh.version();
        fresh.set("tui/never", "").unwrap();
        assert_eq!(fresh.version(), empty, "clearing what was never named writes nothing");
    }
}
