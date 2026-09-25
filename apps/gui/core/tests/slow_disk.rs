//! The app's device table does not wait for a sleeping disk
//! (docs/SHELL.md 第十六轮). The app asks for it every two seconds; on
//! 2026-09-24 each ask sampled this machine on the spot, one `statfs` per
//! volume under one process-wide lock, and a USB disk asleep made every
//! ask queue behind the one before.
//!
//! The slow volume is behind the injection point
//! (`khor_node::vitals::local::install`) — never a real disk.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use khor_node::Node;
use khor_node::vitals::local::{self, Probe};

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One volume that answers at once, one that takes ten seconds.
struct Fake;

impl Probe for Fake {
    fn mounts(&self) -> Vec<String> {
        vec![
            "/khor-test-fast-volume".into(),
            "/khor-test-slow-volume".into(),
        ]
    }
    fn space(&self, mount: &str) -> Option<(u64, u64)> {
        if mount == "/khor-test-slow-volume" {
            std::thread::sleep(Duration::from_secs(10));
        }
        Some((500 << 30, 200 << 30))
    }
}

/// Twenty asks at once, the way a busy app and a CLI and a peer might
/// land together: all twenty answer, and all within one second.
///
/// Each ask runs on its own thread with its own runtime, so a blocking
/// ask cannot stop the clock that measures it.
#[test]
fn twenty_device_tables_at_once_answer_within_a_second() {
    let home = std::env::temp_dir().join(format!("khor-gui-slow-disk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let home = Scratch(home);
    Node::open_as(home.0.clone(), "alpha").unwrap();
    local::install(&home.0, Arc::new(Fake), local::EVERY);

    let (tx, rx) = mpsc::channel();
    let t = Instant::now();
    for _ in 0..20 {
        let (tx, root) = (tx.clone(), home.0.clone());
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let rows = rt.block_on(khor_gui_core::list_devices(&root));
            let _ = tx.send(rows.map(|r| r.len()));
        });
    }
    let end = t + Duration::from_secs(1);
    let mut answered = 0;
    while answered < 20 {
        let left = end.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(rows) => {
                assert_eq!(rows, Ok(1), "the table is this one machine");
                answered += 1;
            }
            Err(_) => break,
        }
    }
    assert_eq!(
        answered, 20,
        "only {answered} of 20 device tables answered within 1 s — the asks are waiting on a disk"
    );
    eprintln!("20 parallel list_devices: {} ms", t.elapsed().as_millis());
}
