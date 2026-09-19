//! Local-network address lookup (docs/NET.md 发现): two machines on one
//! segment should find each other without asking a server across the
//! internet where the other one is.
//!
//! **Four tests: two pass here, two cannot pass here at all — and the
//! reason the last two cannot is the firewall, not the code.**
//!
//! Measured 2026-09-19, one variable between the arms — same binary,
//! same peer, same port, only the destination address differs, each
//! round a fresh pair of endpoints in a fresh process:
//!
//! ```text
//! 127.0.0.1        -> 连上了  连上了
//! 172.23.250.213   -> 超时    超时     (this machine's own LAN address)
//! 100.103.22.1     -> 超时    超时     (its tailnet address)
//! both of the above-> 超时    超时
//! ```
//!
//! `socketfilterfw` says the firewall is **enabled** and auto-allows
//! **signed** software only. A `cargo test` binary is unsigned, so
//! inbound UDP to it is dropped on every address except loopback — and
//! iroh publishes loopback only when there is no other interface, so
//! nothing mDNS ever announces is reachable from here.
//!
//! **This is the thing I twice announced I had ruled out.** The first
//! "not the firewall" came from a probe that dialled `127.0.0.1` first
//! and then called the next two dials green — they were reusing the
//! path the loopback dial had already opened. A control arm contaminated
//! by the arm before it, which is this repo's oldest false green wearing
//! a new hat.
//!
//! So:
//!
//! - [`what_mdns_hears_becomes_a_dial_candidate`] — **green**, the half
//!   khor owns: what the wire answers ends up in the list khor dials
//!   from, at the port that machine is listening on. It opens no
//!   connection, which is why the firewall cannot reach it.
//! - [`a_machine_heard_on_the_wire_answers_on_the_port_it_announced`] —
//!   **green**, and it closes the chain as far as this host allows:
//!   announce → hear → candidate list → dial → echo. It dials the
//!   announced **port** at loopback, because the announced *address* is
//!   the half the firewall eats; its own doc says what that costs.
//! - [`local_lookup_alone_carries_a_dial_and_its_absence_does_not`] and
//!   [`khor_dials_a_machine_it_only_heard_on_the_wire`] — **red here and
//!   unprovable here.** Both end in a connection over an address this
//!   host will not accept a packet on. Run them on two machines, or on
//!   one with the firewall off, and they are answering a real question;
//!   run them here and they answer the firewall's.
//!
//! ⚠️ **And this is a product finding, not only a test one.** The same
//! block stands between two real Macs: khor's own binary is unsigned
//! too, so a direct connection needs the「是否允许接受传入连接」dialog
//! to have been accepted on the receiving side. Nobody can click it over
//! ssh. `zh.toml`'s `web-face-down` already describes exactly this
//! failure — 「握手连得上,数据一个字节也不给」 — from the other end of
//! the same rope.
//!
//! **There was a fourth test, "did the registration take", deleted on
//! 2026-09-18 rather than kept**: once the service moved onto the
//! builder, the only thing it could read was a flag set to `true` right
//! after a successful bind. No edit anywhere makes such a gate red, so
//! it was not a gate — it was a green line in a list of green lines.

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
    assert!(
        heard,
        "mdns should have put this machine on the segment map"
    );

    let addr = endpoint::dial_addr(&server_id.to_string(), &[], &[]).expect("the id should parse");
    let heard_at: Vec<String> = addr.ip_addrs().map(|a| a.to_string()).collect();
    assert!(
        !heard_at.is_empty(),
        "the dial should carry what mdns heard"
    );
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
/// ⚠️ **Red here, and the file head says why: nothing mDNS announces is
/// an address this host will accept a packet on.** Not a khor fault and
/// not an iroh fault — run it on two machines.
///
/// What was measured along the way is worth keeping, because it says
/// the chain is sound right up to the wire and nobody should re-derive
/// it:
///
/// - **Discovery works.** With tracing on, A sees B's announcement on
///   `_irohv1._udp.local` within a second and logs「adding endpoint to
///   Mdns address book」with B's addresses.
/// - **Resolution works, and iroh is told.** The dial logs `connecting …
///   ip_addresses=[]` and **4ms later** the service logs `sending
///   AddressLookupItem` with those addresses, `provenance: "mdns"`.
/// - Multicast on this host is fine: send 3, receive 3, on port 5353
///   and 15353 alike.
///
/// **Two explanations were written down before the firewall was found,
/// and both are disproved. Kept, because the next person will think of
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
/// There is still one open question underneath the firewall, and it
/// needs a host that can answer its own packets before anyone can ask
/// it: does iroh act on an `AddressLookupItem` that lands *after*
/// `connect` began? (That file's TODO points at n0-computer/iroh#4280.)
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

