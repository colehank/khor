//! One device's node: identity, the session surface (docs/SESSION.md),
//! and the live link to the rest of the network (docs/NET.md).
//!
//! The kind trait ([`KindSurface`]) was cut only when the third kind
//! (live) arrived — the first two shared nothing but "produce rows",
//! and a trait cut from that would have frozen an accident. What three
//! real implementations actually share: produce rows, claim an id,
//! answer what "looked at now" means, and close.

// The shared integration wire helpers also exercise this very unit-test
// Node, including its private observers, without a second crate instance.
#[cfg(test)]
extern crate self as khor_node;

pub mod a2a_http;
pub mod accounts;
mod acp_management;
mod acp_mcp;
pub(crate) mod acp_process;
pub mod adaptor;
#[path = "khor_agent_process.rs"]
pub(crate) mod agent_process;
mod agent_setup;
pub mod answers;
pub mod backup;
pub(crate) mod beside;
pub mod cagent;
pub mod carry;
pub mod chat;
pub mod code;
mod codex_process;
pub mod codex_terminal;
pub mod codexagent;
pub mod commands;
pub mod config;
pub mod cwd;
pub mod day;
pub mod delegation;
mod doc_transfer_state;
pub mod doctor;
pub mod drops;
pub(crate) mod egress;
pub mod face;
pub mod favicon;
pub mod files;
pub mod git;
pub mod gui_host;
pub mod hardware;
pub mod history;
pub(crate) mod hop;
pub mod host;
pub mod integration;
pub mod ipc;
pub mod join;
pub mod journal;
pub mod keeper;
pub mod khor_agent;
pub mod legacy_codex;
pub mod link;
pub mod list;
pub mod live;
pub mod lock;
pub mod mcp;
pub mod mcp_worker;
pub mod merge;
pub mod modes;
pub(crate) mod once;
pub mod pause;
pub mod pictures;
pub mod pipe;
pub mod place;
pub mod post;
pub mod preview;
pub mod proto;
pub mod quota;
pub mod reach;
pub mod reactor;
pub mod recognition;
pub(crate) mod remote_cancel;
pub mod road;
pub mod roadbook;
pub(crate) mod roads;
pub mod satellite;
pub mod secret;
pub mod serves;
pub mod shell_start;
mod store_owner;
pub mod subscriptions;
pub mod tabs;
#[cfg(test)]
pub(crate) mod testdir;
pub mod top;
pub mod transfer;
pub mod tree;
pub mod trust;
pub mod tunnel;
pub mod usage;
pub mod vitals;
pub mod watch;
pub mod wide;
pub(crate) mod wires;
pub mod workspace;
pub(crate) mod wsfacts;

pub use khor_core::avatar::{
    Avatar, AvatarSeed, AvatarStyle, FaceShape, PRESETS, Palette, Preset, Variant, avatar, preset,
};
pub use khor_core::{
    CpuDay, Fill, Hop, Leg, LegKind, Link, Load, ReachOutcome, ReachSeen, ReachView, Road,
    RoadKind, RoadTally, Session, SessionId, State, Tokens, TopBy, TopRow, Usage, UsageDay, Vitals,
    WireCounts, kind,
};
/// A relay and the key it admits by — what [`Node::set_relays`] takes.
pub use khor_net::endpoint::Relay;
pub use khor_sync::chat::{
    AgentApproval, AgentConfig, AgentTurn, CommandRecord, Fate, FileRef, Message, MsgBody,
    Step as FateStep,
};
pub use khor_sync::devices::DeviceInfo;
pub use trust::{PendingRow, RootState, Standing};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use store_owner::Owner;

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
use khor_catalog::msg;
use khor_core::{DeviceId, Event};
/// Re-exported for the reason `AgentSpec` is: the faces and the CLI
/// register accounts, and they should name the account's shape without
/// depending on the sync crate for it.
pub use khor_sync::accounts::{Account, AccountKind, Provider};
use khor_sync::accounts::{AccountDoc, accounts_dir};
/// How one registered ACP agent starts (`khor_sync::agents::Spec`).
/// Re-exported because `Node::agents` hands it out, and a type a public
/// method returns that callers cannot name is a method they cannot use.
pub use khor_sync::agents::Spec as AgentSpec;
/// The shipped snapshot of the ACP registry (`khor_sync::agents::registry`).
/// Re-exported here for the same reason `AgentSpec` is: the CLI talks to
/// the node, and a second path into the sync crate would be a second
/// place to keep in step.
pub use khor_sync::agents::registry as agent_registry;
use khor_sync::agents::{AgentDoc, agents_dir};
/// One conversation an agent said it has, as khor remembers it
/// (`khor_sync::agents::Seen`).
pub use khor_sync::agents::{Answer as AgentAnswer, Seen as AgentSeen};
use khor_sync::chat::{ChatDoc, Sender, channel_of_machine};
use khor_sync::devices::{DeviceDoc, devices_dir};

/// What asking an agent for its sessions answered
/// (`khor_acp::Listing`), and one of them.
///
/// Re-exported for the reason `AgentSpec` and `agent_registry` are: the
/// CLI and the faces talk to the node, and a second path into the
/// protocol crate is a second place to keep in step.
pub use khor_acp::{Listing, Recorded};
use khor_sync::dirpins::{DirPinDoc, dirpins_dir};
use khor_sync::pins::{PinDoc, pins_dir};
/// What sort of place a recorded visit was to
/// (`khor_sync::recents::Kind`). Re-exported because `Node::recents`
/// takes and reports one, and a type a public method demands that
/// callers cannot name is a method they cannot call.
pub use khor_sync::recents::Kind as PlaceKind;
use khor_sync::recents::{RecentDoc, recents_dir};
use khor_sync::roster::{RosterDoc, roster_dir};
pub use khor_sync::seen::SeenMessage;
use khor_sync::seen::{SeenDoc, seen_dir};
use khor_sync::store::{Loaded, load_pile};
use khor_sync::titles::{TitleDoc, titles_dir};
use khor_sync::webpins::{WebPinDoc, webpins_dir};
use khor_sync::workspaces::{State as WsState, WorkspaceDoc, workspaces_dir};

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
    /// On the list because somebody imported it (`khor_sync::roster`,
    /// docs/SHELL.md 第八轮) — so 删除 takes it off the list and leaves
    /// the session itself alone, and the delete window says exactly that.
    pub imported: bool,
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
    matches!(
        session.state.state,
        State::Busy | State::Blocked | State::Errored
    ) && session.unbacked
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
    /// The session it was last visited from, where one was said
    /// (`khor_sync::recents`' origins, UX-37 §1).
    pub session: Option<String>,
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

/// One borrow of another machine's network that this serve is hosting
/// (docs/NET.md 借网): the task holding its local proxy port, plus what a
/// second ask and the reaper each need to know about it.
pub(crate) struct HostedBorrow {
    /// The exit machine. A second ask for the same one is answered with
    /// this borrow rather than a second lease.
    pub(crate) machine: String,
    /// Where its proxy listens — the address that second ask is given.
    pub(crate) addr: std::net::SocketAddr,
    /// **Whether it wears a session row** (docs/NET.md 可达性, 2026-08-17
    /// 定形: 一层不占会话列表). True only for a borrow a person holds by
    /// hand (`khor borrow`) — that one has no other row to show itself on
    /// and nowhere else to hang its lifetime, which is exactly why it is
    /// a session. A browsing tab's borrow has both, so it is unlisted.
    ///
    /// It is here because it decides two things that would otherwise be
    /// guessed: whether `reap_borrows` may ask `claims` about this borrow
    /// (an unlisted one has no row, so the answer is always "gone"), and
    /// whether a second ask may share it (it may not across the two —
    /// closing a row must not pull the network out from under a page).
    pub(crate) listed: bool,
    pub(crate) task: tokio::task::JoinHandle<()>,
}

/// The pipes this node is holding open, by whatever it keys them under:
/// where the near side listens, which far port it was built for, and the
/// task carrying it. A `type` only, to keep `clippy::type_complexity`
/// off the two fields below that share this shape.
pub(crate) type PipeMap<K> = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<K, (std::net::SocketAddr, u16, tokio::task::JoinHandle<()>)>,
    >,
>;

/// The digest of the session rows last sent one asker, and the rows
/// (`Node::rows_sent`).
pub(crate) type RowsSent = (Vec<u8>, Vec<khor_core::Session>);

/// The entry point that deliberately changed a machine's face. Stored
/// with the style in one LWW value, so provenance cannot describe a
/// different winning style after sync.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvatarChangeOrigin {
    Api,
    Cli,
    Gui,
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
    /// Khor tier plus `KHOR_RELAY` plus the user's own (docs/NET.md 中继).
    /// Urls only — this is the list that is published.
    relays: Vec<String>,
    /// The same relays with the keys they are entered with: what the
    /// bind hands iroh. Kept apart from `relays` so that nothing which
    /// publishes a road can reach a key by accident.
    relay_map: Vec<khor_net::endpoint::Relay>,
    /// Whether `relays` is the user's own list *instead of* the Khor
    /// tier rather than on top of it.
    relays_only_mine: bool,
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
    /// Rounds with other machines share the stores, compaction rewrites
    /// them: every round holds this for reading, many at once (one per
    /// machine, `link::Node::sync_peers`), and compaction takes it for
    /// writing only when no round is running. Block files are named by
    /// author and sequence (`khor_sync::store`), so rounds in parallel
    /// never write the same file.
    pub(crate) store_gate: tokio::sync::RwLock<()>,
    /// The sync round running with each machine, by device id, for a
    /// second asker to wait on instead of starting another
    /// (`link::Node::sync_peers`).
    pub(crate) rounds: Arc<Mutex<std::collections::HashMap<String, link::RoundWatch>>>,
    /// The readings this node last got from each machine, by (device id,
    /// kind), with the digest it hands back next round so an unchanged
    /// reading does not cross again (round NET-3, `Request::Versions`).
    /// Memory only: after a restart the first round asks for everything.
    pub(crate) readings_held:
        Arc<Mutex<std::collections::HashMap<(String, String), link::HeldReading>>>,
    /// The day this node last got from each machine's `Load`, by device
    /// id, with its stamp — handed back on the next ask so an unchanged
    /// day does not cross again (round NET-4, `Request::LoadSince`).
    pub(crate) days_held: Arc<Mutex<std::collections::HashMap<String, (u64, khor_core::CpuDay)>>>,
    /// The directories faces asked to have watched on other machines,
    /// and what changed in them (round NET-4, `watch::Watches`).
    pub(crate) watches: Arc<watch::Watches>,
    /// **Gone to the background** (`link::suspending`) until `resume`:
    /// no pump, no visits, no rounds — see the serve loop.
    pub(crate) resting: Arc<std::sync::atomic::AtomicBool>,
    /// When a face last said it is on screen (`ipc::Op::FaceHere`), or
    /// `None` since one said it left: what paces the pump (round NET-6).
    pub(crate) face_seen: Arc<Mutex<Option<std::time::Instant>>>,
    /// When a round out of turn last started, in ms (`link::Node::nudge`).
    pub(crate) nudged: Arc<std::sync::atomic::AtomicU64>,
    /// When a machine page last asked for a round with each machine, by
    /// id (`ipc::Op::Visit`, `link::Node::visit_due`).
    pub(crate) visited: Arc<Mutex<std::collections::HashMap<String, std::time::Instant>>>,
    /// When this node last vouched for itself to each machine, by id
    /// (`link::Node::vouch_to`, round NET-10).
    pub(crate) vouched: Arc<Mutex<std::collections::HashMap<String, std::time::Instant>>>,
    /// When a request from another machine last arrived, in ms; 0 for
    /// never (round NET-11: a machine reached is not deaf).
    pub(crate) reached_at: Arc<std::sync::atomic::AtomicU64>,
    /// Whether the ears this node answers on ever had a home relay.
    pub(crate) ears_homed: Arc<std::sync::atomic::AtomicBool>,
    /// Whether this machine has a relay to go out by (`egress`, C7).
    pub(crate) egress: Arc<Mutex<egress::Egress>>,
    /// The reason the deaf watch last said it stayed, so it says each
    /// once (`link::Node::say_once`).
    pub(crate) deaf_said: Arc<Mutex<Option<&'static str>>>,
    /// What this machine has written about its own roads, and what is
    /// waiting to settle (`roadbook`, round NET-12).
    pub(crate) roadbook: Arc<Mutex<roadbook::RoadBook>>,
    /// The session rows last sent each asker, by its id, with the digest
    /// of that frame (`proto::Response::SessionRowsSince`).
    pub(crate) rows_sent: Arc<Mutex<std::collections::HashMap<String, RowsSent>>>,
    /// This machine's workspace facts, kept by the resident
    /// (`wsfacts::WsFacts`, round NET-8); `None` outside it.
    pub(crate) wsfacts: Arc<Mutex<Option<Arc<wsfacts::WsFacts>>>>,
    /// The borrow proxies this serve hosts (docs/NET.md 借网), keyed by
    /// their session id. Each is a task holding a local proxy port and a
    /// lease to the exit; the serve reaps one once its session row is
    /// gone (a `close` from any process removes the row), which is also
    /// what frees the port. Empty on a node that is not the resident
    /// serve.
    ///
    /// **The exit machine is in the value so a second ask for the same
    /// one finds the first** (`start_borrow`). It is not the key because
    /// reaping and closing both arrive holding a session id and nothing
    /// else; a machine-keyed map would make those two walk the whole
    /// table to answer a question the id already answers.
    pub(crate) borrows: Arc<tokio::sync::Mutex<std::collections::HashMap<SessionId, HostedBorrow>>>,
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
    pub(crate) reaches: PipeMap<SessionId>,
    /// The pipes this machine holds in front of somebody else's VS Code
    /// (工作区批5), keyed by machine.
    ///
    /// **A separate map from `reaches` above, and not a generalisation
    /// of it.** That one is keyed by a session id and reuses a pipe
    /// when the far *port* still matches, because a host that died and
    /// came back is the failure it was written for. This one is keyed
    /// by a directory and its far side is a server khor started itself
    /// — so「is it still the same port」is a question about a child
    /// process khor is holding, answered where that child is held
    /// (`code::serve`), not by remembering a number here. One map with
    /// two meanings would have to carry both stories.
    pub(crate) codes: PipeMap<String>,
    /// The wires this node holds to other machines — one long-lived
    /// connection per machine and protocol (`crate::wires` has the
    /// lifecycle and why). Shared so a `WireWatch` taken before `serve`
    /// consumes the node still sees what the serve does with them.
    pub(crate) wires: Arc<wires::Wires>,
    /// The resident's own handle on itself, set by `serve`: what lets a
    /// task the node spawns (a borrow that re-fetches its wire, a far
    /// stream answered concurrently) outlive the call that started it.
    /// Empty on a node that is not serving.
    pub(crate) me: std::sync::OnceLock<std::sync::Weak<Node>>,
    // Bound on first agent API use or explicit resident acquisition. Opening
    // a Node alone never acquires a process-local agent runtime.
    pub(crate) agent_generation: std::sync::OnceLock<u64>,
    /// The endpoint the resident answers on now; replaced with fresh
    /// ears. What a long-lived task reads instead of an endpoint it
    /// captured, which may since have been closed.
    pub(crate) ears: Mutex<Option<Arc<iroh::Endpoint>>>,
    /// Whether this node's resident is an app's (`serve_resumable`) —
    /// written into `endpoint.json` so `khor quit` leaves it alone.
    pub(crate) app_resident: std::sync::atomic::AtomicBool,
    /// Whether opening this node made the network's root key — the one
    /// moment a person must be told to write its words down
    /// (`trust::Node::bootstrap_trust`).
    root_born: bool,
    /// How this node's recent dials went, per far machine
    /// (`crate::roads`). Only the resident's is ever read — through
    /// `ipc::Op::Roads` — because only it lives long enough to have
    /// dialled anything more than once.
    pub(crate) roads: roads::Roads,
    /// **The roads this machine carries itself** (round ROAD-2,
    /// `khor_net::carry`): ssh sessions to members that reach nothing
    /// else, and hops through a third machine. One table for the life of
    /// the node; every bind of the resident registers it, unless both
    /// switches are off ([`Node::carrying`]).
    pub(crate) carry: khor_net::carry::Links,
    /// Whether this machine can host a session (`khor_core::hosts_here`,
    /// docs/IOS.md): `false` on a phone. Read once at open and written to
    /// this machine's own row in the device table, so every machine in
    /// the network knows; every door that would start a process here
    /// asks it first ([`Node::refuse_unless_hosting`]).
    hosts: bool,
}

/// A read-only look at the wires a node is holding (`Node::wires`).
///
/// **It exists because reuse is otherwise unobservable.** A second ask
/// riding the first ask's wire looks, from outside the process, exactly
/// like two asks on two wires: two asks, two answers. A test that could
/// only assert "both asks answered" would pass either way. This is the
/// arm that can tell them apart.
///
/// It hands out names and counts, never the connections: nothing
/// outside the node may send on a wire the node is holding.
#[derive(Clone)]
pub struct WireWatch(Arc<wires::Wires>, Vec<(String, String)>);

impl WireWatch {
    pub(crate) fn of(wires: &Arc<wires::Wires>, names: Vec<(String, String)>) -> Self {
        Self(wires.clone(), names)
    }

    /// The machines this node holds a live wire to, by name, sorted.
    pub async fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .0
            .held()
            .into_iter()
            .map(|id| {
                self.1
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or(id)
            })
            .collect();
        names.sort();
        names
    }

    /// Whether the wire held for that machine is up.
    pub async fn is_open(&self, channel: &str) -> Option<bool> {
        let id = self
            .1
            .iter()
            .find(|(_, n)| n == channel)
            .map(|(i, _)| i.clone())?;
        Some(self.0.held().contains(&id))
    }

    /// The wire counters (`khor_core::WireCounts`).
    pub fn counts(&self) -> khor_core::WireCounts {
        self.0.counts.snapshot()
    }
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
/// Which khor this is.
///
/// **Published here rather than read with `env!` at each caller**, and
/// the reason is the thing the number exists to answer: 「哪台落后了」.
/// Every crate in this workspace carries `version.workspace = true`, so
/// `env!("CARGO_PKG_VERSION")` in the CLI, in the GUI shell and in the
/// library are three spellings of one value **today** — and the day a
/// crate is published on its own, they stop being. One of the three
/// would then print a different number than the other two, and nothing
/// would say which was the one a person installed.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// How long a machine's label may be, in characters not bytes — the
/// limit is about how much of a row a name may eat, and a CJK name says
/// far more per character than a Latin one. Generous: this is a guard
/// against a pasted paragraph, not a style rule.
const MAX_LABEL: usize = 64;

