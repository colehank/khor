//! Trust on this machine: which root it verifies with, which roster
//! decides who counts, and the verbs that change either (docs/CONNECT.md
//! 身份：用户根钥匙, 分期 2). The record formats and the pairing rule are
//! `khor_net::trust`'s; this is where they meet the store.
//!
//! # The files
//!
//! | file | what | who has it |
//! |---|---|---|
//! | `.khor/root.key` | the root secret, 0600 (`khor_net::root`) | the machine that holds the root |
//! | `.khor/root.pub` | **the anchor**: the root this machine verifies with | every machine in a network with a root |
//! | `.khor/root.unshown` | a root was born here and nobody has seen its words yet | the holder, until the words are shown |
//! | `.khor/pending/<id>` | a join this machine saw that no root holder has signed yet | whoever saw it |
//!
//! The roster itself is **not** a file: it rides the device table, in a
//! map beside the rows (`khor_sync::devices::TRUST`), every signed
//! version under its own key. A reader verifies each against the anchor
//! and takes the highest `seq` ([`newest_in`]); the table's
//! last-writer-wins never enters the question. The root secret never
//! enters any document, frame or log line.
//!
//! # Standing
//!
//! Every other machine — and this one — stands one of four ways
//! ([`Standing`]), and the gate every request but `Pair` passes
//! ([`Node::admit`]) answers only the first two:
//!
//! - **Legacy**: this machine has no anchor. A network from before roots
//!   keeps working the way it did — the device table is the trust —
//!   until one machine runs `khor root init` (or a pairing brings a root
//!   in), and every machine the resulting roster names takes it up on
//!   its next sync. A fresh machine is never here: it makes its own root
//!   at first start.
//! - **Trusted**: the newest roster lists it and has not revoked it —
//!   or, while the roster is fresh, it carries a certificate newer than
//!   the roster (CONNECT.md 新设备出示证书，全网立刻认).
//! - **Pending**: in the table, not in the roster. Waiting for a root
//!   holder to approve it.
//! - **Revoked**: the roster says so. Nothing it presents counts.
//!
//! # 撤销要新鲜
//!
//! A roster older than [`nt::FRESH_DAYS`] is stale: the machine goes on
//! trusting what that roster already listed, and **stops accepting
//! presented certificates** — the only way in that does not go through
//! a newer roster. `khor doctor` says how stale. The holder re-signs
//! every [`nt::REFRESH_DAYS`] ([`Node::trust_tick`]), so this only bites
//! while the holder has been away for a month.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use khor_catalog::msg;
use khor_net::trust::{self as nt, Id, Kind, Member, Roster, Settle};
use khor_sync::{devices::DeviceDoc, store::Doc};

use crate::Node;

const ROOT_KEY: &str = "root.key";
const ROOT_PUB: &str = "root.pub";
const UNSHOWN: &str = "root.unshown";
const PENDING_DIR: &str = "pending";

/// How another machine stands with this one (the module head).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Legacy,
    Trusted,
    Pending,
    Revoked,
}

impl Standing {
    /// The stable key a face keys off.
    pub fn key(self) -> &'static str {
        match self {
            Standing::Legacy => "legacy",
            Standing::Trusted => "trusted",
            Standing::Pending => "pending",
            Standing::Revoked => "revoked",
        }
    }

    /// Whether the gate lets it through.
    pub fn admitted(self) -> bool {
        matches!(self, Standing::Legacy | Standing::Trusted)
    }
}

/// One thing waiting for a root holder (`khor pending`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingRow {
    pub id: String,
    /// The name on the glass.
    pub name: String,
    /// `device` or `satellite` for a join; `leave` for a machine shown
    /// the door somewhere that has not been revoked yet.
    pub kind: String,
    /// When this machine first saw it waiting, when it did.
    pub since_ms: Option<u64>,
}

/// What `khor root show` and the app's first-start prompt read.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RootState {
    /// The root public key, hex — `None` in a network from before roots.
    pub root: Option<String>,
    /// Whether this machine holds the root secret.
    pub holds: bool,
    pub seq: Option<u64>,
    pub issued_ms: Option<u64>,
    pub age_days: Option<u64>,
    /// Days past [`nt::FRESH_DAYS`], when stale.
    pub stale_days: Option<u64>,
    /// This machine's own [`Standing`], by key.
    pub standing: String,
    /// A root was born here and its words have not been shown yet.
    pub unshown: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PendingNote {
    name: String,
    kind: String,
    sponsor: Option<String>,
    since_ms: u64,
}

/// A roster's key in the table: its `seq`, zero-padded so keys sort the
/// way seqs do, and the tail of its signature so two versions signed with
/// the same `seq` (two holders at once) do not overwrite each other.
fn roster_key(r: &Roster) -> String {
    let b = r.bytes();
    let tail: String = b[b.len() - 8..]
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    format!("{:020}-{tail}", r.seq)
}

fn key_seq(k: &str) -> Option<u64> {
    k.split('-').next()?.parse().ok()
}

/// Rosters this process has already verified, by anchor and bytes. The
/// gate asks on every request and the answer is the same bytes almost
/// every time; verifying is one signature per certificate.
static VERIFIED: Mutex<Vec<Verified>> = Mutex::new(Vec::new());

/// One cache entry: the anchor it verified under, the bytes, the result.
type Verified = (Id, Vec<u8>, Arc<Roster>);

fn verified(anchor: &iroh::PublicKey, bytes: &[u8]) -> Option<Arc<Roster>> {
    let a = *anchor.as_bytes();
    if let Some(r) = VERIFIED
        .lock()
        .expect("roster cache poisoned")
        .iter()
        .find(|(x, b, _)| *x == a && b.as_slice() == bytes)
        .map(|(_, _, r)| r.clone())
    {
        return Some(r);
    }
    let r = Arc::new(nt::verify_roster(bytes, anchor).ok()?);
    let mut c = VERIFIED.lock().expect("roster cache poisoned");
    if c.len() >= 32 {
        c.clear();
    }
    c.push((a, bytes.to_vec(), r.clone()));
    Some(r)
}

/// The newest roster in `doc` that verifies under `anchor` — the one
/// that counts. `khor_net::trust::newest`'s rule, through the cache.
pub(crate) fn newest_in(doc: &DeviceDoc, anchor: &iroh::PublicKey) -> Option<Arc<Roster>> {
    doc.rosters()
        .into_iter()
        .filter_map(|(_, b)| verified(anchor, &b))
        .max_by(|a, b| (a.seq, a.issued_ms, a.bytes()).cmp(&(b.seq, b.issued_ms, b.bytes())))
}

/// **The trust question, with the clock as an argument** — what the gate
/// asks and what the freshness rule is tested through.
///
/// `presented` is the certificate the machine's row carries: it counts
/// only when it is newer than the roster, the roster is fresh, and the
/// roster has not revoked the machine.
pub fn standing(
    roster: Option<&Roster>,
    anchor: Option<&iroh::PublicKey>,
    id: &Id,
    presented: Option<&[u8]>,
    now_ms: u64,
) -> Standing {
    let Some(anchor) = anchor else {
        return Standing::Legacy;
    };
    let Some(r) = roster else {
        return Standing::Pending;
    };
    if r.is_revoked(id) {
        return Standing::Revoked;
    }
    if r.admits(id, now_ms) {
        return Standing::Trusted;
    }
    if !r.is_stale(now_ms)
        && let Some(bytes) = presented
        && let Ok(c) = nt::verify_cert(bytes, anchor)
        && &c.subject == id
        && c.seq > r.seq
        && c.live_at(now_ms)
    {
        return Standing::Trusted;
    }
    Standing::Pending
}

fn id_of(hex: &str) -> Result<Id, String> {
    nt::id_from_hex(hex).ok_or_else(|| msg::not_a_machine_id(format_args!("{hex:?}")))
}

/// A verified, owned decision for the immediate settlement below. This is
/// not a durable intent or permission to replay against a later trust state.
struct SettlementPlan {
    anchor: Option<Id>,
    action: SettlementAction,
    roster: Option<Roster>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettlementAction {
    Keep,
    Adopt,
}

/// Verify before consulting or changing this machine's trust state. The
/// claimed root is only an input to the existing full signature verifier.
fn verified_settlement_roster(bytes: &[u8]) -> Result<Option<Roster>, String> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let root = nt::claimed_root(bytes).ok_or_else(|| nt::TrustError::Garbled.to_string())?;
    nt::verify_roster(bytes, &root)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Pure decision over one snapshot. `mine` is the roster selected under
/// `anchor`; `theirs` comes from verified_settlement_roster. The plan owns the
/// exact signed bytes/sequence and anchor identity, not a later roster lookup.
fn plan_settlement(
    anchor: Option<Id>,
    mine: Option<&Roster>,
    me: &Id,
    theirs: Option<Roster>,
    them: &str,
    i_dialed: bool,
) -> Result<SettlementPlan, String> {
    let action = match nt::settle(mine, me, theirs.as_ref(), &id_of(them)?, i_dialed) {
        Settle::Clash => return Err(msg::pair_two_roots().into()),
        Settle::Adopt => SettlementAction::Adopt,
        Settle::Keep => SettlementAction::Keep,
    };
    Ok(SettlementPlan {
        anchor,
        action,
        roster: theirs,
    })
}

impl Node {
    fn khor_file(&self, name: &str) -> PathBuf {
        self.root().join(".khor").join(name)
    }

    /// The root this machine verifies with, or `None` (Legacy).
    pub(crate) fn anchor(&self) -> Option<iroh::PublicKey> {
        anchor_at(self.root())
    }

    fn set_anchor(&self, root: &iroh::PublicKey) -> Result<(), String> {
        set_anchor_at(self.root(), root)
    }

    fn root_secret(&self) -> Result<Option<iroh::SecretKey>, String> {
        root_secret_at(self.root())
    }

    /// Whether this machine holds the network's root.
    pub fn holds_root(&self) -> bool {
        matches!(self.root_secret(), Ok(Some(_)))
    }

    fn my_id(&self) -> Id {
        self.device().0
    }

    /// The newest roster that counts here, or `None` (Legacy, or a store
    /// that cannot be read).
    pub fn roster(&self) -> Option<Roster> {
        let anchor = self.anchor()?;
        let loaded = self.devices_loaded().ok()?;
        newest_in(&loaded.doc, &anchor).map(|r| (*r).clone())
    }

    pub(crate) fn standing_in(&self, doc: &DeviceDoc, id: &str, now_ms: u64) -> Standing {
        let anchor = self.anchor();
        let Some(idb) = nt::id_from_hex(id) else {
            return if anchor.is_some() {
                Standing::Pending
            } else {
                Standing::Legacy
            };
        };
        let roster = anchor.as_ref().and_then(|a| newest_in(doc, a));
        standing(
            roster.as_deref(),
            anchor.as_ref(),
            &idb,
            doc.cert_of(id).as_deref(),
            now_ms,
        )
    }

    /// How `id` stands with this machine right now.
    pub fn standing_of(&self, id: &str) -> Standing {
        match self.devices_loaded() {
            Ok(l) => self.standing_in(&l.doc, id, crate::now_ms()),
            Err(_) => Standing::Pending,
        }
    }

    /// **The gate** every request but `Pair` passes (`link`'s dispatch):
    /// in the table, and in the roster. Two refusals, two words — a
    /// machine that never paired and one that paired and waits (or was
    /// revoked) have different next steps.
    pub(crate) fn admit(&self, remote: &str) -> Result<(), String> {
        let loaded = self.devices_loaded()?;
        if loaded.doc.get(remote).is_none() {
            return Err(msg::not_paired().into());
        }
        if self
            .standing_in(&loaded.doc, remote, crate::now_ms())
            .admitted()
        {
            Ok(())
        } else {
            Err(msg::not_in_roster().into())
        }
    }

    /// Stores a verified roster, and drops the versions it replaces.
    fn store_roster(&self, doc: &DeviceDoc, r: &Roster) -> Result<(), String> {
        store_roster_at(doc, r)
    }

