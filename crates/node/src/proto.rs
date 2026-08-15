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
}

pub fn encode<T: Serialize>(t: &T) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec(t).map_err(|e| format!("编不出帧: {e}"))
}

pub fn decode<T: for<'a> Deserialize<'a>>(bytes: &[u8]) -> Result<T, String> {
    rmp_serde::from_slice(bytes).map_err(|e| format!("解不出帧: {e}"))
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
}
