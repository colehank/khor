//! Local-network address lookup (docs/NET.md 发现): two machines on one
//! segment should find each other without asking a server across the
//! internet where the other one is.
//!
//! **Three tests, and they are three different claims.** One is green
//! and two are red, and which is which is the useful part:
//!
//! - [`what_mdns_hears_becomes_a_dial_candidate`] — **green**, and it is
//!   the half khor owns: what the wire answers ends up in the list khor
//!   dials from, at the port that machine is listening on. No connection
//!   is attempted, so nothing in it rests on the two below.
//! - [`local_lookup_alone_carries_a_dial_and_its_absence_does_not`] —
//!   red. Asks whether **iroh** resolves a bare key from mDNS.
//! - [`khor_dials_a_machine_it_only_heard_on_the_wire`] — red. Asks
//!   whether a dial carrying those candidates arrives.
//!
//! **There was a fourth, "did the registration take", and it was deleted
//! on 2026-09-18 rather than kept**: once the service moved onto the
//! builder, the only thing that assertion could read was a flag set to
//! `true` right after a successful bind. No edit anywhere makes such a
//! gate red, so it was not a gate — it was a green line in a list of
//! green lines, which is the expensive kind of nothing.

use std::time::Duration;

use khor_net::endpoint::{self, ALPN};
use tokio::time::timeout;

