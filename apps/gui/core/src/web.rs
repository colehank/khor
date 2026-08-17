//! The browser landing's data layer (docs/NET.md 借网): pinned pages and
//! the borrow behind opening one. Like [`crate::files`], it renames the
//! node's answers for TS and decides nothing.
//!
//! Opening a page is two halves that live in two skins: the borrow (a
//! lease + a local proxy port) is here, callable from both skins; turning
//! that port into a real browsing window is the tauri skin's alone, since
//! only it has a webview to point at the proxy.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// One pinned page, with the exit machine's current name looked up here —
/// the pin is keyed by device id (`khor_sync::webpins`), and a machine
/// that left the table keeps its pin under a short hex, so an unpin stays
/// reachable from everywhere.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WebPinRow {
    pub device: String,
    pub name: String,
    pub url: String,
}

pub fn web_pins(root: &Path) -> Result<Vec<WebPinRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    let devices = n.devices()?;
    Ok(n
        .web_pins()?
        .into_iter()
        .map(|(device, url)| WebPinRow {
            name: devices
                .iter()
                .find(|d| d.id == device)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| device.chars().take(8).collect()),
            device,
            url,
        })
        .collect())
}

/// Pins or unpins a page against an exit machine. `on` is explicit for
/// the reason every pin API's is (`api.ts`).
pub fn pin_web(root: &Path, machine: &str, url: &str, on: bool) -> Result<(), String> {
    Node::open(root.to_path_buf())?.pin_web(machine, url, on)
}

/// Where a borrow's proxy listens, for a window (or a test) to point at.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WebBorrow {
    /// The borrow session's id — closing it collapses the proxy.
    pub session: String,
    /// `127.0.0.1:<port>`, the proxy a webview sets as its `proxy_url`.
    pub addr: String,
}

/// Opens a borrow of `machine`'s network and answers where its proxy
/// listens. The tauri skin then builds a webview window with this address
/// as its proxy; the dev bridge gets the same answer but has no window to
/// open, which is why the window half lives in the skin and not here.
pub async fn borrow_web(root: &Path, machine: &str) -> Result<WebBorrow, String> {
    let (session, addr) = Node::open(root.to_path_buf())?.borrow(machine).await?;
    Ok(WebBorrow { session, addr })
}
