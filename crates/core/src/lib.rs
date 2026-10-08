//! The session primitive: every interaction answers five questions
//! (docs/SESSION.md). Pure data — no IO, no async.
//!
//! Wire discipline for everything serialized here: msgpack structs are
//! positional arrays, so field order is wire order, new fields go at the
//! tail with serde defaults, and `skip_serializing_if` is banned — a
//! mid-frame optional desyncs the array on every older end.

use serde::{Deserialize, Serialize};

pub mod avatar;
pub mod map;

/// A device's identity: its public key (docs/NET.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub [u8; 32]);

impl DeviceId {
    /// The id as the device table spells it: the public key, lowercase
    /// hex. The table is keyed by this, so anything holding a `DeviceId`
    /// and wanting the table's entry goes through here rather than
    /// spelling the conversion again.
    pub fn hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Network-unique, stable across reconnects. Minting is kind-specific;
/// everything outside the kind treats it as opaque.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

/// Kinds are an open set: a client older than the kind must still render
/// the row, so this is a string, not an enum. Nothing may match on it
/// exhaustively.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Kind(pub String);

pub mod kind {
    //! Kinds shipped with Khor. A list of names, not an enum: new kinds
    //! must appear without touching this crate.
    pub const SHELL: &str = "shell";
    pub const TUI: &str = "tui";
    /// A conversation khor is the protocol client of (ACP): khor paints
    /// the exchange itself, there is no terminal to attach.
    pub const GUI: &str = "gui";
    pub const CHAT: &str = "chat";
    /// The group conversation of every machine in the khor (STORE-BRIEF
    /// phase 3): a session of the whole khor, not of one machine.
    pub const BROADCAST: &str = "broadcast";
    pub const TRANSFER: &str = "transfer";
    /// A held borrow of another machine's network (docs/NET.md 借网): a
    /// local proxy in front of one lease. It is the one borrow shape that
    /// is a session of its own — a session or a browser window borrowing
    /// an exit wears the exit on its own row, but a proxy the user holds
    /// by hand has no other row to ride, so it is one.
    pub const BORROW: &str = "borrow";
}

pub mod category {
    //! Whose session this is — see [`crate::Session::category`].
    //!
    //! Only the two Khor produces itself are named here. Vendor
    //! categories are the adaptors' own names and arrive with them, so
    //! a third vendor must not have to touch this crate — the same
    //! open-set rule [`kind`] follows.

    /// Khor's own: a device chat, a file transfer. Not a vendor's, and
    /// not a command the user started either.
    pub const KHOR: &str = "khor";
    /// A command the user started through Khor (`khor run`, `khor open`)
    /// and nothing more is known about. **Not a fallback**: it says "you
    /// started this yourself", which is a real answer. A row nobody can
    /// place carries no category at all.
    pub const SHELL: &str = "shell";
}

/// Milliseconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Millis(pub u64);

/// The six state words. There is no seventh: unreachable is a freshness
/// axis, not a state (docs/SESSION.md, the offline section).
///
/// Crosses the wire as its `key()` string, never as a variant index —
/// indices renumber when variants reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum State {
    Busy,
    /// An action is stuck and the user's approval is the only way out.
    /// "Waiting to be looked at" is `Done`, not this — a badge that can
    /// never reach zero is no badge.
    Blocked,
    Done,
    /// Something failed but the process is alive; continuing needs no
    /// restart. The name must not suggest "stopped".
    Errored,
    Failed,
    Idle,
}

impl State {
    pub const ALL: [State; 6] = [
        State::Busy,
        State::Blocked,
        State::Done,
        State::Errored,
        State::Failed,
        State::Idle,
    ];

    /// Where this word sits when a list is grouped by state: **who is
    /// waiting for you**, most first.
    ///
    /// Not the declaration order, and not severity. A failed session is
    /// over and asks nothing of anyone; a blocked one cannot move at all
    /// until you answer, which is why it leads even though nothing is
    /// wrong with it.
    ///
    /// 失败 lands last because 失败沉底 is already this codebase's rule
    /// (docs/UX.md 角标, and `live.rs` says "sinks in the list" twice):
    /// it stays on the list, it never holds the badge, and it does not
    /// push past sessions that are still going.
    ///
    /// **One order, defined here.** Any face that grouped by state would
    /// otherwise have to enumerate the six words itself, and a second
    /// enumeration is how a seventh word gets invented (docs/UX.md
    /// 状态呈现).
    pub const fn rank(self) -> u8 {
        match self {
            // An action is stuck; nothing proceeds until you allow it.
            State::Blocked => 0,
            // Broken but alive: it is waiting on whether to carry on.
            State::Errored => 1,
            // The turn ended; waiting to be read.
            State::Done => 2,
            // Working — the one word that explicitly does not need you.
            State::Busy => 3,
            State::Idle => 4,
            // Over, and 沉底.
            State::Failed => 5,
        }
    }

    /// Wire and catalog key. The UI looks display words up by this;
    /// nothing user-facing is spelled in code (docs/UX.md).
    pub const fn key(self) -> &'static str {
        match self {
            State::Busy => "busy",
            State::Blocked => "blocked",
            State::Done => "done",
            State::Errored => "errored",
            State::Failed => "failed",
            State::Idle => "idle",
        }
    }
}

/// **Which road this machine is actually using to reach another one,
/// right now.**
///
/// A reading, not a setting — nobody picks it, iroh does, and it can
/// change under you mid-connection. The question it exists to answer is
/// one a person cannot answer by feel: on the campus network measured
/// 2026-08-21 a direct path was 3.8–9.8ms and the relay 6.0ms, so
/// **falling back to a relay is silent**. Without a readout, "do we
/// still need a relay on this network" is unanswerable.
///
/// # Read off the wire this machine holds, not off iroh's address book
///
/// The resident keeps one long-lived connection to every machine it
/// talks to (`khor_node`'s wires), and the word is **the path that
/// connection has selected**, settled over two samples a second apart so
/// a path being re-chosen does not flicker the row. No wire, or a wire
/// that went silent and was retired, is [`Hop::Idle`] — not connected,
/// which is not unreachable ([`Link::down_ms`]).
///
/// ⚠️ 墓碑 (2026-10-03, round NET): this used to read iroh's
/// `remote_info`, counting every address it still marked open. That map
/// is not cleared when a connection closes, so a machine frozen for 40 s
/// read 直连 in all 225 samples taken, and a fifth variant, `Unknown`
/// (说不出), stood for five unrelated things: no entry in the map, the
/// status call failing, a late answer, a missing row, and a phone's new
/// endpoint. None of those was "the road is unknown"; each now says what
/// it is — not connected, or the last reading kept.
///
/// # Every variant is a fact
///
/// - [`Hop::Idle`] is *khor holds no live wire to that machine*. That is
///   not *unreachable*: a wire closes when it idles, and the next open
///   dials it fine. Unreachable is [`Link::down_ms`] beside it — the
///   resident's last attempt to reach that machine failed and nothing
///   has been heard from it since (round NET-13).
/// - [`Hop::NoServe`] is *nobody on this machine is holding the
///   endpoint*, so there was no one to ask. Fixable, and the fix is a
///   command — which is why it is not folded into not connected.
///
/// Liveness rides beside the word, not inside it ([`Link::heard_ms`]):
/// how long since that machine was last heard on the wire.
///
/// Crosses the wire as its `key()` string, like [`State`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Hop {
    /// Straight there — the selected path is an IP.
    Direct,
    /// Through a relay server.
    Relay,
    /// No live wire to that machine.
    Idle,
    /// No resident on this machine, so nothing here holds a wire to read
    /// the answer off.
    NoServe,
    /// Over this machine's own ssh session to it (round ROAD-2, khor
    /// inside ssh): this machine carries it. Tail-appended; an older face
    /// paints the key.
    Ssh,
    /// Through a third khor machine that splices the two ends (round
    /// ROAD-2, the peer hop). Tail-appended.
    Forwarded,
}

impl Hop {
    pub const ALL: [Hop; 6] = [Hop::Direct, Hop::Relay, Hop::Idle, Hop::NoServe, Hop::Ssh, Hop::Forwarded];

    /// Wire and catalog key.
    pub const fn key(self) -> &'static str {
        match self {
            Hop::Direct => "direct",
            Hop::Relay => "relay",
            // **Not `idle`**, though that is the plain word: `[state]`
            // already owns an `idle`, and a face that looked a hop up in
            // the wrong catalog table would then find a real word and
            // paint it. A key that exists in only one table makes that
            // mistake echo the key instead, which is visible.
            Hop::Idle => "quiet",
            Hop::NoServe => "no-serve",
            Hop::Ssh => "ssh",
            Hop::Forwarded => "forwarded",
        }
    }
}

impl From<Hop> for String {
    fn from(hop: Hop) -> String {
        hop.key().to_owned()
    }
}

impl TryFrom<String> for Hop {
    type Error = String;

    fn try_from(s: String) -> Result<Hop, String> {
        Hop::ALL
            .into_iter()
            .find(|h| h.key() == s)
            .ok_or_else(|| format!("not a hop: {s}"))
    }
}

/// **One workspace's facts, as the machine holding it keeps them**
/// (round NET-8, UX-15's workspace row): how many files differ in the
/// checkout, its branch, and when the tree last changed. Kept by that
/// machine's resident and redone when its file watcher says something
/// changed (`khor_node::wsfacts`); other machines learn it as one reading
/// of a round, only when it changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct WorkspaceFact {
    /// Absolute, on that machine — the workspace table's path.
    pub path: String,
    /// The checkout's git state as a prompt says it (UX-48); `None` for
    /// a plain folder or a status that would not read.
    pub git: Option<GitSegment>,
    /// The branch HEAD is on; `None` for a plain folder, a detached
    /// HEAD, or a repository with no commit yet.
    pub branch: Option<String>,
    /// Files that differ from HEAD, untracked ones included and ignored
    /// ones not (`git status --porcelain`'s rows); `None` for a plain
    /// folder or a status that would not read.
    pub changed: Option<u32>,
    /// `changed` stopped at the status walk's cap rather than being whole.
    pub changed_capped: bool,
    /// When the tree last changed, ms since the epoch: the newest change
    /// the watcher saw, or the newest mtime under it before the watcher
    /// was up. Changes under `.git`, `target`, `node_modules` and the
    /// like do not count (`khor_node::workspace::SKIP`). 0 for a tree
    /// with nothing to date.
    #[ts(type = "number")]
    pub changed_ms: i64,
    /// **A plain folder's most recently modified file** (UX-48: the row's
    /// second line and how long ago it changed); `None` for a checkout,
    /// a folder with no file to name, and from an older resident. Tail
    /// field.
    #[serde(default)]
    pub newest: Option<NewestFile>,
}

/// **The file a plain folder last changed in** (UX-48), looked for at the
/// top level and one directory down — hidden entries and the build and
/// dependency directories left out (`khor_node::workspace::SKIP`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct NewestFile {
    /// Relative to the folder: the file's name at the top level,
    /// `dir/name` one level down.
    pub path: String,
    /// Its modification time, ms since the epoch.
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// **A checkout's git state, as a prompt's git segment says it** (UX-48,
/// the owner: the workspace row's top-right is 「main ⇡861 *1 !1 ?2」 —
/// powerlevel10k's counts), with HEAD's subject and how long all of it
/// has held.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct GitSegment {
    /// The branch HEAD is on; `None` detached or before the first commit.
    pub branch: Option<String>,
    /// HEAD's id, eight hex digits — what a detached HEAD is called;
    /// `None` before the first commit.
    pub head: Option<String>,
    /// HEAD's subject line: the row's second line.
    pub subject: Option<String>,
    /// Commits the upstream lacks and the other way round; 0 with none.
    pub ahead: u32,
    pub behind: u32,
    pub stashes: u32,
    /// Paths conflicted, staged, edited and not staged, untracked.
    pub conflicted: u32,
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    /// The counts stopped at the status walk's cap: each is a floor.
    pub capped: bool,
    /// **When this segment last changed**, ms since the epoch — any of
    /// the fields above but this one; on a resident new to the checkout,
    /// the latest moment git's own files say it could have.
    #[ts(type = "number")]
    pub since_ms: i64,
}

impl GitSegment {
    /// Equal but for the time: the segment did not change.
    pub fn same(&self, other: &GitSegment) -> bool {
        GitSegment { since_ms: 0, ..self.clone() } == GitSegment { since_ms: 0, ..other.clone() }
    }
}

/// **One machine's connection, as the resident holds it**: the road its
/// wire is on, and how long ago that machine was last heard.
///
/// `heard_ms` is liveness, kept on its own axis so the road word never
/// has to carry it: a wire answers keepalives every few seconds, so a
/// healthy machine reads a few seconds at most, and the number grows
/// from the last thing heard once it stops — after the wire is retired
/// too, which is what lets a row say 「12 秒前」 about a machine that
/// went quiet. `None` when it has never been heard since the resident
/// started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub hop: Hop,
    pub heard_ms: Option<u64>,
    /// **The live wire's round trip, whole milliseconds** (round NET-8,
    /// the machine row's latency): QUIC's own smoothed RTT for the path
    /// the wire is on — RFC 6298's estimator, 7/8 of the last value and
    /// 1/8 of the new sample (RFC 9002 §5.3, noq's `RttEstimator`), so it
    /// is already the EWMA a row wants and is not smoothed again here.
    /// `None` without a live wire: no wire, no number.
    #[serde(default)]
    pub rtt_ms: Option<u32>,
    /// **Which relay, when the way is a relay** (round NET-8 addendum,
    /// UX-16's map): the relay's host, and port when it is not the
    /// scheme's own (`khor_net::path::relay_name`). `None` on a direct
    /// wire and without a live one.
    #[serde(default)]
    pub relay: Option<String>,
    /// **How long ago the attempt that failed was, while that machine is
    /// unreachable** (round NET-13: the owner saw rows greyed as
    /// unreachable that opened at a click): the resident's last dial to it failed,
    /// or a request on its wire went unanswered, and nothing has been
    /// heard from it since. `None` otherwise — a live wire, a wire that
    /// closed idle, a machine not dialled since the resident started —
    /// and from an older resident, which never says it: a face reads
    /// `None` as *not known to be unreachable*. Tail field.
    #[serde(default)]
    pub down_ms: Option<u64>,
    /// **The last round trip measured to that machine on any wire**,
    /// whole milliseconds, kept after the wire goes: what a face draws a
    /// machine with no live wire by, at half ink (round NET-13). `None`
    /// when no wire to it was ever measured since the resident started.
    /// Tail field.
    #[serde(default)]
    pub last_rtt_ms: Option<u32>,
    /// **How long since the road [`Link::last_hop`] says settled**, in ms
    /// (UX-48: the machine row's "direct, 3 hours"), live or not: with a
    /// live wire, how long it has been on the road `hop` says (reset when
    /// the road changes or a new wire is made); with none, how long since
    /// the last wire's road settled. `None` when no wire to that machine
    /// ever settled since the resident started. Tail field.
    #[serde(default)]
    pub since_ms: Option<u64>,
    /// **The road the last settled wire was on**, live or gone (UX-48: an
    /// idle machine's row reads the road it last used, not "offline"): `hop`
    /// itself while a wire is live; [`Hop::Direct`] or [`Hop::Relay`]
    /// after it went. `None` when no wire ever settled since the resident
    /// started, and from an older resident. Tail field.
    #[serde(default)]
    pub last_hop: Option<Hop>,
    /// **The middle, when the wire goes through one** (round ROAD-3, the
    /// peer hop): `hop` is then [`Hop::Forwarded`]. `None` otherwise, and
    /// from an older resident. Tail field.
    #[serde(default)]
    pub via: Option<String>,
}

impl Link {
    /// The catalog key of the word for a machine whose
    /// [`Link::down_ms`] is set (`[hop]` `down`): not a road, so not a
    /// [`Hop`], but looked up in the same table as one.
    pub const DOWN_KEY: &'static str = "down";
}

/// **How the resident's wires have fared since it started** — the
/// counters `khor doctor` prints so a word on somebody's screen can be
/// traced to what produced it (round NET-FIX).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCounts {
    /// Wires opened, and dials that did not produce one.
    pub dialed: u64,
    pub dial_failed: u64,
    /// Wires retired, by why: nothing heard for the silence limit; a
    /// request sent and nothing heard back within the ack limit; closed
    /// by the far side or by QUIC; the network changed and the wire
    /// stayed quiet after it; the endpoint itself was replaced.
    pub retired_silent: u64,
    pub retired_unanswered: u64,
    pub retired_closed: u64,
    pub retired_network: u64,
    pub retired_new_ears: u64,
    /// Requests whose wire was alive but whose answer ran past budget.
    pub answers_late: u64,
    /// Readings asked again on a fresh wire after the first one died.
    pub reads_retried: u64,
    /// Road readings handed out, by word.
    pub read_direct: u64,
    pub read_relay: u64,
    pub read_idle: u64,
    /// Network changes and wakes this resident noticed.
    pub network_changes: u64,
    /// Status questions the resident was asked and did not answer in
    /// time (any op past its deadline).
    pub ops_late: u64,
    /// The two failures above, per machine (device id, failed dials,
    /// wires retired unanswered) — only machines with either — so a
    /// count on the doctor's line names whose it is. Tail field.
    #[serde(default)]
    pub by_peer: Vec<(String, u64, u64)>,
    /// UDP bytes the wires this resident dialed have sent and received,
    /// every wire it ever held included (round NET-3: what its rounds
    /// cost the road). Tail fields.
    #[serde(default)]
    pub sent_bytes: u64,
    #[serde(default)]
    pub received_bytes: u64,
    /// Rounds the pump ran (round NET-6: 5 s apart with a face on screen,
    /// 30 s without). Tail field.
    #[serde(default)]
    pub rounds: u64,
    /// Readings handed out while that machine was unreachable
    /// ([`Link::down_ms`], round NET-13); `read_idle` counts only the
    /// idle ones since. Tail field.
    #[serde(default)]
    pub read_down: u64,
}

/// **This machine's connections, as a page shows them** (round NET-2,
/// 更多 → 网络 → 连接): where it is homed, how far each relay is, and
/// per machine the road, the round trip, the last time it was heard and
/// the last thing that went wrong — the doctor's account, in rows a
/// person can screenshot from a phone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionsView {
    /// The relay this machine is homed on — where others find it.
    pub home_relay: Option<String>,
    pub home_connected: bool,
    /// The home relay's last connection error, when it has one.
    pub home_error: Option<String>,
    /// The relay the person asked to home on (首选中继), if any.
    pub preferred_relay: Option<String>,
    pub relays: Vec<RelayReading>,
    pub peers: Vec<PeerConnection>,
}

/// One relay and a measured round trip to it (`None`: no answer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayReading {
    pub url: String,
    pub rtt_ms: Option<u64>,
}

/// One machine's connection, with what went wrong last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerConnection {
    pub id: String,
    pub hop: Hop,
    pub heard_ms: Option<u64>,
    /// The held wire's smoothed round trip, when one is held.
    pub rtt_ms: Option<u64>,
    /// The last failure, in khor's own words, and how long ago.
    pub last_failure: Option<String>,
    pub failure_ago_ms: Option<u64>,
    pub failed: u64,
    pub unanswered: u64,
}

