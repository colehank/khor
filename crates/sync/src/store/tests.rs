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

/// **A snapshot pulled from another machine is not taken by this one's
/// compaction** (round NET-10, the owner's hinton). Blocks cross between
/// machines under their own names, snapshots included, and before round
/// NET-6 a snapshot's name was only a number: this machine's count of
/// its own increments. Two machines whose compactors had written the
/// same count named their snapshots alike, so a local compaction wrote
/// over the one it had pulled and then deleted it with the blocks it
/// covered, and the document was empty from then on. hinton's device
/// table: every row gone, its ledger still naming `snap-00000040`, which
/// lecun's ledger names too. Red on 0fd2715b^, green since.
#[test]
fn a_snapshot_pulled_from_another_machine_survives_a_local_compaction() {
    use crate::store::{AnyDoc, load};
    let (far, here) = (tmpdir("pulled-far"), tmpdir("pulled-here"));
    let mut w = open_channel(&far, 0xF1).unwrap();
    w.doc.tell(&me("far"), "written far away").unwrap();
    w.store.flush(&w.doc).unwrap();
    let c = load::<AnyDoc>(&far, 0xF2).unwrap();
    let mut store = c.store;
    let snap = store.compact(&c.doc).unwrap();

    let mut h = open_channel(&here, 0xE1).unwrap();
    h.doc.tell(&me("here"), "written here").unwrap();
    h.store.flush(&h.doc).unwrap();
    let name = snap.file_name().unwrap().to_string_lossy().to_string();
    h.store.absorb(&h.doc, &name, &fs::read(&snap).unwrap()).unwrap();
    let c = load::<AnyDoc>(&here, 0xF2).unwrap();
    let mut store = c.store;
    store.compact(&c.doc).unwrap();

    let text = render(&open_channel(&here, 0xE3).unwrap().doc);
    assert!(
        text.contains("written far away") && text.contains("written here"),
        "both lines survive the compaction:\n{text}"
    );
    let _ = fs::remove_dir_all(&far);
    let _ = fs::remove_dir_all(&here);
}

/// **Copies in turn write under one name** (PEERS-BRIEF, 2026-10-10).
/// Every process used to bring a fresh name and leave a row in the version
/// vector for good: the owner's devices pile had 1515. Fifty copies opened
/// one after another, each writing, add one name between them, and each
/// carries on from the last one's counter, so no line is lost.
#[test]
fn copies_in_turn_write_under_one_name() {
    let dir = tmpdir("pens-in-turn");
    // The directory has to exist for it to have pens.
    let mut seed = open_channel(&dir, 0x5E).unwrap();
    seed.doc.tell(&me("a"), "seed").unwrap();
    seed.store.flush(&seed.doc).unwrap();
    drop(seed);
    for i in 0..50 {
        let mut c = open_channel(&dir, 0x100 + i).unwrap();
        c.doc.tell(&me("a"), &format!("turn {i}")).unwrap();
        c.store.flush(&c.doc).unwrap();
    }
    let back = open_channel(&dir, 0x200).unwrap();
    let text = render(&back.doc);
    for i in 0..50 {
        assert!(text.contains(&format!("turn {i}")), "turn {i} is there:\n{text}");
    }
    assert_eq!(back.doc.version().len(), 2, "the seed's name and one pen");
    let _ = fs::remove_dir_all(&dir);
}

