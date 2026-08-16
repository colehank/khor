//! The device table: who is in the network (docs/NET.md).
//! One CRDT map, replicated everywhere — joining any one device reveals
//! the whole network.
//!
//! Shape: `devices : Map<"<id>/<field>", value>` — **one flat map, one
//! register per field.** Per-key LWW, so a rename on one device and an
//! address update on another both survive. `addrs` is one JSON-array
//! field (whole-list LWW): addresses are dialing hints, freshest writer
//! wins is right for them.
//!
//! # Why flat, and not a map per device
//!
//! It used to be `Map<id, Map{…}>`, and the nesting was the bug. A
//! device's row was a container, two replicas could create the container
//! for the *same* id independently (a machine registers itself at open
//! while the inviter creates its row answering a pairing request), and
//! loro settles a container conflict by keeping one container **whole**
//! and discarding the other's contents — not by merging them field by
//! field the way it does once a single container exists. Every field
//! with exactly one legitimate writer was therefore one pairing away
//! from being silently blank; `style` is where it finally showed.
//!
//! Flat keys have no container to lose. The root map is addressed by
//! name rather than created by an op, so two replicas writing
//! `aa/style` and `aa/name` are two writes to one map, and per-key LWW
//! covers them from the first write rather than from the second. The
//! per-key promise at the top of this file now starts where one would
//! think it does.
//!
//! Ids are public keys in hex, so `/` can only ever be the separator
//! this module put there.
//!
//! # The old shape is still read, and still written
//!
//! A network can have an older khor in it, and it only knows the nested
//! shape. So this module **reads flat first and falls back to the
//! nested entry**, and every write also updates the nested entry as a
//! mirror. An old peer therefore keeps seeing names, addresses, styles
//! and pins that this version writes, and rows an old peer wrote are
//! visible here.
//!
//! Migration is what falls out of that, not a step anyone runs: a row
//! that exists only in the old shape is lifted onto the flat table by
//! the first write that touches it (this machine's own row on the next
//! `register_self`, a peer's row when it is next upserted or pinned).
//! There is no flag day and nothing to run twice — [`DeviceDoc::upsert`]
//! and friends decide "unchanged" by looking at the **flat** register,
//! so a value that never changed still gets written once, and only once.
//!
//! **What the mirror cannot cover**, stated because it is the one hole:
//! once a flat register exists for a field, readers here prefer it, so
//! an *old* machine that renames itself after a new machine has written
//! its row shows its old name to new machines until it upgrades. It is
//! narrow (it needs a rename inside the mixed-version window) and it
//! heals itself the moment that machine runs a khor that writes flat
//! keys. The alternative — no mirror — is worse and not narrow: a
//! machine that joined through a new khor would be **absent** from an
//! old one's list with nothing anywhere saying so.
//!
//! The mirror is a shim with an end date: when no khor that predates
//! the flat table is left in any network, the mirror, the fallback and
//! `Node::reassert_self` come out together. It is on the ledger, and
//! `an_old_khor_still_reads_what_this_version_writes` is what turns red
//! if someone takes half of it out.
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

/// The separator in a flat key. Device ids are hex, so a key that holds
/// one was written by this module and a key that does not is an old
/// nested entry.
const SEP: char = '/';

const F_NAME: &str = "name";
const F_ADDRS: &str = "addrs";
const F_STYLE: &str = "style";
const F_PINNED: &str = "pinned";

