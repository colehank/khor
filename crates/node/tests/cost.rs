//! What the session list costs, measured rather than guessed.
//!
//! `#[ignore]` on purpose, like the real-disk checks: this reads the
//! machine it runs on, so the numbers are a property of that machine and
//! a threshold here would be a test that fails on someone else's laptop.
//! It exists to be re-run by hand when the answer might have changed —
//! "the list feels slow", or a machine with far more processes than the
//! one the recorded numbers came from.
//!
//! ```text
//! cargo test -p khor-node --test cost -- --ignored --nocapture --test-threads=1
//! ```
//!
//! **`--test-threads=1` is not tidiness.** The scan counter these tests
//! read (`adaptor::snapshots_taken`) is one number for the whole
//! process, so a test measuring "scans per list" while another is
//! snapshotting beside it measures both. Run in parallel on 2026-08-16
//! that came out as 21 scans over 12 calls; serially, 12 — and 21 would
//! have read as "the list scans twice", which is the exact wrong
//! conclusion the ledger already recorded once from guessing.
//!
//! The recorded numbers live where the decision they support lives:
//! `khor_node::adaptor::Procs::snapshot`, and
//! `khor_node::adaptor::tmux` for the multiplexer call.

use std::time::Instant;

use khor_node::Node;
use khor_node::adaptor::Procs;

fn ms(f: impl Fn()) -> f64 {
    let t = Instant::now();
    f();
    t.elapsed().as_secs_f64() * 1000.0
}

/// Times the process-table scan behind every session list.
///
/// Reported as a spread over several runs, not one number: the first
/// scan of a cold process table is not the one the GUI pays every five
/// seconds, and quoting the cold one would overstate the cost the same
/// way quoting only the warmest would hide it.
#[test]
#[ignore]
fn what_a_process_table_snapshot_costs() {
    // Warm the OS caches, then measure — and print the cold one anyway,
    // because "the first list after launch" is a real moment a user sees.
    let cold = ms(|| {
        Procs::snapshot();
    });

    let mut runs: Vec<f64> = (0..12)
        .map(|_| {
            ms(|| {
                Procs::snapshot();
            })
        })
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let procs = Procs::snapshot();
    let n = procs.len();
    let median = runs[runs.len() / 2];

    println!("processes on this machine : {n}");
    println!("cold snapshot             : {cold:.1} ms");
    println!(
        "warm snapshot             : min {:.1} / median {median:.1} / max {:.1} ms  (n={})",
        runs[0],
        runs[runs.len() - 1],
        runs.len()
    );
    println!(
        "at one snapshot per 5s poll: {:.3}% of a core",
        median / 5000.0 * 100.0
    );

    assert!(n > 0, "a machine running this test has processes on it");
}