/// The other half of `pens_across_processes`, run as a child process:
/// takes a pen, says which, waits until every sibling holds one, writes,
/// flushes and exits. Does nothing when run as a test of its own.
#[test]
fn pen_child() {
    let (Ok(dir), Ok(tag), Ok(wave)) = (
        std::env::var("KHOR_PEN_DIR"),
        std::env::var("KHOR_PEN_TAG"),
        std::env::var("KHOR_PEN_WAVE"),
    ) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let wave: usize = wave.parse().unwrap();
    let mut c = open_channel(&dir, crate::chat::ChatDoc::fresh_peer()).unwrap();
    println!("pen={:016x}", crate::store::Doc::peer_id(&c.doc));
    let ready = dir.with_extension("ready");
    fs::create_dir_all(&ready).unwrap();
    fs::write(ready.join(&tag), b"").unwrap();
    let began = std::time::Instant::now();
    while fs::read_dir(&ready).unwrap().count() < wave && began.elapsed().as_secs() < 20 {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    c.doc.tell(&me("a"), &format!("from child {tag}")).unwrap();
    c.store.flush(&c.doc).unwrap();
}

/// **Processes at once never share a name, processes in turn reuse them**
/// (PEERS-BRIEF). Two waves of four processes, each holding its copy
/// until all four of its wave hold one: the four of a wave have four
/// names, the second wave's are the first wave's, and every line is on
/// disk after (a shared name would have lost one).
#[test]
fn pens_across_processes() {
    let dir = tmpdir("pens-processes");
    let mut seed = open_channel(&dir, 0x5E).unwrap();
    seed.doc.tell(&me("a"), "seed").unwrap();
    seed.store.flush(&seed.doc).unwrap();
    drop(seed);
    let exe = std::env::current_exe().unwrap();
    let mut waves = Vec::new();
    for wave in 0..2 {
        let _ = fs::remove_dir_all(dir.with_extension("ready"));
        let children: Vec<_> = (0..4)
            .map(|i| {
                std::process::Command::new(&exe)
                    .args(["--exact", "store::tests::pen_child", "--nocapture", "--test-threads=1"])
                    .env("KHOR_PEN_DIR", &dir)
                    .env("KHOR_PEN_TAG", format!("{wave}-{i}"))
                    .env("KHOR_PEN_WAVE", "4")
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        let names: std::collections::BTreeSet<String> = children
            .into_iter()
            .map(|c| {
                let out = c.wait_with_output().unwrap();
                assert!(out.status.success(), "a child failed");
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    // The harness's「test … 」may share the line.
                    .find_map(|l| l.split_once("pen=").map(|(_, n)| n[..16].to_owned()))
                    .expect("the child says its name")
            })
            .collect();
        assert_eq!(names.len(), 4, "four processes at once, four names: {names:?}");
        waves.push(names);
    }
    assert_eq!(waves[0], waves[1], "the second wave writes under the first wave's names");
    let back = open_channel(&dir, 0x200).unwrap();
    let text = render(&back.doc);
    for tag in ["0-0", "0-1", "0-2", "0-3", "1-0", "1-1", "1-2", "1-3"] {
        assert!(text.contains(&format!("from child {tag}")), "child {tag}'s line is there:\n{text}");
    }
    assert_eq!(back.doc.version().len(), 5, "the seed's name and four pens");
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(dir.with_extension("ready"));
}

/// An old pile with `names` writers, each a block of its own, as every
/// khor process used to leave one: rows of the device table, with an
/// address each so the rows are nested maps. Returns what it holds.
fn old_pile(dir: &std::path::Path, names: u64, tag: &str) -> crate::devices::DeviceDoc {
    use crate::devices::DeviceDoc;
    use crate::store::Doc;
    fs::create_dir_all(dir).unwrap();
    let all = DeviceDoc::open(0xACC).unwrap();
    for i in 0..names {
        let peer = 0x1000 + i;
        let w = DeviceDoc::open(peer).unwrap();
        w.merge(&all.snapshot().unwrap()).unwrap();
        let before = w.version();
        // A few rows, rewritten over and over, as the real pile is.
        let row = format!("{tag}-{}", i % 5);
        w.upsert(&row, &row, &[format!("10.0.0.{i}:1")]).unwrap();
        let bytes = w.changes_since(&before).unwrap();
        fs::write(dir.join(format!("u-{peer:016x}-00000000.loro")), &bytes).unwrap();
        all.merge(&bytes).unwrap();
    }
    all
}

/// **An old pile is rebuilt under one name and reads the same**
/// (PEERS-BRIEF step 3). Forty writers' blocks become one snapshot by one
/// writer, the ledger and the old pens go, pen 0 holds the new name and
/// the epoch is written; the next copy writes under that name and its
/// counter carries on. Opening it again does not rebuild it again.
#[test]
fn an_old_pile_is_reseeded_under_one_name() {
    use crate::devices::DeviceDoc;
    use crate::store::{Doc, EPOCH, load, load_pile};
    let dir = tmpdir("reseed");
    let all = old_pile(&dir, 40, "row");
    // Read the old way once: the ledger and a pen exist, as on a real disk.
    drop(load::<DeviceDoc>(&dir, 0x77).unwrap());
    assert!(dir.join(".merged").exists() && dir.join(".pen-0").exists());
    let old_pen = fs::read_to_string(dir.join(".pen-0")).unwrap();
    assert_eq!(all.version().len(), 40);

    let mut p = load_pile::<DeviceDoc>(&dir, 0x78).unwrap();
    assert_eq!(format!("{:?}", p.doc.all()), format!("{:?}", all.all()), "and reads the same");
    assert_eq!(p.doc.version().len(), 1, "one writer name: {:?}", p.doc.version());
    assert_eq!(p.store.epoch(), EPOCH);
    assert_eq!(fs::read_to_string(dir.join(".epoch")).unwrap().trim(), EPOCH.to_string());
    let loro: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".loro"))
        .collect();
    assert_eq!(loro.len(), 1, "one block, the snapshot: {loro:?}");
    let pen = fs::read_to_string(dir.join(".pen-0")).unwrap();
    assert_ne!(pen, old_pen, "the old pens are gone");
    assert_eq!(p.doc.peer_id(), u64::from_str_radix(pen.trim(), 16).unwrap(), "the copy holds pen 0, the new name");

    p.doc.upsert("after", "after", &[]).unwrap();
    p.store.flush(&p.doc).unwrap();
    drop(p);
    let again = load_pile::<DeviceDoc>(&dir, 0x79).unwrap();
    assert_eq!(fs::read_to_string(dir.join(".pen-0")).unwrap(), pen, "not rebuilt twice");
    assert!(again.doc.get("after").is_some());
    assert_eq!(again.doc.version().len(), 1, "the write carried on under the one name");
    let _ = fs::remove_dir_all(&dir);
}

/// **A pile somebody holds is not rebuilt under them.** A copy read
/// before a rebuild would write after it, on top of blocks that are gone;
/// so the rebuild waits until nobody holds the pile, and meanwhile the
/// copy that asked says the old epoch.
#[test]
fn a_held_pile_is_reseeded_only_once_let_go() {
    use crate::devices::DeviceDoc;
    use crate::store::{Doc, EPOCH, load, load_pile};
    let dir = tmpdir("reseed-held");
    old_pile(&dir, 6, "row");
    drop(load::<DeviceDoc>(&dir, 0x80).unwrap());
    let held = load::<DeviceDoc>(&dir, 0x81).unwrap();
    let asked = load_pile::<DeviceDoc>(&dir, 0x82).unwrap();
    assert_eq!(asked.store.epoch(), 0, "not rebuilt while held");
    assert_eq!(asked.doc.version().len(), 6, "the old names");
    drop((held, asked));
    let after = load_pile::<DeviceDoc>(&dir, 0x83).unwrap();
    assert_eq!(after.store.epoch(), EPOCH, "rebuilt once let go");
    assert_eq!(after.doc.version().len(), 1);
    let _ = fs::remove_dir_all(&dir);
}

/// **Two replicas re-seeded apart agree after a round** (PEERS-BRIEF):
/// each machine rebuilds its own pile under its own name, so the two
/// rebuilt histories are unrelated; for a map of last-writer-wins rows
/// that is two concurrent writes of every key, and a round settles every
/// key alike on both sides. What each wrote only on its own side before
/// the rebuild is on both after.
#[test]
fn two_replicas_reseeded_apart_agree_after_a_round() {
    use crate::devices::DeviceDoc;
    use crate::store::{Doc, EPOCH, load_pile};
    use crate::wire::{Peer, answer};
    let (a_dir, b_dir) = (tmpdir("reseed-a"), tmpdir("reseed-b"));
    let shared = old_pile(&a_dir, 20, "row");
    fs::create_dir_all(&b_dir).unwrap();
    for e in fs::read_dir(&a_dir).unwrap().flatten() {
        fs::copy(e.path(), b_dir.join(e.file_name())).unwrap();
    }
    // Apart before the rebuild: a row each, and the same row rewritten.
    for (dir, peer, mine) in [(&a_dir, 0xA0u64, "only-a"), (&b_dir, 0xB0, "only-b")] {
        let w = DeviceDoc::open(peer).unwrap();
        w.merge(&shared.snapshot().unwrap()).unwrap();
        let v = w.version();
        w.upsert(mine, mine, &[]).unwrap();
        w.upsert("row-0", mine, &[]).unwrap();
        fs::write(dir.join(format!("u-{peer:016x}-00000000.loro")), w.changes_since(&v).unwrap()).unwrap();
    }
    let mut a = load_pile::<DeviceDoc>(&a_dir, 0xA1).unwrap();
    let mut b = load_pile::<DeviceDoc>(&b_dir, 0xB1).unwrap();
    assert_eq!((a.store.epoch(), b.store.epoch()), (EPOCH, EPOCH));
    assert_ne!(a.doc.peer_id(), b.doc.peer_id(), "each under its own name");

    let mut peer = Peer::new();
    for _ in 0..2 {
        peer.round(&mut a.store, &a.doc, |h, c| answer(&mut b.store, &b.doc, h, c)).unwrap();
    }
    let (on_a, on_b) = (format!("{:?}", a.doc.all()), format!("{:?}", b.doc.all()));
    assert_eq!(on_a, on_b, "every key agrees");
    for row in ["only-a", "only-b", "row-0", "row-1", "row-4"] {
        assert!(a.doc.get(row).is_some(), "{row} is there:\n{on_a}");
    }
    let name = a.doc.get("row-0").unwrap().name;
    assert!(name == "only-a" || name == "only-b", "the rewritten row is one side's: {name}");
    assert_eq!(a.doc.version().len(), 2, "two names in all");
    let _ = fs::remove_dir_all(&a_dir);
    let _ = fs::remove_dir_all(&b_dir);
}

/// **No op leaves before it is on disk** (PEERS-BRIEF, peers-a 「Left」
/// 3). A copy writes and, without flushing, answers a request, pushes on a
/// round and hands out its table: each time, a copy read from disk right
/// after already holds the line.
#[test]
fn every_way_ops_leave_flushes_them_first() {
    use crate::wire::{Peer, answer, table_b64, version_b64};
    let (dir, far_dir) = (tmpdir("flush-first"), tmpdir("flush-first-far"));
    let mut far = open_channel(&far_dir, 0x3F).unwrap();
    far.doc.tell(&me("far"), "far").unwrap();
    far.store.flush(&far.doc).unwrap();
    let on_disk = |line: &str| render(&open_channel(&dir, 0x3E).unwrap().doc).contains(line);

    let mut w = open_channel(&dir, 0x31).unwrap();
    w.doc.tell(&me("a"), "answered").unwrap();
    let theirs = version_b64(&far.store, &far.doc);
    let reply = answer(&mut w.store, &w.doc, &theirs, "").unwrap();
    assert!(!reply.changes.is_empty(), "the line left");
    assert!(on_disk("answered"), "and it was on disk first");

    w.doc.tell(&me("a"), "pushed").unwrap();
    let out = Peer::knowing(version_b64(&far.store, &far.doc)).outgoing(&mut w.store, &w.doc).unwrap();
    assert!(out.pushes());
    assert!(on_disk("pushed"));

    w.doc.tell(&me("a"), "handed out").unwrap();
    table_b64(&mut w.store, &w.doc).unwrap();
    assert!(on_disk("handed out"));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&far_dir);
}