/// **How this machine's recent dials to another one went** (docs/
/// CONNECT.md 度量与验收: 首包时间、直连占比).
///
/// Where [`Hop`] is the road in use *right now*, this is a short
/// history: how long the last dial took to get an answer, and where
/// the last few settled. The two answer different questions — 「is it
/// on a relay now」 and 「does this pair usually punch through」 — and a
/// network that punched through nine times and fell to the relay on
/// the tenth reads as 中继 on one and 9/10 on the other.
///
/// Kept by the resident serve, in memory, from the moment it started:
/// a machine it has not dialled since has no tally, and a restart
/// starts every tally again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadTally {
    /// Milliseconds from starting the most recent dial to the far end's
    /// first answer — the handshake completing, relay head start and
    /// all. `None` when no dial to it has come back since the serve
    /// started.
    pub first_reply_ms: Option<u64>,
    /// Of the last few dials that settled on a road, how many settled
    /// direct…
    pub direct: u32,
    /// …and how many stayed on a relay.
    pub relay: u32,
}

#[cfg(test)]
mod hop_tests {
    use super::Hop;

    /// Keys are what travel; a duplicate would make two readings one.
    #[test]
    fn every_hop_has_its_own_key_and_survives_a_round_trip() {
        let mut seen = std::collections::HashSet::new();
        for h in Hop::ALL {
            assert!(seen.insert(h.key()), "{} is claimed twice", h.key());
            assert_eq!(Hop::try_from(h.key().to_owned()), Ok(h));
        }
        assert!(Hop::try_from("teleport".to_owned()).is_err());
        assert!(
            Hop::try_from("unsure".to_owned()).is_err(),
            "the retired 说不出 does not come back through the wire"
        );
    }
}

impl From<State> for String {
    fn from(state: State) -> String {
        state.key().to_owned()
    }
}

impl TryFrom<String> for State {
    type Error = UnknownState;

    fn try_from(key: String) -> Result<Self, UnknownState> {
        State::ALL
            .into_iter()
            .find(|state| state.key() == key)
            .ok_or(UnknownState(key))
    }
}

/// A seventh word rejected at the wire boundary.
#[derive(Debug)]
pub struct UnknownState(pub String);

impl std::fmt::Display for UnknownState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not one of the six state words: {:?}", self.0)
    }
}

impl std::error::Error for UnknownState {}

/// A word plus when it became true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateStamp {
    pub state: State,
    pub at: Millis,
}

/// What one cell of a session's day strip shows: the last 24 hours of
/// the list, one stretch of time at a time (`khor_node::day`).
///
/// # Two of the three are kinds of nothing, and they are not the same
///
/// The tempting shape is "a word, or blank". That shape is the same
/// trap [`Hop`] took apart one size down: an answer khor cannot give
/// does not become *no value*, it becomes **whichever neighbour it is
/// folded into**. Here the two nothings are:
///
/// - [`DayCell::Unwatched`] — *khor was not looking*. No resident was
///   running on this machine for that stretch (it was off, or khor was
///   not installed yet), so nothing was recorded about anything.
///   Machine-wide: it is the same for every row.
/// - [`DayCell::Absent`] — *khor was looking and this row was not
///   there*. The session had not started yet, or it was closed. Per
///   row, and a real observation rather than a gap.
///
/// Folded into one, "khor was off all night" and "this session started
/// this morning" paint identically — and the second is by far the more
/// common, so the fold would teach people to read every gap as "nothing
/// happened". Folded into 空闲 they would be worse still: that is a
/// word, and the six words are for what a session *was*, never for what
/// khor failed to see (docs/SESSION.md 离线不是第七个词).
///
/// **Neither nothing may be painted in one of the six colours**, for
/// the same reason. Faces draw them as ground and texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayCell {
    Word(State),
    Unwatched,
    Absent,
}

impl DayCell {
    /// Which cell a coarser cell shows when several of them fall into
    /// it. **One order, defined here** — the same argument
    /// [`State::rank`] makes: a strip is drawn at whatever resolution
    /// the screen has, so somebody always has to merge, and a second
    /// enumeration of these three is how a fourth gets invented.
    ///
    /// Words lead, among themselves by [`State::rank`]. That is the
    /// deliberate bias and it is the product's: 待批 is the one word
    /// worth a strip at all, and a merge that let a neighbouring 空闲
    /// swallow it would hide exactly what someone is looking for. The
    /// price is stated rather than hidden — **a cell says "at some
    /// point in here it was 待批", never "it was 待批 for this long"**.
    /// If a strip ever reads as a duration, the fix is a narrower cell,
    /// not a different order.
    ///
    /// [`DayCell::Absent`] is last because it is the strongest claim of
    /// the three: *khor was watching and there was nothing*. A cell may
    /// only make it when every minute in it says so.
    pub const fn rank(self) -> u8 {
        match self {
            DayCell::Word(word) => word.rank(),
            DayCell::Unwatched => 6,
            DayCell::Absent => 7,
        }
    }
}

#[cfg(test)]
mod day_cell_tests {
    use super::{DayCell, State};

    /// **The two nothings are two.** Folded, "khor was off" and "this
    /// row was not there" wear one cell, and the strip stops being able
    /// to say which — see the type's own head for why that is the
    /// expensive direction.
    #[test]
    fn the_two_nothings_stay_apart() {
        assert_ne!(DayCell::Unwatched, DayCell::Absent);
        assert_ne!(DayCell::Unwatched.rank(), DayCell::Absent.rank());
        // And neither may collide with a word: a merge decides by rank,
        // and two things at the same rank merge arbitrarily.
        for word in State::ALL {
            assert_ne!(DayCell::Word(word).rank(), DayCell::Unwatched.rank());
            assert_ne!(DayCell::Word(word).rank(), DayCell::Absent.rank());
        }
    }

    /// **A word that happened is never merged away.** The property, and
    /// it holds for all six — including 失败, which is last among the
    /// words and would be the one a lazy order dropped.
    #[test]
    fn any_word_outranks_either_nothing() {
        for word in State::ALL {
            assert!(
                DayCell::Word(word).rank() < DayCell::Unwatched.rank(),
                "{word:?}"
            );
            assert!(
                DayCell::Word(word).rank() < DayCell::Absent.rank(),
                "{word:?}"
            );
        }
    }

    /// The enumeration beside the property above, because the property
    /// alone is satisfied by implementations that are plainly wrong —
    /// one that ranks all six words the same passes it, and so does one
    /// that swaps the two nothings. This one pins each of the eight.
    #[test]
    fn each_cell_sits_where_it_is_supposed_to() {
        assert_eq!(DayCell::Word(State::Blocked).rank(), 0);
        assert_eq!(DayCell::Word(State::Errored).rank(), 1);
        assert_eq!(DayCell::Word(State::Done).rank(), 2);
        assert_eq!(DayCell::Word(State::Busy).rank(), 3);
        assert_eq!(DayCell::Word(State::Idle).rank(), 4);
        assert_eq!(DayCell::Word(State::Failed).rank(), 5);
        assert_eq!(DayCell::Unwatched.rank(), 6);
        assert_eq!(DayCell::Absent.rank(), 7);
    }

    /// The word half of the order **is** [`State::rank`], not a second
    /// copy of it. A copy is how the two would come to disagree about
    /// where 待批 sits, and that disagreement would be invisible: both
    /// orders are plausible on their own.
    #[test]
    fn the_word_order_is_the_lists_own_order() {
        for word in State::ALL {
            assert_eq!(DayCell::Word(word).rank(), word.rank());
        }
    }
}

/// How full one thing is: bytes in use out of bytes there are.
///
/// Both halves travel, never the percentage alone — "78%" and "78% of
/// 2 TB" are two different sentences, and the one worth reading is the
/// second. Whoever paints it divides; nobody downstream has to guess at
/// a denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Fill {
    /// `number` on the TS side, as everywhere else here: serde_json sends
    /// a number, and 2^53 bytes is eight petabytes.
    #[ts(type = "number")]
    pub used: u64,
    #[ts(type = "number")]
    pub total: u64,
}

/// How close to full a [`Fill`] is, in the two steps worth a colour.
///
/// A judgment, so it lives here and not in a screen: the thresholds are
/// the fact, and two frontends eyeballing `used / total` against numbers
/// of their own would disagree about the same machine.
///
/// **90 and 95, uniform across memory and disk.** Uniform because a pair
/// of numbers someone can hold in their head survives; per-resource
/// tuning is a precision nobody has measured a need for. 95 is where a
/// nearly-full filesystem actually starts failing writes, and 90 is the
/// last round number before it. A threshold set wrong is an alarm that
/// teaches people to ignore alarms — which is why there are two steps
/// and not a gradient.
///
/// **Derived, never on the wire.** [`Vitals`] frames carry the two
/// numbers; anything that wants the word asks this method. Sending the
/// word too would let the two drift apart in a cached reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum Strain {
    /// ≥ 90% full: worth a glance.
    Tight,
    /// ≥ 95% full: worth acting on.
    Critical,
}

impl Fill {
    /// The strain word for this reading, or `None` while there is nothing
    /// to say — including a zero `total`, which is "khor could not read
    /// this", not "an empty resource" (the same judgment as the disk line
    /// printing 读不出 instead of `0 / 0`).
    pub fn strain(&self) -> Option<Strain> {
        if self.total == 0 {
            return None;
        }
        // Integer arithmetic on purpose: `used * 100 >= total * 90` has no
        // float edge where 89.999… rounds its way over a threshold.
        let (used, total) = (self.used as u128, self.total as u128);
        if used * 100 >= total * 95 {
            Some(Strain::Critical)
        } else if used * 100 >= total * 90 {
            Some(Strain::Tight)
        } else {
            None
        }
    }
}

/// What a machine is doing right now: the other half of docs/KHOR.md's
/// core promise, beside the sessions.
///
/// **Asked for, never replicated.** A reading is true for seconds, so
/// putting it in the device CRDT would mean every machine's every sample
/// landing on every disk forever to answer a question whose only
/// interesting answer is the present one. It crosses on its own op and is
/// cached with the moment it was taken, so a machine that cannot be
/// reached shows its last reading and how old it is, never a
/// current-looking number (docs/SESSION.md 离线不是第七个词 — the same
/// two axes, applied to a machine instead of a session).
///
/// **The GPU arrives as an absent-able field rather than an always-present
/// one**, and [`Gpu`] says why. `sysinfo` — the process table khor already
/// carries — has no GPU API at all (the only occurrences of the word in its
/// source are a macOS thermal sensor key and unrelated FFI), so the reading
/// is taken per platform through in-process APIs rather than by shelling
/// out: khor expects nothing installed, `nvidia-smi` included.
/// Not `Copy` since it carries a version string — and it never wanted to
/// be: a reading is a message a machine sent, and messages get moved and
/// cloned deliberately, not duplicated by accident.
///
/// **Five fields have been appended at the tail here — and what that
/// costs an older peer is the whole reading, not the field it has not
/// heard of** (`khor_node::proto`'s head has the mechanism and the
/// question to ask before a sixth). It is acceptable *here* for one
/// reason worth writing down rather than rediscovering: a reading
/// carries the moment it was taken, so a peer that refuses the frame
/// keeps painting the last one it could read **with an age that goes on
/// climbing**. The failure is visible, which is the whole test — on a
/// frame whose consumer had no such axis, the same appends would have
/// been silent lies. The question was asked again for the fourth and
/// fifth (`gpus`, `volumes`, 2026-09-02) and the answer is the same
/// branch: the un-upgraded peer's copy of this machine goes stale in
/// plain sight, never wrong.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Vitals {
    /// Whole-machine CPU use, 0–100.
    ///
    /// A **rate**, not a level: work done between two readings over the
    /// time between them. So it is always "over some window", and which
    /// window is a choice the sampler makes rather than a property of the
    /// machine — `khor_node::vitals` picks one and shows the measurements
    /// behind the pick.
    pub cpu_pct: f32,
    /// How many cores that percentage is spread over.
    pub cores: u32,
    pub mem: Fill,
    /// The filesystem khor's home sits on. `None` when no mount point
    /// contains it: a machine with no answer here paints nothing rather
    /// than `0 / 0`, which reads as an empty disk (docs/SESSION.md
    /// 认不出就不落词 — a landing that cannot be reached becomes its
    /// neighbour unless it is allowed to be absent).
    ///
    /// Wire: at the tail with a serde default, so a khor that predates it
    /// still decodes here (`a_vitals_frame_from_an_older_peer_...`).
    #[serde(default)]
    pub disk: Option<Fill>,
    /// The graphics hardware, when khor can find out — see [`Gpu`].
    ///
    /// Wire: at the tail with a serde default, the same discipline the
    /// disk followed one field earlier.
    #[serde(default)]
    pub gpu: Option<Gpu>,
    /// Which khor that machine is running.
    ///
    /// **It rides with the readings because it is the same kind of
    /// fact**: something a machine says about itself, on the cadence it
    /// already says the others on. The alternative — a verb you type at
    /// one machine — answers about the machine you are standing at,
    /// which is the one you did not need to ask about.
    ///
    /// The question this exists for only appears once machines install
    /// and upgrade themselves: *which of them did not take the last
    /// one*. Before that, `khor` came from a build you were watching.
    ///
    /// `None` is a real answer and not a bug — it names a peer whose
    /// khor predates this field, which is exactly the machine an
    /// upgrade sweep is looking for.
    ///
    /// Wire: at the tail with a serde default, the third field in a row
    /// to follow that discipline.
    #[serde(default)]
    pub version: Option<String>,
    /// The cards one by one — the per-card truth [`Vitals::gpu`]
    /// deliberately flattens (its own doc calls the breakdown "a
    /// different screen"; 2026-09-02 the user asked for that screen:
    /// 「多个GPU，应该分多个环」). The aggregate stays and stays filled,
    /// derived from these when they are known, so older faces keep
    /// their one ring.
    ///
    /// Empty means khor could not read per-card — an older satellite
    /// text, a platform without a reader — never "no cards": that is
    /// still [`Vitals::gpu`]'s `None`.
    ///
    /// Wire: at the tail with a serde default, the fourth field to
    /// follow that discipline. The consumer's error branch, named as
    /// `khor_node::proto`'s head demands: an older peer refuses this
    /// whole reading and keeps painting the last one it could decode,
    /// with `vitals-taken` climbing beside it — visible, not misread.
    #[serde(default)]
    pub gpus: Vec<GpuCard>,
    /// The mounted filesystems worth acting on, one by one — where
    /// [`Vitals::disk`] answers only for the volume khor's home fills
    /// (2026-09-02:「硬盘存储」明细). `disk` stays filled for older
    /// faces. Empty means not surveyed (an older peer, an older
    /// satellite text), never "no disks".
    ///
    /// Wire: tail, serde default, fifth in the row; same named error
    /// branch as `gpus` one field up.
    #[serde(default)]
    pub volumes: Vec<Volume>,
    /// Seconds since this machine booted (`sysinfo::System::uptime`).
    /// Tail-appended with a default, the wire rule: an older node's
    /// frame decodes with 0, which the card prints as nothing rather
    /// than as a machine that booted this instant.
    #[serde(default)]
    #[ts(type = "number")]
    pub uptime_s: u64,
    /// What kind of machine this is (SHELL.md 第五轮 第 1 条: the face
    /// picks the machine's icon from [`Model::kind`]).
    ///
    /// `None` when the machine could not say — never a guessed kind.
    /// Wire: tail, serde default; an older peer's frame decodes as
    /// `None`, which the face draws as the plain machine icon it drew
    /// before this field existed.
    #[serde(default)]
    pub model: Option<Model>,
    /// Which operating system family this machine runs — what a machine
    /// row's corner draws (docs/SHELL.md 第五轮第 7 条改画法: 「角标按系统」).
    ///
    /// `None` when it could not say, and from an older peer: the face
    /// draws the plain machine mark for both, never a guess.
    /// Wire: tail, serde default.
    #[serde(default)]
    pub system: Option<SystemKind>,
}

/// The families a machine row's corner has a mark for. **Closed on
/// purpose**, [`ModelKind`]'s reason: a new family is a new mark, which
/// is a decision.
///
/// **macOS and iOS are two** (round NET-12, UX-27, the owner 2026-10-06:
/// a phone must be known as a phone; the Mac wears the Finder, the
/// iPhone the Apple mark). They were one, `apple`, from 2026-09-22
/// (「所有的macos系统(ios和macos)都用这个」); `Apple` stays as what an
/// older peer still says, and the face draws it as the Apple mark.
/// This build never says it.
///
/// Wire: enum variants travel by name, so an older peer meets `macos`
/// or `ios` as a variant it does not know and refuses that vitals frame
/// whole, as it refuses any longer frame (`khor_node::proto`'s head):
/// its row keeps its last reading until it is updated too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum SystemKind {
    Macos,
    Ios,
    Linux,
    Windows,
    Android,
    /// macOS or iOS, as a khor from before the split said it.
    Apple,
}

#[cfg(test)]
mod system_kind_tests {
    use super::SystemKind;

    /// The wire spelling is the face's match arm, and this build's own
    /// family is what the node reports about itself.
    #[test]
    fn a_system_family_crosses_as_one_kebab_word() {
        assert_eq!(
            serde_json::to_string(&SystemKind::Macos).unwrap(),
            "\"macos\""
        );
        assert_eq!(serde_json::to_string(&SystemKind::Ios).unwrap(), "\"ios\"");
        // What an older peer says still reads, as the family it meant.
        assert_eq!(
            serde_json::from_str::<SystemKind>("\"apple\"").unwrap(),
            SystemKind::Apple
        );
        assert!(SystemKind::Ios.is_handheld() && SystemKind::Android.is_handheld());
        assert!(!SystemKind::Macos.is_handheld() && !SystemKind::Apple.is_handheld());
        assert_eq!(
            serde_json::to_string(&SystemKind::Android).unwrap(),
            "\"android\""
        );
        #[cfg(target_os = "macos")]
        assert_eq!(SystemKind::this_machine(), Some(SystemKind::Macos));
        #[cfg(target_os = "linux")]
        assert_eq!(SystemKind::this_machine(), Some(SystemKind::Linux));
    }
}

impl SystemKind {
    /// The family this binary was built for — which is the family the
    /// machine it runs on belongs to. `None` for anything else.
    pub fn this_machine() -> Option<SystemKind> {
        if cfg!(target_os = "macos") {
            Some(SystemKind::Macos)
        } else if cfg!(target_os = "ios") {
            Some(SystemKind::Ios)
        } else if cfg!(target_os = "android") {
            Some(SystemKind::Android)
        } else if cfg!(target_os = "linux") {
            Some(SystemKind::Linux)
        } else if cfg!(target_os = "windows") {
            Some(SystemKind::Windows)
        } else {
            None
        }
    }

    /// **A phone or a tablet**: what the face asks rather than matching
    /// family names. `false` for an older peer's `apple`, which cannot
    /// say which it was; the device table's `hosts` says so for every
    /// machine whatever its age.
    pub fn is_handheld(self) -> bool {
        matches!(self, SystemKind::Ios | SystemKind::Android)
    }
}

