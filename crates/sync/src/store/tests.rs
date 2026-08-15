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
        st.doc.tell(&me("a"), &format!("line {i}")).unwrap();
    }
    let first = st.store.flush(&st.doc).unwrap().expect("should write a block");
    let big = fs::metadata(&first).unwrap().len();

    st.doc.tell(&me("a"), "another line").unwrap();
    let second = st.store.flush(&st.doc).unwrap().expect("should write another block");
    let small = fs::metadata(&second).unwrap().len();

    assert!(
        small * 5 < big,
        "the second block should hold only that line (first {big} B, second {small} B)"
    );
    // No new content, no file — not even an empty one.
    assert!(st.store.flush(&st.doc).unwrap().is_none(), "nothing new must write no file");
    let _ = fs::remove_dir_all(&dir);
}

/// What is flushed comes back, identical.
#[test]
fn what_is_flushed_comes_back() {
    let dir = tmpdir("roundtrip");
    let mut st = open_channel(&dir, 1).unwrap();
    st.doc.tell(&me("a"), "one").unwrap();
    st.doc.tell(&me("a"), "two").unwrap();
    st.store.flush(&st.doc).unwrap();
    let want = render(&st.doc);

    let back = open_channel(&dir, 1).unwrap();
    assert!(back.broken.is_empty(), "no block should be broken");
    assert_eq!(render(&back.doc), want);
    let _ = fs::remove_dir_all(&dir);
}

/// One broken block neither kills the document nor disappears silently:
/// it must be counted, so the UI can say a slice is missing.
#[test]
fn one_broken_block_does_not_kill_the_channel() {
    let dir = tmpdir("broken");
    let mut st = open_channel(&dir, 1).unwrap();
    st.doc.tell(&me("a"), "the good line").unwrap();
    st.store.flush(&st.doc).unwrap();
    fs::write(dir.join("u-000000000000ffff-00000000.loro"), b"not a loro block").unwrap();

    let back = open_channel(&dir, 1).unwrap();
    assert_eq!(back.broken.len(), 1, "broken blocks must be counted, not swallowed");
    assert_eq!(back.doc.messages().len(), 1, "the good line must remain");
    let _ = fs::remove_dir_all(&dir);
}

/// Compaction loses not a word, and the old blocks are gone.
#[test]
fn compacting_keeps_every_word() {
    let dir = tmpdir("compact");
    let mut st = open_channel(&dir, 1).unwrap();
    for i in 0..20 {
        st.doc.tell(&me("a"), &format!("line {i}")).unwrap();
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
    assert!(before >= 20, "before compaction there should be a pile of blocks, got {before}");

    st.store.compact(&st.doc).unwrap();
    let after = count_blocks(&dir);
    assert_eq!(after, 1, "after compaction only the snapshot should remain, got {after}");

    let back = open_channel(&dir, 1).unwrap();
    assert_eq!(render(&back.doc), want, "compaction must not lose a line");
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

    one.doc.tell(&me("a"), "from process one").unwrap();
    let p1 = one.store.flush(&one.doc).unwrap().unwrap();
    two.doc.tell(&me("a"), "from process two").unwrap();
    let p2 = two.store.flush(&two.doc).unwrap().unwrap();

    assert_ne!(p1, p2, "the two blocks must not share a name");
    let back = open_channel(&dir, 0xA3).unwrap();
    let text = render(&back.doc);
    assert!(
        text.contains("from process one") && text.contains("from process two"),
        "both lines must be there:\n{text}"
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
    let mut l = open_channel(&dir, 7).expect("an empty doc must open");
    l.doc
        .tell(&Sender { id: "a".into(), name: "A".into() }, "a line")
        .expect("tell failed");
    l.store.flush(&l.doc).expect("flush failed");

    let mode = |p: &std::path::Path| {
        std::fs::metadata(p).expect("no metadata").permissions().mode() & 0o777
    };
    assert_eq!(mode(&dir), 0o700, "the doc dir must not let others cd in");

    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("cannot list the dir") {
        let p = e.expect("cannot read the entry").path();
        assert_eq!(mode(&p), 0o600, "{} must not be readable by others", p.display());
        n += 1;
    }
    // Prove the round wrote anything at all: a write-nothing
    // implementation idles through the loop above and every assertion
    // passes. At least two files: one block plus the ledger.
    assert!(n >= 2, "this pass should write block and ledger, got only {n} files");
    let _ = fs::remove_dir_all(dir.parent().unwrap());
}
