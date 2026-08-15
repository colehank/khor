//! The document model of one conversation.
//!
//! Shape: `doc.msgs : List<Map{id, at, from_id, from_name, kind, ...}>`.
//! A message is a map, not a JSON string: maps merge per key, so a
//! concurrent edit and retract both survive, where a string is whole-value
//! LWW and silently drops one side. Unknown keys are kept and forwarded —
//! an old version relaying newer content must not strip it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use loro::{ExportMode, LoroDoc, LoroList, LoroMap, LoroValue, UndoManager, VersionVector};

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
        let inner = LoroDoc::new();
        inner
            .set_peer_id(peer)
            .map_err(|e| format!("设不上 peer id: {e}"))?;
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

    /// The raw document, for `store` only.
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

    /// Says a line. Returns the message id.
    pub fn say(&self, from: &Sender, text: &str) -> Result<String, String> {
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
        self.inner
            .export(ExportMode::updates(theirs))
            .map_err(|e| format!("导不出增量: {e}"))
    }

    /// Full state. Used only for the first flush and compaction.
    pub fn snapshot(&self) -> Result<Vec<u8>, String> {
        self.inner
            .export(ExportMode::Snapshot)
            .map_err(|e| format!("导不出快照: {e}"))
    }

    /// Merges bytes in. Idempotent: the same bytes twice equal once.
    pub fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty() {
            return Ok(());
        }
        self.inner
            .import(bytes)
            .map(|_| ())
            .map_err(|e| format!("合不进来: {e}"))
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
mod tests {
    use std::fs;

    use super::*;
    use crate::chat::testutil::{me, render, tmpdir};