/// **A pen whose holder sent and died before flushing is retired**
/// (PEERS-BRIEF, peers-a 「Left」 3). The holder sent two lines to one
/// machine and four to another, unflushed, the way every khor before
/// this one pushed on a round, and died. The next holder takes the same
/// name, writes over the first counters (that collision is already paid),
/// and on its next round learns from the first machine that the name has
/// ops this disk never kept: the pen gets a fresh name. So the copy after
/// it writes under a new name, and the second machine's two later lines
/// and its own line cross both ways; under the old name its counter would
/// land on those two lines and one side would skip the other's.
#[test]
fn a_pen_whose_holder_sent_unflushed_and_died_is_retired() {
    use crate::store::Doc;
    use crate::wire::{Peer, answer};
    let (dir, f_dir, g_dir) = (tmpdir("retire"), tmpdir("retire-f"), tmpdir("retire-g"));
    let mut seed = open_channel(&dir, 0x50).unwrap();
    seed.doc.tell(&me("a"), "seed").unwrap();
    seed.store.flush(&seed.doc).unwrap();
    drop(seed);
    let (mut f, mut g) = (open_channel(&f_dir, 0x5F).unwrap(), open_channel(&g_dir, 0x56).unwrap());

    let a = open_channel(&dir, 0x51).unwrap();
    let name = a.doc.peer_id();
    for i in 1..=2 {
        a.doc.tell(&me("a"), &format!("lost {i}")).unwrap();
    }
    f.doc.merge(&a.doc.changes_since(&f.doc.version()).unwrap()).unwrap();
    for i in 3..=4 {
        a.doc.tell(&me("a"), &format!("lost {i}")).unwrap();
    }
    g.doc.merge(&a.doc.changes_since(&g.doc.version()).unwrap()).unwrap();
    drop(a);

    let mut b = open_channel(&dir, 0x52).unwrap();
    assert_eq!(b.doc.peer_id(), name, "the next holder takes the same name");
    b.doc.tell(&me("a"), "after").unwrap();
    let mut peer = Peer::new();
    for _ in 0..2 {
        peer.round(&mut b.store, &b.doc, |h, c| answer(&mut f.store, &f.doc, h, c)).unwrap();
    }
    assert_eq!(b.store.retired(), Some(name), "the name is retired");
    drop(b);

    let mut c = open_channel(&dir, 0x53).unwrap();
    assert_ne!(c.doc.peer_id(), name, "the copy after writes under a new name");
    c.doc.tell(&me("a"), "from c").unwrap();
    let mut peer = Peer::new();
    for _ in 0..2 {
        peer.round(&mut c.store, &c.doc, |h, cs| answer(&mut g.store, &g.doc, h, cs)).unwrap();
    }
    let (here, there) = (render(&c.doc), render(&g.doc));
    assert!(there.contains("from c"), "its line reached the second machine:\n{there}");
    assert!(here.contains("lost 3") && here.contains("lost 4"), "and that machine's lines came here:\n{here}");
    for d in [dir, f_dir, g_dir] {
        let _ = fs::remove_dir_all(d);
    }
}