/// **A machine heard on the wire answers on the port it announced.**
///
/// The chain end to end: announce → hear → into khor's candidate list →
/// dial → echo. Everything the merge exists for.
///
/// ⚠️ **One substitution, and it is the firewall's fault, not a
/// convenience.** The dial is made to the *port* mDNS announced, at
/// `127.0.0.1` rather than at the address it was announced on. The file
/// head has the measurement: on this host, inbound packets to an
/// unsigned binary are dropped on every address but loopback, so a dial
/// to the announced address cannot arrive no matter what khor does. The
/// port is the half that came off the wire and it is the half this test
/// is about — a wrong port, a stale announcement, a machine that has
/// since rebound, all still red it.
///
/// **What it therefore does not prove**: that a packet crosses a real
/// segment to another machine. Nothing runnable on one host proves
/// that, and [`khor_dials_a_machine_it_only_heard_on_the_wire`] is
/// where it gets proved the day there are two.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "要真多播,受本机网络环境影响——手动跑"]
async fn a_machine_heard_on_the_wire_answers_on_the_port_it_announced() {
    let server = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    let client = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .expect("bind should succeed");
    let server_id = server.id();
    // The ports this machine is really listening on, read before the
    // wire is consulted — the assertion below is that what came off the
    // wire is one of these, so it cannot be taken from the wire.
    let listening: Vec<u16> = server
        .addr()
        .addrs
        .iter()
        .filter_map(|t| match t {
            iroh::TransportAddr::Ip(sa) => Some(sa.port()),
            _ => None,
        })
        .collect();

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

    // Straight out of khor's own candidate list — no stored hint, no
    // relay, nothing but what the wire said.
    let from_the_wire = endpoint::dial_addr(&server_id.to_string(), &[], &[])
        .expect("the id should parse")
        .ip_addrs()
        .next()
        .copied()
        .expect("the dial should carry what mdns heard");
    assert!(
        listening.contains(&from_the_wire.port()),
        "{from_the_wire} is not a port this machine listens on ({listening:?})"
    );
    assert!(
        !from_the_wire.ip().is_loopback(),
        "the announcement should be a real address — a loopback one would make \
         the substitution below a no-op and this test a tautology"
    );

    let addr = iroh::EndpointAddr::new(server_id).with_ip_addr(std::net::SocketAddr::from((
        std::net::Ipv4Addr::LOCALHOST,
        from_the_wire.port(),
    )));
    let conn = timeout(Duration::from_secs(20), client.connect(addr, ALPN))
        .await
        .expect("a dial on the announced port must not hang")
        .expect("the machine that announced that port should answer on it");
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    send.write_all(b"found you on the wire").await.unwrap();
    send.finish().unwrap();
    let back = recv.read_to_end(1024).await.unwrap();
    assert_eq!(back, b"found you on the wire", "the echo should come back");

    echo.abort();
    client.close().await;
}

/// **A dial reaches a machine khor only ever heard about on the wire.**
///
/// Not a restatement of the test above. That one asks whether *iroh*
/// resolves a bare key; this one asks whether *khor* gets there anyway,
/// carrying the candidates it supplies itself (`dial_addr`). The two can
/// differ — and today they do not: this is red as well, which is itself
/// the finding.
///
/// ⚠️ **Everything up to the connection is green** — the test at the
/// head of this file proves that as its own claim: the address is in
/// the dial, at the right port, with no relay road. The connection then
/// fails for the reason in the file head, which is the firewall and not
/// this code. On two machines this is the test that says whether the
/// merge was worth making.
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