/// What one poll of the session list costs end to end — the thing the
/// GUI actually calls, every few seconds.
///
/// This is the number that settles the question the ledger asked, and it
/// answers a second one the ledger only guessed at: **how many times a
/// single `sessions()` scans the process table.** `Node::sessions` walks
/// three kinds and only the live one sweeps, so the answer should be
/// once — and the way to see that is the ratio below, not by reading the
/// call graph and hoping.
#[test]
#[ignore]
fn what_one_poll_of_the_session_list_costs() {
    let home = std::env::temp_dir().join(format!("khor-cost-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let n = Node::open_as(home.clone(), "cost").unwrap();

    n.sessions().expect("warm the caches and the home");

    // Counted, not inferred from the call graph — that is the reading
    // the ledger got wrong.
    let before = khor_node::adaptor::snapshots_taken();
    let mut runs: Vec<f64> = (0..12)
        .map(|_| {
            ms(|| {
                n.sessions().unwrap();
            })
        })
        .collect();
    let scans = khor_node::adaptor::snapshots_taken() - before;
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let list = runs[runs.len() / 2];
    println!(
        "process-table scans       : {scans} over 12 calls = {} per list",
        scans / 12
    );

    let mut snaps: Vec<f64> = (0..12)
        .map(|_| {
            ms(|| {
                Procs::snapshot();
            })
        })
        .collect();
    snaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let snap = snaps[snaps.len() / 2];

    println!("sessions() median         : {list:.1} ms");
    println!("snapshot() median         : {snap:.1} ms");
    println!("ratio                     : {:.2}x", list / snap);
    println!(
        "at one poll per 5s        : {:.3}% of a core",
        list / 5000.0 * 100.0
    );

    let _ = std::fs::remove_dir_all(&home);
}

/// What asking the machine's tmux costs, per list.
///
/// Separate from the two above because it is **not** included in them:
/// `Node::open_as` under a temp home is not the real home's node, so it
/// never talks to tmux (`adaptor::Discovery::for_root`). A reader adding
/// up what one production poll costs has to add this one in.
///
/// It is also the only cost here that is a **subprocess**: a fork, an
/// exec, and a round trip to a server over a unix socket, where
/// everything else is a syscall. That is why it gets its own number
/// rather than being assumed small.
#[test]
#[ignore]
fn what_asking_tmux_costs() {
    use khor_node::adaptor::tmux::Tmux;

    let procs = Procs::snapshot();
    let tmux = Tmux::default_server();
    let first = tmux.sweep(&procs);
    println!("tmux sessions on this machine: {}", first.rows.len());
    if first.rows.is_empty() {
        println!("(no tmux server here — the numbers below would be the empty path)");
    }

    let mut runs: Vec<f64> = (0..12)
        .map(|_| {
            ms(|| {
                tmux.sweep(&procs);
            })
        })
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = runs[runs.len() / 2];
    println!(
        "one list-panes            : min {:.1} / median {median:.1} / max {:.1} ms  (n={})",
        runs[0],
        runs[runs.len() - 1],
        runs.len()
    );
    println!(
        "at one call per 5s poll   : {:.3}% of a core",
        median / 5000.0 * 100.0
    );
}

/// What one session list costs **in a home somebody actually uses**.
///
/// ⚠️ **The three tests above build their own empty home, and that is
/// why they read as 「the list is cheap」.** On 2026-09-11 the figure
/// they produce (7.9 ms in release) was found to be 95–131 ms in this
/// developer's real home — twelve to sixteen times more, because what
/// the list costs is reading documents and walking a registry, and a
/// fixture has neither. A cost test on a fixture answers a different
/// question than the one it gets read for.
///
/// This one opens the home `KHOR_HOME` names, or the default one. It
/// prints the size of what it measured beside the time, because a
/// duration without that is not comparable to a later run.
///
/// It writes: [`khor_sync::store::load`] saves a ledger on every read,
/// so this touches the same files the running app touches every two
/// seconds. It adds no kind of write the app does not already do.
///
/// ```text
/// cargo test -p khor-node --release --test cost -- --ignored --nocapture --test-threads=1 real_home
/// ```
#[test]
#[ignore]
fn what_one_poll_costs_in_a_real_home() {
    let root = Node::root_from_env();
    let sessions_on_disk = std::fs::read_dir(root.join(".khor").join("sessions"))
        .map(|d| d.flatten().count())
        .unwrap_or(0);
    let channels = std::fs::read_dir(root.join(".khor").join("chat"))
        .map(|d| d.flatten().count())
        .unwrap_or(0);
    if sessions_on_disk == 0 && channels == 0 {
        println!(
            "home at {} has nothing in it — this is the fixture case",
            root.display()
        );
        println!("(nothing measured: a real reading needs a real home)");
        return;
    }
    let Ok(n) = Node::open(root.clone()) else {
        println!("could not open {} — nothing measured", root.display());
        return;
    };

    // Warm once: the first list after opening pays for caches nobody
    // pays for on the beat.
    let _ = n.sessions();

    let loads_before = khor_sync::store::loads_taken();
    let nanos_before = khor_sync::store::load_nanos();
    let scans_before = khor_node::adaptor::snapshots_taken();
    let mut runs: Vec<f64> = (0..12)
        .map(|_| {
            ms(|| {
                let _ = n.sessions();
            })
        })
        .collect();
    let loads = khor_sync::store::loads_taken() - loads_before;
    let load_ms = (khor_sync::store::load_nanos() - nanos_before) as f64 / 1e6;
    let scans = khor_node::adaptor::snapshots_taken() - scans_before;
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = runs[runs.len() / 2];

    println!("home                      : {}", root.display());
    println!("sessions on disk          : {sessions_on_disk}");
    println!("chat channels             : {channels}");
    println!(
        "rows the list produced    : {}",
        n.sessions().map(|v| v.len()).unwrap_or(0)
    );
    println!(
        "sessions() over 12        : min {:.1} / median {median:.1} / max {:.1} ms",
        runs[0],
        runs[runs.len() - 1]
    );
    println!(
        "document loads            : {loads} over 12 calls = {} per list",
        loads / 12
    );
    println!(
        "  their total time        : {load_ms:.1} ms = {:.0}% of the 12 calls",
        load_ms / runs.iter().sum::<f64>() * 100.0
    );
    println!(
        "process-table scans       : {scans} over 12 calls = {} per list",
        scans / 12
    );
    println!(
        "at the app's 2 s beat     : {:.2}% of a core",
        median / 2000.0 * 100.0
    );
}

/// What an **empty** document read costs — the floor under every one of
/// the ten a session list does.
///
/// Asked because the real-home measurement above says document loads are
/// 73% of the list, and four of them are chat channels for machines this
/// one has never exchanged a word with: those directories do not exist.
/// If the floor is near zero, the cost is real data and the answer is to
/// read less often; if the floor is most of it, the cost is **opening a
/// document at all** and the answer is somewhere else entirely.
#[test]
#[ignore]
fn what_an_empty_document_read_costs() {
    use khor_sync::chat::ChatDoc;
    use khor_sync::store::{Loaded, load};

    let nowhere = std::env::temp_dir().join("khor-cost-no-such-channel");
    let _ = std::fs::remove_dir_all(&nowhere);
    // Warm.
    let _: Loaded<ChatDoc> = load(&nowhere, 1).unwrap();

    let mut runs: Vec<f64> = (0..24)
        .map(|_| {
            ms(|| {
                let _: Loaded<ChatDoc> = load(&nowhere, 1).unwrap();
            })
        })
        .collect();
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "empty document read       : min {:.2} / median {:.2} / max {:.2} ms  (n={})",
        runs[0],
        runs[runs.len() / 2],
        runs[runs.len() - 1],
        runs.len()
    );
    println!(
        "ten of these per list     : {:.1} ms",
        runs[runs.len() / 2] * 10.0
    );
}

/// What **compaction** would buy, measured on a copy so nothing real is
/// touched.
///
/// ⚠️ **`BlockStore::compact` has no caller in the product.** Only the
/// tests call it. So every document grows a block file per write and
/// never sheds one, and [`khor_sync::store::load`] replays the whole
/// history on every read — which is what the session list does ten times
/// a beat. On this machine after twelve days: the device table is **545
/// blocks**, 2.3 MB.
///
/// This test copies a document's directory to a temp dir, times a read,
/// compacts the copy, and times a read again. It answers 「how much of
/// the list's cost is history nobody compacted」 without deciding
/// anything: compaction deletes files a concurrent reader may be part
/// way through listing, and that is a product decision, not a
/// measurement.
#[test]
#[ignore]
fn what_compaction_would_buy() {
    use khor_sync::devices::DeviceDoc;
    use khor_sync::store::{Loaded, load};

    let src = Node::root_from_env().join(".khor").join("devices");
    let Ok(rd) = std::fs::read_dir(&src) else {
        println!("no device document at {} — nothing measured", src.display());
        return;
    };
    let files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    let dst = std::env::temp_dir().join("khor-cost-compaction");
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).unwrap();
    for f in &files {
        if f.is_file() {
            let _ = std::fs::copy(f, dst.join(f.file_name().unwrap()));
        }
    }
    println!("copied {} files from the device document", files.len());

    let read = |n: usize| -> f64 {
        let mut runs: Vec<f64> = (0..n)
            .map(|_| {
                ms(|| {
                    let _: Loaded<DeviceDoc> = load(&dst, 1).unwrap();
                })
            })
            .collect();
        runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        runs[runs.len() / 2]
    };

    let before = read(12);
    println!("read, as it is on disk    : {before:.1} ms");

    let loaded: Loaded<DeviceDoc> = load(&dst, 1).unwrap();
    let mut store = loaded.store;
    store.compact(&loaded.doc).unwrap();
    let left = std::fs::read_dir(&dst)
        .map(|d| d.flatten().count())
        .unwrap_or(0);
    println!("files after compaction    : {left}");

    let after = read(12);
    println!("read, compacted           : {after:.1} ms");
    println!(
        "saved per read            : {:.1} ms ({:.0}%)",
        before - after,
        (before - after) / before * 100.0
    );

    let _ = std::fs::remove_dir_all(&dst);
}

