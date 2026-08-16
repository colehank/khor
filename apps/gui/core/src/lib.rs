//! The GUI's data layer: the same khor-node calls the CLI makes, shaped
//! into rows the frontend can render. Two skins share this module — the
//! tauri commands (apps/gui/src-tauri) and the dev bridge (`bridge` bin)
//! — so what the browser verifies is what the app ships.
//!
//! No judgment lives here: words, ordering, unread all come from the
//! node. The GUI must not re-derive any of it (docs/UX.md 状态呈现).

use std::path::Path;

use khor_node::{Node, SessionId};
use serde::Serialize;
use ts_rs::TS;

/// One list row. `word` is the state *key* (`busy`…); the frontend looks
/// the display word up in the catalog at the last moment (docs/UX.md 文案).
#[derive(Debug, Clone, Serialize, TS)]
pub struct SessionRow {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub word: String,
    /// ms; `number` on the TS side — serde_json sends a number, and
    /// 2^53 outlives every timestamp and unread count in question.
    #[ts(type = "number")]
    pub at_ms: u64,
    #[ts(type = "number")]
    pub unread: u64,
    /// Present on reported rows: the offline axis, never a seventh word.
    pub source: Option<SourceTag>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct SourceTag {
    pub device: String,
    #[ts(type = "number")]
    pub age_ms: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DeviceRow {
    pub id: String,
    pub name: String,
    pub me: bool,
}

fn open(root: &Path) -> Result<Node, String> {
    Node::open(root.to_path_buf())
}

pub fn list_sessions(root: &Path) -> Result<Vec<SessionRow>, String> {
    let rows = open(root)?.sessions()?;
    Ok(rows
        .into_iter()
        .map(|v| SessionRow {
            id: v.session.id.0,
            kind: v.session.kind.0,
            title: v.session.title,
            word: v.session.state.state.key().to_owned(),
            at_ms: v.session.state.at.0,
            unread: v.session.unread,
            source: v.source.map(|(device, age_ms)| SourceTag { device, age_ms }),
        })
        .collect())
}

pub fn list_devices(root: &Path) -> Result<Vec<DeviceRow>, String> {
    let n = open(root)?;
    let me = n.device_str().to_owned();
    Ok(n.devices()?
        .into_iter()
        .map(|d| DeviceRow { me: d.id == me, id: d.id, name: d.name })
        .collect())
}

pub fn seen(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.seen(&SessionId(id.to_owned()))
}

pub fn close_session(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.close(&SessionId(id.to_owned()))
}
