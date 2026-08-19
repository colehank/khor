//! Building the roads `khor_net::via` hands to iroh.
//!
//! The transport lives a layer down, where there is no device table and
//! no pairing; it knows how to *be* a road and nothing about who may
//! lend one. This module is the other half: it opens the stream to the
//! exit, speaks the borrow's handshake, and turns the two halves into
//! the channels the transport pumps.
//!
//! # The endpoint arrives late, and that is not a wart
//!
//! A road is built with iroh, and the transport that needs it is
//! registered *while* iroh is being built. So the endpoint is filled in
//! once, after the bind returns, and a road asked for before then simply
//! is not built — the datagram that asked is dropped, and QUIC sends it
//! again a moment later. The alternative, holding the send path until an
//! endpoint exists, would stall every other path iroh is probing.

use std::sync::{Arc, OnceLock};

use khor_net::via::{Exits, Road, Sink, Source};

/// How many datagrams may sit in either direction of one road. Small,
/// for `khor_net::via`'s reason: a queue deeper than the window only
/// delays packets the sender has already given up on.
const QUEUE: usize = 64;

/// The khor side of a road: what it takes to reach an exit, and the
/// endpoint to reach it on.
pub struct Roads {
    node: Arc<crate::Node>,
    /// Set once, after `bind` returns. See the module head.
    ep: OnceLock<iroh::Endpoint>,
}

impl Roads {
    pub fn new(node: Arc<crate::Node>) -> Arc<Roads> {
        Arc::new(Roads { node, ep: OnceLock::new() })
    }

    /// Hands over the endpoint the roads will be built on. Idempotent by
    /// construction — a second call is ignored, because a node has one
    /// endpoint and a second one would mean two identities.
    pub fn on(&self, ep: iroh::Endpoint) {
        let _ = self.ep.set(ep);
    }
}

impl Exits for Roads {
    fn open(
        &self,
        road: Road,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(Sink, Source), String>> + Send>>
    {
        let Some(ep) = self.ep.get().cloned() else {
            return Box::pin(async { Err(khor_catalog::msg::ROAD_NOT_READY.to_owned()) });
        };
        let node = self.node.clone();
        Box::pin(async move { open_road(node, ep, road).await })
    }
}

async fn open_road(
    node: Arc<crate::Node>,
    ep: iroh::Endpoint,
    road: Road,
) -> Result<(Sink, Source), String> {
    use tokio::io::AsyncReadExt;

    // The exit is named by id here rather than by name: this is machine
    // talking to machine, and the name is a thing people use.
    let borrow = node.tunnel_to_id(&ep, road.exit).await?;
    let (mut send, mut recv) =
        borrow.open(&format!("{}{}", crate::tunnel::UDP_PREFIX, road.far)).await?;

    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(QUEUE);
    let (in_tx, in_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(QUEUE);

    // Outbound: length-prefixed, because a stream has no datagram
    // boundaries and the exit has to know where each one ends.
    tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            let len = (bytes.len() as u16).to_be_bytes();
            if send.write_all(&len).await.is_err() || send.write_all(&bytes).await.is_err() {
                break;
            }
        }
        let _ = send.finish();
    });

    // Inbound. The borrow is moved in so the connection outlives the
    // road: dropping it would close the stream under the exit.
    tokio::spawn(async move {
        let _held = borrow;
        let mut len = [0u8; 2];
        loop {
            if recv.read_exact(&mut len).await.is_err() {
                break;
            }
            let n = u16::from_be_bytes(len) as usize;
            let mut buf = vec![0u8; n];
            if recv.read_exact(&mut buf).await.is_err() {
                break;
            }
            if in_tx.send(buf).await.is_err() {
                break;
            }
        }
    });

    Ok((out_tx, in_rx))
}
