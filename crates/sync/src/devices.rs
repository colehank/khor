//! The device table: who is in the network (docs/NET.md).
//! One CRDT map, replicated everywhere — joining any one device reveals
//! the whole network.
//!
//! Shape: `devices : Map<id_hex, Map{name, addrs}>`. Per-key LWW, so a
//! rename on one device and an address update on another both survive.
//! `addrs` is one JSON-array field (whole-list LWW): addresses are
//! dialing hints, freshest writer wins is right for them.

use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroMap, LoroValue, VersionVector};

use crate::glue;
use crate::store::Doc;

/// Location relative to the home directory (same reasoning as
/// `chat::REL_DIR`).
pub const REL_DIR: &str = ".khor/devices";

pub fn devices_dir(home: &Path) -> PathBuf {
    home.join(REL_DIR)
}

const DEVICES: &str = "devices";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// The public key, hex — the machine id (docs/NET.md).
    pub id: String,
    /// Self-reported channel name.
    pub name: String,
    /// Last known dialing hints (`ip:port` strings). Best effort;
    /// discovery is the real path finder.
    pub addrs: Vec<String>,
}

pub struct DeviceDoc {
    inner: LoroDoc,
}

impl DeviceDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    fn map(&self) -> LoroMap {
        self.inner.get_map(DEVICES)
    }

    /// Inserts or updates a device. Writes only fields that actually
    /// changed — a no-change upsert must not grow the version vector on
    /// every startup.
    pub fn upsert(&self, id: &str, name: &str, addrs: &[String]) -> Result<(), String> {
        let addrs_json = serde_json::to_string(addrs).map_err(|e| e.to_string())?;
        let map = self.map();
        let entry = match map.get(id).and_then(as_map) {
            Some(m) => m,
            // insert_container returns the attached handle; writing to
            // the pre-attach one goes nowhere, silently.
            None => map
                .insert_container(id, LoroMap::new())
                .map_err(|e| e.to_string())?,
        };
        let mut dirty = false;
        if str_of(&entry, "name").as_deref() != Some(name) {
            entry.insert("name", name).map_err(|e| e.to_string())?;
            dirty = true;
        }
        if str_of(&entry, "addrs").as_deref() != Some(addrs_json.as_str()) {
            entry
                .insert("addrs", addrs_json.as_str())
                .map_err(|e| e.to_string())?;
            dirty = true;
        }
        if dirty {
            self.inner.commit();
        }
        Ok(())
    }

    /// Every device, sorted by name.
    pub fn all(&self) -> Vec<DeviceInfo> {
        let map = self.map();
        let mut out = Vec::new();
        for id in map.keys() {
            let Some(entry) = map.get(&id).and_then(as_map) else {
                continue;
            };
            out.push(DeviceInfo {
                id: id.to_string(),
                name: str_of(&entry, "name").unwrap_or_default(),
                addrs: str_of(&entry, "addrs")
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub fn get(&self, id: &str) -> Option<DeviceInfo> {
        self.all().into_iter().find(|d| d.id == id)
    }

    pub fn by_name(&self, name: &str) -> Option<DeviceInfo> {
        self.all().into_iter().find(|d| d.name == name)
    }
}

impl Doc for DeviceDoc {
    fn open(peer: u64) -> Result<Self, String> {
        DeviceDoc::new(peer)
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
        self.map().keys().count()
    }
}

fn as_map(v: loro::ValueOrContainer) -> Option<LoroMap> {
    v.into_container().ok()?.into_map().ok()
}

fn str_of(m: &LoroMap, k: &str) -> Option<String> {
    match m.get(k)?.into_value().ok()? {
        LoroValue::String(s) => Some(s.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_round_trips_with_name_and_addrs() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa11", "turing", &["10.0.0.2:11204".into()]).unwrap();
        let got = d.get("aa11").expect("should be there");
        assert_eq!(got.name, "turing");
        assert_eq!(got.addrs, vec!["10.0.0.2:11204".to_string()]);
        assert_eq!(d.by_name("turing").unwrap().id, "aa11");
        assert_eq!(d.items(), 1);
    }

    /// Joining one device reveals the whole network: merge B's table into
    /// A's and both see everyone.
    #[test]
    fn merging_two_tables_shows_everyone_on_both() {
        let a = DeviceDoc::new(1).unwrap();
        a.upsert("aa", "mac", &[]).unwrap();
        let b = DeviceDoc::new(2).unwrap();
        b.upsert("bb", "phone", &[]).unwrap();

        b.merge(&a.changes_since(&Default::default()).unwrap()).unwrap();
        a.merge(&b.changes_since(&Default::default()).unwrap()).unwrap();

        for (who, d) in [("a", &a), ("b", &b)] {
            let names: Vec<String> = d.all().into_iter().map(|x| x.name).collect();
            assert_eq!(names, vec!["mac", "phone"], "{who} should see the whole network");
        }
    }

    /// Per-key LWW: a rename on one device and an address update on
    /// another both survive the merge — a whole-entry rewrite would drop
    /// one silently.
    #[test]
    fn a_concurrent_rename_and_addr_update_both_survive() {
        let a = DeviceDoc::new(1).unwrap();
        a.upsert("aa", "old-name", &[]).unwrap();
        let b = DeviceDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();

        a.upsert("aa", "new-name", &[]).unwrap();
        b.upsert("aa", "old-name", &["1.2.3.4:5".into()]).unwrap();

        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();

        for (who, d) in [("a", &a), ("b", &b)] {
            let got = d.get("aa").unwrap();
            assert_eq!(got.name, "new-name", "{who}: the rename must survive");
            assert_eq!(got.addrs, vec!["1.2.3.4:5".to_string()], "{who}: the address must survive");
        }
    }

    /// A no-change upsert grows nothing: otherwise every startup writes a
    /// block and the version vector inflates for free.
    #[test]
    fn an_identical_upsert_writes_nothing() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &["1.1.1.1:1".into()]).unwrap();
        let before = d.version();
        d.upsert("aa", "mac", &["1.1.1.1:1".into()]).unwrap();
        assert_eq!(d.version(), before, "no change must mean no new ops");
    }
}
