//! The document model of one conversation.
//!
//! Shape: `doc.msgs : List<Map{id, at, from_id, from_name, kind, ...}>`.
//! A message is a map, not a JSON string: maps merge per key, so a
//! concurrent edit and retract both survive, where a string is whole-value
//! LWW and silently drops one side. Unknown keys are kept and forwarded —
//! an old version relaying newer content must not strip it.
//!
//! **A file message's move is written by each machine that pulls it**:
//! how it stands (`step`, `moved`, `why`, `via`) and where its files
//! landed. Every device reads the same record, so every device shows the
//! same state. ⚠️ 墓碑 (STORE-BRIEF phase 5): a machine's conversation had
//! one receiver, whose record sat on the message itself beside the
//! sender's `into`; that conversation is gone, and with it those keys.
//!
//! **The broadcast has many receivers** (STORE-BRIEF phase 3): a file
//! there is answered by every machine on its own, so its record is one
//! fate per receiver — a key `fate:<device
//! id>` on the message map, holding the same four facts as one plain
//! value. Each receiver writes only its own key, so two machines landing
//! the same file at once never touch each other's record. Not a nested
//! map: two machines creating one child container at once would race,
//! and the loser's whole record would vanish.
//!
//! **A message names its targets** (STORE-BRIEF phase 5, one channel):
//! `to` is a list of `{device, path}` — the machines the sender pushed it
//! to, by device id, and the directory on each (empty for its receive
//! location). Written once, when the message is composed, and never
//! again: the names a person typed were resolved then, so a machine
//! renamed later is shown by its new name and the message still means
//! the same machine. A file names its `holder`, the device that serves
//! its bytes — empty for the sender; another machine's file (`hinton:/x`)
//! is pulled from hinton, not through the sender. A files message may
//! carry words too (`text`, read as [`Message::note`]).
//!
//! **The document's head** (foundation B): a root map `head` that names
//! the machine running a khor-wide session (`home`), empty for one every
//! machine answers itself (the broadcast). The khor agent's conversation
//! is the first that will name one.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use khor_catalog::msg;
use loro::{LoroDoc, LoroList, LoroMap, LoroValue, UndoManager, VersionVector};

use crate::glue;

/// The one place this list is named.
const MSGS: &str = "msgs";
/// The document's head: facts about the conversation itself.
const HEAD: &str = "head";
/// A receiver's own fate key on a message map, followed by its device id.
const FATE_OF: &str = "fate:";

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
    /// The device that serves the bytes, by id: empty for the message's
    /// sender, another machine's id for a file that machine offered when
    /// the message was composed (`hinton:/data/x.bin`).
    pub holder: String,
}

/// A machine a message was pushed to, and where on it (module head).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The machine, by device id — never by name.
    pub device: String,
    /// The directory the files land in there, as the sender wrote it
    /// (`~/data`, `/srv/in`); empty for that machine's receive location.
    pub path: String,
}

/// How a file message's move stands, as its receiver last wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Bytes are moving.
    Moving,
    /// Every file is on the receiver.
    Landed,
    /// The move broke; `why` says how. A retry carries on from the bytes
    /// already there.
    Failed,
    /// Someone cancelled it; nothing partial was kept.
    Stopped,
}

impl Step {
    pub fn key(&self) -> &'static str {
        match self {
            Step::Moving => "moving",
            Step::Landed => "landed",
            Step::Failed => "failed",
            Step::Stopped => "stopped",
        }
    }

    pub fn from_key(k: &str) -> Option<Step> {
        Some(match k {
            "moving" => Step::Moving,
            "landed" => Step::Landed,
            "failed" => Step::Failed,
            "stopped" => Step::Stopped,
            _ => return None,
        })
    }
}

/// The receiver's account of a file message's move. Absent until the
/// receiver has started it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fate {
    pub step: Step,
    /// Bytes on the receiver so far, all files together.
    pub moved: i64,
    /// Where each file landed on the receiver, in the message's order;
    /// empty until it has.
    pub landed: Vec<String>,
    /// Why it failed; empty otherwise.
    pub why: String,
    /// The device id the bytes were pulled from once landed: the sender,
    /// or a machine that landed them before (a broadcast's holder,
    /// BCAST-BRIEF); empty otherwise. Not `from`: that key is the
    /// message's sender.
    pub via: String,
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

