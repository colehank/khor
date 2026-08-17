//! Shared by the wire-control halves of the acceptance tests. Each test
//! binary compiles this module and uses only the helpers it needs, so a
//! helper unused by one is dead code there — allowed, not a real unused.
#![allow(dead_code)]

use khor_net::endpoint::{self, ALPN, TUNNEL_ALPN};
use khor_node::proto::{self, Request, Response, MAX_FRAME};
use std::time::Duration;
use tokio::time::timeout;

/// A bare wire request from an arbitrary key — the shape a hostile or
/// merely confused client would send.
pub async fn raw_request(
    secret: iroh::SecretKey,
    target_id: &str,
    target_addrs: &[String],
    req: &Request,
) -> Result<Response, String> {
    let ep = endpoint::bind(secret, &[]).await.map_err(|e| e.to_string())?;
    let addr = endpoint::dial_addr(target_id, target_addrs, &[]).map_err(|e| e.to_string())?;
    let conn = timeout(Duration::from_secs(10), ep.connect(addr, ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send.write_all(&proto::encode(req)?).await.map_err(|e| e.to_string())?;
    send.finish().map_err(|e| e.to_string())?;
    let bytes = recv.read_to_end(MAX_FRAME).await.map_err(|e| e.to_string())?;
    let resp = proto::decode(&bytes)?;
    ep.close().await;
    Ok(resp)
}

/// A bare borrow-tunnel dial from an arbitrary key: speak the tunnel
/// ALPN, ask for `dest`, and return the exit's one status byte. The
/// control half of the tunnel test — an unpaired key must get `REFUSED`
/// (1) here while a paired key gets through the same live serve.
pub async fn raw_tunnel(
    secret: iroh::SecretKey,
    target_id: &str,
    target_addrs: &[String],
    dest: &str,
) -> Result<u8, String> {
    let ep = endpoint::bind(secret, &[]).await.map_err(|e| e.to_string())?;
    let addr = endpoint::dial_addr(target_id, target_addrs, &[]).map_err(|e| e.to_string())?;
    let conn = timeout(Duration::from_secs(10), ep.connect(addr, TUNNEL_ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    let bytes = dest.as_bytes();
    send.write_all(&(bytes.len() as u16).to_be_bytes())
        .await
        .map_err(|e| e.to_string())?;
    send.write_all(bytes).await.map_err(|e| e.to_string())?;
    let mut status = [0u8; 1];
    recv.read_exact(&mut status).await.map_err(|e| e.to_string())?;
    ep.close().await;
    Ok(status[0])
}
