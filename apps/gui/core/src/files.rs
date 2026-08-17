//! The files landing's data layer: one machine's directory, shaped for
//! the screen. The order, the cap and the "where is this about" all
//! come from the node ([`khor_node::files`]) — this layer renames
//! fields for TS and decides nothing.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// A directory answer: where it is about (absolute, the far machine's
/// own spelling — the asker may have said `""` for home), its rows in
/// the node's order, and whether the cap cut it short.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirRow>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirRow {
    pub name: String,
    pub dir: bool,
    #[ts(type = "number")]
    pub size: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
}

/// One machine's directory — the call `khor ls` makes. Async because
/// the machine may be far away and the answer rides a dial.
pub async fn ls(root: &Path, machine: &str, path: &str) -> Result<DirListing, String> {
    let n = Node::open(root.to_path_buf())?;
    let (path, entries, truncated) = n.ls_of(machine, path).await?;
    Ok(DirListing {
        path,
        entries: entries
            .into_iter()
            .map(|e| DirRow { name: e.name, dir: e.dir, size: e.size, at_ms: e.at_ms })
            .collect(),
        truncated,
    })
}

/// Takes a file off a machine into this one's downloads directory —
/// the call `khor pull` makes, with the landing the desktop convention
/// picks (`~/Downloads`, falling back to home). Answers where it
/// landed: the place was chosen silently, so it must be said.
pub async fn pull(root: &Path, machine: &str, path: &str) -> Result<String, String> {
    let n = Node::open(root.to_path_buf())?;
    let dir = dirs_download();
    let (_, dest) = n.pull_path(machine, path, &dir).await?;
    Ok(dest.display().to_string())
}

fn dirs_download() -> std::path::PathBuf {
    let home = std::env::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dl = home.join("Downloads");
    if dl.is_dir() { dl } else { home }
}

/// One pinned directory, with the machine's current name looked up
/// here — the pin is keyed by device id (`khor_sync::dirpins`), and a
/// machine that left the table keeps its pin under a short hex: the
/// path is still real on that disk, and hiding it would make an unpin
/// impossible from everywhere.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DirPinRow {
    pub device: String,
    pub name: String,
    pub path: String,
}

pub fn dir_pins(root: &Path) -> Result<Vec<DirPinRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    let devices = n.devices()?;
    Ok(n
        .dir_pins()?
        .into_iter()
        .map(|(device, path)| DirPinRow {
            name: devices
                .iter()
                .find(|d| d.id == device)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| device.chars().take(8).collect()),
            device,
            path,
        })
        .collect())
}

/// Pins or unpins a directory — the call `khor pin -d` makes. `on` is
/// explicit for the reason every pin API's is (`api.ts`).
pub fn pin_dir(root: &Path, machine: &str, path: &str, on: bool) -> Result<(), String> {
    Node::open(root.to_path_buf())?.pin_dir(machine, path, on)
}
