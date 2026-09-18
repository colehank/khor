//! Local-network address lookup (docs/NET.md 发现): two machines on one
//! segment should find each other without asking a server across the
//! internet where the other one is.
//!
//! **Two tests because one of them cannot prove what it looks like it
//! proves.** `endpoint::bind` builds on `presets::N0`, and that preset
//! carries n0's DNS/pkarr lookup. So "bound two endpoints, dialled one
//! by key alone, it connected" is a sentence with *two* explanations,
//! and the one it reads as is the one being tested — the classic arms
//! that agree about nothing. Splitting them:
//!
//! - [`the_bind_puts_local_lookup_on_the_endpoint`] asks only what khor
//!   controls — did the registration take. No network, always runs.
//! - [`local_lookup_alone_carries_a_dial_and_its_absence_does_not`]
//!   asks whether multicast actually resolves anything on this machine,
//!   and it builds endpoints with **no relay and no n0 lookup at all**
//!   so that mDNS is the only thing left that could answer. It carries
//!   its own torn-down arm: the same dial with mDNS *not* added must
//!   fail, or the first arm proved nothing.
//!
//! Neither proves the thing khor actually wants, which is **two
//! different machines on one router finding each other** — that needs
//! two machines, and nothing runnable from one of them substitutes for
//! it. What these hold is the floor under it: the wiring exists, and
//! multicast is not dead on this host.

use std::time::Duration;

use khor_net::endpoint::{self, ALPN};
use tokio::time::timeout;

/// Local lookup is on the endpoint khor binds.
///
/// A thin assertion on purpose: it is the one that goes false silently.
/// mDNS failing to start breaks nothing visible — the preset's lookup
/// answers exactly as it did before, the machine just goes on paying a
/// round trip to a server to find the machine next to it. There is no
/// symptom to notice, so the noticing has to be written down.
#[tokio::test]
async fn the_bind_puts_local_lookup_on_the_endpoint() {
    let endpoint = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    assert!(
        endpoint::mdns_live(),
        "bind must leave local-network lookup running on the endpoint"
    );
    endpoint.close().await;
}

/// With nothing else able to answer, mDNS carries a dial that has only
/// a key — and without it the same dial goes nowhere.
///
/// **`presets::Minimal` rather than khor's own bind, and that is the
/// whole point of this test.** Minimal has no relay and no n0 lookup,
/// so an endpoint holding an id and no addresses has exactly one way to
/// learn where the other one is. Run through `endpoint::bind` instead
/// and a green here would be indistinguishable from n0's pkarr doing
/// the work.
///
/// Ignored by default: it joins a multicast group, and a host where
/// multicast does not pass (a locked-down network, a sandbox) reds it
/// while nothing is wrong with khor.
/// `cargo test -p khor-net --test mdns -- --ignored`
///
/// ⚠️ **It is red on this machine as of 2026-09-18, and the reason is
/// not known.** Written down because the shape of the failure is the
/// expensive kind: the dial times out, which reads like "mdns is not
/// wired up" — and it is wired up, the registration test above is
/// green. What was measured on the day, so the next person does not
/// re-derive it:
///
/// - Raw multicast on this host is fine. Send-and-receive on group
///   224.0.0.251 got 3 of 3 back, **on port 5353 and on 15353 alike**
///   (one variable between the arms, which is the only reason that
///   sentence is worth anything — an earlier pair differed in port
///   *and* in whether anything was transmitting, and it pointed at the
///   wrong culprit).
/// - So it is not macOS's local-network permission and not a dead
///   multicast stack.
/// - This host's LAN carries no mDNS from anybody else: `dns-sd -B
///   _services._dns-sd._udp local.` lists only interface 1, which is
///   `lo0` — every service it found is this machine's own. en1 is a
///   172.23.0.0/17, the sort of managed network that isolates clients.
/// - Unknown: why the *same-process* pair above fails, which the LAN
///   cannot explain.
///
/// **So this gate is red for a reason nobody has found, not for a
/// reason somebody dismissed** — and until it is green, nothing in this
/// file is evidence that khor's mDNS resolves a peer anywhere.
#[tokio::test]
#[ignore = "要真多播,受本机网络环境影响——手动跑"]
async fn local_lookup_alone_carries_a_dial_and_its_absence_does_not() {
    // The arm that must connect: both ends announcing on the segment.
    let (server, client) = (bare(true).await, bare(true).await);
    let server_id = server.id();
    let echo = tokio::spawn(async move {
        let incoming = server.accept().await.expect("accept ended early");
        let conn = incoming.await.expect("handshake failed");
        let (mut send, mut recv) = conn.accept_bi().await.expect("no stream");
        let msg = recv.read_to_end(1024).await.expect("read failed");
        send.write_all(&msg).await.expect("write failed");
        send.finish().expect("finish failed");
        tokio::time::sleep(Duration::from_secs(3)).await;
    });
    // Nothing but the key — no ip, no relay. Twenty seconds because a
    // multicast answer is a round of announce-and-respond, not a lookup.
    let addr = iroh::EndpointAddr::new(server_id);
    let conn = timeout(Duration::from_secs(20), client.connect(addr, ALPN))
        .await
        .expect("a local dial must not hang")
        .expect("mdns should have resolved the peer on this segment");
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    send.write_all(b"over the wire we share").await.unwrap();
    send.finish().unwrap();
    let back = recv.read_to_end(1024).await.unwrap();
    assert_eq!(back, b"over the wire we share", "the echo should come back");
    echo.abort();
    client.close().await;

    // The torn-down arm. Same endpoints, same dial, mdns not added — if
    // this connects, something other than mdns answered above and the
    // green up there is about that something.
    let (deaf_server, deaf_client) = (bare(false).await, bare(false).await);
    let deaf_id = deaf_server.id();
    let addr = iroh::EndpointAddr::new(deaf_id);
    let nowhere = timeout(Duration::from_secs(10), deaf_client.connect(addr, ALPN)).await;
    assert!(
        nowhere.is_err() || nowhere.unwrap().is_err(),
        "with no lookup of any kind, a key alone must not reach a peer"
    );
    deaf_client.close().await;
    deaf_server.close().await;
}

/// An endpoint with nothing on it but, optionally, local lookup.
async fn bare(listening: bool) -> iroh::Endpoint {
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .expect("a minimal bind should succeed");
    if listening {
        let mdns = iroh_mdns_address_lookup::MdnsAddressLookup::builder()
            .build(endpoint.id())
            .expect("mdns should start");
        endpoint
            .address_lookup()
            .expect("a live endpoint has lookup services")
            .add(mdns);
    }
    endpoint
}
