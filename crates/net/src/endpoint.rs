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

/// The Khor relay tier (docs/NET.md 中继): the relays every khor binds
/// with unless this machine was told to use only its own — each with
/// the shared key its relay asks for.
///
/// **Two, because one cannot be reached by everyone** (2026-10-01,
/// measured before either went in):
///
/// - **Beijing**, plain HTTP on 3340. For machines whose network does
///   not leave the mainland — the lab's three reach it in 5 ms and
///   cannot reach Hong Kong at all (a campus network that carries
///   domestic traffic only). An address and a port, no domain: a domain
///   on a mainland machine's 443 needs an ICP filing.
/// - **Hong Kong**, `hk.relay.khor.cloud`, TLS with a Let's Encrypt
///   certificate. For everything that can cross the border, and for
///   machines abroad.
///
/// Each endpoint homes on whichever it measures nearest; two machines
/// on different homes meet through the callee's, and Beijing is the one
/// both sides can reach. **Two addresses, never one name over both**: a
/// relay is stateful — a machine is found only on the relay it is
/// connected to — so one name resolving to two boxes splits the fleet.
///
/// **A key each, and not the same key.** A key compiled into a binary
/// can be dug out of it, so these stop casual free-riding, not a
/// determined one (CONNECT.md 分期 0 says the same); what they buy is
/// that neither box is an open relay for every iroh program. Beijing's
/// crosses the wire in the clear, which is why it must not also open
/// Hong Kong.
///
/// **n0's public relays stay behind these as the fallback** (the owner,
/// 2026-09-30: 「还是不去 n0 吧，n0 可以兜底」). [`bind_with`] is where
/// the fallback is added; 「只使用自己的中继」 drops both tiers.
///
/// ⚠️ 墓碑: until 2026-10-01 this was one open relay,
/// `http://39.97.7.248:3340` — mandala's aliyun box on loan, running
/// `iroh-relay --dev` and admitting everyone — with `KHOR_RELAY_TOKEN`
/// a separate `None`. That box's lease ends; the key now rides with
/// each address because the two relays do not share one.
pub const KHOR_RELAYS: &[(&str, Option<&str>)] = &[
    (
        "http://101.200.80.77:3340",
        Some("bbe31ccf721dde188508517840c24673ef9d4a5865af18a3"),
    ),
    (
        BEIJING_TLS,
        Some("19ccb31e891ffa3ab66f95f5b743c7d4b419afe635b97e5e"),
    ),
    (
        "https://hk.relay.khor.cloud",
        Some("a086ea41af0a8bfe73ac98a914a18542bea845123e8b9eaf"),
    ),
];

/// **Beijing over TLS** (round NET-7, option B): the same box as
/// `http://101.200.80.77:3340`, a second relay process on a high port
/// with a certificate obtained by DNS (acme.sh, `scripts/relay-https.sh`)
/// — a domain on a mainland box's 80 or 443 needs an ICP filing, a high
/// port is the way around it that the filing rules leave.
///
/// **Its own key**, not the plain relay's: that one has crossed the wire
/// in the clear since 2026-10-01, so it opens nothing new once the plain
/// relay is retired.
pub const BEIJING_TLS: &str = "https://pk.relay.khor.cloud:3341";

/// **Which relay replaces which** (vendor/iroh patch 5): a machine whose
/// home would be the first homes on the second whenever its latest
/// report reached it. Both stay in the map with their keys, because a
/// relay is stateful — a machine still homed on the old one (one that
/// cannot reach the new, or an older khor that does not know it) is
/// found only there, and reaching it there takes that relay's key.
///
/// So the day the TLS relay starts answering, every machine of this
/// build that can reach it moves over on its next report, and one that
/// cannot stays on the plain relay as before: nothing has to be switched
/// on both ends at once.
pub const KHOR_SUCCESSORS: &[(&str, &str)] = &[("http://101.200.80.77:3340", BEIJING_TLS)];

/// `KHOR_RELAY_SUCCESSOR`: more `old|new` pairs for [`KHOR_SUCCESSORS`],
/// comma-separated — the same move for relays somebody runs themselves,
/// and the tests' way to make it on loopback.
pub const RELAY_SUCCESSOR_ENV: &str = "KHOR_RELAY_SUCCESSOR";

/// `KHOR_RELAY_CA`: a PEM file of certificates to trust for relays besides
/// Mozilla's roots — a relay with a private CA, or a test's self-signed
/// one.
pub const RELAY_CA_ENV: &str = "KHOR_RELAY_CA";

/// Every `(old, new)` pair, as written: [`KHOR_SUCCESSORS`], then
/// [`RELAY_SUCCESSOR_ENV`]'s.
pub fn successor_pairs() -> Vec<(String, String)> {
    let from_env = std::env::var(RELAY_SUCCESSOR_ENV).unwrap_or_default();
    KHOR_SUCCESSORS
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .chain(from_env.split(',').filter_map(|p| {
            let (a, b) = p.trim().split_once('|')?;
            Some((a.trim().to_owned(), b.trim().to_owned()))
        }))
        .collect()
}

