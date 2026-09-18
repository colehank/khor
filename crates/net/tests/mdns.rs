//! Local-network address lookup (docs/NET.md 发现): two machines on one
//! segment should find each other without asking a server across the
//! internet where the other one is.
//!
//! **One test, and it is red.** There was a second one — "did the
//! registration take" — and it was deleted on 2026-09-18 rather than
//! kept: once the service moved onto the builder, the only thing that
//! assertion could read was a flag this file set to `true` right after
//! a successful bind. There is no edit anywhere that makes such a gate
//! red, so it was not a gate; it was a green line in a list of green
//! lines, which is the expensive kind of nothing.

use std::time::Duration;

use khor_net::endpoint::ALPN;
use tokio::time::timeout;

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
/// ⚠️ **Red on this machine as of 2026-09-18.** Every piece of the
/// chain was measured working and the chain still does not close, so
/// what follows is the evidence rather than a guess — the next person
/// should start from the one thing nobody has checked, not redo these:
///
/// - **Discovery works.** With tracing on, A sees B's announcement on
///   `_irohv1._udp.local` within a second and logs「adding endpoint to
///   Mdns address book」with B's three addresses.
/// - **Resolution works.** Calling `AddressLookup::resolve(B)` on A's
///   service by hand hands back exactly those addresses, tagged
///   `provenance: "mdns"`.
/// - **Those addresses are dialable.** Connecting A to B at each of
///   them explicitly — `127.0.0.1`, the LAN address, the tailnet
///   address, even the proxy's `198.18.0.1` — connects, all four.
/// - **Dialing B by key alone times out**, on `presets::Minimal` and on
///   khor's own bind, with the service on the builder and with it added
///   after the bind.
/// - Not the environment: raw multicast on this host sends 3 and
///   receives 3, on port 5353 and 15353 alike (one variable between the
///   arms — an earlier pair differed in two and pointed at the wrong
///   culprit). So not macOS's local-network permission, not a dead
///   multicast stack.
///
/// **The leading guess, and it is only that: both endpoints are on one
/// host**, so they advertise the *same three IPs* and differ only by
/// port. A dialer that drops discovered candidates matching a local
/// interface — an ordinary guard against dialing yourself — would
/// produce every line above. **Nobody has read iroh's dial path to
/// check**, and if it is true then this test cannot pass on one machine
/// and the thing to write instead needs two.
///
/// Until one of those happens, nothing in this file is evidence that
/// khor's mDNS resolves a peer anywhere.
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
///
/// **Registered on the builder, which is the only door that reaches the
/// dialer.** Adding the service to a bound endpoint
/// (`endpoint.address_lookup().add(..)`) leaves it discovering and
/// answering `resolve()` while connections never consult it — that is
/// what made this test red for a day, and the arm below is what tells
/// the two apart.
async fn bare(listening: bool) -> iroh::Endpoint {
    let mut builder =
        iroh::Endpoint::builder(iroh::endpoint::presets::Minimal).alpns(vec![ALPN.to_vec()]);
    if listening {
        builder = builder.address_lookup(iroh_mdns_address_lookup::MdnsAddressLookup::builder());
    }
    builder.bind().await.expect("a minimal bind should succeed")
}
