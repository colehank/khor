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
    /// What that machine is doing right now (CPU, memory, disk).
    ///
    /// An op rather than a field in the device document, because a
    /// reading is true for seconds — see `khor_core::Vitals`. A khor that
    /// predates it answers [`Response::Refused`] (the name is on the
    /// wire, so an unknown op is a refusal and not a misread neighbour),
    /// which the asking side treats as "no vitals" and nothing else.
    Vitals,
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

        let bytes = encode(&Request::Vitals).unwrap();
        assert!(
            bytes.windows(6).any(|w| w == b"Vitals"),
            "the op name must be on the wire, or this proves nothing about names"
        );
        let refused = decode::<OlderRequest>(&bytes);
        assert!(refused.is_err(), "an older peer read it as {refused:?}");

        // …and the same peer's own ops still decode here, so the
        // assertion above is about the new name and not about the two
        // enums having drifted apart in some other way.
        let theirs = encode(&Request::Sessions).unwrap();
        assert!(matches!(decode::<Request>(&theirs), Ok(Request::Sessions)));
        assert!(matches!(decode::<OlderRequest>(&theirs), Ok(OlderRequest::Sessions)));
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
        };
        let bytes = encode(&Response::Vitals { vitals: sent }).unwrap();
        match decode::<Response>(&bytes).unwrap() {
            Response::Vitals { vitals } => assert_eq!(vitals, sent),
            other => panic!("decoded wrong: {other:?}"),
        }
    }
}
