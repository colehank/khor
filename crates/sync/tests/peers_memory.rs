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

/// The table behind the numbers above (docstat's synthetic loop, which
/// stays a hand tool for the real store): live heap kept per round for
/// 10 to 3000 peers. Not a gate:
/// `cargo test --release -p khor-sync --test peers_memory -- --ignored --nocapture`.
#[test]
#[ignore]
fn kept_per_round_by_peers() {
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    for peers in [10u64, 300, 1500, 3000] {
        let a = LoroDoc::new();
        a.set_peer_id(7_000_001).unwrap();
        for p in 0..peers {
            let w = LoroDoc::new();
            w.set_peer_id(1000 + p).unwrap();
            w.import(&a.export(ExportMode::Snapshot).unwrap()).unwrap();
            w.get_map("devices").insert(&format!("k{}", p % 90), p as i64).unwrap();
            a.import(&w.export(ExportMode::updates(&a.oplog_vv())).unwrap()).unwrap();
        }
        let b = LoroDoc::new();
        b.set_peer_id(7_000_002).unwrap();
        b.import(&a.export(ExportMode::Snapshot).unwrap()).unwrap();
        let mut before = live_kb();
        for i in 0..=100 {
            // The first round walks the history once (a few MB at 1500
            // peers); what is kept after it is the question.
            if i == 1 {
                before = live_kb();
            }
            a.get_map("devices").insert("ka", i).unwrap();
            a.commit();
            b.get_map("devices").insert("kb", i).unwrap();
            b.commit();
            let to_a = b.export(ExportMode::updates(&a.oplog_vv())).unwrap();
            let to_b = a.export(ExportMode::updates(&b.oplog_vv())).unwrap();
            a.import(&to_a).unwrap();
            b.import(&to_b).unwrap();
        }
        println!("peers {peers:>5}: {:.1} KB kept per round", (live_kb() - before) as f64 / 100.0);
    }
}

