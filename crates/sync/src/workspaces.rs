//! Workspaces: the directories a person declared they work in, one
//! flag per (machine, absolute path), replicated everywhere.
//!
//! # Declared, never derived
//!
//! khor used to *infer* this list — fold the vendors' session records
//! up to the nearest `.git` and call every checkout a workspace. The
//! user threw that out on 2026-09-14, and the two sentences are worth
//! keeping because nothing on a disk can answer them:
//! 「用户有agent记录的目录不一定是工作区.可能只是随手打开的」
//! 「git目录也不一定是工作区.很多工作区开始的时候不是git目录,或用户
//! 根本就没想着用git管理」. A directory becomes a workspace because a
//! person said so, which makes it a decision — and decisions replicate
//! (docs/NET.md), the same class as [`crate::pins`] and
//! [`crate::titles`].
//!
//! # Why this key carries a machine
//!
//! [`crate::dirpins`]'s reason, and here it is the identity itself: a
//! workspace **is** `(machine, directory)`. `~/working/khor` on the
//! desk and on the laptop are two workspaces that happen to hold the
//! same repository, and a machineless key would merge them into one row
//! that opens on whichever disk answered first. So the key is
//! `<device_hex>:<path>` — the id and not the name, because a rename
//! must not orphan every row a person built. A device hex is
//! fixed-width lowercase hex, so the first `:` splits without ambiguity
//! no matter what the path contains.
//!
//! **No name of its own.** The row's name is the path's last component
//! (`~/working/test` → `test`), worked out where it is shown. A stored
//! name would be a second thing to keep in step with a directory that
//! can be renamed underneath it, and the name is not what identifies
//! the row anyway — the path is.
//!
//! # Three states on one register, and why removal writes a value
//!
//! 不是工作区 / 是 / 是且置顶. One LWW register rather than a flag plus
//! a pin table: a pin on a row that is not a workspace means nothing,
//! and two tables could hold exactly that. Sorting lives at the reader
//! ([`State::Pinned`] first), which is the only place that knows what
//! else is on the screen.
//!
//! **Removing is a write**, [`crate::titles`]'s rule: taking a
//! workspace off the list is a decision and has to travel, or the
//! device that removed it keeps it gone while every other device keeps
//! showing it. So the setter is plain LWW in both directions and never
//! monotonic.
//!
//! **And the write is [`State::No`], not a deleted key.** Not because a
//! deletion would fail to replicate — loro carries one — but because
//! the register would then have two kinds of write with two merge rules
//! to reason about where one will do, and because the reader must not
//! be able to tell 「removed」 from 「never declared」: one register,
//! one rule, and `No` and absent are one answer.
//!
//! **Removing does not touch the directory** (user, 2026-09-14): this
//! table is a list of what a person is working on, not a filesystem.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory (same reasoning as
/// `chat::REL_DIR`).
pub const REL_DIR: &str = ".khor/workspaces";

pub fn workspaces_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const WORKSPACES: &str = "workspaces";

/// What one key says. The numbers are the wire spelling and must not be
/// reordered — they are what is already written in stores on other
/// machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Not a workspace — never declared one, or removed from the list.
    /// The two read the same on purpose (module head).
    No,
    /// A workspace.
    Yes,
    /// A workspace, and it leads the list.
    Pinned,
}

impl State {
    fn code(self) -> i64 {
        match self {
            State::No => 0,
            State::Yes => 1,
            State::Pinned => 2,
        }
    }

    /// A code this version does not know reads as [`State::No`] rather
    /// than as a guess: an unknown state painted as a workspace would
    /// put a row on the screen that no reader here can act on.
    fn of(code: i64) -> State {
        match code {
            1 => State::Yes,
            2 => State::Pinned,
            _ => State::No,
        }
    }
}

/// The table's one key spelling. The split is [`split`]; nothing else
/// may take the key apart, or two readers disagree about what a row is.
pub fn key(device_hex: &str, path: &str) -> String {
    format!("{device_hex}:{path}")
}

/// `(device_hex, path)` back out of a key. `None` for a key some other
/// version wrote in a spelling this one does not know — skipped, never
/// guessed at.
pub fn split(key: &str) -> Option<(&str, &str)> {
    key.split_once(':')
}

pub struct WorkspaceDoc {
    inner: LoroDoc,
}

