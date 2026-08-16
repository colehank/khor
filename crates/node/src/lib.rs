//! One device's node: identity, the session surface (docs/SESSION.md),
//! and the live link to the rest of the network (docs/NET.md).
//!
//! The kind trait ([`KindSurface`]) was cut only when the third kind
//! (live) arrived — the first two shared nothing but "produce rows",
//! and a trait cut from that would have frozen an accident. What three
//! real implementations actually share: produce rows, claim an id,
//! answer what "looked at now" means, and close.

pub mod adaptor;
pub mod chat;
pub mod host;
pub mod ipc;
pub mod link;
pub mod list;
pub mod live;
pub mod proto;
pub mod transfer;
pub mod vitals;

pub use khor_core::avatar::{
    avatar, preset, Avatar, AvatarSeed, AvatarStyle, FaceShape, Palette, Preset, Variant, PRESETS,
};
pub use khor_core::{kind, Fill, Session, SessionId, State, Vitals};
pub use khor_sync::chat::{FileRef, Message, MsgBody};
pub use khor_sync::devices::DeviceInfo;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use khor_catalog::msg;
use khor_core::{DeviceId, Event};
use khor_sync::chat::{channel_dir, channel_of_machine, ChatDoc, Sender};
use khor_sync::devices::{devices_dir, DeviceDoc};
use khor_sync::pins::{pins_dir, PinDoc};
use khor_sync::seen::{seen_dir, SeenDoc};
use khor_sync::store::{load, Loaded};

use crate::chat::ChatKind;
use crate::live::LiveKind;
use crate::transfer::TransferKind;

/// One kind's contribution to the session surface. Methods take the node
/// because every kind leans on shared context (the device table, the
/// seen watermarks) without owning it.
pub(crate) trait KindSurface {
    /// Whether this id is this kind's to act on, on this device.
    fn claims(&self, node: &Node, id: &SessionId) -> bool;
    /// The rows this kind derives on this device.
    fn rows(&self, node: &Node) -> Result<Vec<Session>, String>;
    /// Whether the rows travel as peer reports (only home can derive
    /// them) or every device derives its own (CRDT kinds).
    fn reportable(&self) -> bool;
    /// The watermark "looked at now" sets for this id.
    fn seen_at(&self, node: &Node, id: &SessionId) -> Result<i64, String>;
    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String>;
}

/// One row plus where it came from. Local rows carry no source; a row
/// learned from another device's report carries that device's name and
/// the report's age — the offline axis (docs/SESSION.md 离线不是第七个
/// 词): an unreachable device keeps its last word, aging visibly, and
/// "don't know" is never painted as a concrete word.
#[derive(Debug, Clone)]
pub struct SessionView {
    pub session: Session,
    /// `(device name, report age in ms)` for reported rows.
    pub source: Option<(String, u64)>,
    /// Pinned by someone, somewhere in the network (`khor_sync::pins`).
    /// Read here rather than carried on the row: the pin document
    /// replicates, so every device answers this from its own copy — a
    /// reported row from an older machine is pinned all the same.
    pub pinned: bool,
}

/// What subscribers receive. `watch()` is the one feed both faces
/// consume: the GUI repaints rows from it, the CLI prints from it —
/// polling is never the API.
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// A session's row changed (state, unread, title).
    Row(Session),
    /// A kind-namespaced event inside a session.
    Event(Event),
    /// The session is gone.
    Closed(SessionId),
}

pub struct Node {
    root: PathBuf,
    /// The identity key; also what the endpoint binds with. One live
    /// endpoint per key.
    key: iroh::SecretKey,
    device: DeviceId,
    /// The machine id as peers name it: the public key, hex.
    device_str: String,
    /// Own channel name.
    name: String,
    /// This process's writer peer, shared by every doc it writes.
    peer: u64,
    /// Relay roads this machine binds with and offers to others: the
    /// Khor tier plus `KHOR_RELAY` (docs/NET.md 中继).
    relays: Vec<String>,
    chat: ChatKind,
    transfer: TransferKind,
    live: LiveKind,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<NodeEvent>>>>,
    /// Serializes sync pumps within one process: two concurrent pumps
    /// share block stores and could collide on sequence numbers. The
    /// ticker skips when busy; an explicit sync waits its turn.
    pub(crate) sync_gate: tokio::sync::Mutex<()>,
}

impl Node {
    /// Opens the node rooted at `root`, named after the hostname (or
    /// `KHOR_NAME` when set — tests and same-machine dual instances).
    pub fn open(root: PathBuf) -> Result<Node, String> {
        let host = std::env::var("KHOR_NAME")
            .unwrap_or_else(|_| gethostname::gethostname().to_string_lossy().to_string());
        Self::open_as(root, &host)
    }

