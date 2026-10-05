//! Exercised through `ChatDoc` — the store is generic, the traps it
//! guards are not.

use std::fs;
use std::path::PathBuf;

use crate::chat::{Sender, open_channel};
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
    let first = st
        .store
        .flush(&st.doc)
        .unwrap()
        .expect("should write a block");
    let big = fs::metadata(&first).unwrap().len();

    st.doc.tell(&me("a"), "another line").unwrap();
    let second = st
        .store
        .flush(&st.doc)
        .unwrap()
        .expect("should write another block");
    let small = fs::metadata(&second).unwrap().len();

    assert!(
        small * 5 < big,
        "the second block should hold only that line (first {big} B, second {small} B)"
    );
    // No new content, no file — not even an empty one.
    assert!(
        st.store.flush(&st.doc).unwrap().is_none(),
        "nothing new must write no file"
    );
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
    fs::write(
        dir.join("u-000000000000ffff-00000000.loro"),
        b"not a loro block",
    )
    .unwrap();

    let back = open_channel(&dir, 1).unwrap();
    assert_eq!(
        back.broken.len(),
        1,
        "broken blocks must be counted, not swallowed"
    );
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
    assert!(
        before >= 20,
        "before compaction there should be a pile of blocks, got {before}"
    );

    st.store.compact(&st.doc).unwrap();
    let after = count_blocks(&dir);
    assert_eq!(
        after, 1,
        "after compaction only the snapshot should remain, got {after}"
    );

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