fn fkey(id: &str, field: &str) -> String {
    let mut k = String::with_capacity(id.len() + 1 + field.len());
    k.push_str(id);
    k.push(SEP);
    k.push_str(field);
    k
}

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

    /// The old nested entry for `id`, created empty if absent. Only the
    /// compatibility mirror writes through this; readers reach it via
    /// [`DeviceDoc::field`]'s fallback.
    fn legacy_entry(&self, id: &str) -> Result<LoroMap, String> {
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

    /// One field's flat register, ignoring the old shape entirely.
    ///
    /// **This is what the setters compare against, not [`Self::field`].**
    /// Comparing the effective value would leave a row that an older
    /// khor wrote sitting in the nested shape forever, because its
    /// values already match and every write would be skipped as a
    /// no-op — an upgrade would migrate nothing. Comparing the register
    /// makes the first write after an upgrade the migration, and the
    /// second one a no-op again.
    fn flat(&self, id: &str, field: &str) -> Option<LoroValue> {
        self.map()
            .get(&fkey(id, field))
            .and_then(|v| v.into_value().ok())
    }

    fn flat_str(&self, id: &str, field: &str) -> Option<String> {
        match self.flat(id, field)? {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        }
    }

    /// One field as a reader sees it: the flat register if there is one,
    /// otherwise whatever the old nested entry holds.
    fn field(&self, id: &str, field: &str) -> Option<LoroValue> {
        if let Some(v) = self.flat(id, field) {
            return Some(v);
        }
        self.map()
            .get(id)
            .and_then(as_map)?
            .get(field)
            .and_then(|v| v.into_value().ok())
    }

    fn str_field(&self, id: &str, field: &str) -> Option<String> {
        match self.field(id, field)? {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        }
    }

    /// A flag that was never written reads false — "nobody pinned it"
    /// and "somebody unpinned it" are the same state to every reader.
    fn bool_field(&self, id: &str, field: &str) -> bool {
        matches!(self.field(id, field), Some(LoroValue::Bool(true)))
    }

    /// Writes one field to the flat register **and** to the old nested
    /// entry. The second write is the compatibility mirror (module
    /// head): it is what an older khor in the same network reads, and
    /// it is the half that comes out when the last of them is gone.
    fn write_field(&self, id: &str, field: &str, v: impl Into<LoroValue> + Clone) -> Result<(), String> {
        self.map()
            .insert(&fkey(id, field), v.clone())
            .map_err(|e| e.to_string())?;
        self.legacy_entry(id)?
            .insert(field, v)
            .map_err(|e| e.to_string())
    }

    /// Every device the table knows, from either shape. A key holding
    /// the separator is `<id>/<field>`; one without it is an old nested
    /// entry, whose key is the id itself.
    fn ids(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for k in self.map().keys() {
            let k = k.to_string();
            let id = match k.split_once(SEP) {
                Some((id, _)) => id.to_string(),
                None => k,
            };
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
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
        let mut dirty = false;
        if self.flat_str(id, F_NAME).as_deref() != Some(name) {
            self.write_field(id, F_NAME, name)?;
            dirty = true;
        }
        if self.flat_str(id, F_ADDRS).as_deref() != Some(addrs_json.as_str()) {
            self.write_field(id, F_ADDRS, addrs_json.as_str())?;
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
        if self.flat_str(id, F_STYLE).as_deref() == Some(style_json) {
            return Ok(());
        }
        self.write_field(id, F_STYLE, style_json)?;
        self.inner.commit();
        Ok(())
    }

    /// Pins or unpins a machine. Anyone may pin anyone — unlike
    /// [`DeviceDoc::set_style`], this is not a claim only the subject can
    /// make. Unchanged means unwritten, same reason as the others.
    pub fn set_pinned(&self, id: &str, on: bool) -> Result<(), String> {
        // Nothing written in either shape and nothing to write:
        // unpinning a machine nobody pinned must not cost an op, and
        // `khor unpin -m` on a stranger is a call people make.
        if !on && self.field(id, F_PINNED).is_none() {
            return Ok(());
        }
        if self.flat(id, F_PINNED) == Some(LoroValue::Bool(on)) {
            return Ok(());
        }
        self.write_field(id, F_PINNED, on)?;
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
        let mut out: Vec<DeviceInfo> = self
            .ids()
            .into_iter()
            .map(|id| DeviceInfo {
                name: self.str_field(&id, F_NAME).unwrap_or_default(),
                addrs: self
                    .str_field(&id, F_ADDRS)
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                style: self.str_field(&id, F_STYLE),
                pinned: self.bool_field(&id, F_PINNED),
                id,
            })
            .collect();
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

    /// Devices, not keys. A flat table has several keys per device, so
    /// counting keys would report the table as four times its size in
    /// the sync line humans read.
    fn items(&self) -> usize {
        self.ids().len()
    }
}

fn as_map(v: loro::ValueOrContainer) -> Option<LoroMap> {
    v.into_container().ok()?.into_map().ok()
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

    /// **Two replicas creating the same device's row independently keep
    /// both sides' fields.** This is the whole reason the table is flat,
    /// and it is exactly what pairing does: a device registers itself at
    /// open, and the inviter creates its row while answering the pairing
    /// request — concurrently, with no chance to agree first.
    ///
    /// Under the old nested shape this test was the opposite one, and it
    /// asserted the loss: the row was a container, two replicas creating
    /// one for the same id produced two containers, and the merge kept
    /// one **whole** while discarding the other's contents. It hid for
    /// as long as every field had more than one writer agreeing — both
    /// sides write the same `name`, both wrote `addrs` as `[]` at that
    /// moment — so `style`, with exactly one legitimate writer, is where
    /// the loss finally showed.
    ///
    /// The recovery used to be `khor_node::Node::reassert_self`. With
    /// registers there is nothing to recover: no container is created,
    /// so none can lose.
    #[test]
    fn two_replicas_creating_one_row_keep_both_of_their_fields() {
        let beta = DeviceDoc::new(1).unwrap();
        beta.upsert("bb", "beta", &[]).unwrap();
        beta.set_style("bb", "{\"mine\":true}").unwrap();

        // The inviter, independently, creates a row for the newcomer
        let alpha = DeviceDoc::new(2).unwrap();
        alpha.upsert("aa", "alpha", &[]).unwrap();
        alpha.upsert("bb", "beta", &[]).unwrap();

        beta.merge(&alpha.snapshot().unwrap()).unwrap();
        assert_eq!(
            beta.get("bb").unwrap().style.as_deref(),
            Some("{\"mine\":true}"),
            "the one writer of `style` must not lose it to a concurrent row creation"
        );
        assert_eq!(beta.get("bb").unwrap().name, "beta");

        // …and it holds from the other seat too, without anyone
        // restating anything
        alpha.merge(&beta.changes_since(&alpha.version()).unwrap()).unwrap();
        assert_eq!(alpha.get("bb").unwrap().style.as_deref(), Some("{\"mine\":true}"));
        assert_eq!(
            alpha.all().into_iter().map(|d| d.name).collect::<Vec<_>>(),
            vec!["alpha", "beta"],
            "and both machines are still in the table"
        );
    }

    /// Counting keys instead of devices would report a flat table at
    /// several times its size, in the sync line a human reads.
    #[test]
    fn items_counts_devices_not_fields() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &["1.1.1.1:1".into()]).unwrap();
        d.set_style("aa", "{}").unwrap();
        d.set_pinned("aa", true).unwrap();
        assert_eq!(d.items(), 1, "one device, whatever it has written about it");
        d.upsert("bb", "phone", &[]).unwrap();
        assert_eq!(d.items(), 2);
    }

    // ── the mixed network: an older khor knows only the nested shape ──

    /// Writes a row the way khor wrote it before the flat table: one
    /// nested container per device. Hand-built on purpose — generating
    /// the fixture with today's code would make it a description of
    /// today's code rather than of what is on an old disk.
    fn write_old_shape(
        d: &DeviceDoc,
        id: &str,
        name: &str,
        addrs: &[&str],
        style: Option<&str>,
        pinned: bool,
    ) {
        let map = d.inner.get_map(DEVICES);
        let entry = match map.get(id).and_then(as_map) {
            Some(m) => m,
            None => map.insert_container(id, LoroMap::new()).unwrap(),
        };
        entry.insert("name", name).unwrap();
        entry
            .insert("addrs", serde_json::to_string(addrs).unwrap().as_str())
            .unwrap();
        if let Some(s) = style {
            entry.insert("style", s).unwrap();
        }
        if pinned {
            entry.insert("pinned", true).unwrap();
        }
        d.inner.commit();
    }

    /// Reads the way an older khor reads: the nested container and
    /// nothing else. This is the far side of a mixed network, and what
    /// the compatibility mirror has to keep true.
    fn read_old_shape(d: &DeviceDoc, id: &str) -> Option<(String, Vec<String>, Option<String>, bool)> {
        let entry = d.inner.get_map(DEVICES).get(id).and_then(as_map)?;
        let s = |k: &str| match entry.get(k)?.into_value().ok()? {
            LoroValue::String(v) => Some(v.to_string()),
            _ => None,
        };
        Some((
            s("name")?,
            s("addrs")
                .and_then(|a| serde_json::from_str(&a).ok())
                .unwrap_or_default(),
            s("style"),
            matches!(
                entry.get("pinned").and_then(|v| v.into_value().ok()),
                Some(LoroValue::Bool(true))
            ),
        ))
    }

    /// **An old disk reads back whole.** Every field an older khor could
    /// have written is still there through the fallback, with nothing
    /// run first — there is no migration step and so no flag day.
    #[test]
    fn a_row_an_older_khor_wrote_reads_back_with_every_field() {
        let d = DeviceDoc::new(1).unwrap();
        write_old_shape(&d, "aa", "turing", &["10.0.0.2:11204"], Some("{\"v\":1}"), true);

        let got = d.get("aa").expect("an old row must still be a row");
        assert_eq!(got.name, "turing");
        assert_eq!(got.addrs, vec!["10.0.0.2:11204".to_string()]);
        assert_eq!(got.style.as_deref(), Some("{\"v\":1}"));
        assert!(got.pinned);
        assert_eq!(d.items(), 1, "and it counts once, not once per field");
        assert_eq!(d.by_name("turing").unwrap().id, "aa");
    }

    /// **The first write lifts an old row onto the flat table, and only
    /// the first.** The value has not changed, so a setter comparing the
    /// effective value would skip it and the row would stay nested
    /// forever — an upgrade that migrates nothing. The second write is a
    /// no-op again, which is what makes this idempotent rather than a
    /// block on every startup.
    #[test]
    fn the_first_write_migrates_an_old_row_and_the_next_one_writes_nothing() {
        let d = DeviceDoc::new(1).unwrap();
        write_old_shape(&d, "aa", "turing", &["10.0.0.2:11204"], Some("{\"v\":1}"), false);
        assert_eq!(d.flat_str("aa", F_NAME), None, "control: nothing flat yet");

        // Re-registering with the same values is what an upgraded
        // machine does on its next open
        d.upsert("aa", "turing", &["10.0.0.2:11204".into()]).unwrap();
        assert_eq!(
            d.flat_str("aa", F_NAME).as_deref(),
            Some("turing"),
            "the unchanged value still has to reach the register once"
        );

        let after_migration = d.version();
        d.upsert("aa", "turing", &["10.0.0.2:11204".into()]).unwrap();
        assert_eq!(
            d.version(),
            after_migration,
            "…and the write after that is a no-op again"
        );
        // Nothing was lost on the way across
        assert_eq!(d.get("aa").unwrap().style.as_deref(), Some("{\"v\":1}"));
    }

    /// **An older khor still reads everything this version writes.** The
    /// mirror is what keeps a mixed network from breaking silently: drop
    /// it and a machine that joined through a new khor is simply absent
    /// from an old one's list, with nothing anywhere saying so.
    ///
    /// This is the test that goes red if someone removes half the shim.
    /// Removing all of it is a decision (module head: when no khor that
    /// predates the flat table is left), and it deletes this test too.
    #[test]
    fn an_old_khor_still_reads_what_this_version_writes() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &["9.9.9.9:9".into()]).unwrap();
        d.set_style("aa", "{\"shape\":\"square\"}").unwrap();
        d.set_pinned("aa", true).unwrap();

        let (name, addrs, style, pinned) =
            read_old_shape(&d, "aa").expect("an old khor must still find the row");
        assert_eq!(name, "mac");
        assert_eq!(addrs, vec!["9.9.9.9:9".to_string()]);
        assert_eq!(style.as_deref(), Some("{\"shape\":\"square\"}"));
        assert!(pinned);

        // …including a later change, not just the row's first state
        d.upsert("aa", "renamed", &["9.9.9.9:9".into()]).unwrap();
        d.set_pinned("aa", false).unwrap();
        let (name, _, _, pinned) = read_old_shape(&d, "aa").unwrap();
        assert_eq!(name, "renamed", "a rename has to reach the old shape too");
        assert!(!pinned, "and so does taking a pin back");
    }

    /// A row an older khor pinned can be unpinned here. The setter
    /// compares the flat register to decide whether to write, and a
    /// register that does not exist yet reads as "not pinned" — so
    /// without looking at the effective value first, taking back a pin
    /// that only exists in the old shape would silently do nothing.
    #[test]
    fn a_pin_written_by_an_older_khor_can_be_taken_back() {
        let d = DeviceDoc::new(1).unwrap();
        write_old_shape(&d, "aa", "mac", &[], None, true);
        assert!(d.get("aa").unwrap().pinned, "control: it arrived pinned");

        d.set_pinned("aa", false).unwrap();
        assert!(!d.get("aa").unwrap().pinned, "unpinning an old row must take");
        assert!(
            !read_old_shape(&d, "aa").unwrap().3,
            "…on the old side of the network as well"
        );
    }

    /// Unpinning a machine nobody pinned writes nothing — `khor unpin
    /// -m` on a stranger is a call people make, and it must not cost a
    /// block. The other setters get this from their value comparison;
    /// this one needs it stated, because an absent register and an
    /// explicit `false` read the same.
    #[test]
    fn unpinning_a_machine_that_was_never_pinned_writes_nothing() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &[]).unwrap();
        let before = d.version();
        d.set_pinned("aa", false).unwrap();
        assert_eq!(d.version(), before, "no change must mean no new ops");
    }
}