/// **One road a machine reports about itself** (round NET-12, the 我的
/// khor dial): how it reaches `to` right now. Written into the device
/// table by the machine itself (`khor_node::roadbook`), read by every
/// other machine for the pairs it is not part of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Road {
    /// The machine reached, by id.
    pub to: String,
    pub road: RoadKind,
    /// The relay's name when `road` is `relay` (`Link::relay`).
    #[serde(default)]
    pub relay: Option<String>,
    /// Round trip in whole ms, as last written: it moves in the table
    /// only by more than a quarter, at most once in ten minutes.
    #[serde(default)]
    pub ms: Option<u16>,
    /// **The machine this road goes through** (round ROAD-1), when it
    /// does not reach `to` itself: a satellite's sponsor, which runs ssh
    /// for it. `road`, `relay` and `ms` are then the first leg's, to
    /// `via`. `None` for a road of its own. At most one: khor's roads
    /// have at most one forwarding hop.
    #[serde(default)]
    pub via: Option<String>,
    /// **Which leg broke**, when `road` is `none` on a road with legs:
    /// its two ends by device id. `None` on a road of its own, whose one
    /// leg is the road.
    #[serde(default)]
    pub broken: Option<Leg>,
}

impl Road {
    /// A road of its own: no hop, nothing broken.
    pub fn own(to: impl Into<String>, road: RoadKind, relay: Option<String>, ms: Option<u16>) -> Self {
        Road { to: to.into(), road, relay, ms, via: None, broken: None }
    }
}

/// **The leg of a road that broke** (round ROAD-1): which kind of leg,
/// and its two ends by device id where the end is a device — a jump
/// host is not one, so that end is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Leg {
    pub from: Option<String>,
    pub to: Option<String>,
    pub leg: LegKind,
    /// **ssh's own words** for why (round join-e), as the sponsor read
    /// them off its last failed sample (`khor_node::satellite::words`):
    /// one short line, no address prefix, never a key or a secret — the
    /// machine page shows them under its sentence. `None` where ssh said
    /// nothing worth carrying, and on khor's own legs.
    #[serde(default)]
    pub said: Option<String>,
}

/// Which leg, read off the wire or off ssh's own words
/// (`khor_node::satellite::broken`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum LegKind {
    /// This machine to the sponsor: khor's own wire.
    ToSponsor,
    /// The sponsor to the ssh jump host.
    ToJump,
    /// The jump host on to the satellite.
    JumpToTarget,
    /// The sponsor's ssh to the satellite, for a reason ssh's words do
    /// not name.
    Ssh,
    /// The satellite answered and refused the key: it takes a password
    /// only (BatchMode stopped at its door).
    Auth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum RoadKind {
    Direct,
    Relay,
    /// No live wire: unreachable from that machine.
    None,
    /// **A sponsor's road to its satellite** (round ROAD-1): its ssh to it
    /// answered the last sample (docs/NET.md 卫星). An older khor does not
    /// know the word and reads the register as no road.
    Ssh,
}

/// **One machine's try at an address a person gave** (round ROAD-1,
/// `khor reach`): it resolved the name with its own resolver, opened a
/// TCP connection with a short timeout, read the first line the server
/// says, and looked the name up in its own known_hosts. Nothing else —
/// no scan, no keyscan (`khor_node::reach`).
///
/// Crosses the wire (`Response::Probed`) and the hand-off; every field is
/// required on the wire, so a later field goes at the tail with a serde
/// default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ReachSeen {
    pub outcome: ReachOutcome,
    /// The TCP connect, whole ms; only when `reached`.
    #[ts(type = "number | null")]
    pub ms: Option<u32>,
    /// The address its resolver gave and it tried last (`ip:port`).
    pub addr: Option<String>,
    /// The server's first line, printable ASCII, at most
    /// [`REACH_BANNER_MAX`] characters (`SSH-2.0-OpenSSH_9.6`); `None`
    /// when the server said nothing first, which a web server does not.
    pub banner: Option<String>,
    /// The host key this machine's known_hosts holds for the name, as
    /// the SHA256 digest in hex — the same shape as a satellite's id
    /// (docs/NET.md 卫星). `None` when it holds none.
    pub host_key: Option<String>,
    /// Why it was not reached, in the far machine's words.
    pub why: Option<String>,
}

/// The longest banner a probe keeps.
pub const REACH_BANNER_MAX: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum ReachOutcome {
    /// Connected, and the server said an ssh version line.
    Reached,
    /// **Connected, and nothing answered as ssh** (round ROAD-1, the
    /// lead 2026-10-08): no first line, or one that is not `SSH-`. A
    /// fake-ip proxy accepts every connection this way (measured:
    /// `no-such-host.invalid` → 198.18.0.23:22), so a connect alone is
    /// not reached, and a silent row is never the entrance.
    Silent,
    /// The address answered and refused the connection, or the network
    /// said there is no route.
    Refused,
    /// Nothing answered inside the timeout.
    Timeout,
    /// Its resolver has no address for the name.
    NoName,
    /// That machine could not be asked: offline, paused, or the ask failed.
    Unasked,
    /// A khor older than the probe: it refused the question by its name.
    Predates,
}

/// **Every machine's try at one address**, and the pick (round ROAD-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachView {
    pub host: String,
    pub port: u16,
    /// By device id; this machine first.
    pub rows: Vec<(String, ReachSeen)>,
    /// The entrance: the lowest `ms` among the reached, this machine on
    /// a tie ([`ReachView::pick`]); when the keys disagree, only among
    /// the rows whose key is this machine's own.
    pub entrance: Option<String>,
    /// Two machines' known_hosts hold different keys for the name: the
    /// name means a different machine from each — the same
    /// `192.168.1.5` on two LANs.
    pub disagree: bool,
}

impl ReachView {
    /// Fills `entrance` and `disagree` from `rows`; `me` is this
    /// machine's id. Only a row that said ssh's version line is a
    /// candidate. A tie in ms goes to this machine — no hop beats a hop —
    /// and otherwise to the earlier row, so the pick is stable.
    ///
    /// **When the keys disagree** the name means two machines, and the
    /// one meant is the one this machine knows: only rows holding this
    /// machine's own key are candidates, and with no key here there is no
    /// pick — the person chooses.
    pub fn pick(mut self, me: &str) -> Self {
        let mut keys = self.rows.iter().filter_map(|(_, s)| s.host_key.as_deref());
        let first = keys.next();
        self.disagree = first.is_some_and(|k| keys.any(|o| o != k));
        let mine = self.rows.iter().find(|(id, _)| id == me).and_then(|(_, s)| s.host_key.clone());
        let disagree = self.disagree;
        self.entrance = self
            .rows
            .iter()
            .filter(|(_, s)| s.outcome == ReachOutcome::Reached)
            .filter(|(_, s)| !disagree || (mine.is_some() && s.host_key == mine))
            .filter_map(|(id, s)| s.ms.map(|ms| (ms, id != me, id)))
            .min_by_key(|(ms, not_me, _)| (*ms, *not_me))
            .map(|(_, _, id)| id.clone());
        self
    }
}

#[cfg(test)]
mod reach_tests {
    use super::{ReachOutcome, ReachSeen, ReachView};

    fn seen(outcome: ReachOutcome, ms: Option<u32>, key: Option<&str>) -> ReachSeen {
        ReachSeen { outcome, ms, addr: None, banner: None, host_key: key.map(Into::into), why: None }
    }

    fn view(rows: Vec<(&str, ReachSeen)>) -> ReachView {
        ReachView {
            host: "h".into(),
            port: 22,
            rows: rows.into_iter().map(|(id, s)| (id.to_owned(), s)).collect(),
            entrance: None,
            disagree: false,
        }
        .pick("me")
    }

    #[test]
    fn the_entrance_is_the_fastest_reached_and_this_machine_on_a_tie() {
        let v = view(vec![
            ("me", seen(ReachOutcome::Reached, Some(9), None)),
            ("a", seen(ReachOutcome::Reached, Some(3), None)),
            ("b", seen(ReachOutcome::Timeout, None, None)),
        ]);
        assert_eq!(v.entrance.as_deref(), Some("a"));
        let tie = view(vec![
            ("a", seen(ReachOutcome::Reached, Some(3), None)),
            ("me", seen(ReachOutcome::Reached, Some(3), None)),
        ]);
        assert_eq!(tie.entrance.as_deref(), Some("me"), "no hop beats a hop");
        let none = view(vec![("me", seen(ReachOutcome::Refused, None, None))]);
        assert_eq!(none.entrance, None);
        let silent = view(vec![
            ("me", seen(ReachOutcome::Silent, Some(1), None)),
            ("a", seen(ReachOutcome::Reached, Some(40), None)),
        ]);
        assert_eq!(silent.entrance.as_deref(), Some("a"), "a connect with no ssh line is never the entrance");
    }

    #[test]
    fn two_keys_for_one_name_are_two_machines() {
        let same = view(vec![
            ("me", seen(ReachOutcome::Reached, Some(1), Some("aa"))),
            ("a", seen(ReachOutcome::Reached, Some(2), Some("aa"))),
            ("b", seen(ReachOutcome::Reached, Some(2), None)),
        ]);
        assert!(!same.disagree, "one key, and a machine that holds none, agree");
        let two = view(vec![
            ("me", seen(ReachOutcome::Reached, Some(1), Some("aa"))),
            ("a", seen(ReachOutcome::Timeout, None, Some("bb"))),
        ]);
        assert!(two.disagree);
        // Disagreeing: only the rows holding this machine's key.
        let mine = view(vec![
            ("me", seen(ReachOutcome::Timeout, None, Some("aa"))),
            ("a", seen(ReachOutcome::Reached, Some(2), Some("bb"))),
            ("b", seen(ReachOutcome::Reached, Some(5), Some("aa"))),
        ]);
        assert_eq!(mine.entrance.as_deref(), Some("b"));
        let unknown = view(vec![
            ("me", seen(ReachOutcome::Timeout, None, None)),
            ("a", seen(ReachOutcome::Reached, Some(2), Some("bb"))),
            ("b", seen(ReachOutcome::Reached, Some(5), Some("aa"))),
        ]);
        assert_eq!(unknown.entrance, None, "no key here: the person chooses");
    }
}

/// Whether this build's machine can **host** — hold a session's process
/// (a PTY, an agent, a shell), run tmux, be handed a path to open, have
/// VS Code, claude's hooks or a web face of its own (docs/IOS.md).
///
/// **A build-time truth, not a guess.** A phone's app is a sandbox that
/// may not start processes, has no user home and no second window; the
/// build that runs there is the iOS or Android one, and nothing else is.
/// Every other build is a computer and hosts. It is the same line tauri
/// draws between its `desktop` and `mobile` cfgs, drawn here where the
/// node can read it.
///
/// The node says this about itself in the device table
/// (`khor_sync::devices`' `hosts`), so every machine in the network —
/// not only the phone — knows not to offer a session on it. A row that
/// predates the field reads as hosting: every khor before this one ran
/// on a computer.
pub fn hosts_here() -> bool {
    !cfg!(any(target_os = "ios", target_os = "android"))
}

#[cfg(test)]
mod hosts_tests {
    /// The builds the tests run on are computers; a phone build is the
    /// one that answers no (read in the simulator, docs/IOS.md).
    #[test]
    fn a_computer_build_hosts() {
        assert!(super::hosts_here());
    }
}

/// A machine's model: a stable [`ModelKind`] for code to branch on, and
/// the name the machine gives itself for a person to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Model {
    pub kind: ModelKind,
    /// As the machine spells it: `Mac mini (2024)` (Apple Silicon's
    /// device tree), `Macmini8,1` (an Intel Mac's `hw.model`), a DMI
    /// `product_name` such as `SYS-4028GR-TR`. `None` when all the
    /// machine offers is a vendor placeholder (`To Be Filled By O.E.M.`).
    pub name: Option<String>,
}

/// The kinds a face draws a different icon for. **A closed set on
/// purpose**: a face must be able to match it exhaustively, and a new
/// kind is a new icon, which is a decision, not a data update.
///
/// No laptop kind other than [`ModelKind::Macbook`]: no machine in this
/// fleet is one, and a laptop running Linux reads as
/// [`ModelKind::Unknown`] rather than as a desktop it is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKind {
    MacMini,
    Macbook,
    Imac,
    MacStudio,
    Server,
    Desktop,
    Unknown,
}

/// What the graphics hardware is doing.
///
/// **A machine khor cannot ask carries no `Gpu` at all**, which is why
/// [`Vitals::gpu`] is an option rather than a struct full of zeroes. The
/// two read completely differently to whoever is looking: a gauge sitting
/// at 0% is a statement about the hardware, and khor being unable to ask
/// is a statement about khor. Painting the first when the second is true
/// is the same mistake the disk field is shaped to avoid — an invented
/// ring is worse than a missing one — and it is
/// docs/SESSION.md 认不出就不落词 applied to a reading rather than a word.
///
/// **Nothing here is read by running a command.** macOS goes through
/// IOKit's registry and Linux through the vendor's own shared library, so
/// a machine without `nvidia-smi` on its PATH is not a machine khor is
/// blind to — and, the direction that actually bites, a machine that has
/// it is not one where khor spawns a process every five seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Gpu {
    /// Use across all the cards, 0–100.
    ///
    /// The same shape as [`Vitals::cpu_pct`] and averaged the same way, so
    /// a machine with one card pinned and one idle reads 50 — exactly as a
    /// machine with half its cores pinned does. That flattening is
    /// inherited on purpose rather than solved here: a per-card breakdown
    /// is a different screen, and this one answers "how busy is that
    /// machine".
    pub util_pct: f32,
    /// How many cards that percentage is spread over, the counterpart of
    /// [`Vitals::cores`].
    pub cards: u32,
    /// Video memory, **summed** across the cards.
    ///
    /// Summed, unlike the disk, and the difference is not an
    /// inconsistency: a machine has many filesystems serving different
    /// purposes and exactly one of them fills up as khor works, whereas
    /// the cards are one pool of the same thing. That is the reason
    /// [`Vitals::cores`] is a total too.
    ///
    /// `None` where there is no such number to report rather than where it
    /// happens to be zero. Unified-memory machines are the case that
    /// exists today: Apple Silicon's GPU spends the memory already
    /// reported in [`Vitals::mem`], and giving it its own bar would put
    /// the same bytes on the screen twice under two names.
    pub mem: Option<Fill>,
}

/// One graphics card by itself — the row [`Gpu`] averages away.
///
/// A machine with one busy card and three idle ones reads 25% in the
/// aggregate, which is a true sentence about the machine and a false
/// lead about the job: the person deciding where to run looks for the
/// idle card, not the mean. So each card carries its own share.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct GpuCard {
    /// The driver's own name for the card, verbatim. Empty when the
    /// platform reader has no name to give (macOS reads utilisation out
    /// of the IO registry and no name with it) — a face falls back to
    /// its generic word, never to an invented model.
    pub name: String,
    /// This card's use, 0–100. Off-scale figures never get here: a
    /// number that is not a percentage is evidence the wrong thing was
    /// read, and that card is dropped whole (`khor_node::vitals::gpu`'s
    /// clamp scar).
    pub util_pct: f32,
    /// This card's own video memory. `None` on unified memory, where
    /// those bytes are already in [`Vitals::mem`].
    pub mem: Option<Fill>,
    /// The card's own temperature, when the driver reports one (NVML
    /// does; Apple's GPUs do not). Tail-appended, default `None`.
    #[serde(default)]
    pub temp_c: Option<f32>,
}

/// One mounted filesystem a person can act on — a volume of
/// [`Vitals::volumes`].
///
/// Pseudo and system mounts never get in (`khor_node::vitals` owns the
/// predicate); what remains is the short list somebody asking 「还有多少
/// 地方」 actually means. This very machine keeps its build output on an
/// external volume, so the single-disk answer was hiding the one that
/// fills up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Volume {
    /// The mount point, spelled as the machine spells it. Shortening it
    /// for a screen is painting and stays with the painters.
    pub mount: String,
    pub fill: Fill,
}

/// One directory a person asked a machine to keep an eye on, and the
/// volume it sits on — a row of `Response::Watched` (2026-09-06:「用户可能
/// 有自己想关注的磁盘…增删目录,而后更新关注磁盘」). The list of paths is
/// a field on the machine's row in the device table (`watch`); the
/// machine itself answers with these, because only it can say which of
/// its volumes a path is on.
///
/// **Three shapes, kept apart on the wire.** The directory is there and
/// on a volume khor lists (`volume: Some`); the directory is not there
/// (`missing`, and no volume — a typo, or a disk that was unplugged);
/// the directory is there but on a filesystem khor does not list
/// (`missing: false`, `volume: None` — a pseudo mount like `/proc`, or
/// one `khor_node::vitals::keeps_volume` refuses). The third is the one
/// that used to become its neighbour: a path on an unlisted filesystem
/// has the root as its longest listed prefix, and reporting the root's
/// numbers under that path is a reading about the wrong disk.
///
/// **Not a field on [`Vitals`].** A sixth tail-append there would cost
/// an older peer the whole reading (`khor_node::proto`'s head measures
/// why); a new op costs it nothing — it never asks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct WatchedDir {
    /// The path as the person wrote it, trimmed — the row's identity,
    /// and what the sentence on screen names.
    pub path: String,
    /// Nothing at that path on that machine.
    pub missing: bool,
    /// The volume holding it, with the volume's own fill — the same
    /// numbers the volume's own row prints, so the two cannot disagree.
    pub volume: Option<Volume>,
}

