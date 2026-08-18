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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
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

/// Tokens spent, in the four kinds a vendor bills separately.
///
/// **There is no total here, and that is a judgment rather than an
/// omission.** Measured on this machine 2026-08-17 across every claude
/// transcript on disk: 32.2 million output tokens against 16.2 **billion**
/// cached input. A single sum would be 99.8% cache reads, so it would move
/// with how often a conversation was resumed and barely at all with how
/// much work was done — a number that looks like an answer and tracks the
/// wrong thing. Whoever paints this picks the kinds worth showing; nobody
/// downstream is handed a total that hides which is which.
///
/// The four are what both vendors bill separately today, spelled so that
/// one name means one thing on every vendor (`khor_node::usage` does the
/// per-vendor arithmetic, and says where the two disagree).
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS,
)]
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
    /// **Whose** session this is (`claude`, `codex`, [`category::KHOR`],
    /// [`category::SHELL`]) — a different question from [`Session::kind`],
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
    /// [`category::SHELL`] — filing it under a neighbour would be a
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_session(kind: &str) -> Session {
        Session {
            id: SessionId("dev0/demo/1".to_owned()),
            kind: Kind(kind.to_owned()),
            title: "build khor".to_owned(),
            home: DeviceId([7; 32]),
            state: StateStamp { state: State::Busy, at: Millis(1) },
            unread: 3,
            category: Some("claude".to_owned()),
            last: Some("last said".to_owned()),
            via: None,
            term: true,
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

    #[test]
    fn the_session_frame_is_a_fixed_order_array_of_ten() {
        let session = a_session("shell");
        let bytes = rmp_serde::to_vec(&session).unwrap();
        // 0x9a = msgpack fixarray of 10. Adding, removing, or reordering a
        // field must land here first, consciously. Six → seven appended
        // `category`; seven → nine appended `last` and `via` together
        // (会话行改版批); nine → ten appended `term` (会话身份批) —
        // always at the tail, never in the middle: a mid-frame optional
        // desyncs the array on every older end.
        assert_eq!(bytes[0], 0x9a);
        let (id, kind, title, home, state, unread, category, last, via, term): (
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
        ) = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(
            (id, kind, title, home, state, unread, category, last, via, term),
            (
                session.id,
                session.kind,
                session.title,
                session.home,
                session.state,
                session.unread,
                session.category,
                session.last,
                session.via,
                session.term
            )
        );
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
            StateStamp { state: State::Busy, at: Millis(1) },
            3u64,
        );
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(bytes[0], 0x96, "the sample must be a six-field frame, or it proves nothing");

        let row: Session = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(row.category, None, "an older peer said nothing about category");
        assert_eq!(row.title, "build khor", "…and the rest of the row survived");
        assert_eq!(row.unread, 3);
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
        assert_eq!(bytes[0], 0x93, "the sample must be a three-field frame, or it proves nothing");

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
        };
        let bytes = rmp_serde::to_vec(&v).unwrap();
        assert_eq!(bytes[0], 0x95, "five fields today");
        assert_eq!(rmp_serde::from_slice::<Vitals>(&bytes).unwrap(), v);
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
        let full = Fill { used: u64::MAX, total: u64::MAX };
        assert_eq!(full.strain(), Some(Strain::Critical));
        let fine = Fill { used: u64::MAX / 2, total: u64::MAX };
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
        let old = (7.5f32, 4u32, Fill { used: 2, total: 16 }, Some(Fill { used: 5, total: 9 }));
        let bytes = rmp_serde::to_vec(&old).unwrap();
        assert_eq!(bytes[0], 0x94, "the sample must be a four-field frame, or it proves nothing");

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
                gpu: Some(Gpu { util_pct: 48.0, cards: 2, mem }),
            };
            let bytes = rmp_serde::to_vec(&v).unwrap();
            assert_eq!(bytes[0], 0x95, "five fields today");
            assert_eq!(rmp_serde::from_slice::<Vitals>(&bytes).unwrap(), v);
        }
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
            tokens: Tokens { input: 1, cached_input: 2, cache_write: 3, output: 4 },
        };
        let bytes = rmp_serde::to_vec(&day.tokens).unwrap();
        assert_eq!(bytes[0], 0x94, "four billing kinds today");
        let (input, cached_input, cache_write, output): (u64, u64, u64, u64) =
            rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!((input, cached_input, cache_write, output), (1, 2, 3, 4));

        let bytes = rmp_serde::to_vec(&day).unwrap();
        assert_eq!(bytes[0], 0x93, "day, category, tokens");
        assert_eq!(rmp_serde::from_slice::<UsageDay>(&bytes).unwrap(), day);

        let answer = Usage { days: vec![day], unreadable: 2 };
        let bytes = rmp_serde::to_vec(&answer).unwrap();
        assert_eq!(bytes[0], 0x92, "the rows and what could not be read");
        assert_eq!(rmp_serde::from_slice::<Usage>(&bytes).unwrap(), answer);
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
        let blind = Usage { days: Vec::new(), unreadable: 2 };
        assert_ne!(rmp_serde::to_vec(&quiet).unwrap(), rmp_serde::to_vec(&blind).unwrap());
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
        let mut t = Tokens { input: u64::MAX, cached_input: 0, cache_write: 0, output: 5 };
        t.add(Tokens { input: 10, cached_input: 0, cache_write: 0, output: 5 });
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