    /// Opens with an explicit machine name (pre-cleaning).
    pub fn open_as(root: PathBuf, name: &str) -> Result<Node, String> {
        let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
            .map_err(|e| e.to_string())?;
        let public = key.public();
        let device = DeviceId(*public.as_bytes());
        let device_str = public.to_string();
        let name = channel_of_machine(name)
            .ok_or_else(|| msg::name_not_pathable(format_args!("{name:?}")))?;
        let chat = ChatKind::new(
            root.clone(),
            Sender { id: device_str.clone(), name: name.clone() },
        );
        let transfer = TransferKind::new(root.clone(), device_str.clone(), name.clone());
        // The one place khor decides to read other vendors' files. Every
        // other construction of a LiveKind discovers nothing, so no test
        // answers with whatever the machine happens to be running.
        let live = LiveKind::new(root.clone(), device)
            .discovering(Arc::new(adaptor::Discovery::for_root(&root)));
        let node = Node {
            root,
            key,
            device,
            device_str,
            name,
            peer: ChatDoc::fresh_peer(),
            relays: khor_net::endpoint::configured_relays(),
            chat,
            transfer,
            live,
            subscribers: Arc::new(Mutex::new(Vec::new())),
            sync_gate: tokio::sync::Mutex::new(()),
        };
        node.register_self()?;
        Ok(node)
    }

