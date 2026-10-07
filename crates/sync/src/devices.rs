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
//! `style` is a device's avatar style, one JSON object (whole-value
//! LWW). It rides the table rather than a wire frame because of what it
//! has to achieve: **a machine looks the same to everyone** — whoever
//! is looking derives its face from this one row, not from local
//! preference. Whole-value and not per-slot: half a palette from one
//! writer and a variant from another is a face nobody chose.
//!
//! > **It was 「self-reported」 until 2026-09-21, and that half is gone.**
//! > 「头像设置这, 要支持对每台机器的修改」 (the user, 2026-09-20).
//! > Anyone may now write anyone's style, which is the rule `F_LABEL`
//! > has always had and for the same reason: this is one person's fleet,
//! > so 「who is allowed to name that machine」 was never a question the
//! > network needed to answer. **One field, last writer wins** — the
//! > ruling names the alternative and refuses it: 「不要做成「自报一份
//! > + 覆盖一份」两个字段」. A machine restyling itself writes the
//! > same register a phone across the room writes, and the later write
//! > is the face.
//!
//! `pinned` is "this machine matters", and it rides the table for the
//! reason `khor_sync::pins` states for sessions: a pin belongs to the
//! thing pinned, so it has to be one fact the network agrees on rather
//! than one answer per screen. It lands in the same place `style` does
//! and now under the same rule — anyone may write it about anyone —
//! because the device table is already the one row per machine everybody
//! replicates, and a second table keyed by device id would be the same
//! key twice.
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
/// A machine that was shown the door. Written like any other field, so
/// it travels like any other field — see [`DeviceDoc::set_gone`].
const F_GONE: &str = "gone";
/// The machine that adopted this satellite (device id, hex) — present
/// exactly when the row is a satellite (附属机器批: a machine only ssh
/// can reach, spoken for by its sponsor; docs/NET.md 卫星). Fields, not
/// a new document: an older khor replicates unknown registers untouched
/// and paints the row as an ordinary unreachable machine, which is the
/// safe misreading.
const F_SPONSOR: &str = "sponsor";
/// The ssh destination the sponsor reaches it by (a Host alias or
/// user@host). Only meaningful where the sponsor's own ssh config is —
/// it replicates anyway because it is a decision, not a credential, and
/// the sponsor must still know it after its own restart.
const F_SSH_HOST: &str = "ssh_host";
/// Whether the satellite had tmux when it was adopted (or last probed)
/// — what decides if a terminal on it can be made persistent. Stored as
/// a bool register but read three-valued: "unprobed" must not read as
/// "absent".
const F_SAT_TMUX: &str = "sat_tmux";
/// The directories this machine is asked to keep an eye on (2026-09-06
/// 关注目录): one JSON array, whole-value LWW like `addrs`, and anyone
/// may write it like `pinned` — it is a fact about the machine that the
/// person looking at it decides. It rides the table for `pinned`'s
/// reason (the head): one row per machine that everybody replicates
/// already, and a second table keyed by device id would be the same key
/// twice. What the whole-value rule costs: two people editing one
/// machine's list at the same moment, and the later write wins whole —
/// the trade `addrs` already made. An older khor replicates the register
/// untouched and never reads it.
const F_WATCH: &str = "watch";
/// Where files sent to this machine land (STORE-BRIEF phase 1): a
/// directory, absolute or `~`-led, and empty or absent for the default
/// (`~/khor-drop`; on a phone, the app's Documents). Anyone may write it,
/// `watch`'s rule: it is a fact about the machine that whoever looks at it
/// decides, and it reads the same on every screen. Only the machine itself
/// resolves it (`~` is its home, and only it can tell whether the
/// directory is usable), and says where it resolved to in `F_RECEIVE_AT`.
/// An older khor replicates the register untouched and never reads it.
const F_RECEIVE_DIR: &str = "receive_dir";
/// Where this machine's received files land **now**, absolute, as the
/// machine itself last resolved `F_RECEIVE_DIR` — the setting expanded,
/// or the default when the setting is unusable. Written by the machine
/// alone; another screen lists this directory and compares it with the
/// setting to say when the setting did not take.
const F_RECEIVE_AT: &str = "receive_at";
/// The name a person gave this machine, overriding the one it reports
/// about itself (`F_NAME`).
///
/// **Two names, and the split is the whole point.** `F_NAME` is the
/// channel — what the network files the machine under, settled once per
/// store (`Node::open_as`), and the segment in `chat/<name>` that
/// [`crate::seen`] and [`crate::pins`] key on. `F_LABEL` is what people
/// read. Renaming moves the label and nothing else, so it cannot strand
/// a directory, a watermark or a pin — which is exactly what renaming
/// the *machine* used to do.
///
/// Anyone may write it, like `pinned` and `watch`: what a machine is
/// called is a fact about it that whoever is looking decides, and it
/// should read the same on every screen. Empty means「no override」and
/// is how a label is taken back — stored rather than deleted, because a
/// register that goes absent and a register that goes empty must not
/// have to be told apart by every reader.
///
/// `upsert` never touches it. That is load-bearing: the machine
/// re-reports its own name on every sync, and a label written into
/// `F_NAME` would be overwritten by its owner within seconds — with
/// nothing anywhere saying why the new name kept vanishing.
const F_LABEL: &str = "label";
/// This machine's certificate, signed by the user's root key
/// (docs/CONNECT.md 名单是信任的唯一出处) — **opaque bytes here**: this
/// crate replicates documents and never reads inside a signature
/// (`khor_net::trust` does, at the one gate that decides trust).
///
/// It rides the row so a machine's proof of membership travels with the
/// row everybody already replicates, and anybody holding the root's
/// public key can check it without asking. **Last writer wins here and
/// that decides nothing**: a reader verifies what it finds and holds the
/// roster ([`TRUST`]) as the authority — a stale or forged value in this
/// register is a certificate that fails to count, not a trust decision.
///
/// Written flat only, never into the old nested mirror: no khor that
/// reads the nested shape knows what a certificate is.
const F_CERT: &str = "cert";
/// Whether this machine can host a session (`khor_core::hosts_here`):
/// `false` on a phone, which may not start a process, has no tmux, no
/// VS Code and nothing to open a file with. **Only the machine itself
/// writes it** (`Node::register_self`, on every open) — it is a fact
/// about the build it runs, not a setting anyone decides.
///
/// **Absent reads as hosting**, and that is a default rather than a
/// shim: every khor that predates the field ran on a computer, so a row
/// it wrote is a computer's row. Written flat only, never into the old
/// nested mirror (`F_CERT`'s reason: no khor that reads the nested
/// shape knows what this is); an older khor replicates the register
/// untouched and never reads it.
const F_HOSTS: &str = "hosts";