/// A command's durable receipt. Only its submitting executor updates
/// this value; importing it is never an instruction to execute it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct CommandRecord {
    pub id: String,
    pub line: String,
    pub owner: String,
    pub owner_name: String,
    pub status: String,
    #[ts(type = "number")]
    pub started_at_ms: i64,
    #[ts(type = "number | null")]
    pub ended_at_ms: Option<i64>,
    pub stdout: String,
    pub stderr: String,
    pub data_json: Option<String>,
    pub session: Option<String>,
    pub error: Option<String>,
    /// A khor agent turn uses the same durable, executor-owned receipt.
    /// Absent on command receipts written by older versions.
    #[serde(default)]
    pub agent: Option<AgentTurn>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct AgentConfig {
    pub home: String,
    pub account: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct AgentTurn {
    pub model: String,
    pub approval: Option<AgentApproval>,
    #[serde(default)]
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct AgentApproval {
    pub id: String,
    pub tool: String,
    pub arguments: String,
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
    /// In a khor-wide conversation, each receiver's own account of its
    /// move, by device id; a machine that has not answered the offer is
    /// absent.
    pub fates: BTreeMap<String, Fate>,
    /// The machines this message was pushed to, in the order they were
    /// named; empty for a message to nobody in particular.
    pub to: Vec<Target>,
    /// The words said along with a files message; empty otherwise (a
    /// text message's words are its body).
    pub note: String,
    pub command: Option<CommandRecord>,
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

    /// Records one command submission with a stable id chosen before
    /// execution. Re-delivery updates its receipt rather than appending.
    pub fn command(&self, from: &Sender, record: &CommandRecord) -> Result<String, String> {
        let encoded = serde_json::to_string(record).map_err(|e| e.to_string())?;
        let at = record
            .ended_at_ms
            .unwrap_or(record.started_at_ms)
            .max(record.agent.as_ref().map_or(0, |a| a.updated_at_ms));
        if let Some(m) = self.find(&record.id) {
            if str_of(&m, "from_id").as_deref() != Some(from.id.as_str()) {
                return Err("command receipt belongs to another executor".to_owned());
            }
            if let Some(prior) = str_of(&m, "command").and_then(|s| serde_json::from_str::<CommandRecord>(&s).ok()) {
                if prior == *record || prior.ended_at_ms.is_some() {
                    return Ok(record.id.clone());
                }
            }
            m.insert("command", encoded).map_err(|e| e.to_string())?;
            m.insert("at", at).map_err(|e| e.to_string())?;
            self.inner.commit();
            return Ok(record.id.clone());
        }
        self.push(from, "text", |m| {
            m.insert("id", record.id.as_str()).map_err(|e| e.to_string())?;
            m.insert("at", at).map_err(|e| e.to_string())?;
            m.insert("text", record.line.as_str()).map_err(|e| e.to_string())?;
            m.insert("command", encoded).map_err(|e| e.to_string())
        })?;
        Ok(record.id.clone())
    }

    /// Writes one composed message: words, the machines it is pushed to
    /// and the files it carries, all known now (the module head). A text
    /// message when there are no files, a files message (with the words
    /// as its note) when there are. Returns the message id.
    pub fn post(&self, from: &Sender, text: &str, to: &[Target], files: &[FileRef]) -> Result<String, String> {
        let kind = if files.is_empty() { "text" } else { "files" };
        self.push(from, kind, |m| {
            let err = |e: loro::LoroError| e.to_string();
            if !text.is_empty() || files.is_empty() {
                m.insert("text", text).map_err(err)?;
            }
            if !to.is_empty() {
                let list: Vec<LoroValue> = to
                    .iter()
                    .map(|t| {
                        let map: std::collections::HashMap<String, LoroValue> = [
                            ("device".to_owned(), LoroValue::from(t.device.as_str())),
                            ("path".to_owned(), LoroValue::from(t.path.as_str())),
                        ]
                        .into_iter()
                        .collect();
                        LoroValue::from(map)
                    })
                    .collect();
                m.insert("to", list).map_err(err)?;
            }
            if files.is_empty() {
                return Ok(());
            }
            let list = LoroList::new();
            for f in files {
                let fm = LoroMap::new();
                fm.insert("name", f.name.as_str()).map_err(err)?;
                fm.insert("size", f.size).map_err(err)?;
                fm.insert("digest", f.digest.as_str()).map_err(err)?;
                if !f.holder.is_empty() {
                    fm.insert("holder", f.holder.as_str()).map_err(err)?;
                }
                list.push_container(fm).map_err(err)?;
            }
            m.insert_container("files", list).map_err(err)?;
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
        self.msgs().push_container(m).map_err(msg::cant_append)?;
        self.inner.commit();
        Ok(id)
    }

    /// Edits a message's body. Rewrites only the `text` key — a whole-map
    /// rewrite would overwrite a concurrent retract from another device.
    pub fn edit(&self, id: &str, text: &str) -> Result<(), String> {
        let m = self.find(id).ok_or_else(|| msg::no_such_message(id))?;
        m.insert("text", text).map_err(|e| e.to_string())?;
        m.insert("edited", true).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Writes one receiver's account of a file message's move, in a
    /// khor-wide conversation (the module head): its own key only, and
    /// nothing when it says what the key already holds.
    pub fn set_fate_of(&self, id: &str, receiver: &str, fate: &Fate) -> Result<(), String> {
        let m = self.find(id).ok_or_else(|| msg::no_such_message(id))?;
        let key = format!("{FATE_OF}{receiver}");
        let value: std::collections::HashMap<String, LoroValue> = [
            ("step".to_owned(), LoroValue::from(fate.step.key())),
            ("moved".to_owned(), LoroValue::from(fate.moved)),
            ("why".to_owned(), LoroValue::from(fate.why.as_str())),
            ("via".to_owned(), LoroValue::from(fate.via.as_str())),
            ("landed".to_owned(), LoroValue::from(fate.landed.clone())),
        ]
        .into_iter()
        .collect();
        m.insert(&key, value).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// The machine that runs this conversation, by device id; `None`
    /// when every machine answers for itself (the module head).
    pub fn home(&self) -> Option<String> {
        str_of(&self.inner.get_map(HEAD), "home").filter(|h| !h.is_empty())
    }

    /// Names the machine that runs this conversation; empty clears it.
    /// Moving a home is writing another id here.
    pub fn set_home(&self, device: &str) -> Result<(), String> {
        self.inner
            .get_map(HEAD)
            .insert("home", device)
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    pub fn agent_config(&self) -> Option<AgentConfig> {
        str_of(&self.inner.get_map(HEAD), "agent_config")
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn set_agent_config(&self, config: &AgentConfig) -> Result<(), String> {
        self.inner
            .get_map(HEAD)
            .insert(
                "agent_config",
                serde_json::to_string(config).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Retracts a message. Soft delete: sets a flag, keeps the slot.
    /// Hard removal breaks context (replies point at nothing) and turns
    /// un-retract into a reinsert-at-position problem. This is not
    /// "delete for me" — that would be a machine-local hide list, a
    /// different verb.
    pub fn retract(&self, id: &str) -> Result<(), String> {
        let m = self.find(id).ok_or_else(|| msg::no_such_message(id))?;
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
        fates: read_fates(m),
        to: read_targets(m),
        note: if kind == "files" { str_of(m, "text").unwrap_or_default() } else { String::new() },
        command: str_of(m, "command").and_then(|s| serde_json::from_str(&s).ok()),
    }
}

/// The machines a message was pushed to (`to`, the module head); an
/// entry without a device is skipped rather than read as nobody.
fn read_targets(m: &LoroMap) -> Vec<Target> {
    let Some(LoroValue::List(list)) = m.get("to").and_then(|v| v.into_value().ok()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|t| match t {
            LoroValue::Map(t) => {
                let text = |k: &str| match t.get(k) {
                    Some(LoroValue::String(s)) => s.to_string(),
                    _ => String::new(),
                };
                let device = text("device");
                (!device.is_empty()).then(|| Target { device, path: text("path") })
            }
            _ => None,
        })
        .collect()
}

/// Every receiver's `fate:<device>` key (the module head).
fn read_fates(m: &LoroMap) -> BTreeMap<String, Fate> {
    let mut out = BTreeMap::new();
    m.for_each(|k, v| {
        let Some(device) = k.strip_prefix(FATE_OF) else {
            return;
        };
        let Ok(LoroValue::Map(f)) = v.into_value() else {
            return;
        };
        let text = |k: &str| match f.get(k) {
            Some(LoroValue::String(s)) => s.to_string(),
            _ => String::new(),
        };
        let Some(step) = Step::from_key(&text("step")) else {
            return;
        };
        let moved = match f.get("moved") {
            Some(LoroValue::I64(n)) => *n,
            Some(LoroValue::Double(d)) => *d as i64,
            _ => 0,
        };
        let landed = match f.get("landed") {
            Some(LoroValue::List(l)) => l
                .iter()
                .filter_map(|p| match p {
                    LoroValue::String(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        out.insert(device.to_owned(), Fate { step, moved, landed, why: text("why"), via: text("via") });
    });
    out
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
            holder: str_of(&f, "holder").unwrap_or_default(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