    /// Signs the next roster with this machine's root and stores it. A
    /// holder with no roster yet (a recovered root, a table that lost it)
    /// starts from one that certifies itself.
    fn sign_next(
        &self,
        doc: &DeviceDoc,
        next: impl FnOnce(&Roster, &iroh::SecretKey, u64) -> Vec<u8>,
    ) -> Result<Roster, String> {
        let key = self
            .root_secret()?
            .ok_or_else(|| msg::root_not_here().to_owned())?;
        let now = crate::now_ms();
        let cur = match newest_in(doc, &key.public()) {
            Some(c) => (*c).clone(),
            None => nt::verify_roster(
                &nt::genesis(&key, now, &[Member::device(self.my_id())]),
                &key.public(),
            )
            .map_err(|e| e.to_string())?,
        };
        let r =
            nt::verify_roster(&next(&cur, &key, now), &key.public()).map_err(|e| e.to_string())?;
        self.store_roster(doc, &r)?;
        Ok(r)
    }

    /// Makes sure the holder is on its own roster: the first roster of a
    /// root (`seq` 1) when there is none yet, else the next one with this
    /// machine admitted. Nothing when it already is.
    fn certify_self(&self, doc: &DeviceDoc, key: &iroh::SecretKey) -> Result<(), String> {
        let now = crate::now_ms();
        match newest_in(doc, &key.public()) {
            Some(r) if r.admits(&self.my_id(), now) => Ok(()),
            Some(_) => self
                .certify(doc, self.device_str(), Kind::Device, None)
                .map(|_| ()),
            None => {
                let r = nt::verify_roster(
                    &nt::genesis(key, now, &[Member::device(self.my_id())]),
                    &key.public(),
                )
                .map_err(|e| e.to_string())?;
                self.store_roster(doc, &r)?;
                if let Some(c) = r.signed_cert(&self.my_id()) {
                    doc.set_cert(self.device_str(), c)?;
                }
                Ok(())
            }
        }
    }

    /// Signs `id` into the roster and puts its certificate on its row.
    fn certify(
        &self,
        doc: &DeviceDoc,
        id: &str,
        kind: Kind,
        sponsor: Option<&str>,
    ) -> Result<Roster, String> {
        let m = Member {
            subject: id_of(id)?,
            kind,
            sponsor: sponsor.and_then(nt::id_from_hex),
        };
        let r = self.sign_next(doc, |cur, key, now| cur.admit(key, now, &m))?;
        if let Some(c) = r.signed_cert(&m.subject) {
            doc.set_cert(id, c)?;
        }
        self.drop_pending_note(id);
        Ok(r)
    }

    // ── pending ─────────────────────────────────────────────────────

    fn pending_dir(&self) -> PathBuf {
        self.khor_file(PENDING_DIR)
    }

    /// Remembers that `id` waits for a root holder. The first sighting's
    /// time is kept; a second pairing of the same machine does not reset
    /// how long it has been waiting.
    fn note_pending(&self, id: &str, name: &str, kind: Kind, sponsor: Option<&str>) {
        if nt::id_from_hex(id).is_none() {
            return;
        }
        let path = self.pending_dir().join(id);
        if path.exists() {
            return;
        }
        let note = PendingNote {
            name: name.to_owned(),
            kind: kind.key().to_owned(),
            sponsor: sponsor.map(str::to_owned),
            since_ms: crate::now_ms(),
        };
        if std::fs::create_dir_all(self.pending_dir()).is_ok()
            && let Ok(bytes) = serde_json::to_vec(&note)
        {
            let _ = std::fs::write(path, bytes);
        }
    }

    fn drop_pending_note(&self, id: &str) {
        if nt::id_from_hex(id).is_some() {
            let _ = std::fs::remove_file(self.pending_dir().join(id));
        }
    }

    fn pending_notes(&self) -> Vec<(String, PendingNote)> {
        let Ok(rd) = std::fs::read_dir(self.pending_dir()) else {
            return Vec::new();
        };
        rd.flatten()
            .filter_map(|e| {
                let id = e.file_name().to_string_lossy().to_string();
                nt::id_from_hex(&id)?;
                let note: PendingNote =
                    serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?;
                Some((id, note))
            })
            .collect()
    }

    /// What waits for a root holder (`khor pending`), on any machine:
    ///
    /// - every row of the table the roster does not list (a machine that
    ///   paired through a machine without the root, a satellite adopted
    ///   by one) — including this machine, when it is the one waiting;
    /// - a revoked machine that has asked to come back (a pairing noted
    ///   it here);
    /// - a machine shown the door somewhere that the roster still lists
    ///   (`kind` = `leave`): the removal waits for the revocation.
    ///
    /// Empty in a network from before roots — there is nobody to ask.
    /// Notes for machines that have since been decided are cleared here.
    pub fn pending(&self) -> Result<Vec<PendingRow>, String> {
        let Some(anchor) = self.anchor() else {
            return Ok(Vec::new());
        };
        let loaded = self.devices_loaded()?;
        let doc = &loaded.doc;
        let now = crate::now_ms();
        let roster = newest_in(doc, &anchor);
        let notes = self.pending_notes();
        let mut out: Vec<PendingRow> = Vec::new();
        for d in doc.all() {
            let st = self.standing_in(doc, &d.id, now);
            let note = notes.iter().find(|(id, _)| id == &d.id).map(|(_, n)| n);
            let waiting = match st {
                Standing::Pending => true,
                Standing::Revoked => note.is_some(),
                Standing::Legacy | Standing::Trusted => false,
            };
            if !waiting {
                if note.is_some() && st == Standing::Trusted {
                    self.drop_pending_note(&d.id);
                }
                continue;
            }
            out.push(PendingRow {
                name: d.shown().to_owned(),
                kind: if d.sponsor.is_some() {
                    Kind::Satellite.key()
                } else {
                    Kind::Device.key()
                }
                .to_owned(),
                since_ms: note.map(|n| n.since_ms),
                id: d.id,
            });
        }
        if let Some(r) = &roster {
            for id in doc.gone_ids() {
                if let Some(idb) = nt::id_from_hex(&id)
                    && r.admits(&idb, now)
                {
                    out.push(PendingRow {
                        name: doc.name_of(&id).unwrap_or_else(|| id.clone()),
                        kind: "leave".to_owned(),
                        since_ms: None,
                        id,
                    });
                }
            }
        }
        Ok(out)
    }

    fn pending_named(&self, machine: &str) -> Result<PendingRow, String> {
        let typed = machine.trim();
        let rows = self.pending()?;
        rows.iter()
            .find(|r| r.id == typed)
            .or_else(|| rows.iter().find(|r| r.name == typed))
            .or_else(|| {
                let ch = khor_sync::chat::channel_of_machine(typed)?;
                let doc = self.devices_loaded().ok()?.doc;
                rows.iter()
                    .find(|r| doc.get(&r.id).is_some_and(|d| d.name == ch))
            })
            .cloned()
            .ok_or_else(|| msg::nothing_pending_named(typed))
    }

    /// Approves one waiting machine **on the machine that holds the
    /// root**: signs its certificate (a join) or its revocation (a
    /// removal), and answers its name and the new roster's `seq`. The
    /// roster reaches everyone else on the next sync — [`Node::approve`]
    /// pushes one right away.
    pub fn approve_here(&self, machine: &str) -> Result<(String, u64), String> {
        let _trust = self.root_mutation_guard()?;
        if !self.holds_root() {
            return Err(msg::approve_needs_root().into());
        }
        let row = self.pending_named(machine)?;
        let loaded = self.devices_loaded()?;
        let r = if row.kind == "leave" {
            let id = id_of(&row.id)?;
            self.sign_next(&loaded.doc, |cur, key, now| cur.revoke(key, now, &id))?
        } else {
            let sponsor = loaded.doc.get(&row.id).and_then(|d| d.sponsor);
            let kind = if sponsor.is_some() {
                Kind::Satellite
            } else {
                Kind::Device
            };
            self.certify(&loaded.doc, &row.id, kind, sponsor.as_deref())?
        };
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        self.drop_pending_note(&row.id);
        Ok((row.name, r.seq))
    }

    /// [`Node::approve_here`], then one sync so the roster leaves this
    /// machine now rather than on the next beat (best effort: a machine
    /// that cannot be reached gets it when it can).
    pub async fn approve(&self, machine: &str) -> Result<(String, u64), String> {
        let out = self.approve_here(machine)?;
        let _ = self.sync_now().await;
        Ok(out)
    }

    /// Revokes a machine **on the machine that holds the root**: the next
    /// roster drops its certificate and lists its id, and every machine
    /// that takes that roster refuses it. The row stays in the table —
    /// `forget` is what shows it the door.
    pub fn revoke_here(&self, machine: &str) -> Result<(String, u64), String> {
        let _trust = self.root_mutation_guard()?;
        if !self.holds_root() {
            return Err(msg::revoke_needs_root().into());
        }
        let loaded = self.devices_loaded()?;
        let typed = machine.trim();
        // By id first, including a machine already shown the door; then
        // by either name, the way every other verb resolves one.
        let (id, name) = if nt::id_from_hex(typed).is_some() && loaded.doc.name_of(typed).is_some()
        {
            (
                typed.to_owned(),
                loaded.doc.name_of(typed).unwrap_or_default(),
            )
        } else {
            let (_, dev) = self.resolve(typed)?;
            let id = dev.hex();
            let name = loaded.doc.name_of(&id).unwrap_or_else(|| typed.to_owned());
            (id, name)
        };
        if id == self.device_str() {
            return Err(msg::cant_revoke_holder().into());
        }
        let idb = id_of(&id)?;
        if newest_in(&loaded.doc, &self.anchor().expect("a holder has an anchor"))
            .is_some_and(|r| r.is_revoked(&idb))
        {
            return Err(msg::already_revoked(&name));
        }
        let r = self.sign_next(&loaded.doc, |cur, key, now| cur.revoke(key, now, &idb))?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        self.drop_pending_note(&id);
        Ok((name, r.seq))
    }

    /// [`Node::revoke_here`], then one sync — [`Node::approve`]'s reason.
    pub async fn revoke(&self, machine: &str) -> Result<(String, u64), String> {
        let out = self.revoke_here(machine)?;
        let _ = self.sync_now().await;
        Ok(out)
    }

    /// Revokes `id` if this machine holds the root and the roster lists
    /// it — what `forget` adds on the holder (`Node::forget_device`).
    pub(crate) fn revoke_if_holder(&self, doc: &DeviceDoc, id: &str) -> Result<(), String> {
        if !self.holds_root() {
            return Ok(());
        }
        let idb = id_of(id)?;
        let anchor = self.anchor().expect("a holder has an anchor");
        if newest_in(doc, &anchor).is_some_and(|r| r.cert(&idb).is_some() && !r.is_revoked(&idb)) {
            self.sign_next(doc, |cur, key, now| cur.revoke(key, now, &idb))?;
        }
        Ok(())
    }

    /// A satellite just written into the table (`adopt`): signed on the
    /// holder, noted as waiting anywhere else (CONNECT.md 卫星也进名单).
    /// The subject is its host-key fingerprint and the sponsor is this
    /// machine. Nothing happens in a network from before roots.
    pub(crate) fn enroll_satellite(
        &self,
        doc: &DeviceDoc,
        id: &str,
        name: &str,
    ) -> Result<(), String> {
        if self.anchor().is_none() {
            return Ok(());
        }
        if self.holds_root() {
            self.certify(doc, id, Kind::Satellite, Some(self.device_str()))?;
        } else {
            self.note_pending(id, name, Kind::Satellite, Some(self.device_str()));
        }
        Ok(())
    }

    // ── pairing ─────────────────────────────────────────────────────

    /// What this machine hands the other side of a pairing: its newest
    /// roster, or nothing in a network from before roots.
    #[cfg(test)]
    pub(crate) fn roster_offer(&self) -> Vec<u8> {
        self.roster()
            .map(|r| r.bytes().to_vec())
            .unwrap_or_default()
    }

    /// [`Node::roster_offer`] from a table already in hand — the issuer
    /// answers with what it just wrote, before the next load would see it.
    #[cfg(test)]
    pub(crate) fn roster_offer_in(&self, doc: &DeviceDoc) -> Vec<u8> {
        self.anchor()
            .and_then(|a| newest_in(doc, &a))
            .map(|r| r.bytes().to_vec())
            .unwrap_or_default()
    }

