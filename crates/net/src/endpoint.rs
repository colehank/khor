//! Binding, dialing, and the pairing ticket.

use anyhow::{Context, Result};
use khor_catalog::msg;

/// One protocol, one ALPN. Version bumps mean a new string.
pub const ALPN: &[u8] = b"khor/0";

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
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret)
        .alpns(vec![ALPN.to_vec()]);
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
        let mut relays: Vec<std::sync::Arc<iroh::RelayConfig>> =
            iroh::defaults::prod::default_relay_map().relays();
        relays.extend(extras.into_iter().map(std::sync::Arc::new));
        builder = builder.relay_mode(iroh::endpoint::RelayMode::Custom(
            relays.into_iter().collect(),
        ));
    }
    Ok(builder.bind().await?)
}

/// A dial address: identity plus every road we know. iroh probes them in
/// parallel, picks by RTT, and keeps querying discovery on top — we only
/// supply the candidates it cannot look up. Both lists take both shapes:
/// `ip:port` becomes a direct road, `http(s)://…` a relay road — stored
/// hints carry them mixed. Unparseable entries are skipped, not fatal:
/// they come from tickets and stored hints, and one typo must not kill
/// the dial.
pub fn dial_addr(id_hex: &str, direct: &[String], relays: &[String]) -> Result<iroh::EndpointAddr> {
    let id: iroh::EndpointId = id_hex.parse().context(msg::NOT_A_MACHINE_ID_SHORT)?;
    let mut addr = iroh::EndpointAddr::new(id);
    for road in direct.iter().chain(relays) {
        if let Ok(sock) = road.parse::<std::net::SocketAddr>() {
            addr = addr.with_ip_addr(sock);
        } else if road.starts_with("http://") || road.starts_with("https://") {
            if let Ok(url) = road.parse::<iroh::RelayUrl>() {
                addr = addr.with_relay_url(url);
            }
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
            .context(msg::NOT_A_TICKET_BASE64)?;
        serde_json::from_slice(&bytes).context(msg::NOT_A_TICKET_CONTENT)
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
        assert_eq!((ips, relays), (1, 2), "one direct road, two relay roads, no garbage");
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
            e.contains(msg::NOT_A_TICKET_CONTENT) || e.contains(msg::NOT_A_TICKET_BASE64),
            "{e}"
        );
    }
}
