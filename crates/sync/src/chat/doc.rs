//! The document model of one conversation.
//!
//! Shape: `doc.msgs : List<Map{id, at, from_id, from_name, kind, ...}>`.
//! A message is a map, not a JSON string: maps merge per key, so a
//! concurrent edit and retract both survive, where a string is whole-value
//! LWW and silently drops one side. Unknown keys are kept and forwarded —
//! an old version relaying newer content must not strip it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use loro::{LoroDoc, LoroList, LoroMap, LoroValue, UndoManager, VersionVector};

use crate::glue;

/// The one place this list is named.
const MSGS: &str = "msgs";

/// Who spoke. The name travels with the message: history shows the name
/// at the time of writing, and must not rewrite itself on a later rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    pub id: String,
    pub name: String,
}

/// Attachment metadata. Bytes never enter the document — they go through
/// a transfer, fetched on demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
    pub name: String,
    pub size: i64,
    /// Content digest for "do I already have this". Empty = not computed.
    pub digest: String,
}

/// Message body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsgBody {
    Text(String),
    Files(Vec<FileRef>),
    /// A kind this version cannot read, carrying the original kind name.
    /// Explicit, so the UI can say so — an unreadable message must not
    /// fall into the neighbouring arm and render as blank text.
    Unknown(String),
}

/// One message as read out of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    /// Unix ms on the sender's clock, not the reader's.
    pub at: i64,
    pub from: Sender,
    pub body: MsgBody,
    /// Retracted. The slot stays — see [`ChatDoc::retract`].
    pub retracted: bool,
    /// Body was edited; the UI draws "edited" from this.
    pub edited: bool,
}

/// One conversation.
pub struct ChatDoc {
    inner: LoroDoc,
    undo: UndoManager,
    /// Suffix for ids minted in the same millisecond. Atomic rather than
    /// `Cell` — `Cell` makes the type `!Sync` and it must cross an await
    /// in any real transport.
    seq: AtomicU64,
}

impl ChatDoc {
    /// Opens an empty conversation.
    ///
    /// `peer` identifies **a live writer**, not a device. Two live writers
    /// sharing a peer lose data: their ops carry the same (author,
    /// counter) pairs and merge drops one side without error. One device
    /// can host several live writers at once (GUI open plus a CLI run),
    /// so never derive it from the device id — see the test
    /// `sharing_one_peer_between_two_writers_silently_loses_a_message`.
    /// Display identity lives on [`Sender::id`], not here.
    ///
    /// Cost: every peer ever used adds one entry to the version vector.
    /// Accepted — a few KB of metadata versus lost data.
    pub fn new(peer: u64) -> Result<Self, String> {
        let inner = glue::with_peer(peer)?;
        let undo = UndoManager::new(&inner);
        Ok(Self {
            inner,
            undo,
            seq: AtomicU64::new(0),
        })
    }