    /// Meets the other side's roster (`khor_net::trust::settle`): refuses
    /// two networks, gives way, or keeps — and keeps their roster too when
    /// it is under the same root, so the newer of the two counts on both
    /// sides. Runs **before** anything else of the pairing is written, so
    /// a refusal leaves both tables as they were.
    #[cfg(test)]
    pub(crate) fn settle_with(
        &self,
        doc: &DeviceDoc,
        theirs: &[u8],
        them: &str,
        i_dialed: bool,
    ) -> Result<(), String> {
        let theirs = verified_settlement_roster(theirs)?;
        let anchor = self.anchor();
        let mine = anchor.as_ref().and_then(|a| newest_in(doc, a));
        let plan = plan_settlement(
            anchor.as_ref().map(|a| *a.as_bytes()),
            mine.as_deref(),
            &self.my_id(),
            theirs,
            them,
            i_dialed,
        )?;
        self.apply_settlement(doc, plan)
    }

    /// Consume an immediate plan while the caller still owns the same Loaded.
    /// The existing mutation order and key safety check are preserved. Durable
    /// replay, a shared trust fence and stale-plan checks belong to the intent
    /// slice; this function alone provides none of those guarantees.
    #[cfg(test)]
    fn apply_settlement(&self, doc: &DeviceDoc, plan: SettlementPlan) -> Result<(), String> {
        match plan.action {
            SettlementAction::Adopt => {
                self.give_way_to(doc, &plan.roster.expect("adopting needs theirs"))
            }
            SettlementAction::Keep => {
                if let (Some(anchor), Some(roster)) = (plan.anchor, plan.roster)
                    && roster.root == anchor
                {
                    self.store_roster(doc, &roster)?;
                }
                Ok(())
            }
        }
    }

    /// Takes the other side's root. This machine's own — if it had one —
    /// certified nobody but itself (that is when [`nt::settle`] answers
    /// `Adopt`), so it goes: key file, words prompt and its rosters.
    #[cfg(test)]
    fn give_way_to(&self, doc: &DeviceDoc, theirs: &Roster) -> Result<(), String> {
        let key_path = self.khor_file(ROOT_KEY);
        if let Some(key) = khor_net::root::load(&key_path).map_err(|e| e.to_string())?
            && key.public().as_bytes() != &theirs.root
        {
            if newest_in(doc, &key.public()).is_some_and(|r| !r.lone(&self.my_id())) {
                // Guarded twice on purpose: a root that certified other
                // machines is a network, and deleting it here would strand
                // every one of them.
                return Err(msg::pair_two_roots().into());
            }
            std::fs::remove_file(&key_path).map_err(|e| msg::cant_write(key_path.display(), e))?;
            let _ = std::fs::remove_file(self.khor_file(UNSHOWN));
        }
        let every: Vec<String> = doc.rosters().into_iter().map(|(k, _)| k).collect();
        doc.drop_rosters(&every)?;
        self.set_anchor(&theirs.root_key())?;
        self.store_roster(doc, theirs)
    }

    /// After a pairing has settled whose root it is: the holder signs
    /// whichever of the two parties the roster does not list yet;
    /// anywhere else they are noted as waiting — on both sides, so either
    /// machine's `khor pending` shows them. Answers whether anything was
    /// signed (the dialing side then pushes it back over the same
    /// connection).
    #[cfg(test)]
    pub(crate) fn enroll_after_pair(
        &self,
        doc: &DeviceDoc,
        parties: &[(&str, &str)],
    ) -> Result<bool, String> {
        if self.anchor().is_none() {
            return Ok(false);
        }
        let holds = self.holds_root();
        let now = crate::now_ms();
        let mut signed = false;
        for (id, name) in parties {
            if self.standing_in(doc, id, now) == Standing::Trusted {
                // The row carries the roster's certificate, not one a
                // merge left there from a root this machine gave up.
                if let (Some(a), Some(idb)) = (self.anchor(), nt::id_from_hex(id))
                    && let Some(c) =
                        newest_in(doc, &a).and_then(|r| r.signed_cert(&idb).map(<[u8]>::to_vec))
                {
                    doc.set_cert(id, &c)?;
                }
                continue;
            }
            if holds {
                self.certify(doc, id, Kind::Device, None)?;
                signed = true;
            } else {
                self.note_pending(id, name, Kind::Device, None);
            }
        }
        Ok(signed)
    }

    // ── a table that lost its rows ──────────────────────────────────

    /// What this machine sends a machine that refused its round
    /// ([`crate::proto::Request::Vouch`]): its own row and its newest
    /// roster. `None` where there is nothing to vouch with — a network
    /// from before roots, or a roster that does not admit this machine.
    pub(crate) fn vouch_offer(&self) -> Option<(String, Vec<String>, Vec<u8>)> {
        let anchor = self.anchor()?;
        let loaded = self.devices_loaded().ok()?;
        let r = newest_in(&loaded.doc, &anchor)?;
        if !r.admits(&self.my_id(), crate::now_ms()) {
            return None;
        }
        let addrs = loaded
            .doc
            .get(self.device_str())
            .map(|d| d.addrs)
            .unwrap_or_default();
        Some((self.name().to_owned(), addrs, r.bytes().to_vec()))
    }

    /// **Takes a machine back on the root's word** (round NET-10).
    ///
    /// A table that lost its rows cannot heal by itself: the gate refuses
    /// every machine it no longer lists, and the pump visits only the
    /// machines it lists — so nobody can bring the rows back. Measured on
    /// the owner's hinton, 2026-10-05: its device table held its own row
    /// and nothing else (no roster, no certificate) for a day and a half,
    /// while every other machine still listed it as paired; each of their
    /// rounds was refused with 「not in the machine table」.
    ///
    /// The roster is what decides trust (docs/CONNECT.md 名单是信任的唯一
    /// 出处), and the anchor (`root.pub`) is a file of its own, beside
    /// the table, so it survives the table. A roster that verifies under
    /// it and names both machines is the same admission a round with a
    /// machine on the roster already grants — this only writes the row a
    /// round needs to start. Refused when:
    ///
    /// - there is no anchor (a network from before roots, or a machine
    ///   that left: `leave_trust` removes it);
    /// - the roster is not signed by the anchor's root, does not admit
    ///   both machines, or is stale (the freshness rule presented
    ///   certificates keep, `standing`);
    /// - it is older than the roster this table holds — an old roster
    ///   must not undo a revocation;
    /// - this table shows the asker the door (`set_gone`): a person
    ///   decided that here.
    #[cfg(test)]
    pub(crate) fn take_vouch(
        &self,
        remote: &str,
        name: &str,
        addrs: &[String],
        roster: &[u8],
    ) -> Result<(), String> {
        PairTrust::new(
            crate::doc_transfer_state::Context {
                root: self.root().clone(),
                writer: self.writer_peer(),
                local: self.my_id(),
            },
            self.name().into(),
        )
        .take_vouch(remote, name, addrs, roster)
    }

    // ── keeping it alive ────────────────────────────────────────────

    /// Called once when a node opens. Answers whether a root was born
    /// **now** — the moment the CLI tells a person to write the words
    /// down.
    ///
    /// - An anchor already: nothing (one file read — every open pays it).
    /// - A key without an anchor: the key says whose network this is.
    /// - No key, and the table holds others: a network from before roots
    ///   — take up a roster that names this machine, if one has arrived.
    /// - No key and alone: **this is the first machine**. It makes the
    ///   root, certifies itself, and marks the words unshown.
    pub(crate) fn bootstrap_trust(
        &self,
        _recovered: &crate::doc_transfer_state::RootMutationGuard,
    ) -> Result<bool, String> {
        if self.anchor().is_some() {
            // **A holder is always on its own roster** — and after `khor id
            // --reset` its device id is a new one. Only a machine with a
            // key file pays for the look (every other open is one read).
            if self.khor_file(ROOT_KEY).exists()
                && let Some(key) = self.root_secret()?
            {
                let loaded = self.devices_loaded()?;
                if !newest_in(&loaded.doc, &key.public())
                    .is_some_and(|r| r.admits(&self.my_id(), crate::now_ms()))
                {
                    self.certify_self(&loaded.doc, &key)?;
                    let mut store = loaded.store;
                    store.flush(&loaded.doc)?;
                }
            }
            return Ok(false);
        }
        let key_path = self.khor_file(ROOT_KEY);
        let loaded = self.devices_loaded()?;
        let mut born = false;
        let key = match khor_net::root::load(&key_path).map_err(|e| e.to_string())? {
            Some(k) => k,
            None => {
                let alone = loaded.doc.all().iter().all(|d| d.id == self.device_str());
                if !alone {
                    self.adopt_from_table(&loaded.doc)?;
                    return Ok(false);
                }
                born = khor_net::root::create(&key_path, &khor_net::root::generate())
                    .map_err(|e| e.to_string())?;
                khor_net::root::load(&key_path)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| msg::root_key_corrupt(key_path.display()))?
            }
        };
        self.set_anchor(&key.public())?;
        self.certify_self(&loaded.doc, &key)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        if born {
            let _ = std::fs::write(self.khor_file(UNSHOWN), b"");
        }
        Ok(born)
    }