/// One row of a machine's process top: what is eating it right now
/// (`khor_node::top` takes the reading; the machine card and `khor top`
/// print it).
///
/// **A per-core rate**, the way `top` prints it: a process holding four
/// cores reads 400, not 40 — capped at 100 the busiest kind of process
/// would read like an ordinary one. Read-only by design: khor answers
/// 这台机器在忙什么, and the verbs that change a machine belong to a
/// terminal a person opened on purpose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct TopRow {
    pub pid: u32,
    /// The process's own short name, as the platform reports it.
    pub name: String,
    /// Whose it is — the account name when the machine can spell it,
    /// the numeric id when it cannot, empty when the platform will not
    /// say. Never invented: a row wearing the asker's name would be a
    /// guess dressed as a fact.
    pub user: String,
    /// CPU over the sampling window, percent of one core.
    pub cpu_pct: f32,
    /// Resident memory, bytes. A `number` on the TS side, the `Fill`
    /// rule: bytes fit a double and a `bigint` cannot be divided.
    #[ts(type = "number")]
    pub mem: u64,
    /// The share of a GPU this process was using over the sampler's
    /// window, percent — summed over the cards it is on, so a process
    /// pinning two cards reads 200 the way `cpu_pct` reads 400 for four
    /// cores.
    ///
    /// **`None` is "this machine cannot say", and it is not zero.** Only
    /// NVIDIA's driver reports per-process utilisation (NVML); macOS
    /// gives per-process nothing, and a card too old for the call gives
    /// nothing either. A zero here would say "this process is not using
    /// the GPU" on a machine that never looked — the neighbouring-value
    /// trap the ledger keeps warning about.
    ///
    /// Tail-appended with a serde default, the wire rule: an older
    /// node's row decodes with `None`, which draws as an empty cell.
    #[serde(default)]
    pub gpu_pct: Option<f32>,
    /// GPU memory this process holds, bytes — summed over cards, and
    /// `None` on the same terms as `gpu_pct`.
    ///
    /// **This is the figure that answers「为什么我的任务起不来」**: a
    /// card can read 0% busy while holding 43 GiB, and only the memory
    /// says there is no room (`vitals::gpu::nvidia` measured exactly
    /// that on turing).
    #[serde(default)]
    #[ts(type = "number | null")]
    pub gpu_mem: Option<u64>,
    /// How long this process has been running, seconds.
    ///
    /// **A length, not a moment** (2026-09-04:「表格多个时间列」). The
    /// question a person asks a remote machine's table is 「这进程是刚
    /// 起的还是挂了三天」, and that is a duration; a start *time* would
    /// have to be read against the far machine's date and timezone, and
    /// against two clocks that do not agree. The far side counts, so
    /// neither clock has to be trusted.
    ///
    /// `None` when the platform will not say — absent, never zero,
    /// because a zero here reads as "started just now".
    ///
    /// Tail-appended with a serde default, the wire rule.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub run_secs: Option<u64>,
    /// **The one word from the command line that says which program
    /// this is** — the first argument after the executable that is not
    /// a flag.
    ///
    /// **Why a second name at all**: `name` is the platform's short
    /// name (on Linux the `comm` out of `/proc/[pid]/stat`), and on the
    /// machine where this table matters most it collapses the answer —
    /// three workers holding 21.7G, 10.9G and 10.3G of card memory all
    /// called `python3`, so 「按显存排,看谁在吃卡」 arrives at three
    /// identical rows.
    ///
    /// **Why one word and not the whole line.** Where the identifying
    /// part sits moves with how the program was called — `-m
    /// vllm.entrypoints.openai.api_server` puts it last, `train.py
    /// --config …` puts it first — so a face that truncates a whole
    /// argv is guessing which end to keep, and guesses wrong half the
    /// time. Picking the word here means the face never has to guess.
    ///
    /// `None` when the machine will not spell it (a kernel thread has
    /// no command line) or when nothing but flags followed the
    /// executable. Absent, never an empty string: an empty one draws as
    /// a name that failed to load rather than as a process that has
    /// none.
    #[serde(default)]
    pub arg: Option<String>,
    /// The process that started this one, when the machine will say.
    ///
    /// **It is here so the face can fold**: ten rows reading
    /// `tokio-rt-worker serve` are ten members of one pool, and a table
    /// that spends ten lines on them is spending its width on one
    /// thing said ten times. Folding needs a key that cannot join two
    /// *different* things, and name-plus-argument is not that key on
    /// its own — two unrelated `python3 main.py` under two different
    /// parents are two answers. The parent is what makes the key mean
    /// 「siblings」.
    ///
    /// `None` where the machine will not say, and **absent is not
    /// zero**: pid 0 is a real parent on every platform khor runs on,
    /// so a row that could not be asked must not claim it.
    ///
    /// Appended at the tail with a `serde default`, like everything
    /// above it — see the round-trip test in `khor_node::proto`.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub ppid: Option<u32>,
    /// **What the machine says this process is doing right now**, as a
    /// stable token the face turns into a word (`zh.toml`
    /// `top-status-*`).
    ///
    /// **A token, not a Chinese word and not an enum.** Chinese belongs
    /// in the catalog. An *enum* would be the [`TopBy`] break in
    /// reverse: this rides **inside** a row, so a variant a peer had
    /// never heard of would refuse the whole frame rather than blank
    /// one cell — a machine's entire table lost to a status nobody has
    /// seen before. A string cannot do that, and a token the face has
    /// no word for is printed as it stands: honest, and diagnosable
    /// from the screen.
    ///
    /// **The platforms disagree about what their own words mean**, so
    /// the mapping lives in one place (`khor_node::top::status_token`)
    /// with the platform as a *parameter* rather than a `cfg`, which is
    /// what lets both branches be tested from either machine. Two of
    /// the disagreements are not guessable from the variant name:
    /// sysinfo's `Idle` is an idle kernel thread on Linux and **a
    /// process being forked** on macOS, and its `Dead` is a really-dead
    /// process on Linux but a thread in **uninterruptible wait** on
    /// macOS.
    ///
    /// `None` means only 「an older node did not send it」. A node that
    /// collects this has a token for **every** row, `unknown-<n>`
    /// included — **nothing falls through to 运行**, because a row
    /// wrongly called running is the neighbouring-value trap landing on
    /// the one column whose whole job is to say something is wrong.
    #[serde(default)]
    pub status: Option<String>,
    /// **CPU-milliseconds this process has burned since it started** —
    /// 累计, next to `cpu_pct`'s 现在.
    ///
    /// The two answer different questions and neither stands in for the
    /// other: a compile at 300% for eight seconds and a daemon at 0.1%
    /// for three weeks read the same on `cpu_pct` if you look at the
    /// wrong moment. This is the column that separates 「刚开始烧」 from
    /// 「烧了一整天」.
    ///
    /// **CPU-milliseconds, not wall milliseconds**: a process on four
    /// cores accumulates four seconds per second, the way `cpu_pct`
    /// reads 400. So it can and does exceed `run_secs` — which is why
    /// the face must not draw the two in the same shape.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub cpu_ms: Option<u64>,
    /// **Bytes this process has read and written since it started**,
    /// the two summed.
    ///
    /// **Since it started, not since the last look.** The deltas
    /// sysinfo also offers are measured over 「however long it was since
    /// whoever asked last asked」 — 5 seconds for this machine's card,
    /// 20 for a far one's, one `MINIMUM_CPU_UPDATE_INTERVAL` for a
    /// one-shot CLI. **A byte count over an unnamed window is not a
    /// fact**; a total from a stated origin is, and the 时长 column
    /// beside it is the divisor for anybody who wants a rate.
    ///
    /// **`None` where khor may not read it** — which is most of a
    /// shared box. `/proc/<pid>/io` is owner-only and macOS's
    /// `proc_pid_rusage` refuses another user's process, and in **both**
    /// cases the library hands back **zero** rather than an error
    /// (measured in sysinfo 0.39's own source: the Linux reader
    /// `return`s on a failed open and leaves the field at 0). A zero
    /// there reads as 「这个进程不碰盘」 on rows that may be hammering
    /// it, so the node decides by ownership *before* it reads
    /// (`khor_node::top::io_readable`): our own processes, or anything
    /// at all when we are root; everything else is absent.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub io_bytes: Option<u64>,
}

/// How much of that one word crosses the wire, characters.
///
/// It is a single argv token, not a line — but a token can be a
/// deeply-nested path, so it still wants a ceiling. Counted in `char`s
/// rather than bytes so a cut cannot land inside a UTF-8 sequence and
/// produce a row that will not decode.
pub const ARG_CAP: usize = 160;

/// Which column a process table is ranked by, and which way.
///
/// **It rides to the machine rather than being applied here.** A table
/// of ten thousand processes (measured on turing, a shared box) sorted
/// on the asking side means ten thousand rows crossing the wire every
/// few seconds to show nine — and the alternative, sorting a cut, ranks
/// the wrong set: the process holding the most memory is rarely among
/// the hungriest by CPU. Ranking where the rows are lets the answer be
/// both true and small.
///
/// `Busiest` is the node's own order (CPU, then memory, then pid) and
/// the default a frame decodes to, so a machine that never heard of
/// this field answers exactly what it always answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ts_rs::TS)]
pub enum TopBy {
    #[default]
    Busiest,
    Pid,
    Name,
    User,
    Cpu,
    Mem,
    Gpu,
    Vram,
    /// How long the process has been up ([`TopRow::run_secs`]).
    ///
    /// **Appended at the end, and the position is not what matters.**
    /// This enum crosses the wire **by variant name**, measured
    /// 2026-09-04 from an older node's own refusal: `unknown variant
    /// `Pid`, expected one of `Busiest`, `Name`, `User`, `Cpu`, `Mem`,
    /// `Gpu`, `Vram``. Two things follow, and they pull in opposite
    /// directions:
    ///
    /// - Inserting in the middle is **safe** — no older peer silently
    ///   reads `Cpu` as its own `Mem` the way an index-keyed encoding
    ///   would. That failure would have been a table sorted by the
    ///   wrong column with every light green.
    /// - Adding a name at all is **a break** — every node that predates
    ///   it refuses the frame outright, and the face has to survive
    ///   that. It does, since a refusal now keeps the column heads and
    ///   says what the far side said (`MachineCard`'s `Busy`,
    ///   `link::load_on_with`).
    ///
    /// So a new column word is a fleet upgrade, not a free addition —
    /// unlike a new *field* on [`TopRow`], which an old node simply
    /// does not send.
    Time,
    /// The word under 状态 ([`TopRow::status`]).
    ///
    /// **This one groups; it does not rank.** There is no order in
    /// which 僵尸 is greater than 休眠 that is not somebody's opinion,
    /// and a made-up severity would be a judgement dressed as a
    /// reading. What a person actually wants from pressing this head is
    /// 「把不正常的那几行凑到一块」, and sorting the token alphabetically
    /// delivers exactly that — the same word lands together, and which
    /// clump is on top is the reader's to flip.
    ///
    /// **The token is what sorts, not the Chinese word**, because the
    /// node does not have the Chinese: it lives in the face's catalog.
    /// So the grouping is stable across languages, and no clump ever
    /// splits in two.
    Status,
    /// Accumulated CPU time ([`TopRow::cpu_ms`]).
    CpuTime,
    /// Bytes read and written ([`TopRow::io_bytes`]).
    ///
    /// **Absent sinks, and here absent is the common case**: on a
    /// shared box khor cannot read another account's figures at all, so
    /// this column ranks the rows it may read and leaves the rest at
    /// the bottom **both ways** — 「读不到」 must never surface as
    /// 「最少的那个」.
    Io,
}

/// The last day of a machine's CPU, five minutes at a stride — what the
/// band under the machine card's CPU ring draws (`khor_node::history`
/// keeps it).
///
/// **A gap is a gap.** A bucket nobody sampled — the machine slept, the
/// serve was down — is `None` and is drawn as absence, never bridged: a
/// line across it would claim the machine was doing something nobody
/// measured (docs/UX.md 不知道就说不知道).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CpuDay {
    /// The stride, so a face never has to know the recorder's constant
    /// by heart. Milliseconds fit a double (the `Fill` rule).
    #[ts(type = "number")]
    pub bucket_ms: u64,
    /// When the newest slot closes (ms since the epoch, the recording
    /// machine's clock) — the axis's right edge.
    #[ts(type = "number")]
    pub now_ms: u64,
    /// Oldest first, one slot per bucket, `None` where nobody measured.
    pub buckets: Vec<Option<f32>>,
    /// Memory in use, percent, on the same buckets — the dashed line
    /// beside the CPU's on the card's day. Same length as `buckets`
    /// once a recorder that knows about it has run; empty from an older
    /// node (tail-appended, default), and the face draws no line then.
    #[serde(default)]
    pub mem: Vec<Option<f32>>,
    /// The same day for the graphics cards, and for the filesystem khor
    /// lives on (2026-09-03:「GPU、CPU、Mem、Disk 都绘制」).
    ///
    /// **Empty, not zero-filled, on a machine that never recorded
    /// them** — the wire rule, so an older node's day decodes to two
    /// series and the chart draws two rows instead of four flat lines
    /// claiming a machine had no GPU all day.
    #[serde(default)]
    pub gpu: Vec<Option<f32>>,
    #[serde(default)]
    pub disk: Vec<Option<f32>>,
    /// The cards' memory over the same buckets, summed across the
    /// machine.
    ///
    /// **Empty on a unified-memory machine, and that is correct rather
    /// than missing.** [`Gpu::mem`] is `None` there because those bytes
    /// are already counted in [`Vitals::mem`] — drawing them again
    /// under a second name would put one machine's memory on the screen
    /// twice. The face draws a row only for a series that has a reading
    /// in it, so an Apple Silicon machine shows four rows and an NVIDIA
    /// one shows five, from the same code.
    ///
    /// Tail-appended with a serde default, the wire rule: a node that
    /// never recorded it sends nothing and this decodes empty.
    #[serde(default)]
    pub vram: Vec<Option<f32>>,
}

/// What a machine is busy with: the top now and the day behind it. One
/// answer for one screen (the machine card asks both together, and two
/// round trips would describe two moments).
///
/// **On the wire as one new op** (`khor_node::proto::Request::Load`),
/// so a khor that predates it refuses the whole ask rather than reading
/// a neighbour — and any field added here later rides the tail with a
/// serde default, the rule every wire struct follows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Load {
    pub top: Vec<TopRow>,
    pub cpu_day: CpuDay,
}

/// Tokens spent, in the four kinds a vendor bills separately.
///
/// **The total is the sum of all four** — new input, output, cache reads
/// and cache writes (owner, 2026-10-07:「我们的总量也要等于新读入、输出、
/// 读缓存、写缓存四项之和」). Wherever a face says 总量, it is that sum
/// and nothing narrower.
///
/// **The four stay separate here all the same, and the sum is not a
/// field.** Measured on this machine 2026-08-17 across every claude
/// transcript on disk: 32.2 million output tokens against 16.2 **billion**
/// cached input, so the total is almost entirely cache reads — it moves
/// with how often a conversation was resumed as much as with how much
/// work was done. That is why a reader still gets the kinds: the total
/// says how much went through, and only the four say what it was.
///
/// ⚠️ **Tombstone (2026-10-07): this comment used to argue against any
/// total**, on the measurement above, and the face's 总量 was input plus
/// output. The owner ruled the other way; the measurement stays because
/// it is still true and still the reason the kinds are kept apart.
///
/// The four are what both vendors bill separately today, spelled so that
/// one name means one thing on every vendor (`khor_node::usage` does the
/// per-vendor arithmetic, and says where the two disagree).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Tokens {
    /// Input the model had to read afresh — **never including what came
    /// out of a cache**. The vendors disagree about this one: claude
    /// reports it already excluding cache reads, codex reports a total
    /// that includes them, so one of the two is a subtraction.
    #[ts(type = "number")]
    pub input: u64,
    /// Input served from cache instead of being read afresh.
    #[ts(type = "number")]
    pub cached_input: u64,
    /// Input written **into** the cache, which every vendor bills apart
    /// from both of the above.
    #[ts(type = "number")]
    pub cache_write: u64,
    /// What the model produced, reasoning included where a vendor
    /// separates the two — it is output either way, and a vendor that
    /// does not separate it has no second number to add.
    #[ts(type = "number")]
    pub output: u64,
}

impl Tokens {
    /// Adds another reading in, saturating. A counter that wrapped would
    /// read as a machine that spent almost nothing.
    pub fn add(&mut self, other: Tokens) {
        self.input = self.input.saturating_add(other.input);
        self.cached_input = self.cached_input.saturating_add(other.cached_input);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
        self.output = self.output.saturating_add(other.output);
    }

    /// Whether anything at all was spent. A day with four zeroes is not a
    /// day worth a row.
    pub fn is_zero(self) -> bool {
        self == Tokens::default()
    }
}

/// One vendor's spend on one day, on one machine.
///
/// The machine is not a field: an answer is always *some machine's*, and
/// the asker knows which one it asked — the same shape [`Vitals`] has, for
/// the same reason (a device id inside the payload is a second copy of a
/// fact the envelope already carries, and two copies can disagree).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct UsageDay {
    /// The civil date, `YYYY-MM-DD`, **in the local time zone of the
    /// machine that spent them**.
    ///
    /// Local rather than UTC because a person's "today" is their own, and
    /// they were sitting at that machine when it happened. The cost is
    /// stated rather than hidden: two machines in different zones cut the
    /// day at two different instants, so a network total for "today" is a
    /// sum of each machine's own today. UTC would have made the boundary
    /// wrong for everybody instead of right for each.
    pub day: String,
    /// Whose tokens these were — the same open set of strings as
    /// [`Session::category`], so `claude` means the same thing in both
    /// places and a third vendor needs nothing added here.
    pub category: String,
    pub tokens: Tokens,
}

/// One model's share of one vendor's spend on one day, on one machine.
///
/// **A second list beside [`UsageDay`], not a field inside it** — the
/// ruling [`ModelDay`] already carries for money, and it lands harder
/// here because of a reason that one does not have: **an older peer**.
/// A `model` field on `UsageDay` would make every row of that list a
/// vendor-day *total* when the answer came from a khor that predates
/// this, and a (vendor, model) row when it came from one that does not
/// — one list meaning two things, **with nothing in the frame saying
/// which**. So the day axis stays one shape for everybody and the
/// breakdown is a layer that only some answers have.
///
/// # The two properties a reader may rely on
///
/// **This list is exhaustive.** Every token in [`Usage::days`] appears
/// in exactly one row here, so the rows for a (day, category) sum to
/// that day's row exactly. It is not a gate's promise but the shape of
/// the code: `khor_node::usage::Meters::tally_in` folds *one* map keyed
/// by (day, vendor, model) and projects the day list out of it, so
/// there is no second pass that could disagree. Whoever draws
/// percentages from this can trust the denominator; a breakdown that
/// carried only the *named* part would hand them a 100% that quietly
/// left tokens out.
///
/// **Empty here beside a non-empty [`Usage::days`] means the answering
/// machine cannot tell** — a khor from before this existed — and it is
/// distinguishable from "spent nothing", which empties both. A face
/// that painted the first as an empty chart would be stating a fact
/// nobody established.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct UsageModelDay {
    /// The civil date, as [`UsageDay::day`] cuts it and in the same
    /// zone.
    pub day: String,
    /// Whose tokens these were, as [`UsageDay::category`] spells it.
    pub category: String,
    /// The model's name **as the vendor's own file spelled it**, and
    /// `None` when that file named no model.
    ///
    /// **`None` is not "khor does not recognise it"** — a record khor
    /// cannot read is counted in [`Usage::unreadable`] and never
    /// reaches here. It is the narrower and more honest sentence: the
    /// vendor wrote the tokens down without writing which model spent
    /// them. Four of the thirteen vendor names khor meters have no
    /// model in the samples khor has at all; the rest can still produce
    /// an unnamed record one at a time.
    ///
    /// **`None` rather than `""`** so that a face cannot paint it by
    /// accident: an empty string renders as an empty label and reads as
    /// a model whose name is missing, where null forces whoever draws
    /// it to reach for the word (`cli.usage-model-unnamed`). A default
    /// painted as fact is the failure this crate keeps being written
    /// against.
    pub model: Option<String>,
    pub tokens: Tokens,
}

/// What one machine has spent, by day and by vendor.
///
/// **Asked for, never replicated**, the same call [`Vitals`] makes and for
/// a related reason: this is derived from files the vendors own and khor
/// only reads, so it is re-derivable at any time on the machine that has
/// those files and meaningless to copy anywhere else.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Usage {
    /// Sorted, oldest day first, and one row per (day, vendor). A day on
    /// which nothing was spent has no row rather than a row of zeroes.
    pub days: Vec<UsageDay>,
    /// Records khor found but could not read: a line in a vendor's file
    /// whose shape it does not understand. **Never silently zero** — the
    /// same "适配器过时" signal [`Session`] discovery carries as
    /// `unreadable_sessions`, and the honest alternative to a total that
    /// quietly omits what it could not parse.
    #[ts(type = "number")]
    pub unreadable: u64,
    /// The same spending cut one axis finer. Sorted the same way, then
    /// by model with the unnamed row last. See [`UsageModelDay`] for
    /// why this is a second list and for the two properties it holds.
    ///
    /// Tail field, defaulted: an older peer's answer has none and says
    /// so by being empty.
    #[serde(default)]
    pub models: Vec<UsageModelDay>,
    /// The civil date **on the machine that answered**, cut in its own
    /// zone at the moment it worked this out.
    ///
    /// **Not derivable from anything else here, which is the whole
    /// reason it is a field.** [`UsageDay::day`] is in the answering
    /// machine's zone and nothing in this frame says which zone that
    /// is; a face that used its *own* today to decide "up to and
    /// including today" would be right about the machine it runs on and
    /// off by a day about every other one. `khor_node::usage`'s window
    /// carries the same ruling for the same reason — a face that worked
    /// out today its own way eventually disagrees with the rows it is
    /// filtering, and the disagreement only shows up around midnight.
    ///
    /// `None` from a khor that predates it. That is 「it did not say」,
    /// and a face falling back to its own today should know that is
    /// what it is doing rather than believe it was told.
    #[serde(default)]
    pub today: Option<String>,
}

