//! Cross-module integration: doc + store + plan composed the way a real
//! driver composes them. Single-module behavior is tested next to its
//! code.

use std::fs;
use std::path::PathBuf;

use super::doc::ChatDoc;
use super::plan::{plan, Side};
use super::store::ChatStore;
use super::testutil::{me, render, tmpdir};

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

/// Two machines converge through a node that understands nothing — the
/// falsifiable form of "an ssh-only machine is a member of the network".
#[test]
fn two_machines_converge_through_a_node_that_understands_nothing() {
    let dumb = Dumb::new("dumb-node");
    let dir_a = tmpdir("mach-a");
    let dir_b = tmpdir("mach-b");

    let mut a = ChatStore::load(&dir_a, 0xA).unwrap();
    a.doc.tell(&me("mac"), "从 Mac 发的").unwrap();
    a.store.flush(&a.doc).unwrap();
    let (pull, push) = sync_with_dumb(&mut a.store, &a.doc, &dumb);
    assert_eq!((pull, push), (0, 1), "A 该推一块上去,没有可拉的");

    let mut b = ChatStore::load(&dir_b, 0xB).unwrap();
    let (pull, _) = sync_with_dumb(&mut b.store, &b.doc, &dumb);
    assert_eq!(pull, 1, "B 该从哑节点拉到 A 那一块");
    b.doc.tell(&me("phone"), "从手机发的").unwrap();
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
        m.doc.tell(&me("a"), &format!("第 {i} 句")).unwrap();
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
