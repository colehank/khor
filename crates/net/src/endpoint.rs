//! Binding, dialing, and the pairing ticket.

use anyhow::{Context, Result};
use khor_catalog::msg;

/// One protocol, one ALPN. Version bumps mean a new string.
pub const ALPN: &[u8] = b"khor/0";

/// The borrow tunnel (docs/NET.md 借网) runs on its own ALPN, not the
/// main one: `khor/0` is strict request-reply (read a frame to FIN,
/// answer, close), and a long-lived duplex byte pipe cannot live inside
/// that handler. Both ALPNs bind on the same endpoint; the accept loop
/// routes by which one a connection speaks.
pub const TUNNEL_ALPN: &[u8] = b"khor-tunnel/0";

/// The Khor relay tier (docs/NET.md 中继): joins the n0 defaults on
/// every bind, so a network that cannot reach n0 falls to it without
/// anyone flipping a switch. Currently mandala's aliyun box, on loan —
/// khor's own fleet replaces this list, not the mechanism.
pub const KHOR_RELAYS: &[&str] = &["http://39.97.7.248:3340"];

/// Every relay this machine should use: the Khor tier plus `KHOR_RELAY`
/// (comma-separated, the self-hosted tier). Deduplicated, order kept.
pub fn configured_relays() -> Vec<String> {
    let mut out: Vec<String> = KHOR_RELAYS.iter().map(|s| s.to_string()).collect();
    if let Ok(own) = std::env::var("KHOR_RELAY") {
        for url in own.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if !out.iter().any(|have| have == url) {
                out.push(url.to_string());
            }
        }
    }
    out
}

/// Machines mDNS has seen on this segment, and where it saw them.
///
/// **khor keeps its own copy of what the lookup found, and that is not
/// redundancy.** iroh is handed the same answer — measured, four
/// milliseconds into a dial, tagged `provenance: "mdns"` — and on
/// iroh 1.0.3 it does nothing observable with one that arrives after
/// `connect` began (the whole account is on `tests/mdns.rs`). khor does
/// not have to wait for that: every dial it makes already carries
/// candidates it supplies itself, from the device table. This map is
/// one more source for that list, so a machine found on the wire is
/// dialled the same way a machine read out of the table is.
///
/// Process-global because the map describes **the segment**, not an
/// endpoint: two endpoints in one process are on the same wire and see
/// the same neighbours. It is a cache of something that expires on its
/// own — an entry outliving the machine that announced it costs one
/// dead candidate in a list that already tolerates them
/// ([`dial_addr`]), which is why nothing here has a timer.
static NEARBY: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<iroh::EndpointId, Vec<std::net::SocketAddr>>>,
> = std::sync::LazyLock::new(Default::default);