/// **How long** one subscription window is — and nothing else.
///
/// ⚠️ **This used to be `QuotaWindowKind`, and it answered two questions
/// at once**: how long the window is *and* what it is limited to.
/// `seven_day_opus` fused them, and that fusion is what dropped Fable's
/// window on 2026-09-06 — a per-model weekly window the endpoint had
/// started reporting and this enum had no variant for. The endpoint's
/// own `limits[]` array had already split the two apart (`group` for the
/// length, `scope.model` for the limit), so khor follows it: length
/// here, limit in [`QuotaWindow::scope`].
///
/// **A key, not a label**, for the two khor can order. The words live in
/// `crates/catalog` and are looked up where they are painted, the same
/// rule the six state words follow — a Rust file that spelled the
/// window's name would be the one place that wording could not be
/// changed from.
///
/// **`Other` is the length khor cannot order**, carrying the endpoint's
/// own group word. It exists because the old parser's posture — drop a
/// window you have never heard of — was resting on a premise `limits[]`
/// removed: back then an unknown window was a bare percentage with no
/// idea what it was a percentage *of*, and dropping it was right. Now
/// every entry says its own percent, reset and group, so dropping one
/// would be throwing away the one thing that makes the new shape worth
/// reading: that the endpoint will grow limits khor has never seen.
///
/// **What `Other` does not buy**: a place on the ring. The ring's layers
/// are ordered by length and an unknown length cannot be put in that
/// order, so these are detail-only — which is exactly what
/// `typeof span === "string"` says on the TS side, one condition rather
/// than a table of exceptions. And its word is the endpoint's, so a
/// group word that is not human ("juniper_tide") shows up as itself.
/// **That is the endpoint's word, not a khor sentence** — the same
/// bargain `providerWord`'s and `troubleWord`'s default branches make.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum QuotaWindowSpan {
    FiveHour,
    SevenDay,
    Other(String),
}

/// How full one subscription window is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct QuotaWindow {
    /// How long, and therefore where it sits on the ring.
    pub span: QuotaWindowSpan,
    /// What it is limited to. `None` is the whole plan for that length;
    /// `Some(name)` is one model's share of it, **named by the endpoint
    /// and never by khor** — the display name out of `limits[]`
    /// (「Fable」), or, on the old key path that has no `limits[]`, the
    /// model the endpoint's own key names (`seven_day_opus` → `Opus`).
    ///
    /// **Together with `span` this is the window's identity**, which is
    /// what lets the two sources be merged without a name comparison:
    /// the top-level `seven_day` and `limits[]`'s `weekly_all` are the
    /// same `(SevenDay, None)` and come out as one row.
    pub scope: Option<String>,
    /// 0–100, clamped at the parser. A number outside that range is a
    /// misread rather than a fuller-than-full window.
    pub used_pct: f32,
    /// When this window rolls over, unix seconds. `None` once the moment
    /// has passed — a reset time in the past is not a fact about the
    /// future, and the reading beside it has already been zeroed.
    ///
    /// A `number` on the TS side, as elsewhere here: unix seconds are
    /// nowhere near 2^53, and a `bigint` would make every screen that
    /// does arithmetic on it convert first.
    #[ts(type = "number | null")]
    pub resets_at: Option<i64>,
    /// The endpoint's own two extra facts about this limit, carried and
    /// **not painted yet** (`is_active` / `severity` in `limits[]`).
    ///
    /// They are here rather than dropped because they are facts the
    /// endpoint stated, and a parser that discards them makes them
    /// unavailable to the question they would answer. They are not drawn
    /// because neither has a decided meaning on this screen yet:
    /// `is_active` was `false` on the session window that was 29% full
    /// and `true` on the weekly — so it is not「the one that matters」—
    /// and `severity` was `normal` on all three, which is a scale nobody
    /// has seen the other end of. Painting either would be this layer
    /// deciding a state, which is the one thing it must not do.
    ///
    /// `None` on the old key path: those keys say neither.
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub severity: Option<String>,
}

/// The prepaid credits that carry on after the plan's windows are full
/// — Anthropic's「usage credits」, billed at API prices out of a
/// balance the person topped up.
///
/// **Not a window, and it must not be drawn as one.** A window is a
/// fraction of a period that refills by itself; this is money, it does
/// not refill, and it has no reset. Putting it on the ring or into the
/// row's summary would make「how full am I」answer with two different
/// kinds of thing — so it lives in the detail and nowhere else.
///
/// **Every figure is in the currency's major unit**, converted at the
/// parser out of the minor units the endpoint reports with their own
/// exponent. The two blocks it reports (`extra_usage` and `spend`) are
/// the same money spelled twice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct QuotaCredits {
    /// Whether spending past the plan is switched on right now.
    pub enabled: bool,
    /// Why not, as the endpoint's own key (`out_of_credits`). A reason
    /// this build has never heard of is shown as itself, the same
    /// bargain [`QuotaTrouble`]'s words make — the person can act on a
    /// key they can read, and cannot act on one khor swallowed.
    pub disabled_reason: Option<String>,
    /// Spent so far, and what is left. `None` where the endpoint said
    /// nothing rather than said zero: a balance nobody reported is not
    /// an empty wallet, and drawing it as 0 would be this layer
    /// inventing the one number a person would act on.
    pub used: Option<f64>,
    pub balance: Option<f64>,
    pub monthly_limit: Option<f64>,
    pub currency: Option<String>,
    /// Whether credits were ever switched on. Distinguishes「off, and
    /// you have used this before」from「off, and you have never had
    /// it」, which are different sentences to a reader.
    pub ever_enabled: bool,
}

/// What this machine's Claude subscription has left.
///
/// **Read through the user's own Claude Code login, and the interface has
/// to say so.** The credential was stored so that `claude` could run;
/// using it to answer a second question widens what it was given for,
/// and the user agreed to that widening knowing it (ruling 2026-08-21).
/// What they agreed to is not a secret khor keeps — a number appearing
/// with no account behind it reads as khor's own knowledge, which it is
/// not, so whichever screen paints this says whose login it came
/// through.
///
/// **Never replicated.** Like [`Vitals`] this is asked for and cached
/// with the moment it was taken; unlike Vitals it is not even a fact
/// about the machine — it is a fact about an *account*, and the same
/// account seen from two machines is one quota, not two.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Quota {
    /// Windows the endpoint reported, **ordered by khor and not by the
    /// endpoint**: shortest length first, and within a length the whole
    /// plan before any one model's share of it. The endpoint lists them
    /// in whatever order it likes and two sources are merged here, so an
    /// order that came out of either one would be a different list on
    /// two machines reading the same account. Empty is a real answer —
    /// an account the endpoint reports no window limits for is not an
    /// error.
    pub windows: Vec<QuotaWindow>,
    /// When this reading was taken, unix seconds. `None` means "just
    /// now, this call". **Not decoration**: a served cache entry and a
    /// fresh fetch look identical otherwise, and 18% from ten minutes
    /// ago is a different sentence from 18% now — the same axis
    /// [`VitalsReading`] carries, applied to an account.
    #[ts(type = "number | null")]
    pub as_of: Option<i64>,
    /// The plan this login is on — 「Max 20x」, 「Pro」 — as the stored
    /// credential states it, **not asked for over the network**.
    ///
    /// `claudeAiOauth.rateLimitTier` already sits in the file khor
    /// reads for the access token, so this costs no request. The
    /// endpoint's `/api/oauth/profile` answers it too, and is
    /// deliberately not asked: it is another non-public endpoint and it
    /// carries personal profile besides, for a fact already on disk.
    ///
    /// The slug is de-slugged, never invented: `default_claude_max_20x`
    /// becomes 「Max 20x」, and a slug that does not fit that shape is
    /// shown as itself rather than guessed at.
    ///
    /// **Read fresh on every [`Quota`], including cache hits**, because
    /// it is a fact about the login rather than about the reading — a
    /// person who changes plan should not have to wait out a cache.
    #[serde(default)]
    pub plan: Option<String>,
    /// The prepaid credits, where the endpoint reported them. `None`
    /// from the statusline file, which says nothing about money.
    #[serde(default)]
    pub credits: Option<QuotaCredits>,
    /// How full this subscription is **as a whole**, `0..=1` — the
    /// share an arc draws and the number in its middle. `None` when no
    /// window speaks for the whole plan.
    ///
    /// **Which window that is, is decided in one place**
    /// (`khor_node::quota::whole_plan_fill`) and this field is its
    /// answer. It is the same doctrine [`AccountReading::fill`] states
    /// — the fraction is the node's, never the screen's — arriving late
    /// because a subscription had no [`AccountReading`] until
    /// 2026-09-07.
    ///
    /// ⚠️ **Tombstone: this rule used to live in the face**, as
    /// `QuotaLanding.tsx`'s `headline()`. That was survivable while a
    /// subscription was one probed row; it stopped being survivable the
    /// day a subscription could also be a *registered account*, because
    /// then the same picture had two sources — the probe's windows
    /// through `headline()`, and the registration's reading through
    /// `AccountReading::fill` — and 「the two rows look identical」
    /// became a property somebody had to keep true. **It is now a
    /// property nothing can break: both rows read this one number.**
    ///
    /// **Recomputed on every read, including cache hits**, the way
    /// `plan` is: it is a function of `windows`, so computing it beside
    /// them is what makes it impossible for the two to disagree — and
    /// a cache entry written by a khor that predates this field gets it
    /// for free instead of decoding as 「no fraction」.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub fill: Option<f32>,
    /// Which strategy in the vendor's chain answered (`statusline`,
    /// `oauth-usage`, `rollout`, `wham-usage` — `khor_node::quota`),
    /// kept with the reading so a cached one still says where it came
    /// from. `None` from a cache written before the chain existed.
    #[serde(default)]
    pub source: Option<String>,
    /// Whether the vendor says the limit has been reached, where it
    /// says so at all (Codex's `rate_limit.limit_reached`). `None` is
    /// 「not said」, never 「not reached」.
    #[serde(default)]
    pub limit_reached: Option<bool>,
}

/// Why there is no quota reading — as a cause, never as a sentence.
///
/// The words live in `crates/catalog` like every other user-facing
/// string, and they are looked up where they are painted. A Rust file
/// holding them would be the one place that wording could not be
/// changed from, and this particular wording matters: three of these
/// five are things the person can act on, and telling them *which* is
/// the entire value of saying anything at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum QuotaTrouble {
    /// No Claude Code login on this machine.
    NoLogin,
    /// The stored credential is expired or was refused.
    ///
    /// **khor does not refresh it**, which is a decision and not a gap:
    /// minting a token against this endpoint would be khor acting as
    /// Claude Code rather than reading what Claude Code left behind. The
    /// person runs `claude` once and it is fixed.
    Stale,
    /// Rate limited. Whole minutes until khor will try again — said out
    /// loud, because a panel that just stays empty during a ten-minute
    /// cooldown is indistinguishable from one that is broken.
    Cooling {
        #[ts(type = "number")]
        minutes: i64,
    },
    /// The endpoint answered in a shape this build cannot read.
    Unreadable,
    /// The endpoint could not be reached.
    Unreachable,
}

/// One registered API account's usage, as the machine holding its key
/// last read it (`khor_node::accounts`). What crosses the mesh in
/// [`Request::Quotas`]'s answer and what the 额度 page draws a ring from.
///
/// **The fraction is decided here, not on the screen.** A provider that
/// has a hard limit (a new-api token's remaining quota) fills against
/// it; one that bills per token (the two vendors) fills against the
/// budget the person registered — and with no budget there is no
/// fraction, only numbers, and the ring draws its track (`fill: None`).
/// A face that computed the share itself would be a second opinion
/// about what "full" means, and the two would drift.
/// One bucket of a provider's own cost report: what it says was spent
/// over a stretch, and when that stretch began.
///
/// **The instant is the provider's, not a calendar day khor worked
/// out.** Both vendors answer buckets with their own start
/// (Anthropic's `starting_at`, OpenAI's `start_time`), asked for at a
/// day's width — so carrying the bucket's start says exactly what was
/// reported and nothing more. A screen east or west of the reporting
/// zone would cut its days elsewhere; this way the disagreement is
/// visible instead of baked in
/// ([`UsageDay`] carries the same ruling for khor's own days).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct DaySpend {
    /// When this bucket starts, unix seconds, as the provider stated it.
    #[ts(type = "number")]
    pub at: i64,
    /// What it says was spent in it, in the reading's `currency`.
    pub spent: f64,
}

/// What one model cost on one day, where a provider itemises that far.
///
/// **A second list beside [`DaySpend`], not a field inside it.** Every
/// provider that reports anything reports a day total; only new-api
/// itemises the models under it. Folding the model into `DaySpend`
/// would make one list mean two things depending on who filled it —
/// the drift this crate keeps being written against — so the day axis
/// stays one shape for everybody and the breakdown is a layer that only
/// some readings have.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct ModelDay {
    /// The day this belongs to, unix seconds at its start.
    #[ts(type = "number")]
    pub at: i64,
    /// The model's name **as the provider spelled it**.
    pub model: String,
    pub spent: f64,
}

