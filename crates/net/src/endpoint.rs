//! Binding, dialing, and the pairing ticket.

use anyhow::{Context, Result};

/// One protocol, one ALPN. Version bumps mean a new string.
pub const ALPN: &[u8] = b"khor/0";

/// Binds this machine's endpoint. One live endpoint per key — a second
/// bind with the same key knocks the first off the relay, and both stop
/// working (learned in production).
pub async fn bind(secret: iroh::SecretKey) -> Result<iroh::Endpoint> {
    Ok(iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await?)
}

/// A dial address: identity plus every road we know. iroh probes them in
/// parallel, picks by RTT, and keeps querying discovery on top — we only
/// supply the candidates it cannot look up. Unparseable entries are
/// skipped, not fatal: they come from tickets and stored hints, and one
/// typo must not kill the dial.
pub fn dial_addr(id_hex: &str, direct: &[String], relays: &[String]) -> Result<iroh::EndpointAddr> {
    let id: iroh::EndpointId = id_hex.parse().context("这不是一个机器 id")?;
    let mut addr = iroh::EndpointAddr::new(id);
    for d in direct {
        if let Ok(sock) = d.parse::<std::net::SocketAddr>() {
            addr = addr.with_ip_addr(sock);
        }
    }
    for r in relays {
        if let Ok(url) = r.parse::<iroh::RelayUrl>() {
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
            .context("这不是一张配对票(base64 解不开)")?;
        serde_json::from_slice(&bytes).context("这不是一张配对票(内容对不上)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_round_trips() {
        let t = Ticket {
            id: "aa".repeat(32),
            name: "alpha".into(),
            direct: vec!["127.0.0.1:4433".into()],
            relays: vec![],
            token: "deadbeef".into(),
        };
        assert_eq!(Ticket::decode(&t.encode().unwrap()).unwrap(), t);
        // Garbage is refused with the word 票, not a panic.
        let e = Ticket::decode("不是票").unwrap_err().to_string();
        assert!(e.contains("配对票"), "{e}");
    }
}