/// Where mDNS last saw this machine, as dial hints. Empty when it has
/// not been seen — which is the ordinary case off a shared segment.
pub fn nearby(id: &iroh::EndpointId) -> Vec<String> {
    NEARBY
        .lock()
        .map(|m| {
            m.get(id)
                .map(|v| v.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

/// Hands an already-built lookup service to [`iroh::endpoint::Builder`].
///
/// The builder is the door that reaches the dialer, and it wants a
/// *builder*; khor needs the built service too, to read its event
/// stream. This passes the one it made rather than letting iroh make a
/// second one — two mdns services on one endpoint would announce the
/// same key twice on the same wire.
#[derive(Debug)]
struct Prebuilt(iroh_mdns_address_lookup::MdnsAddressLookup);

impl iroh::address_lookup::AddressLookupBuilder for Prebuilt {
    fn into_address_lookup(
        self,
        _endpoint: &iroh::Endpoint,
    ) -> Result<
        impl iroh::address_lookup::AddressLookup,
        iroh::address_lookup::AddressLookupBuilderError,
    > {
        Ok(self.0)
    }
}

/// Keeps [`NEARBY`] current from one service's event stream.
///
/// Discovered replaces rather than merges: the announcement is the whole
/// truth about where that machine is *now*, and a union would keep a
/// port it has since moved off. Expired removes — a neighbour that left
/// the wire should stop being dialled as if it were on it.
fn follow(mdns: iroh_mdns_address_lookup::MdnsAddressLookup) {
    tokio::spawn(async move {
        use n0_future::StreamExt;
        let mut events = mdns.subscribe().await;
        while let Some(event) = events.next().await {
            let Ok(mut map) = NEARBY.lock() else { return };
            match event {
                iroh_mdns_address_lookup::DiscoveryEvent::Discovered { endpoint_info, .. } => {
                    let seen: Vec<std::net::SocketAddr> = endpoint_info
                        .data
                        .addrs()
                        .filter_map(|a| match a {
                            iroh::TransportAddr::Ip(sa) => Some(*sa),
                            _ => None,
                        })
                        .filter(|sa| !unroutable(&iroh::TransportAddr::Ip(*sa)))
                        .collect();
                    if seen.is_empty() {
                        map.remove(&endpoint_info.endpoint_id);
                    } else {
                        map.insert(endpoint_info.endpoint_id, seen);
                    }
                }
                iroh_mdns_address_lookup::DiscoveryEvent::Expired { endpoint_id } => {
                    map.remove(&endpoint_id);
                }
                // `DiscoveryEvent` is `#[non_exhaustive]`. An event this
                // build has never heard of leaves the map as it is —
                // the entries in it were put there by events this build
                // does understand, and dropping them on a word we
                // cannot read would be forgetting a neighbour because
                // somebody added a sentence.
                _ => {}
            }
        }
    });
}

/// Binds this machine's endpoint. One live endpoint per key — a second
/// bind with the same key knocks the first off the relay, and both stop
/// working (learned in production).
///
/// `extra_relays` join the n0 defaults; iroh probes them all and homes
/// on the closest reachable one. That probing IS the automatic tier
/// fallback: where n0 is unreachable, the extra relay is the only probe
/// that answers. Self-hosted relays are assumed to run no QUIC address
/// discovery — assuming it costs a dead-port wait on every connection.
pub async fn bind(secret: iroh::SecretKey, extra_relays: &[String]) -> Result<iroh::Endpoint> {
    bind_with(secret, extra_relays, false).await
}

/// [`bind`], plus the switch that makes `extra_relays` the **whole**
/// relay map instead of an addition to n0's.
///
/// A separate entry point rather than a third parameter on `bind`:
/// `bind` has twenty-odd call sites and all but one of them want the
/// default, so widening it would put every test in this workspace in a
/// red window to spell out `false`.
///
/// **`exclusive` with an empty list is ignored, not honoured.** Honouring
/// it means [`iroh::endpoint::RelayMode::Disabled`] — a machine with no
/// relay at all, reachable only where a hole punch lands, which is the
/// one outcome nobody asks for by leaving a box empty. The refusal that
/// makes this unreachable lives at the write door (`Node::set_relays`);
/// this line is the second net, for a hand-edited file.
pub async fn bind_with(
    secret: iroh::SecretKey,
    extra_relays: &[String],
    exclusive: bool,
) -> Result<iroh::Endpoint> {
    // Built here rather than by iroh so that `follow` can read what it
    // finds; registered on the builder all the same (`Prebuilt`).
    let mdns = iroh_mdns_address_lookup::MdnsAddressLookup::builder()
        .build(secret.public())
        .context(msg::no_local_lookup())?;
    follow(mdns.clone());
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret)
        // Compiled-in Mozilla roots instead of the operating system's:
        // the system route reads the macOS Keychain, and on a Mac
        // operated over ssh that read hangs 25-40 seconds — every
        // process's FIRST bind paid it (measured 2026-08-20: khor's
        // own test binaries at 36-43s per process, while the verdict
        // hid because the hang is synchronous and froze single-thread
        // tokio timers with it, #73). These roots only verify outward
        // TLS (https relays, pkarr, DoH); iroh's own encryption never
        // touched the Keychain in the first place.
        .ca_tls_config(iroh::tls::CaTlsConfig::embedded())
        // Explicit public nameservers instead of "read the system's":
        // on macOS the system route goes through SCDynamicStore, whose
        // ApplicationBundleID lookup hung 40 SECONDS per process for
        // unbundled binaries — measured with `sample` on 2026-08-20,
        // the whole of #73's frozen-timer mystery (the hang is
        // synchronous, so it froze single-threaded tokio clocks too).
        // khor's own relays are IPs and need no DNS at all; these
        // servers only resolve n0's relay/lookup names where they are
        // reachable. The cost — a network whose ONLY nameserver is an
        // internal one loses n0 name resolution — is on the ledger.
        .dns_resolver(
            iroh::dns::DnsResolver::builder()
                .with_nameserver("223.5.5.5:53".parse().unwrap(), iroh::dns::DnsProtocol::Udp)
                .with_nameserver("8.8.8.8:53".parse().unwrap(), iroh::dns::DnsProtocol::Udp)
                .with_nameserver("1.1.1.1:53".parse().unwrap(), iroh::dns::DnsProtocol::Udp)
                .with_nameserver(
                    "[2400:3200::1]:53".parse().unwrap(),
                    iroh::dns::DnsProtocol::Udp,
                )
                .build(),
        )
        .alpns(vec![ALPN.to_vec(), TUNNEL_ALPN.to_vec()])
        // **On the builder, not on the bound endpoint — and that is the
        // whole difference between wired up and merely present.**
        // `endpoint.address_lookup().add(..)` after a bind (what the
        // crate's own README shows) leaves a service that discovers and
        // answers `resolve()` correctly while the dialer never consults
        // it: measured 2026-09-18, A saw B, `resolve(B)` handed back B's
        // three addresses with provenance "mdns", dialling B by key
        // alone timed out, and dialling the very same addresses by hand
        // connected. Every piece worked and the chain did not.
        .address_lookup(Prebuilt(mdns.clone()))
        // Same door, same reason: the filter that keeps this machine
        // from publishing an address nobody can route to belongs where
        // the services are assembled.
        .addr_filter(iroh::address_lookup::AddrFilter::new(|addrs| {
            if addrs.iter().any(unroutable) {
                std::borrow::Cow::Owned(addrs.iter().filter(|a| !unroutable(a)).cloned().collect())
            } else {
                std::borrow::Cow::Borrowed(addrs)
            }
        }));
    let extras: Vec<iroh::RelayConfig> = extra_relays
        .iter()
        .filter_map(|r| r.parse::<iroh::RelayUrl>().ok())
        .map(|url| {
            let mut c = iroh::RelayConfig::from(url);
            c.quic = None;
            c
        })
        .collect();
    if !extras.is_empty() {
        // Exclusive means the user's list *is* the map — n0's servers
        // are not in it and this machine stops touching them for relay.
        // (Their DNS/pkarr lookup is a different service and is still
        // on; taking that out is its own switch and nobody has asked.)
        let mut relays: Vec<std::sync::Arc<iroh::RelayConfig>> = if exclusive {
            Vec::new()
        } else {
            iroh::defaults::prod::default_relay_map().relays()
        };
        relays.extend(extras.into_iter().map(std::sync::Arc::new));
        builder = builder.relay_mode(iroh::endpoint::RelayMode::Custom(
            relays.into_iter().collect(),
        ));
    }
    Ok(builder.bind().await?)
}

/// Is this an address no peer could ever reach us on?
///
/// **One range, and it is here because it was measured, not because it
/// looked suspect.** On 2026-09-18 this machine's endpoint advertised
/// `198.18.0.1` alongside its real addresses: that is 198.18.0.0/15,
/// which RFC 2544 reserves for benchmarking and which local proxy tools
/// hand out as fake-IP on a tun interface. It is never a host another
/// machine can route to, so every peer that reads it spends probes on a
/// road that cannot exist — and worse, a peer that happens to run the
/// same proxy has a `198.18.0.1` of its own, so the probe does not go
/// nowhere, it goes to the wrong place.
///
/// **What is deliberately still published**, because each of these is a
/// road that does work somewhere:
/// - `100.64.0.0/10` — CGNAT, and also Tailscale's range. A peer on the
///   same tailnet reaches us there, and it is the best road when it is
///   available.
/// - `169.254.0.0/16` / `fe80::/10` — link-local, which is exactly the
///   same segment this machine might share with another.
/// - Every RFC 1918 private range — that is what a LAN looks like.
///
/// Loopback is left alone too: iroh already decides what to do with it,
/// and a second opinion here would be a rule nobody could find.
fn unroutable(addr: &iroh::TransportAddr) -> bool {
    match addr {
        iroh::TransportAddr::Ip(sa) => match sa.ip() {
            std::net::IpAddr::V4(v4) => {
                let o = v4.octets();
                o[0] == 198 && (o[1] == 18 || o[1] == 19)
            }
            std::net::IpAddr::V6(_) => false,
        },
        iroh::TransportAddr::Relay(_) => false,
        // `TransportAddr` is `#[non_exhaustive]`. A kind of road this
        // build has never heard of gets published, not hidden: the cost
        // of publishing one nobody can use is some wasted probes, and
        // the cost of hiding one that works is a machine that silently
        // stops being reachable the way it used to be.
        _ => false,
    }
}

/// A dial address: identity plus every road we know. iroh probes them in
/// parallel, picks by RTT, and keeps querying discovery on top — we only
/// supply the candidates it cannot look up. Both lists take both shapes:
/// `ip:port` becomes a direct road, `http(s)://…` a relay road — stored
/// hints carry them mixed. Unparseable entries are skipped, not fatal:
/// they come from tickets and stored hints, and one typo must not kill
/// the dial.
pub fn dial_addr(id_hex: &str, direct: &[String], relays: &[String]) -> Result<iroh::EndpointAddr> {
    let id: iroh::EndpointId = id_hex.parse().context(msg::not_a_machine_id_short())?;
    let mut addr = iroh::EndpointAddr::new(id);
    // **What mDNS saw goes in beside the stored hints, not instead of
    // them.** A machine on this segment is usually also in the device
    // table, at an address that was true the last time it wrote one
    // down; the announcement is where it is now. iroh probes the whole
    // list in parallel and keeps the one that answers first, so a stale
    // entry costs a probe and a fresh one wins the dial.
    //
    // It is merged **here**, at the one door every khor dial goes
    // through, so「我在同一个网段里」needs no caller to remember it.
    let nearby = nearby(&id);
    for road in direct.iter().chain(relays).chain(nearby.iter()) {
        if let Ok(sock) = road.parse::<std::net::SocketAddr>() {
            addr = addr.with_ip_addr(sock);
        } else if (road.starts_with("http://") || road.starts_with("https://"))
            && let Ok(url) = road.parse::<iroh::RelayUrl>()
        {
            addr = addr.with_relay_url(url);
        }
    }
    Ok(addr)
}

/// Which kinds of road this endpoint is **actually using** to reach a
/// remote, right now. `None` when iroh has no entry for it at all.
///
/// **Only the addresses iroh marks as in use are counted.** The list it
/// hands back also holds candidates that are merely known, and reading
/// one of those would report a direct road for a machine whose every
/// byte is going through a relay — the exact wrong answer to the
/// question this exists for ("do we still need the relay here?"). Same
/// judgment `probe.rs` already carries: what decides is the path that
/// was selected, not the mere existence of a direct one.
///
/// Names nothing: it answers in facts and the word is chosen where the
/// words live (`khor_core::Hop::of`). The third flag is a road in use
/// that is neither kind — iroh's address enum is open, and folding a
/// transport khor has no word for into "direct" or "relay" would be a
/// confident lie rather than an admission.
pub async fn roads_in_use(ep: &iroh::Endpoint, id_hex: &str) -> Option<RoadsInUse> {
    let id: iroh::EndpointId = id_hex.parse().ok()?;
    let info = ep.remote_info(id).await?;
    let mut seen = RoadsInUse::default();
    for a in info.addrs() {
        if !matches!(a.usage(), iroh::endpoint::TransportAddrUsage::Active) {
            continue;
        }
        match a.addr() {
            iroh::TransportAddr::Ip(_) => seen.direct = true,
            iroh::TransportAddr::Relay(_) => seen.relay = true,
            _ => seen.other = true,
        }
    }
    Some(seen)
}

/// The answer [`roads_in_use`] gives: which kinds are carrying traffic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoadsInUse {
    pub direct: bool,
    pub relay: bool,
    /// A road in use that is neither of the above.
    pub other: bool,
}

/// This endpoint's dialable addresses, as strings. Unspecified bind
/// addresses (0.0.0.0) map to loopback — right for the same-machine
/// case; LAN and NAT paths come from discovery.
pub fn local_addrs(ep: &iroh::Endpoint) -> Vec<String> {
    let mut out: Vec<String> = ep
        .addr()
        .addrs
        .iter()
        .filter_map(|t| match t {
            iroh::TransportAddr::Ip(s) => Some(s.to_string()),
            _ => None,
        })
        .collect();
    for s in ep.bound_sockets() {
        let mapped = if s.ip().is_unspecified() {
            format!("127.0.0.1:{}", s.port())
        } else {
            s.to_string()
        };
        if !out.contains(&mapped) {
            out.push(mapped);
        }
    }
    out
}

/// The pairing ticket: who to dial, where, and the one-time secret.
/// Our own JSON-in-base64 shape — the wire must not follow a library's
/// types across versions.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Ticket {
    /// Issuer's machine id (public key, hex).
    pub id: String,
    /// Issuer's name, for the accepting side to display.
    pub name: String,
    /// Direct address candidates.
    pub direct: Vec<String>,
    /// Relay URL candidates.
    pub relays: Vec<String>,
    /// One-time pairing secret; the issuer burns it on first use.
    pub token: String,
}

impl Ticket {
    pub fn encode(&self) -> Result<String> {
        use base64::Engine;
        let json = serde_json::to_vec(self)?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode(text: &str) -> Result<Ticket> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(text.trim())
            .context(msg::not_a_ticket_base64())?;
        serde_json::from_slice(&bytes).context(msg::not_a_ticket_content())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stored hints carry both road shapes mixed; garbage is skipped,
    /// never fatal — hints come from tickets and human-set env vars.
    #[test]
    fn a_hint_list_carries_both_roads_and_skips_garbage() {
        let id = iroh::SecretKey::generate().public().to_string();
        let addr = dial_addr(
            &id,
            &[
                "192.168.1.9:11204".into(),
                "http://39.97.7.248:3340".into(),
                "garbage".into(),
            ],
            &["https://r.example".into()],
        )
        .unwrap();
        let (mut ips, mut relays) = (0, 0);
        for a in &addr.addrs {
            match a {
                iroh::TransportAddr::Ip(_) => ips += 1,
                iroh::TransportAddr::Relay(_) => relays += 1,
                _ => {}
            }
        }
        assert_eq!(
            (ips, relays),
            (1, 2),
            "one direct road, two relay roads, no garbage"
        );
    }

    #[test]
    fn a_ticket_round_trips() {
        let t = Ticket {
            id: "aa".repeat(32),
            name: "alpha".into(),
            direct: vec!["127.0.0.1:4433".into()],
            relays: vec!["http://39.97.7.248:3340".into()],
            token: "deadbeef".into(),
        };
        assert_eq!(Ticket::decode(&t.encode().unwrap()).unwrap(), t);
        // Garbage is refused with the ticket wording, not a panic.
        // "not-a-ticket" happens to be valid url-safe base64, so it dies
        // at the content gate; either refusal is the ticket wording.
        let e = Ticket::decode("not-a-ticket").unwrap_err().to_string();
        assert!(
            e.contains(msg::not_a_ticket_content()) || e.contains(msg::not_a_ticket_base64()),
            "{e}"
        );
    }

    /// The publish filter drops the road nobody can take and keeps every
    /// road that works somewhere.
    ///
    /// **The keep list is the half that matters.** "198.18 is dropped"
    /// is satisfied by a filter that drops everything, and a machine
    /// that publishes no addresses at all still connects — over a relay
    /// — so nothing else in this codebase would have gone red. The
    /// neighbours are named one by one because each of them is a real
    /// road: a tailnet, a LAN, the same segment.
    #[test]
    fn only_the_unroutable_range_is_hidden() {
        let ip = |s: &str| iroh::TransportAddr::Ip(s.parse().unwrap());
        for road in [
            // Tailscale / CGNAT — this machine really is reachable here.
            ip("100.103.22.1:41641"),
            // A LAN.
            ip("172.23.250.213:53181"),
            ip("192.168.1.9:53181"),
            ip("10.0.0.4:53181"),
            // Link-local: the same segment, which is the case mDNS is
            // for.
            ip("169.254.7.7:53181"),
            // A public address.
            ip("48.47.144.30:40217"),
            // Relays are not IPs and are never this filter's business.
            iroh::TransportAddr::Relay("http://39.97.7.248:3340".parse().unwrap()),
        ] {
            assert!(
                !unroutable(&road),
                "{road:?} is a road that works somewhere"
            );
        }
        // 198.18.0.0/15 — RFC 2544 benchmarking, handed out by local
        // proxies on a tun interface. Both halves of the /15.
        for road in [ip("198.18.0.1:53181"), ip("198.19.255.254:53181")] {
            assert!(unroutable(&road), "{road:?} can never be another machine");
        }
        // The edges, so the /15 is a /15 and not a /16 or a /14.
        for road in [ip("198.17.255.255:1"), ip("198.20.0.0:1")] {
            assert!(!unroutable(&road), "{road:?} is outside the range");
        }
    }
}