/// The channel name this store joined the network under, settled once
/// and then read back for the life of the store.
///
/// # Why it is not recomputed from the hostname every time
///
/// `chat/<machine name>` is a **directory**, and the key that
/// [`khor_sync::seen`] and [`khor_sync::pins`] use for that window
/// network-wide. Derive it from the current hostname and renaming a box
/// opens a second, empty window, leaves the conversation in the old
/// directory, and strands the read watermark and the pin on a key
/// nothing points at any more. Nothing errors; the history is simply
/// somewhere else.
///
/// So the first open writes the name down and every later open reads it
/// — the same shape as `identity.key`, and for the same reason: what
/// the network calls this machine is decided once.
///
/// **This is the line between the two names**, and it is worth stating
/// plainly because renaming is now a feature:
///
/// - **channel** — what the network files this machine under. Settled
///   here, never edited, boring on purpose.
/// - **label** — what people see, changed whenever they like
///   (`Node::rename_machine`, `DeviceDoc::set_label`).
///
/// A rename moves the second and never the first, which is why it costs
/// nothing. The visible cost is the other way round: renaming the *OS*
/// hostname no longer changes what khor shows. That is the trade, and
/// it is the right way round now that khor has a rename of its own.
fn settled_channel(root: &std::path::Path, computed: String) -> Result<String, String> {
    let dir = root.join(".khor");
    let path = dir.join("channel");
    if let Ok(text) = std::fs::read_to_string(&path) {
        let kept = text.trim();
        // A hand-emptied or hand-broken file falls through to settling
        // again rather than taking the store down: the worst case is
        // the behaviour khor had before this existed.
        if let Some(ok) = khor_sync::chat::channel_of_machine(kept)
            && ok == kept
        {
            return Ok(ok);
        }
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Best effort: a store that cannot write this still works, it just
    // settles again next time — which is today's behaviour.
    let _ = std::fs::write(&path, &computed);
    Ok(computed)
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
/// # The stamp used to be the hostname, and that made a rename fatal
///
/// A hostname is a **label**, which is the thing people change; the
/// claim needs to be about the **box**. Measured 2026-09-11: a store
/// whose stamp and hostname disagreed refused to open with exit 1, and
/// the refusal advised setting `KHOR_HOME` — advice for a shared home,
/// which on a renamed machine produces a second store and a second
/// identity. **A wrong diagnosis is worse than none**: it sends the
/// person somewhere.
///
/// So the claim is [`crate::hardware::fingerprint`], and the name rides
/// along only as a note for whoever opens the file. Where there is no
/// fingerprint the old rule still applies, unchanged — see
/// [`Owner::is_mine`].
///
/// **Never `KHOR_NAME`**, then or now: a store must not be reachable or
/// unreachable depending on an environment variable.
///
/// # On a phone, the store stays on the phone
///
/// Claimed or freshly made, the store is then kept out of the system's
/// backups ([`backup::keep_out`], docs/IOS.md 第二步): a backup restored
/// onto a new phone would otherwise arrive with this phone's identity
/// key, and one key on two devices is the copied store this stamp exists
/// to refuse — except that the stamp would travel with it.
fn claim_store(root: &std::path::Path) -> Result<(), String> {
    claim_store_stamp(root)?;
    backup::keep_out(&root.join(".khor"));
    Ok(())
}

/// # On a phone, the stamp cannot refuse the store
///
/// On iOS the fingerprint is `identifierForVendor`, and Apple computes it
/// differently by install channel: a development install (cable, Xcode)
/// and a TestFlight or App Store install of the same app get different
/// ids, and so do installs before and after deleting every app of the
/// vendor. The owner met it on 2026-10-03: the TestFlight build installed
/// over the cable build refused its own store as 「localhost 的」 — both
/// names are the phone's `localhost`, only the id had moved.
///
/// And the refusal protects nothing there. A store in the app's sandbox
/// can only be on this device — no shared home reaches it, and the one
/// way out, a backup restored onto another phone, is closed by
/// [`backup::keep_out`]. So on iOS the store is this machine's whatever
/// the stamp says, and the stamp is rewritten with today's fingerprint
/// and name (the 「fallen behind」 branch below). macOS and Linux keep the
/// check: there the store is a directory any machine on the mount can open.
fn claim_store_stamp(root: &std::path::Path) -> Result<(), String> {
    let here = gethostname::gethostname().to_string_lossy().to_string();
    stamp_store(
        root,
        &here,
        hardware::fingerprint(),
        cfg!(target_os = "ios"),
    )
}

/// [`claim_store_stamp`] with what it asks of the machine handed in:
/// the name, the fingerprint, and whether the store lives in a sandbox
/// only this device can reach (iOS), where the stamp may not refuse.
fn stamp_store(
    root: &std::path::Path,
    here: &str,
    fp: Option<String>,
    sandboxed: bool,
) -> Result<(), String> {
    let dir = root.join(".khor");
    let stamp = dir.join("owner");
    let here = here.to_string();
    let mine = Owner {
        machine: fp.clone(),
        name: Some(here.clone()),
    };
    let Ok(text) = std::fs::read_to_string(&stamp) else {
        // Written on the way in, before the key exists: a store that
        // gets a key without a stamp is a store the next machine cannot
        // tell apart from its own.
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        return std::fs::write(&stamp, mine.render()).map_err(|e| e.to_string());
    };
    let owner = Owner::parse(&text);
    if !sandboxed && !owner.is_mine(&here, fp.as_deref()) {
        // Essentially always a name, because this writes both; the
        // other branch is a hand-edited file, and a fingerprint stub is
        // at least something the person can compare against a file.
        let theirs = owner.name.clone().unwrap_or_else(|| {
            owner
                .machine
                .as_deref()
                .map(|m| m[..8.min(m.len())].to_string())
                .unwrap_or_default()
        });
        return Err(msg::store_belongs_to(&theirs, &here));
    }
    // **Mine, and the stamp has fallen behind.** An old bare-hostname
    // file gets its fingerprint, and a renamed machine gets its new
    // name written down — so the next reader of this file is looking at
    // the name the machine actually goes by. Best effort on purpose:
    // failing to freshen a note must not stop a store from opening.
    if owner != mine && mine.machine.is_some() {
        let tmp = stamp.with_extension(format!("tmp{}", std::process::id()));
        if std::fs::write(&tmp, mine.render().as_bytes()).is_ok() {
            let _ = std::fs::rename(&tmp, &stamp);
        }
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(())
}

/// Where this machine's own relays are kept. A local preference file,
/// beside the avatar's and for the same reason: **the choice is made
/// here and only travels outward**, through the device table and the
/// pairing ticket. No other machine can point this one at a relay.
fn relay_prefs_path(root: &std::path::Path) -> PathBuf {
    root.join(".khor").join("relays.json")
}

/// What this machine's relay file says: the list the user typed, and
/// whether it is meant to be the whole story.
#[derive(Debug, Default, Clone)]
pub(crate) struct RelayPlan {
    pub(crate) mine: Vec<khor_net::endpoint::Relay>,
    /// Use `mine` (and `KHOR_RELAY`) **instead of** the Khor tier,
    /// rather than alongside it.
    pub(crate) only_mine: bool,
    /// The relay to home on when reachable (首选中继), by url.
    pub(crate) preferred: Option<String>,
}

/// Reads the relay file, or gives an empty plan.
///
/// An object on disk rather than a bare array, and this flag is why —
/// it was the question the first version could not answer, and adding
/// it here cost a key instead of a migration.
///
/// **Each entry is `{"url": …, "token": …, "qad": …, "pkarr": …}`**
/// (all but `url` optional; `qad` a UDP port, `pkarr` a url — what the
/// server serves besides relaying, `endpoint::Relay`), **or a bare url
/// string**:
/// the string is what the file held before relays took keys, and a
/// machine that upgrades keeps the relays it had rather than coming up
/// on the Khor tier alone with the user's list silently gone.
///
/// Unreadable, unparseable and half-parseable all yield what they can
/// and no error — reaching this path means somebody edited the file by
/// hand, since [`Node::set_relays`] validates before it writes. An entry
/// that does not parse is dropped here rather than at bind, matching
/// `endpoint::dial_addr`: one typo must not cost the machine the relays
/// it typed correctly.
///
/// ⚠️ **`only_mine` is forced back off when the list ends up empty.**
/// Otherwise a hand-edited file could leave this machine with no relay
/// at all — reachable only where a hole punch lands — which is not
/// something anybody chooses by deleting the last row. The write door
/// refuses that combination outright; this is the second net.
fn chosen_relays(root: &std::path::Path) -> RelayPlan {
    let Ok(text) = fs::read_to_string(relay_prefs_path(root)) else {
        return RelayPlan::default();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return RelayPlan::default();
    };
    let mut mine: Vec<khor_net::endpoint::Relay> = Vec::new();
    for v in doc
        .get("relays")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let (url, token, qad, pkarr) = match v {
            serde_json::Value::String(url) => (url.as_str(), None, None, None),
            serde_json::Value::Object(o) => (
                o.get("url").and_then(|u| u.as_str()).unwrap_or(""),
                o.get("token")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .filter(|t| !t.is_empty()),
                o.get("qad")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|p| u16::try_from(p).ok()),
                o.get("pkarr")
                    .and_then(|t| t.as_str())
                    .map(str::trim)
                    .filter(|u| u.parse::<iroh::RelayUrl>().is_ok())
                    .map(str::to_string),
            ),
            _ => continue,
        };
        let url = url.trim();
        if url.is_empty() || url.parse::<iroh::RelayUrl>().is_err() {
            continue;
        }
        if mine.iter().any(|have| have.url == url) {
            continue;
        }
        mine.push(khor_net::endpoint::Relay {
            url: url.to_string(),
            token,
            qad,
            pkarr,
        });
    }
    let only_mine = !mine.is_empty()
        && doc
            .get("only_mine")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
    let preferred = doc
        .get("preferred")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|u| u.parse::<iroh::RelayUrl>().is_ok())
        .map(str::to_owned);
    RelayPlan {
        mine,
        only_mine,
        preferred,
    }
}

/// Every relay a node on this home should bind with, keys included.
///
/// Two shapes, and the flag picks between them: **added** to the Khor
/// tier and `KHOR_RELAY` (the default — iroh probes them all and homes
/// on the closest that answers, so one more entry can only widen the
/// set of networks this machine is reachable on), or **instead of the
/// Khor tier**, for somebody who wants khor touching no relay they do
/// not run. `KHOR_RELAY` is kept in both shapes: it is this machine's
/// own relay as much as the typed ones are, only named in a different
/// place (docs/NET.md 中继).
///
/// n0's public relays are in neither shape: they are the fallback tier
/// `endpoint::bind_with` adds behind this list unless 「only mine」 is
/// on — so 「only mine」 means 「not n0 and not Khor」, a machine that
/// touches no relay its owner did not name.
fn relays_under(root: &std::path::Path) -> (Vec<khor_net::endpoint::Relay>, bool) {
    let plan = chosen_relays(root);
    let mut out = if plan.only_mine {
        khor_net::endpoint::env_relays()
    } else {
        khor_net::endpoint::configured_relays()
    };
    for entry in plan.mine {
        if !out.iter().any(|have| have.url == entry.url) {
            out.push(entry);
        }
    }
    (out, plan.only_mine)
}

impl Node {
    /// Opens the node rooted at `root`, named after the hostname (or
    /// `KHOR_NAME` when set — tests and same-machine dual instances).
    /// Admission and filesystem work block this calling thread.
    pub fn open(root: PathBuf) -> Result<Node, String> {
        Self::open_as(root, &Self::opening_name())
    }

    fn opening_name() -> String {
        std::env::var("KHOR_NAME")
            .unwrap_or_else(|_| gethostname::gethostname().to_string_lossy().to_string())
    }

    /// Control admission never waits behind an ordinary constructor.
    /// Admitted filesystem work still blocks this calling thread.
    pub fn try_open(root: PathBuf) -> Result<Node, String> {
        let permit = doc_transfer_state::RootPermit::try_acquire(&root).map_err(String::from)?;
        Self::open_admitted(root, &Self::opening_name(), khor_core::hosts_here(), permit)
    }

    /// Wait only for local admission, then construct on this same future/thread.
    /// Requires a current Tokio runtime with its time driver enabled for the
    /// bounded admission deadline. No runtime is created; sync callers use `open`.
    /// After admission, filesystem work is synchronous and nonpreemptible in
    /// every runtime flavor, including LocalSet; it can block the executor thread.
    pub async fn open_async(root: PathBuf) -> Result<Node, String> {
        Self::open_hosting_async(root, Self::opening_name(), khor_core::hosts_here()).await
    }

    async fn open_hosting_async(root: PathBuf, name: String, hosts: bool) -> Result<Node, String> {
        tokio::runtime::Handle::try_current().map_err(|_| {
            "async Node opening requires a Tokio runtime; use Node::open".to_owned()
        })?;
        let permit = doc_transfer_state::RootPermit::acquire_async(&root)
            .await
            .map_err(String::from)?;
        Self::open_admitted(root, &name, hosts, permit)
    }

    /// Opens with an explicit machine name (pre-cleaning).
    pub fn open_as(root: PathBuf, name: &str) -> Result<Node, String> {
        Self::open_hosting(root, name, khor_core::hosts_here())
    }

    /// [`Node::open_as`], told whether this machine can host rather than
    /// reading it off the build.
    ///
    /// Every production entry passes the build's truth (`khor_core::hosts_here`):
    /// a phone build answers no,
    /// every other build yes. This door exists so a test on a computer
    /// can stand up the node a phone runs — the build fact cannot be
    /// flipped in a test binary, and what has to be proven (no tmux
    /// asked, no session started, the row says so) is about the node,
    /// not about the build.
    pub fn open_hosting(root: PathBuf, name: &str, hosts: bool) -> Result<Node, String> {
        let permit = doc_transfer_state::RootPermit::acquire(&root).map_err(String::from)?;
        Self::open_admitted(root, name, hosts, permit)
    }

    fn open_admitted(
        root: PathBuf,
        name: &str,
        hosts: bool,
        permit: doc_transfer_state::RootPermit,
    ) -> Result<Node, String> {
        // **Before anything reads a path** (第十四轮 第 4 条): if this
        // process is standing in a directory that has been deleted, it
        // steps onto `/` first — `gix` goes through the current
        // directory even for an absolute path, and khor refused a plain
        // checkout because of it. Only when adrift: opening a node is
        // not a reason to move a process that is standing somewhere
        // real (`cwd`'s head).
        cwd::pin_if_adrift();
        claim_store(&root)?;
        let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
            .map_err(|e| e.to_string())?;
        let public = key.public();
        let device = DeviceId(*public.as_bytes());
        let device_str = public.to_string();
        let name = channel_of_machine(name)
            .ok_or_else(|| msg::name_not_pathable(format_args!("{name:?}")))?;
        let name = settled_channel(&root, name)?;
        let chat = ChatKind::new(
            root.clone(),
            Sender {
                id: device_str.clone(),
                name: name.clone(),
            },
        );
        let transfer = TransferKind::new(root.clone());
        // The one place khor decides to read other vendors' files. Every
        // other construction of a LiveKind discovers nothing, so no test
        // answers with whatever the machine happens to be running.
        //
        // **A machine that cannot host discovers nothing** (docs/IOS.md
        // 第二步): no vendor directories, no tmux, not even the process
        // table — a phone has no agents of its own and may not start a
        // process, so every sweep there was a `posix_spawn` refused in
        // silence, every ten seconds (`day_sampler`). Closed rather than
        // skipped, so every reader of `live` keeps working and simply
        // finds nothing here.
        let discovery = if hosts {
            adaptor::Discovery::for_root(&root)
        } else {
            adaptor::Discovery::empty().with_procs(adaptor::Procs::default())
        };
        let live = LiveKind::new(root.clone(), device).discovering(Arc::new(discovery));
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
        //
        // The ledger is this khor home's, not the vendor home's: it is
        // khor's own record of what it counted, and it outlives the
        // transcripts it was counted from (`usage::ledger`).
        let usage = usage::meters_for(
            &adaptor::vendor_home(&root),
            &root.join(".khor").join(usage::ledger::FILE),
        );
        // Read before `root` moves into the struct, not after — the
        // list is settled once, when the node opens, and every caller
        // that wants a changed list gets it by opening another node.
        let (relay_map, relays_only_mine) = relays_under(&root);
        let relays = relay_map.iter().map(|r| r.url.clone()).collect();
        let carry = khor_net::carry::Links::new(key.public());
        let mut node = Node {
            root,
            key,
            device,
            device_str,
            name,
            peer: ChatDoc::fresh_peer(),
            relays,
            relay_map,
            relays_only_mine,
            chat,
            transfer,
            live,
            usage,
            subscribers: Arc::new(Mutex::new(Vec::new())),
            store_gate: tokio::sync::RwLock::new(()),
            rounds: Arc::default(),
            readings_held: Arc::default(),
            days_held: Arc::default(),
            watches: Arc::default(),
            resting: Arc::default(),
            face_seen: Arc::default(),
            nudged: Arc::default(),
            visited: Arc::default(),
            vouched: Arc::default(),
            reached_at: Arc::default(),
            ears_homed: Arc::default(),
            egress: Arc::default(),
            deaf_said: Arc::default(),
            roadbook: Arc::default(),
            rows_sent: Arc::default(),
            wsfacts: Arc::default(),
            borrows: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            reaches: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            codes: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            wires: Arc::default(),
            me: std::sync::OnceLock::new(),
            agent_generation: std::sync::OnceLock::new(),
            ears: Mutex::new(None),
            app_resident: std::sync::atomic::AtomicBool::new(false),
            root_born: false,
            roads: Arc::default(),
            carry,
            hosts,
        };
        {
            let recovered = permit.attach().map_err(String::from)?;
            node.recover_pairing_before_bootstrap(&recovered)?;
            node.register_self()?;
            // After the row exists: the first machine's root certifies the
            // machine the table now holds (docs/CONNECT.md 身份).
            node.root_born = node.bootstrap_trust(&recovered)?;
        }
        Ok(node)
    }

    /// Whether opening this node made the network's root key just now.
    /// The CLI says 「write the words down」 when it did; the app asks
    /// `root_state().unshown` instead, which stays true until they are
    /// shown.
    pub fn root_born(&self) -> bool {
        self.root_born
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
        // **Whether this machine can host, said by the machine itself**
        // (docs/IOS.md 第二步) — on every open, so the row always says
        // what the build running it is. Every other machine reads it off
        // the table to decide whether to offer a session here.
        loaded.doc.set_hosts(&self.device_str, self.hosts)?;
        // **No style is written here any more** (2026-09-21). It used to
        // push `.khor/avatar.json` into the table on every open, back
        // when the file was the choice and the table was a report of it.
        // The table *is* the choice now (`khor_sync::devices`'s head:
        // one field, anyone may write it, the later write wins) — and a
        // push on every open would be this machine quietly taking its
        // own face back from whoever last restyled it, once per start.
        // A machine nobody has restyled simply has no `style` register,
        // which `Node::avatar_style` reads as the factory default.
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        // **Where this machine's received files land, said on its row**
        // (STORE-BRIEF phase 1): a setting changed while this machine was
        // away is taken on in full now; otherwise read without making
        // anything. Then the files conversations landed before there was
        // a receive directory, moved into it once (`road::move_chat_files`).
        self.follow_receive_dir()?;
        let (dir, _) = self.receive_place(false)?;
        road::move_chat_files(&self.root, &dir);
        Ok(())
    }

    /// This machine's avatar style, or the factory default.
    ///
    /// > **It read `.khor/avatar.json` until 2026-09-21.** That file was
    /// > the choice and the device table was a copy of it, which is
    /// > exactly the shape the ruling refuses now that any machine may
    /// > restyle any other: 「不要做成「自报一份 + 覆盖一份」两个字段」.
    /// > **The table is the only source.** Nothing migrates, because
    /// > nothing has to: `register_self` has been pushing this
    /// > machine's file into the table on every open since the table
    /// > existed, so the row already holds whatever the file said. The
    /// > file is dead and is no longer read, written or removed —
    /// > deleting it would be this version reaching into a store an
    /// > older khor on the same home still writes.
    ///
    /// A row that cannot be parsed falls back **whole**, never slot by
    /// slot — the reason is on `Palette`'s `Deserialize` and it is the
    /// same one here: half a chosen style and half a default is a face
    /// nobody picked.
    pub fn avatar_style(&self) -> AvatarStyle {
        self.avatar_style_of(&self.device_str)
    }

