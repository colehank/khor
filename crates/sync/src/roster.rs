//! What is on the session list: the **roster** (docs/SHELL.md 第八轮,
//! 2026-09-22:「session 里放什么，应该是用户决定，不是我们自动」).
//!
//! Shape: `roster : Map<session_id, "new" | "import" | "">` (per-key LWW;
//! `""` is a row taken off).
//!
//! # What it decides, and what it does not
//!
//! Sessions khor finds on its own — an agent somebody started in their
//! own terminal, a tmux session — are **candidates**, not rows: they are
//! still swept (state, hooks and usage all read them), and a person
//! brings one onto the list by importing it. What khor starts itself
//! (a terminal it holds, an agent in that terminal) enters as `new` the
//! moment it is started. A device's own conversation, a transfer, a
//! borrow and an ACP conversation are khor's by construction and never
//! need to be here.
//!
//! # Why it replicates
//!
//! The list is one list on every machine: a session imported on the
//! laptop is on the desk's list at the next pump, and taking it off
//! anywhere takes it off everywhere. Keyed by the bare session id for
//! [`crate::pins`]' reasons (read its head for how strong that key is).
//!
//! # Taking a row off is not deleting anything
//!
//! `""` says 「not on the list」 and nothing else: the process, the
//! vendor's own files and the multiplexer are untouched, and the session
//! is a candidate again.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory.
pub const REL_DIR: &str = ".khor/roster";

pub fn roster_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const ROSTER: &str = "roster";

/// How a session came onto the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// khor started it.
    New,
    /// A person brought it in from the candidates.
    Import,
}

impl Entry {
    pub fn word(self) -> &'static str {
        match self {
            Entry::New => "new",
            Entry::Import => "import",
        }
    }
}

pub struct RosterDoc {
    inner: LoroDoc,
}

impl RosterDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self {
            inner: glue::with_peer(peer)?,
        })
    }

    /// How this session is on the list, or `None` when it is not. A word
    /// from a newer khor reads as on the list, imported — a row somebody
    /// put there is not one to hide.
    pub fn entry(&self, session: &str) -> Option<Entry> {
        match self
            .inner
            .get_map(ROSTER)
            .get(session)
            .and_then(|v| v.into_value().ok())
        {
            Some(LoroValue::String(s)) => match s.as_str() {
                "" => None,
                "new" => Some(Entry::New),
                _ => Some(Entry::Import),
            },
            _ => None,
        }
    }

    /// Puts a session on the list. Unchanged means unwritten.
    pub fn put(&self, session: &str, how: Entry) -> Result<(), String> {
        if self.entry(session) == Some(how) {
            return Ok(());
        }
        self.write(session, how.word())
    }

    /// Takes a session off the list. Unchanged means unwritten.
    pub fn take_off(&self, session: &str) -> Result<(), String> {
        if self.entry(session).is_none() {
            return Ok(());
        }
        self.write(session, "")
    }

    fn write(&self, session: &str, word: &str) -> Result<(), String> {
        self.inner
            .get_map(ROSTER)
            .insert(session, word)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every session on the list, sorted.
    pub fn all(&self) -> Vec<String> {
        let map = self.inner.get_map(ROSTER);
        let mut out: Vec<String> = map
            .keys()
            .filter(|k| self.entry(k).is_some())
            .map(|k| k.to_string())
            .collect();
        out.sort();
        out
    }
}

impl Doc for RosterDoc {
    fn open(peer: u64) -> Result<Self, String> {
        RosterDoc::new(peer)
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
        self.all().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_goes_on_and_comes_off() {
        let d = RosterDoc::new(1).unwrap();
        assert_eq!(d.entry("tui/abc"), None);
        d.put("tui/abc", Entry::Import).unwrap();
        assert_eq!(d.entry("tui/abc"), Some(Entry::Import));
        d.put("shell/1", Entry::New).unwrap();
        assert_eq!(d.all(), vec!["shell/1".to_string(), "tui/abc".to_string()]);
        d.take_off("tui/abc").unwrap();
        assert_eq!(d.entry("tui/abc"), None);
        assert_eq!(d.items(), 1);
    }

    /// Imported on one machine, on the other's list after a merge — and
    /// taken off the same way.
    #[test]
    fn the_list_is_one_list_on_every_machine() {
        let a = RosterDoc::new(1).unwrap();
        let b = RosterDoc::new(2).unwrap();
        a.put("tui/x", Entry::Import).unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(b.entry("tui/x"), Some(Entry::Import));
        b.take_off("tui/x").unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        assert_eq!(a.entry("tui/x"), None);
    }

    #[test]
    fn the_same_write_twice_writes_once() {
        let d = RosterDoc::new(1).unwrap();
        d.put("tui/z", Entry::New).unwrap();
        let before = d.version();
        d.put("tui/z", Entry::New).unwrap();
        assert_eq!(d.version(), before);
        let fresh = RosterDoc::new(3).unwrap();
        let empty = fresh.version();
        fresh.take_off("tui/never").unwrap();
        assert_eq!(fresh.version(), empty);
    }
}
