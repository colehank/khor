//! Does a second `Node::open` on the same root — from another thread, in the
//! same process — wait behind the first? The window runs its resident and its
//! commands in one process, so this is the shape the list is in.
#![cfg(windows)]

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("khor-selfopen-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    unsafe { std::env::set_var("KHOR_NO_MDNS", "1") };
    root
}

/// The resident's node stays open; a command's node opens beside it.
#[test]
fn a_second_open_in_the_same_process_does_not_wait_forever() {
    let root = scratch("resident");
    let resident = khor_node::Node::open_as(root.clone(), "resident").expect("the resident opens");

    let (sender, receiver) = mpsc::channel();
    let worker_root = root.clone();
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let outcome = khor_gui_core::list_sessions(&worker_root, "recent")
            .map(|rows| format!("{} rows", rows.len()))
            .map_err(|error| error);
        let _ = sender.send((outcome, started.elapsed()));
    });

    match receiver.recv_timeout(Duration::from_secs(20)) {
        Ok((outcome, took)) => {
            println!("command answered in {took:?}: {outcome:?}");
            assert!(
                outcome.is_ok(),
                "the command must answer, not refuse: {outcome:?}"
            );
        }
        Err(_) => panic!("the command never answered within 20s — it waits behind the resident"),
    }

    drop(resident);
    let _ = std::fs::remove_dir_all(&root);
}
