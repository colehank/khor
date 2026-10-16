//! "Where was I?" — the data layer behind both landings' recent lists
//! (`khor_sync::recents`). Like [`crate::files`] it renames the node's
//! answers for TS and decides nothing: the order, the tombstones and
//! the cap are all the node's.
//!
//! One module for two landings because it is one table: a directory and
//! a page are the same decision about the same person, told apart by a
//! word in the key, and every read asks for one kind at a time.

use std::path::Path;

use khor_node::{Node, PlaceKind};
use serde::Serialize;
use ts_rs::TS;

/// One place, with the machine's current name looked up here — the
/// record is keyed by device id (`khor_sync::recents`). A machine that
/// left the table gets a short hex display name; dismissal uses the
/// full `device` identity, independently of that display name.
#[derive(Debug, Clone, Serialize, TS)]
pub struct RecentRow {
    pub device: String,
    pub name: String,
    /// The absolute path, or the URL.
    pub place: String,
    /// ms; `number` on the TS side, for `SessionRow::at_ms`'s reason.
    #[ts(type = "number")]
    pub at_ms: i64,
    /// The session it was last visited from, where one was said — the
    /// 附栏's slice (UX-37 §1); `null` when none was.
    pub session: Option<String>,
}

/// The kind word both skins spell on the way in.
///
/// Refused by name rather than through the catalog, on the same footing
/// as `api.rs`'s missing-argument refusals: this word is written by the
/// caller's code and never by a person, so there is no sentence a user
/// could be shown that would help them. Refused rather than ignored
/// because every read filters by kind — a record stored under a word
/// nothing reads back would look exactly like a visit that took.
fn kind_of(word: &str) -> Result<PlaceKind, String> {
    PlaceKind::parse(word).ok_or_else(|| format!("no such place kind: {word}"))
}

pub fn recents(root: &Path, kind: &str, limit: usize) -> Result<Vec<RecentRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    let devices = n.devices()?;
    Ok(n.recents(kind_of(kind)?, limit)?
        .into_iter()
        .map(|p| RecentRow {
            name: devices
                .iter()
                .find(|d| d.id == p.machine)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| p.machine.chars().take(8).collect()),
            device: p.machine,
            place: p.place,
            at_ms: p.at_ms,
            session: p.session,
        })
        .collect())
}

/// Records a visit. Called when a tab actually lands somewhere, never
/// when one is merely opened: the list is places a person has been.
pub fn recent_visit(root: &Path, kind: &str, machine: &str, place: &str, session: Option<&str>) -> Result<(), String> {
    Node::open(root.to_path_buf())?.recent_visit_from(kind_of(kind)?, machine, place, session)
}

/// "Stop suggesting this." Travels like every other decision, and going
/// back there puts it back.
pub fn forget_recent(root: &Path, kind: &str, machine: &str, place: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.forget_recent(kind_of(kind)?, machine, place)
}

/// Dismisses a row by its stable ledger identity, including orphaned
/// devices. Only the display name depends on the current device table.
pub fn forget_recent_by_device(root: &Path, kind: &str, device: &str, place: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.forget_recent_by_device(kind_of(kind)?, device, place)
}

#[cfg(test)]
mod tests {
    use super::kind_of;

    /// The refusal is the point: a kind nothing reads back would store a
    /// visit that never shows, which reads as the feature being broken
    /// somewhere else entirely. Both known words must pass, or this
    /// test would go green on a `kind_of` that refuses everything.
    #[test]
    fn only_a_known_kind_is_a_kind() {
        assert!(kind_of("dir").is_ok(), "control: dir is a kind");
        assert!(kind_of("page").is_ok(), "control: page is a kind");
        assert!(kind_of("file").is_ok(), "control: file is a kind (UX-37 §1)");
        for word in ["", "Dir", "dirs", "files", "url"] {
            assert!(kind_of(word).is_err(), "must refuse: {word:?}");
        }
    }
}