/// What each document in this home costs to read, one by one.
///
/// The list-level number says ten reads and 61% of the time; this says
/// **which** ten are worth attacking. Reported with the block count
/// beside each, because that is the thing that grows.
#[test]
#[ignore]
fn what_each_document_costs_to_read() {
    use khor_sync::store::{AnyDoc, Loaded, load};

    let home = Node::root_from_env().join(".khor");
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&home) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let blocks = |d: &std::path::Path| {
                std::fs::read_dir(d)
                    .map(|r| {
                        r.flatten()
                            .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
                            .count()
                    })
                    .unwrap_or(0)
            };
            if blocks(&p) > 0 {
                dirs.push(p.clone());
            }
            if let Ok(inner) = std::fs::read_dir(&p) {
                for e in inner.flatten() {
                    let q = e.path();
                    if q.is_dir() && blocks(&q) > 0 {
                        dirs.push(q);
                    }
                }
            }
        }
    }
    if dirs.is_empty() {
        println!("no documents under {} — nothing measured", home.display());
        return;
    }

    let mut rows: Vec<(String, usize, u64, f64)> = Vec::new();
    for d in dirs {
        let blocks = std::fs::read_dir(&d)
            .map(|r| {
                r.flatten()
                    .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
                    .count()
            })
            .unwrap_or(0);
        let bytes: u64 = std::fs::read_dir(&d)
            .map(|r| {
                r.flatten()
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0);
        let _: Loaded<AnyDoc> = load(&d, 1).unwrap();
        let mut runs: Vec<f64> = (0..8)
            .map(|_| {
                ms(|| {
                    let _: Loaded<AnyDoc> = load(&d, 1).unwrap();
                })
            })
            .collect();
        runs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let name = d.strip_prefix(&home).unwrap_or(&d).display().to_string();
        rows.push((name, blocks, bytes, runs[runs.len() / 2]));
    }
    rows.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());
    println!(
        "{:<22} {:>7} {:>10} {:>9}",
        "document", "blocks", "bytes", "read"
    );
    for (name, blocks, bytes, t) in &rows {
        println!("{name:<22} {blocks:>7} {bytes:>10} {t:>8.1} ms");
    }
    println!(
        "{:<22} {:>7} {:>10} {:>8.1} ms",
        "— all of them —",
        "",
        "",
        rows.iter().map(|r| r.3).sum::<f64>()
    );
}
