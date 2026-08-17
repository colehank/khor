//! Pinned directories: one flag per (machine, absolute path),
//! replicated everywhere — the files landing's shortcuts.
//!
//! # Why this key carries a machine, when session pins' key must not
//!
//! [`crate::pins`] keys by bare session id because a session is one
//! thing network-wide, and the whole point is pinning a row whose home
//! is somebody else. A path is the opposite: `/Users/x/Desktop` exists
//! on every Mac in the mesh, and a machineless pin would light a
//! directory on disks nobody chose. So the key is
//! `<device_hex>:<path>` — the id, not the name, because names are
//! user-editable and a rename must not orphan every pin. A device hex
//! is fixed-width lowercase hex, so the first `:` splits without
//! ambiguity no matter what the path contains.
//!
//! Everything else is [`crate::pins`]'s story, deliberately retold on
//! its own table (one kind of landing, one table, one key): plain LWW
//! because unpinning is a thing the user does on purpose, and a pin
//! outliving its directory paints nothing and raises nothing — the
//! reader looks flags up for machines it is already showing.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

pub const REL_DIR: &str = ".khor/dirpins";

pub fn dirpins_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const PINS: &str = "dirpins";

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

pub struct DirPinDoc {
    inner: LoroDoc,
}

impl DirPinDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    pub fn pinned(&self, key: &str) -> bool {
        matches!(
            self.inner.get_map(PINS).get(key).and_then(|v| v.into_value().ok()),
            Some(LoroValue::Bool(true))
        )
    }

    /// Unchanged means unwritten — reachable from a click, and
    /// rewriting the same value inflates the version vector for
    /// nothing (`DeviceDoc::upsert`'s rule).
    pub fn set(&self, key: &str, on: bool) -> Result<(), String> {
        if self.pinned(key) == on {
            return Ok(());
        }
        self.inner.get_map(PINS).insert(key, on).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every pinned key, sorted — which groups by machine for free,
    /// the device hex being the prefix.
    pub fn all(&self) -> Vec<String> {
        let map = self.inner.get_map(PINS);
        let mut out: Vec<String> =
            map.keys().filter(|k| self.pinned(k)).map(|k| k.to_string()).collect();
        out.sort();
        out
    }
}

impl Doc for DirPinDoc {
    fn open(peer: u64) -> Result<Self, String> {
        DirPinDoc::new(peer)
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

    /// The split must survive a path with a colon in it — the whole
    /// reason the machine half is fixed-form hex.
    #[test]
    fn the_key_splits_on_the_machine_boundary_not_inside_the_path() {
        let k = key("abc123", "/tmp/odd:name/dir");
        assert_eq!(split(&k), Some(("abc123", "/tmp/odd:name/dir")));
    }

    /// Pin on one device, pinned on the other — the reason this is a
    /// document and not a preference file.
    #[test]
    fn a_directory_pin_travels_and_unpinning_travels_back() {
        let phone = DirPinDoc::new(1).unwrap();
        let desk = DirPinDoc::new(2).unwrap();
        let k = key("d1e6", "/Users/x/proj");
        phone.set(&k, true).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap()).unwrap();
        assert!(desk.pinned(&k), "a pin made on the phone must reach the desk");
        desk.set(&k, false).unwrap();
        phone.merge(&desk.changes_since(&Default::default()).unwrap()).unwrap();
        assert!(!phone.pinned(&k), "an unpin must travel back");
        assert_eq!(desk.items(), 0, "an unpinned key is not an item");
    }
}