/// The successor pairs a bind hands iroh: [`KHOR_SUCCESSORS`] and
/// [`RELAY_SUCCESSOR_ENV`]'s, each kept only when both relays are in
/// `relays` — a successor this machine holds no entry (and no key) for
/// would be a home it could not stand on.
pub fn successors(relays: &[Relay]) -> Vec<(iroh::RelayUrl, iroh::RelayUrl)> {
    let held = |u: &str| relays.iter().any(|r| r.url.trim_end_matches('/') == u.trim_end_matches('/'));
    successor_pairs()
        .into_iter()
        .filter(|(a, b)| held(a) && held(b))
        .filter_map(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .collect()
}

/// One relay this machine binds with: where it is, and the key its
/// operator asked for.
///
/// **The key never travels.** What goes into the device table's hints,
/// the pairing ticket and `endpoint.json` is [`Relay::url`] alone —
/// those are copied to every machine on the network and kept in
/// history that cannot be deleted. The consequence is on purpose: a
/// peer that wants to reach this machine through a keyed relay has to
/// hold the key itself, typed on that machine. A relay khor runs for
/// its own devices (`khor relay --only-mine`) needs no key at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relay {
    pub url: String,
    /// The shared key the relay admits by, sent as a bearer token.
    pub token: Option<String>,
}

impl Relay {
    /// A relay that asks for nothing.
    pub fn bare(url: impl Into<String>) -> Relay {
        Relay {
            url: url.into(),
            token: None,
        }
    }

    /// Reads one entry the way `KHOR_RELAY` writes it: a relay url,
    /// optionally carrying its key as a `token` query parameter —
    /// `https://relay.example.com?token=SECRET`. `None` when it is not a
    /// relay url at all.
    ///
    /// The query parameter is iroh-relay's own spelling
    /// (`ClientRequest::auth_token` accepts `?token=` beside the bearer
    /// header), so a url copied out of an iroh-relay operator's notes
    /// means the same thing here. The key is lifted out of the url
    /// rather than left in it: the url is what gets published, and a
    /// key left inside it would be published with it.
    pub fn parse(entry: &str) -> Option<Relay> {
        let entry = entry.trim();
        let parsed: iroh::RelayUrl = entry.parse().ok()?;
        let token = parsed
            .query_pairs()
            .find(|(k, _)| k == "token")
            .map(|(_, v)| v.into_owned())
            .filter(|t| !t.is_empty());
        let url = match entry.split_once('?') {
            Some((bare, _)) => bare.to_string(),
            None => entry.to_string(),
        };
        // The part before the query has to stand on its own too:
        // `http://?token=x` parses as a url with the query, and not
        // without it.
        url.parse::<iroh::RelayUrl>().ok()?;
        Some(Relay { url, token })
    }
}

/// The Khor tier as relay entries.
pub fn khor_tier() -> Vec<Relay> {
    KHOR_RELAYS
        .iter()
        .map(|(url, token)| Relay {
            url: url.to_string(),
            token: token.map(str::to_string),
        })
        .collect()
}

/// What `KHOR_RELAY` names: comma-separated entries, each a relay url
/// with an optional `?token=` ([`Relay::parse`]). An entry that does
/// not parse is skipped — one typo must not cost the relays typed
/// correctly, the same rule [`dial_addr`] keeps. A key therefore cannot
/// contain a comma.
pub fn env_relays() -> Vec<Relay> {
    let Ok(own) = std::env::var("KHOR_RELAY") else {
        return Vec::new();
    };
    let mut out: Vec<Relay> = Vec::new();
    for entry in own.split(',').filter_map(Relay::parse) {
        if !out.iter().any(|have| have.url == entry.url) {
            out.push(entry);
        }
    }
    out
}