    /// The table must know this machine before anyone else can. Name
    /// and self-reported avatar style only — addresses belong to a live
    /// endpoint (`serve` writes them), and clobbering them with `[]` on
    /// every open would erase the hints other devices dial by.
    ///
    /// **There used to be a `reassert_self` that ran this again after
    /// every devices merge, and it is gone.** It existed for one reason:
    /// a device's row was a nested container, two replicas could create
    /// the container for the same id concurrently (this machine at open,
    /// the inviter while answering `Request::Pair`), and loro settled
    /// that by keeping one container whole and discarding the other's
    /// contents — so `style`, the one field only this machine may write,
    /// came back blank on a freshly paired device with nothing reporting
    /// a problem. The table is now a flat set of registers keyed
    /// `<id>/<field>` (`khor_sync::devices` module head), no container is
    /// created and none can lose, so re-stating after a merge has
    /// nothing left to repair. It is also why this write, on its own,
    /// is now enough.
    fn register_self(&self) -> Result<(), String> {
        let loaded = self.devices_loaded()?;
        let keep = loaded
            .doc
            .get(&self.device_str)
            .map(|d| d.addrs)
            .unwrap_or_default();
        loaded.doc.upsert(&self.device_str, &self.name, &keep)?;
        // A machine's face is its own palette times its id: whoever is
        // looking paints it from what it reported, so this has to be in
        // the table before anyone syncs, not at first paint.
        loaded.doc.set_style(&self.device_str, &self.avatar_style().to_json()?)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// Where this machine's chosen avatar style is kept. A local
    /// preference file, not a synced document: **the choice is made
    /// here and only travels outward** (through the device table), so
    /// no other device can restyle this one.
    fn avatar_prefs_path(&self) -> PathBuf {
        self.root.join(".khor").join("avatar.json")
    }

    /// This machine's avatar style, or the factory default.
    ///
    /// Unreadable and unparseable both fall back to the default rather
    /// than erroring: "no file" is the ordinary state of a machine
    /// nobody has restyled, and a face is style, never identity — a
    /// fallback here cannot make anyone misread *which* machine they
    /// are looking at.
    ///
    /// A bad file falls back **whole**, never slot by slot — the reason
    /// is on `Palette`'s `Deserialize`, and it is the same one there and
    /// here: half a chosen style and half a default is a face nobody
    /// picked. `Node::restyle` validates before it writes, so the only
    /// way to reach that path is somebody editing `.khor/avatar.json` by
    /// hand.
    pub fn avatar_style(&self) -> AvatarStyle {
        fs::read_to_string(self.avatar_prefs_path())
            .ok()
            .and_then(|t| AvatarStyle::from_json(&t))
            .unwrap_or_default()
    }

    /// Chooses this machine's avatar style: persists it and reports it
    /// to the network in the same move. Splitting those two is how a
    /// machine ends up wearing one face locally and another everywhere
    /// else.
    pub fn set_avatar_style(&self, style: &AvatarStyle) -> Result<(), String> {
        link::write_private(&self.avatar_prefs_path(), style.to_json()?.as_bytes())?;
        let loaded = self.devices_loaded()?;
        loaded.doc.set_style(&self.device_str, &style.to_json()?)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// Changes the axes it is handed and leaves the rest where they are
    /// — the one call behind `khor face` and behind every control on the
    /// settings screen, so the two faces cannot drift into meaning
    /// different things by the same words.
    ///
    /// **Refused by name, never defaulted.** An unknown variant key or a
    /// slot that is not `#rrggbb` comes back as a message. Falling
    /// through to the factory style instead would make a typo look
    /// exactly like the setting failing to take — docs/UX.md forbids
    /// 做了但没变化 wearing the same face as 失败 — and the next thing
    /// anybody does is press it again.
    ///
    /// Read-modify-write over the *whole* style, because whole is what
    /// the device table carries (`khor_sync::devices`: one JSON value,
    /// whole-value LWW). There is deliberately nothing finer to write:
    /// half a palette from one writer and a variant from another is a
    /// face nobody chose.
    pub fn restyle(
        &self,
        colors: Option<&[String]>,
        variant: Option<&str>,
        shape: Option<&str>,
    ) -> Result<AvatarStyle, String> {
        let mut style = self.avatar_style();
        if let Some(colors) = colors {
            style.palette = Palette::parse(colors).ok_or_else(|| {
                msg::not_a_palette(colors.join(khor_catalog::cli::NAME_SEPARATOR))
            })?;
        }
        if let Some(key) = variant {
            style.variant = Variant::from_key(key).ok_or_else(|| {
                let all: Vec<&str> = Variant::ALL.iter().map(|v| v.key()).collect();
                msg::not_a_variant(key, all.join(khor_catalog::cli::NAME_SEPARATOR))
            })?;
        }
        if let Some(key) = shape {
            style.shape = FaceShape::from_key(key).ok_or_else(|| {
                let all: Vec<&str> = FaceShape::ALL.iter().map(|s| s.key()).collect();
                msg::not_a_face_shape(key, all.join(khor_catalog::cli::NAME_SEPARATOR))
            })?;
        }
        self.set_avatar_style(&style)?;
        Ok(style)
    }

    /// This machine's face under a style it has **not** chosen — what
    /// each option on the settings screen is painted with.
    ///
    /// It goes through the same derivation and the same seed as every
    /// row's face. A settings screen that drew its own previews would be
    /// the second painter this whole module exists to prevent, and it
    /// would be the worst place for one: the preview is the only
    /// evidence anybody has before pressing, so a preview that lies is a
    /// choice made on a picture that never appears.
    pub fn face_under(&self, style: &AvatarStyle) -> Avatar {
        avatar(&AvatarSeed::of(&self.device), style)
    }

    /// A device's face: **its own reported palette, keyed by its id.**
    ///
    /// The two halves are deliberate. The seed is the device id, so a
    /// rename keeps the face and a theme switch keeps it too. The style
    /// is whatever *that* device reported, so a machine looks the same
    /// to everyone — before this, a style stored per viewer meant the
    /// same Mac was two different colors on two screens, **with neither
    /// side wrong** and nothing to report.
    ///
    /// A device that reported nothing (an older version, or one seen
    /// only through someone else's table) gets the factory default.
    /// **Unparseable reports get the same treatment**, which is a
    /// narrower rule than mandala's — there, a bad report fell back to
    /// the *viewer's* preference so that a broken report never read as
    /// "it just looks like that".
    ///
    /// **This line used to say "revisit when the settings batch lands".
    /// It landed (`Node::restyle`), and the answer is that nothing
    /// changes here** — the settings screen sets what *this* machine
    /// wears, which is the one style this machine may write. There still
    /// is no per-viewer style for somebody *else's* machine, so there is
    /// still nothing else to fall back to, and inventing one would undo
    /// the paragraph above it. Reaching mandala's rule would mean
    /// deciding that a viewer may dress other machines, which is a
    /// different feature and a worse one.
    pub fn face_of(&self, device: &DeviceInfo) -> Option<Avatar> {
        let seed = AvatarSeed::from_id_hex(&device.id)?;
        let style = device
            .style
            .as_deref()
            .and_then(AvatarStyle::from_json)
            .unwrap_or_default();
        Some(avatar(&seed, &style))
    }

    /// `KHOR_HOME` if set, else the home directory. The override exists
    /// for tests and dual-instance verification on one machine.
    pub fn root_from_env() -> PathBuf {
        std::env::var_os("KHOR_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::home_dir().unwrap_or_else(|| PathBuf::from(".")))
    }

    pub fn device(&self) -> DeviceId {
        self.device
    }

    /// The machine id, printable (public key, hex).
    pub fn device_str(&self) -> &str {
        &self.device_str
    }

    /// This machine's channel name.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn root(&self) -> &PathBuf {
        &self.root
    }

    pub(crate) fn writer_peer(&self) -> u64 {
        self.peer
    }

    pub(crate) fn secret_key(&self) -> &iroh::SecretKey {
        &self.key
    }

    pub(crate) fn relays(&self) -> &[String] {
        &self.relays
    }

    pub(crate) fn devices_loaded(&self) -> Result<Loaded<DeviceDoc>, String> {
        load(&devices_dir(&self.root), self.peer)
    }

    pub(crate) fn seen_loaded(&self) -> Result<Loaded<SeenDoc>, String> {
        load(&seen_dir(&self.root), self.peer)
    }

    pub(crate) fn pins_loaded(&self) -> Result<Loaded<PinDoc>, String> {
        load(&pins_dir(&self.root), self.peer)
    }

    /// Raises a session's seen watermark and persists it. The watermark
    /// travels the network (docs/NET.md): clear here, clear
    /// everywhere on the next sync.
    fn mark_seen(&self, session: &SessionId, at: i64) -> Result<(), String> {
        let loaded = self.seen_loaded()?;
        loaded.doc.mark(&session.0, at)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// Everyone in the network, this machine included.
    pub fn devices(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self.devices_loaded()?.doc.all())
    }

    /// name → (channel, home). Refuses unknown machines listing what the
    /// table has — "not found" alone sends people guessing.
    fn resolve(&self, machine: &str) -> Result<(String, DeviceId), String> {
        let ch = channel_of_machine(machine)
            .ok_or_else(|| msg::machine_name_not_pathable(format_args!("{machine:?}")))?;
        let all = self.devices()?;
        match all.iter().find(|d| d.name == ch) {
            Some(d) => Ok((ch, device_id_from_hex(&d.id)?)),
            None => {
                let names: Vec<&str> = all.iter().map(|d| d.name.as_str()).collect();
                Err(msg::no_such_machine(machine, names.join(khor_catalog::cli::NAME_SEPARATOR)))
            }
        }
    }

    pub(crate) fn channel_of_session(&self, id: &SessionId) -> Result<(String, DeviceId), String> {
        let Some(ch) = id.0.strip_prefix("chat/") else {
            return Err(msg::no_such_session(&id.0));
        };
        self.resolve(ch)
            .map_err(|e| msg::no_such_session_because(&id.0, e))
    }

    fn kinds(&self) -> [&dyn KindSurface; 3] {
        [&self.chat, &self.transfer, &self.live]
    }

    /// The list: one row per session, each answering the five questions.
    /// Local rows first; then rows other devices reported that this one
    /// cannot derive itself, freshest report winning a duplicate id.
    ///
    /// **Pinned rows lead, and the rest keep the order above.** Sorting
    /// here is what lets every face stay a painter: the CLI prints this
    /// order, the app renders it, and neither owns a comparison
    /// (docs/UX.md — the list never re-derives a judgment the library
    /// already made).
    pub fn sessions(&self) -> Result<Vec<SessionView>, String> {
        let pins = self.pins_loaded()?;
        let mut views: Vec<SessionView> = Vec::new();
        for k in self.kinds() {
            for row in k.rows(self)? {
                views.push(SessionView {
                    pinned: pins.doc.pinned(&row.id.0),
                    session: row,
                    source: None,
                });
            }
        }
        let mut have: std::collections::BTreeSet<String> =
            views.iter().map(|v| v.session.id.0.clone()).collect();
        let mut reported = self.cached_peer_rows();
        reported.sort_by_key(|(_, age, _)| *age);
        for (name, age, row) in reported {
            if have.insert(row.id.0.clone()) {
                views.push(SessionView {
                    pinned: pins.doc.pinned(&row.id.0),
                    session: row,
                    source: Some((name, age)),
                });
            }
        }
        // Stable: within each group the order above survives untouched.
        views.sort_by_key(|v| !v.pinned);
        Ok(views)
    }

    /// The list, arranged the way this mode arranges it — the call both
    /// faces make, so `khor sessions --by state` and the app's state
    /// view are the same order by construction, not by agreement.
    ///
    /// Machine names come from the device table here, once, rather than
    /// being looked up per row.
    pub fn sessions_arranged(&self, mode: list::Arrange) -> Result<Vec<list::Arranged>, String> {
        let by_id: std::collections::BTreeMap<String, String> = self
            .devices()?
            .into_iter()
            .map(|d| (d.id, d.name))
            .collect();
        let views = self.sessions()?;
        Ok(list::arrange(views, mode, &|home: &DeviceId| {
            by_id.get(&home.hex()).cloned()
        }))
    }

    /// Pins or unpins a session, for everyone.
    ///
    /// The pin lands in a replicated document, not in this machine's
    /// preferences (`khor_sync::pins` says why), so this is the one call
    /// behind both the CLI verb and the app's button — CLI and GUI stay
    /// equivalent because they are the same function (docs/KHOR.md).
    ///
    /// **Any id may be pinned, including one no row answers to today.**
    /// Refusing unknown ids would mean a row that is merely offline
    /// cannot be pinned from the device looking at it, and a session
    /// that ends would make its own pin an error later. An id with no
    /// row simply paints nothing.
    pub fn pin_session(&self, id: &SessionId, on: bool) -> Result<(), String> {
        let loaded = self.pins_loaded()?;
        loaded.doc.set(&id.0, on)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        self.emit_row_of(id)
    }

    /// Pins or unpins a machine, for everyone. Named the way people
    /// name machines; the pin itself is keyed by device id.
    pub fn pin_device(&self, machine: &str, on: bool) -> Result<(), String> {
        let (_, home) = self.resolve(machine)?;
        let loaded = self.devices_loaded()?;
        loaded.doc.set_pinned(&home.hex(), on)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// The rows only this device can derive — what `Request::Sessions`
    /// answers with. Chat rows are excluded: every device derives those
    /// from the CRDT itself.
    pub(crate) fn reportable_rows(&self) -> Result<Vec<Session>, String> {
        let mut rows = Vec::new();
        for k in self.kinds() {
            if k.reportable() {
                rows.extend(k.rows(self)?);
            }
        }
        Ok(rows)
    }

    // ── what other devices last reported (docs/SESSION.md 离线) ──

    fn peers_dir(&self) -> PathBuf {
        self.root.join(".khor").join("peers")
    }

    /// Remembers a device's reported rows, stamped with now — age at
    /// read time is the "多久没联系上" the UI shows.
    pub(crate) fn cache_peer_rows(
        &self,
        device_id: &str,
        name: &str,
        rows: &[Session],
    ) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        fs::create_dir_all(self.peers_dir()).map_err(msg::cant_make_peers_dir)?;
        let report = PeerReport { at_ms: now_ms(), name: name.to_owned(), rows: rows.to_vec() };
        link::write_private(
            &self.peers_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// Every cached report, flattened to (reporter, age, row). Unreadable
    /// caches are skipped — a stale or missing report only means older
    /// information, never an error.
    fn cached_peer_rows(&self) -> Vec<(String, u64, Session)> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(self.peers_dir()) else {
            return out;
        };
        for e in rd.flatten() {
            let Ok(text) = fs::read_to_string(e.path()) else {
                continue;
            };
            let Ok(report) = serde_json::from_str::<PeerReport>(&text) else {
                continue;
            };
            let age = now_ms().saturating_sub(report.at_ms);
            for row in report.rows {
                out.push((report.name.clone(), age, row));
            }
        }
        out
    }

    // ── what machines last reported about themselves (vitals) ──

    fn vitals_dir(&self) -> PathBuf {
        self.root.join(".khor").join("vitals")
    }

    /// Remembers a machine's reading, stamped with now.
    ///
    /// **Its own file, not a field on the row report.** Rows and vitals
    /// are fetched by two requests that fail independently, and they age
    /// independently too — a shared record would have one round's failure
    /// erase the other's last known answer, and a shared timestamp would
    /// dress a stale reading in the rows' freshness.
    pub(crate) fn cache_peer_vitals(
        &self,
        device_id: &str,
        v: &khor_core::Vitals,
    ) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        fs::create_dir_all(self.vitals_dir()).map_err(msg::cant_make_vitals_dir)?;
        let report = VitalsReport { at_ms: now_ms(), vitals: *v };
        link::write_private(
            &self.vitals_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// What a machine last said about itself, and how old that is.
    ///
    /// **This machine answers by sampling, everyone else from cache**, and
    /// the age is what tells them apart at the far end: zero means the
    /// reading was taken to answer this call. A machine nobody has heard
    /// from yet is `None` — a third state, distinct from an old reading,
    /// because "not asked yet" and "asked an hour ago" are different
    /// things to be told (docs/SESSION.md 离线).
    pub fn vitals_of(&self, device_id: &str) -> Option<(khor_core::Vitals, u64)> {
        if device_id == self.device_str() {
            return Some((vitals::sample(&self.root), 0));
        }
        let text = fs::read_to_string(self.vitals_dir().join(device_id)).ok()?;
        let report: VitalsReport = serde_json::from_str(&text).ok()?;
        Some((report.vitals, now_ms().saturating_sub(report.at_ms)))
    }

    /// Offers a file to a machine's window: the summary (name, size,
    /// digest) travels the CRDT to everyone; the bytes wait here for the
    /// far side's approval. Returns the transfer session id.
    pub fn send(&self, machine: &str, path: &Path) -> Result<SessionId, String> {
        let (channel, home) = self.resolve(machine)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| msg::not_a_file_path(path.display()))?;
        let (digest, size) = transfer::digest_file(path)?;
        let f = FileRef { name: name.clone(), size: size as i64, digest: digest.clone() };
        let told = self.chat.send_files(&channel, home, &[f])?;
        transfer::save_offer(
            &self.root,
            &digest,
            &transfer::Offer {
                path: path
                    .canonicalize()
                    .map_err(msg::cant_canonicalize)?,
                name,
                size,
                started: false,
                done: false,
            },
        )?;
        self.mark_seen(&told.row.id, told.at)?;
        self.emit(NodeEvent::Event(told.event));
        self.emit(NodeEvent::Row(told.row));
        Ok(TransferKind::session_id(&told.msg_id))
    }

    /// The file message behind `transfer/<msg_id>`, with its channel.
    pub(crate) fn find_transfer(&self, msg_id: &str) -> Result<(String, Message), String> {
        for d in self.devices()? {
            let log = self.chat.log(&d.name)?;
            if let Some(m) = log
                .messages
                .into_iter()
                .find(|m| m.id == msg_id && matches!(m.body, MsgBody::Files(_)))
            {
                return Ok((d.name, m));
            }
        }
        Err(msg::no_such_transfer(format_args!("transfer/{msg_id}")))
    }

    /// Subscribes to everything that happens. Events push; the returned
    /// receiver ends when the node drops.
    pub fn watch(&self) -> mpsc::Receiver<NodeEvent> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    /// Tells a machine's window a line. Returns the message id.
    pub fn tell(&self, machine: &str, text: &str) -> Result<String, String> {
        let (channel, home) = self.resolve(machine)?;
        let told = self.chat.tell(&channel, home, text)?;
        // Telling implies looking: own words never count as unread — on
        // any device, once the watermark syncs.
        self.mark_seen(&told.row.id, told.at)?;
        self.emit(NodeEvent::Event(told.event));
        self.emit(NodeEvent::Row(told.row));
        Ok(told.msg_id)
    }

    /// A channel's readout, oldest first.
    pub fn log(&self, machine: &str) -> Result<chat::ChannelLog, String> {
        let (channel, _) = self.resolve(machine)?;
        self.chat.log(&channel)
    }

    /// Marks a session seen; unread drops to zero — here and, once the
    /// watermark syncs, on every device. Seen is the one action every
    /// kind answers the same way: the kind names the watermark, the node
    /// persists it and repaints the row.
    pub fn seen(&self, id: &SessionId) -> Result<(), String> {
        for k in self.kinds() {
            if !k.claims(self, id) {
                continue;
            }
            let at = k.seen_at(self, id)?;
            self.mark_seen(id, at)?;
            if let Some(row) = k.rows(self)?.into_iter().find(|r| r.id == *id) {
                self.emit(NodeEvent::Row(row));
            }
            return Ok(());
        }
        // A row another device reported: its stamp is the watermark, and
        // the mark replicates — the home's own row clears on next sync.
        if let Some(v) = self.sessions()?.into_iter().find(|v| v.session.id == *id) {
            return self.mark_seen(id, v.session.state.at.0 as i64);
        }
        Err(msg::no_such_session(&id.0))
    }

    /// Closes a session — the wind-down is the kind's (docs/SESSION.md
    /// 动作): a device chat deletes payloads it received, a transfer
    /// deletes just its own payload, a live session is terminated and
    /// forgotten.
    pub fn close(&self, id: &SessionId) -> Result<(), String> {
        for k in self.kinds() {
            if !k.claims(self, id) {
                continue;
            }
            k.close(self, id)?;
            match k.rows(self)?.into_iter().find(|r| r.id == *id) {
                // Some kinds keep a row after close (a transfer's summary
                // stays in the CRDT; the row falls back to 待批).
                Some(row) => self.emit(NodeEvent::Row(row)),
                None => self.emit(NodeEvent::Closed(id.clone())),
            }
            return Ok(());
        }
        if let Some(v) = self.sessions()?.into_iter().find(|v| v.session.id == *id) {
            if let Some((name, _)) = v.source {
                return Err(msg::remote_close_not_yet(name));
            }
            // A local row no kind claims is a discovered one: khor found
            // it by reading a vendor's files and never started it, so it
            // has nothing to wind down and no business killing someone
            // else's process. Saying so by name matters — "no such
            // session" about a row the user is looking at reads as a bug
            // in khor rather than a boundary of it.
            return Err(msg::not_khors_to_close(&id.0));
        }
        Err(msg::no_such_session(&id.0))
    }

    // ── the live kind's doors (临时 sessions and hooks) ─────

    /// What khor can honestly say about a session it starts itself.
    ///
    /// A wrapped command is [`khor_core::category::SHELL`] — the user
    /// started it, and that is the whole answer. **A tui gets nothing.**
    /// Khor has the command line, and reading a vendor out of it would
    /// be wrong for an alias, a wrapper or `npx` while looking right,
    /// which is exactly the guess `Session::category` exists to prevent.
    ///
    /// So `khor run --tui -- claude` is an uncategorised row until the
    /// vendor speaks for itself — its hook, or its own files on the next
    /// sweep, both of which fill the gap in
    /// (`LiveKind::learn_category`). Until then it sits in the category
    /// view's "could not tell" group, which is the honest place for it
    /// and not a neighbour's group.
    fn category_of_started(kind: &str) -> Option<&'static str> {
        (kind == khor_core::kind::SHELL).then_some(khor_core::category::SHELL)
    }

    /// Opens a 临时 live session — it lives and dies with the process
    /// `run_ephemeral` runs. The persistent host (`open`) is a coming
    /// batch.
    pub fn open_ephemeral(&self, kind: &str, title: &str) -> Result<SessionId, String> {
        let leaf = link::fresh_leaf()?;
        let id = SessionId(format!("{kind}/{leaf}"));
        self.live.register(&id, kind, title, None, Self::category_of_started(kind))?;
        Ok(id)
    }

    /// Runs the command as the session's process, blocking until it
    /// ends. Returns the exit code, which is also the row's ending.
    pub fn run_ephemeral(&self, id: &SessionId, cmd: &[String]) -> Result<i32, String> {
        live::run_wrapped(&self.live, id, cmd)
    }

    /// Opens a 持久 session: registers it and hands it to a detached
    /// host that owns the PTY (docs/SESSION.md 寿命). Returns once the
    /// host is reachable; attaching is the caller's move.
    pub fn open_persistent(
        &self,
        kind: &str,
        title: &str,
        cmd: &[String],
        size: (u16, u16),
    ) -> Result<SessionId, String> {
        let leaf = link::fresh_leaf()?;
        let id = SessionId(format!("{kind}/{leaf}"));
        self.live.register(&id, kind, title, None, Self::category_of_started(kind))?;
        let dir = self
            .live
            .dir_of(&id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host(&dir, &id, cmd, size)?;
        Ok(id)
    }

    /// The registry dir behind a live session id, for attach clients.
    pub fn session_dir(&self, id: &SessionId) -> Option<PathBuf> {
        self.live.dir_of(id).filter(|d| d.exists())
    }

    /// A process reporting its own word — the hook door. 失败 is refused
    /// by the kind: it derives from the exit code, never from a claim.
    pub fn report_state(&self, id: &SessionId, word: khor_core::State) -> Result<(), String> {
        self.live.report(id, word)
    }

    /// One Claude Code hook payload in, the mapped session move out.
    pub fn claude_hook(&self, payload: &str) -> Result<adaptor::claude::Hooked, String> {
        adaptor::claude::hook(&self.live, payload)
    }

    /// Live agent sessions the last sweep could see but could not read
    /// — a vendor changed a file layout khor is reading (docs/HOOKS.md
    /// 适配器过时). Zero rows and a zero count is an idle machine; zero
    /// rows and a non-zero count is khor being out of date, and nothing
    /// but this number tells those apart.
    pub fn unreadable_sessions(&self) -> usize {
        self.live.unreadable_sessions()
    }

    /// What claude's settings say about khor's hooks. Reads only.
    ///
    /// Rooted the same way discovery is (`adaptor::vendor_home`), which
    /// is what makes this safe to run: a node opened on a temp home
    /// looks at that home's `.claude`, never the user's. The whole
    /// verification for this feature is made of that.
    pub fn hooks_report(&self) -> Result<adaptor::claude::HookReport, String> {
        adaptor::claude::hooks_report(&adaptor::vendor_home(&self.root))
    }

    /// Adds khor's hooks to claude's settings, leaving the rest of that
    /// file alone. See `adaptor::claude::install_hooks`.
    pub fn install_hooks(&self) -> Result<adaptor::claude::HookInstall, String> {
        adaptor::claude::install_hooks(&adaptor::vendor_home(&self.root))
    }

    /// Takes them back out again, leaving the rest of that file alone.
    /// See `adaptor::claude::uninstall_hooks`.
    ///
    /// Rooted through the same `vendor_home` as the other two, which is
    /// what keeps this verifiable: a node opened on a temp home can only
    /// ever edit that home's `.claude`.
    pub fn uninstall_hooks(&self) -> Result<adaptor::claude::HookUninstall, String> {
        adaptor::claude::uninstall_hooks(&adaptor::vendor_home(&self.root))
    }

    /// Re-derives one session's row and pushes it to watchers.
    pub(crate) fn emit_row_of(&self, id: &SessionId) -> Result<(), String> {
        for k in self.kinds() {
            if !k.claims(self, id) {
                continue;
            }
            if let Some(row) = k.rows(self)?.into_iter().find(|r| r.id == *id) {
                self.emit(NodeEvent::Row(row));
            }
            return Ok(());
        }
        Ok(())
    }

    fn emit(&self, event: NodeEvent) {
        // Dead receivers drop out here; send failure is disconnection,
        // not an error.
        self.subscribers
            .lock()
            .unwrap()
            .retain(|tx| tx.send(event.clone()).is_ok());
    }
}

// ── how each kind answers the surface ───────────────────────

impl KindSurface for ChatKind {
    fn claims(&self, node: &Node, id: &SessionId) -> bool {
        node.channel_of_session(id).is_ok()
    }

    fn rows(&self, node: &Node) -> Result<Vec<Session>, String> {
        let seen = node.seen_loaded()?;
        let mut out = Vec::new();
        for d in node.devices()? {
            let wm = seen.doc.watermark(&format!("chat/{}", d.name));
            out.push(self.row(&d.name, device_id_from_hex(&d.id)?, wm)?);
        }
        Ok(out)
    }

    /// Chat rows never travel as reports: the document replicates, so
    /// every device derives its own.
    fn reportable(&self) -> bool {
        false
    }

    /// The max on the senders' clocks — never the local clock (a sender
    /// running ahead would stay unread after being looked at), never the
    /// list-order last (a concurrent merge can land mid-list).
    fn seen_at(&self, node: &Node, id: &SessionId) -> Result<i64, String> {
        let (channel, _) = node.channel_of_session(id)?;
        Ok(self.log(&channel)?.messages.iter().map(|m| m.at).max().unwrap_or(0))
    }

    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String> {
        let (channel, _) = node.channel_of_session(id)?;
        self.close_channel(&channel)
    }
}

impl KindSurface for TransferKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        id.0.starts_with("transfer/")
    }

    fn rows(&self, node: &Node) -> Result<Vec<Session>, String> {
        let seen = node.seen_loaded()?;
        let mut out = Vec::new();
        for d in node.devices()? {
            let dir = channel_dir(node.root(), &d.name)
                .ok_or_else(|| msg::bad_channel_name(format_args!("{:?}", d.name)))?;
            let msgs = node.chat.log(&d.name)?.messages;
            out.extend(self.rows(&d.name, &dir, &msgs, |sid| seen.doc.watermark(sid)));
        }
        Ok(out)
    }

    fn reportable(&self) -> bool {
        true
    }

    /// What "looked at" covers is the landed payload; before it lands
    /// there is nothing unread to clear (待批 clears by answering, not
    /// by looking — docs/UX.md 角标).
    fn seen_at(&self, node: &Node, id: &SessionId) -> Result<i64, String> {
        let (files, dir) = self.files_of(node, id)?;
        Ok(files
            .iter()
            .filter_map(|f| transfer::mtime_ms(&transfer::payload_path(&dir, f)))
            .max()
            .unwrap_or(0))
    }

    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String> {
        let (files, dir) = self.files_of(node, id)?;
        for f in &files {
            self.close_payload(&dir, f)?;
        }
        Ok(())
    }
}

impl TransferKind {
    /// The file refs and channel dir behind `transfer/<msg_id>`.
    fn files_of(&self, node: &Node, id: &SessionId) -> Result<(Vec<FileRef>, PathBuf), String> {
        let Some(msg_id) = id.0.strip_prefix("transfer/") else {
            return Err(msg::no_such_transfer(&id.0));
        };
        let (channel, m) = node.find_transfer(msg_id)?;
        let dir = channel_dir(node.root(), &channel)
            .ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))?;
        let MsgBody::Files(files) = m.body else {
            return Err(msg::no_such_transfer(&id.0));
        };
        Ok((files, dir))
    }
}

