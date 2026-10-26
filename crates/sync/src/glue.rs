//! The `Doc` methods every loro-backed table implements identically.
//! Extracted at the third copy (chat, devices, seen).

use crate::store::{ImportSpan, MergeReport};
use khor_catalog::msg;
use loro::{ExportMode, LoroDoc, VersionVector};

pub(crate) fn with_peer(peer: u64) -> Result<LoroDoc, String> {
    let doc = LoroDoc::new();
    doc.set_peer_id(peer).map_err(msg::cant_set_peer)?;
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
    merge_report(doc, bytes).map(|_| ())
}

pub(crate) fn merge_report(doc: &LoroDoc, bytes: &[u8]) -> Result<MergeReport, String> {
    if bytes.is_empty() {
        return Ok(MergeReport::default());
    }
    let status = doc.import(bytes).map_err(msg::cant_merge)?;
    let mut applied: Vec<_> = status
        .success
        .iter()
        .map(|(writer, (start, end))| ImportSpan {
            writer: *writer,
            start: *start,
            end: *end,
        })
        .collect();
    let mut pending: Vec<_> = status
        .pending
        .iter()
        .flat_map(|ranges| ranges.iter())
        .map(|(writer, (start, end))| ImportSpan {
            writer: *writer,
            start: *start,
            end: *end,
        })
        .collect();
    applied.sort_by_key(|span| span.writer);
    pending.sort_by_key(|span| span.writer);
    Ok(MergeReport { applied, pending })
}
