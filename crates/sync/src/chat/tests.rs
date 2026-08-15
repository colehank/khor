//! These tests guard the properties the CRDT choice rests on. If one of
//! them stops holding, what breaks is not a feature but "consistent
//! across devices" itself — so they are pinned here, not in a spike.

use std::fs;
use std::path::PathBuf;

use super::doc::{ChatDoc, FileRef, MsgBody, Sender};
use super::store::{channel_dir, valid_channel, ChatStore};

fn me(n: &str) -> Sender {
    Sender {
        id: format!("dev-{n}"),
        name: n.into(),
    }
}

/// Reproducible pseudo-random — no rand: these tests must replay the same
/// exchange order across runs, or a red light cannot be reproduced.
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

/// A comparable flat rendering; convergence is judged on it.
fn render(d: &ChatDoc) -> String {
    d.messages()
        .iter()
        .map(|m| {
            format!(
                "{}|{}|{:?}|{}|{}",
                m.id, m.from.id, m.body, m.retracted, m.edited
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "khor-sync-test-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&p);
    p
}

// ══════════ convergence ══════════

/// Five peers, random partitions, random exchange order — identical to
/// the letter at the end. The only falsifiable form of "consistent
/// across devices".
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

    // Control: an unsynced doc must differ, or "all equal" may just mean
    // render() returns the same string for everything.
    let lonely = ChatDoc::new(99).unwrap();
    lonely.say(&me("nobody"), "自言自语").unwrap();
    assert_ne!(render(&lonely), first, "没同步过的不该和大家一样");
}

/// Merging the same block twice equals once — reconnects, replays, and
/// both transports delivering the same segment all lean on this.
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

// ══════════ increments are actually incremental ══════════

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

// ══════════ dumb node: increments as files ══════════

/// Merging via files must equal merging via a direct stream, to the
/// letter. This property is the entire reason a document CRDT was chosen:
/// an ssh-only machine runs no process, so increments can only lie there
/// as files.
#[test]
fn a_dumb_node_that_only_holds_bytes_gives_the_same_result() {
    let a = ChatDoc::new(1).unwrap();
    a.say(&me("mac"), "从 Mac 发的").unwrap();
    let c = ChatDoc::new(3).unwrap();
    c.say(&me("phone"), "从手机发的").unwrap();

    let empty = Default::default();
    let fa = a.changes_since(&empty).unwrap();
    let fc = c.changes_since(&empty).unwrap();

    // The dumb node's directory: each side drops its segment in; the node
    // understands none of it.
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

// ══════════ undo / retract / edit ══════════

/// Undo takes back only my own message; a concurrent one from someone
/// else stays untouched. A "revert to previous version" implementation
/// sweeps the other one away — and stays green in single-machine tests.
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

/// One device edits while another retracts; both must survive. This pins
/// "a message is a map, not a JSON string": whole-value LWW lets the
/// later write erase the earlier one, with no error on either side.
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

// ══════════ unknown kinds ══════════

/// An unreadable kind lands in `Unknown`, not `Text`: a missing arm does
/// not become "no value", it becomes the neighbouring arm — and a voice
/// message rendered as a blank line errors nowhere.
#[test]
fn an_unknown_kind_does_not_land_in_text() {
    let a = ChatDoc::new(1).unwrap();
    // Hand-craft a message from "a future version": this build has no
    // `voice` kind.
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

    // And forwarding must not strip it: an old build relaying still hands
    // the unknown key to the next reader.
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

// ══════════ persistence ══════════

/// Flush writes only what disk lacks. Without this, "append-only"
/// silently becomes quadratic — the Nth flush writes N histories, and
/// everything still works.
#[test]
fn flush_writes_only_what_is_not_on_disk_yet() {
    let dir = tmpdir("flush");
    let mut st = ChatStore::load(&dir, 1).unwrap();
    for i in 0..50 {
        st.doc.say(&me("a"), &format!("第 {i} 句")).unwrap();
    }
    let first = st.store.flush(&st.doc).unwrap().expect("该写出一个块");
    let big = fs::metadata(&first).unwrap().len();

    st.doc.say(&me("a"), "又一句").unwrap();
    let second = st.store.flush(&st.doc).unwrap().expect("该再写一个块");
    let small = fs::metadata(&second).unwrap().len();

    assert!(
        small * 5 < big,
        "第二块该只装那一句(第一块 {big} 字节,第二块 {small} 字节)"
    );
    // No new content, no file — not even an empty one.
    assert!(st.store.flush(&st.doc).unwrap().is_none(), "没有新东西不该写文件");
    let _ = fs::remove_dir_all(&dir);
}

/// What is flushed comes back, identical.
#[test]
fn what_is_flushed_comes_back() {
    let dir = tmpdir("roundtrip");
    let mut st = ChatStore::load(&dir, 1).unwrap();
    st.doc.say(&me("a"), "一").unwrap();
    st.doc.say(&me("a"), "二").unwrap();
    st.store.flush(&st.doc).unwrap();
    let want = render(&st.doc);

    let back = ChatStore::load(&dir, 1).unwrap();
    assert!(back.broken.is_empty(), "不该有坏块");
    assert_eq!(render(&back.doc), want);
    let _ = fs::remove_dir_all(&dir);
}

/// One broken block neither kills the channel nor disappears silently:
/// it must be counted, so the UI can say a slice is missing.
#[test]
fn one_broken_block_does_not_kill_the_channel() {
    let dir = tmpdir("broken");
    let mut st = ChatStore::load(&dir, 1).unwrap();
    st.doc.say(&me("a"), "好的那句").unwrap();
    st.store.flush(&st.doc).unwrap();
    fs::write(dir.join("u-000000000000ffff-00000000.loro"), b"not a loro block").unwrap();

    let back = ChatStore::load(&dir, 1).unwrap();
    assert_eq!(back.broken.len(), 1, "坏块要被数出来,不能静默吞掉");
    assert_eq!(back.doc.messages().len(), 1, "好的那句还得在");
    let _ = fs::remove_dir_all(&dir);
}

/// Compaction loses not a word, and the old blocks are gone.
#[test]
fn compacting_keeps_every_word() {
    let dir = tmpdir("compact");
    let mut st = ChatStore::load(&dir, 1).unwrap();
    for i in 0..20 {
        st.doc.say(&me("a"), &format!("第 {i} 句")).unwrap();
        st.store.flush(&st.doc).unwrap();
    }
    let want = render(&st.doc);
    // Count blocks only, not the ledger (`.merged` shares the directory):
    // otherwise this measures "files in dir" when it asks "blocks left".
    let count_blocks = |d: &PathBuf| {
        fs::read_dir(d)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
            .count()
    };
    let before = count_blocks(&dir);
    assert!(before >= 20, "压实前该有一堆块,实测 {before}");

    st.store.compact(&st.doc).unwrap();
    let after = count_blocks(&dir);
    assert_eq!(after, 1, "压实后只该剩那个快照,实测 {after}");

    let back = ChatStore::load(&dir, 1).unwrap();
    assert_eq!(render(&back.doc), want, "压实不许丢任何一句");
    let _ = fs::remove_dir_all(&dir);
}

/// Two writers on one device, each with its own peer: both lines survive
/// and the block names don't collide (`next_seq` counts from disk).
/// "GUI open plus a CLI run" is a real scene, not a hypothetical.
#[test]
fn two_writers_on_the_same_device_do_not_collide() {
    let dir = tmpdir("collide");
    // Same directory, same device, different peers — ChatDoc::new's contract.
    let mut one = ChatStore::load(&dir, 0xA1).unwrap();
    let mut two = ChatStore::load(&dir, 0xA2).unwrap();

    one.doc.say(&me("a"), "来自进程一").unwrap();
    let p1 = one.store.flush(&one.doc).unwrap().unwrap();
    two.doc.say(&me("a"), "来自进程二").unwrap();
    let p2 = two.store.flush(&two.doc).unwrap().unwrap();

    assert_ne!(p1, p2, "两个块不许同名");
    let back = ChatStore::load(&dir, 0xA3).unwrap();
    let text = render(&back.doc);
    assert!(
        text.contains("来自进程一") && text.contains("来自进程二"),
        "两句都得在:\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Counter-example, kept on purpose: two writers sharing one peer lose a
/// message silently. The contract on `ChatDoc::new` cannot stop the
/// natural-looking "hash the device id" implementation — it stays green
/// in every single-process test. If loro ever starts erroring on peer
/// collisions, delete this test and update the contract; do not invert
/// the assertion.
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

/// The channel-name whitelist blocks command injection, not just bad
/// filenames — the name travels through a remote shell.
#[test]
fn channel_names_are_whitelisted() {
    for ok in ["turing", "mac-mini.local", "a_b-1.2"] {
        assert!(valid_channel(ok), "{ok} 该放行");
    }
    for bad in ["", ".", "..", "a/b", "a b", "a;rm -rf /", "a$(x)", "a\nb"] {
        assert!(!valid_channel(bad), "{bad:?} 该挡住");
    }
    // Invalid names return None — never "cleaned" here, because two
    // devices cleaning differently would split the channel.
    assert!(channel_dir(&PathBuf::from("/home/x"), "a/b").is_none());
    assert_eq!(
        channel_dir(&PathBuf::from("/home/x"), "turing").unwrap(),
        PathBuf::from("/home/x/.khor/chat/turing")
    );
}

// ══════════ aligning two sides ══════════

use super::plan::{plan, Ledger, Side};

/// A dumb node: a directory holding blocks. It merges nothing.
struct Dumb(PathBuf);
impl Dumb {
    fn new(tag: &str) -> Self {
        let d = tmpdir(tag);
        fs::create_dir_all(&d).unwrap();
        Self(d)
    }
    fn side(&self) -> Side {
        let names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".loro"))
            .collect();
        // `merged` empty: it stores for others, merges nothing.
        Side::new(names, Vec::<String>::new())
    }
    fn get(&self, n: &str) -> Vec<u8> {
        fs::read(self.0.join(n)).unwrap()
    }
    fn put(&self, n: &str, b: &[u8]) {
        fs::write(self.0.join(n), b).unwrap();
    }
}

/// One sync round: plan → move → done. Returns (pulled, pushed) counts.
fn sync_with_dumb(st: &mut ChatStore, doc: &ChatDoc, dumb: &Dumb) -> (usize, usize) {
    let p = plan(&st.side().unwrap(), &dumb.side());
    for n in &p.pull {
        st.absorb(doc, n, &dumb.get(n)).unwrap();
    }
    for n in &p.push {
        dumb.put(n, &st.read_block(n).unwrap());
    }
    (p.pull.len(), p.push.len())
}

/// Pull is judged by "ever merged", not "still on disk".
#[test]
fn the_plan_pulls_what_i_have_not_merged_not_what_i_lack_on_disk() {
    // I compacted: only the snapshot remains, but the ledger remembers.
    let mine = Side::new(
        vec!["snap-00000002.loro".to_string()],
        vec![
            "u-0000000000000001-00000000.loro".to_string(),
            "u-0000000000000001-00000001.loro".to_string(),
            "snap-00000002.loro".to_string(),
        ],
    );
    let theirs = Side::new(
        vec![
            "u-0000000000000001-00000000.loro".to_string(),
            "u-0000000000000001-00000001.loro".to_string(),
        ],
        Vec::<String>::new(),
    );
    let p = plan(&mine, &theirs);
    assert!(
        p.pull.is_empty(),
        "压实掉的块不许被拉回来,否则压实成了一个永远做不完的动作:{:?}",
        p.pull
    );
    assert_eq!(p.push, vec!["snap-00000002.loro"], "我的快照该推过去");
}

/// Push must honour the far side's ledger: merged means they have it,
/// even if their compaction deleted the file.
#[test]
fn the_plan_does_not_push_what_they_already_merged() {
    let mine = Side::new(vec!["u-a.loro".to_string()], vec!["u-a.loro".to_string()]);
    let theirs = Side::new(Vec::<String>::new(), vec!["u-a.loro".to_string()]);
    assert!(
        plan(&mine, &theirs).push.is_empty(),
        "对面合过的不该再推"
    );
    // Control: never-met must be pushed to, or a push-nothing
    // implementation stays green.
    let never = Side::default();
    assert_eq!(plan(&mine, &never).push, vec!["u-a.loro"]);
}

/// Two machines converge through a node that understands nothing — the
/// falsifiable form of "an ssh-only machine is a member of the network".
#[test]
fn two_machines_converge_through_a_node_that_understands_nothing() {
    let dumb = Dumb::new("dumb-node");
    let dir_a = tmpdir("mach-a");
    let dir_b = tmpdir("mach-b");

    let mut a = ChatStore::load(&dir_a, 0xA).unwrap();
    a.doc.say(&me("mac"), "从 Mac 发的").unwrap();
    a.store.flush(&a.doc).unwrap();
    let (pull, push) = sync_with_dumb(&mut a.store, &a.doc, &dumb);
    assert_eq!((pull, push), (0, 1), "A 该推一块上去,没有可拉的");

    let mut b = ChatStore::load(&dir_b, 0xB).unwrap();
    let (pull, _) = sync_with_dumb(&mut b.store, &b.doc, &dumb);
    assert_eq!(pull, 1, "B 该从哑节点拉到 A 那一块");
    b.doc.say(&me("phone"), "从手机发的").unwrap();
    b.store.flush(&b.doc).unwrap();
    sync_with_dumb(&mut b.store, &b.doc, &dumb);

    sync_with_dumb(&mut a.store, &a.doc, &dumb);

    assert_eq!(render(&a.doc), render(&b.doc), "两台最终必须一样");
    assert_eq!(a.doc.messages().len(), 2, "两句都得在");

    // Idempotence: another round moves nothing.
    assert_eq!(sync_with_dumb(&mut a.store, &a.doc, &dumb), (0, 0));
    assert_eq!(sync_with_dumb(&mut b.store, &b.doc, &dumb), (0, 0));

    for d in [&dir_a, &dir_b, &dumb.0] {
        let _ = fs::remove_dir_all(d);
    }
}

/// Syncing after compaction must not pull the compacted blocks back —
/// the directory-as-criterion implementation goes red here, and its
/// failure mode in the wild is "compaction undone on every sync, no
/// error anywhere".
#[test]
fn compacting_survives_the_next_sync() {
    let dumb = Dumb::new("dumb-compact");
    let dir = tmpdir("mach-compact");
    let mut m = ChatStore::load(&dir, 0xC).unwrap();

    for i in 0..5 {
        m.doc.say(&me("a"), &format!("第 {i} 句")).unwrap();
        m.store.flush(&m.doc).unwrap();
    }
    sync_with_dumb(&mut m.store, &m.doc, &dumb); // all five uploaded
    let want = render(&m.doc);

    m.store.compact(&m.doc).unwrap();
    let left = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
        .count();
    assert_eq!(left, 1, "压实后本地只剩快照,实测 {left}");

    let (pull, _) = sync_with_dumb(&mut m.store, &m.doc, &dumb);
    assert_eq!(pull, 0, "压实掉的五块一块都不许拉回来");
    assert_eq!(render(&m.doc), want, "内容一个字不许变");

    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&dumb.0);
}

/// The ledger's text format: one name per line, round-trips.
#[test]
fn the_ledger_round_trips() {
    let mut l = Ledger::default();
    l.insert("u-b.loro");
    l.insert("u-a.loro");
    let text = l.render();
    assert_eq!(text, "u-a.loro\nu-b.loro\n", "有序,一行一个");
    let back = Ledger::parse(&text);
    assert_eq!(back.len(), 2);
    assert!(back.has("u-a.loro") && back.has("u-b.loro"));
    // Blank lines and whitespace must not become entries.
    assert_eq!(Ledger::parse("\n  \n u-a.loro \n\n").len(), 1);
}

// ── the live half (wire) ──────────────────────────────────

use super::wire::{self, Peer, Reply, MAX_BYTES};

/// An in-memory far side. It follows the answering order the real driver
/// must use — compute the outgoing segment first, then merge the incoming
/// one — and stays short so it is a second reader of that order, not a
/// second implementation.
struct Far {
    doc: ChatDoc,
}

impl Far {
    fn new(peer: u64) -> Self {
        Self {
            doc: ChatDoc::new(peer).unwrap(),
        }
    }

    fn answer(&self, have: &str, changes: &str) -> Result<Reply, String> {
        let out = wire::changes_since_b64(&self.doc, have)?;
        wire::merge_b64(&self.doc, changes)?;
        Ok(Reply {
            version: wire::version_b64(&self.doc),
            changes: out,
            messages: self.doc.messages().len(),
        })
    }
}

/// Convergence, and the push happens on round two — removing the
/// "first round never pushes" rule makes round one's `pushed` non-zero
/// and this goes red on the spot.
#[test]
fn a_peer_pulls_first_then_pushes() {
    let dir = tmpdir("wire-round");
    let mut mine = ChatStore::load(&dir, 0x51).unwrap();
    mine.doc.say(&me("我"), "这边说的").unwrap();
    mine.store.flush(&mine.doc).unwrap();

    let far = Far::new(0x52);
    far.doc.say(&me("远"), "那边说的").unwrap();

    let mut peer = Peer::new();
    let r1 = peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert_eq!(r1.pushed, 0, "第一趟不许推(还不知道对方到哪儿)");
    assert!(r1.pulled > 0, "第一趟该把对方那句拉回来");
    assert_eq!(r1.messages, 2, "我这边现在两条");
    assert_eq!(far.doc.messages().len(), 1, "对方还没收到我这句");

    let r2 = peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert!(r2.pushed > 0, "第二趟该把我那句推过去");
    assert_eq!(far.doc.messages().len(), 2, "对方收到了");
    assert_eq!(render(&mine.doc), render(&far.doc), "两边逐字相同");

    let _ = fs::remove_dir_all(&dir);
}

/// Control: a settled pair moves zero bytes in both directions. Blocks
/// the full-resync-every-round implementation (green above, quietly
/// hauling the whole history each time), and pins the version-inclusion
/// emptiness criterion in `changes_since_b64` — the bytes-empty criterion
/// goes red here because loro always exports a header block.
#[test]
fn a_settled_pair_moves_nothing() {
    let dir = tmpdir("wire-idle");
    let mut mine = ChatStore::load(&dir, 0x61).unwrap();
    mine.doc.say(&me("我"), "一句").unwrap();
    let far = Far::new(0x62);

    let mut peer = Peer::new();
    for _ in 0..3 {
        peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    }
    let r = peer.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert_eq!((r.pushed, r.pulled), (0, 0), "稳态下两个方向都空,实测 {r:?}");
    assert_eq!(r.messages, 1);

    let _ = fs::remove_dir_all(&dir);
}

/// Remembering the far side's version saves a round trip — not bytes.
/// This test replaced a claim of "forgetting resends the whole history",
/// which it refuted on the spot: the amnesiac's first pull recovers the
/// version and its second round computes the identical delta. Kept so
/// the exaggerated version doesn't get written back and send someone
/// optimizing a cost that doesn't exist.
#[test]
fn remembering_saves_a_round_trip_not_bytes() {
    let dir = tmpdir("wire-amnesia");
    let mut mine = ChatStore::load(&dir, 0x71).unwrap();
    for i in 0..30 {
        mine.doc.say(&me("我"), &format!("第 {i} 句")).unwrap();
    }
    let far = Far::new(0x72);

    // The one who remembers: pushes on round two, zero after.
    let mut good = Peer::new();
    good.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    let push = good.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert!(push.pushed > 0, "第二趟该把 30 句推过去");

    // Say one more; the rememberer delivers in one round.
    mine.doc.say(&me("我"), "新的一句").unwrap();
    let one = good.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert!(one.pushed > 0, "记得对方到哪儿,所以这一趟直接推");
    assert_eq!(far.doc.messages().len(), 31, "一趟就送到了");

    // A fresh restart: same one new line — same length as the last, so a
    // byte difference can't be misread as anything but round count.
    mine.doc.say(&me("我"), "再来一句").unwrap();
    let mut fresh = Peer::new();
    let a = fresh.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert_eq!(a.pushed, 0, "刚重启的第一趟是纯拉");
    assert_eq!(far.doc.messages().len(), 31, "所以这一趟对方还没收到");
    let b = fresh.round(&mut mine.store, &mine.doc, |h, c| far.answer(h, c)).unwrap();
    assert!(b.pushed > 0);
    assert_eq!(far.doc.messages().len(), 32, "第二趟才到");

    // Same byte count — the factual half of "not bytes".
    assert_eq!(
        one.pushed, b.pushed,
        "记得与忘了推的是同一段字节({} vs {}),差的只有趟数",
        one.pushed, b.pushed
    );
    assert_eq!(render(&mine.doc), render(&far.doc));

    let _ = fs::remove_dir_all(&dir);
}

/// Over the cap, the error names both numbers and points at the road
/// that works. Both directions guard — what the far side sends is not
/// ours to control.
#[test]
fn an_oversized_payload_is_refused_by_name() {
    let doc = ChatDoc::new(0x81).unwrap();
    // Feed oversized bytes directly: far cheaper than building 256 KiB of
    // real history, and it hits the same gate.
    let huge = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        vec![0u8; MAX_BYTES + 1],
    );
    let e = wire::merge_b64(&doc, &huge).unwrap_err();
    assert!(e.contains("超过"), "错话里得说清超了限:{e}");
    assert!(e.contains("ssh"), "还得指一条能走的路:{e}");

    // Control: the same path under the cap must pass this gate — without
    // it, a reject-everything implementation is green above.
    let ok = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        vec![0u8; 16],
    );
    let e2 = wire::merge_b64(&doc, &ok).unwrap_err();
    assert!(
        !e2.contains("超过"),
        "16 字节不该撞上限(它会因为不是合法增量而失败,那是另一回事):{e2}"
    );
}

/// The version vector is exempt from the cap: refusing it kills the whole
/// path, and the cap is for content.
#[test]
fn the_version_vector_is_not_capped() {
    let doc = ChatDoc::new(0x91).unwrap();
    doc.say(&me("我"), "一句").unwrap();
    let v = wire::version_b64(&doc);
    assert!(!v.is_empty());
    // Empty = from the beginning, not an error — a freshly paired device.
    assert!(wire::changes_since_b64(&doc, "").is_ok());
    // Bad base64 must read as "not base64", not "sync broken".
    let e = wire::changes_since_b64(&doc, "这不是 base64!!").unwrap_err();
    assert!(e.contains("base64"), "{e}");
}

// ── channel name = a machine's window ─────────────────────

use super::store::channel_of_machine;

/// Clean machine names pass through unchanged — and match what the ssh
/// side sees. Pure function: two machines must compute the same name.
#[test]
fn a_clean_machine_name_is_the_channel_name_unchanged() {
    for n in ["turing", "mac-mini", "gpu.01", "aliyun_2", "A1"] {
        assert_eq!(channel_of_machine(n).as_deref(), Some(n), "{n} 该原样通过");
    }
    // `.local` stripped: one machine self-reports both spellings
    // depending on the entry point.
    assert_eq!(channel_of_machine("zgh-mac.local").as_deref(), Some("zgh-mac"));
    assert_eq!(channel_of_machine("  turing  ").as_deref(), Some("turing"));
}

/// A cleaned name never collides with a clean one — the whole reason the
/// fingerprint exists. Control: the machine truly named `a-b` passes
/// unchanged, or an all-names-get-suffixed implementation is green too.
#[test]
fn a_cleaned_name_never_collides_with_a_clean_one() {
    let dirty = channel_of_machine("a b").expect("该算得出来");
    let clean = channel_of_machine("a-b").expect("该算得出来");
    assert_eq!(clean, "a-b", "干净的必须原样(对照组:不是所有名字都加后缀)");
    assert_ne!(dirty, clean, "清洗过的不许撞上真名,实测 {dirty} vs {clean}");
    assert!(dirty.starts_with("a-b-"), "清洗过的该带指纹,实测 {dirty}");

    // Two different originals with the same cleaned stem stay apart —
    // the fingerprint hashes the original.
    let x = channel_of_machine("a b").unwrap();
    let y = channel_of_machine("a/b").unwrap();
    assert_ne!(x, y, "原名不同就不许同名,实测 {x} vs {y}");
}

/// Same original, same answer, every time.
#[test]
fn the_channel_name_is_a_pure_function_of_the_machine_name() {
    for n in ["张三的电脑", "turing", "a b", "服务器-01"] {
        let a = channel_of_machine(n);
        let b = channel_of_machine(n);
        assert_eq!(a, b, "{n} 算两次该一样");
        if let Some(c) = a {
            assert!(valid_channel(&c), "{n} → {c} 必须是合法频道名");
        }
    }
}

/// Non-ASCII machine names still get a channel; a name with nothing
/// usable gets `None`, not an invented name.
#[test]
fn a_non_ascii_machine_name_still_gets_a_channel() {
    let c = channel_of_machine("张三的电脑").expect("该算得出一个名字");
    assert!(valid_channel(&c), "{c}");
    assert!(!c.is_empty());
    // Mixed names keep their readable part — people `ls` these dirs.
    let m = channel_of_machine("张三的 MacBook").expect("该算得出来");
    assert!(m.contains("MacBook"), "可读的那一截该留着,实测 {m}");

    assert_eq!(channel_of_machine(""), None);
    assert_eq!(channel_of_machine("   "), None);
    // Not None: zero legal chars still yields placeholder + fingerprint,
    // which is deterministic.
    let all_bad = channel_of_machine("。、·").expect("该有个兜底名");
    assert!(valid_channel(&all_bad), "{all_bad}");
}

/// Long names are bounded, and two sharing a long prefix don't collide.
/// The truncation is safe only because the fingerprint hashes the full
/// original — this measures exactly that.
#[test]
fn a_very_long_machine_name_is_bounded_without_colliding() {
    for long in ["机器".repeat(200), "a".repeat(200)] {
        let c = channel_of_machine(&long).expect("超长也该算得出一个名字");
        assert!(valid_channel(&c), "长度 {} : {c}", c.len());
    }
    // Identical first 128 chars, one differing tail char: two channels.
    let a = channel_of_machine(&format!("{}X", "a".repeat(150))).unwrap();
    let b = channel_of_machine(&format!("{}Y", "a".repeat(150))).unwrap();
    assert_ne!(a, b, "只差最后一个字的两台机器不许并成一场:{a} vs {b}");
}

/// What lands on disk is readable only by me. Blocks get copied to other
/// machines and the mode travels; a block is the document itself, so
/// "not plaintext" protects nothing. This measures the mode bits, not
/// which API was called — the latter stays green when broken.
#[cfg(unix)]
#[test]
fn what_lands_on_disk_is_readable_only_by_me() {
    use std::os::unix::fs::PermissionsExt;

    let home = tmpdir("perms");
    let dir = channel_dir(&home, "turing").expect("频道名该是合法的");
    let mut l = ChatStore::load(&dir, 7).expect("空频道要开得起来");
    l.doc
        .say(&Sender { id: "a".into(), name: "A".into() }, "一句话")
        .expect("说不出话");
    l.store.flush(&l.doc).expect("落不了盘");

    let mode = |p: &std::path::Path| {
        std::fs::metadata(p).expect("读不到属性").permissions().mode() & 0o777
    };
    assert_eq!(mode(&dir), 0o700, "频道目录不许让别人 cd 进来");

    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("列不了目录") {
        let p = e.expect("读不到条目").path();
        assert_eq!(mode(&p), 0o600, "{} 不该让别人读得到", p.display());
        n += 1;
    }
    // Prove the round wrote anything at all: a write-nothing
    // implementation idles through the loop above and every assertion
    // passes. At least two files: one block plus the ledger.
    assert!(n >= 2, "这一趟该写出块和账本,实测只有 {n} 个文件");
}
