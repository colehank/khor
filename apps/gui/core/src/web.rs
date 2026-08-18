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

/// Hands a page to whatever this machine opens pages with.
///
/// **The address comes out of an agent's own words** — a conversation
/// pane paints a link because a model wrote one — so the scheme is a
/// whitelist, not a check for the obviously bad. `http` and `https` are
/// pages; `file:` reads this machine's disk, and a scheme this machine
/// happens to have registered can be an action rather than a page (a
/// mail client sending, an app that takes a command). Anything else is
/// refused with its own text, and the face falls back to showing the
/// address so a person can decide for themselves.
///
/// The opener is the platform's, not a tool the user had to install
/// (docs/STACK.md 零依赖 is about **the user's tools**): `open` and
/// `xdg-open` ship with the desktop that would be showing this window.
pub fn open_link(url: &str) -> Result<(), String> {
    let scheme = url.split_once("://").map(|(s, _)| s.to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http") | Some("https")) {
        return Err(khor_catalog::msg::not_a_page(url));
    }
    // A URL is a URL, never a shell word: the opener is executed
    // directly with one argument, so nothing in the address can become
    // a second command.
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod open_link_tests {
    use super::open_link;

    /// The whitelist is the point: what an agent writes is text, and a
    /// scheme this machine has registered can be an action rather than
    /// a page. Only the refusals are asserted here — the allowed side
    /// would open a browser on whoever ran the tests.
    #[test]
    fn only_a_page_is_a_page() {
        for url in [
            "file:///etc/passwd",
            "FILE:///etc/passwd",
            "mailto:someone@example.com",
            "javascript:alert(1)",
            "vscode://file/etc/passwd",
            "/etc/passwd",
            "",
        ] {
            assert!(open_link(url).is_err(), "must refuse: {url}");
        }
    }
}
