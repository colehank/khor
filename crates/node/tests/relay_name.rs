//! **A link from an older resident still reads** (UX-16's map). `Link`
//! crosses ipc as a positional rmp-serde array, and `relay` is its tail:
//! a resident built before the field answers three-field links, and the
//! face built after it must read them with no relay named — the map then
//! draws that relayed link straight, as before. `departed.rs` covers the
//! live field over a real relay; this covers the frame without it.

use khor_core::{Hop, Link};

/// The shape a resident older than `Link.relay` encodes.
#[derive(serde::Serialize)]
struct Before {
    hop: Hop,
    heard_ms: Option<u64>,
    rtt_ms: Option<u32>,
}

#[test]
fn an_older_residents_link_decodes_without_a_relay() {
    let old = khor_node::proto::encode(&Before {
        hop: Hop::Relay,
        heard_ms: Some(1200),
        rtt_ms: Some(41),
    })
    .unwrap();
    let read: Link = khor_node::proto::decode(&old).unwrap();
    assert_eq!(
        read,
        Link {
            hop: Hop::Relay,
            heard_ms: Some(1200),
            rtt_ms: Some(41),
            relay: None,
        }
    );
    // …and a link that names its relay round-trips with it.
    let now = Link {
        relay: Some("relay.example:3341".into()),
        ..read
    };
    let back: Link = khor_node::proto::decode(&khor_node::proto::encode(&now).unwrap()).unwrap();
    assert_eq!(back, now);
}