    /// Presentation-only fallback for callers that cannot show a read
    /// error. Editors and mutations must use `checked_avatar_style_of`:
    /// a failed read is never permission to write factory defaults.
    pub fn avatar_style_of(&self, device: &str) -> AvatarStyle {
        self.devices_loaded()
            .ok()
            .and_then(|loaded| loaded.doc.get(device))
            .and_then(|row| row.style)
            .and_then(|text| AvatarStyle::from_json(&text))
            .unwrap_or_default()
    }

    /// Read the sole persisted style. `None` means the device exists
    /// and has never chosen a style; errors mean its choice is unknown,
    /// including a partial device document with unreadable blocks.
    pub fn checked_avatar_style_of(&self, device: &str) -> Result<Option<AvatarStyle>, String> {
        let loaded = self.devices_loaded()?;
        Self::avatar_style_in(&loaded, device)
    }

    fn avatar_style_in(
        loaded: &Loaded<DeviceDoc>,
        device: &str,
    ) -> Result<Option<AvatarStyle>, String> {
        if !loaded.broken.is_empty() {
            return Err(format!(
                "Cannot read avatar style: {} unreadable device block(s)",
                loaded.broken.len()
            ));
        }
        let row = loaded
            .doc
            .get(device)
            .ok_or_else(|| msg::no_such_machine(device, String::new()))?;
        row.style
            .as_deref()
            .map(|text| {
                serde_json::from_str::<AvatarStyle>(text)
                    .map_err(|e| format!("Cannot read avatar style for {device}: {e}"))
            })
            .transpose()
    }

    /// Sets a complete style in its sole register. The audit stamp is
    /// part of the same JSON value, understood by new readers and
    /// ignored by older AvatarStyle readers. An unchanged choice does
    /// not mint a write or replace the last real change's attribution.
    pub fn set_avatar_style_of(&self, device: &str, style: &AvatarStyle) -> Result<(), String> {
        let loaded = self.devices_loaded()?;
        let previous = Self::avatar_style_in(&loaded, device)?;
        self.write_avatar_style(
            loaded,
            device,
            previous.as_ref(),
            style,
            AvatarChangeOrigin::Api,
        )
    }