    /// Reproducible pseudo-random — no rand: these tests must replay the
    /// same exchange order across runs, or a red light cannot be
    /// reproduced.
    struct Lcg(u64);
    impl Lcg {
        fn upto(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 33) as usize) % n
        }
    }

    /// Five peers, random partitions, random exchange order — identical
    /// to the letter at the end. The only falsifiable form of
    /// "consistent across devices".
    #[test]
    fn five_peers_converge_after_random_partitions() {
        let docs: Vec<ChatDoc> = (1..=5).map(|i| ChatDoc::new(i).unwrap()).collect();
        let mut rng = Lcg(0x5eed);

        for round in 0..40 {
            let a = rng.upto(5);
            docs[a].say(&me(&format!("p{a}")), &format!("第 {round} 句")).unwrap();
            let (x, y) = (rng.upto(5), rng.upto(5));
            if x != y {
                let up = docs[x].changes_since(&docs[y].version()).unwrap();
                docs[y].merge(&up).unwrap();
            }
        }
        // settle
        for _ in 0..3 {
            for i in 0..5 {
                for j in 0..5 {
                    if i != j {
                        let up = docs[i].changes_since(&docs[j].version()).unwrap();
                        docs[j].merge(&up).unwrap();
                    }
                }
            }
        }

        let first = render(&docs[0]);
        for (i, d) in docs.iter().enumerate() {
            assert_eq!(render(d), first, "第 {i} 个节点和第 0 个不一样");
        }
        assert_eq!(docs[0].messages().len(), 40, "一条都不该丢");

        // Control: an unsynced doc must differ, or "all equal" may just
        // mean render() returns the same string for everything.
        let lonely = ChatDoc::new(99).unwrap();
        lonely.say(&me("nobody"), "自言自语").unwrap();
        assert_ne!(render(&lonely), first, "没同步过的不该和大家一样");
    }

    /// Merging the same block twice equals once — reconnects, replays,
    /// and both transports delivering the same segment all lean on this.
    #[test]
    fn merging_the_same_block_twice_changes_nothing() {
        let a = ChatDoc::new(1).unwrap();
        a.say(&me("a"), "一句").unwrap();
        let block = a.snapshot().unwrap();

        let b = ChatDoc::new(2).unwrap();
        b.merge(&block).unwrap();
        let once = render(&b);
        b.merge(&block).unwrap();
        assert_eq!(render(&b), once, "第二遍不该改变任何东西");
    }

    /// A message with a 200MB attachment syncs as metadata only — the
    /// falsifiable form of "bytes never enter the document".
    #[test]
    fn a_huge_attachment_only_costs_its_metadata() {
        let a = ChatDoc::new(1).unwrap();
        for i in 0..200 {
            a.say(&me("a"), &format!("一句普通的话,第 {i} 条")).unwrap();
        }
        let b = ChatDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();

        let before = b.version();
        a.send_files(
            &me("a"),
            &[FileRef {
                name: "dataset.tar.zst".into(),
                size: 209_715_200,
                digest: "b3:9f2c".repeat(6),
            }],
        )
        .unwrap();

        let delta = a.changes_since(&before).unwrap();
        let snap = a.snapshot().unwrap();
        assert!(
            delta.len() < 1024,
            "增量该在 1KB 以内,实测 {} 字节",
            delta.len()
        );
        assert!(
            delta.len() * 10 < snap.len(),
            "增量至少比全量小一个数量级,否则「增量」名不副实(增量 {} / 全量 {})",
            delta.len(),
            snap.len()
        );
    }

    /// Merging via files must equal merging via a direct stream, to the
    /// letter. This property is the entire reason a document CRDT was
    /// chosen: an ssh-only machine runs no process, so increments can
    /// only lie there as files.
    #[test]
    fn a_dumb_node_that_only_holds_bytes_gives_the_same_result() {
        let a = ChatDoc::new(1).unwrap();
        a.say(&me("mac"), "从 Mac 发的").unwrap();
        let c = ChatDoc::new(3).unwrap();
        c.say(&me("phone"), "从手机发的").unwrap();

        let empty = Default::default();
        let fa = a.changes_since(&empty).unwrap();
        let fc = c.changes_since(&empty).unwrap();

        // The dumb node's directory: each side drops its segment in; the
        // node understands none of it.
        let dir = tmpdir("dumb");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.loro"), &fa).unwrap();
        fs::write(dir.join("c.loro"), &fc).unwrap();

        let viafile = ChatDoc::new(7).unwrap();
        viafile.merge(&fs::read(dir.join("a.loro")).unwrap()).unwrap();
        viafile.merge(&fs::read(dir.join("c.loro")).unwrap()).unwrap();

        let direct = ChatDoc::new(8).unwrap();
        direct.merge(&fa).unwrap();
        direct.merge(&fc).unwrap();

        assert_eq!(render(&viafile), render(&direct), "走文件和直连必须一样");
        assert_eq!(viafile.messages().len(), 2, "两边的消息都得在");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Undo takes back only my own message; a concurrent one from someone
    /// else stays untouched. A "revert to previous version"
    /// implementation sweeps the other one away — and stays green in
    /// single-machine tests.
    #[test]
    fn undo_only_takes_back_my_own_message() {
        let mut a = ChatDoc::new(1).unwrap();
        a.say(&me("me"), "我说的第一句").unwrap();

        let b = ChatDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();
        b.say(&me("peer"), "别人说的一句").unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();

        assert_eq!(a.messages().len(), 2);
        assert!(a.can_undo(), "撤销栈里该有东西");
        assert!(a.undo().unwrap(), "undo 该真的执行");

        let left = render(&a);
        assert!(left.contains("别人说的一句"), "别人那条必须还在:\n{left}");
        assert!(!left.contains("我说的第一句"), "我自己那条该没了:\n{left}");
    }

    /// Retraction is a soft delete: the slot stays, marked. Hard removal
    /// makes the context jump and people think they misremembered.
    #[test]
    fn retracting_keeps_the_slot() {
        let d = ChatDoc::new(1).unwrap();
        d.say(&me("a"), "第一句").unwrap();
        let id = d.say(&me("a"), "说错了").unwrap();
        d.say(&me("a"), "第三句").unwrap();

        d.retract(&id).unwrap();
        let msgs = d.messages();
        assert_eq!(msgs.len(), 3, "位置要留着,不是抠掉");
        assert!(msgs[1].retracted, "中间那条该标着撤回");
        assert!(!msgs[0].retracted && !msgs[2].retracted, "别的不许被带上");
    }

    /// One device edits while another retracts; both must survive. This
    /// pins "a message is a map, not a JSON string": whole-value LWW
    /// lets the later write erase the earlier one, with no error on
    /// either side.
    #[test]
    fn a_concurrent_edit_and_retract_both_survive() {
        let a = ChatDoc::new(1).unwrap();
        let id = a.say(&me("a"), "原话").unwrap();

        let b = ChatDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();

        a.edit(&id, "改过的话").unwrap();
        b.retract(&id).unwrap();

        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();

        for (who, d) in [("a", &a), ("b", &b)] {
            let m = &d.messages()[0];
            assert!(m.retracted, "{who}:撤回该留住");
            assert!(m.edited, "{who}:编辑该留住");
            assert_eq!(
                m.body,
                MsgBody::Text("改过的话".into()),
                "{who}:正文该是改过的那句"
            );
        }
        assert_eq!(render(&a), render(&b), "两台最终必须一样");
    }

    /// An unreadable kind lands in `Unknown`, not `Text`: a missing arm
    /// does not become "no value", it becomes the neighbouring arm — and
    /// a voice message rendered as a blank line errors nowhere.
    #[test]
    fn an_unknown_kind_does_not_land_in_text() {
        let a = ChatDoc::new(1).unwrap();
        // Hand-craft a message from "a future version": this build has
        // no `voice` kind.
        {
            let list = a.raw().get_list("msgs");
            let m = loro::LoroMap::new();
            m.insert("id", "future-1").unwrap();
            m.insert("at", 1i64).unwrap();
            m.insert("from_id", "dev-x").unwrap();
            m.insert("from_name", "x").unwrap();
            m.insert("kind", "voice").unwrap();
            m.insert("seconds", 12i64).unwrap(); // a key this build has never heard of
            list.push_container(m).unwrap();
            a.raw().commit();
        }

        let msgs = a.messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0].body,
            MsgBody::Unknown("voice".into()),
            "不认识的种类要明说,不许落进 Text"
        );

        // And forwarding must not strip it: an old build relaying still
        // hands the unknown key to the next reader.
        let b = ChatDoc::new(2).unwrap();
        b.merge(&a.snapshot().unwrap()).unwrap();
        let c = ChatDoc::new(3).unwrap();
        c.merge(&b.snapshot().unwrap()).unwrap();
        let dumped = format!("{:?}", c.raw().get_deep_value());
        assert!(
            dumped.contains("seconds"),
            "经过一个不认识它的版本之后,那个键还得在:\n{dumped}"
        );
    }

    /// Counter-example, kept on purpose: two writers sharing one peer
    /// lose a message silently. The contract on `ChatDoc::new` cannot
    /// stop the natural-looking "hash the device id" implementation — it
    /// stays green in every single-process test. If loro ever starts
    /// erroring on peer collisions, delete this test and update the
    /// contract; do not invert the assertion.
    #[test]
    fn sharing_one_peer_between_two_writers_silently_loses_a_message() {
        let a = ChatDoc::new(1).unwrap();
        let b = ChatDoc::new(1).unwrap(); // deliberately violates the contract
        a.say(&me("a"), "甲说的").unwrap();
        b.say(&me("b"), "乙说的").unwrap();

        let merged = ChatDoc::new(2).unwrap();
        merged.merge(&a.snapshot().unwrap()).unwrap();
        // No error — which is exactly what makes it dangerous.
        let _ = merged.merge(&b.changes_since(&merged.version()).unwrap());

        let text = render(&merged);
        assert_eq!(
            merged.messages().len(),
            1,
            "共用 peer 时会丢一句——这条测试守的是这个事实,不是这个行为:\n{text}"
        );
    }
}