    /// A throwaway peer for merge-only use. Answering a sync request
    /// writes no ops, so the live-writer contract above doesn't apply and
    /// the version vector doesn't grow (only writes create author slots).
    ///
    /// pid + nanos: this needs to not collide, not to be unpredictable —
    /// no rand.
    pub fn fresh_peer() -> u64 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        (u64::from(std::process::id()) << 40) ^ nanos
    }

    /// The raw document, for tests that hand-craft future messages.
    #[cfg(test)]
    pub(crate) fn raw(&self) -> &LoroDoc {
        &self.inner
    }

    fn msgs(&self) -> LoroList {
        self.inner.get_list(MSGS)
    }

    /// `{peer:016x}-{ms:x}-{seq:x}`. Uniqueness rides on `peer` (already
    /// globally unique), ms covers restarts, seq covers same-ms bursts —
    /// no uuid needed.
    fn next_id(&self) -> String {
        let at = now_ms();
        let s = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("{:016x}-{:x}-{:x}", self.inner.peer_id(), at, s)
    }

    /// Tells the conversation a line. Returns the message id.
    pub fn tell(&self, from: &Sender, text: &str) -> Result<String, String> {
        self.push(from, "text", |m| {
            m.insert("text", text).map_err(|e| e.to_string())
        })
    }

    /// Sends files — metadata only; bytes travel a transfer.
    pub fn send_files(&self, from: &Sender, files: &[FileRef]) -> Result<String, String> {
        self.push(from, "files", |m| {
            let list = LoroList::new();
            for f in files {
                let fm = LoroMap::new();
                fm.insert("name", f.name.as_str()).map_err(|e| e.to_string())?;
                fm.insert("size", f.size).map_err(|e| e.to_string())?;
                fm.insert("digest", f.digest.as_str())
                    .map_err(|e| e.to_string())?;
                list.push_container(fm).map_err(|e| e.to_string())?;
            }
            m.insert_container("files", list).map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    fn push(
        &self,
        from: &Sender,
        kind: &str,
        fill: impl FnOnce(&LoroMap) -> Result<(), String>,
    ) -> Result<String, String> {
        let id = self.next_id();
        let m = LoroMap::new();
        m.insert("id", id.as_str()).map_err(|e| e.to_string())?;
        m.insert("at", now_ms()).map_err(|e| e.to_string())?;
        m.insert("from_id", from.id.as_str())
            .map_err(|e| e.to_string())?;
        m.insert("from_name", from.name.as_str())
            .map_err(|e| e.to_string())?;
        m.insert("kind", kind).map_err(|e| e.to_string())?;
        fill(&m)?;
        self.msgs()
            .push_container(m)
            .map_err(|e| format!("加不进去: {e}"))?;
        self.inner.commit();
        Ok(id)
    }

    /// Edits a message's body. Rewrites only the `text` key — a whole-map
    /// rewrite would overwrite a concurrent retract from another device.
    pub fn edit(&self, id: &str, text: &str) -> Result<(), String> {
        let m = self.find(id).ok_or_else(|| format!("没有这条: {id}"))?;
        m.insert("text", text).map_err(|e| e.to_string())?;
        m.insert("edited", true).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Retracts a message. Soft delete: sets a flag, keeps the slot.
    /// Hard removal breaks context (replies point at nothing) and turns
    /// un-retract into a reinsert-at-position problem. This is not
    /// "delete for me" — that would be a machine-local hide list, a
    /// different verb.
    pub fn retract(&self, id: &str) -> Result<(), String> {
        let m = self.find(id).ok_or_else(|| format!("没有这条: {id}"))?;
        m.insert("retracted", true).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Undoes the last step; returns whether anything was undone.
    ///
    /// Undoes only this writer's own ops (loro `UndoManager` semantics);
    /// concurrent ops from others stay untouched. Covers only this
    /// instance's lifetime — history loaded from disk is not on the stack,
    /// and that is right: after a restart the user wants "delete", not
    /// "undo".
    pub fn undo(&mut self) -> Result<bool, String> {
        let did = self.undo.undo().map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(did)
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    fn find(&self, id: &str) -> Option<LoroMap> {
        let list = self.msgs();
        for i in 0..list.len() {
            let Some(m) = list.get(i).and_then(as_map) else {
                continue;
            };
            if str_of(&m, "id").as_deref() == Some(id) {
                return Some(m);
            }
        }
        None
    }

    /// Current messages, in list order.
    pub fn messages(&self) -> Vec<Message> {
        let list = self.msgs();
        (0..list.len())
            .filter_map(|i| list.get(i).and_then(as_map))
            .map(|m| read_msg(&m))
            .collect()
    }

    // ── the two halves of sync ──────────────────────────────

    /// Where I am; the far side computes what I lack from it.
    pub fn version(&self) -> VersionVector {
        self.inner.oplog_vv()
    }

    /// What I have beyond `theirs`. These bytes are the transferable unit:
    /// a stream frame or a file on disk, both work.
    pub fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        glue::changes_since(&self.inner, theirs)
    }

    /// Full state. Used only for the first flush and compaction.
    pub fn snapshot(&self) -> Result<Vec<u8>, String> {
        glue::snapshot(&self.inner)
    }

    /// Merges bytes in. Idempotent: the same bytes twice equal once.
    pub fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        glue::merge(&self.inner, bytes)
    }
}

impl crate::store::Doc for ChatDoc {
    fn open(peer: u64) -> Result<Self, String> {
        ChatDoc::new(peer)
    }

    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }

    fn version(&self) -> VersionVector {
        ChatDoc::version(self)
    }

    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        ChatDoc::changes_since(self, theirs)
    }

    fn snapshot(&self) -> Result<Vec<u8>, String> {
        ChatDoc::snapshot(self)
    }

    fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        ChatDoc::merge(self, bytes)
    }

    fn items(&self) -> usize {
        self.messages().len()
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn as_map(v: loro::ValueOrContainer) -> Option<LoroMap> {
    v.into_container().ok()?.into_map().ok()
}

fn str_of(m: &LoroMap, k: &str) -> Option<String> {
    match m.get(k)?.into_value().ok()? {
        LoroValue::String(s) => Some(s.to_string()),
        _ => None,
    }
}

fn i64_of(m: &LoroMap, k: &str) -> Option<i64> {
    match m.get(k)?.into_value().ok()? {
        LoroValue::I64(n) => Some(n),
        LoroValue::Double(d) => Some(d as i64),
        _ => None,
    }
}

fn bool_of(m: &LoroMap, k: &str) -> bool {
    matches!(
        m.get(k).and_then(|v| v.into_value().ok()),
        Some(LoroValue::Bool(true))
    )
}

fn read_msg(m: &LoroMap) -> Message {
    let kind = str_of(m, "kind").unwrap_or_default();
    let body = match kind.as_str() {
        "text" => MsgBody::Text(str_of(m, "text").unwrap_or_default()),
        "files" => MsgBody::Files(read_files(m)),
        // Unknown kinds stay explicit; see MsgBody::Unknown.
        other => MsgBody::Unknown(other.to_string()),
    };
    Message {
        id: str_of(m, "id").unwrap_or_default(),
        at: i64_of(m, "at").unwrap_or(0),
        from: Sender {
            id: str_of(m, "from_id").unwrap_or_default(),
            name: str_of(m, "from_name").unwrap_or_default(),
        },
        body,
        retracted: bool_of(m, "retracted"),
        edited: bool_of(m, "edited"),
    }
}

fn read_files(m: &LoroMap) -> Vec<FileRef> {
    let Some(list) = m
        .get("files")
        .and_then(|v| v.into_container().ok())
        .and_then(|c| c.into_list().ok())
    else {
        return Vec::new();
    };
    (0..list.len())
        .filter_map(|i| list.get(i).and_then(as_map))
        .map(|f| FileRef {
            name: str_of(&f, "name").unwrap_or_default(),
            size: i64_of(&f, "size").unwrap_or(0),
            digest: str_of(&f, "digest").unwrap_or_default(),
        })
        .collect()
}


#[cfg(test)]
mod tests;