/// Every relay this machine should use by default: the Khor tier plus
/// `KHOR_RELAY`. Deduplicated by url, first one wins, order kept.
pub fn configured_relays() -> Vec<Relay> {
    let mut out = khor_tier();
    for entry in env_relays() {
        if !out.iter().any(|have| have.url == entry.url) {
            out.push(entry);
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
///
/// **The task holds the stream, not the service.** A copy of the service
/// kept here would keep it alive after its endpoint closed, and the stream
/// ends only when the service does: every bind used to leave one behind,
/// answering and decoding on 5353 for the life of the process (the owner's
/// app held 50 sockets there three hours after launch, term-mdns.md). With
/// the copy dropped once subscribed, the endpoint's is the last one, and
/// closing the endpoint ends the service, the stream and this task.
fn follow(mdns: iroh_mdns_address_lookup::MdnsAddressLookup, id: iroh::EndpointId) {
    tokio::spawn(async move {
        use n0_future::StreamExt;
        #[cfg(test)]
        let _following = tests::Following::new(id);
        #[cfg(not(test))]
        let _ = id;
        let mut events = mdns.subscribe().await;
        drop(mdns);
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

/// The environment switch that binds with relays only ([`builder`]).
pub const RELAY_ONLY_ENV: &str = "KHOR_RELAY_ONLY";

/// The environment switch that binds UDP on this one address only, say
/// `127.0.0.1` or `::1` ([`bind_pinned`]); iroh then offers exactly it.
///
/// **For scratch nodes, not for people.** Three nodes on one machine all
/// go direct to each other. One bound on `127.0.0.1` and another on
/// `::1` share no road, so the two meet on the relay alone, while a third
/// bound on both wildcards reaches each directly — the shape of a peer
/// hop (`crates/cli/tests/hop.rs`). Never set by khor.
pub const BIND_IP_ENV: &str = "KHOR_BIND_IP";

/// The environment switch that stands in for n0's public relays, and
/// turns that tier on even with 「only mine」: comma-separated relay urls
/// ([`full_relay_map`]).
///
/// **For scratch nodes, not for people.** A machine with the n0 tier on
/// can home on a public relay that is in no list it keeps — MacLab on
/// n0's aps1, 2026-10-09 — and a test needs that shape on loopback
/// (`crates/cli/tests/home_relay.rs`). Never set by khor.
pub const PUBLIC_RELAYS_ENV: &str = "KHOR_PUBLIC_RELAYS";

/// **The relays a machine publishes: its live home first, then its own
/// list** (net-f, 2026-10-09). The home is where the machine listens; a
/// home outside its own list (n0's, when that tier is on) published
/// nowhere left every peer dialling relays it was not on, and MacLab
/// unreachable for twenty minutes once it homed on aps1. Written without
/// the trailing slash, as the lists are; a list entry that names the home
/// is not written twice.
pub fn home_first(home: Option<&iroh::RelayUrl>, relays: &[String]) -> Vec<String> {
    let mut out: Vec<String> = home.map(|h| h.as_str().trim_end_matches('/').to_owned()).into_iter().collect();
    for r in relays {
        let is_home = home.is_some_and(|h| r.parse::<iroh::RelayUrl>().is_ok_and(|u| &u == h));
        if !is_home {
            out.push(r.clone());
        }
    }
    out
}

/// The one address [`BIND_IP_ENV`] names, if it names one.
fn bind_ip() -> Option<std::net::IpAddr> {
    std::env::var(BIND_IP_ENV).ok()?.trim().parse().ok()
}

/// The environment switch that binds with no mDNS service: the node
/// neither announces itself on the segment nor listens for others.
///
/// **For scratch nodes, not for people.** Every machine on a segment
/// decodes every other's announcement, so each scratch bridge, test
/// serve and loopback relay a lane leaves running costs the person's own
/// app on the same Wi-Fi (term-cpu.md: a 14 h app spent most of its
/// time decoding announcements). A scratch node finds its peers through
/// the device table and the loopback relay; mDNS adds nothing there.
/// The tests' harnesses set it for every node they start.
pub const NO_MDNS_ENV: &str = "KHOR_NO_MDNS";

/// Whether this process binds with mDNS ([`NO_MDNS_ENV`]).
fn on_the_segment() -> bool {
    std::env::var_os(NO_MDNS_ENV).is_none()
}

/// Binds this machine's endpoint. One live endpoint per key — a second
/// bind with the same key knocks the first off the relay, and both stop
/// working (learned in production).
///
/// **`relays` is the whole relay map**, and this is the hermetic shape:
/// iroh probes exactly these and homes on the closest one that answers;
/// nothing is added behind the caller's back, and what this machine
/// talks to for relay and lookup is this list plus mDNS on its own
/// segment. An empty list is an endpoint with no relay at all,
/// reachable only on a road somebody already knows; a node never asks
/// for that (`Node::set_relays` refuses the one setting that would).
/// Tests that stand up a throwaway endpoint want this one; a node binds
/// through [`bind_with`], which adds n0's tier unless the machine is set
/// to use only its own relays.
///
/// Relays are assumed to run no QUIC address discovery — assuming it
/// costs a dead-port wait on every connection. Each carries its own key
/// when it has one ([`Relay::token`]).
///
/// **An HTTP proxy from the environment is honoured** (`HTTP_PROXY`,
/// `http_proxy`, `HTTPS_PROXY`, `https_proxy`, in iroh's order): a
/// network that lets out only TCP 443 through a proxy still reaches a
/// relay that way (CONNECT.md 设备会在的网络). iroh reads no `NO_PROXY`,
/// so a proxy set for the whole shell carries a relay on the same LAN
/// through it too.
pub async fn bind(secret: iroh::SecretKey, relays: &[Relay]) -> Result<iroh::Endpoint> {
    bind_with(secret, relays, false).await
}

/// [`bind`], plus the switch that puts n0 beside `relays` as the
/// fallback tier: n0's public relays in the map after the caller's, and
/// the n0 preset's pkarr/DNS lookup beside mDNS. Off, it is [`bind`]
/// exactly — the shape 「只使用自己的中继」 asks for, where this machine
/// touches no relay and no lookup service it was not told about.
pub async fn bind_with(
    secret: iroh::SecretKey,
    relays: &[Relay],
    n0: bool,
) -> Result<iroh::Endpoint> {
    bind_preferring(secret, relays, n0, None).await
}

/// [`bind_with`], **homing on `preferred` whenever it is reachable** —
/// the person's 首选中继 (设置 → 网络). iroh otherwise homes on the relay
/// with the lowest recent latency, and switches only when another is a
/// third faster (`net_report.rs`); the preference is a vendored patch
/// (`vendor/iroh/KHOR.md`, patch 3) because iroh 1.2 has no knob for it.
/// Unreachable, it is ignored and iroh chooses as before: a preference is
/// never a reason to have no home.
pub async fn bind_preferring(
    secret: iroh::SecretKey,
    relays: &[Relay],
    n0: bool,
    preferred: Option<&str>,
) -> Result<iroh::Endpoint> {
    bind_pinned(secret, relays, n0, preferred, (0, 0), None).await
}

/// [`bind_preferring`] on the UDP ports `(v4, v6)` it was on last time
/// (round NET-11); `0` for either is any free port, as iroh binds by
/// default. **Fails when a port is taken**: the caller binds again on
/// any port, which is what every start did before.
///
/// Why: the device table carries each machine's addresses with their
/// ports, and every start on a fresh port wrote the row again — a block
/// every machine copies and a writer more in every version vector
/// (lecun, 10-05: about 600 rewrites of its own row). A port kept across
/// restarts also keeps the hints the peers hold true. Field practice:
/// Tailscale's tailscaled keeps one UDP port (41641 by default),
/// WireGuard a fixed `ListenPort`, Syncthing 22000.
///
/// **`carry`: the roads khor carries itself** (round ROAD-2,
/// [`crate::carry`]): registered with khor's [`crate::carry::RoadOrder`]
/// selector when given; `None` is the transport not registered at all —
/// both switches off.
pub async fn bind_pinned(
    secret: iroh::SecretKey,
    relays: &[Relay],
    n0: bool,
    preferred: Option<&str>,
    (v4, v6): (u16, u16),
    carry: Option<&crate::carry::Links>,
) -> Result<iroh::Endpoint> {
    let mut b = builder(secret, relays, n0, on_the_segment())?;
    if let Some(links) = carry {
        b = b
            .add_custom_transport(links.transport())
            .path_selector(std::sync::Arc::new(crate::carry::RoadOrder::default()));
    }
    if let Some(ip) = bind_ip().filter(|_| std::env::var_os(RELAY_ONLY_ENV).is_none()) {
        let port = if ip.is_ipv4() { v4 } else { v6 };
        b = b.clear_ip_transports().bind_addr(std::net::SocketAddr::new(ip, port))?;
    } else if (v4, v6) != (0, 0) && std::env::var_os(RELAY_ONLY_ENV).is_none() {
        use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
        b = b
            .clear_ip_transports()
            .bind_addr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, v4)))?
            .bind_addr_with_opts(
                SocketAddr::from((Ipv6Addr::UNSPECIFIED, v6)),
                iroh::endpoint::BindOpts::default().set_is_required(false),
            )?;
    }
    let preferred = preferred.and_then(|u| u.parse::<iroh::RelayUrl>().ok());
    let successors = successors(relays);
    if preferred.is_some() || !successors.is_empty() {
        // `NetReportConfig` is non-exhaustive: built, then set.
        let mut config = iroh::endpoint::NetReportConfig::default();
        config.preferred_relay = preferred;
        config.successors = successors;
        b = b.net_report_config(config);
    }
    Ok(b.bind().await?)
}

/// Everything [`bind`] configures, short of binding — split out so the
/// configuration can be read (its `Debug`) without a socket.
/// `mdns` off is [`NO_MDNS_ENV`]: no service is built, so nothing
/// announces and [`NEARBY`] stays empty.
fn builder(
    secret: iroh::SecretKey,
    relays: &[Relay],
    n0: bool,
    mdns: bool,
) -> Result<iroh::endpoint::Builder> {
    // Built here rather than by iroh so that `follow` can read what it
    // finds; registered on the builder all the same (`Prebuilt`).
    let mdns = if mdns {
        let built = iroh_mdns_address_lookup::MdnsAddressLookup::builder()
            .build(secret.public())
            .context(msg::no_local_lookup())?;
        follow(built.clone(), secret.public());
        Some(built)
    } else {
        None
    };
    // The n0 preset is n0's relays plus a pkarr publisher, a pkarr
    // resolver and a DNS lookup, all pointed at n0's servers; `Minimal`
    // keeps only the crypto provider. The relay map is replaced below
    // either way, so the preset decides the lookup services alone.
    let mut builder = if n0 {
        iroh::Endpoint::builder(iroh::endpoint::presets::N0)
    } else {
        iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
    }
        .secret_key(secret)
        // Compiled-in Mozilla roots instead of the operating system's:
        // the system route reads the macOS Keychain, and on a Mac
        // operated over ssh that read hangs 25-40 seconds — every
        // process's FIRST bind paid it (measured 2026-08-20: khor's
        // own test binaries at 36-43s per process, while the verdict
        // hid because the hang is synchronous and froze single-thread
        // tokio timers with it, #73). These roots only verify outward
        // TLS (https relays); iroh's own encryption never touched the
        // Keychain in the first place.
        .ca_tls_config(ca_roots())
        // Explicit public nameservers instead of "read the system's":
        // on macOS the system route goes through SCDynamicStore, whose
        // ApplicationBundleID lookup hung 40 SECONDS per process for
        // unbundled binaries — measured with `sample` on 2026-08-20,
        // the whole of #73's frozen-timer mystery (the hang is
        // synchronous, so it froze single-threaded tokio clocks too).
        // The Khor tier is an IP and needs no DNS at all; these servers
        // resolve a relay somebody named by its domain. The cost — a
        // network whose ONLY nameserver is an internal one cannot
        // resolve such a relay — is on the ledger.
        .dns_resolver(
            iroh::dns::DnsResolver::builder()
                .add_nameserver_configs(
                    ["223.5.5.5", "8.8.8.8", "1.1.1.1", "2400:3200::1"]
                        .map(|ip| iroh::dns::NameserverConfig::udp(ip.parse().unwrap())),
                )
                .build(),
        )
        .proxy_from_env()
        .transport_config(transport())
        .alpns(vec![ALPN.to_vec(), TUNNEL_ALPN.to_vec()])
        // The filter that keeps this machine from publishing an address
        // nobody can route to belongs where the services are assembled
        // (the reason is the mDNS service's, below). It also decides what
        // a hole-punching round offers the far side (vendor/iroh patch 9):
        // upstream offered every address, 198.18.0.1 among them.
        .addr_filter(iroh::address_lookup::AddrFilter::new(|addrs| {
            if addrs.iter().any(unroutable) {
                std::borrow::Cow::Owned(addrs.iter().filter(|a| !unroutable(a)).cloned().collect())
            } else {
                std::borrow::Cow::Borrowed(addrs)
            }
        }));
    // **On the builder, not on the bound endpoint — and that is the
    // whole difference between wired up and merely present.**
    // `endpoint.address_lookup().add(..)` after a bind (what the
    // crate's own README shows) leaves a service that discovers and
    // answers `resolve()` correctly while the dialer never consults
    // it: measured 2026-09-18, A saw B, `resolve(B)` handed back B's
    // three addresses with provenance "mdns", dialling B by key
    // alone timed out, and dialling the very same addresses by hand
    // connected. Every piece worked and the chain did not.
    if let Some(mdns) = mdns {
        builder = builder.address_lookup(Prebuilt(mdns));
    }
    let map = full_relay_map(relays, n0);
    if !map.is_empty() {
        builder = builder.relay_mode(iroh::endpoint::RelayMode::Custom(map));
    }
    // **`KHOR_RELAY_ONLY`: no direct road at all**, every byte through a
    // relay — the network a phone on carrier NAT often has, made on
    // purpose. A diagnostic and the tests' way to put a slow relay
    // between two nodes on one machine (`crates/cli/tests/slow_relay.rs`),
    // where loopback would otherwise always go direct. Never set by khor.
    if std::env::var_os(RELAY_ONLY_ENV).is_some() {
        builder = builder.clear_ip_transports();
    }
    Ok(builder)
}

/// The roots a relay's certificate is checked against: Mozilla's,
/// compiled in, plus whatever [`RELAY_CA_ENV`] names. A file that does
/// not read adds nothing and says so on stderr: a relay that then fails
/// its handshake is the visible half.
fn ca_roots() -> iroh::tls::CaTlsConfig {
    let roots = iroh::tls::CaTlsConfig::embedded();
    let Some(path) = std::env::var_os(RELAY_CA_ENV) else {
        return roots;
    };
    use rustls::pki_types::pem::PemObject;
    match rustls::pki_types::CertificateDer::pem_file_iter(&path)
        .and_then(|i| i.collect::<Result<Vec<_>, _>>())
    {
        Ok(extra) => roots.with_extra_roots(extra),
        Err(e) => {
            eprintln!("{}: {e}", std::path::Path::new(&path).display());
            roots
        }
    }
}

/// **The QUIC transport khor binds with: iroh's own, with BBRv3 in place
/// of Cubic** (round NET-5). `QuicTransportConfig::builder` starts from
/// iroh's settings — the 5 s keepalive, path idle, multipath and NAT
/// traversal its hole punching depends on — and only the controller
/// changes.
///
/// Measured on a simulated road (`tests/congestion.rs`: noq through a
/// 20 Mbit/s bottleneck with 200 ms of queue): at 300 ms and 2 % random
/// loss Cubic moves 0.33 Mbit/s — the loss formula's ceiling,
/// `1.22 · MSS / (RTT · √p)` (Mathis et al. 1997) — and BBRv3 10.1; a
/// one-byte answer behind a bulk pour takes 469 ms at 300 ms under Cubic
/// (the queue full) and 314 under BBRv3, 192 against 41 at 20 ms. The
/// price: on a clean 300 ms road BBRv3 moves about a tenth less (16.4
/// against 18.2 Mbit/s over a minute); on a 20 ms road the two are level.
///
/// ⚠️ 墓碑 (round NET-2, 3727c05): BBRv3 was measured and **not** taken
/// then. noq 1.3.0 did not tell it when the connection went idle, so it
/// paced a burst after typing by the trickle typing had shown it — a
/// screenful 0.72–0.91 s against Cubic's 0.30 at 200 ms. The vendored
/// noq-proto carries the fix from moq-dev/noq#5 (vendor/noq-proto/KHOR.md),
/// and the same screenful now takes 0.21 s against Cubic's 0.30.
///
/// **And observed-address reports both ways** (NET-DIRECT): each side tells
/// the other, on every direct path, the address it sees the other's
/// packets come from — what QAD asks a relay, asked of a peer. iroh has
/// them off, and ignored them when on; vendor/iroh patch 10 offers what a
/// peer saw as a hole-punching candidate, so a machine whose relays answer
/// no QAD still learns its public address from any machine it reaches
/// directly. A peer of an older build negotiates neither and sends none.
pub fn transport() -> iroh::endpoint::QuicTransportConfig {
    iroh::endpoint::QuicTransportConfig::builder()
        .congestion_controller_factory(std::sync::Arc::new(
            noq_proto::congestion::Bbr3Config::default(),
        ))
        .send_observed_address_reports(true)
        .receive_observed_address_reports(true)
        .build()
}

/// The map a bind hands iroh: the caller's relays ([`relay_map`]), then
/// n0's public ones when `n0` is on. After the caller's, so a keyed or
/// self-run relay is never shadowed by a public one at the same url;
/// iroh homes by measured latency, not by position, so order decides
/// nothing else.
fn full_relay_map(relays: &[Relay], n0: bool) -> iroh::RelayMap {
    let mut map: Vec<std::sync::Arc<iroh::RelayConfig>> = relay_map(relays).relays();
    // A test's stand-in tier is on whatever `n0` says: a scratch node
    // keeps 「only mine」 on, so it touches no real relay, and still has a
    // public tier to home on.
    let named = std::env::var(PUBLIC_RELAYS_ENV).ok();
    if n0 || named.is_some() {
        let public: Vec<std::sync::Arc<iroh::RelayConfig>> = match named {
            Some(named) => relay_map(
                &named
                    .split(',')
                    .map(|u| Relay { url: u.trim().to_owned(), token: None })
                    .collect::<Vec<_>>(),
            )
            .relays(),
            None => iroh::defaults::prod::default_relay_map().relays(),
        };
        for r in public {
            if !map.iter().any(|have| have.url == r.url) {
                map.push(r);
            }
        }
    }
    map.into_iter().collect()
}

/// The relay map a bind hands iroh: every entry that parses, each with
/// its key. An entry that does not parse is dropped here rather than
/// failing the bind — the list comes from files and environment
/// variables, and one typo must not take the rest down with it.
pub fn relay_map(relays: &[Relay]) -> iroh::RelayMap {
    relays
        .iter()
        .filter_map(|r| {
            let url = r.url.parse::<iroh::RelayUrl>().ok()?;
            let mut c = iroh::RelayConfig::from(url);
            c.quic = None;
            c.auth_token = r.token.clone().filter(|t| !t.is_empty());
            Some(std::sync::Arc::new(c))
        })
        .collect()
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
        // A carried road (round ROAD-2) is this machine's id on a link
        // only the far end of that link holds: nobody else can use it.
        iroh::TransportAddr::Custom(_) => true,
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

    #[test]
    fn the_live_home_is_published_first_and_once() {
        let mine = vec!["http://101.200.80.77:3340".to_owned(), "https://hk.relay.khor.cloud".to_owned()];
        let aps1: iroh::RelayUrl = "https://aps1-1.relay.n0.iroh.link.".parse().unwrap();
        assert_eq!(
            home_first(Some(&aps1), &mine),
            ["https://aps1-1.relay.n0.iroh.link.", "http://101.200.80.77:3340", "https://hk.relay.khor.cloud"]
        );
        let beijing: iroh::RelayUrl = "http://101.200.80.77:3340".parse().unwrap();
        assert_eq!(home_first(Some(&beijing), &mine), mine);
        assert_eq!(home_first(None, &mine), mine);
    }

    /// The [`follow`] tasks alive, by the key whose mDNS service each
    /// reads: a guard held for as long as the task runs.
    static FOLLOWING: std::sync::LazyLock<
        std::sync::Mutex<std::collections::HashMap<iroh::EndpointId, usize>>,
    > = std::sync::LazyLock::new(Default::default);

    pub(super) struct Following(iroh::EndpointId);

    impl Following {
        pub(super) fn new(id: iroh::EndpointId) -> Self {
            *FOLLOWING.lock().unwrap().entry(id).or_default() += 1;
            Self(id)
        }

        fn alive(id: &iroh::EndpointId) -> usize {
            FOLLOWING.lock().unwrap().get(id).copied().unwrap_or(0)
        }
    }

    impl Drop for Following {
        fn drop(&mut self) {
            if let Some(n) = FOLLOWING.lock().unwrap().get_mut(&self.0) {
                *n -= 1;
            }
        }
    }

    /// **A closed endpoint takes its mDNS service with it.** `follow` used
    /// to hold a copy of the service for as long as its event stream ran,
    /// and the stream ran for as long as the service lived: every bind
    /// left a service behind, answering and decoding on 5353 for the
    /// life of the process (term-mdns.md: the owner's app held 50 sockets
    /// on 5353 three hours after launch).
    #[tokio::test]
    async fn a_closed_endpoint_takes_its_mdns_with_it() {
        let secret = iroh::SecretKey::generate();
        let id = secret.public();
        let relays = [Relay::bare("http://203.0.113.9:3340")];
        let ep = builder(secret, &relays, false, true).unwrap().bind().await.unwrap();
        let until = |want: usize| async move {
            for _ in 0..100 {
                if Following::alive(&id) == want {
                    return true;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            false
        };
        assert!(until(1).await, "control: the bind's service is followed");
        ep.close().await;
        drop(ep);
        assert!(
            until(0).await,
            "the service outlived its endpoint: {} follower(s) left",
            Following::alive(&id)
        );
    }

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

    /// `KHOR_RELAY`'s entry shape: a url, and a key riding as `?token=`
    /// that is lifted out of the url — the url is what gets published.
    #[test]
    fn a_relay_entry_carries_its_key_outside_the_url() {
        let keyed = Relay::parse(" https://relay.example.com:8443?token=s3cr%2Bt ").unwrap();
        assert_eq!(keyed.url, "https://relay.example.com:8443");
        assert_eq!(
            keyed.token.as_deref(),
            Some("s3cr+t"),
            "the key is percent-decoded"
        );
        let bare = Relay::parse("http://39.97.7.248:3340").unwrap();
        assert_eq!(bare, Relay::bare("http://39.97.7.248:3340"));
        // An empty key is no key: `?token=` alone must not send an empty
        // bearer that a keyed relay refuses and an open one ignores.
        assert_eq!(Relay::parse("http://h:1?token=").unwrap().token, None);
        for garbage in ["", "garbage", "?token=x", "http://?token=x"] {
            assert_eq!(Relay::parse(garbage), None, "{garbage:?} is not a relay");
        }
    }

    /// The map a bind hands iroh: keys on the relays that have them,
    /// none on the ones that do not, and a bad entry dropped rather than
    /// failing the rest.
    #[test]
    fn the_relay_map_carries_each_key_and_skips_garbage() {
        let map = relay_map(&[
            Relay {
                url: "http://203.0.113.9:3340".into(),
                token: Some("k".into()),
            },
            Relay::bare("http://203.0.113.10:3340"),
            Relay::bare("not a url"),
        ]);
        assert_eq!(map.len(), 2, "the bad entry is dropped, the others kept");
        let key_of = |u: &str| map.get(&u.parse().unwrap()).and_then(|c| c.auth_token.clone());
        assert_eq!(key_of("http://203.0.113.9:3340").as_deref(), Some("k"));
        assert_eq!(key_of("http://203.0.113.10:3340"), None);
    }

    /// The hermetic bind touches no n0 service — not a relay in the
    /// map, not a lookup service — and the n0 switch puts n0's relays
    /// and lookup back **after** the caller's own (the owner's
    /// 2026-09-30 ruling: n0 stays as the fallback tier).
    ///
    /// **Two controls, because the hermetic half is absence checks.**
    /// The needle is shown to find n0 where n0 is (iroh's own n0
    /// preset), and the relay probe is shown to find a relay that is
    /// there — without them, a `Debug` that printed nothing and a probe
    /// that always missed would pass as well as the real thing.
    #[tokio::test]
    async fn a_hermetic_bind_has_no_n0_and_the_n0_switch_adds_it_behind_ours() {
        const N0: &str = "iroh.link";
        const MINE: &str = "http://203.0.113.9:3340";
        let relays = [Relay::bare(MINE)];

        let n0 = format!("{:?}", iroh::Endpoint::builder(iroh::endpoint::presets::N0));
        assert!(n0.contains(N0), "control: the needle must find n0 in n0's preset");
        let ours = format!("{:?}", builder(iroh::SecretKey::generate(), &relays, false, true).unwrap());
        assert!(!ours.contains(N0), "the builder names an n0 service: {ours}");
        assert!(
            ours.contains("203.0.113.9"),
            "control: the builder's Debug must show the relays it holds: {ours}"
        );
        let with = format!("{:?}", builder(iroh::SecretKey::generate(), &relays, true, true).unwrap());
        assert!(with.contains(N0), "the n0 switch must name n0's services: {with}");
        let urls: Vec<iroh::RelayUrl> = full_relay_map(&relays, true).urls();
        assert_eq!(urls[0], MINE.parse().unwrap(), "ours comes first: {urls:?}");
        for url in iroh::defaults::prod::default_relay_map().urls::<Vec<_>>() {
            assert!(urls.contains(&url), "n0's {url} is behind ours: {urls:?}");
        }
        assert_eq!(
            full_relay_map(&relays, false).urls::<Vec<_>>(),
            vec![MINE.parse::<iroh::RelayUrl>().unwrap()],
            "the hermetic map is ours alone"
        );

        // `builder` rather than `bind`: what `bind` adds is the process's
        // `KHOR_NO_MDNS`, which a test run may well carry.
        let ep = builder(iroh::SecretKey::generate(), &relays, false, true)
            .unwrap()
            .bind()
            .await
            .unwrap();
        let lookup = ep.address_lookup().unwrap();
        assert_eq!(lookup.len(), 1, "mDNS is the one lookup service: {lookup:?}");
        assert!(!format!("{lookup:?}").contains(N0), "{lookup:?}");
        for url in iroh::defaults::prod::default_relay_map().urls::<Vec<_>>() {
            assert!(
                ep.remove_relay(&url).await.is_none(),
                "n0's relay {url} is in the bound endpoint's map"
            );
        }
        assert!(
            ep.remove_relay(&MINE.parse().unwrap()).await.is_some(),
            "control: the probe must find the relay that is in the map"
        );
        ep.close().await;
    }

    /// **A hole-punching round offers only what is published** (vendor/iroh
    /// patches 8 and 9). Upstream offered the far side every direct
    /// address, a proxy TUN's 198.18.0.1 among them (MacLab's journal,
    /// 2026-10-09). On a machine that has such an address, iroh still holds
    /// it among its direct addresses — the control — and the round leaves
    /// it out; on one that has none the check says so and is vacuous.
    #[tokio::test]
    async fn a_punching_round_offers_no_unroutable_address() {
        use iroh::Watcher;
        let relays = [Relay::bare("http://203.0.113.9:3340")];
        let bind = || async { builder(iroh::SecretKey::generate(), &relays, false, false).unwrap().bind().await.unwrap() };
        let (server, client) = (bind().await, bind().await);
        let port = server.bound_sockets().iter().find(|s| s.is_ipv4()).unwrap().port();
        let addr = iroh::EndpointAddr::new(server.id())
            .with_ip_addr(std::net::SocketAddr::from(([127, 0, 0, 1], port)));
        let held = tokio::spawn({
            let server = server.clone();
            async move {
                let conn = server.accept().await.expect("accept ended early").await.expect("handshake");
                conn.closed().await;
            }
        });
        let mut rounds = client.holepunch_rounds();
        let _conn = client.connect(addr, ALPN).await.expect("loopback dial");
        let round = tokio::time::timeout(std::time::Duration::from_secs(10), rounds.recv())
            .await
            .expect("a round starts when the connection is added")
            .expect("the round");
        assert_eq!(round.remote, server.id());
        let offered: Vec<_> = round.local.iter().filter(|a| unroutable(&iroh::TransportAddr::Ip(**a))).collect();
        assert!(offered.is_empty(), "offered unroutable candidates: {offered:?}");
        let held_here: Vec<std::net::SocketAddr> = client
            .watch_direct_addrs()
            .get()
            .iter()
            .map(|d| d.addr)
            .filter(|a| unroutable(&iroh::TransportAddr::Ip(*a)))
            .collect();
        if held_here.is_empty() {
            eprintln!("no unroutable address on this machine: the check above is vacuous here");
        } else {
            eprintln!("control: iroh holds {held_here:?}, the round offered {:?}", round.local);
        }
        held.abort();
        client.close().await;
        server.close().await;
    }

    /// `KHOR_NO_MDNS`: no lookup service at all on a hermetic bind, so
    /// the node neither announces nor listens on the segment.
    #[tokio::test]
    async fn a_bind_off_the_segment_has_no_mdns() {
        let relays = [Relay::bare("http://203.0.113.9:3340")];
        let ep = builder(iroh::SecretKey::generate(), &relays, false, false)
            .unwrap()
            .bind()
            .await
            .unwrap();
        let lookup = ep.address_lookup().unwrap();
        assert_eq!(lookup.len(), 0, "no lookup service without mDNS: {lookup:?}");
        ep.close().await;
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
