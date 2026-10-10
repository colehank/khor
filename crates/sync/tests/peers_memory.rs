//! **Memory per merge does not scale with the peers** (peers-a,
//! 2026-10-10). loro 1.16.2 deep-copied the version vector of every
//! merged change (`ImVersionVector::extend_to_include_vv` called
//! `im::HashMap::get_mut` per peer) and the dag cached the copy for good:
//! the devices pile, 1515 peers, kept ~1.5 MB per sync round and held
//! 320 MB of live heap after loading and the first concurrent import
//! (37 MB patched).
//! Patched in vendor/loro-internal (KHOR.md). Counts live heap bytes with
//! its own allocator, so it is a binary of its own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, Ordering};

use loro::{ExportMode, LoroDoc};

struct Counting;
static LIVE: AtomicI64 = AtomicI64::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size() as i64, Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n as i64 - l.size() as i64, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static A: Counting = Counting;

/// The counter is per process: one test at a time.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn live_kb() -> i64 {
    LIVE.load(Ordering::Relaxed) / 1024
}

const PEERS: u64 = 1500;

/// A history with `PEERS` writers, each forked from a version up to three
/// writers old, so most changes are merges, as in the real piles.
fn history() -> LoroDoc {
    let a = LoroDoc::new();
    a.set_peer_id(7_000_001).unwrap();
    let mut stale = a.export(ExportMode::Snapshot).unwrap();
    for p in 0..PEERS {
        let w = LoroDoc::new();
        w.set_peer_id(1000 + p).unwrap();
        w.import(&stale).unwrap();
        w.get_map("devices").insert(&format!("k{}", p % 90), p as i64).unwrap();
        a.import(&w.export(ExportMode::updates(&a.oplog_vv())).unwrap()).unwrap();
        if p % 3 == 0 {
            stale = a.export(ExportMode::Snapshot).unwrap();
        }
    }
    a
}

/// Two replicas write concurrently and exchange, round after round.
/// Before the patch: 1660 KB per round; after: 18 KB.
#[test]
fn a_sync_round_keeps_little() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let a = history();
    let b = LoroDoc::new();
    b.set_peer_id(7_000_002).unwrap();
    b.import(&a.export(ExportMode::Snapshot).unwrap()).unwrap();
    let round = |i: i64| {
        a.get_map("devices").insert("ka", i).unwrap();
        a.commit();
        b.get_map("devices").insert("kb", i).unwrap();
        b.commit();
        let to_a = b.export(ExportMode::updates(&a.oplog_vv())).unwrap();
        let to_b = a.export(ExportMode::updates(&b.oplog_vv())).unwrap();
        a.import(&to_a).unwrap();
        b.import(&to_b).unwrap();
    };
    round(0);
    let before = live_kb();
    for i in 1..=20 {
        round(i);
    }
    let per_round = (live_kb() - before) / 20;
    assert!(per_round < 200, "{per_round} KB kept per round with {PEERS} peers");
}

/// The first concurrent import on a doc loaded from a snapshot computes
/// the version vector of every change it walks. Before: 148 MB; after: 8 MB.
#[test]
fn the_first_merge_after_a_load_stays_small() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let a = history();
    let snap = a.export(ExportMode::Snapshot).unwrap();
    a.get_map("devices").insert("ka", 1).unwrap();
    a.commit();
    let b = LoroDoc::new();
    b.set_peer_id(7_000_002).unwrap();
    b.import(&snap).unwrap();
    b.get_map("devices").insert("kb", 1).unwrap();
    b.commit();
    let delta = a.export(ExportMode::updates(&b.oplog_vv())).unwrap();
    let before = live_kb();
    b.import(&delta).unwrap();
    let grew = live_kb() - before;
    assert!(grew < 32 * 1024, "{grew} KB for the first concurrent import with {PEERS} peers");
}
