//! Wire frames between nodes. msgpack: field order is wire order, new
//! fields go at the tail with serde defaults, `skip_serializing_if` is
//! banned — a mid-frame optional desyncs the array on every older end.

use serde::{Deserialize, Serialize};

/// One frame's byte cap — above the sync payload cap
/// (`khor_sync::wire::MAX_BYTES` base64'd) with headroom.
pub const MAX_FRAME: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Join: burn the one-time token, record me, hand me the network.
    Pair { token: String, name: String, addrs: Vec<String> },
    /// One sync exchange over a named doc: `devices`, or `chat/<channel>`.
    Sync { doc: String, have: String, changes: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Pairing done on the issuer's side; `devices` is its table snapshot
    /// (base64) — merging it is what makes one pairing join the whole
    /// network.
    Paired { name: String, devices: String },
    Synced { version: String, changes: String, items: u64 },
    Refused { why: String },
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
