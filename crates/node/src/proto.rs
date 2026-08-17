//! Wire frames between nodes. msgpack: field order is wire order, new
//! fields go at the tail with serde defaults, `skip_serializing_if` is
//! banned — a mid-frame optional desyncs the array on every older end.

use serde::{Deserialize, Serialize};

/// One frame's byte cap — above the sync payload cap
/// (`khor_sync::wire::MAX_BYTES` base64'd) with headroom.
pub const MAX_FRAME: usize = 512 * 1024;

/// One payload slice per request: stays under [`MAX_FRAME`] and makes
/// resume-by-offset free. Room is left for the msgpack envelope.
pub const SLICE: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Join: burn the one-time token, record me, hand me the network.
    Pair { token: String, name: String, addrs: Vec<String> },
    /// One sync exchange over a named doc: `devices`, or `chat/<channel>`.
    Sync { doc: String, have: String, changes: String },
    /// One slice of an offered payload, content-addressed. The offerer
    /// serves bytes only after the far user approved the pull — the
    /// approval IS this request arriving (docs/SESSION.md 传输).
    Fetch { digest: String, offset: u64 },
    /// The rows only the asked device can derive (its transfer faces;
    /// live kinds later). Chat rows are excluded — every device derives
    /// those from the CRDT itself, and duplicates would collide.
    Sessions,
    /// Run an action on a session this device executes for. Handlers
    /// never re-route an incoming Act — what is not theirs to run is
    /// refused, or two serves could bounce one forever.
    Act { session: String, action: String },
    /// What that machine has spent, by day and by vendor
    /// (`khor_core::Usage`).
    ///
    /// An op rather than a replicated document, for a reason the vitals
    /// op does not share: this is **derived from files khor only reads**,
    /// so it is re-derivable at any moment on the machine that has them
    /// and meaningless to copy anywhere else. What the asker keeps is a
    /// cache with the moment it arrived, so an unreachable machine shows
    /// its last answer and how old it is rather than a current-looking
    /// number.
    ///
    /// A khor that predates it answers [`Response::Refused`] — the name
    /// is on the wire, so an unknown op is a refusal and never a misread
    /// neighbour.
    Usage,
    /// What that machine is doing right now (CPU, memory, disk).
    ///
    /// An op rather than a field in the device document, because a
    /// reading is true for seconds — see `khor_core::Vitals`. A khor that
    /// predates it answers [`Response::Refused`] (the name is on the
    /// wire, so an unknown op is a refusal and not a misread neighbour),
    /// which the asking side treats as "no vitals" and nothing else.
    Vitals,
    /// One directory of the asked machine, for the files landing.
    /// `path` must be absolute; empty means that machine's home
    /// directory. Answered to any paired device without a permission
    /// step on purpose — this is one person's network, and what 待批
    /// guards is a payload's bytes, never the looking (docs/NET.md
    /// 入网即全信). A khor that predates it answers
    /// [`Response::Refused`], read as "that machine cannot list".
    Ls { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Pairing done on the issuer's side; `devices` is its table snapshot
    /// (base64) — merging it is what makes one pairing join the whole
    /// network.
    Paired { name: String, devices: String },
    Synced { version: String, changes: String, items: u64 },
    Refused { why: String },
    /// One slice. `total` rides on every slice so the fetcher can show
    /// progress without a separate stat round; end = offset + len == total.
    Slice { total: u64, bytes: serde_bytes::ByteBuf },
    SessionRows { rows: Vec<khor_core::Session> },
    /// An Act ran to completion; for accept, bytes moved.
    Acted { moved: u64 },
    /// One reading, taken when this frame was built.
    Vitals { vitals: khor_core::Vitals },
    /// What that machine has spent. See [`Request::Usage`].
    Usage { usage: khor_core::Usage },
    /// One directory, already ordered — directories first, each half by
    /// name — because the screen paints and never re-sorts (docs/UX.md
    /// 状态呈现). `truncated` is the no-silent-caps rule on the wire: a
    /// directory bigger than the cap says so instead of looking whole.
    Dir { entries: Vec<DirEntry>, truncated: bool },
}

/// One row of a listed directory. Every field required; whatever a
/// later khor wants to add rides the tail with a serde default, never
/// the middle (this file's field discipline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub dir: bool,
    /// Bytes; 0 for directories — a directory's byte size answers a
    /// question nobody browsing is asking.
    pub size: u64,
    /// Modified, ms since epoch. 0 when the machine could not say.
    pub at_ms: u64,
}

pub fn encode<T: Serialize>(t: &T) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec(t).map_err(khor_catalog::msg::cant_encode_frame)
}