impl KindSurface for LiveKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        self.claims(id)
    }

    fn rows(&self, node: &Node) -> Result<Vec<Session>, String> {
        let seen = node.seen_loaded()?;
        Ok(self.rows(|sid| seen.doc.watermark(sid)))
    }

    /// Live state never syncs (docs/NET.md) — only home can derive these
    /// rows, so they travel as reports.
    fn reportable(&self) -> bool {
        true
    }

    fn seen_at(&self, _node: &Node, id: &SessionId) -> Result<i64, String> {
        self.stamp(id)
    }

    fn close(&self, _node: &Node, id: &SessionId) -> Result<(), String> {
        self.close_session(id)
    }
}

/// One device's last session report, as cached on disk.
#[derive(serde::Serialize, serde::Deserialize)]
struct PeerReport {
    at_ms: u64,
    name: String,
    rows: Vec<Session>,
}

/// One device's last reading, as cached on disk. Separate from
/// [`PeerReport`] on purpose — [`Node::cache_peer_vitals`] says why.
#[derive(serde::Serialize, serde::Deserialize)]
struct VitalsReport {
    at_ms: u64,
    vitals: khor_core::Vitals,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// hex → the 32 key bytes. The table stores ids as hex; iroh wants bytes.
pub(crate) fn device_id_from_hex(s: &str) -> Result<DeviceId, String> {
    let s = s.trim();
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(msg::not_a_machine_id(format_args!("{s:?}")));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).unwrap() as u8;
        let lo = (chunk[1] as char).to_digit(16).unwrap() as u8;
        out[i] = (hi << 4) | lo;
    }
    Ok(DeviceId(out))
}

#[cfg(test)]
mod tests;
