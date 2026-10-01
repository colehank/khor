//! **A node that cannot host never asks tmux, nor the process table**
//! (docs/IOS.md 第二步).
//!
//! On a phone the session list was swept every ten seconds by the day
//! sampler (`Node::day_sampler` → `record_day` → `sessions`), and each
//! sweep tried to start `tmux` — a `posix_spawn` the sandbox refuses, in
//! silence, forever. A node that cannot host now discovers nothing at
//! all: no vendor directories, no tmux, no process snapshot.
//!
//! **Measured, not inferred**: `KHOR_TMUX` points the multiplexer at a
//! script that writes down every time it is run, and `KHOR_TMUX_SOCKET`
//! opts a scratch home into asking it (the test door `Discovery::for_root`
//! already has). The control is a node that can host on the same doors:
//! the script runs and the process table is read — without that arm, a
//! fake tmux nobody could reach would pass the first one too.
//!
//! Its own binary because the two doors are process-wide environment,
//! and the two tests take turns on them.

use std::path::PathBuf;
use std::sync::Mutex;

use khor_node::Node;
use khor_node::adaptor::snapshots_taken;

/// A scratch tree that takes itself away however the test leaves (the
/// `Scratch` of `crates/node/src/tests.rs`, one copy per file).
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-nohost-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

/// The two tests share the environment; one at a time.
static TURN: Mutex<()> = Mutex::new(());

/// A tmux that writes a line to `marker` each time it is started, and
/// answers like a tmux with no server. Pointed at through `KHOR_TMUX`,
/// opted into through `KHOR_TMUX_SOCKET`.
fn fake_tmux(dir: &std::path::Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let marker = dir.join("tmux-ran");
    let bin = dir.join("tmux");
    std::fs::write(
        &bin,
        format!("#!/bin/sh\necho \"$@\" >> '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: the two tests in this binary hold `TURN` around every read
    // and write of the environment; nothing else in it runs.
    unsafe {
        std::env::set_var("KHOR_TMUX", &bin);
        std::env::set_var("KHOR_TMUX_SOCKET", "khor-nohost");
    }
    marker
}

#[test]
fn a_node_that_cannot_host_never_asks_tmux_nor_the_process_table() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let home = scratch("phone");
    let marker = fake_tmux(&home.0);
    let n = Node::open_hosting(home.0.join("root"), "phone", false).expect("a node opens");
    let snaps = snapshots_taken();
    // What the phone's resident does every ten seconds, and what every
    // list on its screen asks.
    for _ in 0..3 {
        n.sessions().expect("a list");
        n.record_day().expect("a sample");
    }
    assert!(
        !marker.exists(),
        "tmux was started on a node that cannot host: {:?}",
        std::fs::read_to_string(&marker)
    );
    assert_eq!(
        snapshots_taken(),
        snaps,
        "the process table was read on a node that cannot host"
    );
}

#[test]
fn a_node_that_hosts_does_ask_the_tmux_it_was_pointed_at() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let home = scratch("computer");
    let marker = fake_tmux(&home.0);
    let n = Node::open_hosting(home.0.join("root"), "desk", true).expect("a node opens");
    let snaps = snapshots_taken();
    n.sessions().expect("a list");
    assert!(
        marker.exists(),
        "the control: a computer's sweep starts the tmux it was pointed at"
    );
    assert!(
        snapshots_taken() > snaps,
        "the control: a computer's sweep reads the process table"
    );
}
