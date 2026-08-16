//! The device table: who is in the network (docs/NET.md).
//! One CRDT map, replicated everywhere — joining any one device reveals
//! the whole network.
//!
//! Shape: `devices : Map<id_hex, Map{name, addrs, style}>`. Per-key
//! LWW, so a rename on one device and an address update on another both
//! survive. `addrs` is one JSON-array field (whole-list LWW): addresses
//! are dialing hints, freshest writer wins is right for them.
//!
//! `style` is the device's **self-reported** avatar style, one JSON
//! object (whole-value LWW). It rides the table rather than a wire frame
//! because of what it has to achieve: **a machine looks the same to
//! everyone, painted in its own palette** — whoever is looking derives
//! its face from what it reported about itself, not from local
//! preference. Whole-value and not per-slot: half a palette from one
//! writer and a variant from another is a face nobody chose.
//!
//! `pinned` is "this machine matters", and it rides the table for the
//! reason `khor_sync::pins` states for sessions: a pin belongs to the
//! thing pinned, so it has to be one fact the network agrees on rather
//! than one answer per screen. **It is the opposite of `style` in who
//! may write it** — anyone may pin anyone, while only a machine may
//! describe its own face — and it lands in the same place for the
//! opposite reason: the device table is already the one row per machine
//! everybody replicates, and a second table keyed by device id would be
//! the same key twice.
//!
//! What the two share is that [`DeviceDoc::upsert`] must not touch
//! either. Pairing has every peer writing names and addresses for
//! machines they just learned about; folding a pin into that write would
//! have a routine update silently clear a pin set somewhere else.

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
    /// The avatar style this device reported about itself, as JSON.
    /// `None` means it never reported one (an older version, or a
    /// device seen only through someone else's table) — whoever paints
    /// it falls back to the factory default.
    ///
    /// Kept as an opaque string here: this crate replicates documents
    /// and does not know what an avatar is. Parsing happens at the one
    /// gate that knows, `khor_core::avatar::AvatarStyle::from_json`.
    pub style: Option<String>,
    /// Whether someone pinned this machine to the top of the list.
    /// Network-wide, not per viewer (see the module head).
    pub pinned: bool,
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

    /// The entry for `id`, created empty if absent.
    fn entry(&self, id: &str) -> Result<LoroMap, String> {
        let map = self.map();
        match map.get(id).and_then(as_map) {
            Some(m) => Ok(m),
            // insert_container returns the attached handle; writing to
            // the pre-attach one goes nowhere, silently.
            None => map
                .insert_container(id, LoroMap::new())
                .map_err(|e| e.to_string()),
        }
    }

    /// Inserts or updates a device's name and dialing hints. Writes only
    /// fields that actually changed — a no-change upsert must not grow
    /// the version vector on every startup.
    ///
    /// **Deliberately does not touch `style`.** Everyone writes this for
    /// their peers (pairing writes the far side's name and addresses),
    /// while `style` is a claim only the device itself may make; folding
    /// it in here would have every peer overwrite what a machine said
    /// about its own face with nothing.
    pub fn upsert(&self, id: &str, name: &str, addrs: &[String]) -> Result<(), String> {
        let addrs_json = serde_json::to_string(addrs).map_err(|e| e.to_string())?;
        let entry = self.entry(id)?;
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

    /// Records the avatar style a device reports about itself. Callers
    /// pass their **own** id; a claim about someone else's face is not
    /// theirs to make.
    ///
    /// Unchanged means unwritten, same as [`DeviceDoc::upsert`]: this
    /// runs on every open, and a rewrite each time inflates the version
    /// vector for nothing.
    pub fn set_style(&self, id: &str, style_json: &str) -> Result<(), String> {
        let entry = self.entry(id)?;
        if str_of(&entry, "style").as_deref() == Some(style_json) {
            return Ok(());
        }
        entry.insert("style", style_json).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Pins or unpins a machine. Anyone may pin anyone — unlike
    /// [`DeviceDoc::set_style`], this is not a claim only the subject can
    /// make. Unchanged means unwritten, same reason as the others.
    pub fn set_pinned(&self, id: &str, on: bool) -> Result<(), String> {
        let entry = self.entry(id)?;
        if bool_of(&entry, "pinned") == on {
            return Ok(());
        }
        entry.insert("pinned", on).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every device, pinned ones first, each group by name.
    ///
    /// **Sorted here, once.** Every face reads the table through this,
    /// so a pin puts a machine at the top of the CLI and of the app
    /// without either of them owning a comparison (docs/UX.md: the list
    /// never re-derives a judgment the library already made).
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
                style: str_of(&entry, "style"),
                pinned: bool_of(&entry, "pinned"),
            });
        }
        out.sort_by(|a, b| b.pinned.cmp(&a.pinned).then_with(|| a.name.cmp(&b.name)));
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