pub fn decode<T: for<'a> Deserialize<'a>>(bytes: &[u8]) -> Result<T, String> {
    rmp_serde::from_slice(bytes).map_err(khor_catalog::msg::cant_decode_frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_and_names_its_op() {
        let req = Request::Sync {
            doc: "devices".into(),
            have: "aGF2ZQ".into(),
            changes: String::new(),
        };
        let bytes = encode(&req).unwrap();
        // The op name itself is on the wire: renaming a variant is a
        // protocol change and must land here first, consciously.
        assert!(
            bytes.windows(4).any(|w| w == b"Sync"),
            "the frame must carry the op name"
        );
        let back: Request = decode(&bytes).unwrap();
        match back {
            Request::Sync { doc, have, changes } => {
                assert_eq!((doc.as_str(), have.as_str(), changes.as_str()), ("devices", "aGF2ZQ", ""));
            }
            other => panic!("decoded wrong: {other:?}"),
        }
    }

    /// **A new op must be unreadable to an older peer, not readable as a
    /// different one.** That is the whole safety of adding one: the op
    /// name travels, so a khor that never heard of `Vitals` fails to
    /// decode and answers `Refused`, which the caller ignores. Were ops
    /// numbered instead, appending would be safe and inserting would
    /// silently turn one op into its neighbour — the failure this asserts
    /// cannot happen.
    ///
    /// The older peer is spelled out here as its own enum rather than
    /// mimicked, because a decoder built from today's type would be
    /// testing today's type against itself.
    ///
    /// **Both ops added since are checked**, not just the newest: the
    /// enum below is the one khor shipped before either existed, so
    /// `Vitals` proves the rule held once and `Usage` proves it still
    /// holds — and an op added without touching this test would be the
    /// one nobody checked.
    #[test]
    fn an_op_an_older_peer_never_heard_of_is_refused_not_mistaken() {
        #[derive(Debug, Deserialize)]
        enum OlderRequest {
            Pair { token: String, name: String, addrs: Vec<String> },
            Sync { doc: String, have: String, changes: String },
            Fetch { digest: String, offset: u64 },
            Sessions,
            Act { session: String, action: String },
        }

        for (op, name) in [
            (Request::Vitals, &b"Vitals"[..]),
            (Request::Usage, &b"Usage"[..]),
            (Request::Ls { path: String::new() }, &b"Ls"[..]),
        ] {
            let bytes = encode(&op).unwrap();
            assert!(
                bytes.windows(name.len()).any(|w| w == name),
                "the op name must be on the wire, or this proves nothing about names"
            );
            let refused = decode::<OlderRequest>(&bytes);
            assert!(refused.is_err(), "an older peer read {op:?} as {refused:?}");
        }

        // …and the same peer's own ops still decode here, so the
        // assertion above is about the new name and not about the two
        // enums having drifted apart in some other way.
        let theirs = encode(&Request::Sessions).unwrap();
        assert!(matches!(decode::<Request>(&theirs), Ok(Request::Sessions)));
        assert!(matches!(decode::<OlderRequest>(&theirs), Ok(OlderRequest::Sessions)));
    }

    /// A spending answer survives the round trip whole, **including the
    /// count of what could not be read**.
    ///
    /// That field is the one most easily lost on the way: an answer whose
    /// rows all arrive looks complete, and the only thing saying otherwise
    /// is a number that is usually zero. So the frame carrying a non-zero
    /// one is sent here on purpose.
    #[test]
    fn a_spending_answer_crosses_inside_its_response() {
        let sent = khor_core::Usage {
            days: vec![khor_core::UsageDay {
                day: "2026-08-17".to_owned(),
                category: "claude".to_owned(),
                tokens: khor_core::Tokens {
                    input: 1,
                    cached_input: 2,
                    cache_write: 3,
                    output: 4,
                },
            }],
            unreadable: 5,
        };
        let bytes = encode(&Response::Usage { usage: sent.clone() }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Usage { usage } => assert_eq!(usage, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }

    /// A reading survives the round trip whole. The frame is a response,
    /// so the guard against reordering `Vitals`' own fields lives with
    /// the struct (`khor_core`); what this covers is the envelope.
    #[test]
    fn a_reading_crosses_inside_its_response() {
        let sent = khor_core::Vitals {
            cpu_pct: 42.5,
            cores: 10,
            mem: khor_core::Fill { used: 3, total: 4 },
            disk: Some(khor_core::Fill { used: 5, total: 6 }),
            gpu: Some(khor_core::Gpu { util_pct: 12.0, cards: 1, mem: None }),
        };
        let bytes = encode(&Response::Vitals { vitals: sent }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Vitals { vitals } => assert_eq!(vitals, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }
}
