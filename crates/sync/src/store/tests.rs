//! Exercised through `ChatDoc` — the store is generic, the traps it
//! guards are not.

use std::fs;
use std::path::PathBuf;

use crate::chat::{open_channel, Sender};
use crate::testutil::{me, render, tmpdir};

/// Flush writes only what disk lacks. Without this, "append-only"
/// silently becomes quadratic — the Nth flush writes N histories, and
/// everything still works.
#[test]
fn flush_writes_only_what_is_not_on_disk_yet() {
    let dir = tmpdir("flush");
    let mut st = open_channel(&dir, 1).unwrap();
    for i in 0..50 {
        st.doc.tell(&me("a"), &format!("第 {i} 句")).unwrap();
    }
    let first = st.store.flush(&st.doc).unwrap().expect("该写出一个块");
    let big = fs::metadata(&first).unwrap().len();

    st.doc.tell(&me("a"), "又一句").unwrap();
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
    let mut st = open_channel(&dir, 1).unwrap();
    st.doc.tell(&me("a"), "一").unwrap();
    st.doc.tell(&me("a"), "二").unwrap();
    st.store.flush(&st.doc).unwrap();
    let want = render(&st.doc);

    let back = open_channel(&dir, 1).unwrap();
    assert!(back.broken.is_empty(), "不该有坏块");
    assert_eq!(render(&back.doc), want);
    let _ = fs::remove_dir_all(&dir);
}

/// One broken block neither kills the document nor disappears silently:
/// it must be counted, so the UI can say a slice is missing.
#[test]
fn one_broken_block_does_not_kill_the_channel() {
    let dir = tmpdir("broken");
    let mut st = open_channel(&dir, 1).unwrap();
    st.doc.tell(&me("a"), "好的那句").unwrap();
    st.store.flush(&st.doc).unwrap();
    fs::write(dir.join("u-000000000000ffff-00000000.loro"), b"not a loro block").unwrap();

    let back = open_channel(&dir, 1).unwrap();
    assert_eq!(back.broken.len(), 1, "坏块要被数出来,不能静默吞掉");
    assert_eq!(back.doc.messages().len(), 1, "好的那句还得在");
    let _ = fs::remove_dir_all(&dir);
}

/// Compaction loses not a word, and the old blocks are gone.
#[test]
fn compacting_keeps_every_word() {
    let dir = tmpdir("compact");
    let mut st = open_channel(&dir, 1).unwrap();
    for i in 0..20 {
        st.doc.tell(&me("a"), &format!("第 {i} 句")).unwrap();
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

    let back = open_channel(&dir, 1).unwrap();
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
    let mut one = open_channel(&dir, 0xA1).unwrap();
    let mut two = open_channel(&dir, 0xA2).unwrap();

    one.doc.tell(&me("a"), "来自进程一").unwrap();
    let p1 = one.store.flush(&one.doc).unwrap().unwrap();
    two.doc.tell(&me("a"), "来自进程二").unwrap();
    let p2 = two.store.flush(&two.doc).unwrap().unwrap();

    assert_ne!(p1, p2, "两个块不许同名");
    let back = open_channel(&dir, 0xA3).unwrap();
    let text = render(&back.doc);
    assert!(
        text.contains("来自进程一") && text.contains("来自进程二"),
        "两句都得在:\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// What lands on disk is readable only by me. Blocks get copied to other
/// machines and the mode travels; a block is the document itself, so
/// "not plaintext" protects nothing. This measures the mode bits, not
/// which API was called — the latter stays green when broken.
#[cfg(unix)]
#[test]
fn what_lands_on_disk_is_readable_only_by_me() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tmpdir("perms").join("doc");
    let mut l = open_channel(&dir, 7).expect("空文档要开得起来");
    l.doc
        .tell(&Sender { id: "a".into(), name: "A".into() }, "一句话")
        .expect("说不出话");
    l.store.flush(&l.doc).expect("落不了盘");

    let mode = |p: &std::path::Path| {
        std::fs::metadata(p).expect("读不到属性").permissions().mode() & 0o777
    };
    assert_eq!(mode(&dir), 0o700, "文档目录不许让别人 cd 进来");

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
    let _ = fs::remove_dir_all(dir.parent().unwrap());
}