/// A flag that was never written reads false — "nobody pinned it" and
/// "somebody unpinned it" are the same state to every reader.
fn bool_of(m: &LoroMap, k: &str) -> bool {
    matches!(
        m.get(k).and_then(|v| v.into_value().ok()),
        Some(LoroValue::Bool(true))
    )
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
        d.set_style("aa", "{\"variant\":\"marble\"}").unwrap();
        let before = d.version();
        d.upsert("aa", "mac", &["1.1.1.1:1".into()]).unwrap();
        d.set_style("aa", "{\"variant\":\"marble\"}").unwrap();
        assert_eq!(d.version(), before, "no change must mean no new ops");
    }

    /// **A peer's upsert must not erase what a machine said about its
    /// own face.** Everyone writes names and addresses for their peers
    /// (pairing does), so if `upsert` touched `style` at all, every
    /// peer's routine write would blank the one field only the machine
    /// itself may set — and the symptom is silent: it would simply be
    /// painted in the default palette, which looks like "that's how it
    /// looks", not like data loss.
    #[test]
    fn a_peers_upsert_leaves_a_self_reported_style_alone() {
        let mine = DeviceDoc::new(1).unwrap();
        mine.upsert("aa", "mac", &[]).unwrap();
        mine.set_style("aa", "{\"shape\":\"square\"}").unwrap();

        // Someone else learns about "aa" and writes what they know
        let theirs = DeviceDoc::new(2).unwrap();
        theirs.merge(&mine.snapshot().unwrap()).unwrap();
        theirs.upsert("aa", "mac", &["9.9.9.9:9".into()]).unwrap();
        assert_eq!(
            theirs.get("aa").unwrap().style.as_deref(),
            Some("{\"shape\":\"square\"}"),
            "a peer's write must not drop the reported style"
        );

        // …and it survives the round trip home
        mine.merge(&theirs.changes_since(&mine.version()).unwrap()).unwrap();
        let back = mine.get("aa").unwrap();
        assert_eq!(back.addrs, vec!["9.9.9.9:9".to_string()]);
        assert_eq!(back.style.as_deref(), Some("{\"shape\":\"square\"}"));
    }

    /// A pin travels, and it puts the machine at the head of the list on
    /// the far side too — the ordering is the table's, so nobody has to
    /// re-derive it.
    #[test]
    fn a_pinned_machine_travels_and_leads_the_list() {
        let a = DeviceDoc::new(1).unwrap();
        a.upsert("aa", "alpha", &[]).unwrap();
        a.upsert("bb", "beta", &[]).unwrap();
        a.upsert("cc", "gamma", &[]).unwrap();
        assert_eq!(
            a.all().into_iter().map(|d| d.name).collect::<Vec<_>>(),
            vec!["alpha", "beta", "gamma"],
            "control: by name while nothing is pinned"
        );

        a.set_pinned("cc", true).unwrap();
        assert_eq!(
            a.all().into_iter().map(|d| d.name).collect::<Vec<_>>(),
            vec!["gamma", "alpha", "beta"],
            "the pinned machine leads; the rest keep their order"
        );

        let b = DeviceDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();
        assert!(b.get("cc").unwrap().pinned, "the pin must reach the other device");
        assert_eq!(b.all()[0].name, "gamma", "and lead there too");

        // Taking it back travels as well, and the order returns.
        a.set_pinned("cc", false).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();
        assert!(!b.get("cc").unwrap().pinned);
        assert_eq!(
            b.all().into_iter().map(|d| d.name).collect::<Vec<_>>(),
            vec!["alpha", "beta", "gamma"]
        );
    }

    /// **A peer's routine upsert must not clear a pin.** Pairing has
    /// every device writing names and addresses for machines it just
    /// learned about; if `upsert` wrote `pinned` at all, one of those
    /// writes would silently drop a pin someone set on another device —
    /// and the symptom is a row quietly leaving the top of the list,
    /// which reads as "I must have unpinned it".
    #[test]
    fn a_peers_upsert_leaves_a_pin_alone() {
        let mine = DeviceDoc::new(1).unwrap();
        mine.upsert("aa", "mac", &[]).unwrap();
        mine.set_pinned("aa", true).unwrap();

        let theirs = DeviceDoc::new(2).unwrap();
        theirs.merge(&mine.snapshot().unwrap()).unwrap();
        theirs.upsert("aa", "mac", &["9.9.9.9:9".into()]).unwrap();
        assert!(
            theirs.get("aa").unwrap().pinned,
            "a peer's write must not drop the pin"
        );

        mine.merge(&theirs.changes_since(&mine.version()).unwrap()).unwrap();
        assert!(mine.get("aa").unwrap().pinned, "…nor on the round trip home");
    }

    /// A device that never reported a style reads as `None`, not as an
    /// empty string: "hasn't said" and "said nothing" have to stay
    /// distinguishable, because only the first may fall back silently.
    #[test]
    fn a_device_that_never_reported_a_style_has_none() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &[]).unwrap();
        assert_eq!(d.get("aa").unwrap().style, None);
    }

    /// **Two replicas creating the same device's entry independently is
    /// lossy, and this pins that it is** — the per-key LWW promise at
    /// the top of this file starts one step later than one would think.
    ///
    /// Once an entry exists, concurrent writes to different fields both
    /// survive (`a_concurrent_rename_and_addr_update_both_survive`).
    /// Creating it is different: the entry is a nested container, two
    /// replicas creating one for the same id produce two containers,
    /// and the merge keeps one **whole** and discards the other's
    /// contents. That is exactly what pairing does — a device registers
    /// itself at open, and the inviter creates its row while answering
    /// the pairing request.
    ///
    /// It hid for as long as every field had more than one writer
    /// agreeing: both sides write the same `name`, and both wrote
    /// `addrs` as `[]` at that moment, so the losing container was
    /// indistinguishable. `style` has exactly one legitimate writer, so
    /// it is where the loss finally became visible.
    ///
    /// The recovery is `khor_node::Node::reassert_self` — restate your
    /// own row after any merge — and the second half of this test is
    /// what that relies on. **This asserts today's behavior, not the
    /// behavior we want**: whoever makes the entry a flat set of
    /// registers (`<id>/<field>`) instead of a container should expect
    /// this test to change, and should read it first.
    #[test]
    fn two_replicas_creating_one_entry_keep_only_one_of_them() {
        let beta = DeviceDoc::new(1).unwrap();
        beta.upsert("bb", "beta", &[]).unwrap();
        beta.set_style("bb", "{\"mine\":true}").unwrap();

        // The inviter, independently, creates a row for the newcomer
        let alpha = DeviceDoc::new(2).unwrap();
        alpha.upsert("aa", "alpha", &[]).unwrap();
        alpha.upsert("bb", "beta", &[]).unwrap();

        beta.merge(&alpha.snapshot().unwrap()).unwrap();
        assert_eq!(
            beta.get("bb").unwrap().style,
            None,
            "if this survives, container creation stopped being lossy — read the \
             comment above before deleting the re-assertion that works around it"
        );
        // The fields both sides wrote look untouched, which is why this
        // went unnoticed for as long as every field had two writers
        assert_eq!(beta.get("bb").unwrap().name, "beta");

        // Restating the claim recovers it, and it is what the node does
        // after every devices merge
        beta.set_style("bb", "{\"mine\":true}").unwrap();
        assert_eq!(beta.get("bb").unwrap().style.as_deref(), Some("{\"mine\":true}"));
        // …and it reaches the other side from there
        alpha.merge(&beta.changes_since(&alpha.version()).unwrap()).unwrap();
        assert_eq!(alpha.get("bb").unwrap().style.as_deref(), Some("{\"mine\":true}"));
    }
}
