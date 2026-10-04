//! The workspace landing's data layer: the declared workspaces, shaped
//! for the screen. What a workspace *is* is decided in the node
//! (`khor_node::workspace`, `khor_sync::workspaces`); this layer
//! renames fields for TS and decides nothing.
//!
//! # One read, not one question per machine
//!
//! This used to ask every machine for its own list, because the list
//! was derived from records that lived on each disk. It is a
//! replicated table now, so the whole list is on this disk and the
//! landing reads it once — there is no 「那台机器答不了」state to
//! paint, because no machine was asked.
//!
//! What still needs a far machine is what a row *shows*: its git marks
//! and when it last changed. Those are asked per row, by the face, and
//! a machine that cannot answer leaves those two corners blank while
//! the row stays exactly where it is.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// One workspace: a directory on a machine, and whether it leads the
/// list.
///
/// **No `shown` (the `~`-spelled path) and no session count.** The row
/// prints `machine:path` because that pair *is* the identity, and a
/// count of vendor records was a number from the derived list that
/// nothing here derives any more.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WorkspaceRow {
    /// The machine's id, hex.
    pub device: String,
    /// That machine's current display name, empty when the device table
    /// has no row for it — the row is still shown, and still removable
    /// (`khor_node::workspace::WorkspaceInfo`).
    pub machine: String,
    /// Absolute, on that machine.
    pub path: String,
    /// The path's last component.
    pub name: String,
    pub pinned: bool,
    /// **The row's facts** (round NET-8, UX-15), as the machine holding
    /// the files keeps them (`khor_core::WorkspaceFact`): its own
    /// resident's for this machine, the last round's for another. All
    /// absent when that machine has not said yet — an older khor never
    /// does — and the row leaves those slots empty.
    ///
    /// The branch; `None` for a plain folder (and a detached HEAD).
    pub branch: Option<String>,
    /// Files that differ from HEAD, untracked included; `None` for a
    /// plain folder.
    pub changed: Option<u32>,
    /// `changed` stopped at the status walk's cap: say 「N+」.
    pub changed_capped: bool,
    /// When the tree last changed, ms since the epoch.
    #[ts(type = "number | null")]
    pub changed_ms: Option<i64>,
}

/// Every workspace, pinned first then by path — this machine's copy of
/// the table, no network (`khor_node::Node::workspaces`).
pub fn workspaces(root: &Path) -> Result<Vec<WorkspaceRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    // One facts file per machine, read once for all its rows.
    let mut facts: std::collections::HashMap<String, Vec<khor_core::WorkspaceFact>> = Default::default();
    Ok(n.workspaces()?
        .into_iter()
        .map(|w| {
            let known = facts
                .entry(w.device.clone())
                .or_insert_with(|| n.workspace_facts_of(&w.device))
                .iter()
                .find(|f| f.path == w.path)
                .cloned();
            WorkspaceRow {
                branch: known.as_ref().and_then(|f| f.branch.clone()),
                changed: known.as_ref().and_then(|f| f.changed),
                changed_capped: known.as_ref().is_some_and(|f| f.changed_capped),
                changed_ms: known.as_ref().map(|f| f.changed_ms).filter(|&t| t > 0),
                device: w.device,
                machine: w.machine,
                path: w.path,
                name: w.name,
                pinned: w.pinned,
            }
        })
        .collect())
}

/// Declares a directory a workspace. `machine` empty is this one.
/// **The one call here that goes out on the wire**, and only to ask
/// whether that path is a directory (`Node::ws_add`).
pub async fn ws_add(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.ws_add(machine, path).await
}

/// Takes it off the list. **The directory is not touched** (user,
/// 2026-09-14).
///
/// **`device` and not `machine`**, for both of these and for
/// `khor_node::Node::ws_write`'s reason: a row whose machine has left
/// the device table has no name, and it is exactly the row somebody
/// needs to take off. The face has the id on every row, so it hands
/// over the id.
pub fn ws_rm(root: &Path, device: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.ws_rm(device, path)
}

/// Floats it to the top, or lets it back down. `on` is explicit for the
/// reason every pin API's is (`api.ts`).
pub fn ws_pin(root: &Path, device: &str, path: &str, on: bool) -> Result<(), String> {
    Node::open(root.to_path_buf())?.ws_pin(device, path, on)
}

/// When that directory last changed, ms since the epoch; `0` where
/// there was nothing to date (`khor_node::workspace::touched`).
///
/// Asked per row, of the machine holding the files. A machine that
/// cannot answer refuses, and the face paints no time rather than a
/// wrong one.
pub async fn touched(root: &Path, machine: &str, path: &str) -> Result<i64, String> {
    Node::open(root.to_path_buf())?.touched(machine, path).await
}