/// **A read after one new block imports that block, not the pile**
/// (peers-b, the owner's Mac 2026-10-10 19:20). Ten blocks read and kept;
/// one more written; the next read is one pass that imports one block,
/// and reads every line.
#[test]
fn a_read_after_one_new_block_imports_only_that_block() {
    use crate::store::reads_of;
    let dir = tmpdir("kept-plus");
    for i in 0..10 {
        let mut w = open_channel(&dir, 0x60 + i).unwrap();
        w.doc.tell(&me("a"), &format!("line {i}")).unwrap();
        w.store.flush(&w.doc).unwrap();
    }
    drop(open_channel(&dir, 0x70).unwrap());
    let mut w = open_channel(&dir, 0x71).unwrap();
    w.doc.tell(&me("a"), "the new one").unwrap();
    w.store.flush(&w.doc).unwrap();
    drop(w);
    let (passes, blocks) = reads_of(&dir);
    let back = render(&open_channel(&dir, 0x72).unwrap().doc);
    let (passes2, blocks2) = reads_of(&dir);
    assert_eq!((passes2 - passes, blocks2 - blocks), (1, 1), "one pass, one block");
    for i in 0..10 {
        assert!(back.contains(&format!("line {i}")), "{back}");
    }
    assert!(back.contains("the new one"), "{back}");
    let _ = fs::remove_dir_all(&dir);
}