/// **The roads a machine reports about itself** (round NET-12, the 我的
/// khor dial): one register per peer, `<id>/road/<to>`, holding a small
/// JSON object (`khor_core::Road`, minus `to`). Only the machine itself
/// writes its own (`khor_node::roadbook`), and only when the road's kind
/// changed and held, or its round trip moved by more than a quarter at
/// most once in ten minutes — so the table grows by tens of ops a day,
/// not by every reading. Flat only, `F_CERT`'s reason; an older khor
/// replicates the registers untouched and never reads them.
const F_ROAD: &str = "road/";

/// The roster's home: a map **beside** `devices`, not inside it, keyed by
/// whatever the writer names a version (`khor_node` uses the roster's
/// `seq` and a piece of its signature). Every signed version is its own
/// key, so two replicas that each wrote one keep both after a merge —
/// **nothing here is last-writer-wins about trust**: readers verify every
/// value and take the highest `seq` (`khor_net::trust::newest`).
///
/// Beside rather than inside because every key in `devices` is read as
/// `<id>/<field>`; a roster key there would read as a machine.
const TRUST: &str = "trust";

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
    /// The device id of the machine that adopted this row, when the row
    /// is a satellite (附属机器批) — `None` for every ordinary machine.
    /// Everything a reader needs to route: a satellite's home is its
    /// sponsor.
    pub sponsor: Option<String>,
    /// The ssh destination its sponsor reaches it by. Meaningful only
    /// on the sponsor (whose ssh config defines it); carried on the row
    /// so the sponsor still knows it after a restart.
    pub ssh_host: Option<String>,
    /// `Some(true)` = tmux was there at the last probe, `Some(false)` =
    /// probed and absent, `None` = never probed (an adoption from an
    /// older khor). Three states on purpose: unprobed must not read as
    /// absent.
    pub sat_tmux: Option<bool>,
    /// The directories it is asked to watch, in the order they were
    /// given — empty when nobody asked (`F_WATCH`). The list is the
    /// setting; the readings for it come from the machine itself
    /// (`khor_core::WatchedDir`).
    pub watch: Vec<String>,
    /// What a person renamed it to, or `None` when nobody has
    /// (`F_LABEL`). Read it through [`DeviceInfo::shown`] rather than
    /// directly — every screen must make the same choice between this
    /// and `name`.
    pub label: Option<String>,
    /// Whether the machine can host a session (`F_HOSTS`): `false` for a
    /// phone, `true` for a computer and for a row that never said.
    pub hosts: bool,
    /// The roads this machine reports to the others (`F_ROAD`), as
    /// `(to, the register's JSON)`: opaque here, read by `khor_node`.
    pub roads: Vec<(String, String)>,
    /// Where files sent to it should land, as set (`F_RECEIVE_DIR`):
    /// `None` for the default.
    pub receive_dir: Option<String>,
    /// Where they land now, as the machine itself resolved it
    /// (`F_RECEIVE_AT`): `None` from a khor that predates the setting.
    pub receive_at: Option<String>,
}

impl DeviceInfo {
    /// The name to put on the glass: the override when there is one,
    /// else what the machine calls itself.
    ///
    /// **One function, so「which name」 has one answer.** The list, the
    /// card, the CLI table and the chat header each used `name`
    /// directly; four readers each free to choose is four chances for
    /// two screens to disagree about what a machine is called.
    pub fn shown(&self) -> &str {
        match self.label.as_deref() {
            Some(l) if !l.trim().is_empty() => l,
            _ => &self.name,
        }
    }
}

pub struct DeviceDoc {
    inner: LoroDoc,
}

