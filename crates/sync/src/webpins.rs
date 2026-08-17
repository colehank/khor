//! Pinned web pages: one flag per (exit machine, URL), replicated
//! everywhere — the browser landing's shortcuts (docs/NET.md 借网).
//!
//! # Why this key carries a machine, like [`crate::dirpins`] and unlike
//! [`crate::pins`]
//!
//! A pinned page is not a page — it is "this page, opened through this
//! machine's network". A walled site reached through a laptop in another
//! country is a different thing from the same URL opened directly, and the
//! exit is which of those it is. So the key is `<device_hex>:<url>`, the
//! id and not the name (a rename must not orphan a pin), and the device
//! hex being fixed-width lowercase hex, the first `:` splits it off no
//! matter how many colons the URL carries (`https://…`).
//!
//! Everything else is [`crate::dirpins`]'s story on its own table: plain
//! LWW, because pinning and unpinning are things the user does on purpose.

use loro::{LoroDoc, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

pub const REL_DIR: &str = ".khor/webpins";

pub fn webpins_dir(home: &std::path::Path) -> std::path::PathBuf {
    home.join(REL_DIR)
}

const PINS: &str = "webpins";

/// The table's one key spelling. The split is [`split`]; nothing else may
/// take the key apart, or two readers disagree about what a row is.
pub fn key(device_hex: &str, url: &str) -> String {
    format!("{device_hex}:{url}")
}

/// `(device_hex, url)` back out of a key. `None` for a key some other
/// version wrote in a spelling this one does not know — skipped, never
/// guessed at. Splits on the first `:` only: the device hex has none, the
/// URL has several.
pub fn split(key: &str) -> Option<(&str, &str)> {
    key.split_once(':')
}

pub struct WebPinDoc {
    inner: LoroDoc,
}

impl WebPinDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    pub fn pinned(&self, key: &str) -> bool {
        matches!(
            self.inner.get_map(PINS).get(key).and_then(|v| v.into_value().ok()),
            Some(LoroValue::Bool(true))
        )
    }

    /// Unchanged means unwritten — rewriting the same value inflates the
    /// version vector for nothing (`DeviceDoc::upsert`'s rule).
    pub fn set(&self, key: &str, on: bool) -> Result<(), String> {
        if self.pinned(key) == on {
            return Ok(());
        }
        self.inner.get_map(PINS).insert(key, on).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every pinned key, sorted — which groups by machine for free, the
    /// device hex being the prefix.
    pub fn all(&self) -> Vec<String> {
        let map = self.inner.get_map(PINS);
        let mut out: Vec<String> =
            map.keys().filter(|k| self.pinned(k)).map(|k| k.to_string()).collect();
        out.sort();
        out
    }
}

impl Doc for WebPinDoc {
    fn open(peer: u64) -> Result<Self, String> {
        WebPinDoc::new(peer)
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

    /// A URL is full of colons; the split must land on the machine
    /// boundary, which is the whole reason that half is fixed-form hex.
    #[test]
    fn the_key_splits_on_the_machine_boundary_not_inside_the_url() {
        let k = key("abc123", "https://example.com:8443/a?b=c:d");
        assert_eq!(split(&k), Some(("abc123", "https://example.com:8443/a?b=c:d")));
    }

    /// Pin on one device, pinned on the other — the reason this is a
    /// document and not a preference file.
    #[test]
    fn a_web_pin_travels_and_unpinning_travels_back() {
        let phone = WebPinDoc::new(1).unwrap();
        let desk = WebPinDoc::new(2).unwrap();
        let k = key("d1e6", "https://news.example");
        phone.set(&k, true).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap()).unwrap();
        assert!(desk.pinned(&k), "a pin made on the phone must reach the desk");
        desk.set(&k, false).unwrap();
        phone.merge(&desk.changes_since(&Default::default()).unwrap()).unwrap();
        assert!(!phone.pinned(&k), "an unpin must travel back");
        assert_eq!(desk.items(), 0, "an unpinned key is not an item");
    }
}
