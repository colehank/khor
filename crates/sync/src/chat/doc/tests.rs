use std::fs;

use super::*;
use crate::testutil::{me, render, tmpdir};

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
        docs[a]
            .tell(&me(&format!("p{a}")), &format!("第 {round} 句"))
            .unwrap();
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
        assert_eq!(render(d), first, "node {i} differs from node 0");
    }
    assert_eq!(docs[0].messages().len(), 40, "not one line may be lost");

    // Control: an unsynced doc must differ, or "all equal" may just
    // mean render() returns the same string for everything.
    let lonely = ChatDoc::new(99).unwrap();
    lonely.tell(&me("nobody"), "talking to itself").unwrap();
    assert_ne!(
        render(&lonely),
        first,
        "the unsynced one must not match the rest"
    );
}

/// Merging the same block twice equals once — reconnects, replays,
/// and both transports delivering the same segment all lean on this.
#[test]
fn merging_the_same_block_twice_changes_nothing() {
    let a = ChatDoc::new(1).unwrap();
    a.tell(&me("a"), "a line").unwrap();
    let block = a.snapshot().unwrap();

    let b = ChatDoc::new(2).unwrap();
    b.merge(&block).unwrap();
    let once = render(&b);
    b.merge(&block).unwrap();
    assert_eq!(render(&b), once, "the second pass must change nothing");
}

