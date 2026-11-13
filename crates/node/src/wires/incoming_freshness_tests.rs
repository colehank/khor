//! Packet count since a reactor tick is not the packet's actual age.

use super::*;

const ALPN: &[u8] = b"khor/incoming-freshness";

async fn endpoint() -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_reachability_uses_the_real_packet_age_without_waiting_for_a_tick() {
    let (a, b) = (endpoint().await, endpoint().await);
    let (client, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(a.connect(b.addr(), ALPN), async {
            b.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (client, incoming) = (client.unwrap(), incoming.unwrap());
    let w = Wires::default();
    w.serving(&incoming);
    tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            async {
                let (mut send, mut recv) = client.open_bi().await.unwrap();
                send.write_all(b"packet-age").await.unwrap();
                send.finish().unwrap();
                assert_eq!(recv.read_to_end(32).await.unwrap(), b"packet-age");
            },
            async {
                let (mut send, mut recv) = incoming.accept_bi().await.unwrap();
                let bytes = recv.read_to_end(32).await.unwrap();
                send.write_all(&bytes).await.unwrap();
                send.finish().unwrap();
            }
        );
    })
    .await
    .unwrap();
    assert_eq!(khor_net::path::classify(&incoming), PathKind::Direct);
    let now = Instant::now();
    let patience = Patience::for_rtt(rtt_of(&incoming)).keepalive_missed;
    {
        let mut served = w.served.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(served.len(), 1);
        assert_ne!(served[0].rx, heard_count(&incoming));
        // Only the reactor's cached timestamp is old. The actual last
        // packet timestamp remains the one recorded by the real QUIC path.
        served[0].heard = now - patience - Duration::from_secs(1);
    }
    let peer = a.id().to_string();
    let fresh_without_tick = !w.incoming_for_hop_at(&peer, now).is_empty();
    // This is a pure snapshot-age query at an explicit future instant,
    // not a simulated OS pause or a change to Iroh's running clock.
    let expired_at = now + patience + Duration::from_secs(1);
    let expired_packet_rejected = w.incoming_for_hop_at(&peer, expired_at).is_empty();
    let wire_open = client.close_reason().is_none() && incoming.close_reason().is_none();
    client.close(0u32.into(), b"packet-age fixture complete");
    incoming.close(0u32.into(), b"packet-age fixture complete");
    tokio::join!(a.close(), b.close());
    assert!(wire_open);
    assert!(fresh_without_tick, "real new packets need no reactor tick");
    assert!(
        expired_packet_rejected,
        "changed counters do not make an old packet recent"
    );
}
