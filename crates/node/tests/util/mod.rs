//! Shared by the wire-control halves of the acceptance tests.

use khor_net::endpoint::{self, ALPN};
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