/// **Concurrent readers of a changed directory make one read between
/// them**: eight threads at once after a write; one reads, the others wait
/// and are answered from what it kept, and all eight see the write.
#[test]
fn concurrent_readers_of_a_changed_directory_read_once() {
    use crate::store::reads_of;
    use std::sync::{Arc, Barrier};
    let dir = tmpdir("kept-gate");
    for i in 0..20 {
        let mut w = open_channel(&dir, 0x90 + i).unwrap();
        w.doc.tell(&me("a"), &format!("line {i}")).unwrap();
        w.store.flush(&w.doc).unwrap();
    }
    drop(open_channel(&dir, 0xA0).unwrap());
    let mut w = open_channel(&dir, 0xA1).unwrap();
    w.doc.tell(&me("a"), "written since").unwrap();
    w.store.flush(&w.doc).unwrap();
    drop(w);
    let (passes, _) = reads_of(&dir);
    let start = Arc::new(Barrier::new(8));
    let readers: Vec<_> = (0..8)
        .map(|i| {
            let (dir, start) = (dir.clone(), start.clone());
            std::thread::spawn(move || {
                start.wait();
                render(&open_channel(&dir, 0xB0 + i).unwrap().doc)
            })
        })
        .collect();
    for r in readers {
        assert!(r.join().unwrap().contains("written since"));
    }
    assert_eq!(reads_of(&dir).0 - passes, 1, "one read between the eight");
    let _ = fs::remove_dir_all(&dir);
}
