//! The GUI's data layer: the same khor-node calls the CLI makes, shaped
//! into rows the frontend can render. Two skins share this module — the
//! tauri commands (apps/gui/src-tauri) and the dev bridge (`bridge` bin)
//! — so what the browser verifies is what the app ships.
//!
//! No judgment lives here: words, ordering, unread all come from the
//! node. The GUI must not re-derive any of it (docs/UX.md 状态呈现).

use std::collections::HashMap;
use std::path::Path;

use khor_node::{Avatar, Node, SessionId};
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
    /// Pinned somewhere in the network (`khor_sync::pins`). The frontend
    /// paints a mark from this and **does not sort by it** — the rows
    /// arrive in the order the node decided.
    pub pinned: bool,
    /// The face of the machine this session lives on — derived here,
    /// never in the frontend (`khor_core::avatar`). `None` only when the
    /// home device is not in this table at all, and then the row draws a
    /// blank, **not an invented face**: a made-up face gets believed,
    /// and it is not that machine's.
    pub face: Option<Avatar>,
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
    /// This machine's face, in **its own** reported palette. See
    /// `khor_node::Node::face_of`.
    pub face: Option<Avatar>,
    /// Pinned, network-wide. Same rule as `SessionRow::pinned`: a mark to
    /// paint, never an ordering to redo.
    pub pinned: bool,
}

fn open(root: &Path) -> Result<Node, String> {
    Node::open(root.to_path_buf())
}

pub fn list_sessions(root: &Path) -> Result<Vec<SessionRow>, String> {
    let n = open(root)?;
    // Derived once per list, not once per row: a face is a pure function
    // of (id, reported style), and one machine owns many rows.
    let faces = faces_by_id(&n)?;
    Ok(n.sessions()?
        .into_iter()
        .map(|v| SessionRow {
            face: faces.get(&v.session.home.hex()).cloned(),
            id: v.session.id.0,
            kind: v.session.kind.0,
            title: v.session.title,
            word: v.session.state.state.key().to_owned(),
            at_ms: v.session.state.at.0,
            unread: v.session.unread,
            source: v.source.map(|(device, age_ms)| SourceTag { device, age_ms }),
            pinned: v.pinned,
        })
        .collect())
}

pub fn list_devices(root: &Path) -> Result<Vec<DeviceRow>, String> {
    let n = open(root)?;
    let me = n.device_str().to_owned();
    Ok(n.devices()?
        .into_iter()
        .map(|d| DeviceRow {
            me: d.id == me,
            face: n.face_of(&d),
            pinned: d.pinned,
            id: d.id,
            name: d.name,
        })
        .collect())
}

/// Pins or unpins a session — the call `khor pin <session>` makes. One
/// function behind the verb and the button, so the two cannot drift.
pub fn pin_session(root: &Path, id: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_session(&SessionId(id.to_owned()), on)
}

/// Pins or unpins a machine — the call `khor pin -m <machine>` makes.
pub fn pin_device(root: &Path, machine: &str, on: bool) -> Result<(), String> {
    open(root)?.pin_device(machine, on)
}

/// Every known machine's face, keyed the way the device table keys
/// machines. The judgment behind each one — its own reported palette,
/// seeded by its id — belongs to the node; this only indexes them.
fn faces_by_id(n: &Node) -> Result<HashMap<String, Avatar>, String> {
    Ok(n.devices()?
        .into_iter()
        .filter_map(|d| n.face_of(&d).map(|f| (d.id, f)))
        .collect())
}

pub fn seen(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.seen(&SessionId(id.to_owned()))
}

pub fn close_session(root: &Path, id: &str) -> Result<(), String> {
    open(root)?.close(&SessionId(id.to_owned()))
}

/// Leaves a line in a machine's window — the call `khor tell` makes.
pub fn tell(root: &Path, machine: &str, text: &str) -> Result<(), String> {
    open(root)?.tell(machine, text).map(|_| ())
}

/// Issues a one-time pairing ticket. It carries the live endpoint's
/// addresses, so it needs a resident serve — and both skins embed one
/// (the bridge and the app each start `serve` on their own thread), so
/// the GUI can issue a real ticket without a terminal.
pub fn invite(root: &Path) -> Result<String, String> {
    open(root)?.invite()
}

/// Joins with someone's ticket. Async because `Node::pair` dials — and
/// because a resident serve holds this key's one endpoint, the call
/// usually hands off to it over loopback rather than dialing here.
pub async fn pair(root: &Path, ticket: &str) -> Result<String, String> {
    let node = open(root)?;
    node.pair(ticket).await
}
