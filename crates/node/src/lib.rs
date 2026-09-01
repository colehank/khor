//! One device's node: identity, the session surface (docs/SESSION.md),
//! and the live link to the rest of the network (docs/NET.md).
//!
//! The kind trait ([`KindSurface`]) was cut only when the third kind
//! (live) arrived — the first two shared nothing but "produce rows",
//! and a trait cut from that would have frozen an accident. What three
//! real implementations actually share: produce rows, claim an id,
//! answer what "looked at now" means, and close.

pub mod adaptor;
pub mod cagent;
pub mod codexagent;
pub mod chat;
pub mod day;
pub mod files;
pub mod gui_host;
pub mod host;
pub mod ipc;
pub mod keeper;
pub mod link;
pub mod list;
pub mod mcp;
pub mod live;
pub mod place;
pub mod proto;
pub mod quota;
pub mod reactor;
pub mod satellite;
pub mod transfer;
pub mod tunnel;
pub mod usage;
pub mod vitals;

pub use khor_core::avatar::{
    avatar, preset, Avatar, AvatarSeed, AvatarStyle, FaceShape, Palette, Preset, Variant, PRESETS,
};
pub use khor_core::{kind, Fill, Hop, Session, SessionId, State, Tokens, Usage, UsageDay, Vitals};
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
use khor_sync::agents::{agents_dir, AgentDoc};
/// How one registered ACP agent starts (`khor_sync::agents::Spec`).
/// Re-exported because `Node::agents` hands it out, and a type a public
/// method returns that callers cannot name is a method they cannot use.
pub use khor_sync::agents::Spec as AgentSpec;
/// One conversation an agent said it has, as khor remembers it
/// (`khor_sync::agents::Seen`).
pub use khor_sync::agents::{Answer as AgentAnswer, Seen as AgentSeen};
/// The shipped snapshot of the ACP registry (`khor_sync::agents::registry`).
/// Re-exported here for the same reason `AgentSpec` is: the CLI talks to
/// the node, and a second path into the sync crate would be a second
/// place to keep in step.
pub use khor_sync::agents::registry as agent_registry;
/// One recorded utterance as the protocol update a face paints
/// (`cagent::replayed`).
///
/// **Exported because there were three copies of this translation and
/// the bug was that only one of them was ever fixed.** The live stream,
/// the 垫片's `session/load` replay, and `gui-core`'s reading of a
/// discovered session's transcript each had their own answer; the live
/// one grew tool titles, kinds, statuses and diffs, and the other two
/// silently kept painting a line that said `Bash`. A person looking at
/// an existing conversation sees almost only replayed lines, so the
/// improvement was invisible where it mattered most.
pub use crate::cagent::replayed as replayed_update;

/// What asking an agent for its sessions answered
/// (`khor_acp::Listing`), and one of them.
///
/// Re-exported for the reason `AgentSpec` and `agent_registry` are: the
/// CLI and the faces talk to the node, and a second path into the
/// protocol crate is a second place to keep in step.
pub use khor_acp::{Listing, Recorded};
use khor_sync::dirpins::{dirpins_dir, DirPinDoc};
use khor_sync::webpins::{webpins_dir, WebPinDoc};
/// What sort of place a recorded visit was to
/// (`khor_sync::recents::Kind`). Re-exported because `Node::recents`
/// takes and reports one, and a type a public method demands that
/// callers cannot name is a method they cannot call.
pub use khor_sync::recents::Kind as PlaceKind;
use khor_sync::recents::{recents_dir, RecentDoc};
use khor_sync::pins::{pins_dir, PinDoc};
use khor_sync::seen::{seen_dir, SeenDoc};
use khor_sync::store::{load, Loaded};
use khor_sync::titles::{titles_dir, TitleDoc};

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
    /// Old news: dead, looked at, and dead for longer than
    /// [`STALE_AFTER_MS`] — the default list folds it into a tail
    /// count (`is_stale` has the full judgment). Derived here so every
    /// face folds the same rows; **nothing is deleted** — the row is in
    /// this very list, search still reads it, `--all` prints it.
    pub stale: bool,
    /// The word has outlived its believability: a *live* word (忙碌/
    /// 待批/中断) that nobody can vouch for any more
    /// ([`khor_core::Session::unbacked`]) and that has not moved in
    /// [`QUIET_AFTER_MS`]. Faces grey the row and say how long the
    /// silence is — the word itself never changes (`is_quiet` has the
    /// judgment; docs/SESSION.md 离线不是第七个词, this is that axis's
    /// second case: the machine is reachable, the *signal* is not).
    pub quiet: bool,
}

/// How long a dead, looked-at row stays on the default list before it
/// folds into the tail. Tunable, but only here: a second copy of this
/// number is how two faces come to disagree about what "old" means.
///
/// 48 hours: long enough that "the probe I ran yesterday failed" is
/// still in sight when someone comes back after a weekend day, short
/// enough that the first screen is not half graveyard.
pub const STALE_AFTER_MS: u64 = 48 * 60 * 60 * 1000;

/// Whether a row is old news the default list should fold away.
///
/// Three gates, all required (领队 ruling, 退场批):
/// - **dead** ([`khor_core::Session::dead`]): nothing will move the row
///   again on its own. 待批/忙碌/中断 rows are never dead by
///   construction, so they never fold.
/// - **looked at** (`unread == 0`): unread is a debt, and debts do not
///   expire on their own.
/// - **dead for longer than [`STALE_AFTER_MS`]**, measured from the
///   row's own stamp — for a dead row that is the moment it died (the
///   recorded exit, the crash, the settled transfer), so no new field
///   was invented to answer "dead since when".
///
/// A pinned row never folds: pinning is the person saying this one
/// matters, and a judgment made by hand outranks one made by a clock.
fn is_stale(session: &Session, pinned: bool, now_ms: u64) -> bool {
    session.dead
        && session.unread == 0
        && !pinned
        && now_ms.saturating_sub(session.state.at.0) >= STALE_AFTER_MS
}

/// How long an unvouched *live* word stays believable (词的保鲜期批).
/// One constant, one place — the same fact greys the CLI line and the
/// GUI row.
///
/// 24 hours: a hook-observed agent speaks every turn (the hooks fire on
/// every prompt, answer and stop), so a 忙碌 that has said nothing for
/// a whole day is not a day-long turn — it is a signal that stopped
/// (the vendor file gone, the process unfindable). Shorter would grey
/// a long overnight run that is merely quiet between turns; a real
/// case on this machine sat at 忙碌 for five days.
pub const QUIET_AFTER_MS: u64 = 24 * 60 * 60 * 1000;

/// Whether a row's word has outlived its believability.
///
/// Three gates, all required — and the middle one is what keeps this
/// from being an age-only sweep:
/// - **a live word**: 忙碌/待批/中断 claim something is happening *now*,
///   which is the only claim silence can falsify. 完成/失败/空闲 state
///   finished facts and stay honest at any age.
/// - **unvouched** ([`khor_core::Session::unbacked`]): rows with a real
///   verdict available — a pid that answers, a vendor file being read,
///   a listing multiplexer — never fade, however old their stamp;
///   their words are re-derived, not remembered.
/// - **silent past [`QUIET_AFTER_MS`]**, from the word's own stamp.
///
/// The word is left exactly as it was: this feeds greying and a "N 天
/// 没消息" note, never a seventh word (docs/SESSION.md 离线).
fn is_quiet(session: &Session, now_ms: u64) -> bool {
    matches!(session.state.state, State::Busy | State::Blocked | State::Errored)
        && session.unbacked
        && now_ms.saturating_sub(session.state.at.0) >= QUIET_AFTER_MS
}

/// One place this person has been (`khor_sync::recents`). The machine
/// is its id and not its name: the record outlives a rename, and the
/// name is looked up at the surface by whoever is painting, the way a
/// pin's is.
#[derive(Debug, Clone)]
pub struct RecentPlace {
    /// Whose disk or whose network — the device hex.
    pub machine: String,
    /// The absolute path, or the URL.
    pub place: String,
    /// When it was last visited, Unix ms. Never 0: that is the
    /// tombstone and those are not on this list.
    pub at_ms: i64,
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
    /// What the agents on this machine have spent. Held on the node
    /// rather than built per call, because it keeps the bookkeeping that
    /// makes a second reading cheap (`usage::Meters::tally` has the
    /// measurement). Shared rather than owned for two reasons: the async
    /// side can move a reading onto a blocking thread without leaving the
    /// bookkeeping behind, and every node opened on this home in this
    /// process gets the same one (`usage::meters_for`).
    usage: Arc<usage::Meters>,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<NodeEvent>>>>,
    /// Serializes sync pumps within one process: two concurrent pumps
    /// share block stores and could collide on sequence numbers. The
    /// ticker skips when busy; an explicit sync waits its turn.
    pub(crate) sync_gate: tokio::sync::Mutex<()>,
    /// The borrow proxies this serve hosts (docs/NET.md 借网), keyed by
    /// their session id. Each is a task holding a local proxy port and a
    /// lease to the exit; the serve reaps one once its session row is
    /// gone (a `close` from any process removes the row), which is also
    /// what frees the port. Empty on a node that is not the resident
    /// serve.
    pub(crate) borrows:
        Arc<tokio::sync::Mutex<std::collections::HashMap<SessionId, tokio::task::JoinHandle<()>>>>,
    /// The pipes this serve holds to **other machines'** session hosts
    /// (远程 attach), keyed by the session's id over there. One pipe per
    /// session, however many faces attach through it: a terminal has one
    /// destination, and a second listener for the same one would be a
    /// second port for no reason. Empty on a node that is not the
    /// resident serve.
    /// **The far port is part of the key, not just the value.** A pipe
    /// is built for one fixed target (`tunnel::serve_fixed`), and the
    /// far host's port changes whenever it is stood up again — a host
    /// that died and got re-bridged, a serve restarted by an upgrade.
    /// Remembering the pipe without remembering what it points at is
    /// how every attach after such a restart piped into a closed port
    /// and reported 读不到帧: Connection reset — a sentence about a
    /// socket, for a session sitting right there. Measured on turing.
    pub(crate) reaches: Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<
                SessionId,
                (std::net::SocketAddr, u16, tokio::task::JoinHandle<()>),
            >,
        >,
    >,
}

/// This binary's own path, for the re-exec convention (`_host`,
/// `_ghost`, `_cagent`).
///
/// **An upgrade deletes the file this process is running from.** Linux
/// then reports `/proc/self/exe` as `<path> (deleted)`, and spawning
/// that path fails with `ENOENT` — which surfaces as 宿主起不来 about a
/// session that was never the problem.
///
/// Measured, and not a corner: a cluster mounts one home on every
/// machine, so `~/.local/bin/khor` is one file for all of them.
/// Installing khor on the second box replaced the binary the *first*
/// box's resident serve was running, and from that moment that serve
/// could not open a single session — while answering everything else
/// perfectly, because a running process keeps its own inode.
///
/// After an upgrade the right thing to spawn is the **new** binary at
/// the same path: that is what the user just installed, and the
/// alternative (a deleted inode) cannot be spawned at all. When nothing
/// is there any more, the error says the binary moved rather than
/// blaming the session.
pub fn self_exe() -> Result<PathBuf, String> {
    let raw = std::env::current_exe().map_err(msg::cant_find_self)?;
    let text = raw.to_string_lossy();
    let Some(live) = text.strip_suffix(" (deleted)") else {
        return Ok(raw);
    };
    let live = PathBuf::from(live);
    if live.exists() {
        return Ok(live);
    }
    Err(msg::self_moved(live.display()))
}

