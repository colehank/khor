//! Why the window's list stayed on its skeleton (2026-11-16): the same call the
//! window makes, with a time rather than silence.
//!
//! `KHOR_PROBE_STORE` points it at the store a running app is serving — the
//! case the window is in, and one a temp directory does not reproduce.
#![cfg(windows)]

use std::path::PathBuf;
use std::time::Instant;

fn store_or(tag: &str) -> PathBuf {
    if let Some(path) = std::env::var_os("KHOR_PROBE_STORE") {
        return PathBuf::from(path);
    }
    let root = std::env::temp_dir().join(format!("khor-gui-list-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("KHOR_NO_MDNS", "1") };
    root
}

/// Every arrangement, each with its own time. A blocked call shows up as a
/// two-second beat that never lands; a slow one shows up as a number.
#[test]
fn every_arrangement_answers_with_a_time() {
    let root = store_or("arrange");
    println!("store: {}", root.display());
    for by in khor_node::list::Arrange::ALL {
        let key = by.key();
        let started = Instant::now();
        match khor_gui_core::list_sessions(&root, key) {
            Ok(rows) => println!("{key}: {} rows in {:?}", rows.len(), started.elapsed()),
            Err(error) => panic!("{key}: {error}"),
        }
    }
    if std::env::var_os("KHOR_PROBE_STORE").is_none() {
        let _ = std::fs::remove_dir_all(&root);
    }
}
