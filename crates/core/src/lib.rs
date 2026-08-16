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