/// **What khor does with what it heard: the wire's answer becomes a dial
/// candidate.**
///
/// Deterministic — no connection is attempted. It asserts the merge:
/// `dial_addr` hands back the address mDNS heard, at a port that machine
/// actually listens on, with no relay road invented alongside it.
///
/// **The control comes first so it cannot be written to suit the
/// result**: a key nobody announced must pick up no roads at all.
/// Without that arm,「the dial carried an address」is also true of a
/// `dial_addr` that makes one up.
///
/// Ignored only because it needs real multicast:
/// `cargo test -p khor-net --test mdns -- --ignored`
#[tokio::test(flavor = "multi_thread")]
#[ignore = "要真多播,受本机网络环境影响——手动跑"]
async fn what_mdns_hears_becomes_a_dial_candidate() {
    let server = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    // **A second endpoint, and it is not decoration**: a machine's own
    // service announces but never reports itself, so with one endpoint
    // the map stays empty and this test reds saying「mdns heard
    // nothing」— which would be true and about nothing. The listener is
    // what turns an announcement into something heard.
    let listener = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    let server_id = server.id();

    let ghost = iroh::SecretKey::generate().public().to_string();
    let empty = endpoint::dial_addr(&ghost, &[], &[]).expect("a ghost id still parses");
    assert_eq!(
        empty.ip_addrs().count(),
        0,
        "a machine nobody announced must not arrive with roads"
    );

    // Announce-and-respond is a round trip on the wire, not a lookup.
    let heard = timeout(Duration::from_secs(20), async {
        loop {
            if !endpoint::nearby(&server_id).is_empty() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .unwrap_or(false);
    assert!(heard, "mdns should have put this machine on the segment map");

    let addr = endpoint::dial_addr(&server_id.to_string(), &[], &[]).expect("the id should parse");
    let heard_at: Vec<String> = addr.ip_addrs().map(|a| a.to_string()).collect();
    assert!(!heard_at.is_empty(), "the dial should carry what mdns heard");
    assert_eq!(
        addr.relay_urls().count(),
        0,
        "and no relay road — nothing on this dial came from anywhere but the wire"
    );
    // The port is what catches a merge that read the right machine off
    // the wrong announcement: it has to be a port this endpoint is
    // actually listening on.
    let listening: Vec<u16> = server
        .addr()
        .addrs
        .iter()
        .filter_map(|t| match t {
            iroh::TransportAddr::Ip(sa) => Some(sa.port()),
            _ => None,
        })
        .collect();
    for road in &heard_at {
        let port = road
            .parse::<std::net::SocketAddr>()
            .expect("a road parses")
            .port();
        assert!(
            listening.contains(&port),
            "{road} is not a port this machine listens on ({listening:?})"
        );
    }
    server.close().await;
    listener.close().await;
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
/// ⚠️ **Red as of 2026-09-18. Every piece of the chain was measured
/// working and the chain still does not close**, so what follows is
/// evidence rather than a guess — start from what nobody has checked,
/// do not redo these:
///
/// - **Discovery works.** With tracing on, A sees B's announcement on
///   `_irohv1._udp.local` within a second and logs「adding endpoint to
///   Mdns address book」with B's addresses.
/// - **Resolution works, and iroh is told.** The dial logs `connecting …
///   ip_addresses=[]` and **4ms later** the service logs `sending
///   AddressLookupItem` with those addresses, `provenance: "mdns"`.
/// - **Those addresses are dialable.** Connecting to each of them
///   explicitly — loopback, the LAN address, the tailnet address, even
///   the proxy's `198.18.0.1` — connects, on `Minimal` and on khor's
///   own bind alike.
/// - **Dialing by key alone times out** in every combination tried.
/// - Not the environment: raw multicast on this host sends 3 and
///   receives 3, **on port 5353 and 15353 alike** (one variable between
///   the arms, which is the only reason that sentence is worth anything
///   — an earlier pair differed in port *and* in whether anything was
///   transmitting, and it pointed at the wrong culprit). So not macOS's
///   local-network permission, not a dead multicast stack.
///
/// **Two explanations were written down and both are disproved, by me,
/// hours apart. They are kept because the next person will think of
/// them too:**
///
/// - ~~「the dialer never consults the lookup」~~ — false, see the 4ms
///   above.
/// - ~~「both endpoints are on one host, so a self-address guard drops
///   the candidates」~~ — false. The guard is real
///   (`socket/remote_map/remote_state.rs`,「not sending datagram to our
///   own address」) and it compares the **whole socket address**; A and
///   B share IPs but differ by port, so nothing is dropped.
///
/// So the question is narrow and upstream: iroh 1.0.3 is handed the
/// right addresses a few milliseconds into the dial and does nothing
/// observable with them — no path, no holepunch, no error, until the
/// caller's timeout. Whoever picks this up reads what the remote-state
/// actor does with an item that arrives *after* `connect` began (that
/// file's TODO points at n0-computer/iroh#4280), or tries it on two
/// real machines — the case this test stands in for.
#[tokio::test]
#[ignore = "要真多播,受本机网络环境影响——手动跑"]
async fn local_lookup_alone_carries_a_dial_and_its_absence_does_not() {
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

    // The torn-down arm. Same dial, mdns not registered — if this
    // connects, something other than mdns answered above and the green
    // up there is about that something.
    let (deaf_server, deaf_client) = (bare(false).await, bare(false).await);
    let addr = iroh::EndpointAddr::new(deaf_server.id());
    let nowhere = timeout(Duration::from_secs(10), deaf_client.connect(addr, ALPN)).await;
    assert!(
        nowhere.is_err() || nowhere.unwrap().is_err(),
        "with no lookup of any kind, a key alone must not reach a peer"
    );
    deaf_client.close().await;
    deaf_server.close().await;
}

/// **A dial reaches a machine khor only ever heard about on the wire.**
///
/// Not a restatement of the test above. That one asks whether *iroh*
/// resolves a bare key; this one asks whether *khor* gets there anyway,
/// carrying the candidates it supplies itself (`dial_addr`). The two can
/// differ — and today they do not: this is red as well, which is itself
/// the finding.
///
/// ⚠️ **Everything up to the connection is green**, and the test at the
/// head of this file proves that as its own claim: the address is in the
/// dial, at the right port, with no relay road. The dial still times
/// out, and so do single-candidate dials made from the same pair of
/// endpoints — **while the same shape of dial from a freshly bound pair
/// connects**. That inconsistency is not explained and it is the thread
/// to pull.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "要真多播,受本机网络环境影响——手动跑"]
async fn khor_dials_a_machine_it_only_heard_on_the_wire() {
    let server = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    let client = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    let server_id = server.id();

    let echo = tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            let Ok(conn) = incoming.await else { continue };
            let Ok((mut send, mut recv)) = conn.accept_bi().await else {
                continue;
            };
            let msg = recv.read_to_end(1024).await.unwrap_or_default();
            let _ = send.write_all(&msg).await;
            let _ = send.finish();
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });

    let heard = timeout(Duration::from_secs(20), async {
        loop {
            if !endpoint::nearby(&server_id).is_empty() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .unwrap_or(false);
    assert!(heard, "mdns should have put the server on the segment map");

    // **No stored hints and no relay roads** — every road on this dial
    // came from the wire, or a green here would be about the relay.
    let addr = endpoint::dial_addr(&server_id.to_string(), &[], &[]).expect("the id should parse");
    let conn = timeout(Duration::from_secs(20), client.connect(addr, ALPN))
        .await
        .expect("a dial on wire-found roads must not hang")
        .expect("it should connect on the address mdns heard");
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    send.write_all(b"found you on the wire").await.unwrap();
    send.finish().unwrap();
    let back = recv.read_to_end(1024).await.unwrap();
    assert_eq!(back, b"found you on the wire", "the echo should come back");

    echo.abort();
    client.close().await;
}

/// An endpoint with nothing on it but, optionally, local lookup.
///
/// **Registered on the builder, which is the door that reaches the
/// dialer.** Adding the service to an already-bound endpoint
/// (`endpoint.address_lookup().add(..)`, which is what the crate's
/// README shows) is a different wiring; both were tried and both end in
/// the same timeout, but this is the documented one.
async fn bare(listening: bool) -> iroh::Endpoint {
    let mut builder =
        iroh::Endpoint::builder(iroh::endpoint::presets::Minimal).alpns(vec![ALPN.to_vec()]);
    if listening {
        builder = builder.address_lookup(iroh_mdns_address_lookup::MdnsAddressLookup::builder());
    }
    builder.bind().await.expect("a minimal bind should succeed")
}