/// What one model spent on one day **in tokens**, as the provider's own
/// account of it.
///
/// # Why this is a third list and not two more fields on [`ModelDay`]
///
/// **Tokens and money are two quantities, and one row holding both
/// would be a row that is half-filled for everybody.** They arrive from
/// the same endpoint, on the same day, about the same model — and they
/// still cannot share a row: new-api reports both, the two vendors'
/// cost report and usage report are **two different endpoints** either
/// of which can fail on its own, and the wallets report neither. A
/// merged row would need a rule for "money but no tokens", and the
/// only honest spelling of that rule is a second list that is simply
/// absent. Same ruling as [`ModelDay`] beside [`DaySpend`], one level
/// further out.
///
/// # These are not khor's own token counts, and must never be compared
/// with them
///
/// `khor_node::usage` counts tokens too, off the agents' transcripts,
/// and the two numbers **are supposed to disagree**: a provider bills
/// per request through one key over its own window, while a transcript
/// records what an agent wrote down on this machine — a different
/// population (other machines, other keys, subagents) cut on a
/// different clock. A gate asserting they match would be a gate written
/// to go red, and nobody reading that red would know which side to
/// change. They answer two questions and belong on screen as two
/// answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ModelTokenDay {
    /// The day this belongs to, unix seconds at its start — the same
    /// axis [`DaySpend::at`] uses, so the two lists line up.
    #[ts(type = "number")]
    pub at: i64,
    /// The model's name **as the provider spelled it**, and `None`
    /// where the provider named none.
    ///
    /// **`None` is a row, not a silence** — the tokens are real and are
    /// inside the day's total, so dropping them would shrink a
    /// denominator without shrinking anything visible. The word for it
    /// is `cli.usage-model-unnamed`, the same one
    /// [`UsageModelDay::model`] reaches for, because it is the same
    /// sentence: the source did not record which model.
    ///
    /// It happens two ways here and they are worth telling apart when
    /// reading a report: a record whose own `model` was blank, and a
    /// whole answer where the provider did not honour the grouping ask
    /// — which arrives as **one unnamed row per day** carrying that
    /// day's whole total. Both are 「nobody said which model」 and
    /// neither is a wrong number.
    pub model: Option<String>,
    pub tokens: Tokens,
    /// Whether `tokens.cached_input` and `tokens.cache_write` are
    /// **figures the provider stated**.
    ///
    /// **`false` means those two are not zero, they are not reported**
    /// — and the difference matters on a screen, where a painted 0 is a
    /// claim that nothing was cached. new-api's log carries
    /// `prompt_tokens` and `completion_tokens` and nothing else, so its
    /// rows say `false`; the two vendors' usage reports itemise the
    /// cache and say `true`.
    ///
    /// A `bool` rather than four `Option`s because the split is a
    /// property of the *source*, not of a row: no provider here reports
    /// the cache for some of its own rows and not others.
    pub cache_split: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct AccountReading {
    /// `khor_sync::accounts::Account::id`.
    pub account: String,
    pub name: String,
    /// `newapi` / `claude` / `openai`, the registered word.
    pub provider: String,
    /// The device (hex id) whose key was read.
    pub home: String,
    /// Used over full, `0..=1`; `None` when there is no denominator.
    #[ts(type = "number | null")]
    pub fill: Option<f32>,
    /// Spent so far in the window, in `currency`; `None` when the
    /// provider does not say.
    #[ts(type = "number | null")]
    pub spent: Option<f64>,
    /// What full is, in `currency`: the provider's own limit, or the
    /// registered budget. `None` with no denominator.
    #[ts(type = "number | null")]
    pub limit: Option<f64>,
    #[serde(default)]
    pub currency: Option<String>,
    /// The provider said this account has no ceiling at all.
    #[serde(default)]
    pub unlimited: bool,
    /// When the window resets, unix seconds, where the provider says.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub resets_at: Option<i64>,
    /// Why there is no number, as a **key** (`account-trouble-*` in the
    /// catalog): the key was refused, the address unreachable, the
    /// answer unreadable, cooling after a failure. `None` when the
    /// numbers are real.
    #[serde(default)]
    pub trouble: Option<String>,
    /// When this reading was taken, ms since the epoch, by the reading
    /// machine's clock.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// What is left, in `currency`, where the provider reports a balance
    /// rather than a spend: a prepaid wallet (DeepSeek, Moonshot), a
    /// capped OpenRouter key. Never derived from `spent` and `limit`
    /// here — a screen can subtract; what it cannot know is which kind
    /// of number the provider gave, and a balance written into `spent`
    /// would read as the opposite of the truth. Tail field, defaulted:
    /// an older reader drops it and keeps the rest.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub remaining: Option<f64>,
    /// When this key stops working, unix **seconds**. `None` is 「never,
    /// or the provider did not say」 — and those two are one answer on
    /// purpose: neither puts a date on a screen.
    ///
    /// ⚠️ **The sentinel is resolved before this field, and it has two
    /// spellings.** (Every `file.go:NN` in this struct is new-api at
    /// the revision `khor_node::accounts`'s head pins — a line number
    /// with no revision behind it points confidently at the wrong
    /// line the moment that branch moves.) A new-api token stores `-1` for 「never expired」
    /// (`model/token.go:22`) while the endpoint rewrites that to `0`
    /// before answering (`controller/token.go:256-259`) — **two
    /// different numbers for one meaning, with a rewrite between them**.
    /// Either would draw as 1970-01-01 if it reached a face: not an
    /// error, not a blank, a **date that looks entirely real**. So the
    /// parser turns anything `<= 0` into `None` and nothing downstream
    /// has to know the story.
    ///
    /// Tail field, defaulted: an older reader drops it and keeps the rest.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expires_at: Option<i64>,
    /// The models this key may use, where the provider limits it.
    /// **Empty means no limit is in force** — not 「limited to nothing」.
    ///
    /// ⚠️ **A new-api token keeps its old list after the limit is
    /// switched off.** `DisableModelLimits` sets `model_limits_enabled`
    /// to false and **does not clear** `model_limits`
    /// (`model/token.go:354-360`), so the endpoint answers a full list
    /// beside a false flag. Reading the list without the flag draws a
    /// row of model names that stopped meaning anything — and 「this key
    /// is limited to these three」 versus 「this key is not limited」 are
    /// opposite sentences, both made of real model names, with nothing
    /// on screen or in any gate to tell them apart. The parser applies
    /// the flag; this field is already the answer.
    #[serde(default)]
    pub models: Vec<String>,
    /// This month so far, one entry per day the provider reported, in
    /// the order it reported them. Empty where a provider has no cost
    /// report at all.
    ///
    /// **The numbers were already being fetched and thrown away.** Both
    /// vendors are asked for `bucket_width=1d` over the month
    /// (`fetch_claude` / `fetch_openai`), and until 2026-09-07 the
    /// reader summed the buckets into `spent` and dropped the shape —
    /// so a page could say 「12.34 spent」 and could not say 「and nine
    /// of it was Tuesday」, from data already on the wire.
    ///
    /// **Not derived from anything.** `spent` stays the provider's own
    /// total rather than a sum of these, because a page must not have
    /// two answers to one question: if a bucket is missing, the total is
    /// still right and the chart is visibly short, which is the honest
    /// pair.
    ///
    /// Tail field, defaulted: an older reader drops it and keeps the rest.
    #[serde(default)]
    pub days: Vec<DaySpend>,
    /// This month's spend and the key's lifetime spend, in `currency`,
    /// **where the provider states them as two separate figures**.
    ///
    /// OpenRouter does (`usage_monthly` and `usage` on `/api/v1/key`),
    /// and until 2026-09-07 only one of them survived: whichever the
    /// fraction happened to need went into `spent` and the other was
    /// dropped. **They are two facts about two periods** — a page that
    /// showed one of them under a bare 「已用」 was answering 「how much
    /// have I spent」 with a number whose period depended on whether the
    /// key had a cap.
    ///
    /// `None` where the provider reports no such figure. `spent` is
    /// untouched and still means 「what the fraction is made of」.
    ///
    /// Tail fields, defaulted: an older reader drops them.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub spent_this_month: Option<f64>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub spent_all_time: Option<f64>,
    /// The same window as `days`, itemised by model, where the provider
    /// itemises it. **Its day totals must equal `days`** — they come out
    /// of one pass over one answer, and a face that drew a stack taller
    /// than its own column would be showing two readings of one number.
    #[serde(default)]
    pub model_days: Vec<ModelDay>,
    /// `Some(n)` when the provider's log **was cut at `n` records** and
    /// what is above is therefore only part of the window asked for.
    ///
    /// ⚠️ **This is the difference between a short chart and a wrong
    /// one.** new-api's `/api/log/token` answers the most recent
    /// `MaxRecentItems` records — 1000, `common/constants.go:60` — in
    /// `id desc`, with **no time window and no paging**
    /// (`model/log.go:140-148`). A busy key can spend that in a day, and
    /// then a chart labelled 「7 days」 holds half of one. It does not
    /// error and it does not look empty: **it draws a true picture of
    /// the wrong stretch**, which is the shape this repo refuses.
    ///
    /// So the count travels, and whoever draws it must say what it
    /// covers. `None` is 「not cut」 — not 「unknown」: a provider with no
    /// log at all has no chart to mislabel.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub log_capped: Option<u32>,
    /// ── What a **subscription** account reads as ──────────────
    ///
    /// The three fields below are empty for every account with a key
    /// behind it and filled only for a registered subscription
    /// (`khor_sync::accounts::AccountKind::Sub`), whose reading is a
    /// [`Quota`] rather than a spend: windows that refill, the plan the
    /// login is on, and the prepaid credits beside them. `fill` above
    /// carries the whole-plan share for it, by the same rule and from
    /// the same function as [`Quota::fill`] — so a subscription needs
    /// no field of its own for the one number the ring draws.
    ///
    /// **Tail fields, `serde default`, no `skip_serializing_if`** —
    /// `khor_node::proto`'s rule, and that module also demands the
    /// sentence be said out loud before appending: *when an older peer
    /// refuses this whole frame, is what it then shows any different
    /// from what it shows on success?*
    ///
    /// **It is different.** The consumer is the `Response::Quotas` arm
    /// of `Node::visit` (`link.rs`), which caches a peer's readings only
    /// on `Ok`. An older khor that cannot decode this longer frame
    /// keeps the answer it carried home last time, **with an age that
    /// goes on climbing** on every row of its 额度 page, and shows
    /// 「那台机器还没答过」 for an account it never heard from. Both
    /// land where the page already knows how to say 「I cannot tell」,
    /// so a tail default is the right one of the three ways — not a new
    /// variant, and not a silent default painted as fact.
    #[serde(default)]
    pub windows: Vec<QuotaWindow>,
    /// The plan the subscription's login is on — 「Max 20x」, 「Pro」.
    /// A product name from the stored credential, never a khor word
    /// ([`Quota::plan`]).
    #[serde(default)]
    pub plan: Option<String>,
    /// The subscription's prepaid credits ([`Quota::credits`]).
    #[serde(default)]
    pub credits: Option<QuotaCredits>,
    /// The same window again, in **tokens** rather than money, itemised
    /// by model — where the provider has a token report at all. See
    /// [`ModelTokenDay`] for why this is a list of its own and for the
    /// one comparison nobody may make with it.
    ///
    /// **Empty is 「this provider does not report tokens」**, and it is
    /// the ordinary answer: the wallets (DeepSeek, Moonshot,
    /// OpenRouter) publish a balance and nothing else, and a registered
    /// subscription's endpoint publishes percentages. Empty beside a
    /// non-empty `days` therefore does **not** mean the reading is
    /// short — money and tokens come from two roads and only three
    /// providers walk the second one.
    ///
    /// **Tail field, `serde default`, no `skip_serializing_if`** — and
    /// the sentence `khor_node::proto` demands before appending: an
    /// older peer refusing this longer frame keeps its previous
    /// readings with an age that goes on climbing, which is different
    /// from what it shows on success and lands where the 額度 page
    /// already says 「I cannot tell」. The same answer the fields above
    /// gave, for the same consumer.
    #[serde(default)]
    pub model_tokens: Vec<ModelTokenDay>,
    /// **The zone whose midnight cut the buckets in `days`,
    /// `model_days` and `model_tokens`** — IANA name, or `"UTC"`.
    ///
    /// Read the day a bucket belongs to **from `at` and this together**.
    /// An instant alone does not name a day: 2026-09-08 00:00 in
    /// Asia/Shanghai is 2026-09-07 16:00Z, and a screen formatting that
    /// instant in UTC prints the day before for every account east of
    /// Greenwich. Measured on real accounts 2026-09-09: **every new-api
    /// row was a day early**, seven days out of seven.
    ///
    /// ⚠️ **Why it is filled on all three roads and not only the one
    /// that needed it.** The two vendors answer buckets of their own,
    /// cut at UTC (Anthropic's `starting_at`, OpenAI's `start_time`);
    /// new-api answers *records*, and `khor_node::accounts`'s
    /// `read_log` cuts them into days itself, at the **reading node's**
    /// local midnight. Filling only the third would leave the contract
    /// two sentences — 「the provider's bucket, except on that road」 —
    /// and a reader would have to know which road they were on to
    /// format a date. Filled everywhere, it is one sentence and needs
    /// no branch: **`at` is an instant, `zone` cut it, read the day
    /// with both.**
    ///
    /// `None` is an older peer that predates this field. The face falls
    /// back to UTC there, **and on the new-api road that is the old
    /// bug, silently** — nothing on the screen says the date might be a
    /// day out.
    ///
    /// ⚠️ **When this default stops being a transition and becomes a
    /// fault**: once every node in the fleet is past this release, a
    /// `None` here means a node fell behind rather than 「not yet
    /// upgraded」, and those two want different handling. Nothing
    /// measures that today; the day someone adds it, this is the line
    /// to read.
    ///
    /// **Tail field, `serde default`, no `skip_serializing_if`** — and
    /// the sentence `khor_node::proto` demands before appending: an
    /// older peer refusing this longer frame keeps its previous
    /// readings with an age that goes on climbing, which is different
    /// from what it shows on success and lands where the 額度 page
    /// already says 「I cannot tell」. The same answer the fields above
    /// gave, for the same consumer.
    #[serde(default)]
    #[ts(type = "string | null")]
    pub zone: Option<String>,
}

/// What khor can say about the subscription right now.
///
/// **Both halves can be empty and that is not a bug**: before the first
/// answer arrives there is neither a reading nor a reason, and a screen
/// that guessed one would be describing a state that has not happened.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct QuotaAnswer {
    pub quota: Option<Quota>,
    pub trouble: Option<QuotaTrouble>,
}

/// Whether a line says anything a person can read: a letter or a digit
/// somewhere in it. A line of only box drawing, block elements,
/// whitespace or punctuation — a prompt's frame (「╰─ ─╯」), a rule — is
/// not a preview (the audit fix plan F2.3: 5 of 6 terminal rows showed
/// one under their title).
pub fn says_something(line: &str) -> bool {
    line.chars().any(char::is_alphanumeric)
}

/// One line of preview out of arbitrary said text, for
/// [`Session::last`]: whitespace runs (newlines included) collapse to
/// single spaces, and the result is cut at a character boundary. The
/// cap is generous because the row truncates visually anyway — this cut
/// only keeps a pasted wall of text from riding every list frame.
pub fn preview(text: &str) -> String {
    let mut out = String::new();
    let mut spaced = true;
    for c in text.chars() {
        if c.is_whitespace() {
            if !spaced {
                out.push(' ');
                spaced = true;
            }
        } else {
            out.push(c);
            spaced = false;
        }
        if out.chars().count() >= 160 {
            break;
        }
    }
    out.trim_end().to_owned()
}

/// Said text with its markdown marks taken off, for a **preview line**.
///
/// Agents write markdown. A conversation pane renders it (the GUI's
/// `components/Markdown`); a one-line preview has nowhere to render it
/// to, so left alone it shows the source: a real row on this machine's
/// list read its last sentence out as literal asterisks and backticks —
/// which is the one thing a preview may not be, the worse reading of the
/// same sentence.
///
/// Deliberately conservative, because the text it is handed is prose,
/// not a document: a delimiter is only a mark when a matching one
/// closes it on the same line and it flanks its content the way
/// CommonMark requires (`2 * 3 * 4` keeps its stars, `snake_case` keeps
/// its underscore). What is not certain is left as typed — a preview
/// with a stray asterisk reads fine; a preview with a word eaten does
/// not.
///
/// **Only for text that is known to be a message.** A shell row's
/// preview is a terminal's last line, where `*` and `` ` `` are bytes
/// somebody's program wrote and mean themselves.
pub fn plain(text: &str) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&plain_line(line));
    }
    out
}

/// One line, marks off: the line-anchored ones first (they only count at
/// the head), then the paired ones inside.
fn plain_line(line: &str) -> String {
    let mut rest = line.trim_start();
    // A rule is a line that is only a rule: it says nothing to preview.
    let bare = rest.trim_end();
    if bare.len() >= 3
        && (bare.chars().all(|c| c == '-')
            || bare.chars().all(|c| c == '*')
            || bare.chars().all(|c| c == '_'))
    {
        return String::new();
    }
    // A fence says what language the block is in, not what was said.
    if bare.starts_with("```") || bare.starts_with("~~~") {
        return String::new();
    }
    // A table's pipes are its grid. Only when the line starts with one:
    // prose does not, and a leading pipe is what every table row has.
    if rest.starts_with('|') {
        if bare.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
            return String::new(); // the rule row under the header
        }
        return bare
            .trim_matches('|')
            .split('|')
            .map(|cell| inline(cell.trim()))
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    }
    // Quote markers stack; a heading or a bullet can sit inside one.
    while let Some(r) = rest.strip_prefix('>') {
        rest = r.trim_start();
    }
    if let Some(r) = rest.strip_prefix(|c| c == '#') {
        let hashes = 1 + r.chars().take_while(|c| *c == '#').count();
        let after = &rest[hashes..];
        if hashes <= 6 && after.starts_with(' ') {
            rest = after.trim_start();
        }
    }
    for mark in ["- ", "* ", "+ "] {
        if let Some(r) = rest.strip_prefix(mark) {
            rest = r.trim_start();
            break;
        }
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && rest[digits..].starts_with(". ") {
        rest = rest[digits + 2..].trim_start();
    }
    // A task box is content, not a mark: whether it is ticked is the
    // whole point of the line.
    inline(rest)
}