/// One store, one machine.
///
/// The identity key **is** the machine on this network, so two machines
/// opening one store are one device wearing two live nodes: two
/// residents writing one registry, two claims to the same rows, and a
/// device list that cannot say which of them anything is on. Nothing
/// downstream can detect that — every file looks fine.
///
/// It is not a hypothetical. A lab cluster puts every machine's home on
/// one NFS export — measured on two boxes that mount the same one, so
/// `~/.khor` is the *same directory* on both. That is the ordinary case
/// there, not the exotic one.
///
/// Stamped with the **hostname**, never `KHOR_NAME`: the name is a
/// label the user may change, and renaming your machine must not lock
/// you out of your own store. Two homes on one machine (the test
/// fleet's three) stamp the same host and none of them refuse.
fn claim_store(root: &std::path::Path) -> Result<(), String> {
    let dir = root.join(".khor");
    let stamp = dir.join("owner");
    let here = gethostname::gethostname().to_string_lossy().to_string();
    match std::fs::read_to_string(&stamp) {
        Ok(owner) => {
            let owner = owner.trim();
            if !owner.is_empty() && owner != here {
                return Err(msg::store_belongs_to(owner, &here));
            }
            Ok(())
        }
        Err(_) => {
            // Written on the way in, before the key exists: a store that
            // gets a key without a stamp is a store the next machine
            // cannot tell apart from its own.
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            std::fs::write(&stamp, &here).map_err(|e| e.to_string())
        }
    }
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
        claim_store(&root)?;
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
        // Rooted through the same `vendor_home` as discovery and the
        // hooks: a node opened on a temp home reads that home's agents,
        // never the real user's. Every verification of this feature is
        // made of that.
        //
        // **Shared across every node on this home in this process**, and
        // that is load-bearing rather than tidy: the GUI's data layer
        // opens a node per call, so a registry built here per node would
        // be a cold one every time — and cold is eighteen seconds
        // (`usage::meters_for`).
        let usage = usage::meters_for(&adaptor::vendor_home(&root));
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
            usage,
            subscribers: Arc::new(Mutex::new(Vec::new())),
            sync_gate: tokio::sync::Mutex::new(()),
            borrows: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            reaches: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
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

    pub(crate) fn dirpins_loaded(&self) -> Result<Loaded<DirPinDoc>, String> {
        load(&dirpins_dir(&self.root), self.peer)
    }

    pub(crate) fn webpins_loaded(&self) -> Result<Loaded<WebPinDoc>, String> {
        load(&webpins_dir(&self.root), self.peer)
    }

    pub(crate) fn recents_loaded(&self) -> Result<Loaded<RecentDoc>, String> {
        load(&recents_dir(&self.root), self.peer)
    }

    pub(crate) fn agents_loaded(&self) -> Result<Loaded<AgentDoc>, String> {
        load(&agents_dir(&self.root), self.peer)
    }

    pub(crate) fn titles_loaded(&self) -> Result<Loaded<TitleDoc>, String> {
        load(&titles_dir(&self.root), self.peer)
    }

    /// Every ACP agent this person has named, by name (`khor_sync::agents`).
    pub fn agents(&self) -> Result<Vec<(String, AgentSpec)>, String> {
        Ok(self.agents_loaded()?.doc.all())
    }

    /// One of them, or `None` for a name nobody registered.
    pub fn agent(&self, name: &str) -> Result<Option<AgentSpec>, String> {
        Ok(self.agents_loaded()?.doc.get(name))
    }

    /// What sessions a registered agent says it has (`session/list`).
    ///
    /// **This is the only discovery path khor has for an agent it did
    /// not write a reader for.** khor finds claude's and codex's
    /// sessions by reading their files on disk; an agent out of the ACP
    /// registry keeps its records wherever and however it likes, so
    /// asking it is the whole of what can be known.
    ///
    /// It is **not** something every agent can answer. The protocol
    /// makes only `session/new`, `session/prompt`, `session/cancel` and
    /// `session/update` mandatory; `session/list` sits behind a
    /// capability, and the registry's own probe on 2026-08-30 had 20 of
    /// 32 agents implement it. [`khor_acp::Listing::Cannot`] is that
    /// answer, and it is deliberately not the same value as an empty
    /// list — **「it cannot tell you」and「it has none」are different
    /// facts**, and a caller that collapsed them would report a
    /// perfectly busy agent as fresh.
    ///
    /// Starts the agent, asks, and stops it. No session is opened: the
    /// protocol allows the question straight after `initialize`, and
    /// opening one to ask would leave a conversation nobody wanted
    /// behind — which on a metered agent is not free.
    pub fn agent_sessions(
        &self,
        name: &str,
        cwd: Option<std::path::PathBuf>,
    ) -> Result<khor_acp::Listing, String> {
        let spec = self.agent_or_refuse(name)?;
        let launch = Self::agent_launch(&spec)?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?
            .block_on(khor_acp::list(&launch, cwd))
            .map_err(|r| r.said().to_owned())
    }

    /// The registration behind a name, or a refusal that **names what
    /// is registered**.
    ///
    /// One function because the CLI and the wizard have to refuse the
    /// same way, and a judgment written twice grows two (docs/KHOR.md:
    /// the two faces are equivalent because they are the same call).
    /// The list is in the sentence for `no_such_machine`'s reason — the
    /// commonest way to be here is a typo, and the real names answer it
    /// without a second command.
    ///
    /// An empty registry gets its own sentence rather than the list
    /// one with nothing after the colon: "you have not named any yet"
    /// and "not that one" are different situations with different next
    /// steps, and 「登记过的有: 」 trailing into nothing is not a
    /// sentence.
    pub fn agent_or_refuse(&self, name: &str) -> Result<AgentSpec, String> {
        let loaded = self.agents_loaded()?;
        if let Some(spec) = loaded.doc.get(name) {
            return Ok(spec);
        }
        let known = loaded.doc.all();
        if known.is_empty() {
            return Err(msg::NO_AGENTS_REGISTERED.into());
        }
        let names: Vec<&str> = known.iter().map(|(n, _)| n.as_str()).collect();
        Err(msg::no_such_agent(name, names.join(khor_catalog::cli::NAME_SEPARATOR)))
    }

    /// Names khor answers to itself, which nobody may register on top
    /// of.
    ///
    /// **This is not tidiness — it is the one way the open registry can
    /// break an existing judgment.** A registration's name becomes its
    /// rows' [`khor_core::Session::category`], and that field means the
    /// same thing in [`khor_core::UsageDay`]: an agent a person called
    /// `claude` would file its sessions into claude's group, have its
    /// row preview looked up in claude's transcripts, and — the
    /// expensive one — appear to be covered by the `claude` line in the
    /// spending panel, which counts only what the claude adaptor read.
    /// That is a confident wrong answer in the exact place
    /// `Session::category` exists to prevent one.
    ///
    /// Enumerated rather than derived. A rule ("no vendor names") would
    /// need a list of vendors to be a rule at all, and the list is what
    /// this is.
    pub const RESERVED_AGENT_NAMES: &'static [&'static str] = &[
        adaptor::claude::VENDOR,
        adaptor::codex::VENDOR,
        khor_core::category::KHOR,
        khor_core::category::SHELL,
    ];

    /// Registers an agent under `name`, replacing any registration
    /// under the same one.
    ///
    /// Refuses a blank name as well as a reserved one: an empty
    /// category collides with the group key that means "nobody could
    /// tell whose this is" (`crate::list::GROUP_CATEGORY`), so the one
    /// row that is an admission of ignorance would collect rows that
    /// are not.
    pub fn register_agent(&self, name: &str, argv: &[String]) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(msg::AGENT_NAME_BLANK.into());
        }
        if Self::RESERVED_AGENT_NAMES.contains(&name) {
            return Err(msg::agent_name_reserved(name));
        }
        let spec = AgentSpec::from_argv(argv).ok_or(msg::AGENT_NEEDS_A_COMMAND)?;
        let loaded = self.agents_loaded()?;
        loaded.doc.set(name, &spec)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Asks a registered agent what conversations it has and remembers
    /// the answer (`session/list` → the agents doc).
    ///
    /// **Asked here rather than on every list poll, and that is the one
    /// real constraint in all of this.** Reading claude's files costs a
    /// millisecond; asking a registry agent costs *starting the agent*.
    /// So the question is put at the moments a person is already
    /// waiting on one — registering it, or asking to see — and the
    /// answer syncs, so no other machine on the mesh has to start that
    /// process to know the same thing.
    ///
    /// Answers what was recorded. An agent that cannot list records
    /// nothing and answers `None`, which stays distinct from `Some([])`
    /// all the way out to the screen.
    pub fn refresh_agent_sessions(&self, name: &str) -> Result<AgentAnswer, String> {
        // **The refusal is recorded, not discarded.** An agent that
        // cannot list is something khor now knows about it; throwing it
        // away leaves the store saying "nobody has asked", which sends
        // the next person to start the agent again for the same
        // nothing.
        let answer = match self.agent_sessions(name, None)? {
            khor_acp::Listing::Cannot => AgentAnswer::Cannot,
            khor_acp::Listing::Sessions(rows) => AgentAnswer::Sessions {
                rows: rows
                    .into_iter()
                    .map(|r| AgentSeen {
                        id: r.id,
                        cwd: r.cwd.display().to_string(),
                        title: r.title,
                        updated_at: r.updated_at,
                    })
                    .collect(),
            },
        };
        let loaded = self.agents_loaded()?;
        loaded.doc.set_seen(name, &answer)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(answer)
    }

    /// What khor remembers this agent said, without asking it again.
    ///
    /// **`None` means nobody has ever asked**, which is not the same as
    /// an agent that answered with nothing — a caller that flattened
    /// them would tell a person "no past conversations" about a
    /// question never put.
    pub fn agent_seen(&self, name: &str) -> Result<Option<AgentAnswer>, String> {
        Ok(self.agents_loaded()?.doc.seen(name))
    }

    /// Adopts a satellite: a machine only ssh can reach, led into the
    /// network by this machine — its sponsor from now on (附属机器批;
    /// docs/NET.md 卫星). Probes it over the user's own ssh road (one
    /// connection: reachable, uname, tmux), mints its id from the host
    /// key already in known_hosts, and writes the row every device will
    /// see on its next sync. Answers `(id, name, tmux)`.
    ///
    /// Re-adoption clears a tombstone the way pairing does: taking a
    /// machine back in is what the verb is, and it cannot happen from
    /// the satellite's own side — it has no side.
    pub fn adopt_satellite(
        &self,
        host: &str,
        name: Option<&str>,
    ) -> Result<(String, String, bool), String> {
        let probe = satellite::probe(host)?;
        let id = satellite::fingerprint_id(host)?;
        let name = name.unwrap_or(host);
        let name = channel_of_machine(name)
            .ok_or_else(|| msg::name_not_pathable(format_args!("{name:?}")))?;
        let loaded = self.devices_loaded()?;
        loaded.doc.upsert(&id, &name, &[])?;
        loaded.doc.set_satellite(&id, self.device_str(), host, probe.tmux)?;
        loaded.doc.set_gone(&id, false)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok((id, name, probe.tmux))
    }

    /// Whether `machine` is a satellite, and if so the ssh road **from
    /// here**: `Ok(Some(host))` when this machine sponsors it,
    /// `Ok(None)` for an ordinary machine, and an error naming the
    /// sponsor when it is somebody else's satellite — a file push runs
    /// the sponsor's scp, and only the sponsor has the road
    /// (一期单 sponsor; docs/NET.md 卫星).
    pub fn satellite_host_here(&self, machine: &str) -> Result<Option<String>, String> {
        let (channel, _) = self.resolve(machine)?;
        let Some(d) = self.devices_loaded()?.doc.by_name(&channel) else {
            return Ok(None);
        };
        let Some(sp) = d.sponsor else { return Ok(None) };
        if sp != self.device_str() {
            let sponsor_name = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .map(|s| s.name)
                .unwrap_or(sp);
            return Err(msg::sat_send_elsewhere(&channel, sponsor_name));
        }
        d.ssh_host
            .map(Some)
            .ok_or_else(|| msg::sat_no_ssh_host(&channel))
    }

    /// Pushes one file into a satellite's home directory over the
    /// sponsor's own scp — the satellite half of `khor send`. No chat
    /// message and no transfer: the far side has no khor to approve or
    /// receive anything, so the bytes just go, and the verb reports
    /// what moved (一期: sponsor 本机才有这条路).
    pub fn send_to_satellite(&self, host: &str, path: &Path) -> Result<u64, String> {
        satellite::push_file(host, path)
    }

    /// Where khor looks for the programs it spawns, and what it hands
    /// them as `PATH`.
    ///
    /// # The bug this exists for
    ///
    /// **A GUI process on macOS does not have the user's `PATH`.**
    /// `khor serve` here is a launchd agent, and launchd hands it
    /// `/usr/bin:/bin:/usr/sbin:/sbin` — while `npx` lives in
    /// `/opt/homebrew/bin`. Every one of the 22 startable entries in
    /// the shipped registry is an `npx` line, so **not one of them
    /// could be started from the app**, while the same command from a
    /// terminal worked perfectly: a terminal has the login shell's
    /// `PATH` and launchd does not. Measured 2026-08-31, same binary,
    /// same agent, only the `PATH` differing.
    ///
    /// It hid behind every green there was — `cargo test`, `tsc`, the
    /// browser gates (routed fixtures, no process), and even a real
    /// backend through the dev bridge, **because the bridge inherits
    /// the shell of whoever started it**. Only the launchd process was
    /// broken, and nothing in this repo ran there.
    ///
    /// # Asked of the login shell, once
    ///
    /// `$SHELL -lic` is what every GUI developer tool on this platform
    /// does about this, for want of anything better: the login shell is
    /// the only thing that knows what the user's `PATH` is, because the
    /// user's `PATH` is something their rc files compute.
    ///
    /// **The cost is real and it is why this is cached**: measured
    /// 0.78s cold and 0.42s warm on this machine, and it *runs the
    /// user's rc files*, side effects and all. So it happens once per
    /// process — [`warm_user_path`] pays it at `khor serve` start,
    /// where a person is not waiting, and everything after reads the
    /// answer.
    ///
    /// **`KHOR_PATH` overrides it**, which is not a nicety: an rc file
    /// that hangs, prompts, or prints would otherwise leave khor with
    /// no way to be told where things are.
    ///
    /// **Not a list of likely directories.** `/opt/homebrew/bin`,
    /// `/usr/local/bin`, nvm's several — that table rots, and the day
    /// it rots nothing goes red: it just fails to find something again,
    /// and (now) says so politely. A table that rots and an assertion
    /// that cannot fail are the same object.
    pub fn user_path() -> &'static str {
        // (the override's name lives at `PATH_ENV`, because the host
        // spawner hands it down to `_host` rather than making that
        // process ask the login shell all over again)
        static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        PATH.get_or_init(|| {
            let here = std::env::var("PATH").unwrap_or_default();
            if let Ok(told) = std::env::var(PATH_ENV) {
                if !told.trim().is_empty() {
                    return told;
                }
            }
            // **The shell's answer is added to, never swapped for,
            // what this process already had.** A shell that answers
            // with something short or strange must not be able to take
            // away directories that were working — the failure mode
            // being avoided is the one this whole function is about.
            merge_paths(&asked_shell().path, &here)
        })
    }

    /// The locale variables to put on a spawned child, **empty when
    /// this process already has a UTF-8 one** — which is every khor
    /// started from a terminal.
    ///
    /// # The bug this exists for
    ///
    /// The third thing launchd's environment does not have, after
    /// `TERM` and `PATH`. A `khor serve` started by launchd has no
    /// `LANG`, no `LC_ALL`, no `LC_CTYPE` — and a tmux client with no
    /// UTF-8 locale runs in **non-UTF-8 mode**: it draws its rules with
    /// the DEC line-drawing set (a literal `q` in khor's screen model)
    /// and replaces every character it cannot encode with `_`.
    ///
    /// Measured 2026-08-31 against tmux's own judgment — same tmux
    /// server, same command, only `LANG` differing:
    ///
    /// ```text
    /// tmux list-clients -F '#{client_utf8}'
    ///   host spawned with no LANG (launchd's shape) → 0
    ///   host spawned with LANG=zh_CN.UTF-8          → 1
    /// ```
    ///
    /// What reached the person was a whole screen of `_` and `q` where
    /// their conversation had been — and the tmux *server* was holding
    /// it perfectly the whole time, which is how it was found.
    ///
    /// # Passed through, not invented
    ///
    /// Unlike `TERM` — which the host states outright, because it
    /// describes khor's own emulator — a locale carries the person's
    /// language and collation as well as the encoding. khor has no
    /// business choosing those, so it takes **their own**, from the
    /// same login shell `user_path` asks, and adds nothing when they
    /// have none: a khor that invented a locale their terminal does
    /// not have would differ from their terminal, which is the
    /// opposite of the point.
    ///
    /// Only variables that actually name UTF-8 are carried, so a shell
    /// answering with a non-UTF-8 locale cannot make this worse than
    /// silence.
    pub fn user_locale() -> &'static [(&'static str, String)] {
        static LOCALE: std::sync::OnceLock<Vec<(&'static str, String)>> =
            std::sync::OnceLock::new();
        LOCALE.get_or_init(|| {
            if LOCALE_VARS.iter().any(|v| says_utf8(&std::env::var(v).unwrap_or_default())) {
                // This process is already speaking UTF-8; touching a
                // child's locale here could only make it differ from
                // the terminal khor was started from.
                return Vec::new();
            }
            asked_shell().locale.clone()
        })
    }

    /// Pays for the login-shell probe now rather than on the first
    /// spawn. Called where nobody is waiting on it (`khor serve`'s
    /// start), and it covers the locale as well as the `PATH` because
    /// there is only ever one probe.
    pub fn warm_user_path() {
        let _ = Self::user_path();
        let _ = Self::user_locale();
    }

    /// The launch JSON for one agent, with its command resolved and the
    /// user's `PATH` attached — **or a refusal that names the program
    /// khor could not find**.
    ///
    /// Every spawn of an ACP agent goes through here, khor's own two
    /// shims included: they are absolute paths so the lookup is free,
    /// and they need the `PATH` too, because what *they* spawn is the
    /// vendor's own binary.
    ///
    /// **The command is resolved to an absolute path here rather than
    /// left for the spawn to look up.** Two reasons, and the second is
    /// the one that matters: it makes the failure khor's to report
    /// instead of an exit status to interpret, and it does not depend
    /// on which `PATH` the process-spawning layer decides to search.
    pub fn agent_launch(spec: &AgentSpec) -> Result<String, String> {
        Self::launch_on(spec, Self::user_path())
    }

    /// [`Self::agent_launch`], against a stated `PATH`. Split out so
    /// the refusal is testable without a login shell in the loop.
    pub(crate) fn launch_on(spec: &AgentSpec, path: &str) -> Result<String, String> {
        let found = find_program(&spec.command, path)
            .ok_or_else(|| msg::command_not_found(&spec.command, path))?;
        serde_json::to_string(&serde_json::json!({
            "command": found,
            "args": spec.args,
            "env": { "PATH": path },
        }))
        .map_err(|e| e.to_string())
    }

    /// The argv the shipped snapshot holds for a registry id, refusing
    /// by name when there is none.
    ///
    /// **Here rather than in the CLI, because two faces expand `@id`
    /// now.** The judgment has two refusals in it — "no such id" and
    /// "that one ships a binary, khor has no command for it" — and the
    /// second is the one a copy would get wrong, because it reads as
    /// the first unless somebody remembered [`registry::Known::argv`]
    /// can be `None` for an entry that is perfectly real.
    pub fn known_argv(&self, id: &str) -> Result<Vec<String>, String> {
        let known = agent_registry::Known::find(id).ok_or_else(|| msg::agents_unknown_id(id))?;
        let argv = known.argv.ok_or_else(|| msg::agents_id_not_startable(id))?;
        Ok(argv.iter().map(|s| (*s).to_owned()).collect())
    }

    /// Registers a snapshot entry under `name` ([`Self::known_argv`]
    /// then [`Self::register_agent`]).
    ///
    /// **The argv never crosses a wire to get here.** A face asks for
    /// an id and this looks the command up; the alternative — a face
    /// that posts the argv it read — is a face that can ask khor to
    /// spawn anything at all, and the browser skin is the second
    /// caller of this table (`khor_gui_core::api`'s head).
    pub fn register_known_agent(&self, name: &str, id: &str) -> Result<(), String> {
        let argv = self.known_argv(id)?;
        self.register_agent(name, &argv)
    }

    /// One conversation this agent was recorded as having, by the id it
    /// gave — `None` for an id nobody recorded under this name.
    ///
    /// **What this is for is the cwd.** `session/load` has to be told
    /// where the conversation was opened, and the only place that
    /// survives is the row the agent itself handed over at
    /// [`Self::refresh_agent_sessions`]. A caller that passed the
    /// directory somebody happened to be standing in would be handing
    /// the agent a different conversation's address.
    pub fn agent_seen_row(&self, name: &str, session: &str) -> Result<Option<AgentSeen>, String> {
        let rows = match self.agent_seen(name)? {
            Some(AgentAnswer::Sessions { rows }) => rows,
            _ => return Ok(None),
        };
        Ok(rows.into_iter().find(|r| r.id == session))
    }

    /// Forgets one. A name nobody registered is not an error — the
    /// caller asked for a world without it, and that is the world.
    pub fn forget_agent(&self, name: &str) -> Result<(), String> {
        let loaded = self.agents_loaded()?;
        loaded.doc.remove(name.trim())?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
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
        let titles = self.titles_loaded()?;
        let now = crate::live::now_ms().max(0) as u64;
        let mut views: Vec<SessionView> = Vec::new();
        for k in self.kinds() {
            for row in k.rows(self)? {
                views.push(SessionView {
                    pinned: pins.doc.pinned(&row.id.0),
                    stale: false,
                    quiet: false,
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
                    stale: false,
                    quiet: false,
                    session: row,
                    source: Some((name, age)),
                });
            }
        }
        // The user's names go on last, over every row alike — the kinds
        // keep deriving their own titles (a sweep recomputes a discovered
        // agent's each round, a reported row is rebuilt from the far
        // machine's report), and this overlay is what makes a rename
        // outlive both (`khor_sync::titles`). Read here rather than
        // carried on the row for the pin's reason: the document
        // replicates, so every device answers from its own copy.
        for v in &mut views {
            if let Some(t) = titles.doc.title_of(&v.session.id.0) {
                v.session.title = t;
            }
            // After the overlay only because both write the same rows;
            // the judgments read state, unread, backing and the pin,
            // never the title.
            v.stale = is_stale(&v.session, v.pinned, now);
            v.quiet = is_quiet(&v.session, now);
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

    /// Writes one sample of the day strip: what the list says right now
    /// (`crates/node/src/day.rs`).
    ///
    /// **The derived list, not the registry.** Half the words here were
    /// never written to disk by anything — `live::face` derives 失败 and
    /// 中断 at read time, `live::settle_done` turns 完成 into 空闲 when
    /// another machine's watermark moves, and discovered rows have no
    /// registry entry at all. This is the only place that sees them.
    ///
    /// Reported rows are skipped: a peer's row is one report with an age
    /// on it, not a history, and recording the cache would turn an
    /// afternoon nobody could reach that machine into an afternoon it
    /// was 忙碌 (`day::unwatched` has the full argument).
    ///
    /// Load-modify-save each time rather than holding the record in
    /// memory: the file is then never behind by more than one sample, so
    /// the newest cell a reader sees is as new as the strip can be, and
    /// a resident that dies takes at most one sample with it.
    pub fn record_day(&self) -> Result<(), String> {
        let views = self.sessions()?;
        let now = crate::live::now_ms();
        let mut record = day::Day::load(&self.root, now);
        record.observe(
            now,
            views
                .into_iter()
                .filter(|v| v.source.is_none())
                .map(|v| (v.session.id.0, v.session.state.state)),
        );
        record.save(&self.root)
    }

    /// Each row's last day, merged down to `cells` cells — one entry per
    /// row the list is showing, keyed by session id.
    ///
    /// **`cells` is the face's to decide and nothing else's.** How many
    /// cells fit is a pixel width, which only the screen knows; which
    /// cell wins when several merge is a judgment, which stays here
    /// (`khor_core::DayCell::rank`). A face that took the raw minutes
    /// and merged them itself would be a second copy of that order.
    ///
    /// This asks `sessions` again rather than taking the caller's rows:
    /// the reported-or-not decision below has to be made from the same
    /// snapshot the strips are cut from, and a row the caller's poll
    /// has and this one does not simply paints nothing for a tick.
    pub fn day_strips(
        &self,
        cells: usize,
    ) -> Result<std::collections::BTreeMap<String, Vec<day::Run>>, String> {
        let views = self.sessions()?;
        let record = day::read(&self.root);
        // Each peer's day once, however many of its rows are showing.
        let mut theirs: std::collections::BTreeMap<String, Option<day::Day>> =
            std::collections::BTreeMap::new();
        Ok(views
            .into_iter()
            .map(|v| {
                let runs = if v.source.is_some() {
                    // **Its home machine's own record**, fetched with its
                    // rows (`Request::Days`) and rolled to now — so a
                    // peer unreachable since noon shows a strip that
                    // stops at noon. A machine that never answered
                    // (an older khor) keeps painting 没在看, which is
                    // the truth about what this one knows.
                    let home = v.session.home.hex();
                    let day = theirs.entry(home.clone()).or_insert_with(|| self.peer_day(&home));
                    match day {
                        Some(d) => d.strip_of(&v.session.id.0, cells),
                        None => day::unwatched(cells),
                    }
                } else {
                    record.strip_of(&v.session.id.0, cells)
                };
                (v.session.id.0, runs)
            })
            .collect())
    }

    /// Takes a sample every [`day::SAMPLE_EVERY`], for as long as the
    /// resident lives.
    ///
    /// **Its own task on its own beat**, the judgment `satellite_sampler`
    /// already made one door down: a sample is a whole `sessions()` —
    /// 13.2 ms of process table and vendor files on this machine's debug
    /// build — and the 5s sync pump must not inherit that weight. It is
    /// also synchronous work, which is the one thing `reactor.rs` exists
    /// to catch starving the runtime, so it goes to a blocking thread
    /// rather than being awaited inline.
    ///
    /// A failing sample is skipped, not logged and not fatal: the strip
    /// is a record of what could be seen, and a minute nobody could
    /// sample is exactly the 不知道 the strip already has a cell for.
    pub(crate) async fn day_sampler(self: std::sync::Arc<Self>) {
        let mut tick = tokio::time::interval(day::SAMPLE_EVERY);
        loop {
            tick.tick().await;
            let me = self.clone();
            let _ = tokio::task::spawn_blocking(move || me.record_day()).await;
        }
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

    /// Names a session, for everyone — or, with an empty title, takes
    /// the name back so the row falls back to what its kind derives
    /// (the vendor's own name, or the title it was opened with).
    ///
    /// The name lands in a replicated document, never in the row's meta:
    /// rows re-derive their titles — a discovered agent's is recomputed
    /// from the vendor's status file on every sweep — so a name written
    /// into meta would be overwritten by the next derivation. The
    /// overlay is applied where the rows are read (`Node::sessions`),
    /// which is also what lets a reported row from another machine wear
    /// it. Replicated because a name is a decision, the same class as a
    /// pin (docs/NET.md 同步).
    ///
    /// **Any id may be named, including one no row answers to today** —
    /// the same ruling as `pin_session`, for the same reasons.
    pub fn rename_session(&self, id: &SessionId, title: &str) -> Result<(), String> {
        let loaded = self.titles_loaded()?;
        loaded.doc.set(&id.0, title)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        self.emit_row_of(id)
    }

    /// Pins or unpins a directory on a machine, for everyone. The key
    /// carries the machine because a bare path exists on every disk in
    /// the mesh (`khor_sync::dirpins` module head); the path must be
    /// absolute for the reason `Ls` demands it — a relative pin names
    /// nowhere.
    pub fn pin_dir(&self, machine: &str, path: &str, on: bool) -> Result<(), String> {
        if !std::path::Path::new(path).is_absolute() {
            return Err(msg::path_not_absolute(path));
        }
        let (_, home) = self.resolve(machine)?;
        let loaded = self.dirpins_loaded()?;
        loaded.doc.set(&khor_sync::dirpins::key(&home.hex(), path), on)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Every pinned directory, as `(device_hex, path)` in the table's
    /// order (sorted keys — grouped by machine for free). Keys some
    /// other version spelled differently are skipped, never guessed at.
    pub fn dir_pins(&self) -> Result<Vec<(String, String)>, String> {
        Ok(self
            .dirpins_loaded()?
            .doc
            .all()
            .iter()
            .filter_map(|k| {
                khor_sync::dirpins::split(k).map(|(d, p)| (d.to_owned(), p.to_owned()))
            })
            .collect())
    }

    /// Pins or unpins a web page against an exit machine, for everyone.
    /// The key carries the machine because a pinned page is "this page
    /// through this machine's network" (`khor_sync::webpins` module head)
    /// — the browser landing's shortcuts.
    pub fn pin_web(&self, machine: &str, url: &str, on: bool) -> Result<(), String> {
        let (_, home) = self.resolve(machine)?;
        let loaded = self.webpins_loaded()?;
        loaded.doc.set(&khor_sync::webpins::key(&home.hex(), url), on)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Every pinned page, as `(device_hex, url)` in the table's order
    /// (sorted keys — grouped by machine for free). Keys some other
    /// version spelled differently are skipped, never guessed at.
    pub fn web_pins(&self) -> Result<Vec<(String, String)>, String> {
        Ok(self
            .webpins_loaded()?
            .doc
            .all()
            .iter()
            .filter_map(|k| {
                khor_sync::webpins::split(k).map(|(d, u)| (d.to_owned(), u.to_owned()))
            })
            .collect())
    }

    /// Records that this person went to a place, for every device
    /// (`khor_sync::recents` — the memory travels, the tab strip does
    /// not). Revisiting moves the instant and adds no row.
    ///
    /// A directory's path must be absolute for the same reason
    /// [`Self::pin_dir`] demands it: a relative path names nowhere, and
    /// a suggestion nobody can open is worse than no suggestion.
    pub fn recent_visit(&self, kind: PlaceKind, machine: &str, place: &str) -> Result<(), String> {
        if kind == PlaceKind::Dir && !std::path::Path::new(place).is_absolute() {
            return Err(msg::path_not_absolute(place));
        }
        let (_, home) = self.resolve(machine)?;
        let loaded = self.recents_loaded()?;
        loaded
            .doc
            .visit(&khor_sync::recents::key(kind, &home.hex(), place), crate::live::now_ms())?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// "Stop suggesting this", for every device. A tombstone rather
    /// than a deletion, so the dismissal travels — and going back there
    /// puts the place back on its own (`khor_sync::recents`).
    pub fn forget_recent(&self, kind: PlaceKind, machine: &str, place: &str) -> Result<(), String> {
        let (_, home) = self.resolve(machine)?;
        let loaded = self.recents_loaded()?;
        loaded.doc.forget(&khor_sync::recents::key(kind, &home.hex(), place))?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Where this person has been of this kind, most recent first, at
    /// most `limit` of them. Dismissed places are not on it, and keys
    /// some other version spelled differently are skipped rather than
    /// guessed at.
    pub fn recents(&self, kind: PlaceKind, limit: usize) -> Result<Vec<RecentPlace>, String> {
        Ok(self
            .recents_loaded()?
            .doc
            .all()
            .iter()
            .filter_map(|(k, at_ms)| {
                let (k_kind, device, place) = khor_sync::recents::split(k)?;
                (k_kind == kind).then(|| RecentPlace {
                    machine: device.to_owned(),
                    place: place.to_owned(),
                    at_ms: *at_ms,
                })
            })
            .take(limit)
            .collect())
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

    /// Shows a machine the door, for the whole network
    /// (`DeviceDoc::set_gone` has why leaving must travel the way
    /// joining does, and why it is a tombstone rather than a deletion).
    ///
    /// **Refuses this machine by name.** The table being written into is
    /// the one that would be left, so from the inside the act has no
    /// meaning — and a machine that removed itself would keep every
    /// row it holds while becoming a stranger to everyone else, which
    /// is not "left the network", it is broken in a way with no word
    /// for it.
    pub fn forget_device(&self, machine: &str) -> Result<String, String> {
        let (name, home) = self.resolve(machine)?;
        if home == self.device() {
            return Err(msg::CANT_FORGET_SELF.into());
        }
        let loaded = self.devices_loaded()?;
        loaded.doc.set_gone(&home.hex(), true)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(name)
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
        // Only machines the table still holds get to keep rows on the
        // list (forget 批): a forgotten machine's report can never
        // refresh, so its rows would hang forever as "N 天没联系上" —
        // rows nothing can answer for. The file name is the reporter's
        // device id (`cache_peer_rows` writes it), and filtering by the
        // reporter covers "home 是它的行" because a report carries only
        // rows the reporter itself derives (`reportable_rows` — home-only
        // by construction). The cache file itself stays: a machine
        // invited back resurfaces its last report at its honest age,
        // which is exactly the offline axis (docs/SESSION.md 离线).
        // An unreadable table is a blip, not a verdict on every peer:
        // `None` skips the filter and paints what is cached, rather than
        // blanking a list the person was just looking at.
        let table: Option<std::collections::BTreeSet<String>> =
            self.devices().ok().map(|devs| devs.into_iter().map(|d| d.id).collect());
        for e in rd.flatten() {
            if let Some(table) = &table {
                if !e.file_name().to_str().is_some_and(|id| table.contains(id)) {
                    continue;
                }
            }
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
        let report = VitalsReport { at_ms: now_ms(), vitals: v.clone() };
        link::write_private(
            &self.vitals_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// [`Node::cache_peer_vitals`] with the reading's true age: what a
    /// sponsor relays about its satellite arrives already aged, and
    /// stamping it "now" would dress the sponsor's minute-old sample in
    /// the arrival's freshness (离线轴: a stale number must look stale).
    /// A peer's day record as it answered it (`Request::Days`), kept
    /// beside its vitals and rows: one file per machine, the text the
    /// peer sent, stamped with when it arrived.
    pub(crate) fn cache_peer_day(&self, device_id: &str, day: &str) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        fs::create_dir_all(self.days_dir()).map_err(msg::cant_make_vitals_dir)?;
        let report = DayReport { at_ms: now_ms(), day: day.to_owned() };
        link::write_private(
            &self.days_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// The cached day of one machine, rolled to now — `None` when that
    /// machine never answered (an older khor, or one never reached).
    fn peer_day(&self, device_id: &str) -> Option<day::Day> {
        let text = fs::read_to_string(self.days_dir().join(device_id)).ok()?;
        let report: DayReport = serde_json::from_str(&text).ok()?;
        day::Day::from_wire(&report.day, now_ms() as i64)
    }

    fn days_dir(&self) -> PathBuf {
        self.root.join("days")
    }

    pub(crate) fn cache_peer_vitals_aged(
        &self,
        device_id: &str,
        v: &khor_core::Vitals,
        age_ms: u64,
    ) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        fs::create_dir_all(self.vitals_dir()).map_err(msg::cant_make_vitals_dir)?;
        let report = VitalsReport {
            at_ms: now_ms().saturating_sub(age_ms),
            vitals: v.clone(),
        };
        link::write_private(
            &self.vitals_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// The sponsor's side of a satellite's numbers: every minute, one
    /// ssh sample per satellite this machine sponsors, kept in the same
    /// per-device cache every peer reading lands in — so `vitals_of`,
    /// the GUI and the [`proto::Request::SatVitals`] answer all read one
    /// record. Runs for as long as the serve does; a satellite that
    /// stops answering simply stops refreshing, and its reading ages in
    /// plain sight (离线轴).
    pub(crate) async fn satellite_sampler(&self) {
        let mut tick = tokio::time::interval(satellite::SAMPLE_EVERY);
        loop {
            tick.tick().await;
            let Ok(devs) = self.devices() else { continue };
            for d in devs {
                if d.sponsor.as_deref() != Some(self.device_str()) {
                    continue;
                }
                let Some(host) = d.ssh_host.clone() else { continue };
                // Off the runtime: a sample is an ssh dial plus a
                // one-second on-host measurement.
                let taken = tokio::task::spawn_blocking(move || satellite::sample(&host)).await;
                if let Ok(Ok(v)) = taken {
                    let _ = self.cache_peer_vitals(&d.id, &v);
                }
            }
        }
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

    // ── what machines have spent (usage) ──────────────────────

    fn usage_dir(&self) -> PathBuf {
        self.root.join(".khor").join("usage")
    }

    /// Remembers a machine's spending answer, stamped with now.
    ///
    /// **Its own file, beside vitals rather than inside it**, and for the
    /// reason the vitals record gives about the rows: the two are
    /// fetched by requests that fail independently and age
    /// independently, so one record would have a lost round erase the
    /// other's last known answer and one shared timestamp would dress a
    /// stale answer in a fresh one's age.
    pub(crate) fn cache_peer_usage(
        &self,
        device_id: &str,
        u: &khor_core::Usage,
    ) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        fs::create_dir_all(self.usage_dir()).map_err(msg::cant_make_usage_dir)?;
        let report = UsageReport { at_ms: now_ms(), usage: u.clone() };
        link::write_private(
            &self.usage_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// What a machine has spent, and how old that answer is.
    ///
    /// **This machine answers by reading, everyone else from cache**, and
    /// the age is what tells them apart at the far end: zero means the
    /// answer was worked out to answer this call. A machine nobody has
    /// heard from yet is `None` — a third state, distinct from an old
    /// answer, because "not asked yet" and "asked an hour ago" are
    /// different things to be told (docs/SESSION.md 离线).
    ///
    /// **Blocking for this machine.** See [`Node::usage`].
    pub fn usage_of(&self, device_id: &str) -> Option<(khor_core::Usage, u64)> {
        if device_id == self.device_str() {
            return Some((self.usage(), 0));
        }
        let text = fs::read_to_string(self.usage_dir().join(device_id)).ok()?;
        let report: UsageReport = serde_json::from_str(&text).ok()?;
        Some((report.usage, now_ms().saturating_sub(report.at_ms)))
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
        // **Offering is not looking, so no watermark moves here** — and
        // this is the one place the person and the machine come apart.
        //
        // The watermark is the *person's* decision, replicated on
        // purpose: a look on the phone clears the desk (docs/NET.md 已读).
        // That is exactly why `tell` raises it and must keep raising it —
        // a line's content *is* the line, so the person who typed it has
        // seen it wherever they read it, and `tests/pairing.rs` pins that
        // ("own words never count as unread, on any device").
        //
        // A file message is not that. What it carries is not content but
        // a summary of bytes that have not arrived anywhere yet, and
        // **"it arrived here" is a fact about a machine, not about a
        // person**. So there is no seat — not even this one — that has
        // looked at it. Raising the watermark from here wrote a machine's
        // fact into the person's document, and the offering machine
        // cleared the badge on the machine the file was actually for:
        // the recipient's row read 空闲/0 where docs/SESSION.md says an
        // arrived file goes to 完成/未看 like any message, and a third
        // machine never saw it at all.
        //
        // This machine still does not badge itself for what it offered.
        // That is a seat's answer and it is derived locally, in
        // `ChatKind::i_offered` — never written into a document that
        // travels.
        //
        // The row is therefore re-read against the watermark that
        // actually stands, not against the offer's own stamp: an offer
        // clears nothing, so a debt already sitting on this row must
        // survive it.
        let wm = self.seen_loaded()?.doc.watermark(&told.row.id.0);
        let row = self.chat.row(&channel, home, wm)?;
        self.emit(NodeEvent::Event(told.event));
        self.emit(NodeEvent::Row(row));
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

    /// Whether this machine already holds the offer behind a transfer
    /// id — asked **before** `accept`, by a caller that wants to say
    /// out loud that it is about to wait.
    ///
    /// A miss here is not an answer about the id (that takes a sync
    /// round — `Node::accept`); it is only "not here yet", which is
    /// precisely the thing worth announcing.
    pub fn transfer_is_here(&self, id: &SessionId) -> bool {
        id.0.strip_prefix("transfer/")
            .is_some_and(|m| self.find_transfer(m).is_ok())
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

    /// The readout with each message's transfer face, aligned by index
    /// — `None` for anything that is not a file message on a seat with
    /// something to say (`TransferKind::message_face`). One read, one
    /// order: two calls would let a message land between them and put
    /// every face off by one.
    pub fn log_faced(
        &self,
        machine: &str,
    ) -> Result<
        (
            chat::ChannelLog,
            Vec<(Option<khor_core::State>, Option<transfer::Delivery>)>,
        ),
        String,
    > {
        let (channel, _) = self.resolve(machine)?;
        let log = self.chat.log(&channel)?;
        let dir = khor_sync::chat::channel_dir(&self.root, &channel)
            .ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))?;
        // The row's own seen mark, read once for the whole readout: the
        // chips answer 完成/空闲 with the same watermark the row counts
        // unread against, so a chip and the row it sits in cannot
        // disagree about what has been looked at. Read here rather than
        // inside the loop for the reason the whole function exists —
        // one read, one order.
        let watermark = self.seen_loaded()?.doc.watermark(&format!("chat/{channel}"));
        // Per message, both readings: the receiver's state word and the
        // sender's neutral delivery fact — at most one of the two is
        // `Some` (the seats are exclusive), and the pane paints
        // whichever it got.
        let faces = log
            .messages
            .iter()
            .map(|m| {
                (
                    self.transfer.message_face(&channel, &dir, m, watermark),
                    self.transfer.message_delivery(m),
                )
            })
            .collect();
        Ok((log, faces))
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
    /// Close a row wherever it lives.
    ///
    /// **Async, and [`Self::close`] is not** — deliberately. A sync
    /// function that builds a runtime to do network work panics the
    /// moment it is called from inside one, which is every caller that
    /// matters (the serve, the app). So the routing lives here, at the
    /// door, exactly where `attach`'s does: one list holds every
    /// machine's sessions, so *which computer is this on* must not be a
    /// question the person answers before they can close a row — but it
    /// is a question the **caller** answers, not the primitive.
    pub async fn close_anywhere(&self, id: &SessionId) -> Result<(), String> {
        if let Some(machine) = self.far_machine(id)? {
            return self.close_on(&machine, id).await;
        }
        self.close(id)
    }

    /// Which machine this row lives on, when it is not this one.
    ///
    /// **The fork is on the row, not on a flag.** A person types one
    /// verb and clicks one row for a session wherever it runs, so every
    /// door that acts on a session asks this one question first and
    /// then goes local or far — `close_anywhere` here, `attach` in the
    /// CLI, `term_open` in the app. `None` also covers a row this
    /// machine simply does not know: acting locally then fails by name
    /// on a session id, which is the truth, rather than by asking a
    /// machine that was never named.
    pub fn far_machine(&self, id: &SessionId) -> Result<Option<String>, String> {
        let me = self.device();
        for v in self.sessions()? {
            if v.session.id == *id {
                if v.session.home == me {
                    return Ok(None);
                }
                return Ok(v.source.map(|(name, _)| name));
            }
        }
        Ok(None)
    }

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
        // **`khor run` happens where the person is standing**, and the
        // host inherits this process's directory — so the answer is
        // here, and asking for it as an argument would only give every
        // caller a chance to pass a different one.
        if let Ok(cwd) = std::env::current_dir() {
            self.live.learn_place(&id, &cwd)?;
        }
        let dir = self
            .live
            .dir_of(&id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host(&dir, &id, cmd, size)?;
        Ok(id)
    }

    /// `open_persistent`, born in a chosen directory — same registration,
    /// the host just starts life standing where the wizard pointed.
    pub fn open_persistent_at(
        &self,
        cwd: &std::path::Path,
        kind: &str,
        title: &str,
        cmd: &[String],
        size: (u16, u16),
    ) -> Result<SessionId, String> {
        let leaf = link::fresh_leaf()?;
        let id = SessionId(format!("{kind}/{leaf}"));
        self.live.register(&id, kind, title, None, Self::category_of_started(kind))?;
        // Where it was born, said now because now is the only moment
        // anyone knows: a shell that has `cd`ed away cannot be asked
        // (`khor_core::Session::place`).
        self.live.learn_place(&id, cwd)?;
        let dir = self
            .live
            .dir_of(&id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host_at(cwd, &dir, &id, cmd, size)?;
        Ok(id)
    }

    /// [`Self::open_persistent_at`] under an id **the caller chose** —
    /// for a terminal-form session khor opens itself and hands the
    /// vendor its own session id (`claude --session-id`).
    ///
    /// The whole point is the id. A khor-minted `tui/<random>` row is a
    /// row that cannot say which vendor conversation it holds, so
    /// 接管 has nothing to resume and the transcript merges into
    /// nothing — and the only thing that could tell it was a hook,
    /// which a machine may simply not have installed. Naming the
    /// session up front removes the dependency instead of reporting it.
    pub fn open_persistent_as(
        &self,
        id: &SessionId,
        cwd: &std::path::Path,
        kind: &str,
        title: &str,
        cmd: &[String],
        size: (u16, u16),
        category: Option<&str>,
    ) -> Result<SessionId, String> {
        self.live.register(id, kind, title, None, category)?;
        self.live.learn_place(id, cwd)?;
        let dir = self
            .live
            .dir_of(id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host_at(cwd, &dir, id, cmd, size)?;
        Ok(id.clone())
    }

    /// Opens a GUI session: a detached host holding one ACP agent. The
    /// id comes back from the host — it is the vendor's own, and exists
    /// only once the agent has answered (`gui_host` module head).
    pub fn open_gui(&self, title: &str, cmd: &[String]) -> Result<SessionId, String> {
        gui_host::spawn_gui_host(None, title, cmd, None, false)
    }

    /// [`Self::open_gui`], for an agent the user registered by name
    /// (批⑥). The name rides to the row as its
    /// [`khor_core::Session::category`] — **the user's own answer to
    /// "whose session is this"**, which is not the guess khor refuses
    /// to make from a command line (`gui_host`'s vendor door, and the
    /// wizard's 智能体 field before it).
    ///
    /// A row opened without a registration keeps no category at all:
    /// an ad-hoc command names nothing khor may put on a row, and an
    /// honest gap is the answer there (docs/SESSION.md 认不出就不落词).
    pub fn open_gui_as(
        &self,
        title: &str,
        cmd: &[String],
        agent: &str,
    ) -> Result<SessionId, String> {
        gui_host::spawn_gui_host(None, title, cmd, Some(agent), false)
    }

    /// Reopens a conversation the agent itself says it has
    /// (`session/list` → `session/load`).
    ///
    /// **Not [`Node::takeover`], and the difference is that nothing is
    /// running.** Takeover ends a live TUI and re-seats its row; there
    /// is nothing here to end and no row to re-seat. What arrives is a
    /// new row wearing the vendor's own session id, with its past
    /// replayed into it — which is what makes it the *same*
    /// conversation rather than a fresh one that happens to know things.
    ///
    /// **The agent has to be able to replay**, or the reopened
    /// conversation comes up empty and reads as "nobody has said
    /// anything" for a session that may have hundreds of lines. That is
    /// `Facts::replays`, and the check belongs before the process is
    /// spawned rather than after — the alternative is a row that exists
    /// and lies.
    ///
    /// **`cwd` is the conversation's own, not the caller's.**
    /// `session/load` carries the directory the session was opened in
    /// (`khor_acp::Handle`), and a real adaptor is entitled to refuse a
    /// mismatch — so the only honest source is the row the agent handed
    /// over when it listed ([`Self::agent_seen_row`]). This took `None`
    /// until 2026-08-31, which loaded every resumed conversation at
    /// whatever directory the process happened to start in: it survived
    /// its first end-to-end run because that run stood in the right
    /// place, which is exactly the shape of a bug that waits.
    pub fn reopen_gui_as(
        &self,
        cwd: &std::path::Path,
        title: &str,
        cmd: &[String],
        agent: &str,
        session: &str,
    ) -> Result<SessionId, String> {
        // Refused here rather than at the spawn: a directory that is
        // gone comes back from the child as a launch failure, and the
        // thing a person can act on is the path.
        if !cwd.is_dir() {
            return Err(msg::no_such_dir(&cwd.display().to_string()));
        }
        // **A conversation that is running somewhere is not resumed, it
        // is joined — and this door cannot join.** `session/load` on a
        // vendor session another process holds starts a second agent on
        // the same transcript: two claudes writing one record, and a row
        // whose word is whichever of them spoke last (found live,
        // 2026-09-01: the person's own terminal claude read 空闲 for an
        // hour because a reopen of its session said so). The takeover
        // is the door that ends the other process first; this one
        // refuses while it lives, naming it.
        if let Some((_, pids)) = self.live.sighting_of(&adaptor::id_for(session)) {
            if let Some(pid) = pids.first() {
                return Err(msg::resume_running_elsewhere(pid));
            }
        }
        gui_host::spawn_gui_host_reopen(Some(cwd), title, cmd, Some(agent), session)
    }

    /// `open_gui`, born in a chosen directory (the wizard's 目录 field);
    /// the agent underneath inherits it as the session's cwd.
    ///
    /// `vendor` is the wizard's 智能体 field — **the user's own answer
    /// to "whose session is this"**, carried to the row. Khor still
    /// refuses to *guess* a vendor from an adapter's command name
    /// (`gui_host` module head); being told is not guessing, and
    /// without this a session khor opened itself was the one kind of
    /// row that could not say who it belonged to.
    pub fn open_gui_at(
        &self,
        cwd: &std::path::Path,
        title: &str,
        cmd: &[String],
        vendor: Option<&str>,
        // `agent`: hand it khor's own verbs (docs/AGENT.md 调度员).
        agent: bool,
    ) -> Result<SessionId, String> {
        gui_host::spawn_gui_host(Some(cwd), title, cmd, vendor, agent)
    }

    /// **常去的地方**: the directories this machine's own records say
    /// work happened in, folded to project roots and newest first
    /// (`crate::place`).
    ///
    /// # What it is not
    ///
    /// Not an inventory of every session. Two sources, because the
    /// fact lives in two places: the *vendors'* records answer for
    /// every conversation, and khor's own registry answers for what
    /// only khor knows — a directory somebody only ever opened a
    /// terminal in (`Meta::place`, written at birth since 2026-09-01;
    /// a shell from before that has no note and counts nowhere).
    ///
    /// # This machine's, and only this machine's
    ///
    /// The records are on this disk. A directory on another machine is
    /// not somewhere this one can start a session anyway, so there is
    /// nothing here to ask the mesh for.
    pub fn places(&self) -> Vec<place::Place> {
        let vendors = adaptor::vendor_home(&self.root);
        let mut seen = adaptor::claude::Claude::at(vendors.join(".claude")).places();
        seen.extend(adaptor::codex::Codex::at(vendors.join(".codex")).places());
        seen.extend(self.live.places());
        place::fold(seen)
    }

    /// 接管 (批C): the conversation stays; the process presenting it
    /// changes hands. Ends whatever shows this session now — a viewer
    /// host's bridge, the TUI process itself — and resurrects the same
    /// vendor session as a conversation row (`gui_host` resume mode,
    /// `--resume` probed: same id, same transcript, works across cwd).
    /// The one-mouth ruling made executable: after this, the chat face
    /// is the only body.
    ///
    /// Refuses honestly: a row that is already a conversation, and a
    /// session whose transcript nobody can find (nothing to resume).
    /// A session with no live process is taken over by resurrection
    /// alone — nothing to kill is not an error.
    pub fn takeover(&self, id: &SessionId) -> Result<(), String> {
        let id_leaf = id
            .0
            .strip_prefix(&format!("{}/", khor_core::kind::TUI))
            .ok_or_else(|| msg::takeover_no_record(&id.0))?;
        // A khor-minted id names no vendor file; the hook-recorded
        // vendor leaf is the bridge (`transcript_of`'s precedent).
        let leaf = self.live.vendor_leaf_of(id).unwrap_or_else(|| id_leaf.to_owned());
        let leaf = leaf.as_str();
        if let Some((kind, _)) = self.live.on_record(id) {
            if kind == khor_core::kind::GUI {
                return Err(msg::already_a_chat(&id.0));
            }
        }
        // Whose conversation is this? The store that resolves the leaf
        // answers — a vendor's transcript dir is its own registry, and
        // the protocol has no wire question for this yet (`crates/acp`'s
        // edge survey: session/list is an unadopted draft).
        let vhome = adaptor::vendor_home(&self.root);
        let claude = adaptor::claude::Claude::at(vhome.join(".claude"));
        let (full, cwd, shim) = if let Some(full) = claude.full_session_id(leaf) {
            let cwd = claude
                .recorded_cwd(leaf)
                .or_else(std::env::home_dir)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            (full, cwd, "_cagent")
        } else {
            let codex = adaptor::codex::Codex::at(vhome.join(".codex"));
            let full = codex
                .full_session_id(leaf)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            let cwd = codex
                .recorded_cwd(&full)
                .or_else(std::env::home_dir)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            (full, cwd, "_codexagent")
        };
        let sighting = self.live.sighting_of(id);
        let title = sighting
            .as_ref()
            .map(|(t, _)| t.clone())
            .or_else(|| self.live.on_record(id).map(|(_, t)| t))
            .unwrap_or_else(|| leaf.to_owned());

        // A host standing on this row is a viewer (agent rows never own
        // their process): end its bridge first, and let it clean up
        // after itself before the ghost writes its own host marker.
        if let Some(dir) = self.live.dir_of(id) {
            // A viewer bridged onto the person's tmux has its own file
            // (`host::view_file_path`); it is "whatever shows this
            // session now" as much as a host is, and goes first.
            for hf in [host::read_view_file(&dir), host::read_host_file(&dir)].into_iter().flatten() {
                #[cfg(unix)]
                unsafe {
                    libc::kill(-(hf.child_pid as i32), libc::SIGTERM);
                }
                for _ in 0..50 {
                    if !link::pid_running(hf.host_pid) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        // End the TUI itself. SIGTERM leaves the transcript whole and
        // resumable (实测③: 0 bad lines, resumes at once); what was not
        // flushed to disk is the interruption the confirm warned about.
        if let Some((_, pids)) = &sighting {
            #[cfg(unix)]
            for &pid in pids {
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGTERM);
                }
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while pids.iter().any(|&p| link::pid_alive(p)) {
                if std::time::Instant::now() > deadline {
                    return Err(msg::takeover_wont_die(&id.0));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        let exe = crate::self_exe()?;
        gui_host::spawn_gui_host_resume(
            Some(&cwd),
            &title,
            &[exe.display().to_string(), shim.into()],
            &full,
        )?;
        // The reborn row wears the vendor's uuid. A khor-minted row was
        // a different spelling of the same conversation — one thing, one
        // row (批A), so the old spelling leaves with the process it
        // described.
        let reborn = adaptor::id_for(&full);
        if reborn != *id {
            let _ = self.live.close_session(id);
        }
        Ok(())
    }

    /// 接管, the other way: a conversation khor holds moves into a
    /// terminal. Same conversation, same row, same vendor uuid — the
    /// form changes and nothing else, which is the 唯一本体 ruling read
    /// in the direction [`Self::takeover`] does not cover.
    ///
    /// The order matters. The ghost dies first (its process group takes
    /// the shim and the agent with it), because two processes holding
    /// one vendor session is the one state the ruling forbids. Then the
    /// terminal host comes up **on the same registry dir**, running the
    /// vendor's own resume; only then is the row re-seated as a
    /// terminal — the host claims the pid itself on the way up
    /// (`LiveKind::reseat`), so nothing here has to guess it.
    pub fn takeover_term(&self, id: &SessionId) -> Result<(), String> {
        let Some((kind, title)) = self.live.on_record(id) else {
            return Err(msg::takeover_no_record(&id.0));
        };
        if kind != khor_core::kind::GUI {
            return Err(msg::already_a_terminal(&id.0));
        }
        // A GUI row's id **is** the vendor's uuid, lossily spelled
        // (`adaptor::id_for`); the transcript's own filename is the
        // whole one.
        let leaf = id
            .0
            .strip_prefix(&format!("{}/", khor_core::kind::TUI))
            .ok_or_else(|| msg::takeover_no_record(&id.0))?;
        // The same store-resolves-it vendor fork as `takeover`, plus
        // each vendor's own resume spelling: claude takes a flag,
        // codex a subcommand (`codex resume <uuid>`, probed 0.146.1).
        let vhome = adaptor::vendor_home(&self.root);
        let claude = adaptor::claude::Claude::at(vhome.join(".claude"));
        let (cwd, resume) = if let Some(full) = claude.full_session_id(leaf) {
            let cwd = claude
                .recorded_cwd(leaf)
                .or_else(std::env::home_dir)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            let bin = std::env::var("KHOR_CLAUDE").unwrap_or_else(|_| "claude".into());
            (cwd, vec![bin, "--resume".into(), full])
        } else {
            let codex = adaptor::codex::Codex::at(vhome.join(".codex"));
            // **Neither vendor knows this row**, which since 批⑥ is a
            // thing that happens on purpose: a session opened with an
            // agent the user registered has no vendor CLI khor knows
            // how to resume, and no transcript in a format khor reads.
            // Saying 「没有它的对话记录」 here would send a person to
            // look for a file that was never supposed to exist — the
            // limit is in khor, and the sentence has to say so.
            let full = codex
                .full_session_id(leaf)
                .ok_or_else(|| msg::no_terminal_form(&id.0))?;
            let cwd = codex
                .recorded_cwd(&full)
                .or_else(std::env::home_dir)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            let bin = std::env::var("KHOR_CODEX").unwrap_or_else(|_| "codex".into());
            (cwd, vec![bin, "resume".into(), full])
        };
        let dir = self.live.dir_of(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;

        // End the conversation's body. The ghost writes itself as both
        // host and child (`gui_host`), so its group is the shim and the
        // agent too.
        for hf in [host::read_view_file(&dir), host::read_host_file(&dir)].into_iter().flatten() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(hf.child_pid as i32), libc::SIGTERM);
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            // `pid_running`, not `pid_alive`: the ghost is this
            // process's own child, so after it exits its pid stays taken
            // until somebody reaps it — and a signal probe would call
            // that "still there" for as long as this process lives.
            while link::pid_running(hf.host_pid) {
                if std::time::Instant::now() > deadline {
                    return Err(msg::takeover_wont_die(&id.0));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        host::spawn_host_at(&cwd, &dir, id, &resume, (80, 24))?;
        self.live.reseat(id, khor_core::kind::TUI, &title, None)
    }

    /// Stands a host up for a discovered tmux session so a terminal can
    /// attach (`LiveKind::attach_multiplexed` has the whole judgment).
    pub fn attach_tmux(&self, id: &SessionId) -> Result<live::Seat, String> {
        self.live.attach_multiplexed(id)
    }

    /// The registry dir behind a live session id, for attach clients.
    pub fn session_dir(&self, id: &SessionId) -> Option<PathBuf> {
        self.live.dir_of(id).filter(|d| d.exists())
    }

    /// Whether this machine hosts this session's terminal — a `host.json`
    /// in its registry dir **naming a process that is still running**.
    /// False for a `khor run` (临时, no host process), a discovered
    /// session (khor never hosted it), and any remote session (its host
    /// is on another machine). It is the signal for "a terminal can
    /// attach here": docs/handoff 终端画屏.
    ///
    /// **The marker file outlives the process it describes.** A host
    /// dies with the bridge that spawned it and leaves its `host.json`
    /// on disk; answering from the file's mere existence said yes about
    /// a corpse, and the caller then skipped the re-bridge that would
    /// have fixed it (`term::term_open`) to connect to a closed port —
    /// a live tmux session whose row painted 宿主没了 on every click.
    /// The question being asked is "is there a terminal to be had here",
    /// and the pid is the only part of that file which can answer it.
    pub fn is_hosted(&self, id: &SessionId) -> bool {
        self.live.dir_of(id).is_some_and(|d| {
            host::read_host_file(&d).is_ok_and(|hf| link::pid_running(hf.host_pid))
        })
    }

    /// A process reporting its own word — the hook door. 失败 is refused
    /// by the kind: it derives from the exit code, never from a claim.
    ///
    /// Being the hook door is what fixes the provenance here: everything
    /// arriving through it is a vendor speaking for itself. The screen
    /// reader does not come this way, and must not be given a route that
    /// does (`live::Source`).
    pub fn report_state(&self, id: &SessionId, word: khor_core::State) -> Result<(), String> {
        self.live.report(id, word, live::Source::Reported)
    }

    /// One Claude Code hook payload in, the mapped session move out.
    pub fn claude_hook(&self, payload: &str) -> Result<adaptor::claude::Hooked, String> {
        adaptor::claude::hook(&self.live, payload)
    }

    /// What the agents on this machine have spent, by day and by vendor.
    ///
    /// **Blocking, and it can take seconds on the first ask** — one pass
    /// over every transcript is 18.5 s on this machine in a debug build
    /// (the figures are on `usage::Meters::tally`). Every ask after that
    /// is a walk of directory entries unless an agent has written
    /// something, so this is affordable to ask repeatedly and not
    /// affordable to ask from inside an async handler: the one caller
    /// there goes through `spawn_blocking`, exactly as vitals does.
    pub fn usage(&self) -> khor_core::Usage {
        self.usage.tally()
    }

    /// What every machine in the network has spent, summed by day and
    /// by vendor.
    ///
    /// **A machine khor has never got an answer out of contributes
    /// nothing, and this cannot say which.** That is not a gap to be
    /// filled with a caveat: whoever paints this shows the machines
    /// beside it, and a machine that has never answered says so on its
    /// own face (`khor_gui_core`, the map). Two halves of one screen,
    /// neither explaining the other.
    ///
    /// **The days were each cut in their own machine's zone**, so a
    /// network-wide "today" is the sum of everyone's own today
    /// (`khor_core::UsageDay`). Two machines eight hours apart disagree
    /// about which side of midnight a late evening falls on; the
    /// alternative is a day boundary that is wrong for everybody instead
    /// of right for each.
    pub fn usage_everywhere(&self) -> Result<khor_core::Usage, String> {
        let mut rows: std::collections::BTreeMap<(String, String), khor_core::Tokens> =
            Default::default();
        let mut unreadable = 0u64;
        for d in self.devices()? {
            let Some((u, _)) = self.usage_of(&d.id) else { continue };
            unreadable = unreadable.saturating_add(u.unreadable);
            for day in u.days {
                rows.entry((day.day, day.category)).or_default().add(day.tokens);
            }
        }
        Ok(khor_core::Usage {
            // Oldest first, the same order one machine's answer comes in
            // — a face that had to sort would be the second sort this
            // repo keeps refusing to grow (`list`).
            days: rows
                .into_iter()
                .map(|((day, category), tokens)| khor_core::UsageDay { day, category, tokens })
                .collect(),
            unreadable,
        })
    }

    /// The meters themselves, so a caller that must not block the
    /// reactor can move the reading onto a blocking thread. The
    /// bookkeeping that makes a second pass cheap lives in there, so
    /// handing out a clone of the registry would hand out a cold one —
    /// this shares the same instance.
    pub(crate) fn usage_meters(&self) -> Arc<usage::Meters> {
        self.usage.clone()
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

    /// The recorded conversation behind a discovered claude session —
    /// the vendor's own transcript, read whole. Claude only, this
    /// batch: other vendors' records are on the ledger. Rooted through
    /// `vendor_home` like everything that reads a vendor's files.
    pub fn transcript_of(
        &self,
        id: &SessionId,
    ) -> Result<Vec<adaptor::claude::Utterance>, String> {
        let leaf = id
            .0
            .strip_prefix(&format!("{}/", khor_core::kind::TUI))
            .ok_or_else(|| msg::no_such_session(&id.0))?;
        // A khor-opened row's own leaf is khor's mint and names no
        // vendor file; the hook-recorded vendor leaf is the bridge
        // (`live::Meta::vendor_leaf`). A discovered row has no record
        // and needs none — its leaf *is* the vendor's.
        let leaf = self.live.vendor_leaf_of(id).unwrap_or_else(|| leaf.to_owned());
        let vhome = adaptor::vendor_home(&self.root);
        let claude = adaptor::claude::Claude::at(vhome.join(".claude"));
        match claude.transcript(&leaf) {
            Ok(t) => Ok(t),
            // The other store's turn — the takeover fork's rule:
            // whoever resolves the leaf owns the session.
            Err(claude_why) => {
                let codex = adaptor::codex::Codex::at(vhome.join(".codex"));
                match codex.full_session_id(&leaf) {
                    Some(full) => codex.transcript(&full),
                    None => Err(claude_why),
                }
            }
        }
    }

    /// A discovered conversation, followed: what its transcript has said
    /// since byte `from`.
    ///
    /// **Following is the fast path, never the only one.** A leaf that
    /// resolves through neither vendor's store falls back to reading
    /// the record whole, marked `rewound` so the face repaints instead
    /// of appending.
    ///
    /// **Both vendors seek now.** The note here used to say only
    /// claude's reader could, "codex's rollout is a different shape" —
    /// and that was wrong in a way worth leaving a stone for: the
    /// rollout is line-delimited JSON, which is all this path needs.
    /// What differs between the two vendors is what a *line* means, and
    /// that difference was already sealed inside each store's own
    /// `utterances_of`. **A true statement about one layer was used to
    /// answer a question at another**, and the cost was a codex
    /// conversation that painted once and then froze — still going, and
    /// nothing on screen saying it had stopped being read.
    ///
    /// The fork is `transcript_of`'s: whoever resolves the leaf owns
    /// the session, claude asked first.
    ///
    /// **Written with a fallback because the first version was not**,
    /// and the smoke caught it: it failed where [`Node::transcript_of`]
    /// succeeds, and a first read that fails is a conversation the
    /// screen reports as having no record at all. A faster path that
    /// can *lose* a conversation is not a faster path.
    pub fn transcript_follow(
        &self,
        id: &SessionId,
        from: u64,
    ) -> Result<adaptor::claude::Follow, String> {
        let seek = id
            .0
            .strip_prefix(&format!("{}/", khor_core::kind::TUI))
            .map(|leaf| self.live.vendor_leaf_of(id).unwrap_or_else(|| leaf.to_owned()))
            .and_then(|leaf| {
                let vhome = adaptor::vendor_home(&self.root);
                adaptor::claude::Claude::at(vhome.join(".claude"))
                    .transcript_since(&leaf, from)
                    .ok()
                    .or_else(|| {
                        let codex = adaptor::codex::Codex::at(vhome.join(".codex"));
                        let full = codex.full_session_id(&leaf)?;
                        codex.transcript_since(&full, from).ok()
                    })
            });
        if let Some(follow) = seek {
            return Ok(follow);
        }
        // The whole record, once, for a session nothing can seek —
        // and `following: false` so the caller stops there. That last
        // field is the difference between this and a silent waste: the
        // first version answered the same way every time and the pane
        // re-read and repainted the whole transcript every 700 ms.
        Ok(adaptor::claude::Follow {
            said: self.transcript_of(id)?,
            at: 0,
            rewound: true,
            following: false,
        })
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

    /// Re-derives one session's row and pushes it to watchers. The
    /// user's name goes on here too, or the pushed row would disagree
    /// with the very list `sessions` answers.
    pub(crate) fn emit_row_of(&self, id: &SessionId) -> Result<(), String> {
        for k in self.kinds() {
            if !k.claims(self, id) {
                continue;
            }
            if let Some(mut row) = k.rows(self)?.into_iter().find(|r| r.id == *id) {
                if let Some(t) = self.titles_loaded()?.doc.title_of(&row.id.0) {
                    row.title = t;
                }
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
            let sid = format!("chat/{}", d.name);
            let msgs = self.log(&d.name)?.messages;
            // A put-away conversation is not derived (docs/SESSION.md
            // 动作: close 对话 = 收起那一行). The mark is a watermark on
            // the senders' clocks, so the next message stamps past it
            // and the row simply comes back — on every device at once,
            // since the mark replicates. Nothing is deleted; the log
            // read above is the very history a reopen paints.
            if let Some(closed) = seen.doc.closed_watermark(&sid) {
                let newest = msgs.iter().map(|m| m.at).max().unwrap_or(0);
                if newest <= closed {
                    continue;
                }
            }
            let wm = seen.doc.watermark(&sid);
            let mut row = self.row(&d.name, device_id_from_hex(&d.id)?, wm)?;
            // A file in flight presses on the machine's own row, now
            // that transfers no longer stand as rows of their own
            // (传输并入对话批 — `TransferKind::urgency` owns which seat
            // may say which word). More pressing wins; the row's own
            // message-based word already covers 完成/未看, because a
            // file message lands like any message.
            if let Some(dir) = channel_dir(node.root(), &d.name) {
                if let Some((word, at)) = node.transfer.urgency(&d.name, &dir, &msgs) {
                    if word.rank() < row.state.state.rank() {
                        row.state = khor_core::StateStamp {
                            state: word,
                            at: khor_core::Millis(at.max(0) as u64),
                        };
                    }
                }
            }
            out.push(row);
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

    /// Close is putting the row away, never deletion: the history
    /// replicates network-wide (a local delete would be pulled right
    /// back), and the received files belong to the filesystem now
    /// (传输并入对话批 corrected the earlier "close deletes received
    /// payloads"). What close *does* is record a put-away watermark at
    /// the newest message's clock — `rows` above stops deriving the row,
    /// and the next message stamps past the mark and brings it back,
    /// history intact (docs/SESSION.md 动作).
    ///
    /// It marks seen to the same instant: putting a conversation away is
    /// at least having looked at it, and a revived row must owe only the
    /// messages that revived it — old unread resurfacing with the row
    /// would be a debt the person already dismissed.
    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String> {
        let (channel, _) = node.channel_of_session(id)?;
        let newest = self.log(&channel)?.messages.iter().map(|m| m.at).max().unwrap_or(0);
        let loaded = node.seen_loaded()?;
        loaded.doc.mark(&id.0, newest)?;
        loaded.doc.put_away(&id.0, newest)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }
}

impl KindSurface for TransferKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        id.0.starts_with("transfer/")
    }

    /// No rows of its own any more (传输并入对话批, 2026-08-23): a
    /// transfer is a file message in the machine's conversation, and its
    /// pressure lands on that chat row's word (`ChatKind`'s rows via
    /// [`TransferKind::urgency`]). The kind itself stays: `claims`,
    /// `seen_at` and `close` still answer `transfer/` ids — accept marks
    /// them seen, and an older peer still *reports* transfer rows, which
    /// are painted as ever (nothing to migrate; those rows die of the
    /// stale fold as their khor upgrades).
    fn rows(&self, _node: &Node) -> Result<Vec<Session>, String> {
        Ok(Vec::new())
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

/// One device's last day record, as cached on disk (`cache_peer_day`).
#[derive(serde::Serialize, serde::Deserialize)]
struct DayReport {
    at_ms: u64,
    day: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct UsageReport {
    at_ms: u64,
    usage: khor_core::Usage,
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

/// The login shell's answer **added to** what this process already
/// had, in that order, without repeats.
///
/// **Added to, never swapped for.** A shell that answers with
/// something short or strange must not be able to take away
/// directories that were working — which is this whole bug pointing
/// the other way, and it would be the harder one to see. An empty
/// answer (no shell, a shell that failed) leaves what was there.
pub(crate) fn merge_paths(asked: &str, here: &str) -> String {
    if asked.trim().is_empty() {
        return here.to_owned();
    }
    let mut seen: Vec<&str> = Vec::new();
    for dir in asked.split(':').chain(here.split(':')) {
        if !dir.is_empty() && !seen.contains(&dir) {
            seen.push(dir);
        }
    }
    seen.join(":")
}

/// Told to khor by hand: "here is where the programs are." Read by
/// [`Node::user_path`] and handed down to spawned `_host` processes so
/// they inherit the answer instead of re-deriving it.
pub(crate) const PATH_ENV: &str = "KHOR_PATH";

/// The three that decide a program's character encoding, most
/// authoritative first — which is POSIX's order, not a preference.
pub(crate) const LOCALE_VARS: [&str; 3] = ["LC_ALL", "LC_CTYPE", "LANG"];

/// Whether a locale value names UTF-8. Both spellings are in the wild
/// (`zh_CN.UTF-8`, `en_US.utf8`), and macOS Terminal's bare `UTF-8`
/// is a third.
pub(crate) fn says_utf8(value: &str) -> bool {
    let v = value.to_ascii_lowercase().replace('-', "");
    v.contains("utf8")
}

/// What the person's login shell says, asked **once per process** —
/// the one place that runs it.
///
/// `$SHELL -lic` is what every GUI developer tool on this platform
/// does about this, for want of anything better: the login shell is
/// the only thing that knows what the user's environment is, because
/// the user's environment is something their rc files compute.
///
/// **The cost is real and it is why this is cached**: measured 0.78s
/// cold on this machine, and it *runs the user's rc files*, side
/// effects and all. One probe answers both questions asked of it — a
/// second `Command` for the locale would double a price already paid.
pub(crate) struct Asked {
    pub path: String,
    pub locale: Vec<(&'static str, String)>,
}

pub(crate) fn asked_shell() -> &'static Asked {
    static ASKED: std::sync::OnceLock<Asked> = std::sync::OnceLock::new();
    ASKED.get_or_init(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let out = std::process::Command::new(&shell)
            .args(["-lic", &probe_script()])
            .stdin(std::process::Stdio::null())
            .output();
        let said = match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
            _ => String::new(),
        };
        read_shell_answer(&said)
    })
}

/// The one line the login shell is asked to run.
///
/// `-l` for the login files, `-i` because on zsh the interactive ones
/// are where a `PATH` is usually built. NUL between the fields because
/// every one of them can hold anything a filesystem can.
///
/// **Built here rather than written inline so it can be run in a
/// test**, and that is not tidiness. The first draft of this spelled
/// the separator `"\0"` in Rust — a real NUL *character* in the argv,
/// where what `printf` needs is the two characters `\` and `0`. A NUL
/// in an argument makes `Command::spawn` fail outright, and this
/// function's error arm is an empty answer: the probe became a silent
/// no-op, `merge_paths` handed back the `PATH` that was already there,
/// and **the repair this whole family of functions exists for
/// quietly stopped happening while every test stayed green.** The
/// branch that could not connect did not become "no value"; it became
/// the value next door.
pub(crate) fn probe_script() -> String {
    format!(
        "printf '%s\\0' \"$PATH\" {}",
        LOCALE_VARS.iter().map(|v| format!("\"${v}\"")).collect::<Vec<_>>().join(" ")
    )
}

/// [`asked_shell`]'s parsing, split out so it is testable without a
/// login shell in the loop.
///
/// **A shell that answered nothing gives nothing**, and a locale that
/// does not name UTF-8 is dropped rather than carried: the whole point
/// is the encoding, and passing a non-UTF-8 one along could only take
/// away a UTF-8 locale that was already in place.
pub(crate) fn read_shell_answer(said: &str) -> Asked {
    let mut fields = said.split('\0');
    let path = fields.next().unwrap_or_default().trim().to_owned();
    let locale = LOCALE_VARS
        .iter()
        .zip(fields)
        .filter(|(_, value)| says_utf8(value.trim()))
        .map(|(name, value)| (*name, value.trim().to_owned()))
        .collect();
    Asked { path, locale }
}

/// The `PATH` to spawn `command` against: **this process's own, unless
/// this process's own cannot find it.**
///
/// Asking the login shell is a repair, not a policy. It costs 0.78s
/// cold and it runs the user's rc files, so a khor that already has a
/// working `PATH` — every khor started from a terminal — must not pay
/// it. A khor that does not (launchd's `khor serve`, whose four
/// directories hold neither `tmux` nor `npx`) pays it once per process
/// and gets a `PATH` that can answer.
///
/// **The fallback is still allowed to fail**, and it should: when the
/// program is genuinely not installed, the caller gets to say so and
/// name where it looked ([`msg::command_not_found`]) rather than hand
/// a child an exec error nobody will ever read.
pub(crate) fn spawn_path(command: &str) -> String {
    let here = std::env::var("PATH").unwrap_or_default();
    if repair_needed(command, &here) {
        return Node::user_path().to_owned();
    }
    here
}

/// [`spawn_path`]'s judgment, split out so it is testable without a
/// login shell in the loop: whether this process's own `PATH` has to
/// be gone beyond for `command`.
pub(crate) fn repair_needed(command: &str, here: &str) -> bool {
    find_program(command, here).is_none()
}

/// Resolves a program name the way a shell would, against a stated
/// `PATH`: a name with a separator in it is taken as written, anything
/// else is looked for in each directory in turn.
///
/// **Executable, not merely present.** A directory named `npx`, or a
/// data file of that name, is not something that starts — and a lookup
/// that answered "found" for one would move the failure back to where
/// it was, a child that dies with nothing to say.
pub(crate) fn find_program(command: &str, path: &str) -> Option<String> {
    let runnable = |p: &std::path::Path| -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            p.is_file()
        }
    };
    if command.contains('/') {
        let p = std::path::Path::new(command);
        return runnable(p).then(|| command.to_owned());
    }
    path.split(':')
        .filter(|d| !d.is_empty())
        .map(|d| std::path::Path::new(d).join(command))
        .find(|p| runnable(p))
        .map(|p| p.display().to_string())
}

#[cfg(test)]
mod tests;
