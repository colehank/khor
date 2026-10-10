//! **A large devices catch-up, measured** (net-i, 2026-10-10): the shape
//! the iPhone met after its frame cap rose from 256 KiB to 4 MiB. Five
//! writers rewrite their rows and roads and sync among themselves; one of
//! them (the phone) is cut off early and keeps writing alone; then the two
//! sides exchange what the other lacks, as `wire::answer` does. Prints the
//! delta sizes, the time each step takes and this process's RSS.
//!
//! Not a gate: `cargo test -p khor-sync --test catchup -- --ignored --nocapture`.

use std::time::Instant;

use khor_sync::devices::DeviceDoc;
use khor_sync::store::Doc;

fn rss_mb() -> u64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse::<u64>().unwrap_or(0) / 1024
}

fn id(i: usize) -> String {
    format!("{i:02x}").repeat(32)
}

/// One machine's write: its row's addresses (ports churn) and a road to
/// each other machine.
fn write(doc: &DeviceDoc, me: usize, round: usize) {
    let addrs: Vec<String> = (0..8).map(|k| format!("219.142.45.{}:{}", 200 + me, 40000 + (round * 7 + k) % 20000)).collect();
    doc.upsert(&id(me), &format!("machine-{me}"), &addrs).unwrap();
    for to in 0..5 {
        if to != me {
            let json = format!(r#"{{"road":"direct","ms":{},"at":{}}}"#, (round + to) % 50, 1_791_500_000_000u64 + round as u64 * 60_000);
            doc.set_road(&id(me), &id(to), &json).unwrap();
        }
    }
}

/// What `a` has beyond `b`, as `wire::answer` computes it (in memory: no
/// store, so no epoch and no flush).
fn delta(a: &DeviceDoc, b: &DeviceDoc) -> Vec<u8> {
    a.changes_since(&b.version()).unwrap()
}

fn sync(a: &DeviceDoc, b: &DeviceDoc) {
    let to_b = delta(a, b);
    let to_a = delta(b, a);
    b.merge(&to_b).unwrap();
    a.merge(&to_a).unwrap();
}

#[test]
#[ignore = "a measurement; run by hand"]
fn a_large_devices_catch_up_is_cheap_enough() {
    let rounds: usize = std::env::var("CATCHUP_ROUNDS").ok().and_then(|r| r.parse().ok()).unwrap_or(1500);
    let docs: Vec<DeviceDoc> = (0..5).map(|i| DeviceDoc::new(100 + i as u64).unwrap()).collect();
    let phone = 4;
    let cut = rounds / 10;
    let began = Instant::now();
    for round in 0..rounds {
        for (me, doc) in docs.iter().enumerate() {
            write(doc, me, round);
        }
        // The fleet syncs in a ring each round; the phone only until the cut.
        for me in 0..5 {
            let next = (me + 1) % 5;
            if round >= cut && (me == phone || next == phone) {
                continue;
            }
            sync(&docs[me], &docs[next]);
        }
    }
    eprintln!("[catchup] built {rounds} rounds in {:?}, rss {} MB", began.elapsed(), rss_mb());

    let (mac, ph) = (&docs[0], &docs[phone]);
    let to_phone = delta(mac, ph);
    let to_mac = delta(ph, mac);
    eprintln!(
        "[catchup] delta mac→phone {} KiB, phone→mac {} KiB; rss {} MB",
        to_phone.len() / 1024,
        to_mac.len() / 1024,
        rss_mb()
    );
    let t = Instant::now();
    ph.merge(&to_phone).unwrap();
    eprintln!("[catchup] phone merges: {:?}, rss {} MB", t.elapsed(), rss_mb());
    let t = Instant::now();
    mac.merge(&to_mac).unwrap();
    eprintln!("[catchup] mac merges: {:?}, rss {} MB", t.elapsed(), rss_mb());
    let t = Instant::now();
    let rows = ph.all().len();
    eprintln!("[catchup] phone reads all rows ({rows}): {:?}", t.elapsed());
    let t = Instant::now();
    let snap = ph.snapshot().unwrap();
    eprintln!("[catchup] phone snapshot {} KiB: {:?}, rss {} MB", snap.len() / 1024, t.elapsed(), rss_mb());
}