/// The paired marks: code spans, emphasis, strikethrough, links.
fn inline(line: &str) -> String {
    let src: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < src.len() {
        let c = src[i];
        // An escape means the next character is itself.
        if c == '\\' && i + 1 < src.len() {
            out.push(src[i + 1]);
            i += 2;
            continue;
        }
        // `code` — equal-length runs, no flanking rule (CommonMark).
        if c == '`' {
            let run = src[i..].iter().take_while(|c| **c == '`').count();
            if let Some(close) = closing_run(&src, i + run, '`', run) {
                out.extend(&src[i + run..close]);
                i = close + run;
                continue;
            }
        }
        // *emphasis* / _emphasis_ / ~~strike~~
        if c == '*' || c == '_' || c == '~' {
            let run = src[i..].iter().take_while(|x| **x == c).count();
            let opens = run <= 3
                && src.get(i + run).is_some_and(|n| !n.is_whitespace())
                && (c != '_'
                    || i == 0
                    || src[i - 1].is_whitespace()
                    || !src[i - 1].is_alphanumeric());
            if opens && let Some(close) = closing_run(&src, i + run, c, run) {
                let closes = !src[close - 1].is_whitespace()
                    && (c != '_'
                        || src
                            .get(close + run)
                            .is_none_or(|n| n.is_whitespace() || !n.is_alphanumeric()));
                if closes {
                    out.push_str(&inline(&src[i + run..close].iter().collect::<String>()));
                    i = close + run;
                    continue;
                }
            }
        }
        // [label](url) and ![alt](url) — the label is what was written
        // here; the address belongs to a face that can open it.
        if c == '[' || (c == '!' && src.get(i + 1) == Some(&'[')) {
            let open = if c == '!' { i + 1 } else { i };
            if let Some(shut) = src[open..].iter().position(|x| *x == ']').map(|p| open + p)
                && src.get(shut + 1) == Some(&'(')
                && let Some(end) = src[shut..].iter().position(|x| *x == ')').map(|p| shut + p)
            {
                out.push_str(&inline(&src[open + 1..shut].iter().collect::<String>()));
                i = end + 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Where the run of `n` copies of `mark` closing the one at `from`
/// starts — a run of exactly `n`, so ``` ``a`` ``` is not closed by a
/// single backtick.
fn closing_run(src: &[char], from: usize, mark: char, n: usize) -> Option<usize> {
    let mut i = from;
    while i < src.len() {
        if src[i] == mark {
            let run = src[i..].iter().take_while(|c| **c == mark).count();
            if run == n && i > from {
                return Some(i);
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

/// The uniform event envelope. Payload bytes are kind-namespaced; this
/// crate never looks inside. Whether events replicate (CRDT) or are
/// fetched from home is a kind property, not an envelope field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub session: SessionId,
    /// Per-session, monotonically increasing; unread derives from it.
    pub seq: u64,
    pub at: Millis,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

/// The five answers the list renders (docs/SESSION.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub kind: Kind,
    pub title: String,
    /// The device responsible: execution and answers happen there.
    pub home: DeviceId,
    pub state: StateStamp,
    /// Events not yet seen. Seen state replicates, so clearing it on one
    /// device clears it everywhere.
    pub unread: u64,
    /// **Whose** session this is (`claude`, `codex`, [`category::khor()`],
    /// [`category::shell()`]) — a different question from [`Session::kind`],
    /// which is what *shape* it is (a tui, a chat, a transfer). One
    /// machine's list is mostly tui rows, and the useful cut across them
    /// is which agent they belong to.
    ///
    /// It rides the row rather than being worked out where the row is
    /// read, because **only the home device can answer it**: the
    /// adaptor that recognised the session knows whose file it read, and
    /// nobody downstream can recover that. Guessing it from the title is
    /// the thing this field exists to prevent.
    ///
    /// `None` means nobody could place it, and it stays `None`: a row
    /// khor started as a tui but cannot attribute to any vendor is not
    /// [`category::shell()`] — filing it under a neighbour would be a
    /// confident wrong answer where an honest gap belongs (docs/SESSION.md
    /// 认不出就不落词, the same rule the state words follow).
    ///
    /// Wire: appended at the tail with a serde default, so an older peer's
    /// six-field frame still decodes (`a_frame_from_an_older_peer_...`).
    #[serde(default)]
    pub category: Option<String>,
    /// The most recent thing said or shown, one line, for the row's
    /// preview (like a chat list's last message). What it is is the
    /// kind's judgment: a chat's last message, a claude session's last
    /// transcript utterance, a hosted terminal's last non-empty screen
    /// line. `None` when the kind has nothing of the sort — the row
    /// simply paints no preview, never an invented one.
    #[serde(default)]
    pub last: Option<String>,
    /// The network exit this session leaves through (device hex), when
    /// it borrows one (docs/NET.md 借网: 消费者行戴出口记号). `None` =
    /// its own network. No producer sets it yet — it lands with
    /// session `--via` (ledger) — but the slot is the row's shape, so
    /// it is minted with the rest.
    #[serde(default)]
    pub via: Option<String>,
    /// The home device can stand a terminal up for this row right now:
    /// it hosts the session already, or the session sits in a local
    /// multiplexer a host can bridge into. Only the home device can
    /// answer this (the process and the multiplexer are both there), so
    /// it rides the row like `category` does. Readers on other devices
    /// must still gate on locality: the terminal itself does not travel
    /// (remote attach is on the ledger).
    #[serde(default)]
    pub term: bool,
    /// Nothing will move this row again on its own: the process behind
    /// it is gone (a recorded exit, or a backing that vanished), or a
    /// transfer reached an end nobody can resume. **Not derivable from
    /// the word**: a shell that exited cleanly and was looked at reads
    /// 空闲, exactly like a shell sitting alive at a prompt — only the
    /// home device can tell them apart, so the fact rides the row like
    /// `category` does. A chat row is never dead: the other machine can
    /// always speak again. What readers do with it (退场 into the folded
    /// tail) is the reader's judgment, gated on more than this one bit.
    ///
    /// Wire: appended at the tail with a serde default, so an older
    /// peer's ten-field frame still decodes — and its rows read as not
    /// dead, which fails safe: a row wrongly kept is clutter, a row
    /// wrongly folded is a disappearance.
    #[serde(default)]
    pub dead: bool,
    /// Nobody can vouch for this row's word any more: no pid to ask, no
    /// vendor file being written, no host — the word is the last thing
    /// anything said, standing only because nobody is in a position to
    /// pronounce the session dead (a hook-observed row after its vendor
    /// file vanished is the living case). **Not an age**: readers pair
    /// it with the stamp's age to decide when an unvouched *live* word
    /// (忙碌/待批/中断) stops being believable and fades — 词的保鲜期批,
    /// the same axis as 离线 (docs/SESSION.md): the word itself never
    /// changes, "don't know any more" is never painted as a new word.
    /// Rows with a real verdict available (a live pid, a vendor file, a
    /// listing multiplexer) carry `false` by construction and never
    /// fade, however old their stamp.
    ///
    /// Wire: appended at the tail with a serde default; an older peer's
    /// rows read as vouched-for, which fails safe — a stale word shown
    /// plainly is the status quo, a fresh word wrongly faded is new
    /// damage.
    #[serde(default)]
    pub unbacked: bool,
    /// **Where this session lives** — its working directory taken up to
    /// the project it belongs to (`khor_node::place`). The list's
    /// 工作区 grouping is the whole of what it is for: people walk into
    /// a directory and start something there, and "which of these are
    /// in tokens" is a question no other field on this row can answer.
    ///
    /// **Only the home device can say**, like [`Session::category`] and
    /// for the same reason: the fact is on that disk — a vendor's own
    /// record of where its conversation ran, or khor's note of where it
    /// started a host. A reader that tried to derive it would be
    /// deriving it from nothing.
    ///
    /// `None` is an honest gap and stays one (docs/SESSION.md
    /// 认不出就不落词): a row whose place nobody recorded is not filed
    /// under a neighbour's project. Rows that legitimately have no
    /// place at all — a conversation with another machine, a transfer —
    /// carry `None` for the same reason and are grouped as such.
    ///
    /// **The working directory itself**, decided at the source
    /// (`khor_node::live::place_of`). It was the enclosing checkout
    /// until 工作区批③ (2026-09-15); a workspace is any directory a
    /// person named now, and the pane that lists a project's sessions
    /// asks 「which of these is running *under* this directory」 — a
    /// question a folded answer cannot be asked. Whoever wants
    /// something coarser folds it: a raw directory can always be
    /// folded, a folded one cannot be unfolded.
    ///
    /// Wire: appended at the tail with a serde default, so an older
    /// peer's twelve-field frame still decodes — and its rows read as
    /// placeless, which is exactly what that peer knows.
    #[serde(default)]
    pub place: Option<String>,
    /// **The session's process sits inside a terminal multiplexer on
    /// its home machine** (a tmux session holds it). The face rule
    /// (2026-09-01: 「什么宿主跑的就用什么显示」) needs this told apart
    /// from [`Session::term`]: `term` says a terminal *can* be had —
    /// which is also true of a conversation khor hosts itself, whose
    /// ghost is a host — while this says where the person actually is.
    /// A session that is both khor's conversation and resumed in the
    /// person's tmux (`claude -r`) is one row of kind `gui`, and this
    /// is what sends it to the terminal by default.
    ///
    /// Only the home device can say (the multiplexer is there). Wire:
    /// appended at the tail with a serde default; an older peer's rows
    /// read as not multiplexed, which is exactly what that peer knew.
    #[serde(default)]
    pub mux: bool,
    /// What the session's shell is doing, from the marks its shell
    /// integration writes (roadmap 1.2, `khor_node::commands`), or for a
    /// tmux row from tmux's own `pane_current_command` and
    /// `pane_current_path`. `None` when nothing knows.
    ///
    /// Wire: appended at the tail with a serde default, like `mux`.
    #[serde(default)]
    pub doing: Option<Doing>,
}

/// **What a terminal session is doing** (roadmap 1.2 and 2.1): the command
/// that runs or last ran, where, how it ended — the row's second line and
/// the tab's name are made from it, never from the prompt's own glyphs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Doing {
    /// The command line as typed, one line (a multi-line one is joined).
    pub command: Option<String>,
    /// It is running now (its end has not been marked).
    pub running: bool,
    /// When it started and ended, ms since the epoch; 0 when unknown.
    #[ts(type = "number")]
    pub started_ms: u64,
    #[ts(type = "number | null")]
    pub ended_ms: Option<u64>,
    /// How it ended, when the shell said.
    pub exit: Option<i32>,
    /// The shell's working directory (OSC 7), a path on the home machine.
    pub cwd: Option<String>,
    /// The terminal's title (OSC 0 / 2), when a program set one.
    pub title: Option<String>,
    /// A full-screen program holds the screen (the alternate buffer or
    /// the mouse taken).
    pub full_screen: bool,
    /// What a program said of its progress (`OSC 9 ; 4`, ConEmu's and
    /// Windows Terminal's): 0–100, or 255 for "busy, no figure". `None`
    /// when nothing is in progress (roadmap 4.1).
    #[serde(default)]
    pub progress: Option<u8>,
    /// The progress said something went wrong (`OSC 9 ; 4 ; 2`).
    #[serde(default)]
    pub progress_failed: bool,
    /// The last notification a program asked for (`OSC 9 ; text`,
    /// iTerm2's; `OSC 777 ; notify ; title ; body`, urxvt's and
    /// Ghostty's), and when — the face says it like a finished command.
    #[serde(default)]
    pub notice: Option<String>,
    #[serde(default)]
    #[ts(type = "number")]
    pub notice_ms: u64,
    /// A khor shell shows a tmux client whose pane in front has sent no
    /// command marks (its shell does not load khor's integration), so
    /// nothing it runs reaches the history (SESS-BRIEF 3). False where
    /// nothing knows: an older host or node, or no pane named.
    ///
    /// Wire: appended at the tail with a serde default.
    #[serde(default)]
    pub pane_unmarked: bool,
    /// When the terminal last rang its bell (BEL), ms since the epoch; 0
    /// when it never has. The face marks a session whose bell rang while
    /// it was off screen (the terminal audit, item 67).
    ///
    /// Wire: appended at the tail with a serde default.
    #[serde(default)]
    #[ts(type = "number")]
    pub bell_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **An older peer's reading, and the field it has never heard of.**
    ///
    /// `AccountReading::zone` was appended 2026-09-09 to carry the zone
    /// whose midnight cut the buckets. Nodes do not upgrade at once, so
    /// the wire has to survive a sender that predates it — and this is
    /// the half that is easy to get wrong twice over:
    ///
    ///   1. **the frame really has to be missing the key.** A test that
    ///      serialised the new struct and read it back would pass on a
    ///      `zone` written and read as `null`, which is a different
    ///      fact from 「the sender never had one」. So the key is taken
    ///      out and its absence is asserted **by name** before anything
    ///      is decoded.
    ///   2. **the default has to be `None`, not a value.** A `"UTC"`
    ///      default would put a date on screen and swear to a zone
    ///      nobody sent — 「一档接不上会变成旁边那一档」 in its wire form.
    ///
    /// ⚠️ **The old frame is built by removing the key from a real
    /// one**, not by hand-writing an older shape: the first draft did
    /// hand-write it, guessed `home`'s encoding wrong, and failed to
    /// decode **for a reason that had nothing to do with the field
    /// being tested** — a red that points somewhere else is worse than
    /// no test, and here the shape is not the thing under test.
    #[test]
    fn a_reading_from_before_the_zone_field_still_decodes() {
        let mine = AccountReading {
            account: "acc-1".to_owned(),
            name: "gateway".to_owned(),
            provider: "newapi".to_owned(),
            home: "dev0".to_owned(),
            fill: None,
            spent: Some(1.5),
            limit: None,
            currency: Some("USD".to_owned()),
            unlimited: false,
            resets_at: None,
            trouble: None,
            at_ms: 1_800_000_000_000,
            remaining: None,
            expires_at: None,
            models: Vec::new(),
            days: Vec::new(),
            spent_this_month: None,
            spent_all_time: None,
            model_days: Vec::new(),
            log_capped: None,
            windows: Vec::new(),
            plan: None,
            credits: None,
            model_tokens: Vec::new(),
            zone: Some("Asia/Shanghai".to_owned()),
        };
        let mut frame = serde_json::to_value(&mine).expect("a reading serialises");
        assert!(
            frame.get("zone").is_some(),
            "this node writes the field, or the removal below proves nothing"
        );
        frame.as_object_mut().expect("an object").remove("zone");
        let bytes = serde_json::to_vec(&frame).expect("the older frame serialises");
        let text = String::from_utf8(bytes.clone()).expect("json is utf-8");
        assert!(
            !text.contains("zone"),
            "the sample still carries a zone key, so 「an older peer never sent one」 is not what this is testing: {text}",
        );
        let back: AccountReading =
            serde_json::from_slice(&bytes).expect("an older peer's frame still decodes");
        assert_eq!(
            back.zone, None,
            "a frame with no zone must read as 「nobody said」, never as a zone"
        );
        assert_eq!(
            back.spent,
            Some(1.5),
            "the fields that were there are still there"
        );
        let again: AccountReading =
            serde_json::from_slice(&serde_json::to_vec(&mine).expect("serialises"))
                .expect("decodes");
        assert_eq!(
            again.zone.as_deref(),
            Some("Asia/Shanghai"),
            "and what this node sends survives its own round trip"
        );
    }

    fn a_session(kind: &str) -> Session {
        Session {
            id: SessionId("dev0/demo/1".to_owned()),
            kind: Kind(kind.to_owned()),
            title: "build khor".to_owned(),
            home: DeviceId([7; 32]),
            state: StateStamp {
                state: State::Busy,
                at: Millis(1),
            },
            unread: 3,
            place: None,
            mux: false,
            doing: None,
            category: Some("claude".to_owned()),
            last: Some("last said".to_owned()),
            via: None,
            term: true,
            dead: false,
            unbacked: false,
        }
    }

    #[test]
    fn an_unknown_kind_still_decodes_to_a_basic_row() {
        let bytes = rmp_serde::to_vec(&a_session("holodeck")).unwrap();
        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(row.kind.0, "holodeck");
        assert_eq!(row.title, "build khor");
        assert_eq!(row.state.state, State::Busy);
        assert_eq!(row.unread, 3);
    }

    /// The ranking is a total order over exactly the six — no ties, no
    /// gaps — and it leads with 待批 and ends with 失败. Stated as the
    /// full list rather than as "every word has a rank", because the
    /// property version is blind to an implementation that returns the
    /// same number for everything.
    #[test]
    fn the_ranking_runs_from_who_needs_you_to_what_is_over() {
        let mut words = State::ALL;
        words.sort_by_key(|s| s.rank());
        assert_eq!(
            words.map(|s| s.key()),
            ["blocked", "errored", "done", "busy", "idle", "failed"],
        );
        let mut ranks: Vec<u8> = State::ALL.iter().map(|s| s.rank()).collect();
        ranks.sort_unstable();
        assert_eq!(ranks, vec![0, 1, 2, 3, 4, 5], "a rank each, none shared");
    }

    #[test]
    fn offline_is_not_a_seventh_word() {
        // docs/SESSION.md pins offline as never-a-state, which makes it a
        // sample that can't quietly become real later.
        assert!(State::try_from("offline".to_owned()).is_err());
    }

    #[test]
    fn every_word_crosses_the_wire_as_its_key() {
        for state in State::ALL {
            let bytes = rmp_serde::to_vec(&state).unwrap();
            assert_eq!(&bytes[1..], state.key().as_bytes());
            let back: State = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(back, state);
        }
    }

    /// The wire shape of `Session`, spelled out so the test below can
    /// deserialize into it positionally. A `type` only, to keep
    /// `clippy::type_complexity` off the `let` — the order is the point.
    type SessionFrame = (
        SessionId,
        Kind,
        String,
        DeviceId,
        StateStamp,
        u64,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
        bool,
        bool,
        Option<String>,
        bool,
        Option<Doing>,
    );

    #[test]
    fn the_session_frame_is_a_fixed_order_array_of_fifteen() {
        let session = a_session("shell");
        let bytes = rmp_serde::to_vec(&session).unwrap();
        // 0x9f = msgpack fixarray of 15. Adding, removing, or reordering a
        // field must land here first, consciously. Six → seven appended
        // `category`; seven → nine appended `last` and `via` together
        // (会话行改版批); nine → ten appended `term` (会话身份批);
        // ten → eleven appended `dead` (退场批); eleven → twelve appended
        // `unbacked` (词的保鲜期批); twelve → thirteen appended `place`
        // (工作区批); thirteen → fourteen appended `mux` (面跟宿主批);
        // fourteen → fifteen appended `doing` (roadmap 1.2) —
        // always at the tail, never in the middle: a mid-frame optional
        // desyncs the array on every older end.
        assert_eq!(bytes[0], 0x9f);
        let (
            id,
            kind,
            title,
            home,
            state,
            unread,
            category,
            last,
            via,
            term,
            dead,
            unbacked,
            place,
            mux,
            _doing,
        ): SessionFrame = rmp_serde::from_slice(&bytes).unwrap();
        // Field by field rather than as one tuple comparison: std stops
        // deriving `PartialEq` at twelve, and this frame is thirteen.
        // Deserializing into the tuple above is what pins the *order* —
        // a reordered field lands as the wrong type and never reaches
        // these lines.
        assert_eq!(id, session.id);
        assert_eq!(kind, session.kind);
        assert_eq!(title, session.title);
        assert_eq!(home, session.home);
        assert_eq!(state, session.state);
        assert_eq!(unread, session.unread);
        assert_eq!(category, session.category);
        assert_eq!(last, session.last);
        assert_eq!(via, session.via);
        assert_eq!(term, session.term);
        assert_eq!(dead, session.dead);
        assert_eq!(unbacked, session.unbacked);
        assert_eq!(place, session.place);
        assert_eq!(mux, session.mux);
    }

    /// **Over the wire from an older peer**: a machine that predates
    /// `unbacked` sends eleven fields, and its rows must read as
    /// vouched-for — a stale word shown plainly is the status quo, a
    /// fresh word wrongly faded is new damage. Built by hand as an
    /// eleven-tuple; the first assertion is that the bytes really are an
    /// eleven-array, or this proves nothing.
    #[test]
    fn a_frame_from_an_older_peer_decodes_as_vouched_for() {
        let old = (
            SessionId("dev0/demo/1".to_owned()),
            Kind("tui".to_owned()),
            "build khor".to_owned(),
            DeviceId([7; 32]),
            StateStamp {
                state: State::Busy,
                at: Millis(1),
            },
            0u64,
            Some("claude".to_owned()),
            None::<String>,
            None::<String>,
            false,
            false,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x9b,
            "the sample must be an eleven-field frame, or it proves nothing"
        );

        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert!(
            !row.unbacked,
            "an older peer said nothing about backing; nothing may fade"
        );
        assert_eq!(row.title, "build khor", "…and the rest of the row survived");
    }

    /// **Over the wire from an older peer**: a machine that predates
    /// `mux` sends thirteen fields, and its rows must read as not
    /// multiplexed — the face then routes them as it always did, which
    /// is the truth about what that peer could say.
    #[test]
    fn a_frame_from_an_older_peer_decodes_as_not_multiplexed() {
        let old = (
            SessionId("dev0/demo/1".to_owned()),
            Kind("tui".to_owned()),
            "build khor".to_owned(),
            DeviceId([7; 32]),
            StateStamp {
                state: State::Busy,
                at: Millis(1),
            },
            0u64,
            Some("claude".to_owned()),
            None::<String>,
            None::<String>,
            false,
            false,
            false,
            None::<String>,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x9d,
            "the sample must be a thirteen-field frame, or it proves nothing"
        );

        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert!(!row.mux, "an older peer said nothing about a multiplexer");
        assert_eq!(row.title, "build khor", "…and the rest of the row survived");
    }

    /// **Over the wire from an older peer**: a machine that predates
    /// `place` sends twelve fields, and its rows must read as placeless
    /// — which is the truth about that peer, not a loss: it has no
    /// answer to give, and inventing one from the title or the id is
    /// exactly what an honest gap exists to prevent.
    ///
    /// Built by hand as a twelve-tuple, and the first assertion is that
    /// the bytes really are a twelve-array — without it this could be
    /// encoding today's struct and proving nothing.
    #[test]
    fn a_frame_from_an_older_peer_decodes_as_placeless() {
        let old = (
            SessionId("dev0/demo/1".to_owned()),
            Kind("tui".to_owned()),
            "build khor".to_owned(),
            DeviceId([7; 32]),
            StateStamp {
                state: State::Busy,
                at: Millis(1),
            },
            0u64,
            Some("claude".to_owned()),
            None::<String>,
            None::<String>,
            false,
            false,
            false,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x9c,
            "the sample must be a twelve-field frame, or it proves nothing"
        );

        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(
            row.place, None,
            "an older peer knows of no places, and khor invents none"
        );
        assert_eq!(row.title, "build khor", "…and the rest of the row survived");
    }

    /// **Over the wire from an older peer**: a machine that predates
    /// `dead` sends ten fields, and its rows must read as not dead —
    /// wrongly kept clutter, never a wrong disappearance.
    ///
    /// Built by hand as a ten-tuple, and the first assertion is that the
    /// bytes really are a ten-array — without it this could be encoding
    /// today's struct and proving nothing.
    #[test]
    fn a_frame_from_an_older_peer_decodes_as_not_dead() {
        let old = (
            SessionId("dev0/demo/1".to_owned()),
            Kind("shell".to_owned()),
            "build khor".to_owned(),
            DeviceId([7; 32]),
            StateStamp {
                state: State::Failed,
                at: Millis(1),
            },
            0u64,
            Some("shell".to_owned()),
            None::<String>,
            None::<String>,
            false,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x9a,
            "the sample must be a ten-field frame, or it proves nothing"
        );

        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert!(
            !row.dead,
            "an older peer said nothing about dead, and nothing must fold"
        );
        assert_eq!(row.title, "build khor", "…and the rest of the row survived");
    }

    /// **Over the wire from an older peer**: a machine that predates
    /// `category` sends six fields, and this end must read that row
    /// rather than dropping the peer's whole list.
    ///
    /// The frame is built by hand as a six-tuple, and the first assertion
    /// is that the bytes really are a six-array — without it the test
    /// could be encoding today's struct and proving nothing (a test that
    /// passes because both sides changed together is the failure mode
    /// this whole discipline exists for).
    #[test]
    fn a_frame_from_an_older_peer_decodes_with_no_category() {
        let old = (
            SessionId("dev0/demo/1".to_owned()),
            Kind("tui".to_owned()),
            "build khor".to_owned(),
            DeviceId([7; 32]),
            StateStamp {
                state: State::Busy,
                at: Millis(1),
            },
            3u64,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x96,
            "the sample must be a six-field frame, or it proves nothing"
        );

        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(
            row.category, None,
            "an older peer said nothing about category"
        );
        assert_eq!(row.title, "build khor", "…and the rest of the row survived");
        assert_eq!(row.unread, 3);
    }

    /// **Over the wire from an older peer**: a khor that predates the
    /// fields added on 2026-09-07 sends thirteen, and its readings must
    /// come back with none of them — **not a date at the epoch, not a
    /// phantom model limit, not a chart of a single day in 1970**,
    /// which are the shapes those fields turn into when guessed at.
    ///
    /// **The byte assertion is the whole test.** Without it the sample
    /// could be a fifteen-field frame with the two new fields written
    /// as their defaults, and this would be checking that serde reads
    /// its own output — the thing the repo's rule about backward
    /// compatibility tests exists to forbid. `0x9d` is msgpack's
    /// fixarray of thirteen; anything else means the sample drifted and
    /// the test is no longer about an older peer.
    #[test]
    fn an_account_reading_from_an_older_peer_carries_none_of_the_tail_fields() {
        let old = (
            "acc1".to_owned(),
            "gateway".to_owned(),
            "newapi".to_owned(),
            "aa".to_owned(),
            Some(0.45f32),
            Some(45.0f64),
            Some(100.0f64),
            Some("USD".to_owned()),
            false,
            None::<i64>,
            None::<String>,
            7i64,
            None::<f64>,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x9d,
            "the sample must be a thirteen-field frame, or it proves nothing"
        );
        for absent in [
            "expires",
            "models",
            "days",
            "spent_this",
            "model_days",
            "log_capped",
        ] {
            assert!(
                !String::from_utf8_lossy(&bytes).contains(absent),
                "the sample must not carry {absent}, a key it predates",
            );
        }

        let r: AccountReading = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(
            r.expires_at, None,
            "an older peer said nothing about expiry"
        );
        assert!(r.models.is_empty(), "…and nothing about a model limit");
        assert!(
            r.days.is_empty(),
            "…and no day buckets, rather than a day at the epoch"
        );
        assert_eq!(r.spent_this_month, None, "…and no month figure");
        assert_eq!(r.spent_all_time, None, "…nor a lifetime one");
        assert!(r.model_days.is_empty(), "…no itemised models");
        assert_eq!(r.log_capped, None, "…and a log it never had was not cut");
        assert_eq!(
            r.fill,
            Some(0.45),
            "…while the rest of the reading survived whole"
        );
        assert_eq!(r.name, "gateway");
    }

    /// **Over the wire from an older peer**: a machine whose khor predates
    /// the disk reading sends three fields, and this end must read the two
    /// it did send rather than dropping the whole answer.
    ///
    /// Built by hand as a three-tuple, and the first assertion is that the
    /// bytes really are a three-array — without it this could be encoding
    /// today's struct and proving nothing.
    #[test]
    fn a_vitals_frame_from_an_older_peer_decodes_with_no_disk() {
        let old = (12.5f32, 10u32, Fill { used: 8, total: 32 });
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x93,
            "the sample must be a three-field frame, or it proves nothing"
        );

        let v: Vitals = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(v.disk, None, "an older peer said nothing about a disk");
        assert_eq!(v.cpu_pct, 12.5, "…and the rest of the reading survived");
        assert_eq!((v.cores, v.mem.used, v.mem.total), (10, 8, 32));
    }

    /// The tail field is on the wire when it *is* known — the other half
    /// of the test above, which alone would also pass if `disk` were
    /// dropped from the struct entirely.
    #[test]
    fn a_vitals_frame_carries_the_disk_when_there_is_one() {
        let v = Vitals {
            cpu_pct: 3.0,
            cores: 8,
            mem: Fill { used: 1, total: 2 },
            disk: Some(Fill { used: 5, total: 9 }),
            gpu: None,
            version: None,
            gpus: vec![],
            volumes: vec![],
            uptime_s: 0,
            model: None,
            system: None,
        };
        let bytes = rmp_serde::to_vec(&v).unwrap();
        assert_eq!(
            bytes[0], 0x9b,
            "eleven fields today (the system family rode the tail after the model, 2026-09-22)"
        );
        assert_eq!(rmp_serde::from_slice::<Vitals>(&bytes).unwrap(), v);
    }

    /// **Over the wire from a peer that predates the version field.**
    /// The machine an upgrade sweep is looking for is exactly the one
    /// running an older khor, so the frame it sends is the one this must
    /// not choke on — a decoder that dropped the whole reading would
    /// blank the row of every machine the sweep exists to find.
    ///
    /// Built by hand as a five-tuple, and the first assertion is that
    /// the bytes really are a five-array: without it this could be
    /// encoding today's struct and proving nothing.
    #[test]
    fn a_vitals_frame_from_a_peer_that_predates_versions_decodes_with_none() {
        let old = (
            7.5f32,
            4u32,
            Fill { used: 2, total: 16 },
            Some(Fill { used: 1, total: 4 }),
            Option::<Gpu>::None,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x95,
            "the sample must be a five-field frame, or it proves nothing"
        );

        let v: Vitals = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(
            v.version, None,
            "an older peer said nothing about its version"
        );
        assert_eq!(v.cpu_pct, 7.5, "…and the rest of the reading survived");
        assert_eq!((v.cores, v.mem.used, v.mem.total), (4, 2, 16));
        assert_eq!(v.disk, Some(Fill { used: 1, total: 4 }));
    }

    /// The two steps land exactly on 90 and 95 — an off-by-one here is a
    /// colour that appears a percent early or late on every machine.
    #[test]
    fn strain_steps_at_ninety_and_ninety_five_exactly() {
        let at = |used| Fill { used, total: 1000 }.strain();
        assert_eq!(at(899), None, "89.9% carries no word");
        assert_eq!(at(900), Some(Strain::Tight), "90.0% is the first step");
        assert_eq!(at(949), Some(Strain::Tight));
        assert_eq!(at(950), Some(Strain::Critical), "95.0% is the second");
        assert_eq!(at(1000), Some(Strain::Critical));
    }

    /// A zero total is "khor could not read this", never an empty — and
    /// so never a strained — resource. Guards the `total == 0` branch on
    /// its own: without it, `0 * 100 >= 0 * 95` would answer Critical.
    #[test]
    fn an_unreadable_fill_carries_no_strain_word() {
        assert_eq!(Fill { used: 0, total: 0 }.strain(), None);
        assert_eq!(Fill { used: 7, total: 0 }.strain(), None);
    }

    /// Petabyte-scale numbers must not wrap the arithmetic: `u64::MAX`
    /// bytes times 100 overflows u64, and a wrapped product would read a
    /// full disk as a fine one.
    #[test]
    fn strain_survives_fills_near_u64_max() {
        let full = Fill {
            used: u64::MAX,
            total: u64::MAX,
        };
        assert_eq!(full.strain(), Some(Strain::Critical));
        let fine = Fill {
            used: u64::MAX / 2,
            total: u64::MAX,
        };
        assert_eq!(fine.strain(), None);
    }

    /// **Over the wire from a peer that predates the GPU**: four fields
    /// arrive and this end reads them, rather than dropping the whole
    /// reading because a fifth is missing.
    ///
    /// Hand-built as a four-tuple, and the byte assertion comes first for
    /// the reason it always does here: encoding today's `Vitals` and
    /// decoding it back would pass whatever the struct looked like.
    #[test]
    fn a_vitals_frame_from_an_older_peer_decodes_with_no_gpu() {
        let old = (
            7.5f32,
            4u32,
            Fill { used: 2, total: 16 },
            Some(Fill { used: 5, total: 9 }),
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x94,
            "the sample must be a four-field frame, or it proves nothing"
        );

        let v: Vitals = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(v.gpu, None, "an older peer said nothing about a GPU");
        assert_eq!(v.cpu_pct, 7.5, "…and the rest of the reading survived");
        assert_eq!(v.disk, Some(Fill { used: 5, total: 9 }));
    }

    /// The GPU is on the wire when there is one, and it survives whole —
    /// the half that would still pass if `gpu` were dropped from the
    /// struct is the test above, so this one exists to deny it that.
    ///
    /// **Both shapes of `Gpu::mem` are sent**, because the interesting one
    /// is the absent case and a round trip that only ever carried `Some`
    /// would not notice a `None` turning into `0 / 0` on the way.
    #[test]
    fn a_vitals_frame_carries_the_gpu_when_there_is_one() {
        for mem in [Some(Fill { used: 3, total: 24 }), None] {
            let v = Vitals {
                cpu_pct: 3.0,
                cores: 8,
                mem: Fill { used: 1, total: 2 },
                disk: None,
                gpu: Some(Gpu {
                    util_pct: 48.0,
                    cards: 2,
                    mem,
                }),
                version: None,
                gpus: vec![],
                volumes: vec![],
                uptime_s: 0,
                model: None,
                system: None,
            };
            let bytes = rmp_serde::to_vec(&v).unwrap();
            assert_eq!(
                bytes[0], 0x9b,
                "eleven fields today (the system family rode the tail after the model, 2026-09-22)"
            );
            assert_eq!(rmp_serde::from_slice::<Vitals>(&bytes).unwrap(), v);
        }
    }

    /// **Over the wire from a peer that predates the per-card and
    /// per-volume readings**: six fields arrive and this end reads
    /// them, with both new lists empty — not guessed, never `0 / 0`.
    ///
    /// Hand-built as a six-tuple, byte-asserted first, for the reason
    /// every test in this family says: encoding today's struct and
    /// decoding it back proves nothing about yesterday's peers.
    #[test]
    fn a_vitals_frame_from_an_older_peer_decodes_with_no_cards_or_volumes() {
        let old = (
            7.5f32,
            4u32,
            Fill { used: 2, total: 16 },
            Some(Fill { used: 5, total: 9 }),
            Some(Gpu {
                util_pct: 40.0,
                cards: 2,
                mem: None,
            }),
            Some("0.1.13".to_owned()),
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(
            bytes[0], 0x96,
            "the sample must be a six-field frame, or it proves nothing"
        );

        let v: Vitals = rmp_serde::from_slice(&bytes).unwrap();
        assert!(v.gpus.is_empty(), "an older peer said nothing per card");
        assert!(v.volumes.is_empty(), "…nor per volume");
        assert_eq!(v.cpu_pct, 7.5, "…and the rest of the reading survived");
        assert_eq!(
            v.gpu,
            Some(Gpu {
                util_pct: 40.0,
                cards: 2,
                mem: None
            })
        );
    }

    /// The cards and the volumes are on the wire when they are known and
    /// survive whole — including a card with no memory of its own and a
    /// card with no name, the two shapes a round trip that only ever
    /// carried the full form would let rot quietly.
    #[test]
    fn the_model_kind_is_spelled_the_way_the_face_matches_it() {
        // The face's icon switch keys off these strings (SHELL.md 第五轮
        // 第 1 条 / 第 7 条); a rename here is an icon gone blank there.
        for (kind, word) in [
            (ModelKind::MacMini, "mac-mini"),
            (ModelKind::Macbook, "macbook"),
            (ModelKind::Imac, "imac"),
            (ModelKind::MacStudio, "mac-studio"),
            (ModelKind::Server, "server"),
            (ModelKind::Desktop, "desktop"),
            (ModelKind::Unknown, "unknown"),
        ] {
            assert_eq!(serde_json::to_value(kind).unwrap(), word);
        }
    }

    #[test]
    fn a_vitals_frame_carries_the_cards_and_the_volumes_when_known() {
        let v = Vitals {
            cpu_pct: 3.0,
            cores: 8,
            mem: Fill { used: 1, total: 2 },
            disk: Some(Fill { used: 5, total: 9 }),
            gpu: Some(Gpu {
                util_pct: 25.0,
                cards: 2,
                mem: None,
            }),
            version: None,
            gpus: vec![
                GpuCard {
                    name: "RTX A5000".to_owned(),
                    util_pct: 50.0,
                    mem: Some(Fill { used: 3, total: 24 }),
                    temp_c: Some(61.0),
                },
                GpuCard {
                    name: String::new(),
                    util_pct: 0.0,
                    mem: None,
                    temp_c: None,
                },
            ],
            volumes: vec![
                Volume {
                    mount: "/".to_owned(),
                    fill: Fill { used: 5, total: 9 },
                },
                Volume {
                    mount: "/data".to_owned(),
                    fill: Fill { used: 1, total: 4 },
                },
            ],
            uptime_s: 86_400 * 3,
            model: Some(Model {
                kind: ModelKind::MacMini,
                name: Some("Mac mini (2024)".to_owned()),
            }),
            system: Some(SystemKind::Macos),
        };
        let bytes = rmp_serde::to_vec(&v).unwrap();
        assert_eq!(
            bytes[0], 0x9b,
            "eleven fields today (the system family rode the tail after the model, 2026-09-22)"
        );
        assert_eq!(rmp_serde::from_slice::<Vitals>(&bytes).unwrap(), v);
    }

    /// The token frame is a fixed-order array of four, and a day's frame
    /// an array of three. Adding a fifth billing kind — or a per-model
    /// breakdown on the day — must land here first, consciously, at the
    /// tail (the rule the session and vitals frames follow).
    #[test]
    fn a_usage_frame_is_a_fixed_order_array() {
        let day = UsageDay {
            day: "2026-08-17".to_owned(),
            category: "claude".to_owned(),
            tokens: Tokens {
                input: 1,
                cached_input: 2,
                cache_write: 3,
                output: 4,
            },
        };
        let bytes = rmp_serde::to_vec(&day.tokens).unwrap();
        assert_eq!(bytes[0], 0x94, "four billing kinds today");
        let (input, cached_input, cache_write, output): (u64, u64, u64, u64) =
            rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!((input, cached_input, cache_write, output), (1, 2, 3, 4));

        let bytes = rmp_serde::to_vec(&day).unwrap();
        assert_eq!(bytes[0], 0x93, "day, category, tokens");
        assert_eq!(rmp_serde::from_slice::<UsageDay>(&bytes).unwrap(), day);

        let answer = Usage {
            days: vec![day.clone()],
            unreadable: 2,
            models: vec![UsageModelDay {
                day: day.day.clone(),
                category: day.category.clone(),
                model: Some("a-model".to_owned()),
                tokens: day.tokens,
            }],
            today: Some("2026-08-17".to_owned()),
        };
        let bytes = rmp_serde::to_vec(&answer).unwrap();
        // **Four, and in this order**: the rows, what could not be read,
        // the same rows cut by model, and the answering machine's own
        // day. The last two were appended in that order on 2026-09-07
        // and the count is asserted rather than the names because that
        // is what this encoding actually carries — a field inserted
        // anywhere but the tail moves everything after it and this line
        // is what says so out loud.
        assert_eq!(bytes[0], 0x94);
        assert_eq!(rmp_serde::from_slice::<Usage>(&bytes).unwrap(), answer);
        assert_eq!(
            rmp_serde::to_vec(&answer.models[0]).unwrap()[0],
            0x94,
            "day, category, model, tokens"
        );
    }

    /// **A count of what could not be read survives the wire**, which is
    /// the field most easily lost: an answer whose rows all decode looks
    /// complete, and the only thing saying otherwise is this number.
    ///
    /// The empty answer is sent too, because "no rows and nothing
    /// unreadable" (a machine that has never run an agent) and "no rows
    /// but two unreadable records" (khor gone blind) are the two
    /// situations this field exists to keep apart, and they differ by
    /// exactly one number.
    #[test]
    fn an_empty_answer_and_a_blind_one_do_not_look_alike_on_the_wire() {
        let quiet = Usage::default();
        let blind = Usage {
            unreadable: 2,
            ..Usage::default()
        };
        assert_ne!(
            rmp_serde::to_vec(&quiet).unwrap(),
            rmp_serde::to_vec(&blind).unwrap()
        );
        for answer in [quiet, blind] {
            let bytes = rmp_serde::to_vec(&answer).unwrap();
            assert_eq!(rmp_serde::from_slice::<Usage>(&bytes).unwrap(), answer);
        }
    }

    /// Adding saturates rather than wrapping. A wrapped counter reads as
    /// a machine that spent almost nothing, which is the one wrong answer
    /// nobody would question.
    #[test]
    fn adding_tokens_saturates_and_a_zero_day_knows_it_is_zero() {
        let mut t = Tokens {
            input: u64::MAX,
            cached_input: 0,
            cache_write: 0,
            output: 5,
        };
        t.add(Tokens {
            input: 10,
            cached_input: 0,
            cache_write: 0,
            output: 5,
        });
        assert_eq!(t.input, u64::MAX);
        assert_eq!(t.output, 10);
        assert!(!t.is_zero());
        assert!(Tokens::default().is_zero());
    }

    #[test]
    fn the_event_envelope_round_trips_with_opaque_payload() {
        let event = Event {
            session: SessionId("dev0/chat/9".to_owned()),
            seq: 42,
            at: Millis(1_700_000_000_000),
            payload: vec![0xde, 0xad, 0xbe, 0xef],
        };
        let bytes = rmp_serde::to_vec(&event).unwrap();
        let back: Event = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, event);
    }
}

#[cfg(test)]
mod plain_tests {
    use super::{plain, preview};

    /// The marks come off and the words stay, in the shapes a preview
    /// line actually meets.
    #[test]
    fn the_marks_come_off_and_the_words_stay() {
        assert_eq!(
            plain("it is **Nous**, `arXiv:2606.22030`; v1 was called *\"Nous\"*"),
            "it is Nous, arXiv:2606.22030; v1 was called \"Nous\""
        );
        assert_eq!(plain("## Overview"), "Overview");
        assert_eq!(
            plain("- never refuses an update"),
            "never refuses an update"
        );
        assert_eq!(plain("1. the first step"), "the first step");
        assert_eq!(plain("> the quoted line"), "the quoted line");
        assert_eq!(
            plain("see [khor](https://example.com) here"),
            "see khor here"
        );
        assert_eq!(plain("~~struck~~kept"), "struckkept");
        assert_eq!(plain("**bold with `code`**"), "bold with code");
        assert_eq!(plain("---"), "");
    }

    /// The two block shapes that carry their scaffolding on every line:
    /// a table's grid and a fence's language tag say nothing a preview
    /// line wants, and left in they crowd out the words that do.
    #[test]
    fn a_tables_grid_and_a_fences_tag_are_not_words() {
        let said = "Overview\n\n| item | count |\n|---|---:|\n| members | 16 |\n\n```python\nprint(1)\n```";
        assert_eq!(
            preview(&plain(said)),
            "Overview item count members 16 print(1)"
        );
    }

    /// What is not certainly a mark stays as typed: a preview with a
    /// stray asterisk reads fine, one with a word eaten does not.
    #[test]
    fn prose_that_only_looks_like_markdown_is_left_alone() {
        // Not flanking: a star with space on the inside opens nothing.
        assert_eq!(plain("2 * 3 * 4 = 24"), "2 * 3 * 4 = 24");
        // Intraword underscores are identifiers, not emphasis.
        assert_eq!(
            plain("run snake_case_name here"),
            "run snake_case_name here"
        );
        // Unclosed marks are just characters.
        assert_eq!(
            plain("wrote **and forgot to close"),
            "wrote **and forgot to close"
        );
        assert_eq!(plain("the cost is `unclosed"), "the cost is `unclosed");
        // A dash that leads real content is a bullet; a word is not.
        assert_eq!(plain("-nospace-dash"), "-nospace-dash");
        // An escape means the character itself.
        assert_eq!(plain("a literal \\*star\\*"), "a literal *star*");
    }

    /// `plain` is line-anchored, so it must run before `preview` folds
    /// the newlines away — the other order leaves every bullet's dash.
    #[test]
    fn the_line_anchored_marks_need_the_lines() {
        let said = "## Title\n- one\n- two";
        assert_eq!(preview(&plain(said)), "Title one two");
        assert_eq!(plain(&preview(said)), "Title - one - two");
    }
}

#[cfg(test)]
mod preview_tests {
    use super::preview;

    /// Newlines and runs of spaces collapse to one; the cap cuts at a
    /// character boundary (a CJK wall of text must not split a char).
    #[test]
    fn a_preview_is_one_line_and_bounded() {
        assert_eq!(preview("a  b\n\nc\t d"), "a b c d");
        let long = "字".repeat(500);
        let p = preview(&long);
        assert!(p.chars().count() <= 160, "{}", p.chars().count());
        assert!(p.chars().all(|c| c == '字'));
    }
}