/// **The re-seed, measured** (peers-b, PEERS-BRIEF step 3): a devices pile
/// on disk with `PEERS_RESEED` writers named as khor named them (full
/// width, so the version vector is as long as the owner's), each forked
/// from a stale copy and compacted to one snapshot; then the sync
/// request's version, a load and the first concurrent import with another
/// machine's copy, before and after `store::load_pile` rebuilds it. A
/// temp directory, never a real store.
///
/// `cargo test --release -p khor-sync --test peers_memory -- --ignored --nocapture reseed`
#[test]
#[ignore]
fn reseed_before_and_after() {
    use khor_sync::devices::DeviceDoc;
    use khor_sync::store::{Doc, Loaded, load, load_pile};
    use khor_sync::wire::version_b64;
    const PEERS_RESEED: u64 = 1515;
    let _one = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let root = std::env::temp_dir().join(format!("khor-peers-reseed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (pile, far) = (root.join("devices"), root.join("far"));
    std::fs::create_dir_all(&pile).unwrap();
    let all = DeviceDoc::open(7_000_001).unwrap();
    let mut stale = all.snapshot().unwrap();
    for p in 0..PEERS_RESEED {
        let w = DeviceDoc::open((p + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 1).unwrap();
        w.merge(&stale).unwrap();
        let row = format!("{:064x}", p % 12);
        for k in 0..3 {
            w.upsert(&row, &format!("m{}", p % 12), &[format!("10.0.{}.{k}:{}", p % 7, 40000 + p)]).unwrap();
        }
        all.merge(&w.changes_since(&all.version()).unwrap()).unwrap();
        if p % 3 == 0 {
            stale = all.snapshot().unwrap();
        }
    }
    let snap = all.snapshot().unwrap();
    std::fs::write(pile.join("snap-0000000000000001-00000000.loro"), &snap).unwrap();
    eprintln!("synthetic pile: {PEERS_RESEED} writers, snapshot {} KB", snap.len() / 1024);

    let measure = |label: &str, open: &dyn Fn(&std::path::Path, u64) -> Loaded<DeviceDoc>| {
        // Another machine's copy: the blocks, not this machine's pens.
        let _ = std::fs::remove_dir_all(&far);
        std::fs::create_dir_all(&far).unwrap();
        for e in std::fs::read_dir(&pile).unwrap().flatten() {
            if !e.file_name().to_string_lossy().starts_with(".pen-") {
                std::fs::copy(e.path(), far.join(e.file_name())).unwrap();
            }
        }
        let h0 = live_kb();
        let t = std::time::Instant::now();
        let here = open(&pile, 0xAA);
        let load = t.elapsed();
        let h1 = live_kb();
        let there = open(&far, 0xBB);
        let have = version_b64(&here.store, &here.doc);
        here.doc.upsert(&"aa".repeat(32), "here", &["10.9.9.9:1".to_owned()]).unwrap();
        there.doc.upsert(&"bb".repeat(32), "there", &["10.9.9.8:1".to_owned()]).unwrap();
        let delta = there.doc.changes_since(&here.doc.version()).unwrap();
        let h2 = live_kb();
        let t = std::time::Instant::now();
        here.doc.merge(&delta).unwrap();
        let first = t.elapsed();
        let h3 = live_kb();
        eprintln!(
            "{label}: names {:>5}, request version {:>6} B, load {:>7.1?} heap +{:>5} KB, first concurrent import {:>7.1?} heap +{:>5} KB",
            here.doc.version().len(),
            have.len(),
            load,
            h1 - h0,
            first,
            h3 - h2,
        );
    };
    // A fresh read (`store::read_once`, nothing kept): the pile's blocks
    // and `k` blocks from writers forked from it, so each is concurrent
    // with the others, as a sync round leaves them. Every one is an
    // import whose diff walks the version vector (the owner's Mac,
    // 2026-10-10 19:20: AppDag::vv_to_frontiers at the top of the stack).
    let fresh = |label: &str| {
        let base = std::fs::read_dir(&pile)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
            .map(|e| (e.file_name(), std::fs::read(e.path()).unwrap()))
            .collect::<Vec<_>>();
        let state = DeviceDoc::open(0xF0).unwrap();
        for (_, b) in &base {
            state.merge(b).unwrap();
        }
        for k in [1u64, 8, 32] {
            let dir = root.join(format!("fresh-{label}-{k}"));
            std::fs::create_dir_all(&dir).unwrap();
            for (n, b) in &base {
                std::fs::write(dir.join(n), b).unwrap();
            }
            for i in 0..k {
                let peer = 0x5000_0000 + i;
                let w = DeviceDoc::open(peer).unwrap();
                w.merge(&state.snapshot().unwrap()).unwrap();
                let v = w.version();
                w.upsert(&format!("{:064x}", i % 12), "w", &[format!("10.8.8.{i}:1")]).unwrap();
                std::fs::write(dir.join(format!("u-{peer:016x}-00000000.loro")), w.changes_since(&v).unwrap()).unwrap();
            }
            let t = std::time::Instant::now();
            let l = load::<DeviceDoc>(&dir, 0xF1).unwrap();
            eprintln!("{label}: fresh read of the pile + {k:>2} concurrent blocks: {:>8.1?} ({} names)", t.elapsed(), l.doc.version().len());
        }
    };
    fresh("before");
    measure("before", &|d, p| load::<DeviceDoc>(d, p).unwrap());
    let t = std::time::Instant::now();
    let l = load_pile::<DeviceDoc>(&pile, 0xCC).unwrap();
    eprintln!("re-seed: {:.1?}, epoch {}, names {}", t.elapsed(), l.store.epoch(), l.doc.version().len());
    assert_eq!(l.doc.version().len(), 1);
    drop(l);
    measure("after ", &|d, p| load_pile::<DeviceDoc>(d, p).unwrap());
    fresh("after ");
    let _ = std::fs::remove_dir_all(&root);
}