/// A message with a 200MB attachment syncs as metadata only — the
/// falsifiable form of "bytes never enter the document".
#[test]
fn a_huge_attachment_only_costs_its_metadata() {
    let a = ChatDoc::new(1).unwrap();
    for i in 0..200 {
        a.tell(&me("a"), &format!("一句普通的话,第 {i} 条"))
            .unwrap();
    }
    let b = ChatDoc::new(2).unwrap();
    b.merge(&a.snapshot().unwrap()).unwrap();

    let before = b.version();
    a.post(
        &me("a"),
        "",
        &[],
        &[FileRef {
            name: "dataset.tar.zst".into(),
            size: 209_715_200,
            digest: "b3:9f2c".repeat(6),
            holder: String::new(),
        }],
    )
    .unwrap();

    let delta = a.changes_since(&before).unwrap();
    let snap = a.snapshot().unwrap();
    assert!(
        delta.len() < 1024,
        "the delta should stay under 1KB, got {} bytes",
        delta.len()
    );
    assert!(
        delta.len() * 10 < snap.len(),
        "a delta must be an order smaller than a snapshot, or it is no delta ({} vs {})",
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
    a.tell(&me("mac"), "from the mac").unwrap();
    let c = ChatDoc::new(3).unwrap();
    c.tell(&me("phone"), "from the phone").unwrap();

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
    viafile
        .merge(&fs::read(dir.join("a.loro")).unwrap())
        .unwrap();
    viafile
        .merge(&fs::read(dir.join("c.loro")).unwrap())
        .unwrap();

    let direct = ChatDoc::new(8).unwrap();
    direct.merge(&fa).unwrap();
    direct.merge(&fc).unwrap();

    assert_eq!(
        render(&viafile),
        render(&direct),
        "via file and direct must be identical"
    );
    assert_eq!(
        viafile.messages().len(),
        2,
        "messages from both sides must be there"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Undo takes back only my own message; a concurrent one from someone
/// else stays untouched. A "revert to previous version"
/// implementation sweeps the other one away — and stays green in
/// single-machine tests.
#[test]
fn undo_only_takes_back_my_own_message() {
    let mut a = ChatDoc::new(1).unwrap();
    a.tell(&me("me"), "my first line").unwrap();

    let b = ChatDoc::new(2).unwrap();
    b.merge(&a.snapshot().unwrap()).unwrap();
    b.tell(&me("peer"), "a line from someone else").unwrap();
    a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();

    assert_eq!(a.messages().len(), 2);
    assert!(a.can_undo(), "the undo stack should hold something");
    assert!(a.undo().unwrap(), "undo should really run");

    let left = render(&a);
    assert!(
        left.contains("a line from someone else"),
        "the other's line must remain:\n{left}"
    );
    assert!(
        !left.contains("my first line"),
        "my own line should be gone:\n{left}"
    );
}

/// Retraction is a soft delete: the slot stays, marked. Hard removal
/// makes the context jump and people think they misremembered.
#[test]
fn retracting_keeps_the_slot() {
    let d = ChatDoc::new(1).unwrap();
    d.tell(&me("a"), "first").unwrap();
    let id = d.tell(&me("a"), "a mistake").unwrap();
    d.tell(&me("a"), "third").unwrap();

    d.retract(&id).unwrap();
    let msgs = d.messages();
    assert_eq!(msgs.len(), 3, "the slot stays, not carved out");
    assert!(
        msgs[1].retracted,
        "the middle one should be flagged retracted"
    );
    assert!(
        !msgs[0].retracted && !msgs[2].retracted,
        "the others must not be dragged along"
    );
}

/// One device edits while another retracts; both must survive. This
/// pins "a message is a map, not a JSON string": whole-value LWW
/// lets the later write erase the earlier one, with no error on
/// either side.
#[test]
fn a_concurrent_edit_and_retract_both_survive() {
    let a = ChatDoc::new(1).unwrap();
    let id = a.tell(&me("a"), "the original").unwrap();

    let b = ChatDoc::new(2).unwrap();
    b.merge(&a.snapshot().unwrap()).unwrap();

    a.edit(&id, "the edited text").unwrap();
    b.retract(&id).unwrap();

    a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
    b.merge(&a.changes_since(&b.version()).unwrap()).unwrap();

    for (who, d) in [("a", &a), ("b", &b)] {
        let m = &d.messages()[0];
        assert!(m.retracted, "{who}: the retract must survive");
        assert!(m.edited, "{who}: the edit must survive");
        assert_eq!(
            m.body,
            MsgBody::Text("the edited text".into()),
            "{who}: the body should be the edited text"
        );
    }
    assert_eq!(render(&a), render(&b), "both machines must end identical");
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
        "an unknown kind must say so, never fall into Text"
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
        "after a version that does not know the key, it must remain:\n{dumped}"
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
    a.tell(&me("a"), "from side a").unwrap();
    b.tell(&me("b"), "from side b").unwrap();

    let merged = ChatDoc::new(2).unwrap();
    merged.merge(&a.snapshot().unwrap()).unwrap();
    // No error — which is exactly what makes it dangerous.
    let _ = merged.merge(&b.changes_since(&merged.version()).unwrap());

    let text = render(&merged);
    assert_eq!(
        merged.messages().len(),
        1,
        "a shared peer loses a line — this test guards the fact, not the behaviour:\n{text}"
    );
}

/// **A khor-wide file is one record per receiver** (STORE-BRIEF phase
/// 3): two machines answering the same offer at once, each from its own
/// copy, both keep their account after the merge, and a third device
/// reads both. The single-receiver record is not touched.
#[test]
fn every_receiver_keeps_its_own_fate_of_one_offer() {
    let sender = ChatDoc::new(1).unwrap();
    let a = ChatDoc::new(2).unwrap();
    let b = ChatDoc::new(3).unwrap();
    let third = ChatDoc::new(4).unwrap();
    let f = FileRef {
        name: "a.txt".into(),
        size: 5,
        digest: "cd".repeat(32),
        holder: String::new(),
    };
    let id = sender.post(&me("s"), "", &[], &[f]).unwrap();
    for d in [&a, &b] {
        d.merge(&sender.snapshot().unwrap()).unwrap();
    }
    assert!(a.messages()[0].fates.is_empty(), "nobody has answered yet");

    let landed = Fate {
        step: Step::Landed,
        moved: 5,
        landed: vec!["/home/a/khor-drop/a.txt".into()],
        why: String::new(),
        via: String::new(),
    };
    let failed = Fate {
        step: Step::Failed,
        moved: 2,
        landed: vec![],
        why: "gone".into(),
        via: String::new(),
    };
    a.set_fate_of(
        &id,
        "dev-a",
        &Fate {
            step: Step::Moving,
            moved: 1,
            landed: vec![],
            why: String::new(),
            via: String::new(),
        },
    )
    .unwrap();
    a.set_fate_of(&id, "dev-a", &landed).unwrap();
    let before = a.version();
    a.set_fate_of(&id, "dev-a", &landed).unwrap();
    assert_eq!(a.version(), before, "the same account twice writes nothing");
    b.set_fate_of(&id, "dev-b", &failed).unwrap();

    for d in [&a, &b] {
        third.merge(&d.snapshot().unwrap()).unwrap();
    }
    let m = &third.messages()[0];
    assert_eq!(m.fates.get("dev-a"), Some(&landed));
    assert_eq!(m.fates.get("dev-b"), Some(&failed));
    assert_eq!(m.fates.len(), 2);
}

/// The head names the machine that runs a khor-wide conversation, and
/// travels like everything else; the broadcast names none.
#[test]
fn the_head_names_a_home_that_travels_and_can_move() {
    let a = ChatDoc::new(1).unwrap();
    let b = ChatDoc::new(2).unwrap();
    assert_eq!(a.home(), None, "every machine answers for itself");
    a.set_home("dev-a").unwrap();
    b.merge(&a.snapshot().unwrap()).unwrap();
    assert_eq!(b.home().as_deref(), Some("dev-a"));
    b.set_home("dev-b").unwrap();
    a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
    assert_eq!(
        a.home().as_deref(),
        Some("dev-b"),
        "moving a home is writing another id"
    );
    a.set_home("").unwrap();
    assert_eq!(a.home(), None);
}

/// **A composed message carries its targets, its words and each file's
/// holder** (STORE-BRIEF phase 5): written once at compose time, read the
/// same on every device, by device id — the names typed were resolved
/// then, so nothing here depends on what a machine is called later.
#[test]
fn a_posted_message_carries_its_targets_its_words_and_each_files_holder() {
    use super::Target;
    let sender = ChatDoc::new(1).unwrap();
    let third = ChatDoc::new(2).unwrap();
    let to = vec![
        Target {
            device: "dev-b".into(),
            path: "~/in".into(),
        },
        Target {
            device: "dev-c".into(),
            path: String::new(),
        },
    ];
    let here = FileRef {
        name: "x".into(),
        size: 5,
        digest: "ab".repeat(32),
        holder: String::new(),
    };
    let far = FileRef {
        name: "y".into(),
        size: 7,
        digest: "cd".repeat(32),
        holder: "dev-h".into(),
    };
    let id = sender
        .post(&me("s"), "look at these", &to, &[here.clone(), far.clone()])
        .unwrap();
    let said = sender.post(&me("s"), "hello", &to[..1], &[]).unwrap();
    third.merge(&sender.snapshot().unwrap()).unwrap();
    let msgs = third.messages();
    let files = msgs.iter().find(|m| m.id == id).unwrap();
    assert_eq!(
        files.to, to,
        "every target with its own path, in the order named"
    );
    assert_eq!(
        files.body,
        MsgBody::Files(vec![here, far]),
        "each file with its holder; empty is the sender"
    );
    assert_eq!(files.note, "look at these", "the words said with the files");
    let text = msgs.iter().find(|m| m.id == said).unwrap();
    assert_eq!(text.body, MsgBody::Text("hello".into()));
    assert_eq!(
        (text.to.len(), text.note.as_str()),
        (1, ""),
        "a line's words are its body, not a note"
    );
    let plain = sender.post(&me("s"), "to everyone", &[], &[]).unwrap();
    assert!(
        sender
            .messages()
            .iter()
            .find(|m| m.id == plain)
            .unwrap()
            .to
            .is_empty(),
        "no target, no key"
    );
}