/// **Opening the same home from several places at once must not fail.**
///
/// One device runs a resident serve, the app's backend and a CLI verb at
/// the same time, and every open saves the ledger — a whole-file write.
/// While the temp file was named after its target instead of its writer,
/// the first writer's rename took the file and the second's rename found
/// nothing there, so an open failed with "cannot rename". It surfaced as
/// whatever verb happened to be running: measured at **17 of 40** GUI
/// pin requests failing, silently, while a CLI loop opened the same
/// home.
///
/// The neighbour above (`two_writers_on_the_same_device_do_not_collide`)
/// does not cover this: it flushes the two writers **in turn**, so each
/// temp is renamed away before the next one exists. Only overlapping in
/// time reaches it.
#[test]
fn opening_one_home_from_many_places_at_once_never_fails() {
    let dir = tmpdir("tmprace");
    // The directory has to exist for the ledger to be saved at all.
    let mut seed = open_channel(&dir, 0xC0).unwrap();
    seed.doc.tell(&me("a"), "seed").unwrap();
    seed.store.flush(&seed.doc).unwrap();

    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let dir = dir.clone();
                s.spawn(move || {
                    let mut errs = Vec::new();
                    for _ in 0..12 {
                        match open_channel(&dir, 0xD0 + i) {
                            Ok(mut st) => {
                                st.doc.tell(&me("a"), "concurrent").unwrap();
                                if let Err(e) = st.store.flush(&st.doc) {
                                    errs.push(e);
                                }
                            }
                            Err(e) => errs.push(e),
                        }
                    }
                    errs
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });

    assert!(
        failures.is_empty(),
        "{} of 96 concurrent opens failed: {:?}",
        failures.len(),
        &failures[..failures.len().min(3)]
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
        .tell(
            &Sender {
                id: "a".into(),
                name: "A".into(),
            },
            "a line",
        )
        .expect("tell failed");
    l.store.flush(&l.doc).expect("flush failed");

    let mode = |p: &std::path::Path| {
        std::fs::metadata(p)
            .expect("no metadata")
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode(&dir), 0o700, "the doc dir must not let others cd in");

    let mut n = 0;
    for e in std::fs::read_dir(&dir).expect("cannot list the dir") {
        let p = e.expect("cannot read the entry").path();
        assert_eq!(
            mode(&p),
            0o600,
            "{} must not be readable by others",
            p.display()
        );
        n += 1;
    }
    // Prove the round wrote anything at all: a write-nothing
    // implementation idles through the loop above and every assertion
    // passes. At least two files: one block plus the ledger.
    assert!(
        n >= 2,
        "this pass should write block and ledger, got only {n} files"
    );
    let _ = fs::remove_dir_all(dir.parent().unwrap());
}

/// **A reader that overlaps a compaction must not come back short.**
///
/// Compaction writes the snapshot and *then* deletes the increments, so
/// a reader that listed the directory before the delete and read after
/// it finds files gone. Those reads used to land in `broken`, which is a
/// count nobody surfaces — the document came back missing a slice of
/// itself and said nothing. `load` lists again when that happens, and
/// the snapshot it then finds holds every byte the vanished blocks did.
///
/// ⚠️ **This test does NOT prove the re-listing is what saves it, and
/// two attempts to make it do so both failed.** Taking the re-listing
/// out of `load` leaves this green at 60 blocks and at 600, and the
/// re-listing counter says the vanish branch *was* reached both times.
/// So the branch runs and the outcome does not depend on it.
///
/// Why, as far as the measurement goes: compaction writes the snapshot
/// **before** deleting anything, and `doc.snapshot()` on a document
/// worth 600 blocks takes longer than a reader takes to read all 600.
/// A reader that listed after the snapshot landed holds the whole
/// document in one file and cannot be short; a reader that listed
/// before it has finished reading by the time the deletes start. The
/// window that loses data — listed before the snapshot, still reading
/// when the deletes land — is real in principle and was not reachable
/// here.
///
/// **So what this holds down is the outcome, not the branch**: after a
/// compaction that overlapped live readers, every reader saw every
/// line and one snapshot was left behind. The re-listing stays as the
/// belt to that braces, cheap and reached; a test that pins it would
/// need a seam inside `load` (a `Doc` whose `merge` deletes the rest of
/// the directory part way through), and that is not written.
///
/// One compactor, many readers — the shape the product will have, where
/// the resident serve owns compaction. ⚠️ **Written first as two tight
/// loops it failed with 「0 lines came back」 and an empty directory,
/// and the reader was not the victim: a second compactor read a
/// directory the first was half way through deleting, got an empty
/// document, and wrote an empty snapshot over everything. Two compactors
/// on one document destroy it, and nothing `load` does can fix that.**
/// That is the argument for one owner, and it is why this test has one.
#[test]
fn a_read_that_races_a_compaction_still_sees_every_line() {
    // **Until a read straddled it** (round NET-6): whether one does is
    // timing, and under a full parallel test run 3 of 16 tries had the
    // readers manage only six or seven reads in the window and none
    // across it. Each try is the whole scene again on a fresh pile.
    let mut met = None;
    for attempt in 0..8 {
        let (left, short, reads, relists) = race_one_compaction(attempt);
        assert_eq!(
            left, 1,
            "the compaction should have left one snapshot, found {left}"
        );
        assert_eq!(
            short, 0,
            "{short} of {reads} reads came back missing a line the document has \
             (re-listings this run: {relists})"
        );
        if relists > 0 {
            met = Some(attempt);
            break;
        }
    }
    // **The arm that says this run raced anything at all.** A pass with
    // no re-listing is a pass where the timing never overlapped, and
    // that proves nothing about the code being tested.
    assert!(
        met.is_some(),
        "in eight tries no read found a vanished block, so this run never met the case it is about"
    );
}

/// One try of [`a_read_that_races_a_compaction_still_sees_every_line`]:
/// snapshots left, short reads, reads, re-listings.
fn race_one_compaction(attempt: u32) -> (usize, u64, u64, u64) {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    let dir = tmpdir(&format!("race-compact-{attempt}"));
    // ⚠️ **The pile has to be big enough that a read is slow.** At 60
    // blocks this test passed with the re-listing taken out: a read
    // finishes in well under a millisecond, so it never sat across the
    // one window that loses data — listed *before* the snapshot was
    // written, still reading when the increments were deleted. A reader
    // that listed after the snapshot appeared has the whole document in
    // one file and cannot be short no matter what is deleted next. The
    // real device table on this machine is 557 blocks and takes 20 ms to
    // read, which is exactly the shape being reproduced here.
    let mut st = open_channel(&dir, 1).unwrap();
    for i in 0..600 {
        st.doc.tell(&me("a"), &format!("line {i}")).unwrap();
        st.store.flush(&st.doc).unwrap();
    }
    let want = render(&st.doc);
    let lines = want.lines().count();
    assert!(
        lines >= 600,
        "the fixture should have a pile of lines, got {lines}"
    );

    // Readers, running while the compaction below happens under them.
    let stop = Arc::new(AtomicBool::new(false));
    let short = Arc::new(AtomicU64::new(0));
    let reads = Arc::new(AtomicU64::new(0));
    let readers: Vec<_> = (0..3)
        .map(|k| {
            let (dir, stop, short, reads) = (
                dir.clone(),
                Arc::clone(&stop),
                Arc::clone(&short),
                Arc::clone(&reads),
            );
            let want = want.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let back = open_channel(&dir, 10 + k).unwrap();
                    let got = render(&back.doc);
                    reads.fetch_add(1, Ordering::Relaxed);
                    if want.lines().any(|l| !got.contains(l)) {
                        short.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    // Give the readers a moment to be mid-read, then compact once.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let relists_before = crate::store::relists_after_a_vanish();
    st.store.compact(&st.doc).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(60));
    stop.store(true, Ordering::Relaxed);
    for r in readers {
        r.join().unwrap();
    }
    let relists = crate::store::relists_after_a_vanish() - relists_before;
    let (short, reads) = (short.load(Ordering::Relaxed), reads.load(Ordering::Relaxed));
    let left = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
        .count();
    let _ = fs::remove_dir_all(&dir);

    (left, short, reads, relists)
}

/// **A read that learns nothing must not write anything.**
///
/// `load` keeps the merged-blocks ledger up to date, and used to save it
/// on every read whether or not it had changed. In the steady state —
/// every block already listed, which is every read after the first — that
/// is a temp file, a write and a rename to store what was already
/// stored. Measured 2026-09-11: **4 ms a read**, which was most of what
/// a session list cost. `chat/Mac-mini`, one block of 356 bytes, read in
/// 4.7 ms; the same document with this write skipped, 0.0 ms.
///
/// The ledger only ever grows — compaction deletes blocks but keeps
/// their lines — so 「this read added no name」 is exactly 「the file on
/// disk is still right」.
///
/// Checked by the file's modification time rather than its contents: the
/// contents were always going to be identical, and identical contents
/// written again is the bug.
#[test]
fn a_read_that_learns_nothing_does_not_rewrite_the_ledger() {
    let dir = tmpdir("ledger-quiet");
    let mut st = open_channel(&dir, 1).unwrap();
    st.doc.tell(&me("a"), "one").unwrap();
    st.store.flush(&st.doc).unwrap();
    drop(st);

    let ledger = dir.join(".merged");
    // The first read is allowed to write: it is the one that learns the
    // block's name. **This is the arm that proves the test can tell** —
    // without it, a ledger that is never written at all would pass.
    let _ = fs::remove_file(&ledger);
    let _ = open_channel(&dir, 2).unwrap();
    assert!(
        ledger.exists(),
        "the first read should have written the ledger"
    );
    let after_first = fs::metadata(&ledger).unwrap().modified().unwrap();

    // Far enough apart that a coarse filesystem clock still separates
    // them: this must not pass by the two stamps being unresolvable.
    std::thread::sleep(std::time::Duration::from_millis(50));
    for _ in 0..5 {
        let _ = open_channel(&dir, 3).unwrap();
    }
    let after_more = fs::metadata(&ledger).unwrap().modified().unwrap();
    let quiet = after_more == after_first;

    // And a read that *does* learn something still writes: a new block
    // from somewhere else must land in the ledger.
    let mut st = open_channel(&dir, 4).unwrap();
    st.doc.tell(&me("b"), "two").unwrap();
    st.store.flush(&st.doc).unwrap();
    drop(st);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let _ = open_channel(&dir, 5).unwrap();
    let after_new = fs::metadata(&ledger).unwrap().modified().unwrap();
    let noisy = after_new > after_more;

    let _ = fs::remove_dir_all(&dir);
    assert!(
        quiet,
        "five reads that learned nothing rewrote the ledger anyway"
    );
    assert!(noisy, "a read that met a new block must record it");
}

/// **Two compactors must not both run**, because two compactors on one
/// document destroy it (the test above has the mechanism, found the hard
/// way). Three of this fleet's machines share one `.khor` over NFS, so
/// 「the resident serve owns compaction」 is three owners unless
/// something makes it one.
///
/// Two threads enter `compact_piles_under` on the same home at the same
/// moment. Exactly one may come back having rewritten anything.
///
/// ⚠️ **Take `take_lease` out and this fails**, but not always on the
/// first run: without the lease the two threads can still miss each
/// other. So it runs the race many times and fails if *any* round had
/// two winners — and it prints how many rounds actually overlapped, so
/// a run where the threads never met cannot read as a pass.
#[test]
fn only_one_compactor_may_run_on_one_home() {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Barrier};

    let home = tmpdir("one-compactor");
    let winners = Arc::new(AtomicU64::new(0));
    let rounds = 24;

    for round in 0..rounds {
        // A fresh pile each round: compaction is idempotent, so without
        // this only the first round would have anything to win.
        let dir = home.join(format!("doc{round}"));
        let mut st = open_channel(&dir, 1).unwrap();
        for i in 0..(crate::store::PILE + 4) {
            st.doc.tell(&me("a"), &format!("line {i}")).unwrap();
            st.store.flush(&st.doc).unwrap();
        }
        let want = render(&st.doc);
        drop(st);

        let gate = Arc::new(Barrier::new(2));
        let hands: Vec<_> = (0..2)
            .map(|k| {
                let (home, gate, winners) = (home.clone(), Arc::clone(&gate), Arc::clone(&winners));
                std::thread::spawn(move || {
                    gate.wait();
                    let done = crate::store::compact_piles_under(&home, 100 + k);
                    if !done.is_empty() {
                        winners.fetch_add(1, Ordering::Relaxed);
                    }
                    done.len()
                })
            })
            .collect();
        let each: Vec<usize> = hands.into_iter().map(|h| h.join().unwrap()).collect();
        let ran = each.iter().filter(|n| **n > 0).count();
        assert!(
            ran <= 1,
            "round {round}: {ran} compactors both rewrote something"
        );

        // And the document is still whole, which is the thing the rule
        // exists to protect.
        let back = open_channel(&dir, 9).unwrap();
        let got = render(&back.doc);
        for l in want.lines() {
            assert!(got.contains(l), "round {round}: the document lost {l:?}");
        }
    }

    let won = winners.load(Ordering::Relaxed);
    let _ = fs::remove_dir_all(&home);
    // The arm that says the race was real: if no round ever compacted,
    // the assertions above were all vacuous.
    assert!(
        won > 0,
        "no round compacted anything, so nothing above was actually tested"
    );
}

/// **Two loads open at once in one process keep both their writes**
/// (round NET-6, the pairing row the term lane lost). A resident answers
/// requests concurrently, so the `Pair` arm's load of the device table and
/// any other write to it in the same process can be open at the same
/// time — and every load used to take the node's one peer. Loro names an
/// op by (peer, counter), so the two loads minted ops with the same names
/// and different contents; whichever block merged second was taken for
/// one already held and dropped, on every read after. The joiner, handed
/// the `Pair` arm's snapshot, kept the row; a sync could not bring it
/// back, because by version the desk already had it. That is the row
/// found only in the joiner's copy.
#[test]
fn two_loads_open_at_once_in_one_process_both_keep_their_writes() {
    use crate::devices::DeviceDoc;
    use crate::store::{Doc, load};
    let dir = tmpdir("samepeer");
    // One process, one node peer, two requests in flight.
    let pairing = load::<DeviceDoc>(&dir, 0xB1).unwrap();
    let other = load::<DeviceDoc>(&dir, 0xB1).unwrap();
    pairing.doc.upsert("phone", "phone", &[]).unwrap();
    other.doc.upsert("desk", "desk", &["10.0.0.1:1".to_owned()]).unwrap();
    let mut s = pairing.store;
    s.flush(&pairing.doc).unwrap();
    // What the joiner is handed.
    let joiner = DeviceDoc::open(0xC1).unwrap();
    joiner.merge(&pairing.doc.snapshot().unwrap()).unwrap();
    let mut s = other.store;
    s.flush(&other.doc).unwrap();
    drop((pairing.doc, other.doc));

    let desk = load::<DeviceDoc>(&dir, 0xB2).unwrap();
    assert!(desk.doc.get("desk").is_some(), "the other write is on disk");
    assert!(desk.doc.get("phone").is_some(), "the pairing's row is on disk");
    // And a round with the joiner leaves the two copies alike.
    desk.doc.merge(&joiner.changes_since(&desk.doc.version()).unwrap()).unwrap();
    joiner.merge(&desk.doc.changes_since(&joiner.version()).unwrap()).unwrap();
    assert_eq!(desk.doc.all().len(), joiner.all().len(), "the copies agree after a round");
    let _ = fs::remove_dir_all(&dir);
}

/// **Compaction deletes only what it read** (round NET-6). It loads the
/// pile, writes a snapshot of what it loaded, then deletes — and the
/// delete took whatever the directory listed by then, so a block another
/// writer flushed in between (the `Pair` arm, an app backend, a CLI verb:
/// none of them waits for the resident's store gate) was deleted unread,
/// its lines in nobody's snapshot. The ledger keeps the name, so no sync
/// pulls it back either.
#[test]
fn a_block_written_while_compacting_is_not_deleted_unread() {
    use crate::store::{AnyDoc, load};
    let dir = tmpdir("compactrace");
    let mut seed = open_channel(&dir, 0xD0).unwrap();
    seed.doc.tell(&me("a"), "before").unwrap();
    seed.store.flush(&seed.doc).unwrap();
    // The compactor has read the pile …
    let compactor = load::<AnyDoc>(&dir, 0xD1).unwrap();
    // … another writer flushes …
    let mut late = open_channel(&dir, 0xD2).unwrap();
    late.doc.tell(&me("a"), "during").unwrap();
    late.store.flush(&late.doc).unwrap();
    // … and the compactor rewrites.
    let mut store = compactor.store;
    store.compact(&compactor.doc).unwrap();
    let back = open_channel(&dir, 0xD3).unwrap();
    let text = render(&back.doc);
    assert!(text.contains("before") && text.contains("during"), "both lines survive:\n{text}");
    let _ = fs::remove_dir_all(&dir);
}

/// **A second compaction keeps the first one's snapshot's contents.** A
/// snapshot is named from the writer's next block number, and once
/// compaction has deleted that writer's blocks the number can come round
/// again: the new snapshot landed on the old one's name and the delete
/// that followed took it.
#[test]
fn compacting_twice_keeps_every_word() {
    use crate::store::{AnyDoc, load};
    let dir = tmpdir("compact2");
    for round in 0..2 {
        let mut w = open_channel(&dir, 0xE0).unwrap();
        w.doc.tell(&me("a"), &format!("line {round}")).unwrap();
        w.store.flush(&w.doc).unwrap();
        let c = load::<AnyDoc>(&dir, 0xE1).unwrap();
        let mut store = c.store;
        store.compact(&c.doc).unwrap();
    }
    let back = open_channel(&dir, 0xE2).unwrap();
    let text = render(&back.doc);
    assert!(text.contains("line 0") && text.contains("line 1"), "both lines survive:\n{text}");
    let _ = fs::remove_dir_all(&dir);
}

/// **A directory nothing has written to is not read again** (the owner's
/// Mac app at 70–260 % CPU, 2026-10-05: every list beat and every sync
/// round merged every block of every document). The second read of an
/// unchanged directory comes from what the first kept, and reads the
/// same; a write in between is seen, and a copy read from what was kept
/// writes like any other.
#[test]
fn an_unchanged_directory_is_read_from_what_was_kept() {
    let dir = tmpdir("kept");
    let mut st = open_channel(&dir, 1).unwrap();
    for i in 0..5 {
        st.doc.tell(&me("a"), &format!("line {i}")).unwrap();
        st.store.flush(&st.doc).unwrap();
    }
    drop(st);
    let first = render(&open_channel(&dir, 1).unwrap().doc);
    let kept_before = crate::store::loads_kept();
    let mut again = open_channel(&dir, 1).unwrap();
    assert!(crate::store::loads_kept() > kept_before, "the second read came from what the first kept");
    assert_eq!(render(&again.doc), first, "and it reads the same");

    // A copy read from what was kept writes like any other…
    again.doc.tell(&me("a"), "written on a kept read").unwrap();
    again.store.flush(&again.doc).unwrap();
    drop(again);
    // …and a write from elsewhere is seen at once.
    let mut other = open_channel(&dir, 2).unwrap();
    other.doc.tell(&me("b"), "from another writer").unwrap();
    other.store.flush(&other.doc).unwrap();
    drop(other);
    let now = render(&open_channel(&dir, 1).unwrap().doc);
    assert!(now.contains("written on a kept read") && now.contains("from another writer"), "{now}");
    for i in 0..5 {
        assert!(now.contains(&format!("line {i}")), "{now}");
    }
    let _ = fs::remove_dir_all(&dir);
}
