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