impl WorkspaceDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self {
            inner: glue::with_peer(peer)?,
        })
    }

    pub fn state(&self, key: &str) -> State {
        match self
            .inner
            .get_map(WORKSPACES)
            .get(key)
            .and_then(|v| v.into_value().ok())
        {
            Some(LoroValue::I64(n)) => State::of(n),
            _ => State::No,
        }
    }

    /// Unchanged means unwritten — this sits behind a click, and
    /// rewriting the same value inflates the version vector for nothing
    /// (`DeviceDoc::upsert`'s rule). Removing something that was never
    /// here writes nothing either: there is no decision to travel.
    pub fn set(&self, key: &str, state: State) -> Result<(), String> {
        if self.state(key) == state {
            return Ok(());
        }
        self.inner
            .get_map(WORKSPACES)
            .insert(key, state.code())
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every key that is a workspace today, sorted — which groups by
    /// machine for free, the device hex being the prefix. Removed keys
    /// are not rows; they are tombstones that exist to travel.
    pub fn all(&self) -> Vec<(String, State)> {
        let map = self.inner.get_map(WORKSPACES);
        let mut out: Vec<(String, State)> = map
            .keys()
            .filter_map(|k| match self.state(&k) {
                State::No => None,
                st => Some((k.to_string(), st)),
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

impl Doc for WorkspaceDoc {
    fn open(peer: u64) -> Result<Self, String> {
        WorkspaceDoc::new(peer)
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

    /// Workspaces in force, not keys ever written — a removed one is
    /// not an item, and the sync line that prints this would otherwise
    /// count rows nobody can see (`TitleDoc::items`'s rule).
    fn items(&self) -> usize {
        self.all().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The split must survive a path with a colon in it — the whole
    /// reason the machine half is fixed-form hex.
    #[test]
    fn the_key_splits_on_the_machine_boundary_not_inside_the_path() {
        let k = key("abc123", "/tmp/odd:name/dir");
        assert_eq!(split(&k), Some(("abc123", "/tmp/odd:name/dir")));
    }

    /// All three states are distinguishable and all three travel. The
    /// middle one is the one a two-table design would have lost: pinned
    /// must be able to go back to plain without leaving the list.
    #[test]
    fn the_three_states_round_trip_and_pinning_is_not_a_second_table() {
        let phone = WorkspaceDoc::new(1).unwrap();
        let desk = WorkspaceDoc::new(2).unwrap();
        let k = key("d1e6", "/Users/x/working/test");

        assert_eq!(
            phone.state(&k),
            State::No,
            "never declared = not a workspace"
        );
        phone.set(&k, State::Yes).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(desk.state(&k), State::Yes, "declaring it travels");
        assert_eq!(desk.all(), vec![(k.clone(), State::Yes)]);

        desk.set(&k, State::Pinned).unwrap();
        phone
            .merge(&desk.changes_since(&phone.version()).unwrap())
            .unwrap();
        assert_eq!(phone.state(&k), State::Pinned, "pinning travels back");

        phone.set(&k, State::Yes).unwrap();
        desk.merge(&phone.changes_since(&desk.version()).unwrap())
            .unwrap();
        assert_eq!(
            desk.state(&k),
            State::Yes,
            "unpinning leaves it a workspace"
        );
        assert_eq!(desk.items(), 1);
    }

    /// **Removal is a write and it travels.** A setter that only
    /// stopped listing the row here would leave every other device
    /// showing it, and the next round would bring it back.
    ///
    /// The second half is the concurrent case, which is the one a
    /// removal shares with every other state: 置顶 on one device while
    /// the other removes it must settle on **one** answer, and which
    /// one matters less than that both sides agree.
    #[test]
    fn removing_a_workspace_travels_and_reads_as_never_declared() {
        let a = WorkspaceDoc::new(1).unwrap();
        let b = WorkspaceDoc::new(2).unwrap();
        let k = key("beef", "/srv/thing");
        a.set(&k, State::Pinned).unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(
            b.state(&k),
            State::Pinned,
            "control: it arrived a workspace"
        );

        b.set(&k, State::No).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        assert_eq!(
            a.state(&k),
            State::No,
            "the removal must reach the other side too"
        );
        assert!(a.all().is_empty(), "a removed workspace is not a row");
        assert_eq!(a.items(), 0, "and not an item");

        let j = key("beef", "/srv/other");
        a.set(&j, State::Yes).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();
        a.set(&j, State::Pinned).unwrap();
        b.set(&j, State::No).unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();
        assert_eq!(
            a.state(&j),
            b.state(&j),
            "both sides must land on one answer"
        );
    }

    /// A no-change write grows nothing: this sits behind a click, and
    /// pressing 置顶 on something already pinned must not cost a block.
    #[test]
    fn writing_the_same_state_writes_nothing() {
        let d = WorkspaceDoc::new(1).unwrap();
        let k = key("cafe", "/w/one");
        d.set(&k, State::Yes).unwrap();
        let before = d.version();
        d.set(&k, State::Yes).unwrap();
        assert_eq!(d.version(), before, "no change must mean no new ops");

        // …and removing what was never here writes nothing either: a
        // tombstone with no decision behind it is a row-shaped key that
        // will be merged forever.
        let fresh = WorkspaceDoc::new(3).unwrap();
        let empty = fresh.version();
        fresh.set(&key("cafe", "/w/never"), State::No).unwrap();
        assert_eq!(
            fresh.version(),
            empty,
            "removing what was never declared writes nothing"
        );
    }
}
