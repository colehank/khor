//! The `Doc` methods every loro-backed table implements identically.
//! Extracted at the third copy (chat, devices, seen).

use khor_catalog::msg;
use loro::{ExportMode, LoroDoc, VersionVector};

pub(crate) fn with_peer(peer: u64) -> Result<LoroDoc, String> {
    let doc = LoroDoc::new();
    doc.set_peer_id(peer)
        .map_err(msg::cant_set_peer)?;
    Ok(doc)
}

pub(crate) fn changes_since(doc: &LoroDoc, theirs: &VersionVector) -> Result<Vec<u8>, String> {
    doc.export(ExportMode::updates(theirs))
        .map_err(msg::cant_export_changes)
}

pub(crate) fn snapshot(doc: &LoroDoc) -> Result<Vec<u8>, String> {
    doc.export(ExportMode::Snapshot)
        .map_err(msg::cant_export_snapshot)
}

pub(crate) fn merge(doc: &LoroDoc, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    doc.import(bytes)
        .map(|_| ())
        .map_err(msg::cant_merge)
}
