//! The browsing tabs, shaped for the screen (docs/SHELL.md 第 2 条).
//!
//! **A rename and nothing else.** What a tab *is*, where the list
//! lives, which one is in front after a close, and who may mint an id
//! are all decided in `khor_node::tabs`; this layer gives the fields TS
//! names and decides nothing — the same line `workspaces` draws, and
//! the same one 「CLI 与 GUI 等价」 asks for: `khor tabs` and this
//! module call the identical functions, so there is no second answer
//! for the two to disagree about.
//!
//! # Why the face does not keep its own copy any more
//!
//! It did, in `localStorage`, and everything worked except the part of
//! the ruling that says the terminal has to be able to list, open and
//! close them. A list only the webview can read is a list nothing else
//! can check or repair.

use std::path::Path;

use khor_node::tabs;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One open tab.
///
/// **`title` is stored, not derived**, and that is what lets a restart
/// draw the whole column before a single page has been asked for
/// (第 2 条:「页面内容可以在点到那枚时才加载」).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TabRow {
    pub id: String,
    /// The exit machine's device id, empty on a tab pointed nowhere.
    pub device: String,
    pub url: String,
    pub title: String,
    /// Put at the top by a person; sorts above every unpinned tab and
    /// cannot be closed until it is unpinned (`khor_node::tabs::pin`).
    pub pinned: bool,
}

/// One kind's tabs and which is in front. `active` is empty only when
/// there are no tabs at all (`khor_node::tabs::Open::settle`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TabsOpen {
    pub tabs: Vec<TabRow>,
    pub active: String,
}

fn shape(open: tabs::Open) -> TabsOpen {
    TabsOpen {
        tabs: open
            .tabs
            .into_iter()
            .map(|t| TabRow {
                id: t.id,
                device: t.device,
                url: t.url,
                title: t.title,
                pinned: t.pinned,
            })
            .collect(),
        active: open.active,
    }
}

pub fn list(root: &Path, kind: &str) -> TabsOpen {
    // The retired 「置顶的网页 / 置顶的目录」 rows become sidebar pins the
    // first time this process looks at the tabs (`Node::migrate_place_pins`,
    // once per row, so a later process moves only what has arrived since).
    // Once per process rather than per call: the column polls this.
    static MOVED: std::sync::Once = std::sync::Once::new();
    MOVED.call_once(|| {
        if let Ok(n) = khor_node::Node::open(root.to_path_buf()) {
            let _ = n.migrate_place_pins();
        }
    });
    shape(tabs::list(root, kind))
}

/// Opens one and puts it in front; answers its id. Both arguments may
/// be empty — that is the fresh tab the 「+」 mints.
pub fn open(root: &Path, kind: &str, device: &str, url: &str) -> Result<String, String> {
    tabs::open(root, kind, device, url)
}

/// Points an open tab somewhere and names it. Refused by name for a tab
/// that is not there.
pub fn go(
    root: &Path,
    kind: &str,
    id: &str,
    device: &str,
    url: &str,
    title: &str,
) -> Result<TabsOpen, String> {
    tabs::go(root, kind, id, device, url, title)?;
    Ok(list(root, kind))
}

pub fn focus(root: &Path, kind: &str, id: &str) -> Result<TabsOpen, String> {
    tabs::focus(root, kind, id)?;
    Ok(list(root, kind))
}

/// Pins or unpins one. A pinned tab refuses to close (`khor_node::tabs`).
pub fn pin(root: &Path, kind: &str, id: &str, on: bool) -> Result<TabsOpen, String> {
    tabs::pin(root, kind, id, on)?;
    Ok(list(root, kind))
}

pub fn close(root: &Path, kind: &str, id: &str) -> Result<TabsOpen, String> {
    tabs::close(root, kind, id)?;
    Ok(list(root, kind))
}

/// The whole list at once — what a drag that reorders the column
/// spells. **Ids are still the node's to mint**: a row carrying one
/// this machine never minted is refused rather than trusted.
pub fn save_all(root: &Path, kind: &str, want: &TabsOpen) -> Result<TabsOpen, String> {
    let open = tabs::Open {
        tabs: want
            .tabs
            .iter()
            .map(|t| tabs::Tab {
                id: t.id.clone(),
                device: t.device.clone(),
                url: t.url.clone(),
                title: t.title.clone(),
                pinned: t.pinned,
                // Kept from the file by `save_all`, like `pinned`.
                touched: 0,
            })
            .collect(),
        active: want.active.clone(),
    };
    tabs::save_all(root, kind, &open)?;
    Ok(list(root, kind))
}
