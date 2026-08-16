//! The session primitive: every interaction answers five questions
//! (docs/SESSION.md). Pure data — no IO, no async.
//!
//! Wire discipline for everything serialized here: msgpack structs are
//! positional arrays, so field order is wire order, new fields go at the
//! tail with serde defaults, and `skip_serializing_if` is banned — a
//! mid-frame optional desyncs the array on every older end.

use serde::{Deserialize, Serialize};

pub mod avatar;

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
    pub const CHAT: &str = "chat";
    pub const TRANSFER: &str = "transfer";
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
/// **There is no GPU field, and that is a capability gap rather than a
/// data one.** `sysinfo` — the process table khor already carries — has
/// no GPU API at all: the only occurrences of the word in its source are
/// a macOS thermal sensor key and unrelated FFI. A field that is
/// structurally always empty is worse than no field, because it reads as
/// a machine that failed to report rather than a khor that cannot ask.
/// Adding it means adding a dependency or shelling out per platform, and
/// that is its own decision.
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
    fn the_session_frame_is_a_fixed_order_array_of_seven() {
        let session = a_session("shell");
        let bytes = rmp_serde::to_vec(&session).unwrap();
        // 0x97 = msgpack fixarray of 7. Adding, removing, or reordering a
        // field must land here first, consciously. It was six until
        // `category` was appended at the tail (never in the middle: a
        // mid-frame optional desyncs the array on every older end).
        assert_eq!(bytes[0], 0x97);
        let (id, kind, title, home, state, unread, category): (
            SessionId,
            Kind,
            String,
            DeviceId,
            StateStamp,
            u64,
            Option<String>,
        ) = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(
            (id, kind, title, home, state, unread, category),
            (
                session.id,
                session.kind,
                session.title,
                session.home,
                session.state,
                session.unread,
                session.category
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
        };
        let bytes = rmp_serde::to_vec(&v).unwrap();
        assert_eq!(bytes[0], 0x94, "four fields today");
        assert_eq!(rmp_serde::from_slice::<Vitals>(&bytes).unwrap(), v);
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
