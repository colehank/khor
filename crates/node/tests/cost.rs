//! What the session list costs, measured rather than guessed.
//!
//! `#[ignore]` on purpose, like the two real-disk checks: this reads the
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

use khor_node::adaptor::Procs;
use khor_node::Node;

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
    println!("process-table scans       : {scans} over 12 calls = {} per list", scans / 12);

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
    println!("at one call per 5s poll   : {:.3}% of a core", median / 5000.0 * 100.0);
}