    /// A network from before roots, after `khor root init` ran somewhere:
    /// take up the root of a roster that names this machine. Answers
    /// whether it did.
    fn adopt_from_table(&self, doc: &DeviceDoc) -> Result<bool, String> {
        let now = crate::now_ms();
        let me = self.my_id();
        let best = doc
            .rosters()
            .into_iter()
            .filter_map(|(_, b)| verified(&nt::claimed_root(&b)?, &b))
            .filter(|r| r.admits(&me, now))
            .max_by_key(|r| r.seq);
        match best {
            Some(r) => {
                self.set_anchor(&r.root_key())?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// The trust beat, run when a serve starts and after every sync pump:
    /// a network from before roots looks for a roster naming this machine;
    /// the holder signs the list again when it is getting old
    /// ([`nt::REFRESH_DAYS`]). Best effort — a failure here leaves the
    /// roster one beat older, and the next beat tries again.
    pub(crate) fn trust_tick(&self) -> Result<(), String> {
        let _trust = self.root_mutation_guard()?;
        let loaded = self.devices_loaded()?;
        if self.anchor().is_none() {
            if khor_net::root::load(&self.khor_file(ROOT_KEY))
                .map_err(|e| e.to_string())?
                .is_none()
            {
                self.adopt_from_table(&loaded.doc)?;
            }
            return Ok(());
        }
        let Some(key) = self.root_secret()? else {
            return Ok(());
        };
        let now = crate::now_ms();
        if newest_in(&loaded.doc, &key.public()).is_some_and(|r| r.needs_refresh(now)) {
            self.sign_next(&loaded.doc, |cur, key, now| cur.refresh(key, now))?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        Ok(())
    }

    /// Whether this machine itself is in the roster (or in a network from
    /// before roots). A machine that is not expects every peer to refuse
    /// it, which is not deafness (`link`'s serve beat).
    pub(crate) fn self_admitted(&self) -> bool {
        self.standing_of(self.device_str()).admitted()
    }

    // ── the root verbs ──────────────────────────────────────────────

    /// What `khor root show` prints and the app's prompt reads.
    pub fn root_state(&self) -> RootState {
        let now = crate::now_ms();
        let roster = self.roster();
        RootState {
            root: self.anchor().map(|a| a.to_string()),
            holds: self.holds_root(),
            seq: roster.as_ref().map(|r| r.seq),
            issued_ms: roster.as_ref().map(|r| r.issued_ms),
            age_days: roster.as_ref().map(|r| r.age_days(now)),
            stale_days: roster.as_ref().and_then(|r| r.stale_days(now)),
            standing: self.standing_of(self.device_str()).key().to_owned(),
            unshown: self.khor_file(UNSHOWN).exists() && self.holds_root(),
        }
    }

    /// The root as its 24 words — **only on the machine that holds it,
    /// and only after a person asked** (the CLI makes them type this
    /// machine's name; the app makes them press a button). Showing them
    /// is what the unshown mark was waiting for, so it goes.
    pub fn root_words(&self) -> Result<String, String> {
        let key = self
            .root_secret()?
            .ok_or_else(|| msg::root_not_here().to_owned())?;
        let _ = std::fs::remove_file(self.khor_file(UNSHOWN));
        Ok(khor_net::root::to_words(&key))
    }

    /// [`Node::root_words`] behind the CLI's confirmation: **this
    /// machine's own name, typed back** — `Node::leave`'s rule, checked
    /// here rather than at the verb so there is one rule. Any other
    /// machine's name, or none, is refused with the sentence that says
    /// what to type.
    pub fn root_words_confirmed(&self, typed: &str) -> Result<String, String> {
        let shown = self.shown_name();
        if typed.trim().is_empty() {
            return Err(msg::words_need_confirm(&shown));
        }
        match self.resolve(typed) {
            Ok((_, dev)) if dev == self.device() => self.root_words(),
            _ => Err(msg::words_names_another(typed.trim(), &shown)),
        }
    }

    /// The app showed the words and the person said they wrote them
    /// down, or closed the prompt: either way it is not asked again.
    pub fn root_words_seen(&self) {
        let _ = std::fs::remove_file(self.khor_file(UNSHOWN));
    }

    /// Restores the root from its words, making this machine a holder.
    ///
    /// - The same root this machine already trusts: it becomes a holder.
    /// - No root yet, or a root that certified nobody but this machine: the
    ///   words' root replaces it.
    /// - A different network (a root that certified others): refused —
    ///   leave that network first; restoring would strand it.
    ///
    /// The holder then makes sure it is itself on the roster.
    pub fn root_recover(&self, words: &str) -> Result<String, String> {
        let _trust = self.root_mutation_guard()?;
        let key = khor_net::root::from_words(words).map_err(|e| e.to_string())?;
        let loaded = self.devices_loaded()?;
        let me = self.my_id();
        if let Some(a) = self.anchor()
            && a != key.public()
        {
            if newest_in(&loaded.doc, &a).is_some_and(|r| !r.lone(&me)) {
                return Err(msg::recover_other_network(a));
            }
            let every: Vec<String> = loaded.doc.rosters().into_iter().map(|(k, _)| k).collect();
            loaded.doc.drop_rosters(&every)?;
        }
        khor_net::root::save(&self.khor_file(ROOT_KEY), &key).map_err(|e| e.to_string())?;
        self.set_anchor(&key.public())?;
        let _ = std::fs::remove_file(self.khor_file(UNSHOWN));
        self.certify_self(&loaded.doc, &key)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(key.public().to_string())
    }

    /// Gives a network from before roots its root, here: every machine in
    /// the table is certified in one roster — satellites as satellites of
    /// their sponsors — and each takes the root up on its next sync.
    /// Refused where there already is a root. Answers the root and how
    /// many it signed.
    pub fn root_init(&self) -> Result<(String, usize), String> {
        let _trust = self.root_mutation_guard()?;
        if let Some(a) = self.anchor() {
            return Err(msg::root_already(a));
        }
        let loaded = self.devices_loaded()?;
        let mut members = Vec::new();
        for d in loaded.doc.all() {
            let Some(subject) = nt::id_from_hex(&d.id) else {
                continue;
            };
            members.push(match d.sponsor.as_deref().and_then(nt::id_from_hex) {
                Some(sp) => Member {
                    subject,
                    kind: Kind::Satellite,
                    sponsor: Some(sp),
                },
                None => Member::device(subject),
            });
        }
        if !members.iter().any(|m| m.subject == self.my_id()) {
            members.push(Member::device(self.my_id()));
        }
        let key_path = self.khor_file(ROOT_KEY);
        if !khor_net::root::create(&key_path, &khor_net::root::generate())
            .map_err(|e| e.to_string())?
        {
            return Err(msg::root_already(format_args!("{}", key_path.display())));
        }
        let key = khor_net::root::load(&key_path)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| msg::root_key_corrupt(key_path.display()))?;
        self.set_anchor(&key.public())?;
        let r = nt::verify_roster(&nt::genesis(&key, crate::now_ms(), &members), &key.public())
            .map_err(|e| e.to_string())?;
        self.store_roster(&loaded.doc, &r)?;
        for m in &members {
            if let Some(c) = r.signed_cert(&m.subject) {
                loaded.doc.set_cert(&nt::id_hex(&m.subject), c)?;
            }
        }
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        let _ = std::fs::write(self.khor_file(UNSHOWN), b"");
        Ok((key.public().to_string(), members.len()))
    }

    /// Forgets the network's trust on a machine leaving it (`Node::leave`)
    /// — unless it holds the root: the key is the network's, not this
    /// machine's, and throwing it away would leave everybody else without
    /// one. A machine without it is in no network afterwards, and makes a
    /// root of its own the next time it opens alone.
    pub(crate) fn leave_trust(&self, doc: &DeviceDoc) -> Result<(), String> {
        if self.holds_root() {
            return Ok(());
        }
        let every: Vec<String> = doc.rosters().into_iter().map(|(k, _)| k).collect();
        doc.drop_rosters(&every)?;
        let _ = std::fs::remove_file(self.khor_file(ROOT_PUB));
        let _ = std::fs::remove_dir_all(self.pending_dir());
        Ok(())
    }
}

impl Node {
    pub(crate) fn root_mutation_guard(
        &self,
    ) -> Result<crate::doc_transfer_state::RootMutationGuard, String> {
        root_mutation_at(self.root(), self.my_id())
    }

    // Keep this lease through self-registration and bootstrap. A later ordinary
    // Doc journal publication must not cause a second recovery decision after
    // the critical intents have been reconciled under this same root fence.
    pub(crate) fn recover_pairing_before_bootstrap(
        &self,
        _guard: &crate::doc_transfer_state::RootMutationGuard,
    ) -> Result<(), String> {
        use crate::doc_transfer_state::{Context, State, pairing};
        if pairing::needs_recovery(self.root(), self.my_id()).map_err(String::from)? {
            let context = Context {
                root: self.root().clone(),
                writer: self.writer_peer(),
                local: self.my_id(),
            };
            State::open(&context)
                .map_err(String::from)?
                .recover_pairing(
                    &PairTrust::new(context.clone(), self.name().into()),
                    &context,
                )
                .map_err(String::from)?;
        }
        Ok(())
    }
}

pub(crate) enum PairSettlementSide {
    Issuer,
    Joiner,
}

// Local planning input; the persisted Settlement keeps its existing layout.
pub(crate) struct PairSettlementRequest {
    pub operation: Id,
    pub remote: Id,
    pub side: PairSettlementSide,
    pub remote_name: String,
    pub remote_addrs: Vec<String>,
    pub input_roster: Option<crate::proto::DocManifest>,
}

pub(crate) struct PairTrust {
    context: crate::doc_transfer_state::Context,
    name: String,
    local_hex: String,
}
impl PairTrust {
    pub(crate) fn new(context: crate::doc_transfer_state::Context, name: String) -> Self {
        let local_hex = nt::id_hex(&context.local);
        Self {
            context,
            name,
            local_hex,
        }
    }
    fn root(&self) -> &PathBuf {
        &self.context.root
    }
    fn my_id(&self) -> Id {
        self.context.local
    }
    fn device_str(&self) -> &str {
        &self.local_hex
    }
    fn khor_file(&self, name: &str) -> PathBuf {
        self.root().join(".khor").join(name)
    }
    fn devices_loaded(&self) -> Result<khor_sync::store::Loaded<DeviceDoc>, String> {
        khor_sync::store::load_pile(
            &khor_sync::devices::devices_dir(self.root()),
            self.context.writer,
        )
    }
    fn anchor(&self) -> Option<iroh::PublicKey> {
        anchor_at(self.root())
    }
    fn set_anchor(&self, root: &iroh::PublicKey) -> Result<(), String> {
        set_anchor_at(self.root(), root)
    }
    fn root_secret(&self) -> Result<Option<iroh::SecretKey>, String> {
        root_secret_at(self.root())
    }
    fn standing_in(&self, doc: &DeviceDoc, id: &str, now: u64) -> Standing {
        let Some(id) = nt::id_from_hex(id) else {
            return Standing::Pending;
        };
        let anchor = self.anchor();
        let roster = anchor.as_ref().and_then(|a| newest_in(doc, a));
        standing(
            roster.as_deref(),
            anchor.as_ref(),
            &id,
            doc.cert_of(&nt::id_hex(&id)).as_deref(),
            now,
        )
    }
    fn store_roster(&self, doc: &DeviceDoc, roster: &Roster) -> Result<(), String> {
        store_roster_at(doc, roster)
    }
    fn root_mutation_guard(&self) -> Result<crate::doc_transfer_state::RootMutationGuard, String> {
        root_mutation_at(self.root(), self.my_id())
    }
    fn pair_invite_hash_path(&self, hash: Id) -> PathBuf {
        self.root().join(".khor/invites").join(nt::id_hex(&hash))
    }
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
    pub(crate) fn anchor_id(&self) -> Option<Id> {
        self.anchor().map(|key| *key.as_bytes())
    }
    pub(crate) fn roster_offer(&self) -> Result<Vec<u8>, String> {
        let loaded = self.devices_loaded()?;
        Ok(self
            .anchor()
            .and_then(|a| newest_in(&loaded.doc, &a))
            .map(|r| r.bytes().to_vec())
            .unwrap_or_default())
    }
    pub(crate) fn collect_pairing(
        &self,
        context: &crate::doc_transfer_state::Context,
        state: &mut crate::doc_transfer_state::State,
    ) -> Result<(), crate::doc_transfer_state::Error> {
        let now = crate::now_ms();
        let retired = state.expired_pair_ids(now);
        if !retired.is_empty() {
            let _guard = crate::doc_transfer_state::pairing::root_fence(&context.root)?;
            state.recover_pairing(self, context)?;
            let mut loaded = self.devices_loaded()?;
            let keys: Vec<_> = retired
                .into_iter()
                .map(|id| pair_archive(id, self.my_id()))
                .collect();
            loaded.doc.drop_rosters(&keys)?;
            loaded.store.flush(&loaded.doc)?;
        }
        state.collect_pairing(now)
    }
    pub(crate) fn fixed_roster_bytes(
        &self,
        manifest: &crate::proto::DocManifest,
    ) -> Result<Vec<u8>, String> {
        let loaded = self.devices_loaded()?;
        loaded
            .doc
            .rosters()
            .into_iter()
            .map(|(_, bytes)| bytes)
            .find(|bytes| {
                bytes.len() as u64 == manifest.total_bytes
                    && blake3::hash(bytes).as_bytes() == &manifest.digest
            })
            .ok_or_else(|| "fixed roster preparation needs reconciliation".into())
    }
    pub(crate) fn release_local_settlement_output(
        &self,
        state: &mut crate::doc_transfer_state::State,
        id: Id,
    ) -> Result<(), crate::doc_transfer_state::Error> {
        let plan = state.pair_settlement(id).ok_or("missing settlement")?;
        if !plan.finished || !plan.dialed {
            return Err("uncommitted local settlement output".into());
        }
        if let Some(manifest) = &plan.desired_roster {
            let loaded = self.devices_loaded()?;
            if !loaded.doc.rosters().iter().any(|(_, raw)| {
                raw.len() as u64 == manifest.total_bytes
                    && blake3::hash(raw).as_bytes() == &manifest.digest
            }) {
                return Err("local settlement roster is not present on disk".into());
            }
            drop(loaded);
            state.release_roster_blob(manifest)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn take_vouch(
        &self,
        remote: &str,
        name: &str,
        addrs: &[String],
        roster: &[u8],
    ) -> Result<(), String> {
        self.take_vouch_inner(remote, name, addrs, roster, None)
            .map_err(String::from)
    }
    pub(crate) fn apply_vouch(
        &self,
        state: &mut crate::doc_transfer_state::State,
        row: &mut crate::doc_transfer_state::pairing::Vouch,
    ) -> Result<(), crate::doc_transfer_state::Error> {
        let bytes = state.load_roster_blob(&row.manifest)?;
        let remote = nt::id_hex(&row.remote);
        let name = row.name.clone();
        let addrs = row.addrs.clone();
        self.take_vouch_inner(&remote, &name, &addrs, &bytes, Some((state, row)))
    }
    fn take_vouch_inner(
        &self,
        remote: &str,
        name: &str,
        addrs: &[String],
        roster: &[u8],
        mut pending: Option<(
            &mut crate::doc_transfer_state::State,
            &mut crate::doc_transfer_state::pairing::Vouch,
        )>,
    ) -> Result<(), crate::doc_transfer_state::Error> {
        use crate::doc_transfer_state::Error;
        use khor_sync::doc_transfer::DocumentVersion;
        let _trust = self.root_mutation_guard()?;
        let refused = || msg::vouch_refused().to_owned();
        let anchor = self.anchor().ok_or_else(refused)?;
        let r = nt::verify_roster(roster, &anchor).map_err(|_| refused())?;
        let them = id_of(remote)?;
        let now = crate::now_ms();
        if r.is_stale(now) || !r.admits(&them, now) || !r.admits(&self.my_id(), now) {
            return Err(refused().into());
        }
        let mut loaded = self.devices_loaded()?;
        if newest_in(&loaded.doc, &anchor).is_some_and(|mine| mine.seq > r.seq)
            || loaded.doc.gone_ids().iter().any(|g| g == remote)
        {
            return Err(refused().into());
        }
        let target = pending
            .as_ref()
            .and_then(|(_, row)| row.target.as_ref())
            .map(|v| DocumentVersion::decode(v).map_err(Error::from_std))
            .transpose()?;
        if let Some(target) = &target {
            if target.epoch != loaded.store.epoch() {
                return Err("Vouch document epoch changed".into());
            }
            if loaded.doc.version().includes_vv(&target.version) {
                return Ok(());
            }
        }
        self.store_roster(&loaded.doc, &r)?;
        loaded.doc.upsert(remote, name, addrs)?;
        for id in [remote, self.device_str()] {
            if let Some(c) = nt::id_from_hex(id).and_then(|b| r.signed_cert(&b).map(<[u8]>::to_vec))
            {
                loaded.doc.set_cert(id, &c)?;
            }
        }
        if let Some((state, row)) = pending.as_mut() {
            row.target = Some(
                DocumentVersion {
                    epoch: loaded.store.epoch(),
                    version: loaded.doc.version(),
                }
                .encode()
                .map_err(Error::from_std)?
                .into(),
            );
            state.save_vouch((**row).clone())?;
        }
        loaded.store.flush(&loaded.doc)?;
        Ok(())
    }

    /// The caller owns the trust fence before loading this snapshot. No user
    /// trust mutation occurs until the complete desired signed bytes are staged
    /// and the Ready intent has reached the shared journal.
    pub(crate) fn prepare_pair_settlement(
        &self,
        context: &crate::doc_transfer_state::Context,
        state: &mut crate::doc_transfer_state::State,
        request: PairSettlementRequest,
    ) -> Result<crate::doc_transfer_state::pairing::Settlement, crate::doc_transfer_state::Error>
    {
        use crate::doc_transfer_state::{
            Error,
            pairing::{self, Settlement},
        };
        let PairSettlementRequest {
            operation: id,
            remote,
            side,
            remote_name: name,
            remote_addrs: addrs,
            input_roster: input,
        } = request;
        let dialed = matches!(side, PairSettlementSide::Joiner);
        if let Some(mut saved) = state.pair_settlement(id) {
            if saved.ready {
                return Ok(saved);
            }
            if let Some(manifest) = &saved.desired_roster {
                if matches!(
                    state.roster_progress(manifest)?,
                    Some(khor_sync::transfer::Progress::VerifiedComplete { .. })
                ) {
                    saved.ready = true;
                    state.save_settlement(saved.clone())?;
                    return Ok(saved);
                }
                // An unpublished Preparing body may be replaced. Errors and
                // unknown stages remain errors; only this exact owned id goes.
                state.release_roster_blob(manifest)?;
            }
            state.remove_unready_settlement(id)?;
        }
        let raw = input
            .as_ref()
            .map(|m| state.load_roster_blob(m))
            .transpose()?
            .unwrap_or_default();
        let theirs = verified_settlement_roster(&raw)?;
        let loaded = self.devices_loaded()?;
        let anchor = self.anchor();
        let mine = anchor.as_ref().and_then(|a| newest_in(&loaded.doc, a));
        let decision = plan_settlement(
            anchor.as_ref().map(|a| *a.as_bytes()),
            mine.as_deref(),
            &self.my_id(),
            theirs.clone(),
            &nt::id_hex(&remote),
            dialed,
        )?;
        let desired_anchor = match decision.action {
            SettlementAction::Adopt => theirs.as_ref().map(|r| r.root),
            SettlementAction::Keep => decision.anchor,
        };
        let mut desired = match decision.action {
            SettlementAction::Adopt => theirs,
            SettlementAction::Keep => {
                let offered = theirs.filter(|r| Some(r.root) == desired_anchor);
                match (mine.as_deref(), offered) {
                    (Some(a), Some(b))
                        if (a.seq, a.issued_ms, a.bytes()) > (b.seq, b.issued_ms, b.bytes()) =>
                    {
                        Some(a.clone())
                    }
                    (_, Some(b)) => Some(b),
                    (Some(a), None) => Some(a.clone()),
                    _ => None,
                }
            }
        };
        if let Some(key) = self.root_secret()?
            && Some(*key.public().as_bytes()) == desired_anchor
        {
            let now = crate::now_ms();
            let mut roster = match desired {
                Some(roster) => roster,
                None => nt::verify_roster(
                    &nt::genesis(&key, now, &[Member::device(self.my_id())]),
                    &key.public(),
                )
                .map_err(Error::from_std)?,
            };
            for party in [remote, self.my_id()] {
                let cert = loaded.doc.cert_of(&nt::id_hex(&party));
                if standing(
                    Some(&roster),
                    Some(&key.public()),
                    &party,
                    cert.as_deref(),
                    now,
                ) != Standing::Trusted
                {
                    roster = nt::verify_roster(
                        &roster.admit(&key, now, &Member::device(party)),
                        &key.public(),
                    )
                    .map_err(Error::from_std)?;
                }
            }
            desired = Some(roster);
        }
        if let Some(key) = khor_net::root::load(&self.khor_file(ROOT_KEY))
            .map_err(|e| Error::from(e.to_string()))?
            && Some(*key.public().as_bytes()) != desired_anchor
            && newest_in(&loaded.doc, &key.public()).is_some_and(|r| !r.lone(&self.my_id()))
        {
            return Err(msg::pair_two_roots().into());
        }
        let bytes = desired.as_ref().map_or(&[][..], |r| r.bytes());
        let manifest = if bytes.is_empty() {
            None
        } else {
            Some(pairing::roster_manifest(
                id,
                if dialed {
                    pairing::LOCAL_SETTLEMENT
                } else {
                    pairing::ISSUER_ROSTER
                },
                context.local,
                remote,
                bytes,
            )?)
        };
        let settlement = Settlement {
            id,
            remote,
            dialed,
            before_anchor: anchor.map(|a| *a.as_bytes()),
            before_roster: mine.as_ref().map(|r| *blake3::hash(r.bytes()).as_bytes()),
            before_key: khor_net::root::load(&self.khor_file(ROOT_KEY))
                .map_err(|e| Error::from(e.to_string()))?
                .map(|k| *k.public().as_bytes()),
            desired_anchor,
            desired_roster: manifest.clone(),
            epoch: loaded.store.epoch(),
            target: None,
            ready: false,
            finished: false,
            remote_name: name,
            remote_addrs: addrs,
            row_before: loaded
                .doc
                .get(&nt::id_hex(&remote))
                .map(|r| (r.name, r.addrs)),
            input_roster: input,
        };
        drop(loaded);
        state.save_settlement(settlement.clone())?;
        if let Some(manifest) = &manifest {
            state.prepare_roster_blob(manifest, bytes)?;
        }
        let mut settlement = settlement;
        settlement.ready = true;
        state.save_settlement(settlement.clone())?;
        Ok(settlement)
    }

    pub(crate) fn apply_pair_settlement(
        &self,
        context: &crate::doc_transfer_state::Context,
        state: &mut crate::doc_transfer_state::State,
        id: Id,
    ) -> Result<(), crate::doc_transfer_state::Error> {
        use crate::{doc_transfer_state::Error, proto::PairPhase};
        use khor_sync::doc_transfer::DocumentVersion;
        let mut plan = state
            .pair_settlement(id)
            .ok_or("missing settlement intent")?;
        if !plan.ready {
            return Err("settlement body is still Preparing".into());
        }
        if plan.finished {
            return Ok(());
        }
        let mut loaded = self.devices_loaded()?;
        if loaded.store.epoch() != plan.epoch {
            return Err("settlement document epoch changed".into());
        }
        let current_anchor = self.anchor().map(|a| *a.as_bytes());
        if current_anchor != plan.before_anchor && current_anchor != plan.desired_anchor {
            return Err("settlement root changed".into());
        }
        let covered = plan
            .target
            .as_ref()
            .map(|bytes| DocumentVersion::decode(bytes).map_err(Error::from_std))
            .transpose()?
            .is_some_and(|target| {
                target.epoch == loaded.store.epoch()
                    && loaded.doc.version().includes_vv(&target.version)
            });
        let current_key = khor_net::root::load(&self.khor_file(ROOT_KEY))
            .map_err(|e| Error::from(e.to_string()))?
            .map(|key| *key.public().as_bytes());
        if current_key != plan.before_key && !(covered && current_key.is_none()) {
            return Err("settlement private-key identity changed".into());
        }
        if !covered {
            let roster = plan
                .desired_roster
                .as_ref()
                .map(|m| state.load_roster_blob(m))
                .transpose()?
                .unwrap_or_default();
            let desired = verified_settlement_roster(&roster)?;
            if desired
                .as_ref()
                .is_some_and(|r| Some(r.root) != plan.desired_anchor)
            {
                return Err("settlement signed root differs from intent".into());
            }
            // Publishing the desired anchor precedes the document flush. On
            // restart the disk may still contain only the old signed roster;
            // validate it under the saved old public identity, not the newly
            // published anchor. Either exact before/desired bytes are safe.
            let old = plan
                .before_anchor
                .and_then(|a| iroh::PublicKey::from_bytes(&a).ok())
                .and_then(|a| newest_in(&loaded.doc, &a));
            let next = plan
                .desired_anchor
                .and_then(|a| iroh::PublicKey::from_bytes(&a).ok())
                .and_then(|a| newest_in(&loaded.doc, &a));
            let old_digest = old.as_ref().map(|r| *blake3::hash(r.bytes()).as_bytes());
            let next_digest = next.as_ref().map(|r| *blake3::hash(r.bytes()).as_bytes());
            let desired_digest = desired
                .as_ref()
                .map(|r| *blake3::hash(r.bytes()).as_bytes());
            if old_digest != plan.before_roster && next_digest != desired_digest {
                return Err("settlement roster changed".into());
            }
            if next_digest.is_some()
                && next_digest != desired_digest
                && plan.before_anchor != plan.desired_anchor
            {
                return Err("desired network roster changed".into());
            }
            if !plan.dialed {
                let claim = state.pair_claim(id).ok_or("unowned issuer settlement")?;
                let path = self.pair_invite_hash_path(claim.token_hash);
                match std::fs::remove_file(&path) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (), // this durable Ready intent owns the burn
                    Err(e) => return Err(msg::cant_burn_invite(e).into()),
                }
            }
            if let Some(anchor) = plan.desired_anchor {
                self.set_anchor(&iroh::PublicKey::from_bytes(&anchor).map_err(Error::from_std)?)?;
            }
            #[cfg(test)]
            crate::link::pairing::tests::checkpoint(context, id, "anchor");
            if let Some(desired) = &desired {
                if plan.before_anchor != plan.desired_anchor {
                    let old: Vec<_> = loaded
                        .doc
                        .rosters()
                        .into_iter()
                        .map(|(key, _)| key)
                        .filter(|key| !key.starts_with("pair-archive:"))
                        .collect();
                    loaded.doc.drop_rosters(&old)?;
                }
                self.store_roster(&loaded.doc, desired)?;
            }
            let remote = nt::id_hex(&plan.remote);
            let current = loaded.doc.get(&remote);
            let concurrent = current.as_ref().map(|r| (&r.name, &r.addrs))
                != plan.row_before.as_ref().map(|r| (&r.0, &r.1));
            let (name, addrs) = if concurrent {
                current
                    .as_ref()
                    .map(|r| (r.name.as_str(), r.addrs.as_slice()))
                    .unwrap_or((plan.remote_name.as_str(), plan.remote_addrs.as_slice()))
            } else {
                (plan.remote_name.as_str(), plan.remote_addrs.as_slice())
            };
            loaded.doc.upsert(&remote, name, addrs)?;
            loaded.doc.set_gone(&remote, false)?;
            if let Some(desired) = &desired {
                for party in [plan.remote, context.local] {
                    if let Some(cert) = desired.signed_cert(&party) {
                        loaded.doc.set_cert(&nt::id_hex(&party), cert)?;
                    }
                }
            }
            if let Some(input) = &plan.input_roster {
                let raw = state.load_roster_blob(input)?;
                verified_settlement_roster(&raw)?;
                loaded
                    .doc
                    .put_roster(&pair_archive(id, context.local), &raw)?;
            }
            plan.target = Some(
                DocumentVersion {
                    epoch: loaded.store.epoch(),
                    version: loaded.doc.version(),
                }
                .encode()
                .map_err(Error::from_std)?
                .into(),
            );
            state.save_settlement(plan.clone())?;
            loaded.store.flush(&loaded.doc)?;
            #[cfg(test)]
            crate::link::pairing::tests::checkpoint(context, id, "flushed");
        }
        // Flush and its saved target precede retirement. Never delete a new
        // key, even if another owner restored a different root after a crash.
        if let Some(old) = plan.before_key
            && Some(old) != plan.desired_anchor
        {
            let path = self.khor_file(ROOT_KEY);
            if let Some(key) =
                khor_net::root::load(&path).map_err(|e| Error::from(e.to_string()))?
            {
                if key.public().as_bytes() != &old {
                    return Err("root key changed before retirement".into());
                }
                if newest_in(&loaded.doc, &key.public()).is_some_and(|r| !r.lone(&self.my_id())) {
                    return Err(msg::pair_two_roots().into());
                }
                std::fs::remove_file(&path).map_err(Error::from_std)?;
            }
            match std::fs::remove_file(self.khor_file(UNSHOWN)) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(Error::from_std(e)),
            }
        }
        for (party, name) in [
            (plan.remote, plan.remote_name.as_str()),
            (context.local, self.name.as_str()),
        ] {
            let hex = nt::id_hex(&party);
            let path = self.root().join(".khor/pending").join(&hex);
            if self.standing_in(&loaded.doc, &hex, crate::now_ms()) == Standing::Trusted {
                match std::fs::remove_file(&path) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(Error::from_std(e)),
                }
            } else if self.anchor().is_some() && !path.exists() {
                let note = PendingNote {
                    name: name.into(),
                    kind: Kind::Device.key().into(),
                    sponsor: None,
                    since_ms: crate::now_ms(),
                };
                std::fs::create_dir_all(path.parent().unwrap()).map_err(Error::from_std)?;
                std::fs::write(&path, serde_json::to_vec(&note).map_err(Error::from_std)?)
                    .map_err(Error::from_std)?;
            }
        }
        drop(loaded);
        if let Some(mut claim) = state.pair_claim(id) {
            claim.phase = PairPhase::AdmissionFlushed;
            claim.anchor = plan.desired_anchor;
            claim.roster_out = plan.desired_roster.clone();
            claim.devices_have = context
                .version("devices")?
                .encode()
                .map_err(Error::from_std)?
                .into();
            state.save_claim(claim)?;
        } else if let Some(mut local) = state.pair_local(id) {
            local.phase = PairPhase::AdmissionFlushed;
            local.anchor = plan.desired_anchor;
            state.save_local(local)?;
        } else {
            return Err("settlement owner disappeared".into());
        }
        plan.finished = true;
        state.save_settlement(plan)?;
        Ok(())
    }

    pub(crate) fn pair_replay_allowed(
        &self,
        remote: Id,
        anchor: Option<Id>,
    ) -> Result<bool, String> {
        if self.anchor().map(|a| *a.as_bytes()) != anchor {
            return Ok(false);
        }
        let loaded = self.devices_loaded()?;
        let remote = nt::id_hex(&remote);
        Ok(loaded.doc.get(&remote).is_some()
            && loaded.doc.get(self.device_str()).is_some()
            && !loaded
                .doc
                .gone_ids()
                .iter()
                .any(|id| id == &remote || id == self.device_str())
            && self.standing_in(&loaded.doc, &remote, crate::now_ms()) != Standing::Revoked
            && self.standing_in(&loaded.doc, self.device_str(), crate::now_ms())
                != Standing::Revoked)
    }
    pub(crate) fn release_pair_input(
        &self,
        state: &mut crate::doc_transfer_state::State,
        id: Id,
        manifest: &crate::proto::DocManifest,
    ) -> Result<(), crate::doc_transfer_state::Error> {
        let loaded = self.devices_loaded()?;
        let archive = loaded
            .doc
            .rosters()
            .into_iter()
            .find(|(key, _)| key == &pair_archive(id, self.my_id()));
        if !archive.is_some_and(|(_, bytes)| {
            bytes.len() as u64 == manifest.total_bytes
                && blake3::hash(&bytes).as_bytes() == &manifest.digest
        }) {
            return Err("settled roster archive is missing or changed".into());
        }
        drop(loaded);
        state.release_roster_blob(manifest)
    }
    pub(crate) fn vouch_certificate(&self) -> Option<Vec<u8>> {
        let anchor = self.anchor()?;
        let loaded = self.devices_loaded().ok()?;
        newest_in(&loaded.doc, &anchor)?
            .signed_cert(&self.my_id())
            .map(Vec::from)
    }
    pub(crate) fn authorize_vouch_header(
        &self,
        remote: Id,
        certificate: &[u8],
    ) -> Result<Id, String> {
        let refused = || msg::vouch_refused().to_owned();
        if certificate.len() > 1024 {
            return Err(refused());
        }
        let anchor = self.anchor().ok_or_else(refused)?;
        let cert = nt::verify_cert(certificate, &anchor).map_err(|_| refused())?;
        if cert.subject != remote || !cert.live_at(crate::now_ms()) {
            return Err(refused());
        }
        let loaded = self.devices_loaded()?;
        if loaded.doc.gone_ids().contains(&nt::id_hex(&remote)) {
            return Err(refused());
        }
        Ok(*anchor.as_bytes())
    }
}

fn anchor_at(root: &std::path::Path) -> Option<iroh::PublicKey> {
    let text = std::fs::read_to_string(root.join(".khor").join(ROOT_PUB)).ok()?;
    iroh::PublicKey::from_bytes(&nt::id_from_hex(text.trim())?).ok()
}
fn set_anchor_at(root: &std::path::Path, anchor: &iroh::PublicKey) -> Result<(), String> {
    crate::doc_transfer_state::publish_anchor(root, format!("{anchor}\n").as_bytes())
}
fn root_secret_at(root: &std::path::Path) -> Result<Option<iroh::SecretKey>, String> {
    let Some(key) =
        khor_net::root::load(&root.join(".khor").join(ROOT_KEY)).map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    Ok((anchor_at(root) == Some(key.public())).then_some(key))
}
fn store_roster_at(doc: &DeviceDoc, r: &Roster) -> Result<(), String> {
    doc.put_roster(&roster_key(r), r.bytes())?;
    let older: Vec<_> = doc
        .rosters()
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| {
            !key.starts_with("pair-archive:") && key_seq(key).is_none_or(|seq| seq < r.seq)
        })
        .collect();
    doc.drop_rosters(&older)
}
fn root_mutation_at(
    root: &std::path::Path,
    local: Id,
) -> Result<crate::doc_transfer_state::RootMutationGuard, String> {
    let guard = crate::doc_transfer_state::pairing::root_fence(root).map_err(String::from)?;
    if crate::doc_transfer_state::pairing::needs_recovery(root, local).map_err(String::from)? {
        return Err("trust settlement recovery is pending".into());
    }
    Ok(guard)
}

fn pair_archive(id: Id, recipient: Id) -> String {
    format!(
        "pair-archive:{}:{}",
        nt::id_hex(&id),
        nt::id_hex(&recipient)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_790_000_000_000;

    fn id(n: u8) -> Id {
        [n; 32]
    }

    fn roster(key: &iroh::SecretKey, members: &[Member], at: u64) -> Roster {
        nt::verify_roster(&nt::genesis(key, at, members), &key.public()).unwrap()
    }

    #[test]
    fn with_no_anchor_everyone_stands_as_before_roots() {
        assert_eq!(standing(None, None, &id(1), None, NOW), Standing::Legacy);
        assert!(Standing::Legacy.admitted());
    }

    #[test]
    fn the_roster_decides_listed_waiting_and_revoked() {
        let key = iroh::SecretKey::generate();
        let r = roster(&key, &[Member::device(id(1)), Member::device(id(2))], NOW);
        let r = nt::verify_roster(&r.revoke(&key, NOW, &id(2)), &key.public()).unwrap();
        let a = key.public();
        assert_eq!(
            standing(Some(&r), Some(&a), &id(1), None, NOW),
            Standing::Trusted
        );
        assert_eq!(
            standing(Some(&r), Some(&a), &id(2), None, NOW),
            Standing::Revoked
        );
        assert_eq!(
            standing(Some(&r), Some(&a), &id(3), None, NOW),
            Standing::Pending
        );
        assert_eq!(
            standing(None, Some(&a), &id(1), None, NOW),
            Standing::Pending
        );
    }

    /// B6: a presented certificate newer than the roster counts while
    /// the roster is fresh, and **stops counting when it goes stale** —
    /// while everyone the stale roster listed still counts (fail open).
    /// The clock is the argument; nothing sleeps.
    #[test]
    fn a_stale_roster_keeps_its_members_and_takes_no_new_certificate() {
        let key = iroh::SecretKey::generate();
        let a = key.public();
        let old = roster(&key, &[Member::device(id(1))], NOW);
        // Signed later, elsewhere: a roster this machine has not seen yet,
        // whose certificate for id(5) rides id(5)'s row.
        let newer =
            nt::verify_roster(&old.admit(&key, NOW + 1, &Member::device(id(5))), &a).unwrap();
        let presented = newer.signed_cert(&id(5)).unwrap().to_vec();

        let fresh = NOW + nt::FRESH_DAYS * nt::DAY_MS;
        assert_eq!(
            standing(Some(&old), Some(&a), &id(5), Some(&presented), fresh),
            Standing::Trusted,
            "fresh: a newer certificate is taken on sight"
        );
        let stale = fresh + 1;
        assert!(old.is_stale(stale));
        assert_eq!(
            standing(Some(&old), Some(&a), &id(5), Some(&presented), stale),
            Standing::Pending,
            "stale: no new certificate"
        );
        assert_eq!(
            standing(Some(&old), Some(&a), &id(1), None, stale + 400 * nt::DAY_MS),
            Standing::Trusted,
            "stale: the old members still count"
        );
    }

    #[test]
    fn a_presented_certificate_cannot_undo_a_revocation_or_stand_in_for_another() {
        let key = iroh::SecretKey::generate();
        let a = key.public();
        let r1 = roster(&key, &[Member::device(id(1)), Member::device(id(2))], NOW);
        let old_cert = r1.signed_cert(&id(2)).unwrap().to_vec();
        let r2 = nt::verify_roster(&r1.revoke(&key, NOW, &id(2)), &a).unwrap();
        assert_eq!(
            standing(Some(&r2), Some(&a), &id(2), Some(&old_cert), NOW),
            Standing::Revoked,
            "the old certificate is older than the revocation"
        );
        // A certificate for somebody else, carried on this row.
        let r3 = nt::verify_roster(&r2.admit(&key, NOW, &Member::device(id(7))), &a).unwrap();
        let sevens = r3.signed_cert(&id(7)).unwrap().to_vec();
        assert_eq!(
            standing(Some(&r2), Some(&a), &id(3), Some(&sevens), NOW),
            Standing::Pending
        );
        // A certificate from another root.
        let other = iroh::SecretKey::generate();
        let theirs = roster(&other, &[Member::device(id(3))], NOW);
        assert_eq!(
            standing(Some(&r2), Some(&a), &id(3), theirs.signed_cert(&id(3)), NOW),
            Standing::Pending
        );
    }

    #[test]
    fn roster_keys_sort_by_seq_and_read_back() {
        let key = iroh::SecretKey::generate();
        let r = roster(&key, &[Member::device(id(1))], NOW);
        let k = roster_key(&r);
        assert_eq!(key_seq(&k), Some(1));
        let r2 = nt::verify_roster(&r.refresh(&key, NOW), &key.public()).unwrap();
        let mut ks = [roster_key(&r2), k.clone()];
        ks.sort();
        assert_eq!(ks[0], k, "zero-padded keys sort the way seqs do");
    }

    #[test]
    fn settlement_plan_owns_the_verified_roster_sequence_and_anchor_snapshot() {
        let local_key = iroh::SecretKey::generate();
        let remote_key = iroh::SecretKey::generate();
        let mine = roster(&local_key, &[Member::device(id(1))], NOW);
        let theirs = roster(
            &remote_key,
            &[Member::device(id(2)), Member::device(id(3))],
            NOW,
        );
        let expected_bytes = theirs.bytes().to_vec();
        let mut wire = expected_bytes.clone();
        let anchor = *local_key.public().as_bytes();
        let plan = plan_settlement(
            Some(anchor),
            Some(&mine),
            &id(1),
            verified_settlement_roster(&wire).unwrap(),
            &nt::id_hex(&id(2)),
            false,
        )
        .unwrap();
        wire.fill(0);
        let newer = theirs.refresh(&remote_key, NOW + 1);
        let newer = nt::verify_roster(&newer, &remote_key.public()).unwrap();
        assert_eq!(plan.action, SettlementAction::Adopt);
        assert_eq!(plan.anchor, Some(anchor));
        let fixed = plan.roster.as_ref().unwrap();
        assert_eq!(fixed.seq, theirs.seq);
        assert!(
            fixed.bytes() == expected_bytes.as_slice(),
            "plan must retain the exact verified signed bytes"
        );
        assert!(
            fixed.seq < newer.seq,
            "later roster creation must not change a prepared plan"
        );
        assert!(
            mine.root == anchor && mine.lone(&id(1)),
            "pure planning must leave the local snapshot intact"
        );
    }

    #[test]
    fn settlement_planning_uses_the_actual_identities_and_dialer_direction() {
        let local_key = iroh::SecretKey::generate();
        let remote_key = iroh::SecretKey::generate();
        let mine = roster(&local_key, &[Member::device(id(1))], NOW);
        let theirs = roster(&remote_key, &[Member::device(id(2))], NOW);
        let decide = |me: Id, them: Id, dialed| {
            plan_settlement(
                Some(*local_key.public().as_bytes()),
                Some(&mine),
                &me,
                verified_settlement_roster(theirs.bytes()).unwrap(),
                &nt::id_hex(&them),
                dialed,
            )
            .unwrap()
            .action
        };
        assert_eq!(decide(id(1), id(2), true), SettlementAction::Adopt);
        assert_eq!(decide(id(1), id(2), false), SettlementAction::Keep);
        assert_eq!(
            decide(id(9), id(2), true),
            SettlementAction::Keep,
            "a one-member roster is not lone for a different local identity"
        );
        assert_eq!(
            decide(id(1), id(9), false),
            SettlementAction::Adopt,
            "the remote identity, not just roster size, decides whether its root is lone"
        );
    }

    #[test]
    fn settlement_planning_refuses_bad_signatures_and_two_existing_networks() {
        let local_key = iroh::SecretKey::generate();
        let remote_key = iroh::SecretKey::generate();
        let mine = roster(
            &local_key,
            &[Member::device(id(1)), Member::device(id(3))],
            NOW,
        );
        let theirs = roster(
            &remote_key,
            &[Member::device(id(2)), Member::device(id(4))],
            NOW,
        );
        let mut bad = theirs.bytes().to_vec();
        *bad.last_mut().unwrap() ^= 1;
        assert!(verified_settlement_roster(&bad).is_err());
        let decision = plan_settlement(
            Some(*local_key.public().as_bytes()),
            Some(&mine),
            &id(1),
            verified_settlement_roster(theirs.bytes()).unwrap(),
            &nt::id_hex(&id(2)),
            false,
        );
        assert!(
            matches!(decision, Err(why) if why == msg::pair_two_roots()),
            "two non-lone roots must be refused before an apply plan exists"
        );
        assert!(verified_settlement_roster(&[]).unwrap().is_none());
    }

    // ── on a real store (no network) ────────────────────────────────

    use crate::testdir::TempTree;
    use khor_catalog::cli;
    use khor_sync::store::Doc;

    fn node(t: &TempTree, name: &str) -> Node {
        Node::open_as(t.path(), name).unwrap()
    }

    /// What pairing through a machine without the root leaves behind,
    /// minus the wire: `b` meets `a`'s roster the way `pair_with` does
    /// (it dialed), and `a` gets `b`'s row the way a sync would bring it.
    fn join_through_a_member(a: &Node, b: &Node) {
        let lb = b.devices_loaded().unwrap();
        b.settle_with(&lb.doc, &a.roster_offer(), a.device_str(), true)
            .unwrap();
        b.enroll_after_pair(&lb.doc, &[(b.device_str(), b.name())])
            .unwrap();
        let mut sb = lb.store;
        sb.flush(&lb.doc).unwrap();
        carry(b, a);
    }

    /// One direction of a sync, by hand.
    fn carry(from: &Node, to: &Node) {
        let lf = from.devices_loaded().unwrap();
        let lt = to.devices_loaded().unwrap();
        lt.doc.merge(&lf.doc.snapshot().unwrap()).unwrap();
        let mut st = lt.store;
        st.flush(&lt.doc).unwrap();
    }

    fn fake_id(n: u8) -> String {
        nt::id_hex(&[n; 32])
    }

    /// B1: the first machine makes the root, once, owner-only, and the
    /// words are waiting to be shown until somebody sees them.
    #[test]
    fn the_first_machine_makes_its_root_once_and_waits_to_show_the_words() {
        let t = TempTree::fresh("trust-born");
        let a = node(&t, "alpha");
        assert!(
            a.root_born(),
            "alone and without a root: this is the first machine"
        );
        assert!(a.holds_root());
        let key = khor_net::root::load(&t.join(".khor/root.key"))
            .unwrap()
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(t.join(".khor/root.key"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let st = a.root_state();
        assert_eq!(st.root, Some(key.public().to_string()));
        assert!(st.holds && st.unshown);
        assert_eq!(st.standing, "trusted", "the holder certifies itself");
        assert_eq!(st.seq, Some(1), "a root's first roster is version 1");
        assert_eq!(a.standing_of(a.device_str()), Standing::Trusted);

        let again = node(&t, "alpha");
        assert!(!again.root_born(), "a second open makes nothing");
        assert_eq!(again.root_state().root, st.root);

        let words = a.root_words().unwrap();
        assert_eq!(words.split(' ').count(), khor_net::root::WORDS);
        assert_eq!(
            khor_net::root::from_words(&words).unwrap().public(),
            key.public()
        );
        assert!(!a.root_state().unshown, "shown once is shown");
    }

    /// `khor id --reset` makes the holder a new machine; it is still the
    /// holder, so it signs its new self back in at the next open.
    #[test]
    fn a_holder_that_resets_its_identity_signs_its_new_self_in() {
        let t = TempTree::fresh("trust-reset");
        let a = node(&t, "alpha");
        let old = a.device_str().to_owned();
        let root = a.root_state().root;
        crate::lock::reset_identity(&t.path(), "alpha", "alpha").unwrap();
        let b = node(&t, "alpha");
        assert_ne!(b.device_str(), old, "a new machine");
        assert!(!b.root_born(), "not a new root");
        assert_eq!(b.root_state().root, root);
        assert!(b.holds_root());
        assert_eq!(b.standing_of(b.device_str()), Standing::Trusted);
    }

    #[test]
    fn the_words_are_given_only_for_this_machines_own_name() {
        let t = TempTree::fresh("trust-words");
        let a = node(&t, "alpha");
        let shown = a.shown_name();
        assert_eq!(
            a.root_words_confirmed("  ").unwrap_err(),
            msg::words_need_confirm(&shown)
        );
        assert_eq!(
            a.root_words_confirmed("somebody").unwrap_err(),
            msg::words_names_another("somebody", &shown)
        );
        assert_eq!(
            a.root_words_confirmed("alpha").unwrap(),
            a.root_words().unwrap()
        );
    }

    /// B1: the words bring the same root back on another machine; a
    /// machine's own fresh root gives way, a real network's does not.
    #[test]
    fn recovering_the_words_restores_the_root_and_refuses_another_network() {
        let (ta, tc, td) = (
            TempTree::fresh("trust-rec-a"),
            TempTree::fresh("trust-rec-c"),
            TempTree::fresh("trust-rec-d"),
        );
        let a = node(&ta, "alpha");
        let words = a.root_words().unwrap();
        let root = a.root_state().root.unwrap();

        let c = node(&tc, "gamma");
        assert_ne!(c.root_state().root.as_ref(), Some(&root), "c made its own");
        assert_eq!(c.root_recover(&words).unwrap(), root);
        assert!(c.holds_root());
        assert_eq!(c.root_state().root, Some(root.clone()));
        assert_eq!(c.standing_of(c.device_str()), Standing::Trusted);

        let mut wrong: Vec<&str> = words.split(' ').collect();
        wrong[3] = "notaword";
        assert_eq!(
            c.root_recover(&wrong.join(" ")).unwrap_err(),
            msg::root_words_unknown(4, "notaword")
        );

        // d is a network of two under its own root.
        let d = node(&td, "delta");
        let ld = d.devices_loaded().unwrap();
        ld.doc.upsert(&fake_id(9), "nine", &[]).unwrap();
        assert!(
            d.enroll_after_pair(&ld.doc, &[(&fake_id(9), "nine")])
                .unwrap()
        );
        let mut sd = ld.store;
        sd.flush(&ld.doc).unwrap();
        let theirs = d.anchor().unwrap();
        assert_eq!(
            d.root_recover(&words).unwrap_err(),
            msg::recover_other_network(theirs)
        );
        assert!(
            !d.root_state().root.unwrap().eq(&root),
            "d keeps its network"
        );
    }

    /// B4 without the wire: a machine joining through one without the
    /// root waits — on both sides — and the holder's approval lets it in.
    #[test]
    fn joining_through_a_member_waits_until_the_holder_approves() {
        let (ta, tb) = (
            TempTree::fresh("trust-join-a"),
            TempTree::fresh("trust-join-b"),
        );
        let a = node(&ta, "alpha");
        let b = node(&tb, "beta");
        assert!(b.root_born(), "b starts as a network of its own");
        join_through_a_member(&a, &b);
        assert!(!b.holds_root(), "b's own root gave way");
        assert!(!tb.join(".khor/root.key").exists(), "and its key is gone");
        assert_eq!(b.root_state().root, a.root_state().root);
        assert_eq!(b.standing_of(b.device_str()), Standing::Pending);
        assert!(b.pending().unwrap().iter().any(|r| r.id == b.device_str()));
        assert_eq!(a.standing_of(b.device_str()), Standing::Pending);
        let waiting = a.pending().unwrap();
        assert_eq!(waiting.len(), 1, "{waiting:?}");
        assert_eq!(
            (waiting[0].name.as_str(), waiting[0].kind.as_str()),
            ("beta", "device")
        );

        assert_eq!(
            b.approve_here("beta").unwrap_err(),
            msg::approve_needs_root()
        );
        let (name, seq) = a.approve_here("beta").unwrap();
        assert_eq!(name, "beta");
        assert_eq!(a.roster().unwrap().seq, seq);
        assert_eq!(a.standing_of(b.device_str()), Standing::Trusted);
        assert!(a.pending().unwrap().is_empty());
        carry(&a, &b);
        assert_eq!(
            b.standing_of(b.device_str()),
            Standing::Trusted,
            "the roster travels"
        );
        assert!(b.pending().unwrap().is_empty(), "and the note is cleared");
        assert_eq!(
            a.approve_here("beta").unwrap_err(),
            msg::nothing_pending_named("beta")
        );
    }

    /// B5: a satellite adopted on the holder is signed as a satellite of
    /// it; adopted anywhere else it waits, and the holder's approval
    /// signs it as the adopter's.
    #[test]
    fn a_satellite_is_signed_on_the_holder_and_waits_anywhere_else() {
        let (ta, tb) = (
            TempTree::fresh("trust-sat-a"),
            TempTree::fresh("trust-sat-b"),
        );
        let a = node(&ta, "alpha");
        let sat = fake_id(0x51);
        a.enter_satellite(
            &sat,
            "box",
            &crate::satellite::Dest::new("box.example", None),
            false,
        )
        .unwrap();
        let r = a.roster().unwrap();
        let c = r
            .cert(&nt::id_from_hex(&sat).unwrap())
            .expect("signed on the holder");
        assert_eq!(c.kind, Kind::Satellite);
        assert_eq!(c.sponsor, nt::id_from_hex(a.device_str()));
        assert!(a.pending().unwrap().is_empty());

        let b = node(&tb, "beta");
        join_through_a_member(&a, &b);
        a.approve_here("beta").unwrap();
        carry(&a, &b);
        let sat2 = fake_id(0x52);
        b.enter_satellite(
            &sat2,
            "box2",
            &crate::satellite::Dest::new("box2.example", None),
            true,
        )
        .unwrap();
        let waiting = b.pending().unwrap();
        assert!(
            waiting
                .iter()
                .any(|w| w.id == sat2 && w.kind == "satellite"),
            "{waiting:?}"
        );
        assert!(
            !b.roster()
                .unwrap()
                .admits(&nt::id_from_hex(&sat2).unwrap(), crate::now_ms())
        );
        carry(&b, &a);
        a.approve_here("box2").unwrap();
        let c2 = a
            .roster()
            .unwrap()
            .cert(&nt::id_from_hex(&sat2).unwrap())
            .cloned()
            .unwrap();
        assert_eq!(c2.kind, Kind::Satellite);
        assert_eq!(
            c2.sponsor,
            nt::id_from_hex(b.device_str()),
            "the adopter speaks for it"
        );
    }

    /// `forget` on the holder revokes; anywhere else the removal waits
    /// for the holder, and approving it is the revocation.
    #[test]
    fn forgetting_revokes_on_the_holder_and_waits_elsewhere() {
        let (ta, tb) = (
            TempTree::fresh("trust-fgt-a"),
            TempTree::fresh("trust-fgt-b"),
        );
        let a = node(&ta, "alpha");
        let (x, y) = (fake_id(0x0a), fake_id(0x0b));
        {
            let la = a.devices_loaded().unwrap();
            la.doc.upsert(&x, "xray", &[]).unwrap();
            la.doc.upsert(&y, "yoke", &[]).unwrap();
            a.enroll_after_pair(&la.doc, &[(&x, "xray"), (&y, "yoke")])
                .unwrap();
            let mut s = la.store;
            s.flush(&la.doc).unwrap();
        }
        a.forget_device("xray").unwrap();
        assert_eq!(a.standing_of(&x), Standing::Revoked);
        assert!(
            a.roster()
                .unwrap()
                .is_revoked(&nt::id_from_hex(&x).unwrap())
        );

        let b = node(&tb, "beta");
        join_through_a_member(&a, &b);
        a.approve_here("beta").unwrap();
        carry(&a, &b);
        b.forget_device("yoke").unwrap();
        assert_eq!(b.standing_of(&y), Standing::Trusted, "b cannot revoke");
        let waiting = b.pending().unwrap();
        assert!(
            waiting.iter().any(|w| w.id == y && w.kind == "leave"),
            "{waiting:?}"
        );
        carry(&b, &a);
        assert!(
            a.pending()
                .unwrap()
                .iter()
                .any(|w| w.id == y && w.kind == "leave")
        );
        a.approve_here("yoke").unwrap();
        assert_eq!(a.standing_of(&y), Standing::Revoked);
        assert!(a.pending().unwrap().is_empty());
        assert_eq!(a.revoke_here("beta").map(|(n, _)| n), Ok("beta".to_owned()));
        assert_eq!(
            a.revoke_here("beta").unwrap_err(),
            msg::already_revoked("beta")
        );
        assert_eq!(
            a.revoke_here("alpha").unwrap_err(),
            msg::cant_revoke_holder()
        );
        assert_eq!(
            b.revoke_here("alpha").unwrap_err(),
            msg::revoke_needs_root()
        );
    }

    /// Empties a table on disk the way hinton's was found (round NET-10):
    /// every block gone, the ledger kept — so no sync would pull the
    /// blocks back by name — and the machine's own row written again by
    /// its next open.
    fn lose_the_table(t: &TempTree, name: &str) -> Node {
        let dir = khor_sync::devices::devices_dir(&t.path());
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "loro") {
                std::fs::remove_file(e.path()).unwrap();
            }
        }
        node(t, name)
    }

    /// NET-10: a machine whose table lost every row but its own is taken
    /// back on a roster signed by its own root, and on nothing else.
    #[test]
    fn a_table_that_lost_its_rows_takes_a_machine_back_on_the_roots_word_only() {
        let (ta, tb, tc) = (
            TempTree::fresh("trust-vouch-a"),
            TempTree::fresh("trust-vouch-b"),
            TempTree::fresh("trust-vouch-c"),
        );
        let a = node(&ta, "alpha");
        let b = node(&tb, "beta");
        join_through_a_member(&a, &b);
        a.approve_here("beta").unwrap();
        carry(&a, &b);
        let old = a.vouch_offer().expect("a member on the roster vouches");

        let b = lose_the_table(&tb, "beta");
        let names = |n: &Node| {
            n.devices()
                .unwrap()
                .into_iter()
                .map(|d| d.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&b), ["beta"], "only its own row is left");
        assert_eq!(b.admit(a.device_str()).unwrap_err(), msg::not_paired());
        assert_eq!(
            b.standing_of(b.device_str()),
            Standing::Pending,
            "and no roster"
        );

        // Another network's roster, and a roster that does not name the asker.
        let c = node(&tc, "gamma");
        let (cn, ca, cr) = c.vouch_offer().unwrap();
        assert_eq!(
            b.take_vouch(c.device_str(), &cn, &ca, &cr).unwrap_err(),
            msg::vouch_refused()
        );
        let stranger = fake_id(0x5e);
        assert_eq!(
            b.take_vouch(&stranger, "x", &[], &old.2).unwrap_err(),
            msg::vouch_refused()
        );
        assert_eq!(names(&b), ["beta"], "a refusal writes nothing");

        b.take_vouch(a.device_str(), &old.0, &old.1, &old.2)
            .unwrap();
        assert_eq!(b.admit(a.device_str()), Ok(()));
        assert_eq!(b.standing_of(b.device_str()), Standing::Trusted);
        carry(&a, &b);
        assert_eq!(
            names(&b),
            ["alpha", "beta"],
            "the round after brings the rest"
        );

        // An older roster cannot undo a newer one, and a machine shown
        // the door here is not let back in by vouching for itself.
        a.enter_satellite(
            &fake_id(0x51),
            "box",
            &crate::satellite::Dest::new("box.example", None),
            false,
        )
        .unwrap();
        carry(&a, &b);
        assert!(b.roster().unwrap().seq > old_seq(&old.2, &a));
        assert_eq!(
            b.take_vouch(a.device_str(), &old.0, &old.1, &old.2)
                .unwrap_err(),
            msg::vouch_refused()
        );
        let (n, ad, r) = a.vouch_offer().unwrap();
        {
            let lb = b.devices_loaded().unwrap();
            lb.doc.set_gone(a.device_str(), true).unwrap();
            let mut s = lb.store;
            s.flush(&lb.doc).unwrap();
        }
        assert_eq!(
            b.take_vouch(a.device_str(), &n, &ad, &r).unwrap_err(),
            msg::vouch_refused()
        );
    }

    fn old_seq(roster: &[u8], under: &Node) -> u64 {
        nt::verify_roster(roster, &under.anchor().unwrap())
            .unwrap()
            .seq
    }

    /// B6: a roster past its date is said by `khor doctor` in days, and
    /// the holder's beat signs it fresh again.
    #[test]
    fn a_stale_roster_is_said_in_days_and_the_holder_refreshes_it() {
        let t = TempTree::fresh("trust-stale");
        let a = node(&t, "alpha");
        let key = khor_net::root::load(&t.join(".khor/root.key"))
            .unwrap()
            .unwrap();
        let cur = a.roster().unwrap();
        let old_at = crate::now_ms() - 40 * nt::DAY_MS;
        // The same members, signed forty days ago with a later seq — what
        // a machine holds when the holder has been away that long.
        let old = nt::verify_roster(
            &cur.admit(&key, old_at, &Member::device(a.my_id())),
            &key.public(),
        )
        .unwrap();
        let old = nt::verify_roster(&old.refresh(&key, old_at), &key.public()).unwrap();
        {
            let l = a.devices_loaded().unwrap();
            a.store_roster(&l.doc, &old).unwrap();
            let mut s = l.store;
            s.flush(&l.doc).unwrap();
        }
        let st = a.root_state();
        assert_eq!((st.age_days, st.stale_days), (Some(40), Some(10)));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (health, said, _) = rt.block_on(crate::doctor::check_roster(&a));
        assert_eq!(health, crate::doctor::Health::Warn);
        assert_eq!(said, cli::doctor_roster_stale(10, 40));
        assert!(said.contains("10"), "名单已过期 N 天 names N: {said}");

        a.trust_tick().unwrap();
        let st = a.root_state();
        assert_eq!(st.stale_days, None, "the holder signed it again");
        assert!(st.seq.unwrap() > old.seq);
        let (health, _, _) = rt.block_on(crate::doctor::check_roster(&a));
        assert_eq!(health, crate::doctor::Health::Ok);
    }

    /// A network from before roots: `root init` signs the whole table —
    /// satellites as satellites — and a machine the roster names takes
    /// the root up on its next beat.
    #[test]
    fn a_network_from_before_roots_gets_one_and_its_machines_take_it_up() {
        let (ta, tb) = (
            TempTree::fresh("trust-init-a"),
            TempTree::fresh("trust-init-b"),
        );
        let a = node(&ta, "alpha");
        let b = node(&tb, "beta");
        // Both back to the shape an older khor left: no root anywhere.
        for (n, t) in [(&a, &ta), (&b, &tb)] {
            let _ = std::fs::remove_file(t.join(".khor/root.key"));
            let _ = std::fs::remove_file(t.join(".khor/root.pub"));
            let l = n.devices_loaded().unwrap();
            let every: Vec<String> = l.doc.rosters().into_iter().map(|(k, _)| k).collect();
            l.doc.drop_rosters(&every).unwrap();
            let mut s = l.store;
            s.flush(&l.doc).unwrap();
        }
        carry(&b, &a);
        carry(&a, &b);
        let sat = fake_id(0x5a);
        a.enter_satellite(
            &sat,
            "box",
            &crate::satellite::Dest::new("box.example", None),
            false,
        )
        .unwrap();
        assert_eq!(a.standing_of(b.device_str()), Standing::Legacy);
        assert!(
            a.pending().unwrap().is_empty(),
            "nobody to ask before roots"
        );

        let (root, count) = a.root_init().unwrap();
        assert_eq!(count, 3, "alpha, beta and the satellite");
        assert!(a.holds_root());
        let r = a.roster().unwrap();
        assert_eq!(
            r.cert(&nt::id_from_hex(&sat).unwrap()).unwrap().kind,
            Kind::Satellite
        );
        assert!(r.admits(&nt::id_from_hex(b.device_str()).unwrap(), crate::now_ms()));
        assert_eq!(a.root_init().unwrap_err(), msg::root_already(&root));

        assert_eq!(b.standing_of(a.device_str()), Standing::Legacy);
        carry(&a, &b);
        b.trust_tick().unwrap();
        assert_eq!(b.root_state().root, Some(root));
        assert!(!b.holds_root());
        assert_eq!(b.standing_of(a.device_str()), Standing::Trusted);
        assert_eq!(b.standing_of(b.device_str()), Standing::Trusted);
    }
}