impl DeviceDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self {
            inner: glue::with_peer(peer)?,
        })
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
    fn write_field(
        &self,
        id: &str,
        field: &str,
        v: impl Into<LoroValue> + Clone,
    ) -> Result<(), String> {
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
    /// **Deliberately does not touch `style` or `pinned`.** Everyone
    /// writes this for their peers (pairing writes the far side's name
    /// and addresses), and this call runs on every open — folding
    /// either of those in would have a routine update silently clear a
    /// face somebody chose or a pin somebody set. That `style` may now
    /// be written *by anyone* (see the module head) does not make it
    /// something to write *by accident*: the rule here is about which
    /// call carries the intent, not about who is allowed to have it.
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

    /// Sets a device's avatar style. **Anyone may, for anyone**
    /// — [`DeviceDoc::set_label`]'s rule, and the module head says why
    /// it is now this one's too. One field and the later write wins;
    /// there is deliberately no second field for 「what the machine says
    /// about itself」, because two fields is two answers and the screen
    /// would have to pick one.
    ///
    /// Unchanged means unwritten, same as [`DeviceDoc::upsert`]: a
    /// rewrite that changes nothing inflates the version vector for
    /// nothing.
    pub fn set_style(&self, id: &str, style_json: &str) -> Result<(), String> {
        if self.flat_str(id, F_STYLE).as_deref() == Some(style_json) {
            return Ok(());
        }
        self.write_field(id, F_STYLE, style_json)?;
        self.inner.commit();
        Ok(())
    }

    /// Pins or unpins a machine. Anyone may pin anyone, which is
    /// [`DeviceDoc::set_style`]'s rule as well since 2026-09-21.
    /// Unchanged means unwritten, same reason as the others.
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

    /// Sets the list of directories a machine is asked to watch. Anyone
    /// may, for anyone, `set_pinned`'s rule; the caller has already
    /// tidied the list (`khor_node` refuses a relative path, a `..`, a
    /// duplicate — one voice for the CLI and the face). Unchanged means
    /// unwritten, the others' reason.
    pub fn set_watch(&self, id: &str, paths: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(paths).map_err(|e| e.to_string())?;
        if paths.is_empty() && self.field(id, F_WATCH).is_none() {
            return Ok(());
        }
        if self.flat_str(id, F_WATCH).as_deref() == Some(json.as_str()) {
            return Ok(());
        }
        self.write_field(id, F_WATCH, json.as_str())?;
        self.inner.commit();
        Ok(())
    }

    /// Sets where files sent to a machine land, for everyone; empty takes
    /// the setting back to the default. Anyone may, for anyone,
    /// `set_watch`'s rule; the caller has already tidied the value
    /// (`khor_node` refuses a relative path). Unchanged means unwritten.
    pub fn set_receive_dir(&self, id: &str, dir: &str) -> Result<(), String> {
        self.set_str_once(id, F_RECEIVE_DIR, dir)
    }

    /// Records where a machine's received files land now — written by
    /// that machine alone, after it resolved the setting.
    pub fn set_receive_at(&self, id: &str, path: &str) -> Result<(), String> {
        self.set_str_once(id, F_RECEIVE_AT, path)
    }

    /// One string register, written only when it changes; empty is
    /// stored rather than deleted (`F_LABEL`'s reason) unless it was
    /// never written at all.
    fn set_str_once(&self, id: &str, field: &str, v: &str) -> Result<(), String> {
        match self.flat_str(id, field) {
            Some(was) if was == v => return Ok(()),
            None if v.is_empty() && self.field(id, field).is_none() => return Ok(()),
            _ => {}
        }
        self.write_field(id, field, v)?;
        self.inner.commit();
        Ok(())
    }

    /// Marks a row as a satellite: spoken for by `sponsor`, reached over
    /// ssh at `ssh_host`, with tmux present or not as probed
    /// (附属机器批). One call rather than three setters because the
    /// three facts are one decision — a row with a sponsor but no ssh
    /// destination is a satellite nobody can reach.
    ///
    /// Any paired machine may write it (it is how adoption enters the
    /// table), but in practice only the sponsor does: it is the one
    /// holding the ssh config the destination means anything to.
    pub fn set_satellite(
        &self,
        id: &str,
        sponsor: &str,
        ssh_host: &str,
        tmux: bool,
    ) -> Result<(), String> {
        let mut dirty = false;
        if self.flat_str(id, F_SPONSOR).as_deref() != Some(sponsor) {
            self.write_field(id, F_SPONSOR, sponsor)?;
            dirty = true;
        }
        if self.flat_str(id, F_SSH_HOST).as_deref() != Some(ssh_host) {
            self.write_field(id, F_SSH_HOST, ssh_host)?;
            dirty = true;
        }
        if self.flat(id, F_SAT_TMUX) != Some(LoroValue::Bool(tmux)) {
            self.write_field(id, F_SAT_TMUX, tmux)?;
            dirty = true;
        }
        if dirty {
            self.inner.commit();
        }
        Ok(())
    }

    /// Shows a machine the door, network-wide — or takes it back in.
    ///
    /// **Leaving is expressed as "not in the table", which is why this
    /// needs no new gate.** Every door already refuses a machine the
    /// table does not hold (`msg::not_paired()`), and [`DeviceDoc::all`]
    /// — the only way anything reads this doc — skips a machine that is
    /// `gone`. So the whole of "it can no longer act here" falls out of
    /// one bit, rather than out of a list of places that must each
    /// remember to check.
    ///
    /// **It propagates, because joining does.** A mesh whose rule is
    /// 接入一次即在网里 cannot have leaving be a private opinion: a
    /// machine forgotten on one laptop and still trusted on another is
    /// not out of the network, it is out of one person's view of it.
    /// The bit rides the same document the membership does.
    ///
    /// A tombstone rather than a deletion, and for the ordinary CRDT
    /// reason: the removed machine still holds its own copy of this
    /// document and keeps writing its own row (`Link` records its
    /// addresses on every start). A row that was merely deleted would
    /// come back on the next merge; a row marked gone stays gone,
    /// because nothing that machine writes touches this field.
    ///
    /// Pairing clears it — that is what taking a machine back in *is*,
    /// and it is deliberately not something the machine can do alone.
    pub fn set_gone(&self, id: &str, on: bool) -> Result<(), String> {
        if !on && self.field(id, F_GONE).is_none() {
            return Ok(());
        }
        if self.flat(id, F_GONE) == Some(LoroValue::Bool(on)) {
            return Ok(());
        }
        self.write_field(id, F_GONE, on)?;
        self.inner.commit();
        Ok(())
    }

    /// Throws every row away but one — what is left of this table after
    /// the machine holding it leaves the network (`Node::leave`).
    ///
    /// **A real delete, not the tombstone [`DeviceDoc::set_gone`]
    /// writes**, and the difference is which direction the news is
    /// travelling. `set_gone` is a statement *to* the network about a
    /// machine, so it has to survive merging with a replica that still
    /// holds the row. This is the opposite: the network has already been
    /// told (that is what the tombstone on the leaver's own row was
    /// for), and what is being emptied is one machine's private copy of
    /// a document it is done with. There is no later merge to resurrect
    /// anything — a machine the table does not hold is refused at every
    /// door (`msg::not_paired`), so the peers cannot sync back in, and
    /// re-joining goes through a ticket, which arrives as a fresh
    /// snapshot.
    ///
    /// Field-agnostic on purpose: it deletes by key rather than
    /// rebuilding the kept row out of the fields this module knows
    /// today, so a field added tomorrow is not silently dropped on the
    /// one row that has to survive.
    pub fn keep_only(&self, id: &str) -> Result<(), String> {
        let map = self.map();
        // Both shapes: `<id>/<field>` registers and the old nested
        // entry keyed by the id itself ([`DeviceDoc::ids`]).
        let doomed: Vec<String> = map
            .keys()
            .map(|k| k.to_string())
            .filter(|k| match k.split_once(SEP) {
                Some((owner, _)) => owner != id,
                None => k != id,
            })
            .collect();
        if doomed.is_empty() {
            return Ok(());
        }
        for k in &doomed {
            map.delete(k).map_err(|e| e.to_string())?;
        }
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
            .filter(|id| !self.bool_field(id, F_GONE))
            // A machine shown the door is not a machine with a flag on
            // it — it is one this table no longer holds. Filtering here
            // is what makes that true everywhere at once, `get` and
            // `by_name` included, since both read through this.
            .map(|id| DeviceInfo {
                name: self.str_field(&id, F_NAME).unwrap_or_default(),
                addrs: self
                    .str_field(&id, F_ADDRS)
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                style: self.str_field(&id, F_STYLE),
                pinned: self.bool_field(&id, F_PINNED),
                sponsor: self.str_field(&id, F_SPONSOR),
                ssh_host: self.str_field(&id, F_SSH_HOST),
                sat_tmux: match self.field(&id, F_SAT_TMUX) {
                    Some(LoroValue::Bool(b)) => Some(b),
                    _ => None,
                },
                watch: self
                    .str_field(&id, F_WATCH)
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                label: self
                    .str_field(&id, F_LABEL)
                    .filter(|l| !l.trim().is_empty()),
                hosts: !matches!(self.flat(&id, F_HOSTS), Some(LoroValue::Bool(false))),
                roads: self.roads_raw(&id),
                receive_dir: self
                    .str_field(&id, F_RECEIVE_DIR)
                    .filter(|d| !d.trim().is_empty()),
                receive_at: self
                    .str_field(&id, F_RECEIVE_AT)
                    .filter(|d| !d.trim().is_empty()),
                id,
            })
            .collect();
        // **Ordered by the name on the glass**, not by the channel: a
        // list sorted on a name nobody can see reads as unsorted, and
        // renaming would move a row to a place with no rule behind it.
        out.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| a.shown().cmp(b.shown()))
        });
        out
    }

    pub fn get(&self, id: &str) -> Option<DeviceInfo> {
        self.all().into_iter().find(|d| d.id == id)
    }

    /// A machine by either of its names — what people call it, or what
    /// it calls itself.
    ///
    /// **The label is tried first and the channel second**, and the
    /// order is the answer to one real collision: rename A to 「B」 while
    /// a machine truly channelled `B` exists. Somebody typing `B` then
    /// means the one wearing the name, because that is the only one they
    /// can see it on. The machine underneath is still reachable by id,
    /// which is what every non-human path uses anyway.
    pub fn by_name(&self, name: &str) -> Option<DeviceInfo> {
        let all = self.all();
        all.iter()
            .find(|d| d.label.as_deref() == Some(name))
            .or_else(|| all.iter().find(|d| d.name == name))
            .cloned()
    }

    /// Stores a machine's certificate on its row (`F_CERT`). Unchanged
    /// means unwritten, the others' reason.
    pub fn set_cert(&self, id: &str, cert: &[u8]) -> Result<(), String> {
        if let Some(LoroValue::Binary(have)) = self.flat(id, F_CERT)
            && have.as_slice() == cert
        {
            return Ok(());
        }
        self.map()
            .insert(&fkey(id, F_CERT), cert)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Says whether a machine can host (`F_HOSTS`). The machine says it
    /// about itself on every open; unchanged means unwritten, the
    /// others' reason. Flat only — see `F_HOSTS`.
    pub fn set_hosts(&self, id: &str, hosts: bool) -> Result<(), String> {
        if self.flat(id, F_HOSTS) == Some(LoroValue::Bool(hosts)) {
            return Ok(());
        }
        self.map()
            .insert(&fkey(id, F_HOSTS), hosts)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// The certificate a row carries, unverified — `None` when it carries
    /// none. **Read for gone rows too**: a certificate is a fact about a
    /// key, and whether the table still lists the machine is a separate
    /// question.
    pub fn cert_of(&self, id: &str) -> Option<Vec<u8>> {
        match self.flat(id, F_CERT)? {
            LoroValue::Binary(b) => Some(b.to_vec()),
            _ => None,
        }
    }

    fn trust_map(&self) -> LoroMap {
        self.inner.get_map(TRUST)
    }

    /// Stores one signed roster under `key` ([`TRUST`]). A key already
    /// holding these bytes is left alone.
    pub fn put_roster(&self, key: &str, bytes: &[u8]) -> Result<(), String> {
        let map = self.trust_map();
        if let Some(LoroValue::Binary(have)) = map.get(key).and_then(|v| v.into_value().ok())
            && have.as_slice() == bytes
        {
            return Ok(());
        }
        map.insert(key, bytes).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every stored roster, unverified, with its key.
    pub fn rosters(&self) -> Vec<(String, Vec<u8>)> {
        let map = self.trust_map();
        map.keys()
            .filter_map(|k| {
                let k = k.to_string();
                match map.get(&k)?.into_value().ok()? {
                    LoroValue::Binary(b) => Some((k, b.to_vec())),
                    _ => None,
                }
            })
            .collect()
    }

    /// Removes the rosters under `keys` — a holder pruning versions a
    /// newer one has replaced, or a machine clearing a root it gave up.
    pub fn drop_rosters(&self, keys: &[String]) -> Result<(), String> {
        let map = self.trust_map();
        let mut dirty = false;
        for k in keys {
            if map.get(k).is_some() {
                map.delete(k).map_err(|e| e.to_string())?;
                dirty = true;
            }
        }
        if dirty {
            self.inner.commit();
        }
        Ok(())
    }

    /// Every machine the table has shown the door ([`DeviceDoc::set_gone`])
    /// — the rows [`DeviceDoc::all`] hides. What a root holder reads to
    /// see which removals still wait for a revocation.
    pub fn gone_ids(&self) -> Vec<String> {
        self.ids()
            .into_iter()
            .filter(|id| self.bool_field(id, F_GONE))
            .collect()
    }

    /// The name a row carries, gone or not — what a pending removal is
    /// shown under.
    pub fn name_of(&self, id: &str) -> Option<String> {
        let label = self.str_field(id, F_LABEL).filter(|l| !l.trim().is_empty());
        label.or_else(|| self.str_field(id, F_NAME))
    }

    /// Writes one road `id` reports to `to` (`F_ROAD`); `false`, and no
    /// op, when the register already holds `json`.
    pub fn set_road(&self, id: &str, to: &str, json: &str) -> Result<bool, String> {
        let key = fkey(id, &format!("{F_ROAD}{to}"));
        if let Some(LoroValue::String(have)) = self.map().get(&key).and_then(|v| v.into_value().ok())
            && have.as_str() == json
        {
            return Ok(false);
        }
        self.map().insert(&key, json).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(true)
    }

    /// Every road `id` reports, as `(to, json)`, sorted by `to`.
    pub fn roads_raw(&self, id: &str) -> Vec<(String, String)> {
        let prefix = fkey(id, F_ROAD);
        let map = self.map();
        let mut out: Vec<(String, String)> = map
            .keys()
            .filter_map(|k| {
                let k = k.to_string();
                let to = k.strip_prefix(&prefix)?.to_owned();
                match map.get(&k)?.into_value().ok()? {
                    LoroValue::String(v) => Some((to, v.to_string())),
                    _ => None,
                }
            })
            .collect();
        out.sort();
        out
    }

    /// Renames a machine, or takes the rename back when `label` is
    /// empty. Anybody may call it (`F_LABEL`).
    pub fn set_label(&self, id: &str, label: &str) -> Result<(), String> {
        let label = label.trim();
        // Clearing a label nobody set must not cost an op — the same
        // rule `set_pinned` keeps, and for the same reason: the gesture
        // is one people make on rows they never touched.
        if label.is_empty() && self.field(id, F_LABEL).is_none() {
            return Ok(());
        }
        if self.flat_str(id, F_LABEL).as_deref() == Some(label) {
            return Ok(());
        }
        self.write_field(id, F_LABEL, label)?;
        self.inner.commit();
        Ok(())
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

    /// The watch list travels like a pin, and an upsert leaves it alone:
    /// pairing writes names and addresses for every peer, and a list
    /// that a routine upsert could clear is a setting that lasts until
    /// the next start.
    #[test]
    fn a_watch_list_travels_and_an_upsert_does_not_touch_it() {
        let a = DeviceDoc::new(1).unwrap();
        let b = DeviceDoc::new(2).unwrap();
        a.upsert("cc", "turing", &[]).unwrap();
        a.set_watch("cc", &["/data/models".into(), "/scratch".into()])
            .unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(b.get("cc").unwrap().watch, vec!["/data/models", "/scratch"]);
        b.upsert("cc", "turing", &["10.0.0.9:11204".into()])
            .unwrap();
        assert_eq!(
            b.get("cc").unwrap().watch.len(),
            2,
            "an upsert must not clear the list"
        );
        b.set_watch("cc", &[]).unwrap();
        a.merge(&b.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert!(
            a.get("cc").unwrap().watch.is_empty(),
            "clearing travels back"
        );
        assert_eq!(d_items(&a), 1);
    }

    /// The receive location travels like the watch list, an upsert
    /// leaves it alone, and empty takes it back to the default; where the
    /// machine says its files land rides beside it, apart.
    #[test]
    fn a_receive_dir_travels_and_an_upsert_does_not_touch_it() {
        let a = DeviceDoc::new(1).unwrap();
        let b = DeviceDoc::new(2).unwrap();
        a.upsert("cc", "turing", &[]).unwrap();
        assert_eq!(a.get("cc").unwrap().receive_dir, None, "unset is the default");
        a.set_receive_dir("cc", "~/inbox").unwrap();
        a.set_receive_at("cc", "/home/t/inbox").unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap()).unwrap();
        let got = b.get("cc").unwrap();
        assert_eq!(got.receive_dir.as_deref(), Some("~/inbox"));
        assert_eq!(got.receive_at.as_deref(), Some("/home/t/inbox"));
        b.upsert("cc", "turing", &["10.0.0.9:11204".into()]).unwrap();
        assert_eq!(b.get("cc").unwrap().receive_dir.as_deref(), Some("~/inbox"), "an upsert must not clear it");
        b.set_receive_dir("cc", "").unwrap();
        a.merge(&b.changes_since(&Default::default()).unwrap()).unwrap();
        assert_eq!(a.get("cc").unwrap().receive_dir, None, "clearing travels back");
        assert_eq!(a.get("cc").unwrap().receive_at.as_deref(), Some("/home/t/inbox"));
    }

    fn d_items(d: &DeviceDoc) -> usize {
        d.items()
    }

    #[test]
    fn a_device_round_trips_with_name_and_addrs() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa11", "turing", &["10.0.0.2:11204".into()])
            .unwrap();
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

        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        a.merge(&b.changes_since(&Default::default()).unwrap())
            .unwrap();

        for (who, d) in [("a", &a), ("b", &b)] {
            let names: Vec<String> = d.all().into_iter().map(|x| x.name).collect();
            assert_eq!(
                names,
                vec!["mac", "phone"],
                "{who} should see the whole network"
            );
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
            assert_eq!(
                got.addrs,
                vec!["1.2.3.4:5".to_string()],
                "{who}: the address must survive"
            );
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
        mine.merge(&theirs.changes_since(&mine.version()).unwrap())
            .unwrap();
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
        assert!(
            b.get("cc").unwrap().pinned,
            "the pin must reach the other device"
        );
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

        mine.merge(&theirs.changes_since(&mine.version()).unwrap())
            .unwrap();
        assert!(
            mine.get("aa").unwrap().pinned,
            "…nor on the round trip home"
        );
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
        alpha
            .merge(&beta.changes_since(&alpha.version()).unwrap())
            .unwrap();
        assert_eq!(
            alpha.get("bb").unwrap().style.as_deref(),
            Some("{\"mine\":true}")
        );
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
    fn read_old_shape(
        d: &DeviceDoc,
        id: &str,
    ) -> Option<(String, Vec<String>, Option<String>, bool)> {
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
        write_old_shape(
            &d,
            "aa",
            "turing",
            &["10.0.0.2:11204"],
            Some("{\"v\":1}"),
            true,
        );

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
        write_old_shape(
            &d,
            "aa",
            "turing",
            &["10.0.0.2:11204"],
            Some("{\"v\":1}"),
            false,
        );
        assert_eq!(d.flat_str("aa", F_NAME), None, "control: nothing flat yet");

        // Re-registering with the same values is what an upgraded
        // machine does on its next open
        d.upsert("aa", "turing", &["10.0.0.2:11204".into()])
            .unwrap();
        assert_eq!(
            d.flat_str("aa", F_NAME).as_deref(),
            Some("turing"),
            "the unchanged value still has to reach the register once"
        );

        let after_migration = d.version();
        d.upsert("aa", "turing", &["10.0.0.2:11204".into()])
            .unwrap();
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
        assert!(
            !d.get("aa").unwrap().pinned,
            "unpinning an old row must take"
        );
        assert!(
            !read_old_shape(&d, "aa").unwrap().3,
            "…on the old side of the network as well"
        );
    }

    /// **The point of a tombstone is that the machine cannot undo it by
    /// existing.** A removed machine keeps its own copy of this document
    /// and writes its own row on every start (`Link` records its
    /// addresses there), and those writes merge back. A row that was
    /// merely deleted would return on the next merge and the removal
    /// would read as a glitch; the bit has to survive the writes the
    /// removed machine keeps making.
    ///
    /// The first assertion is the control: without it a `set_gone` that
    /// did nothing at all would pass the one that matters.
    #[test]
    fn a_machine_shown_the_door_does_not_come_back_by_writing_its_own_row() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &["1.2.3.4:5".into()]).unwrap();
        assert!(d.get("aa").is_some(), "control: it was in the table");

        d.set_gone("aa", true).unwrap();
        assert!(
            d.get("aa").is_none(),
            "shown the door means out of the table"
        );
        assert!(
            !d.all().iter().any(|x| x.id == "aa"),
            "…and out of every reading of it"
        );

        // Exactly what that machine's own serve does on every start.
        d.upsert("aa", "mac", &["9.9.9.9:1".into()]).unwrap();
        assert!(
            d.get("aa").is_none(),
            "writing its own row must not let it back in"
        );

        // And the way back is a pairing, which is somebody else's act.
        d.set_gone("aa", false).unwrap();
        let back = d.get("aa").expect("pairing takes a machine back in");
        assert_eq!(
            back.addrs,
            vec!["9.9.9.9:1".to_string()],
            "with what it last said about itself"
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

    /// **Whether a machine can host travels, and absence is a computer**
    /// (docs/IOS.md 第二步). A phone says no about itself; every table
    /// it merges into says no for it — so a desktop never offers a
    /// session there. A row that never said (any khor before this field,
    /// and a row written in the old nested shape) reads as hosting: the
    /// control, without which an implementation that read every row as
    /// a phone would pass the first half.
    #[test]
    fn a_machine_that_cannot_host_says_so_everywhere_and_silence_is_a_computer() {
        let phone = DeviceDoc::new(1).unwrap();
        phone.upsert("bb", "phone", &[]).unwrap();
        phone.set_hosts("bb", false).unwrap();
        assert!(!phone.get("bb").unwrap().hosts, "it says no about itself");
        let before = phone.version();
        phone.set_hosts("bb", false).unwrap();
        assert_eq!(phone.version(), before, "saying it again writes nothing");

        let desk = DeviceDoc::new(2).unwrap();
        desk.upsert("aa", "mac", &[]).unwrap();
        desk.set_hosts("aa", true).unwrap();
        write_old_shape(&desk, "cc", "turing", &[], None, false);
        desk.merge(&phone.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert!(!desk.get("bb").unwrap().hosts, "the no crosses the network");
        assert!(desk.get("aa").unwrap().hosts, "a computer says yes");
        assert!(
            desk.get("cc").unwrap().hosts,
            "a row from a khor before the field is a computer's"
        );
        // A routine upsert (pairing re-writes names and addresses for
        // every peer) must not take the answer back.
        desk.upsert("bb", "phone", &["10.0.0.7:1".into()]).unwrap();
        assert!(!desk.get("bb").unwrap().hosts, "an upsert leaves it alone");
        let nested = desk.inner.get_map(DEVICES).get("bb").and_then(as_map).unwrap();
        assert!(
            nested.get(F_HOSTS).is_none(),
            "flat only — nothing new in the old mirror"
        );
    }

    /// The satellite mark round-trips and replicates (附属机器批): the
    /// sponsor writes it once, every merged table answers with sponsor,
    /// ssh destination and the tmux probe — and an ordinary machine
    /// answers `None` on all three (the control: an implementation that
    /// stamped everything a satellite would pass the first half alone).
    #[test]
    fn a_satellite_mark_replicates_and_ordinary_machines_carry_none() {
        let sponsor = DeviceDoc::new(1).unwrap();
        sponsor.upsert("aa", "mac", &[]).unwrap();
        sponsor
            .upsert("5a".repeat(32).as_str(), "gpu-box", &[])
            .unwrap();
        sponsor
            .set_satellite("5a".repeat(32).as_str(), "aa", "gpubox", true)
            .unwrap();

        let far = DeviceDoc::new(2).unwrap();
        far.merge(&sponsor.changes_since(&Default::default()).unwrap())
            .unwrap();
        let sat = far
            .get("5a".repeat(32).as_str())
            .expect("the satellite crossed");
        assert_eq!(sat.sponsor.as_deref(), Some("aa"), "the sponsor travels");
        assert_eq!(
            sat.ssh_host.as_deref(),
            Some("gpubox"),
            "…with the ssh destination"
        );
        assert_eq!(sat.sat_tmux, Some(true), "…and the tmux probe");
        let plain = far.get("aa").expect("the sponsor itself crossed");
        assert_eq!(
            (plain.sponsor, plain.ssh_host, plain.sat_tmux),
            (None, None, None),
            "an ordinary machine is no satellite"
        );
    }

    /// The rename, and the four things that would each quietly undo it.
    ///
    /// **The first arm is the load-bearing one.** A machine re-reports
    /// its own name on every sync; a label kept in `F_NAME` would be
    /// overwritten by its owner within seconds, and nothing anywhere
    /// would say why the new name kept vanishing. Put the label back
    /// into `F_NAME` and this arm is the one that goes red.
    #[test]
    fn a_rename_survives_the_machine_re_reporting_its_own_name() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "turing", &[]).unwrap();
        assert_eq!(
            d.get("aa").unwrap().shown(),
            "turing",
            "no rename, no override"
        );

        d.set_label("aa", "北京服务器").unwrap();
        assert_eq!(
            d.get("aa").unwrap().shown(),
            "北京服务器",
            "the rename shows"
        );
        assert_eq!(
            d.get("aa").unwrap().name,
            "turing",
            "…and the channel does not move"
        );

        // The sync that would eat it.
        d.upsert("aa", "turing", &["1.2.3.4:1".to_string()])
            .unwrap();
        assert_eq!(
            d.get("aa").unwrap().shown(),
            "北京服务器",
            "the machine re-reporting its own name must not undo a rename"
        );

        // Both names find it, because both are names somebody may know
        // it by.
        assert_eq!(
            d.by_name("北京服务器").map(|x| x.id),
            Some("aa".to_string())
        );
        assert_eq!(d.by_name("turing").map(|x| x.id), Some("aa".to_string()));

        // Taking it back.
        d.set_label("aa", "").unwrap();
        assert_eq!(
            d.get("aa").unwrap().shown(),
            "turing",
            "an emptied rename falls back"
        );
        assert_eq!(
            d.get("aa").unwrap().label,
            None,
            "…and reads as no override at all"
        );
    }

    /// A rename crosses to the other machines, like `pinned` does —
    /// a name that only showed on the screen that typed it would be a
    /// per-viewer nickname, which is a different feature.
    #[test]
    fn a_rename_crosses_to_the_other_machines() {
        let here = DeviceDoc::new(1).unwrap();
        here.upsert("aa", "turing", &[]).unwrap();
        here.set_label("aa", "北京服务器").unwrap();

        let there = DeviceDoc::new(2).unwrap();
        there.upsert("aa", "turing", &[]).unwrap();
        there
            .merge(&here.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(
            there.get("aa").unwrap().shown(),
            "北京服务器",
            "the rename travelled"
        );
    }

    /// Sorted by the name on the glass. A list ordered on a name nobody
    /// can see reads as unsorted, and renaming would move a row to a
    /// place with no rule behind it.
    #[test]
    fn the_list_is_ordered_by_the_name_people_read() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "alpha", &[]).unwrap();
        d.upsert("bb", "beta", &[]).unwrap();
        assert_eq!(
            d.all()
                .iter()
                .map(|x| x.shown().to_owned())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );
        // Renamed to sort first, and the list follows what is readable.
        d.set_label("bb", "aardvark").unwrap();
        assert_eq!(
            d.all()
                .iter()
                .map(|x| x.shown().to_owned())
                .collect::<Vec<_>>(),
            vec!["aardvark", "alpha"]
        );
    }

    /// Two replicas each store a roster version; after merging both are
    /// there. **Nothing about which one counts is decided by the merge** —
    /// that is `khor_net::trust::newest`'s, over all of them.
    #[test]
    fn two_rosters_written_on_two_replicas_both_survive_the_merge() {
        let a = DeviceDoc::new(1).unwrap();
        let b = DeviceDoc::new(2).unwrap();
        a.put_roster("00000000000000000002-aa", b"two").unwrap();
        b.put_roster("00000000000000000001-bb", b"one").unwrap();
        a.merge(&b.changes_since(&Default::default()).unwrap())
            .unwrap();
        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        for d in [&a, &b] {
            let mut got = d.rosters();
            got.sort();
            assert_eq!(
                got,
                vec![
                    ("00000000000000000001-bb".to_string(), b"one".to_vec()),
                    ("00000000000000000002-aa".to_string(), b"two".to_vec()),
                ]
            );
        }
        // The roster map is not a machine: the table still counts none.
        assert_eq!(a.items(), 0);
        assert!(a.all().is_empty());
        a.drop_rosters(&["00000000000000000001-bb".into()]).unwrap();
        assert_eq!(a.rosters().len(), 1);
    }

    /// A certificate rides the row, travels, and an upsert leaves it
    /// alone — pairing re-writes every peer's name and addresses.
    #[test]
    fn a_certificate_rides_the_row_and_an_upsert_does_not_touch_it() {
        let a = DeviceDoc::new(1).unwrap();
        let b = DeviceDoc::new(2).unwrap();
        a.upsert("cc", "turing", &[]).unwrap();
        a.set_cert("cc", &[1, 2, 3]).unwrap();
        let before = a.version();
        a.set_cert("cc", &[1, 2, 3]).unwrap();
        assert_eq!(a.version(), before, "unchanged means unwritten");
        b.merge(&a.changes_since(&Default::default()).unwrap())
            .unwrap();
        assert_eq!(b.cert_of("cc"), Some(vec![1, 2, 3]));
        b.upsert("cc", "turing", &["10.0.0.9:1".into()]).unwrap();
        assert_eq!(b.cert_of("cc"), Some(vec![1, 2, 3]));
        assert_eq!(b.cert_of("dd"), None);
        assert_eq!(b.items(), 1, "the certificate is a field, not a machine");
    }

    #[test]
    fn gone_rows_are_listed_for_the_holder_and_keep_their_names() {
        let d = DeviceDoc::new(1).unwrap();
        d.upsert("aa", "mac", &[]).unwrap();
        d.upsert("bb", "phone", &[]).unwrap();
        d.set_label("bb", "pocket").unwrap();
        d.set_gone("bb", true).unwrap();
        assert_eq!(d.gone_ids(), vec!["bb".to_string()]);
        assert_eq!(d.name_of("bb").as_deref(), Some("pocket"));
        assert!(d.get("bb").is_none(), "still hidden from the table");
    }
}
