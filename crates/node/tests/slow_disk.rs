//! A sleeping disk slows its own row and nothing else (docs/SHELL.md
//! 第十六轮).
//!
//! 2026-09-24: a USB disk that had gone to sleep answered its `statfs` in
//! 3.65 s, and because this machine's reading was taken inside every
//! request that wanted it, `khor devices` took 10–36 s and the app's
//! machine page waited with it. These tests put a volume that takes ten
//! seconds to answer **behind the injection point**
//! (`khor_node::vitals::local::install`) — never a real disk: a test that
//! waited on this machine's own sleeping disk would be measuring the
//! machine, not khor.

mod util;

use std::sync::Arc;
use std::time::{Duration, Instant};

use khor_node::Node;
use khor_node::vitals::local::{self, Probe};

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-slow-disk-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// Two volumes that do not exist: one answers at once, one after
/// `slow`. `first` holds up the very first list of mounts, so a test can
/// look at the sampler before its first round is done.
struct Fake {
    slow: Duration,
    first: Duration,
    listed: std::sync::atomic::AtomicBool,
}

impl Fake {
    fn new(slow: Duration, first: Duration) -> Arc<Fake> {
        Arc::new(Fake {
            slow,
            first,
            listed: std::sync::atomic::AtomicBool::new(false),
        })
    }
}

const FAST: &str = "/khor-test-fast-volume";
const SLOW: &str = "/khor-test-slow-volume";

impl Probe for Fake {
    fn mounts(&self) -> Vec<String> {
        if !self.listed.swap(true, std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(self.first);
        }
        vec![FAST.into(), SLOW.into()]
    }
    fn space(&self, mount: &str) -> Option<(u64, u64)> {
        if mount == SLOW {
            std::thread::sleep(self.slow);
        }
        Some((500 << 30, 200 << 30))
    }
}

/// Every ask for this machine's reading comes back within 50 ms, even
/// while one of its volumes takes ten seconds to answer — and once the
/// reading is there, that volume is named as stale.
#[test]
fn a_sleeping_volume_does_not_hold_up_this_machines_reading() {
    let home = root("latency");
    let n = Node::open_as(home.0.clone(), "alpha").unwrap();
    local::install(
        &home.0,
        Fake::new(Duration::from_secs(10), Duration::ZERO),
        local::EVERY,
    );
    let me = n.device_str().to_owned();
    let end = Instant::now() + Duration::from_secs(8);
    let mut worst = Duration::ZERO;
    let reading = loop {
        let t = Instant::now();
        let got = n.local_vitals();
        let took = t.elapsed();
        worst = worst.max(took);
        assert!(
            took < Duration::from_millis(50),
            "this machine's reading took {} ms to answer — a volume's statfs is in the request",
            took.as_millis()
        );
        if let Some(r) = got {
            break r;
        }
        assert!(
            Instant::now() < end,
            "no reading within 8 s of starting the sampler"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let t = Instant::now();
    assert!(n.vitals_of(&me).is_some());
    worst = worst.max(t.elapsed());
    assert!(
        reading.stale.iter().any(|m| m == SLOW),
        "the ten-second volume is not marked stale: {:?}",
        reading.stale
    );
    assert!(
        reading.vitals.volumes.iter().any(|v| v.mount == FAST),
        "the volume that answered is missing: {:?}",
        reading.vitals.volumes
    );
    eprintln!("worst vitals_of(self) latency: {} us", worst.as_micros());
}

/// Before the sampler's first round this machine has no reading — the
/// third state, 「还没问过」 — and after it the reading ages like any
/// other: its age grows with the clock until the next round.
#[test]
fn this_machine_has_no_reading_before_its_first_round_and_an_aging_one_after() {
    let home = root("age");
    let n = Node::open_as(home.0.clone(), "alpha").unwrap();
    local::install(
        &home.0,
        Fake::new(Duration::ZERO, Duration::from_millis(400)),
        Duration::from_secs(600),
    );
    let me = n.device_str().to_owned();
    assert!(
        n.vitals_of(&me).is_none(),
        "this machine had a reading before its sampler's first round"
    );
    let end = Instant::now() + Duration::from_secs(8);
    let (_, a) = loop {
        if let Some(r) = n.vitals_of(&me) {
            break r;
        }
        assert!(Instant::now() < end, "no reading within 8 s");
        std::thread::sleep(Duration::from_millis(20));
    };
    std::thread::sleep(Duration::from_millis(300));
    let (_, b) = n.vitals_of(&me).unwrap();
    assert!(
        b >= a + 250,
        "the reading's age did not grow with the clock: {a} ms, then {b} ms 300 ms later"
    );
}