    fn write_avatar_style(
        &self,
        loaded: Loaded<DeviceDoc>,
        device: &str,
        previous: Option<&AvatarStyle>,
        style: &AvatarStyle,
        origin: AvatarChangeOrigin,
    ) -> Result<(), String> {
        if previous == Some(style) {
            return Ok(());
        }
        let mut value = serde_json::to_value(style).map_err(|e| e.to_string())?;
        value["change"] = serde_json::json!({
            "writer": self.device_str(),
            "at_ms": now_ms(),
            "origin": origin,
        });
        loaded.doc.set_style(device, &value.to_string())?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// This machine's, for the callers that mean this one.
    pub fn set_avatar_style(&self, style: &AvatarStyle) -> Result<(), String> {
        self.set_avatar_style_of(&self.device_str.clone(), style)
    }

    /// This machine's own relays, as the user typed them.
    ///
    /// The stored list only — not the Khor tier and not `KHOR_RELAY`.
    /// A settings screen has to be able to show what it will edit, and
    /// the merged list ([`Node::relays`]) has entries in it that no
    /// control on that screen can remove. Urls only; which of them
    /// carry a key is [`Node::relays_keyed`].
    pub fn relay_choices(&self) -> Vec<String> {
        chosen_relays(&self.root)
            .mine
            .into_iter()
            .map(|r| r.url)
            .collect()
    }

    /// Which of [`Node::relay_choices`] were entered with a key.
    ///
    /// **Whether, never what.** A screen needs to say that a relay is
    /// keyed, and it has no use for the key: showing it would put a
    /// secret on glass for a question the word 「带密钥」 answers.
    pub fn relays_keyed(&self) -> Vec<String> {
        chosen_relays(&self.root)
            .mine
            .into_iter()
            .filter(|r| r.token.is_some())
            .map(|r| r.url)
            .collect()
    }

    /// Whether this machine is set to use only its own relays — not
    /// the Khor tier.
    pub fn relays_only_mine(&self) -> bool {
        self.relays_only_mine
    }

    /// Whether binds add n0's public relays behind this machine's own
    /// as the fallback tier (`endpoint::bind_with`): every machine but
    /// one set to use only its own relays.
    pub fn relays_n0(&self) -> bool {
        !self.relays_only_mine
    }

    /// Sets this machine's own relays, replacing whatever was stored.
    /// Returns the merged list that the next node on this home will
    /// bind with.
    ///
    /// **A key rides only with the entry it was typed for.** An entry
    /// whose `token` is `None` keeps the key already stored for that
    /// url, if there is one — the screen never holds keys, so it names
    /// the relays to keep and passes a key only for the one being
    /// added. A url left out of `entries` takes its key with it; to
    /// drop a key, remove the relay and add it again without one.
    ///
    /// **Validated before it is written, and rejected whole.** Each
    /// entry has to parse as an [`iroh::RelayUrl`]; one that does not
    /// fails the call and names itself. Writing a bad entry and letting
    /// the reader skip it would put the machine in the state this whole
    /// setting exists to end — a relay the user believes is configured
    /// and nothing anywhere saying it is not.
    ///
    /// ⚠️ **It does not move a running endpoint.** The relays a node
    /// binds with are read once, when the node opens. The GUI opens a
    /// node per call so it sees this immediately; a live `khor serve`
    /// and every connection already standing keep the list they started
    /// with until they are restarted. That is a sentence the screen has
    /// to say out loud — a setting that visibly took and changes nothing
    /// until a restart nobody mentioned is the same silence as no
    /// setting at all.
    pub fn set_relays(
        &self,
        entries: &[khor_net::endpoint::Relay],
        only_mine: bool,
    ) -> Result<Vec<String>, String> {
        let stored = chosen_relays(&self.root).mine;
        let mut kept: Vec<khor_net::endpoint::Relay> = Vec::new();
        for entry in entries {
            let url = entry.url.trim();
            if url.is_empty() {
                continue;
            }
            url.parse::<iroh::RelayUrl>()
                .map_err(|_| msg::not_a_relay_url(url))?;
            if kept.iter().any(|have| have.url == url) {
                continue;
            }
            let was = stored.iter().find(|s| s.url == url);
            let token = match entry.token.as_deref().map(str::trim) {
                Some("") => None,
                Some(t) => Some(t.to_string()),
                None => was.and_then(|s| s.token.clone()),
            };
            // What the server serves besides relaying stays with its url
            // the way the key does: the screen names relays by url only.
            kept.push(khor_net::endpoint::Relay {
                url: url.to_string(),
                token,
                qad: entry.qad.or(was.and_then(|s| s.qad)),
                pkarr: entry.pkarr.clone().or(was.and_then(|s| s.pkarr.clone())),
            });
        }
        // 「只用我自己那台」with no 自己那台 is the one combination that
        // reads like a choice and is not one: it leaves the machine with
        // no relay at all, reachable only where a hole punch lands.
        // Refused here rather than quietly downgraded, because a switch
        // that turns itself back off is a screen lying about its state.
        if only_mine && kept.is_empty() {
            return Err(msg::relays_only_mine_needs_one().to_string());
        }
        let rows: Vec<serde_json::Value> = kept
            .iter()
            .map(|r| {
                let mut row = serde_json::json!({ "url": r.url });
                if let Some(t) = &r.token {
                    row["token"] = t.as_str().into();
                }
                if let Some(p) = r.qad {
                    row["qad"] = p.into();
                }
                if let Some(u) = &r.pkarr {
                    row["pkarr"] = u.as_str().into();
                }
                row
            })
            .collect();
        // The preference survives an edit of the list while its relay is
        // still on it (`Node::set_preferred_relay`).
        let preferred = chosen_relays(&self.root).preferred.filter(|p| {
            kept.iter().any(|k| k.url == *p)
                || khor_net::endpoint::configured_relays()
                    .iter()
                    .any(|k| k.url == *p)
        });
        let doc =
            serde_json::json!({ "relays": rows, "only_mine": only_mine, "preferred": preferred });
        // 0600, beside the identity key: the keys in it are shared
        // secrets, and the file is the only place they are kept.
        link::write_private(&relay_prefs_path(&self.root), doc.to_string().as_bytes())?;
        Ok(relays_under(&self.root)
            .0
            .into_iter()
            .map(|r| r.url)
            .collect())
    }

    /// **首选中继**: the relay this machine homes on whenever it is
    /// reachable, or `None` for iroh's own choice (the nearest). Must be
    /// one of the relays this machine binds with. Takes effect on the next
    /// bind — the resident's next fresh ears, or its restart.
    pub fn set_preferred_relay(&self, url: Option<&str>) -> Result<(), String> {
        let url = url.map(str::trim).filter(|u| !u.is_empty());
        if let Some(u) = url
            && !self.relays().iter().any(|r| r == u)
        {
            return Err(msg::preferred_not_a_relay(u));
        }
        let mut doc: serde_json::Value = fs::read_to_string(relay_prefs_path(&self.root))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        doc["preferred"] = url.map_or(serde_json::Value::Null, |u| serde_json::json!(u));
        link::write_private(&relay_prefs_path(&self.root), doc.to_string().as_bytes())
    }

    /// The relay this machine prefers to home on (首选中继), if any.
    pub fn preferred_relay(&self) -> Option<String> {
        chosen_relays(&self.root).preferred
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
        self.restyle_machine(None, colors, variant, shape)
    }

    /// The same act, on a machine named the way people name machines.
    ///
    /// **`None` is this one**, and any other name goes through
    /// [`Node::device_by_name`] — the same resolution a rename and a
    /// forget use, so 「which machine did you mean」 has one answer in
    /// this tree rather than one per verb. An unknown name is refused by
    /// name; it is not quietly read as this machine, because the face
    /// that came back would be the wrong machine's and nothing on screen
    /// would say so.
    pub fn restyle_machine(
        &self,
        machine: Option<&str>,
        colors: Option<&[String]>,
        variant: Option<&str>,
        shape: Option<&str>,
    ) -> Result<AvatarStyle, String> {
        self.restyle_machine_from(machine, colors, variant, shape, AvatarChangeOrigin::Api)
    }

    pub fn restyle_machine_from(
        &self,
        machine: Option<&str>,
        colors: Option<&[String]>,
        variant: Option<&str>,
        shape: Option<&str>,
        origin: AvatarChangeOrigin,
    ) -> Result<AvatarStyle, String> {
        let device = match machine {
            None => self.device_str.clone(),
            Some(name) => self.device_by_name(name)?,
        };
        self.restyle_device_from(&device, colors, variant, shape, origin)
    }

    /// The act itself, on a device **id** — what the faces call once
    /// they already have one (the GUI's rows carry ids, not names).
    ///
    /// Read-modify-write over the *whole* style, because whole is what
    /// the device table carries (`khor_sync::devices`: one JSON value,
    /// whole-value LWW). There is deliberately nothing finer to write:
    /// half a palette from one writer and a variant from another is a
    /// face nobody chose.
    pub fn restyle_device(
        &self,
        device: &str,
        colors: Option<&[String]>,
        variant: Option<&str>,
        shape: Option<&str>,
    ) -> Result<AvatarStyle, String> {
        self.restyle_device_from(device, colors, variant, shape, AvatarChangeOrigin::Api)
    }

    pub fn restyle_device_from(
        &self,
        device: &str,
        colors: Option<&[String]>,
        variant: Option<&str>,
        shape: Option<&str>,
        origin: AvatarChangeOrigin,
    ) -> Result<AvatarStyle, String> {
        // Keep the same loaded document through read, validation and
        // write. Never reload with a default in between the two halves.
        let loaded = self.devices_loaded()?;
        let previous = Self::avatar_style_in(&loaded, device)?;
        let mut style = previous.clone().unwrap_or_default();
        if colors.is_none() && variant.is_none() && shape.is_none() {
            return Ok(style);
        }
        if let Some(colors) = colors {
            style.palette = Palette::parse(colors).ok_or_else(|| {
                msg::not_a_palette(colors.join(khor_catalog::cli::name_separator()))
            })?;
        }
        if let Some(key) = variant {
            style.variant = Variant::from_key(key).ok_or_else(|| {
                let all: Vec<&str> = Variant::ALL.iter().map(|v| v.key()).collect();
                msg::not_a_variant(key, all.join(khor_catalog::cli::name_separator()))
            })?;
        }
        if let Some(key) = shape {
            style.shape = FaceShape::from_key(key).ok_or_else(|| {
                let all: Vec<&str> = FaceShape::ALL.iter().map(|s| s.key()).collect();
                msg::not_a_face_shape(key, all.join(khor_catalog::cli::name_separator()))
            })?;
        }
        self.write_avatar_style(loaded, device, previous.as_ref(), &style, origin)?;
        Ok(style)
    }

    /// A machine's device id, by whatever it is called on screen.
    ///
    /// Refused by name for a machine khor does not have — the shape
    /// `khor forget` and `khor watch` already answer in, and the reason
    /// is theirs: a name that matched nothing and fell through to this
    /// machine would be an act performed on the wrong subject with
    /// nothing anywhere saying so.
    pub fn device_by_name(&self, name: &str) -> Result<String, String> {
        let loaded = self.devices_loaded()?;
        if let Some(d) = loaded.doc.by_name(name) {
            return Ok(d.id);
        }
        // Listed by the name on the glass, `Node::resolve`'s rule: a
        // list of channels would name machines the person cannot see
        // anywhere.
        let all = loaded.doc.all();
        let names: Vec<&str> = all.iter().map(|d| d.shown()).collect();
        Err(msg::no_such_machine(
            name,
            names.join(khor_catalog::cli::name_separator()),
        ))
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

    /// The same preview for a machine that is not this one.
    ///
    /// **The seed has to be that machine's**, which is the whole reason
    /// this exists beside `face_under`: the seed is the device id, so a
    /// chooser painting its swatches with *this* machine's seed would
    /// show somebody a set of faces none of which is the face they are
    /// about to give that machine. An id that is not sixty-four hex
    /// characters has no seed and no face — `None`, never a default
    /// drawn from nothing.
    pub fn face_under_for(&self, device: &str, style: &AvatarStyle) -> Option<Avatar> {
        Some(avatar(&AvatarSeed::from_id_hex(device)?, style))
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

    /// `KHOR_HOME` if set, else this machine's store under the home
    /// directory. The override exists for tests and dual-instance
    /// verification on one machine.
    ///
    /// **The home directory is not always the store.** A cluster mounts
    /// one home on every machine, and a store is one machine's
    /// (`claim_store`), so on a shared home only the machine named in
    /// `~/.khor/owner` may use `$HOME`; every other one gets
    /// `$HOME/.khor-hosts/<hostname>`.
    ///
    /// This used to be a line the installer appended to `~/.profile`,
    /// and that line was wrong for the owner: it read
    /// `KHOR_HOME="$HOME/.khor-hosts/$(hostname)"` unconditionally,
    /// while `.profile` on a shared home is **one file for the whole
    /// fleet**. Written by hinton's install, read by turing, which then
    /// pointed its own terminals at a store that was not turing's.
    /// Measured 2026-09-11 on that pair: `khor devices` in a turing
    /// shell listed one machine (itself, under a second identity), and
    /// the same command with `KHOR_HOME=$HOME` listed four.
    ///
    /// **The owner stamp cannot catch that one.** It refuses a machine
    /// that opens somebody else's store; here nobody did — a fresh empty
    /// directory was created and claimed, correctly, by the machine that
    /// should never have been sent there.
    ///
    /// So the rule lives here rather than in a file anybody's login may
    /// rewrite. `khor_root` in `scripts/onset-lib.sh` is the same rule
    /// in sh, for the steps that need the path before khor can run.
    pub fn root_from_env() -> PathBuf {
        if let Some(explicit) = std::env::var_os("KHOR_HOME") {
            return PathBuf::from(explicit);
        }
        Self::root_from_env_default()
    }

    /// The store a process of this user started **without** `KHOR_HOME`
    /// works out — which is also what any other such process of this
    /// user on this machine is serving (`serves::residents`).
    pub fn root_from_env_default() -> PathBuf {
        let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
        #[cfg(target_os = "ios")]
        return Self::sandbox_root(&home);
        #[cfg(not(target_os = "ios"))]
        Self::store_under(&home)
    }

    /// Prove this machine owns an existing store without opening or changing it.
    /// Store selection alone can return an unclaimed or foreign fallback path.
    pub fn verify_store_owner(root: &std::path::Path) -> Result<(), String> {
        let here = gethostname::gethostname().to_string_lossy().to_string();
        store_owner::verify(root, &here, hardware::fingerprint().as_deref()).map_err(|error| {
            match error {
                store_owner::VerificationError::Io(error) => error.to_string(),
                store_owner::VerificationError::Invalid(reason) => reason.to_owned(),
                store_owner::VerificationError::Foreign(theirs) => {
                    msg::store_belongs_to(&theirs, &here)
                }
            }
        })
    }

    /// Where the store lives inside an iOS app's sandbox. `$HOME` there is
    /// the app's own data container, and the system lets an app write in
    /// `Documents`, `Library` and `tmp` — not at the container's top, where
    /// `~/.khor` would have gone. `Library/Application Support` is the
    /// place Apple names for data the app makes and the person never opens
    /// as files (`Documents` would show the private key in the Files app
    /// once file sharing is on). It does not exist on a fresh install;
    /// `claim_store` creates the path on the way in.
    ///
    /// **No side rooms**: a container is one app on one phone, never a
    /// home mounted on several machines, so [`Node::store_under`]'s search
    /// has nothing to find.
    ///
    /// ⚠️ The system backs this directory up, identity key included, so a
    /// restore onto a new phone arrives with the old phone's key — the
    /// copied-store case. Excluding `.khor` from backup is iOS step 2
    /// (`docs/IOS.md`).
    pub fn sandbox_root(home: &std::path::Path) -> PathBuf {
        home.join("Library").join("Application Support")
    }

    /// Which store under `home` belongs to this machine. Split out so a
    /// test can ask about a directory it built, rather than about the
    /// machine it happens to run on.
    ///
    /// **The side rooms are searched, not computed.** `.khor-hosts/<name>`
    /// is where a machine's own store lands on a shared home, and the
    /// name in that path is the one it went by the day it was made — so
    /// after a rename, computing the path again lands on an empty
    /// directory and mints a second identity, which is the exact failure
    /// this batch is about. The stamp inside each room says whose it is
    /// ([`Owner`]), so the rooms are read and the matching one used
    /// **whatever it is called**.
    ///
    /// A stale directory name is the price, and it is the right one: the
    /// name on the door is a note, and moving a directory a resident has
    /// files open in is not something to do behind anybody's back.
    pub fn store_under(home: &std::path::Path) -> PathBuf {
        let here = gethostname::gethostname().to_string_lossy().to_string();
        let fp = hardware::fingerprint();
        let front = Owner::parse(
            &std::fs::read_to_string(home.join(".khor").join("owner")).unwrap_or_default(),
        );
        if front.is_mine(&here, fp.as_deref()) {
            return home.to_path_buf();
        }
        let hosts = home.join(".khor-hosts");
        // Only a fingerprint can recognise a room across a rename; with
        // none, this walk would just re-find the room named after the
        // current hostname, which is where the fallback lands anyway.
        if fp.is_some()
            && let Ok(rd) = std::fs::read_dir(&hosts)
        {
            let mut rooms: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            // Sorted so two rooms claiming one machine (a copied
            // store) resolve the same way on every run rather than
            // by readdir order.
            rooms.sort();
            for room in rooms {
                let stamp = room.join(".khor").join("owner");
                let Ok(text) = std::fs::read_to_string(&stamp) else {
                    continue;
                };
                let stamped = Owner::parse(&text);
                // **A blank stamp says nothing, and `is_mine` reads
                // "nothing" as "yours"** — right at the front door,
                // where it finishes a half-made store, and wrong
                // here, where it would hand this machine the first
                // unstamped room it walked past.
                if stamped == Owner::default() {
                    continue;
                }
                if stamped.is_mine(&here, fp.as_deref()) {
                    return room;
                }
            }
        }
        hosts.join(here)
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

    /// Whether this machine can host a session — `false` on a phone
    /// (`khor_core::hosts_here`, docs/IOS.md).
    pub fn hosts(&self) -> bool {
        self.hosts
    }

    /// **The one refusal every door that starts a process here gives on
    /// a machine that cannot host**, before anything is registered: a
    /// row put on the list for a host that then fails to start is a row
    /// that lies (docs/IOS.md: 「终端形式先登记行再起进程」). In the
    /// catalog's words, never the OS's (`posix_spawn` refused in a
    /// sandbox is not a sentence a person can act on).
    pub(crate) fn refuse_unless_hosting(&self) -> Result<(), String> {
        if self.hosts {
            Ok(())
        } else {
            Err(msg::machine_cannot_host(&self.name))
        }
    }

    /// This machine's name **as a person reads it** — the rename when
    /// there is one, else the channel (`DeviceInfo::shown`).
    ///
    /// Sentences that ask somebody to type this machine's name use it,
    /// and that is the whole reason it exists: `Node::name` answers the
    /// channel, and after a rename the channel is a word nobody can see
    /// on any screen. Both still resolve (`Node::resolve` tries the
    /// label first), so this chooses which one to *print*, never which
    /// one to accept.
    ///
    /// Falls back to the channel when the table cannot be read or holds
    /// no row for this machine — true before the first pairing, and a
    /// machine's own name is not a thing khor should fail to say.
    pub fn shown_name(&self) -> String {
        self.devices_loaded()
            .ok()
            .and_then(|l| l.doc.get(self.device_str()))
            .map(|d| d.shown().to_owned())
            .unwrap_or_else(|| self.name.clone())
    }

    /// Whether this machine is paused (docs/MORE.md 暂停) — read from
    /// the file every time, never cached, so a resident serve and a
    /// `khor pause` in another terminal cannot disagree about it
    /// (`crate::pause` has the whole of that reasoning).
    pub fn paused(&self) -> bool {
        crate::pause::is_on(&self.root)
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

    /// Every relay this node binds with and advertises: the Khor tier
    /// (unless only-mine is on), `KHOR_RELAY`, and whatever the user
    /// typed here. Urls only — see [`Node::relay_map`] for the keys.
    ///
    /// **Not the same list as [`Node::relay_choices`], and a screen
    /// showing relays needs both.** This one is what the machine
    /// actually uses; that one is the part a control can remove. Offer
    /// only this one and the user is looking at rows that ignore the
    /// delete key; offer only that one and the screen is silent about
    /// the relay the connection is really standing on.
    ///
    /// Settled when the node opened — see [`Node::set_relays`] for what
    /// that means for a change made while something is running.
    pub fn relays(&self) -> &[String] {
        &self.relays
    }

    /// [`Node::relays`] with the keys they were entered with — what an
    /// endpoint of this node binds with, and nothing else should read.
    pub fn relay_map(&self) -> &[khor_net::endpoint::Relay] {
        &self.relay_map
    }

    pub(crate) fn devices_loaded(&self) -> Result<Loaded<DeviceDoc>, String> {
        load_pile(&devices_dir(&self.root), self.peer)
    }

    pub(crate) fn seen_loaded(&self) -> Result<Loaded<SeenDoc>, String> {
        load_pile(&seen_dir(&self.root), self.peer)
    }

    pub(crate) fn pins_loaded(&self) -> Result<Loaded<PinDoc>, String> {
        load_pile(&pins_dir(&self.root), self.peer)
    }

    /// What is on the session list (`khor_sync::roster`, docs/SHELL.md
    /// 第八轮).
    pub(crate) fn roster_loaded(&self) -> Result<Loaded<RosterDoc>, String> {
        load_pile(&roster_dir(&self.root), self.peer)
    }

    pub(crate) fn dirpins_loaded(&self) -> Result<Loaded<DirPinDoc>, String> {
        load_pile(&dirpins_dir(&self.root), self.peer)
    }

    pub(crate) fn workspaces_loaded(&self) -> Result<Loaded<WorkspaceDoc>, String> {
        load_pile(&workspaces_dir(&self.root), self.peer)
    }

    pub(crate) fn webpins_loaded(&self) -> Result<Loaded<WebPinDoc>, String> {
        load_pile(&webpins_dir(&self.root), self.peer)
    }

    pub(crate) fn recents_loaded(&self) -> Result<Loaded<RecentDoc>, String> {
        load_pile(&recents_dir(&self.root), self.peer)
    }

    pub(crate) fn agents_loaded(&self) -> Result<Loaded<AgentDoc>, String> {
        load_pile(&agents_dir(&self.root), self.peer)
    }

    /// The registered API accounts (`khor_sync::accounts`) — the
    /// descriptions, never the keys; those stay in `keys_dir` on the
    /// machine that registered them.
    pub(crate) fn accounts_loaded(&self) -> Result<Loaded<AccountDoc>, String> {
        load_pile(&accounts_dir(&self.root), self.peer)
    }

    pub(crate) fn titles_loaded(&self) -> Result<Loaded<TitleDoc>, String> {
        load_pile(&titles_dir(&self.root), self.peer)
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
            return Err(msg::no_agents_registered().into());
        }
        let names: Vec<&str> = known.iter().map(|(n, _)| n.as_str()).collect();
        Err(msg::no_such_agent(
            name,
            names.join(khor_catalog::cli::name_separator()),
        ))
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
            return Err(msg::agent_name_blank().into());
        }
        if Self::RESERVED_AGENT_NAMES.contains(&name) {
            return Err(msg::agent_name_reserved(name));
        }
        let spec = AgentSpec::from_argv(argv).ok_or(msg::agent_needs_a_command())?;
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
        // Asking starts the agent, and a phone starts nothing
        // (docs/IOS.md); the face does not offer it there.
        if !self.hosts {
            return Err(msg::agent_ask_needs_a_computer().to_owned());
        }
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
        host: &satellite::Dest,
        name: Option<&str>,
    ) -> Result<(String, String, bool), String> {
        let probe = satellite::probe(host)?;
        let id = satellite::fingerprint_id(host)?;
        let name = name.unwrap_or(&host.host);
        let name = channel_of_machine(name)
            .ok_or_else(|| msg::name_not_pathable(format_args!("{name:?}")))?;
        self.enter_satellite(&id, &name, host, probe.tmux)?;
        Ok((id, name, probe.tmux))
    }

    /// The table half of [`Node::adopt_satellite`], after the ssh probe
    /// has answered: the row, the satellite mark, the tombstone cleared —
    /// and the roster (docs/CONNECT.md 卫星也进名单): signed here when
    /// this machine holds the root, waiting in `khor pending` otherwise.
    pub(crate) fn enter_satellite(
        &self,
        id: &str,
        name: &str,
        host: &satellite::Dest,
        tmux: bool,
    ) -> Result<(), String> {
        let _trust = self.root_mutation_guard()?;
        let loaded = self.devices_loaded()?;
        loaded.doc.upsert(id, name, &[])?;
        loaded
            .doc
            .set_satellite(id, self.device_str(), &host.as_row(), tmux)?;
        loaded.doc.set_gone(id, false)?;
        self.enroll_satellite(&loaded.doc, id, name)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// Whether `machine` is a satellite, and if so the ssh road **from
    /// here**: `Ok(Some(host))` when this machine sponsors it,
    /// `Ok(None)` for an ordinary machine, and an error naming the
    /// sponsor when it is somebody else's satellite — a file push runs
    /// the sponsor's scp, and only the sponsor has the road
    /// (一期单 sponsor; docs/NET.md 卫星).
    pub fn satellite_host_here(&self, machine: &str) -> Result<Option<satellite::Dest>, String> {
        // The broadcast is no machine, satellite or not.
        if machine.starts_with(khor_sync::chat::WIDE) {
            return Ok(None);
        }
        let (channel, _) = self.resolve(machine)?;
        let Some(d) = self.devices_loaded()?.doc.by_name(&channel) else {
            return Ok(None);
        };
        let Some(sp) = d.sponsor.clone() else {
            return Ok(None);
        };
        if sp != self.device_str() {
            let sponsor_name = self
                .devices_loaded()?
                .doc
                .get(&sp)
                .map(|s| s.name)
                .unwrap_or(sp);
            return Err(msg::sat_send_elsewhere(&channel, sponsor_name));
        }
        crate::satellite::Dest::of(&d)
            .map(Some)
            .ok_or_else(|| msg::sat_no_ssh_host(&channel))
    }

    /// Pushes one file into a satellite's home directory over the
    /// sponsor's own scp — the satellite half of `khor send`. No chat
    /// message and no transfer: the far side has no khor to approve or
    /// receive anything, so the bytes just go, and the verb reports
    /// what moved (一期: sponsor 本机才有这条路).
    pub fn send_to_satellite(&self, host: &satellite::Dest, path: &Path) -> Result<u64, String> {
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
            if let Ok(told) = std::env::var(PATH_ENV)
                && !told.trim().is_empty()
            {
                return told;
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
            if LOCALE_VARS
                .iter()
                .any(|v| says_utf8(&std::env::var(v).unwrap_or_default()))
            {
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
    ///
    /// **Either name works, and the rename is tried first.** What
    /// somebody types is what they read on a screen, and after a rename
    /// that is the label; the channel still resolves because it is what
    /// scripts and older habits use. The order settles the one real
    /// collision — renaming A to 「B」 while a machine is channelled `B`
    /// — in favour of the name that is actually visible.
    ///
    /// **The label is matched raw, before cleaning.** `channel_of_machine`
    /// exists to make a name safe as a path segment, and a label never
    /// becomes one; running 「北京服务器」 through it yields a
    /// fingerprinted stem that matches nothing, so a renamed machine
    /// would be unreachable by the only name anybody can see.
    fn resolve(&self, machine: &str) -> Result<(String, DeviceId), String> {
        let all = self.devices()?;
        let typed = machine.trim();
        if let Some(d) = all.iter().find(|d| d.label.as_deref() == Some(typed)) {
            return Ok((d.name.clone(), device_id_from_hex(&d.id)?));
        }
        let ch = channel_of_machine(machine)
            .ok_or_else(|| msg::machine_name_not_pathable(format_args!("{machine:?}")))?;
        match all.iter().find(|d| d.name == ch) {
            Some(d) => Ok((ch, device_id_from_hex(&d.id)?)),
            None => {
                // Listed by the name on the glass: a list of channels
                // would name machines the person cannot see anywhere.
                let names: Vec<&str> = all.iter().map(|d| d.shown()).collect();
                Err(msg::no_such_machine(
                    machine,
                    names.join(khor_catalog::cli::name_separator()),
                ))
            }
        }
    }

    /// [`Self::resolve`] with the empty name meaning **this machine**,
    /// which is the spelling every `machine` argument in this file
    /// already uses (`places_on`, the GUI's `machine: Option<String>`).
    ///
    /// Answered without reading the device table on purpose: a write
    /// about this machine's own disk must not depend on this machine
    /// having a row in its own table, which is true after pairing and
    /// not before.
    fn resolve_or_here(&self, machine: &str) -> Result<(String, DeviceId), String> {
        if machine.is_empty() {
            return Ok((self.name().to_owned(), self.device()));
        }
        self.resolve(machine)
    }

    /// Refuses a session on `machine` when that machine cannot host
    /// (docs/IOS.md 第二步), **before anything is dialled**: a phone says
    /// so about itself in the device table, so the asking side knows
    /// without a round trip — and without the phone having to be awake
    /// to say no. The empty name is this machine, `resolve_or_here`'s
    /// spelling. A name the table does not hold is left to the verb's own
    /// resolve, which names the machines it does hold.
    pub(crate) fn refuse_unless_it_hosts(&self, machine: &str) -> Result<(), String> {
        let Ok((channel, home)) = self.resolve_or_here(machine) else {
            return Ok(());
        };
        if home == self.device() {
            return self.refuse_unless_hosting();
        }
        match self.devices()?.into_iter().find(|d| d.name == channel) {
            Some(d) if !d.hosts => Err(msg::machine_cannot_host(d.shown())),
            _ => Ok(()),
        }
    }

    /// A typed machine name as an id, hex, with the empty name meaning
    /// this machine — the spelling every `machine` argument uses. The
    /// workspace verbs are keyed by id ([`Node::ws_write`]) and the CLI
    /// is handed names, so this is the one place the two meet.
    ///
    /// **An id typed in place of a name is taken as itself**, and it is
    /// not a second grammar for its own sake: a workspace row whose
    /// machine has left the device table has no name at all, and that
    /// row is exactly the one somebody needs to take off the list
    /// ([`Node::workspaces`]). Its id is the only name left, which is
    /// why `khor ws ls` prints it in the machine column for such a row.
    /// The table is tried first, so a machine that is present always
    /// wins over this fallback.
    pub fn machine_id(&self, machine: &str) -> Result<String, String> {
        match self.resolve_or_here(machine) {
            Ok((_, id)) => Ok(id.hex()),
            Err(no_such) => device_id_from_hex(machine)
                .map(|id| id.hex())
                .map_err(|_| no_such),
        }
    }

    /// Pauses or resumes talking to one machine (SHELL.md 第五轮 第 3
    /// 条), by the name a person typed. **This machine's own name, or the
    /// empty name, is `khor pause` itself** — one verb for the menu item
    /// on every row. Answers the sentence to print.
    pub fn set_machine_paused(&self, machine: &str, on: bool) -> Result<String, String> {
        let (_, id) = self.resolve_or_here(machine)?;
        if id == self.device() {
            crate::pause::set_on(&self.root, on)?;
            return Ok(if on { msg::paused() } else { msg::resumed() }.to_owned());
        }
        let id = id.hex();
        let shown = self
            .devices()?
            .into_iter()
            .find(|d| d.id == id)
            .map(|d| d.shown().to_owned())
            .unwrap_or_else(|| machine.to_owned());
        crate::pause::set_peer(&self.root, &id, &shown, on)?;
        Ok(if on {
            msg::peer_paused(&shown)
        } else {
            msg::peer_resumed(&shown)
        })
    }

    /// Whether this machine is paused toward the one with this id —
    /// for the faces' rows (`khor devices`, the app's device list).
    pub fn machine_paused(&self, id: &str) -> bool {
        if id == self.device_str() {
            self.paused()
        } else {
            crate::pause::peer_on(&self.root, id)
        }
    }

    /// [`Self::resolve`] for callers outside this module — the CLI's
    /// `rename -m` needs an id, and turning a typed name into one is
    /// exactly this.
    pub fn resolve_machine(&self, machine: &str) -> Result<(String, DeviceId), String> {
        self.resolve(machine)
    }

    fn kinds(&self) -> [&dyn KindSurface; 3] {
        [&self.transfer, &self.live, &wide::WideKind]
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
        self.roster_upgrade_once();
        let roster = self.roster_loaded()?;
        Ok(self
            .every_session()?
            .into_iter()
            .filter(|v| listed(&roster.doc, &v.session))
            .map(|mut v| {
                v.imported =
                    roster.doc.entry(&v.session.id.0) == Some(khor_sync::roster::Entry::Import);
                v
            })
            .collect())
    }

    /// Every session this machine can see, **on the list or not** — the
    /// list is [`Self::sessions`], and what it leaves out are the import
    /// candidates ([`Self::import_candidates`]). Same order, same
    /// overlays.
    pub(crate) fn every_session(&self) -> Result<Vec<SessionView>, String> {
        let pins = self.pins_loaded()?;
        let titles = self.titles_loaded()?;
        let seen = self.seen_loaded()?;
        let now = crate::live::now_ms().max(0) as u64;
        let mut views: Vec<SessionView> = Vec::new();
        for k in self.kinds() {
            for row in k.rows(self)? {
                views.push(SessionView {
                    pinned: pins.doc.pinned(&row.id.0),
                    stale: false,
                    quiet: false,
                    imported: false,
                    session: row,
                    source: None,
                });
            }
        }
        let mut have: std::collections::BTreeSet<String> =
            views.iter().map(|v| v.session.id.0.clone()).collect();
        let mut reported = self.cached_peer_rows();
        reported.sort_by_key(|(_, age, _)| *age);
        for (name, age, mut row) in reported {
            if have.insert(row.id.0.clone()) {
                // Read decisions are local facts already. A peer's stale report
                // must not wait for a sync round trip to reflect them here.
                if row.state.state == State::Done
                    && seen.doc.watermark(&row.id.0) >= row.state.at.0 as i64
                {
                    row.unread = 0;
                    row.state.state = State::Idle;
                }
                views.push(SessionView {
                    pinned: pins.doc.pinned(&row.id.0),
                    stale: false,
                    quiet: false,
                    imported: false,
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
            v.session.fallback_unbound_terminal();
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
        let by_id = self
            .devices()?
            .into_iter()
            .map(|d| (d.id, d.name))
            .collect();
        self.sessions_arranged_with(mode, &by_id)
    }

    /// The same list, for a caller that already holds the id→name table.
    ///
    /// **It exists because reading the table is not free and one screen
    /// wanted it three times.** `khor_gui_core::list_sessions` builds a
    /// face per machine, a name per machine, and then asked for this —
    /// each of which called [`Node::devices`] on its own, so one refresh
    /// of a list read the device doc three times. Through here it reads
    /// it once and derives all three.
    ///
    /// ⚠️ **The table is the caller's, so its freshness is too.** Every
    /// call above reads the doc at the moment it asks; this one is as
    /// current as whatever was handed in. That is right for one screen
    /// built from one reading — a row and its machine's name then come
    /// from the same instant — and wrong for a caller that holds a table
    /// across beats, which would name a machine by what it used to be
    /// called.
    pub fn sessions_arranged_with(
        &self,
        mode: list::Arrange,
        by_id: &std::collections::BTreeMap<String, String>,
    ) -> Result<Vec<list::Arranged>, String> {
        let views = self.sessions()?;
        Ok(list::arrange(views, mode, &|home: &DeviceId| {
            by_id.get(&home.hex()).cloned()
        }))
    }

    /// **删除 on an imported row takes it off the list, and that is all**
    /// (docs/SHELL.md 第八轮 第 3 条): the process, the vendor's own files
    /// and the multiplexer stay exactly as they were, and the session is
    /// a candidate again. Answers whether it was one.
    fn take_off_if_imported(&self, id: &SessionId) -> Result<bool, String> {
        let loaded = self.roster_loaded()?;
        if loaded.doc.entry(&id.0) != Some(khor_sync::roster::Entry::Import) {
            return Ok(false);
        }
        // **接管 makes it khor's** (`takeover`, `takeover_term`): the
        // agent khor imported now runs under a host khor started, or is
        // a conversation khor holds. 删除 on that is the ordinary close —
        // there is a process of khor's to wind down — so the row only
        // leaves the list here, and the close below does the rest.
        if self.khor_holds(id) {
            loaded.doc.take_off(&id.0)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
            return Ok(false);
        }
        loaded.doc.take_off(&id.0)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        // khor's own terminal onto it goes; the session does not.
        self.live.drop_lens(id);
        self.emit(NodeEvent::Closed(id.clone()));
        Ok(true)
    }

    /// **One search over everything khor holds** (docs/SHELL.md 第十轮:
    /// 全局搜索 ⌘K, `khor find`): sessions, machines, workspaces, the
    /// browsing tabs, and the pages the app's 「…」 opens. One matching
    /// rule and one order, here, so the window's list and `khor find`
    /// are the same list — a second rule on a face is how they come to
    /// disagree about what the first hit is.
    ///
    /// **Not file contents** (this round): the roads that read a disk
    /// are the workspace's own search, and folding them in here would
    /// make one keystroke ask four machines for their files.
    ///
    /// The order: what *starts* with the word first, then what merely
    /// holds it; within that, sessions, machines, workspaces, tabs,
    /// pages — the order of how specific a thing is to what somebody is
    /// doing. Ties break on the label, so two runs answer the same.
    pub fn find(&self, query: &str) -> Result<Vec<Hit>, String> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let mut out: Vec<Hit> = Vec::new();
        let by_id: std::collections::BTreeMap<String, String> = self
            .devices()?
            .into_iter()
            .map(|d| (d.id.clone(), d.shown().to_owned()))
            .collect();
        for v in self.sessions()? {
            let s = v.session;
            out.push(Hit {
                kind: "session".to_owned(),
                id: s.id.0.clone(),
                label: if s.title.is_empty() {
                    s.id.0.clone()
                } else {
                    s.title.clone()
                },
                note: by_id.get(&s.home.hex()).cloned(),
            });
        }
        for d in self.devices()? {
            out.push(Hit {
                kind: "machine".to_owned(),
                id: d.id.clone(),
                label: d.shown().to_owned(),
                note: None,
            });
        }
        for w in self.workspaces()? {
            out.push(Hit {
                kind: "workspace".to_owned(),
                id: format!("{}:{}", w.device, w.path),
                label: w.name.clone(),
                note: Some(w.path.clone()),
            });
        }
        for t in crate::tabs::list(&self.root, "page").tabs {
            out.push(Hit {
                kind: "tab".to_owned(),
                id: t.id.clone(),
                label: if t.title.is_empty() {
                    t.url.clone()
                } else {
                    t.title.clone()
                },
                note: (!t.url.is_empty()).then(|| t.url.clone()),
            });
        }
        // 更多's own two and 设置's seven (UX-16), by the words the face
        // names them with (`views/more/MorePage` `moreName`).
        for (key, label) in [
            ("me", khor_catalog::gui::my_khor()),
            ("appearance", khor_catalog::config::appearance()),
            ("terminal", khor_catalog::gui::view_terminal()),
            ("session", khor_catalog::config::session()),
            ("workspace", khor_catalog::config::workspace()),
            ("connection", khor_catalog::config::connection()),
            ("security", khor_catalog::gui::security()),
            ("about", khor_catalog::gui::settings_about()),
            ("doctor", khor_catalog::gui::doctor()),
        ] {
            out.push(Hit {
                kind: "page".to_owned(),
                id: key.to_owned(),
                label: label.to_owned(),
                note: None,
            });
        }
        let rank = |h: &Hit| -> Option<(u8, u8, String)> {
            let label = h.label.to_lowercase();
            let note = h.note.clone().unwrap_or_default().to_lowercase();
            let id = h.id.to_lowercase();
            let how = if label.starts_with(&q) {
                0
            } else if label.contains(&q) {
                1
            } else if note.contains(&q) || id.contains(&q) {
                2
            } else {
                return None;
            };
            let kind = match h.kind.as_str() {
                "session" => 0,
                "machine" => 1,
                "workspace" => 2,
                "tab" => 3,
                _ => 4,
            };
            Some((how, kind, label))
        };
        let mut scored: Vec<((u8, u8, String), Hit)> = out
            .into_iter()
            .filter_map(|h| rank(&h).map(|r| (r, h)))
            .collect();
        scored.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(scored.into_iter().map(|(_, h)| h).collect())
    }

    /// Whether khor itself holds this session now: a conversation it is
    /// the protocol client of, or a terminal it hosts (a takeover leaves
    /// one behind). What a found session never has.
    fn khor_holds(&self, id: &SessionId) -> bool {
        let held_kind = self
            .every_session()
            .ok()
            .and_then(|all| all.into_iter().find(|v| v.session.id == *id))
            .is_some_and(|v| {
                let kind = v.session.kind.0.as_str();
                kind != khor_core::kind::SHELL && kind != khor_core::kind::TUI
            });
        held_kind
            || self
                .live
                .dir_of(id)
                .is_some_and(|dir| crate::host::read_host_file(&dir).is_ok())
    }

    /// What could be brought onto the list (docs/SHELL.md 第八轮 导入):
    /// every terminal and agent this machine can see — its own sweep and
    /// what the others reported — that is not on it and has not ended,
    /// and every conversation a registered ACP agent said it has
    /// (`session/list`, as recorded) that is not open here already.
    /// Newest activity first.
    pub fn import_candidates(&self) -> Result<Vec<Candidate>, String> {
        let roster = self.roster_loaded()?;
        let every = self.every_session()?;
        let mut out: Vec<Candidate> = every
            .iter()
            .filter(|v| !listed(&roster.doc, &v.session) && !v.session.dead)
            .map(|v| Candidate {
                id: v.session.id.0.clone(),
                form: v.session.kind.0.clone(),
                home: v.session.home.hex(),
                title: v.session.title.clone(),
                place: v.session.place.clone(),
                at_ms: v.session.state.at.0 as i64,
                category: v.session.category.clone(),
                agent: None,
                command: v.session.doing.as_ref().and_then(|d| d.command.clone()),
            })
            .collect();
        let me = self.device_str().to_owned();
        // **An ACP conversation is reopened on this machine**
        // (`resume_agent_session`), so a phone offers none (docs/IOS.md):
        // every one of them would be refused at the press.
        let agents = if self.hosts {
            self.agents()?
        } else {
            Vec::new()
        };
        for (name, _) in agents {
            let Some(khor_sync::agents::Answer::Sessions { rows }) = self.agent_seen(&name)? else {
                continue;
            };
            for r in rows {
                let as_row = adaptor::id_for(&r.id);
                if every.iter().any(|v| v.session.id == as_row) {
                    continue;
                }
                out.push(Candidate {
                    id: format!("{ACP_CANDIDATE}{name}:{}", r.id),
                    form: khor_core::kind::GUI.to_owned(),
                    home: me.clone(),
                    title: r.title.clone().unwrap_or_default(),
                    place: (!r.cwd.is_empty()).then(|| r.cwd.clone()),
                    at_ms: r
                        .updated_at
                        .as_deref()
                        .and_then(|t| t.parse::<jiff::Timestamp>().ok())
                        .map_or(0, |t| t.as_millisecond()),
                    category: None,
                    agent: Some(name.clone()),
                    command: None,
                });
            }
        }
        out.sort_by_key(|c| std::cmp::Reverse(c.at_ms));
        Ok(out)
    }

    /// Brings one candidate onto the list, by the id
    /// [`Self::import_candidates`] gave it. A terminal or an agent goes
    /// on as `import`; an ACP conversation is opened again with
    /// `session/load` (a conversation khor holds is on the list by
    /// construction). Answers the row's session id.
    pub fn import(&self, id: &str) -> Result<SessionId, String> {
        if let Some(rest) = id.strip_prefix(ACP_CANDIDATE)
            && let Some((agent, session)) = rest.split_once(':')
        {
            return self.resume_agent_session(agent, session);
        }
        // Any row this machine can see and the list does not have — an
        // ended one too, when somebody names it: the panel offers only
        // the living, but an id typed on purpose is a choice.
        let loaded = self.roster_loaded()?;
        let known = self
            .every_session()?
            .into_iter()
            .any(|v| v.session.id.0 == id && !listed(&loaded.doc, &v.session));
        if !known {
            return Err(msg::not_a_candidate(id));
        }
        loaded.doc.put(id, khor_sync::roster::Entry::Import)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(SessionId(id.to_owned()))
    }

    /// Opens a conversation a registered agent already has, by the id it
    /// gave (`session/load`) — the import of an ACP candidate, and what
    /// the new-session wizard's 「已有的对话」 used to do.
    ///
    /// **The id must be one khor recorded**: an id that was never listed
    /// has no address to open at, so it is refused by name rather than
    /// opened at some default — a fresh conversation called the old one
    /// is the failure being avoided. It brings its own directory and
    /// name, decided by it.
    pub fn resume_agent_session(&self, agent: &str, session: &str) -> Result<SessionId, String> {
        self.refuse_unless_hosting()?;
        let spec = self.agent_or_refuse(agent)?;
        let seen = self
            .agent_seen_row(agent, session)?
            .ok_or_else(|| msg::resume_unknown_session(session, agent))?;
        let title = seen.title.clone().unwrap_or_else(|| {
            std::path::Path::new(&seen.cwd)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| agent.to_owned())
        });
        self.reopen_gui_as(
            std::path::Path::new(&seen.cwd),
            &title,
            &[Node::agent_launch(&spec)?],
            agent,
            session,
        )
    }

    /// Puts a session khor just started on the list (`khor_sync::roster`,
    /// `new`). Every opener calls this, on the machine the session is
    /// on; the document carries it everywhere else.
    pub(crate) fn roster_new(&self, id: &SessionId) -> Result<(), String> {
        let loaded = self.roster_loaded()?;
        loaded.doc.put(&id.0, khor_sync::roster::Entry::New)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// **Once per store**: what khor opened before the list became the
    /// user's (docs/SHELL.md 第八轮 升级) stays on it — every registry
    /// entry a khor host was started for goes on as `new`. What was only
    /// ever found (a sweep, a hook, a multiplexer) does not: 「升级时自动
    /// 发现的：不保留」. A local mark says it ran; best effort, a failure
    /// costs a retry on the next read and nothing else.
    fn roster_upgrade_once(&self) {
        let mark = self.root.join(".khor").join("roster-upgraded");
        if mark.exists() {
            return;
        }
        let Ok(loaded) = self.roster_loaded() else {
            return;
        };
        for id in self.live.hosted_ids() {
            let _ = loaded.doc.put(&id.0, khor_sync::roster::Entry::New);
        }
        let mut store = loaded.store;
        if store.flush(&loaded.doc).is_ok() {
            let _ = std::fs::create_dir_all(self.root.join(".khor"));
            let _ = std::fs::write(&mark, b"");
        }
    }

    /// Rewrite this home's piled-up documents as snapshots.
    ///
    /// ⚠️ **The resident serve calls this and nothing else may**, because
    /// two compactors on one document destroy it
    /// (`khor_sync::store::compact_dir` has the mechanism). The serve is
    /// the natural owner: there is one of it, and it already holds the
    /// tick this runs on.
    ///
    /// **Why it exists at all**: `BlockStore::compact` had no caller in
    /// the product until 2026-09-11, so every document grew a block per
    /// write and never shed one, and every read replayed the whole
    /// history. This machine's device table was 557 blocks and 20 ms to
    /// read; the session list reads ten documents on a 2 s beat.
    pub fn compact_piles(&self) -> Vec<(PathBuf, khor_sync::store::Compacted)> {
        khor_sync::store::compact_piles_under(&self.root.join(".khor"), self.peer)
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
                .iter()
                .filter(|v| v.source.is_none() && v.session.state_known())
                .map(|v| (v.session.id.0.clone(), v.session.state.state)),
        );
        record.unavailable(
            now,
            views
                .into_iter()
                .filter(|v| v.source.is_none() && !v.session.state_known())
                .map(|v| v.session.id.0),
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
                    let day = theirs
                        .entry(home.clone())
                        .or_insert_with(|| self.peer_day(&home));
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
        // One sample on the way back from a stopped process (a phone in
        // the background), not every one it missed — `link::serve` says why.
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let me = self.clone();
            let _ = tokio::task::spawn_blocking(move || me.record_day()).await;
        }
    }

    /// Where this machine's CPU day is kept (`history`), beside the
    /// other things the resident records about itself.
    fn history_dir(root: &Path) -> PathBuf {
        root.join(".khor").join("history")
    }

    /// Records one CPU reading into this machine's day
    /// (`history::CpuHistory`). Load-modify-save each beat, the
    /// `record_day` arrangement: the file is never behind by more than
    /// one sample, and a resident that dies takes at most one with it.
    /// **Blocking** — it waits, at most one list-and-volume round, for
    /// the background sampler's first reading; it never takes one itself,
    /// so the disks are asked by one thread only (docs/SHELL.md 第十六轮).
    /// No reading in that time is no record for this beat.
    pub fn record_cpu(root: &Path) {
        let Some(v) = vitals::local::reading_within(root, std::time::Duration::from_secs(5))
            .map(|r| r.vitals)
        else {
            return;
        };
        let pct = |f: &khor_core::Fill| {
            (f.total > 0).then(|| (f.used as f64 / f.total as f64 * 100.0) as f32)
        };
        // **The disk of the day is the whole of the disk dial**, summed
        // over the volumes this machine reports so the line under the
        // dials and the dial itself answer with one number. A node that
        // reports no volumes falls back to the one filesystem it knows.
        let disk_pct = if v.volumes.is_empty() {
            v.disk.as_ref().and_then(pct)
        } else {
            let (used, total) = v.volumes.iter().fold((0u64, 0u64), |(u, t), x| {
                (u + x.fill.used, t + x.fill.total)
            });
            pct(&khor_core::Fill { used, total })
        };
        // **A store that is gone is not re-made by a recorder.** The
        // write creates the directory it needs, so a resident outliving
        // its store (a test's scratch home taken away while its serve's
        // tasks still run) put `.khor/history/cpu` back every minute —
        // one of the directories every test run left in `$TMPDIR`
        // (SHELL.md 第五轮 第 4 条: `khor-far-beta-*`, holding exactly
        // that one file). Asked after the sample, which takes a while,
        // and just before the write.
        if !root.join(".khor").is_dir() {
            return;
        }
        history::CpuHistory::open(&Self::history_dir(root)).record(
            now_ms(),
            history::Reading {
                cpu_pct: v.cpu_pct,
                mem_pct: pct(&v.mem).unwrap_or(0.0),
                // Absent, not zero, on a machine with no card khor can
                // ask — the whole point of the optional column.
                gpu_pct: v.gpu.as_ref().map(|g| g.util_pct),
                disk_pct,
                // **Also absent on a unified-memory machine**, where
                // the cards' bytes are already `mem_pct` and drawing
                // them again would put the same memory on the screen
                // under two names (`khor_core::Gpu::mem`). A `total` of
                // zero is 「读不出」 rather than 「空的」, the same
                // first branch `Fill::strain` takes.
                vram_pct: v
                    .gpu
                    .as_ref()
                    .and_then(|g| g.mem.as_ref())
                    .filter(|f| f.total > 0)
                    .map(|f| (f.used as f64 / f.total as f64 * 100.0) as f32),
            },
        );
    }

    /// Takes a CPU reading every [`history::RECORD_EVERY`], for as long
    /// as the resident lives — its own task on its own beat, the
    /// `day_sampler` judgment one door up. **The record only exists
    /// while a resident does**, and a minute with none is the gap the
    /// band paints as absence rather than a number nobody measured.
    pub(crate) async fn cpu_recorder(self: std::sync::Arc<Self>) {
        let mut tick = tokio::time::interval(history::RECORD_EVERY);
        // One sample on the way back from a stopped process (a phone in
        // the background), not every one it missed — `link::serve` says why.
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let root = self.root.clone();
            let _ = tokio::task::spawn_blocking(move || Self::record_cpu(&root)).await;
        }
    }

    /// What this machine is busy with: the `top` hungriest processes
    /// right now and the CPU day its resident has recorded
    /// (`khor_core::Load`). The day is read from disk rather than from
    /// memory because the reader is usually a different process from
    /// the recorder — the GUI opens its own `Node` — and the file is the
    /// one copy both see. **Blocking** — the sampler's rule; the one
    /// async caller (`link`) goes through `spawn_blocking`.
    ///
    /// **`top` of 0 means every process, not none.**
    ///
    /// A face that sorts by a column has to sort the whole table: the
    /// row holding the most memory is rarely among the eight hungriest
    /// by CPU, so "sort by memory" over a cut of eight answers a
    /// different question than it was asked — and answers it
    /// confidently, which is the worse half. The machine card is a
    /// window over the whole list for exactly that reason, and asks
    /// with 0.
    ///
    /// Zero rather than a sentinel like `u32::MAX` because zero is what
    /// a face means when it says "no limit", and because the wire
    /// carries a `u32` whose maximum is a plausible typo. A caller that
    /// really wants a cut still asks for one (`khor top` does).
    ///
    /// **And `by` is why a cut is enough.** The ranking happens here,
    /// so "the two hundred biggest by memory" is a true answer to that
    /// question rather than a re-sort of the two hundred busiest by
    /// CPU (`top::rank`). Before it existed the card had to choose
    /// between a wrong order and ten thousand rows every five seconds.
    /// [`Self::load`] for an asker that holds the day stamped `held`
    /// (`Request::LoadSince`): the table, the day's stamp, and the day
    /// only when the stamp moved.
    pub fn load_since(
        root: &Path,
        top: usize,
        by: khor_core::TopBy,
        desc: bool,
        held: u64,
    ) -> (Vec<khor_core::TopRow>, u64, Option<khor_core::CpuDay>) {
        let history = history::CpuHistory::open(&Self::history_dir(root));
        let now = now_ms();
        let stamp = history.stamp(now);
        (
            top::sample_top(if top == 0 { usize::MAX } else { top }, by, desc),
            stamp,
            (stamp != held).then(|| history.day(now)),
        )
    }

    pub fn load(root: &Path, top: usize, by: khor_core::TopBy, desc: bool) -> khor_core::Load {
        khor_core::Load {
            top: top::sample_top(if top == 0 { usize::MAX } else { top }, by, desc),
            cpu_day: history::CpuHistory::open(&Self::history_dir(root)).day(now_ms()),
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

    /// The workspaces, read off this machine's copy of the table and
    /// **never off the network**: a workspace is a decision, decisions
    /// replicate, so the list is already here (`khor_sync::workspaces`).
    /// That is what lets this landing answer instantly and have no
    /// 「那台机器答不了」state at all — the two things that do need
    /// that machine (its git marks, its mtimes) are asked per row and
    /// paint nothing when they cannot be had.
    ///
    /// A row whose machine is no longer in the device table keeps its
    /// place with an empty `machine`: the directory is still on that
    /// disk, and dropping the row would make it unremovable.
    ///
    /// **Pinned first, then by path.** The screen re-sorts the rest by
    /// when each was last changed — a fact this call does not have and
    /// will not go and get.
    pub fn workspaces(&self) -> Result<Vec<workspace::WorkspaceInfo>, String> {
        let devices = self.devices()?;
        let mut out: Vec<workspace::WorkspaceInfo> = self
            .workspaces_loaded()?
            .doc
            .all()
            .into_iter()
            .filter_map(|(k, state)| {
                let (device, path) = khor_sync::workspaces::split(&k)?;
                Some(workspace::WorkspaceInfo {
                    machine: devices
                        .iter()
                        .find(|d| d.id == device)
                        .map(|d| d.shown().to_owned())
                        .unwrap_or_default(),
                    device: device.to_owned(),
                    name: workspace::basename(path),
                    path: path.to_owned(),
                    pinned: state == WsState::Pinned,
                })
            })
            .collect();
        out.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.device.cmp(&b.device))
        });
        Ok(out)
    }

    /// Takes a workspace off the list, for everyone. **The directory is
    /// not touched** (user, 2026-09-14): this list says what somebody
    /// is working on, and stopping working on something is not deleting
    /// it.
    pub fn ws_rm(&self, device_hex: &str, path: &str) -> Result<(), String> {
        self.ws_write(device_hex, path, WsState::No)
    }

    /// Floats a workspace to the top of the list, or lets it back down.
    /// `on` is explicit rather than a toggle for the reason every pin
    /// in this file is: a toggle typed twice is a no-op nobody can see.
    ///
    /// **Unpinning leaves it a workspace** — the two are one register
    /// with three states, so this can never take a row off the list by
    /// accident (`khor_sync::workspaces` module head).
    pub fn ws_pin(&self, device_hex: &str, path: &str, on: bool) -> Result<(), String> {
        self.ws_write(
            device_hex,
            path,
            if on { WsState::Pinned } else { WsState::Yes },
        )
    }

    /// The one place a workspace row is written.
    ///
    /// **Keyed by the machine's id and never by its name**, unlike
    /// every other verb in this file. The list can hold a row whose
    /// machine has left the device table — it must, or that row could
    /// never be taken off ([`Node::workspaces`]) — and such a row has
    /// no name at all. A name would have to be resolved, an *empty* one
    /// resolves to this machine ([`Self::resolve_or_here`]), and the
    /// write would land on a key about a different disk: the same
    /// directory on two Macs is two rows, and one of them would be
    /// quietly rewritten. Callers holding a name turn it into an id
    /// first ([`Node::machine_id`]).
    ///
    /// The path must be absolute for the reason `Ls` demands it — a
    /// relative path names a different directory on every machine that
    /// reads the row.
    fn ws_write(&self, device_hex: &str, path: &str, state: WsState) -> Result<(), String> {
        if !std::path::Path::new(path).is_absolute() {
            return Err(msg::path_not_absolute(path));
        }
        // Parsed rather than trusted: a key written under something
        // that is not an id is a row no reader can ever match, and the
        // write would look like it took. A machine that has left the
        // table still parses — that is the whole case this exists for.
        let id = device_id_from_hex(device_hex)?;
        // **Spelled the way the row was stored.** `ws_add` keys a
        // directory by its physical path (`link::ws_add`), so a pin or a
        // removal typed through a link must resolve to the same key or
        // it writes a second row instead of touching the first. Only
        // this machine's own links can be resolved here; a path on
        // another machine is taken as given, which is what every row on
        // the screen already carries.
        let path = if id == self.device() {
            std::fs::canonicalize(path)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.to_owned())
        } else {
            path.to_owned()
        };
        let loaded = self.workspaces_loaded()?;
        loaded
            .doc
            .set(&khor_sync::workspaces::key(&id.hex(), &path), state)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Pins or unpins a directory on a machine, for everyone. The key
    /// carries the machine because a bare path exists on every disk in
    /// the mesh (`khor_sync::dirpins` module head); the path must be
    /// absolute for the reason `Ls` demands it — a relative pin names
    /// nowhere.
    /// ⚠️ Retired as a face (docs/SHELL.md 第六轮): nothing but the
    /// migration's own tests writes this table any more.
    #[cfg(test)]
    pub(crate) fn pin_dir(&self, machine: &str, path: &str, on: bool) -> Result<(), String> {
        if !std::path::Path::new(path).is_absolute() {
            return Err(msg::path_not_absolute(path));
        }
        let (_, home) = self.resolve(machine)?;
        let loaded = self.dirpins_loaded()?;
        loaded
            .doc
            .set(&khor_sync::dirpins::key(&home.hex(), path), on)?;
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
            .filter_map(|k| khor_sync::dirpins::split(k).map(|(d, p)| (d.to_owned(), p.to_owned())))
            .collect())
    }

    /// Pins or unpins a web page against an exit machine, for everyone.
    /// The key carries the machine because a pinned page is "this page
    /// through this machine's network" (`khor_sync::webpins` module head)
    /// — the browser landing's shortcuts.
    /// ⚠️ Retired as a face (docs/SHELL.md 第六轮): nothing but the
    /// migration's own tests writes this table any more.
    #[cfg(test)]
    pub(crate) fn pin_web(&self, machine: &str, url: &str, on: bool) -> Result<(), String> {
        let (_, home) = self.resolve(machine)?;
        let loaded = self.webpins_loaded()?;
        loaded
            .doc
            .set(&khor_sync::webpins::key(&home.hex(), url), on)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Moves the retired 「置顶的网页 / 置顶的目录」 rows into the sidebars
    /// (docs/SHELL.md 第六轮:「浏览的置顶 = 侧边栏置顶组里的标签, 和会话一模
    /// 一样」). Answers how many moved this time.
    ///
    /// # Where they were, and where they go
    ///
    /// Both lived in **replicated** documents (`khor_sync::webpins`,
    /// `khor_sync::dirpins`) — one list for the whole network, keyed by
    /// (machine, place).
    ///
    /// - **A pinned page** becomes a pinned browse tab in *this* machine's
    ///   `.khor/tabs.json`, on the exit machine it was pinned against. Every
    ///   machine moves every page: the column used to be on every machine's
    ///   fresh tab, so every machine keeps it — as a pin in its own sidebar.
    /// - **A pinned directory** becomes a pinned **workspace** — a browse
    ///   tab cannot point at a directory (the files landing it belonged to
    ///   retired on 2026-09-11), and a directory somebody keeps on a machine
    ///   is exactly what the workspace list is. The workspace table is
    ///   replicated, so **only the machine that holds the directory** moves
    ///   it: otherwise a machine upgraded next week would pin again a
    ///   workspace somebody had taken off in the meantime.
    ///
    /// # Once
    ///
    /// Every key moved is written down in `.khor/place-pins-moved.json`,
    /// here and never replicated. A key in there is not moved again, so a
    /// second run moves nothing and a tab closed since is not brought back;
    /// a key that arrives later over sync (a machine that had not synced
    /// yet the first time) is moved when it arrives. The old documents are
    /// left as they are — other machines may still be on the old build —
    /// and nothing reads them but this.
    pub fn migrate_place_pins(&self) -> Result<usize, String> {
        let note = self.root.join(".khor").join("place-pins-moved.json");
        let mut done: std::collections::BTreeSet<String> = std::fs::read_to_string(&note)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let mut moved = 0;
        for (device, url) in self.web_pins()? {
            let key = format!("page\u{0}{device}\u{0}{url}");
            if done.contains(&key) {
                continue;
            }
            crate::tabs::adopt_pinned(&self.root, "page", &device, &url)?;
            done.insert(key);
            moved += 1;
        }
        let me = self.device_str().to_owned();
        for (device, path) in self.dir_pins()? {
            let key = format!("dir\u{0}{device}\u{0}{path}");
            if device != me || done.contains(&key) {
                continue;
            }
            self.ws_pin(&device, &path, true)?;
            done.insert(key);
            moved += 1;
        }
        if moved > 0 {
            let text = serde_json::to_vec_pretty(&done).map_err(|e| e.to_string())?;
            std::fs::write(&note, text).map_err(|e| msg::cant_write(note.display(), e))?;
        }
        Ok(moved)
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
            .filter_map(|k| khor_sync::webpins::split(k).map(|(d, u)| (d.to_owned(), u.to_owned())))
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
        self.recent_visit_from(kind, machine, place, None)
    }

    /// [`Self::recent_visit`], from a session: the place joins that
    /// session's slice of the 附栏's 最近 (UX-37 §1). `None` is a visit
    /// from nowhere in particular, which keeps whatever origin the place
    /// already had.
    pub fn recent_visit_from(
        &self,
        kind: PlaceKind,
        machine: &str,
        place: &str,
        session: Option<&str>,
    ) -> Result<(), String> {
        if kind != PlaceKind::Page && !std::path::Path::new(place).is_absolute() {
            return Err(msg::path_not_absolute(place));
        }
        let (_, home) = self.resolve(machine)?;
        let loaded = self.recents_loaded()?;
        loaded.doc.visit_from(
            &khor_sync::recents::key(kind, &home.hex(), place),
            crate::live::now_ms(),
            session,
        )?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// "Stop suggesting this", for every device. A tombstone rather
    /// than a deletion, so the dismissal travels — and going back there
    /// puts the place back on its own (`khor_sync::recents`).
    pub fn forget_recent(&self, kind: PlaceKind, machine: &str, place: &str) -> Result<(), String> {
        let (_, home) = self.resolve(machine)?;
        self.forget_recent_by_device(kind, &home.hex(), place)
    }

    /// Dismisses a ledger entry by its immutable device identity. The
    /// device need not remain in the device table: this writes a local,
    /// replicated tombstone and never opens a route to that machine.
    pub fn forget_recent_by_device(
        &self,
        kind: PlaceKind,
        device: &str,
        place: &str,
    ) -> Result<(), String> {
        let home = device_id_from_hex(device)?;
        let loaded = self.recents_loaded()?;
        loaded
            .doc
            .forget(&khor_sync::recents::key(kind, &home.hex(), place))?;
        let mut store = loaded.store;
        store.flush(&loaded.doc).map(|_| ())
    }

    /// Where this person has been of this kind, most recent first, at
    /// most `limit` of them. Dismissed places are not on it, and keys
    /// some other version spelled differently are skipped rather than
    /// guessed at.
    pub fn recents(&self, kind: PlaceKind, limit: usize) -> Result<Vec<RecentPlace>, String> {
        let loaded = self.recents_loaded()?;
        let doc = &loaded.doc;
        Ok(doc
            .all()
            .iter()
            .filter_map(|(k, at_ms)| {
                let (k_kind, device, place) = khor_sync::recents::split(k)?;
                (k_kind == kind).then(|| RecentPlace {
                    machine: device.to_owned(),
                    place: place.to_owned(),
                    at_ms: *at_ms,
                    session: doc.origin_of(k),
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

    /// Sets the directories a machine is asked to keep an eye on, for
    /// everyone (2026-09-06 关注目录) — the list rides that machine's row
    /// in the device table (`DeviceDoc::set_watch`), and the machine
    /// itself answers `Request::Watched` with what each is on. Refuses
    /// here, at the one write entry, so the CLI and the face say the
    /// same sentence (`tidy_watch`). Answers the list as kept: trimmed,
    /// deduplicated, in the order given.
    pub fn set_watch(&self, machine: &str, paths: &[String]) -> Result<Vec<String>, String> {
        let list = tidy_watch(paths)?;
        let (_, home) = self.resolve(machine)?;
        let loaded = self.devices_loaded()?;
        loaded.doc.set_watch(&home.hex(), &list)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(list)
    }

    /// Renames a machine, or takes the rename back when `label` is
    /// empty. Answers the name the row now shows, so a caller that just
    /// cleared an override does not have to guess what it fell back to.
    ///
    /// **By device id, not by name.** Every other writer here takes the
    /// name somebody typed and resolves it; this one must not, because
    /// the name is the thing being changed — resolving 「rename A to B」
    /// through a table where B may already exist is a way to rename the
    /// wrong row. The screens all hold the id already.
    ///
    /// Refuses a label that is only whitespace-and-punctuation away from
    /// nothing? No: **any text is a legal label**, including CJK, spaces
    /// and emoji. It is never a path segment and never a key — that is
    /// what `F_LABEL` being separate from the channel bought — so the
    /// rules `channel_of_machine` has to enforce do not apply to it. The
    /// one limit is length, so a row cannot be made unreadable by a
    /// paste.
    pub fn rename_machine(&self, device_id: &str, label: &str) -> Result<String, String> {
        if device_id_from_hex(device_id).is_err() {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        let label = label.trim();
        if label.chars().count() > MAX_LABEL {
            return Err(msg::label_too_long(MAX_LABEL));
        }
        let loaded = self.devices_loaded()?;
        loaded.doc.set_label(device_id, label)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(loaded
            .doc
            .get(device_id)
            .map(|d| d.shown().to_string())
            .unwrap_or_default())
    }

    /// The directories a machine's row asks it to watch — the setting,
    /// not the readings. Empty for a machine nobody asked, and for one
    /// the table does not hold.
    pub fn watch_of(&self, device_id: &str) -> Vec<String> {
        self.devices_loaded()
            .ok()
            .and_then(|l| l.doc.get(device_id))
            .map(|d| d.watch)
            .unwrap_or_default()
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
    ///
    /// The refusal names the verb that *is* this act done properly
    /// ([`Node::leave`], 2026-09-20): everything this one refuses to do
    /// by halves, that one does whole. A refusal with nowhere to go
    /// reads as 「khor cannot do this」 when what is true is 「not with
    /// this word」.
    pub fn forget_device(&self, machine: &str) -> Result<String, String> {
        let (name, home) = self.resolve(machine)?;
        if home == self.device() {
            return Err(msg::cant_forget_self(self.shown_name()));
        }
        self.forget_id(&home.hex())?;
        Ok(name)
    }

    /// Whether this machine logs in to `d` over ssh by the road on its
    /// row: it sponsors the satellite, or carries the member (round
    /// ROAD-4) — the rows whose kept secret stays.
    pub(crate) fn logs_in_to(&self, d: &khor_sync::devices::DeviceInfo) -> bool {
        let me = Some(self.device_str());
        d.sponsor.as_deref() == me || (d.sponsor.is_none() && d.carrier.as_deref() == me)
    }

    /// The names of the secrets this machine's ssh roads still ask for:
    /// what the sweep keeps ([`crate::secret::sweep`]).
    pub(crate) fn secrets_in_use(
        &self,
        devs: &[khor_sync::devices::DeviceInfo],
    ) -> std::collections::HashSet<String> {
        devs.iter()
            .filter(|d| self.logs_in_to(d))
            .filter_map(satellite::Dest::of)
            .filter(satellite::Dest::asks)
            .map(|to| to.secret_key())
            .collect()
    }

    /// [`Node::forget_device`] by id: 转正 drops the satellite row while
    /// the new member wears the same name, so a name would be ambiguous.
    pub(crate) fn forget_id(&self, id: &str) -> Result<(), String> {
        let _trust = self.root_mutation_guard()?;
        let loaded = self.devices_loaded()?;
        // A satellite of this machine, or a member it carries, that
        // logged in with a password or a key's passphrase: the secret
        // goes with it (docs/NET.md 添加机器 认证方式). Removed on another
        // machine, the sponsor's sweep drops it instead
        // (`satellite_sampler`).
        if let Some(row) = loaded.doc.get(id)
            && self.logs_in_to(&row)
            && let Some(to) = satellite::Dest::of(&row)
            && to.asks()
        {
            crate::secret::drop_kept(&self.root, &to.secret_key())?;
        }
        loaded.doc.set_gone(id, true)?;
        // On the machine holding the root the removal is also a
        // revocation (docs/CONNECT.md 名单是信任的唯一出处); anywhere else
        // it waits in `khor pending` for the holder.
        self.revoke_if_holder(&loaded.doc, id)?;
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
        let report = PeerReport {
            at_ms: now_ms(),
            name: name.to_owned(),
            rows: rows.to_vec(),
        };
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
        let table: Option<std::collections::BTreeSet<String>> = self
            .devices()
            .ok()
            .map(|devs| devs.into_iter().map(|d| d.id).collect());
        for e in rd.flatten() {
            if let Some(table) = &table
                && !e.file_name().to_str().is_some_and(|id| table.contains(id))
            {
                continue;
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

    /// Every directory holding what **other** machines left on this
    /// disk: their last reported rows, their readings, their watch
    /// lists. All three are caches keyed by device id, every reader
    /// walks the device table before reaching for one, and none of them
    /// holds anything this machine derived itself.
    ///
    /// It exists for `Node::leave`, which empties the table and has to
    /// empty these with it — and as one list rather than three calls at
    /// the call site, so a fourth cache added later is dropped by the
    /// leave that forgot it existed.
    pub(crate) fn peer_caches(&self) -> [PathBuf; 3] {
        [self.peers_dir(), self.vitals_dir(), self.watched_dir()]
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
        let report = VitalsReport {
            at_ms: now_ms(),
            vitals: v.clone(),
        };
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
        let report = DayReport {
            at_ms: now_ms(),
            day: day.to_owned(),
        };
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
        // One sample on the way back from a stopped process (a phone in
        // the background), not every one it missed — `link::serve` says why.
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let Ok(devs) = self.devices() else { continue };
            // The sweep (docs/NET.md 添加机器 认证方式): a password or
            // passphrase kept for a satellite whose row left the table —
            // removed here or on any other machine — goes now. A member
            // this machine carries logs in by its road too (round ROAD-4).
            let _ = crate::secret::sweep(&self.root, &self.secrets_in_use(&devs), now_ms());
            for d in devs {
                if d.sponsor.as_deref() != Some(self.device_str()) {
                    continue;
                }
                let Some(host) = satellite::Dest::of(&d) else {
                    continue;
                };
                // Off the runtime: a sample is an ssh dial plus a
                // one-second on-host measurement.
                let taken =
                    tokio::task::spawn_blocking(move || satellite::sample_read(&host)).await;
                // What broke is kept for the road to it (round ROAD-1).
                let mut book = self.roadbook.lock().unwrap_or_else(|p| p.into_inner());
                match taken {
                    Ok(Ok(v)) => {
                        book.sampled(&d.id, None);
                        drop(book);
                        let _ = self.cache_peer_vitals(&d.id, &v);
                    }
                    Ok(Err((_, broke, said))) => book.sampled(&d.id, Some((broke, said))),
                    Err(_) => {}
                }
            }
        }
    }

    /// What a machine last said about itself, and how old that is.
    ///
    /// **Every machine answers from memory, this one included**: this
    /// machine's reading is the background sampler's last round
    /// ([`vitals::local`]), aged like any other, and never taken inside
    /// the call — a sleeping disk made that call take ten seconds
    /// (docs/SHELL.md 第十六轮). A machine nobody has heard from yet, or
    /// this one before its sampler's first round, is `None` — a third
    /// state, distinct from an old reading, because "not asked yet" and
    /// "asked an hour ago" are different things to be told
    /// (docs/SESSION.md 离线).
    pub fn vitals_of(&self, device_id: &str) -> Option<(khor_core::Vitals, u64)> {
        if device_id == self.device_str() {
            return self.local_vitals().map(|r| (r.vitals, r.age_ms));
        }
        let text = fs::read_to_string(self.vitals_dir().join(device_id)).ok()?;
        let report: VitalsReport = serde_json::from_str(&text).ok()?;
        Some((report.vitals, now_ms().saturating_sub(report.at_ms)))
    }

    /// This machine's reading as the background sampler last took it,
    /// with the volumes that did not answer in time
    /// ([`vitals::local::Reading`]). `None` before its first round.
    pub fn local_vitals(&self) -> Option<vitals::local::Reading> {
        vitals::local::reading(&self.root)
    }

    /// [`Node::local_vitals`], waiting up to `wait` for the sampler's
    /// first round — for a verb that prints once and leaves, where
    /// 「还没问过」 would be a strange thing to say about this machine.
    /// **Blocking.**
    pub fn local_vitals_within(&self, wait: std::time::Duration) -> Option<vitals::local::Reading> {
        vitals::local::reading_within(&self.root, wait)
    }

    /// This machine's watched directories, waiting up to `wait` for a
    /// round that covers the whole list. **Blocking**, for the reason
    /// [`Node::local_vitals_within`] is.
    pub fn local_watched_within(
        &self,
        wait: std::time::Duration,
    ) -> Option<(Vec<khor_core::WatchedDir>, u64)> {
        let paths = self.watch_of(self.device_str());
        vitals::local::watched_within(&self.root, &paths, wait)
    }

    // ── what machines say about their watched directories ─────

    fn watched_dir(&self) -> PathBuf {
        self.root.join(".khor").join("watched")
    }

    /// Remembers what a peer answered about its watch list, stamped
    /// with now. **Its own record, not a field of the vitals one**: the
    /// two requests fail independently, and one shared timestamp would
    /// make a stale list look as fresh as the readings beside it
    /// (`link.rs`'s visit has the same rule for rows and vitals).
    pub(crate) fn cache_peer_watched(
        &self,
        device_id: &str,
        rows: &[khor_core::WatchedDir],
    ) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        fs::create_dir_all(self.watched_dir()).map_err(msg::cant_make_vitals_dir)?;
        let report = WatchedReport {
            at_ms: now_ms(),
            rows: rows.to_vec(),
        };
        link::write_private(
            &self.watched_dir().join(device_id),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    /// A machine's watched directories and how old the answer is — the
    /// background sampler's last round for this machine
    /// ([`vitals::local::watched`]), the last visit for every other.
    /// `None` for a peer that never answered (an older khor refuses the
    /// op, and that reads as no rows, never as empty ones), and for this
    /// machine before its sampler has looked.
    pub fn watched_of(&self, device_id: &str) -> Option<(Vec<khor_core::WatchedDir>, u64)> {
        if device_id == self.device_str() {
            return vitals::local::watched(&self.root, &self.watch_of(device_id));
        }
        let text = fs::read_to_string(self.watched_dir().join(device_id)).ok()?;
        let report: WatchedReport = serde_json::from_str(&text).ok()?;
        Some((report.rows, now_ms().saturating_sub(report.at_ms)))
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
        let report = UsageReport {
            at_ms: now_ms(),
            usage: u.clone(),
        };
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

    /// A file made ready to be fetched from here: its digest, and the
    /// offer record that serves its bytes (`transfer::Offer`) — the part
    /// of a send that is the same whoever receives it.
    pub(crate) fn offer_file(&self, path: &Path) -> Result<FileRef, String> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| msg::not_a_file_path(path.display()))?;
        let (digest, size) = transfer::digest_file(path)?;
        let f = FileRef {
            name: name.clone(),
            size: size as i64,
            digest: digest.clone(),
            holder: String::new(),
        };
        let path = path.canonicalize().map_err(msg::cant_canonicalize)?;
        // The same file offered again — to another machine, or again to
        // the same one — keeps what each receiver already did with it:
        // the state is per receiver (`transfer::Offer::to`).
        let to = transfer::load_offer(&self.root, &digest)
            .ok()
            .flatten()
            .filter(|o| o.path == path && o.size == size)
            .map(|o| o.to)
            .unwrap_or_default();
        transfer::save_offer(
            &self.root,
            &digest,
            &transfer::Offer {
                path,
                name,
                size,
                started: false,
                done: false,
                to,
                held: false,
            },
        )?;
        Ok(f)
    }

    /// The file message behind `transfer/<msg_id>`, with its board: a
    /// khor-wide conversation (the broadcast).
    pub(crate) fn find_transfer(&self, msg_id: &str) -> Result<(String, Message), String> {
        for board in self.known_wide() {
            if let Some(m) = self
                .chat
                .log(&board)?
                .messages
                .into_iter()
                .find(|m| m.id == msg_id && matches!(m.body, MsgBody::Files(_)))
            {
                return Ok((board, m));
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

    /// A khor-wide conversation's readout, oldest first — the broadcast
    /// (`wide/all`). ⚠️ 墓碑 (STORE-BRIEF phase 5): a machine's name read
    /// its `chat/<机器>` conversation; that session kind is gone, and its
    /// documents are neither read nor synced.
    pub fn log(&self, board: &str) -> Result<chat::ChannelLog, String> {
        self.chat.log(wide::board(board)?)
    }

    /// Marks a session seen; unread drops to zero — here and, once the
    /// watermark syncs, on every device. Seen is the one action every
    /// kind answers the same way: the kind names the watermark, the node
    /// persists it and repaints the row.
    pub fn seen(&self, id: &SessionId) -> Result<(), String> {
        self.seen_bounded(id, None)
    }

    /// A GUI receipt covers the row the person saw, not a completion that
    /// arrived while the receipt was in flight. CLI marks retain broad semantics.
    pub fn seen_through(&self, id: &SessionId, at_ms: i64) -> Result<(), String> {
        if at_ms < 0 {
            return Err("invalid read timestamp".to_owned());
        }
        self.seen_bounded(id, Some(at_ms))
    }

    fn seen_bounded(&self, id: &SessionId, through: Option<i64>) -> Result<(), String> {
        let bounded = |at: i64| through.map_or(at, |limit| at.min(limit));
        for k in self.kinds() {
            if !k.claims(self, id) {
                continue;
            }
            let at = k.seen_at(self, id)?;
            self.mark_seen(id, bounded(at))?;
            if let Some(row) = k.rows(self)?.into_iter().find(|r| r.id == *id) {
                self.emit(NodeEvent::Row(row));
            }
            return Ok(());
        }
        // A row another device reported: its stamp is the watermark, and
        // the mark replicates — the home's own row clears on next sync.
        if let Some(v) = self.sessions()?.into_iter().find(|v| v.session.id == *id) {
            return self.mark_seen(id, bounded(v.session.state.at.0 as i64));
        }
        Err(msg::no_such_session(&id.0))
    }

    /// Acknowledge only message versions in a rendered GUI snapshot. Validate
    /// against the current board, so forged future versions cannot clear an
    /// update that has not arrived. A stale snapshot only marks its old version.
    pub fn seen_snapshot(&self, id: &SessionId, messages: &[SeenMessage]) -> Result<(), String> {
        wide::board(&id.0)?;
        let log = self.chat.log(&id.0)?;
        let current: std::collections::HashMap<_, _> =
            log.messages.iter().map(|m| (m.id.as_str(), m.at)).collect();
        for message in messages {
            if message.at_ms < 0
                || !current
                    .get(message.id.as_str())
                    .is_some_and(|at| message.at_ms <= *at)
            {
                return Err("invalid rendered message version".to_owned());
            }
        }
        let loaded = self.seen_loaded()?;
        for message in messages {
            loaded.doc.mark_message(&id.0, &message.id, message.at_ms)?;
        }
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        let _ = self.emit_row_of(id);
        Ok(())
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
        // An imported row is taken off the list and nothing else — on
        // whichever machine it lives, since the list is one document.
        if self.take_off_if_imported(id)? {
            return Ok(());
        }
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
        if self.take_off_if_imported(id)? {
            return Ok(());
        }
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
        self.refuse_unless_hosting()?;
        let leaf = link::fresh_leaf()?;
        let id = SessionId(format!("{kind}/{leaf}"));
        self.live
            .register(&id, kind, title, None, Self::category_of_started(kind))?;
        self.roster_new(&id)?;
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
        self.refuse_unless_hosting()?;
        let leaf = link::fresh_leaf()?;
        let id = SessionId(format!("{kind}/{leaf}"));
        self.live
            .register(&id, kind, title, None, Self::category_of_started(kind))?;
        // **`khor run` happens where the person is standing**, and the
        // host inherits this process's directory — so the answer is
        // here, and asking for it as an argument would only give every
        // caller a chance to pass a different one.
        //
        // **The directory the process started in, not the one it is in
        // now** (第十四轮 第 4 条): opening a node pins the process to a
        // directory that cannot be deleted, and reading it back here
        // would put every `khor run` in `/`.
        if let Some(cwd) = crate::cwd::standing() {
            self.live.learn_place(&id, &cwd)?;
        }
        let dir = self
            .live
            .dir_of(&id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host(&dir, &id, cmd, size)?;
        self.roster_new(&id)?;
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
        self.refuse_unless_hosting()?;
        let leaf = link::fresh_leaf()?;
        let id = SessionId(format!("{kind}/{leaf}"));
        self.live
            .register(&id, kind, title, None, Self::category_of_started(kind))?;
        // Where it was born, said now because now is the only moment
        // anyone knows: a shell that has `cd`ed away cannot be asked
        // (`khor_core::Session::place`).
        self.live.learn_place(&id, cwd)?;
        let dir = self
            .live
            .dir_of(&id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host_at(cwd, &dir, &id, cmd, size)?;
        self.roster_new(&id)?;
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
    // Eight arguments because eight things describe the session being
    // opened, and every caller already spells all of them. Bundling them
    // into a struct would change a public signature for a lint's sake.
    #[allow(clippy::too_many_arguments)]
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
        self.refuse_unless_hosting()?;
        self.live.register(id, kind, title, None, category)?;
        self.live.learn_place(id, cwd)?;
        let dir = self
            .live
            .dir_of(id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        host::spawn_host_at(cwd, &dir, id, cmd, size)?;
        self.roster_new(id)?;
        Ok(id.clone())
    }

    /// Opens a GUI session: a detached host holding one ACP agent. The
    /// id comes back from the host — it is the vendor's own, and exists
    /// only once the agent has answered (`gui_host` module head).
    pub fn open_gui(&self, title: &str, cmd: &[String]) -> Result<SessionId, String> {
        self.refuse_unless_hosting()?;
        gui_host::spawn_gui_host(self.root(), None, title, cmd, None, false)
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
        self.refuse_unless_hosting()?;
        gui_host::spawn_gui_host(self.root(), None, title, cmd, Some(agent), false)
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
        self.refuse_unless_hosting()?;
        // Refused here rather than at the spawn: a directory that is
        // gone comes back from the child as a launch failure, and the
        // thing a person can act on is the path.
        if !cwd.is_dir() {
            return Err(msg::no_such_dir(cwd.display().to_string()));
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
        if let Some((_, pids)) = self.live.sighting_of(&adaptor::id_for(session))
            && let Some(pid) = pids.first()
        {
            return Err(msg::resume_running_elsewhere(pid));
        }
        let codex = adaptor::codex::Codex::at(adaptor::codex_home(&self.root));
        if codex.full_session_id(&live::clean_leaf(session)).as_deref() == Some(session)
            && codex.has_writer(session)?
        {
            return Err(msg::takeover_writer_busy(session));
        }
        gui_host::spawn_gui_host_reopen(self.root(), Some(cwd), title, cmd, Some(agent), session)
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
        self.refuse_unless_hosting()?;
        gui_host::spawn_gui_host(self.root(), Some(cwd), title, cmd, vendor, agent)
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
        let mut seen = adaptor::claude::Claude::at(adaptor::claude_home(&self.root)).places();
        seen.extend(adaptor::codex::Codex::at(adaptor::codex_home(&self.root)).places());
        seen.extend(self.live.places());
        place::fold(seen)
    }

    /// [`Node::places`] with each path also spelled as this machine's
    /// user reads it — the wire shape of [`crate::proto::Response::Places`],
    /// because `~` means this machine's home and only this machine knows
    /// which prefix that is.
    pub fn places_shown(&self) -> Vec<(String, String, u32, i64)> {
        self.places()
            .into_iter()
            .map(|p| {
                (
                    p.path.clone(),
                    place::shorten(Path::new(&p.path)),
                    p.sessions,
                    p.last_ms,
                )
            })
            .collect()
    }

    /// Opens a fresh agent session where the wizard pointed (会话身份批B:
    /// 建 session 四字段 — 目录、智能体、形式、名字). The two forms are
    /// the user ruling's two channels: `chat` hosts khor's own vendor
    /// shim behind the GUI host (`_cagent` / `_codexagent` — zero
    /// dependencies, the skin ruling), `term` hosts the vendor's TUI.
    /// Either way the session is born standing in the chosen directory.
    ///
    /// **On `Node` rather than in the GUI core, because it now runs on
    /// two machines**: locally when the wizard names this one, and on
    /// the far side of [`crate::proto::Request::OpenAgent`] when it
    /// names another. A name resolves to a command *here*, with this
    /// machine's paths and this machine's registrations — which is why
    /// the wire carries the name and not the command.
    ///
    /// `agent` is an open position (批⑥): khor's own two, or anything
    /// this machine registered — refused by name through
    /// [`Node::agent_or_refuse`], the CLI's own refusal.
    pub fn start_session(
        &self,
        dir: &str,
        title: &str,
        form: &str,
        agent: &str,
    ) -> Result<SessionId, String> {
        self.refuse_unless_hosting()?;
        let picked = match agent {
            "" | adaptor::claude::VENDOR => Picked::Claude,
            adaptor::codex::VENDOR => Picked::Codex,
            name => Picked::Registered(name.to_owned(), self.agent_or_refuse(name)?),
        };
        // Registered agents have no terminal launch spelling. Their
        // scheduler form receives khor's tools through standard session MCP.
        match (&picked, form) {
            // The TUI form needs a pre-nameable session, and a bare
            // thread/start writes no rollout to resume (probed).
            (Picked::Codex, "term") => return Err(msg::wizard_codex_chat_only().into()),
            (Picked::Registered(..), "term") => {
                return Err(msg::wizard_registered_no_terminal().into());
            }
            _ => {}
        }
        let home = std::env::home_dir();
        let path = match dir.trim() {
            "" | "~" => home.clone().ok_or_else(|| msg::no_such_dir("~"))?,
            d if d.starts_with("~/") => home
                .clone()
                .ok_or_else(|| msg::no_such_dir(d))?
                .join(&d[2..]),
            d => PathBuf::from(d),
        };
        if !path.is_dir() {
            return Err(msg::no_such_dir(path.display().to_string()));
        }
        let title = if title.trim().is_empty() {
            path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    if agent.is_empty() {
                        "claude".to_owned()
                    } else {
                        agent.to_owned()
                    }
                })
        } else {
            title.trim().to_owned()
        };
        match form {
            "chat" => {
                let (cmd, vendor) = picked.launch()?;
                self.open_gui_at(&path, &title, &cmd, Some(&vendor), false)
            }
            // 调度员 (docs/AGENT.md): the same conversation session, with
            // khor's own verbs in its hands — one flag's difference.
            "agent" => {
                let (cmd, vendor) = picked.launch()?;
                self.open_gui_at(&path, &title, &cmd, Some(&vendor), true)
            }
            "term" => {
                // The TUI child resolves claude the same way the shim
                // does, env door included, so both forms honour one
                // override.
                let claude = std::env::var("KHOR_CLAUDE").unwrap_or_else(|_| "claude".into());
                // **The session is named before it exists**, and the row
                // wears that name — the same trick the shim plays on the
                // protocol side, so 接管 has a vendor file to find.
                let vendor_session = adaptor::uuid_v4()?;
                let id = adaptor::id_for(&vendor_session);
                self.open_persistent_as(
                    &id,
                    &path,
                    khor_core::kind::TUI,
                    &title,
                    &[claude, "--session-id".into(), vendor_session],
                    (80, 24),
                    Some(adaptor::claude::VENDOR),
                )
            }
            other => Err(format!("no such form: {other}")),
        }
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
        self.refuse_unless_hosting()?;
        // A terminal wearing an agent's form (docs/SESSION.md 形态随进程变)
        // is taken over as **that agent**: the conversation is the
        // agent's, and the terminal it ran in stays where it is.
        let roster = self.roster_loaded()?;
        let worn = self.live.worn_by(id, |sid| roster.doc.entry(sid).is_some());
        let id = worn.as_ref().unwrap_or(id);
        let id_leaf =
            id.0.strip_prefix(&format!("{}/", khor_core::kind::TUI))
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
        // A khor-minted id names no vendor file; the hook-recorded
        // vendor leaf is the bridge (`transcript_of`'s precedent).
        let leaf = self
            .live
            .vendor_leaf_of(id)
            .unwrap_or_else(|| id_leaf.to_owned());
        let leaf = leaf.as_str();
        if let Some((kind, _)) = self.live.on_record(id)
            && kind == khor_core::kind::GUI
        {
            return Err(msg::already_a_chat(&id.0));
        }
        // Whose conversation is this? The store that resolves the leaf
        // answers. Generic ACP sessions use session/list + session/load;
        // this conversion specifically needs a known vendor CLI/store.
        let claude = adaptor::claude::Claude::at(adaptor::claude_home(&self.root));
        let (full, cwd, shim) = if let Some(full) = claude.full_session_id(leaf) {
            let cwd = claude
                .recorded_cwd(leaf)
                .or_else(std::env::home_dir)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            (full, cwd, "_cagent")
        } else {
            let codex = adaptor::codex::Codex::at(adaptor::codex_home(&self.root));
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
        // Refuse BEFORE touching a viewer or host when the only owner we know
        // is a thread writer in a potentially shared daemon.
        let codex_store = (shim == "_codexagent")
            .then(|| adaptor::codex::Codex::at(adaptor::codex_home(&self.root)));
        // A khor-owned terminal explicitly running `codex resume <uuid>` is
        // also exact ownership evidence, including before its first OSC title.
        let owned_resume = codex_store.is_some()
            && self
                .live
                .dir_of(id)
                .and_then(|dir| host::read_host_file(&dir).ok())
                .is_some_and(|host| {
                    if !link::pid_running(host.host_pid) {
                        return false;
                    }
                    let procs = adaptor::Procs::snapshot();
                    procs.held_by(&[host.child_pid]).into_iter().any(|pid| {
                        crate::codex_terminal::scoped(&procs, pid)
                            && procs.command(pid).is_some_and(|argv| {
                                let args = argv
                                    .iter()
                                    .skip(1)
                                    .map(std::ffi::OsString::from)
                                    .collect::<Vec<_>>();
                                crate::codex_process::resuming(&args, &full)
                            })
                    })
                });
        if let Some(store) = &codex_store {
            if store.has_writer(&full)?
                && !owned_resume
                && sighting.as_ref().is_none_or(|(_, pids)| pids.is_empty())
            {
                return Err(msg::takeover_writer_busy(&id.0));
            }
        }
        let named = if shim == "_codexagent" {
            std::env::var("KHOR_CODEX").unwrap_or_else(|_| "codex".into())
        } else {
            std::env::var("KHOR_CLAUDE").unwrap_or_else(|_| "claude".into())
        };
        let path = crate::spawn_path(&named);
        crate::find_program(&named, &path).ok_or_else(|| msg::command_not_found(&named, &path))?;
        if !cwd.is_dir() {
            return Err(msg::takeover_no_record(&id.0));
        }
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
            for hf in [host::read_view_file(&dir), host::read_host_file(&dir)]
                .into_iter()
                .flatten()
            {
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
        if let Some(store) = &codex_store {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while store.has_writer(&full)? {
                if std::time::Instant::now() >= deadline {
                    return Err(msg::takeover_writer_busy(&id.0));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        let exe = crate::self_exe()?;
        gui_host::spawn_gui_host_resume(
            self.root(),
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
        self.refuse_unless_hosting()?;
        let Some((kind, title)) = self.live.on_record(id) else {
            return Err(msg::takeover_no_record(&id.0));
        };
        if kind != khor_core::kind::GUI {
            return Err(msg::already_a_terminal(&id.0));
        }
        // A GUI row's id **is** the vendor's uuid, lossily spelled
        // (`adaptor::id_for`); the transcript's own filename is the
        // whole one.
        let leaf =
            id.0.strip_prefix(&format!("{}/", khor_core::kind::TUI))
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
        // The same store-resolves-it vendor fork as `takeover`, plus
        // each vendor's own resume spelling: claude takes a flag,
        // codex a subcommand (`codex resume <uuid>`, probed 0.146.1).
        let claude = adaptor::claude::Claude::at(adaptor::claude_home(&self.root));
        let (cwd, mut resume) = if let Some(full) = claude.full_session_id(leaf) {
            let cwd = claude
                .recorded_cwd(leaf)
                .or_else(std::env::home_dir)
                .ok_or_else(|| msg::takeover_no_record(&id.0))?;
            let bin = std::env::var("KHOR_CLAUDE").unwrap_or_else(|_| "claude".into());
            (cwd, vec![bin, "--resume".into(), full])
        } else {
            let codex = adaptor::codex::Codex::at(adaptor::codex_home(&self.root));
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
        let named = &resume[0];
        let path = crate::spawn_path(named);
        let bin = crate::find_program(named, &path)
            .ok_or_else(|| msg::command_not_found(named, &path))?;
        if resume.get(1).is_some_and(|arg| arg == "resume")
            && crate::codex_process::supports_no_daemon(std::path::Path::new(&bin), &path)
        {
            resume.insert(1, "--no-daemon".into());
        }
        if !cwd.is_dir() {
            return Err(msg::takeover_no_record(&id.0));
        }
        let dir = self
            .live
            .dir_of(id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;

        // End the conversation's body. The ghost writes itself as both
        // host and child (`gui_host`), so its group is the shim and the
        // agent too.
        for hf in [host::read_view_file(&dir), host::read_host_file(&dir)]
            .into_iter()
            .flatten()
        {
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
        // **A form khor holds is on the list, whatever it was before**
        // (docs/SHELL.md 第八轮): a conversation khor opened was never in
        // the roster — it did not need to be — and turning it into a
        // terminal makes it a `tui` row, which does.
        self.roster_new(id)?;
        self.live.reseat(id, khor_core::kind::TUI, &title, None)
    }

    /// Stands a host up for a discovered tmux session so a terminal can
    /// attach (`LiveKind::attach_multiplexed` has the whole judgment).
    ///
    /// **Refused on a machine that hosts nothing** (a phone, docs/IOS.md):
    /// it has no tmux to bridge and no host file to find, and a door that
    /// fell through to here for a far row must say so rather than look
    /// for that row's host on this disk.
    pub fn attach_tmux(&self, id: &SessionId) -> Result<live::Seat, String> {
        if !self.hosts {
            return Err(msg::no_terminal_here(&id.0));
        }
        self.live.attach_multiplexed(id)
    }

    /// How this machine's dead session ended (`live::Ending`, P5-SHELL
    /// item 3): the row's second line names it. `None` for a session that
    /// is not dead or not this machine's — a reported row's home says it.
    pub fn ending(&self, id: &SessionId) -> Option<live::Ending> {
        self.live.ending(id)
    }

    /// A dead session's page (`live::Ended`, P5-SHELL items 1 and 2): its
    /// last screen when its host left one, else the commands it ran.
    pub fn ended(&self, id: &SessionId) -> Option<live::Ended> {
        self.live.ended(id)
    }

    /// The registry dir behind a live session id, for attach clients.
    /// **None on a machine that hosts nothing**: no session's host lives
    /// on it, so nothing there is an address to attach to.
    pub fn session_dir(&self, id: &SessionId) -> Option<PathBuf> {
        if !self.hosts {
            return None;
        }
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
        // A machine that hosts nothing hosts no terminal (docs/IOS.md),
        // and is not asked to read host files to say so.
        self.hosts
            && self.live.dir_of(id).is_some_and(|d| {
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
    ///
    /// ⚠️ **That paragraph is about the rows, and it does not reach the
    /// `today` field** — a distinction worth spelling out because
    /// missing it is what put a `None` there for a day (the tombstone
    /// is on the field, below). 「Whose rows are these」 has no single
    /// answer; 「who answered this」 has exactly one, and it is the
    /// machine running this code.
    ///
    /// **The model breakdown is merged too, and it has to be.** An
    /// empty `models` beside a full `days` is the one sentence
    /// `khor_core::UsageModelDay` reserves for 「the answering machine
    /// cannot cut by model」 — so a summary that summed the days and
    /// dropped the models would not lose a feature, it would **state
    /// something false about every machine in the network**, including
    /// the ones that answered in full. That is the shape this tree
    /// keeps meeting: a branch nobody wired up does not become "no
    /// value", it becomes the value next to it.
    ///
    /// A machine that genuinely cannot cut by model contributes its
    /// days and no model rows, so this list is **not** exhaustive over
    /// this one's days the way a single machine's is — mixing an old
    /// khor into the network is exactly the case where the property has
    /// to give, and the honest place for it to give is here rather than
    /// in a machine's own answer.
    pub fn usage_everywhere(&self) -> Result<khor_core::Usage, String> {
        let mut rows: std::collections::BTreeMap<(String, String), khor_core::Tokens> =
            Default::default();
        let mut models: std::collections::BTreeMap<
            (String, String, Option<String>),
            khor_core::Tokens,
        > = Default::default();
        let mut unreadable = 0u64;
        for d in self.devices()? {
            let Some((u, _)) = self.usage_of(&d.id) else {
                continue;
            };
            unreadable = unreadable.saturating_add(u.unreadable);
            for day in u.days {
                rows.entry((day.day, day.category))
                    .or_default()
                    .add(day.tokens);
            }
            for m in u.models {
                models
                    .entry((m.day, m.category, m.model))
                    .or_default()
                    .add(m.tokens);
            }
        }
        Ok(khor_core::Usage {
            // Oldest first, the same order one machine's answer comes in
            // — a face that had to sort would be the second sort this
            // repo keeps refusing to grow (`list`).
            days: rows
                .into_iter()
                .map(|((day, category), tokens)| khor_core::UsageDay {
                    day,
                    category,
                    tokens,
                })
                .collect(),
            unreadable,
            // `BTreeMap` puts `None` **first** among the models, where a
            // single machine's answer puts it last. Both orders are
            // sorted and neither is the other's bug; a face that cares
            // orders what it draws.
            models: models
                .into_iter()
                .map(
                    |((day, category, model), tokens)| khor_core::UsageModelDay {
                        day,
                        category,
                        model,
                        tokens,
                    },
                )
                .collect(),
            // **This machine's own day, which is what the field means.**
            //
            // ⚠️ **Tombstone (2026-09-08): this was `None`, and the
            // reasoning under it was wrong in a way worth keeping.** It
            // read 「these rows were each cut in their own machine's
            // zone, so any date here would be one machine's opinion
            // wearing the network's name」 — a true sentence about
            // *whose rows these are*, applied to a field that is not
            // about the rows. `khor_core::Usage::today` says what it
            // is: **the day on the machine that answered**, and the
            // machine that answered this is the one running this code.
            //
            // The consequence of the old `None` was not a missing
            // figure: a face needs *some* today to cut 「the last seven
            // days」 with, so it would have reached for its own — right
            // on the machine it runs on and off by a day about a khor
            // it reached over the web. **A branch nobody wired up does
            // not become 「no value」, it becomes the value next to
            // it**, and this tree keeps meeting that.
            //
            // Comparing this machine's window against a peer's days is
            // off by at most a day at the boundary, which
            // `usage::window_start` already weighed and called the
            // smaller of the two errors available — and `khor usage`
            // has cut its window this way all along.
            today: Some(usage::today().to_string()),
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
    pub fn transcript_of(&self, id: &SessionId) -> Result<Vec<adaptor::claude::Utterance>, String> {
        let leaf =
            id.0.strip_prefix(&format!("{}/", khor_core::kind::TUI))
                .ok_or_else(|| msg::no_such_session(&id.0))?;
        // A khor-opened row's own leaf is khor's mint and names no
        // vendor file; the hook-recorded vendor leaf is the bridge
        // (`live::Meta::vendor_leaf`). A discovered row has no record
        // and needs none — its leaf *is* the vendor's.
        let leaf = self
            .live
            .vendor_leaf_of(id)
            .unwrap_or_else(|| leaf.to_owned());
        let claude = adaptor::claude::Claude::at(adaptor::claude_home(&self.root));
        match claude.transcript(&leaf) {
            Ok(t) => Ok(t),
            // The other store's turn — the takeover fork's rule:
            // whoever resolves the leaf owns the session.
            Err(claude_why) => {
                let codex = adaptor::codex::Codex::at(adaptor::codex_home(&self.root));
                match codex.full_session_id(&leaf) {
                    Some(full) => codex.transcript(&full),
                    None => Err(claude_why),
                }
            }
        }
    }

    /// A bounded page of an external transcript, latest events first loaded
    /// from the tail. The cursor is scoped to this session and file identity.
    pub fn transcript_page(
        &self,
        id: &SessionId,
        cursor: Option<&adaptor::transcript_page::Cursor>,
    ) -> Result<adaptor::transcript_page::Page, String> {
        let leaf =
            id.0.strip_prefix(&format!("{}/", khor_core::kind::TUI))
                .ok_or_else(|| "session has no external transcript source".to_owned())?;
        let leaf = self
            .live
            .vendor_leaf_of(id)
            .unwrap_or_else(|| leaf.to_owned());
        let claude = adaptor::claude::Claude::at(adaptor::claude_home(&self.root));
        let (path, parse): (_, fn(&str) -> Vec<adaptor::claude::Utterance>) =
            if let Some((path, _)) = claude.transcript_path(&leaf) {
                (path, adaptor::claude::utterances_of)
            } else {
                let codex = adaptor::codex::Codex::at(adaptor::codex_home(&self.root));
                let full = codex
                    .full_session_id(&leaf)
                    .ok_or_else(|| "no transcript source".to_owned())?;
                let path = codex
                    .rollout_path(&full)
                    .ok_or_else(|| "no transcript source".to_owned())?;
                (path, adaptor::codex::utterances_of)
            };
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        adaptor::transcript_page::read_page(&mut file, &id.0, cursor, parse)
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
        let seek =
            id.0.strip_prefix(&format!("{}/", khor_core::kind::TUI))
                .map(|leaf| {
                    self.live
                        .vendor_leaf_of(id)
                        .unwrap_or_else(|| leaf.to_owned())
                })
                .and_then(|leaf| {
                    adaptor::claude::Claude::at(adaptor::claude_home(&self.root))
                        .transcript_since(&leaf, from)
                        .ok()
                        .or_else(|| {
                            let codex = adaptor::codex::Codex::at(adaptor::codex_home(&self.root));
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
        // A phone has no claude to tell khor anything (docs/IOS.md): the
        // card does not offer this there, and answering 「装好了」 for a
        // settings file in a sandbox nobody reads was the lie.
        if !self.hosts {
            return Err(msg::hooks_need_a_computer().to_owned());
        }
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

impl KindSurface for TransferKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        id.0.starts_with("transfer/")
    }

    /// No rows of its own any more (传输并入对话批, 2026-08-23): a
    /// transfer is a file message in the broadcast, and its pressure lands
    /// on that row's word (`wide::WideKind`'s rows). The kind itself
    /// stays: `claims`,
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
            .filter_map(|f| road::landed(&node.root, &dir, f).and_then(|p| transfer::mtime_ms(&p)))
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
        let dir = khor_sync::chat::board_dir(node.root(), &channel)
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
        // Which rows are on the list decides who holds a host's seat
        // (docs/SESSION.md 形态随进程变): the row a person keeps must not
        // fold into one they never put there.
        let roster = node.roster_loaded()?;
        Ok(self.rows_listed(
            |sid| seen.doc.watermark(sid),
            |sid| roster.doc.entry(sid).is_some(),
        ))
    }

    /// Live state never syncs (docs/NET.md) — only home can derive these
    /// rows, so they travel as reports.
    fn reportable(&self) -> bool {
        true
    }

    /// The stamp of what the row **shows** — a terminal wearing a claude
    /// that just finished is marked seen up to that 完成, not up to the
    /// terminal's own older word.
    fn seen_at(&self, node: &Node, id: &SessionId) -> Result<i64, String> {
        let roster = node.roster_loaded()?;
        self.shown_stamp(id, |sid| roster.doc.entry(sid).is_some())
    }

    fn close(&self, _node: &Node, id: &SessionId) -> Result<(), String> {
        self.close_session(id)
    }
}

/// How an ACP conversation is spelled among the candidates: it is not a
/// session yet, so it has no session id — `acp:<agent>:<its own id>`.
pub const ACP_CANDIDATE: &str = "acp:";

/// One thing [`Node::find`] found: what it is, how a face jumps to it,
/// what a person reads, and where it lives.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Hit {
    /// `session` / `machine` / `workspace` / `tab` / `page`.
    pub kind: String,
    /// The session id, the device id, `<device>:<path>`, the tab id, or
    /// the page's own key — whatever that kind is reached by.
    pub id: String,
    pub label: String,
    /// The machine it is on, the path, the address — what tells two
    /// things with one name apart.
    pub note: Option<String>,
}

/// One thing that could be brought onto the session list
/// ([`Node::import_candidates`]).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Candidate {
    /// What `khor import` and the face send back.
    pub id: String,
    /// The row it becomes: `shell`, `tui` (watched) or `gui` (an ACP
    /// conversation).
    pub form: String,
    /// The machine it is on, by device id.
    pub home: String,
    pub title: String,
    /// Where it runs, when anybody could say.
    pub place: Option<String>,
    /// Its last activity, ms — the order the panel lists them in.
    pub at_ms: i64,
    pub category: Option<String>,
    /// The ACP agent it belongs to, for a conversation.
    pub agent: Option<String>,
    /// What runs in it now, as the row's own reading says
    /// ([`khor_core::Doing::command`]): for a tmux session the pane in
    /// front's program, or the command as typed where its shell marks.
    /// The import panel finds a session by it (IMPORT-BRIEF). `None` at
    /// a prompt, and for an agent, whose row says the agent instead.
    pub command: Option<String>,
}

/// Whether a row is on the session list (docs/SHELL.md 第八轮). A
/// device's conversation, a transfer, a borrow and an ACP conversation
/// are khor's by construction; a terminal or an agent in one is on the
/// list when khor started it or somebody imported it (`khor_sync::roster`).
fn listed(roster: &RosterDoc, row: &Session) -> bool {
    let kind = row.kind.0.as_str();
    !(kind == khor_core::kind::SHELL || kind == khor_core::kind::TUI)
        || roster.entry(&row.id.0).is_some()
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

/// One device's last answer about its watched directories, as cached
/// on disk (`cache_peer_watched`).
#[derive(serde::Serialize, serde::Deserialize)]
struct WatchedReport {
    at_ms: u64,
    rows: Vec<khor_core::WatchedDir>,
}

/// The watch list as it may be kept (2026-09-06 关注目录): every path
/// absolute, trimmed, without `..`, without a trailing slash, and no
/// path twice — refused by name otherwise, in the catalog's words, at
/// this one place so that `khor watch` and the face's editor say the
/// same thing. Order is the order given.
pub fn tidy_watch(paths: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for raw in paths {
        let p = raw.trim();
        if p.is_empty() {
            return Err(msg::watch_path_empty().into());
        }
        if !p.starts_with('/') {
            return Err(msg::watch_path_relative(p));
        }
        if Path::new(p)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(msg::watch_path_dotdot(p));
        }
        let p = if p.len() > 1 {
            p.trim_end_matches('/')
        } else {
            p
        };
        if out.iter().any(|q| q == p) {
            return Err(msg::watch_path_twice(p));
        }
        out.push(p.to_owned());
    }
    Ok(out)
}

#[cfg(test)]
mod watch_tests {
    use super::tidy_watch;

    /// Every refusal by name, and the one tidy that is not a refusal —
    /// so `khor watch` and the card's editor cannot differ on what a
    /// list may hold. The positive case runs first: a validator that
    /// refused everything would pass the four negatives alone.
    #[test]
    fn a_watch_list_is_kept_tidy_and_refused_by_name() {
        let kept = tidy_watch(&[" /data/models/ ".into(), "/".into(), "/scratch".into()]).unwrap();
        assert_eq!(
            kept,
            vec!["/data/models", "/", "/scratch"],
            "trimmed, and a trailing slash goes — but not the root's"
        );
        // Each refusal by its own sentence, read from the catalog: a
        // fragment spelled here went stale the day the catalog was
        // reworded (2026-09-28, 「不能是空的」 → 「不能为空」).
        assert_eq!(
            tidy_watch(&["models".into()]).unwrap_err(),
            khor_catalog::msg::watch_path_relative("models")
        );
        assert_eq!(
            tidy_watch(&["/data/../etc".into()]).unwrap_err(),
            khor_catalog::msg::watch_path_dotdot("/data/../etc")
        );
        assert_eq!(
            tidy_watch(&["  ".into()]).unwrap_err(),
            khor_catalog::msg::watch_path_empty()
        );
        assert_eq!(
            tidy_watch(&["/data".into(), "/data/".into()]).unwrap_err(),
            khor_catalog::msg::watch_path_twice("/data")
        );
    }
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
        LOCALE_VARS
            .iter()
            .map(|v| format!("\"${v}\""))
            .collect::<Vec<_>>()
            .join(" ")
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

/// The wizard's 智能体 answer, resolved to something this machine can
/// start ([`Node::start_session`]).
enum Picked {
    Claude,
    Codex,
    Registered(String, AgentSpec),
}

impl Picked {
    /// What to run, and what the row will say it is.
    ///
    /// khor's own two run khor's own shims (零依赖, the skin ruling); a
    /// registered agent runs the command it was registered with, as the
    /// launch JSON the protocol crate reads. **All three go out as a
    /// launch JSON carrying the user's `PATH`** (`Node::agent_launch`),
    /// khor's own two included: what `_cagent` spawns is `claude`, by
    /// bare name, and on a launchd-started serve that name is not on
    /// the PATH the process has.
    fn launch(&self) -> Result<(Vec<String>, String), String> {
        let mine = |shim: &str, vendor: &str| -> Result<(Vec<String>, String), String> {
            let exe = self_exe()?;
            let spec = AgentSpec {
                command: exe.display().to_string(),
                args: vec![shim.to_owned()],
            };
            Ok((vec![Node::agent_launch(&spec)?], vendor.to_owned()))
        };
        match self {
            Picked::Claude => mine("_cagent", adaptor::claude::VENDOR),
            Picked::Codex => mine("_codexagent", adaptor::codex::VENDOR),
            Picked::Registered(name, spec) => Ok((vec![Node::agent_launch(spec)?], name.clone())),
        }
    }
}
