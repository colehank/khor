//! A tmux row's attacher is handed the modes and a fresh redraw, never
//! the ring (item 41).
//!
//! The ring of a host whose program is a tmux client is tmux's redraws
//! over and over — the owner's long-lived session measured 1 050 926 B —
//! and the phone parsed all of it, visibly, on every entry. tmux can draw
//! the screen afresh instead (`refresh-client` on khor's own client), and
//! the history is read from tmux (item 31). The attacher must still start
//! in the modes the client set (`ESC [ ? 1049 h` is sent once, at
//! attach, and may be trimmed past).
//!
//! A real host, a real pty, a private tmux server: a pane floods far past
//! the ring, then an attacher reads what the host sends in the first two
//! seconds.
#![cfg(unix)]
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::live::LiveKind;

fn tmux(sock: &str, args: &[&str]) -> String {
    let out = Command::new("tmux")
        .arg("-L")
        .arg(sock)
        .args(args)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .stdin(Stdio::null())
        .output()
        .expect("tmux runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_tmux_attacher_gets_the_modes_and_a_redraw_not_the_ring() {
    if Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(true, |s| !s.success())
    {
        eprintln!("skipped: no tmux on this machine");
        return;
    }
    let root = std::env::temp_dir().join(format!("khor-hosttmux-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let sock = format!("khor-test-41-{}", std::process::id());
    tmux(
        &sock,
        &[
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            "one",
            "-x",
            "80",
            "-y",
            "24",
            "/bin/sh",
        ],
    );
    let id = SessionId("shell/41-0".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "shell", "tmux", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        let cmd = ["tmux", "-L", &sock, "attach", "-t", "one"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let dir = k.dir_of(&id).expect("registered");
    // Past the ring: ~1.2 MiB of lines drawn to khor's client.
    tmux(
        &sock,
        &[
            "send-keys",
            "-t",
            "one",
            "i=0; while [ $i -lt 12000 ]; do printf 'flood-%06d 0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123\\n' $i; i=$((i+1)); done; echo DONE-MARK",
            "Enter",
        ],
    );
    let deadline = Instant::now() + Duration::from_secs(60);
    while !tmux(&sock, &["capture-pane", "-p", "-t", "one"]).contains("DONE-MARK") {
        assert!(Instant::now() < deadline, "the flood never finished");
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));

    let mut conn = khor_node::host::connect(&dir, 80, 24).expect("the host takes an attacher");
    conn.set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let mut got = Vec::new();
    let mut buf = [0u8; 65536];
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
    }
    tmux(&sock, &["kill-server"]);
    let _ = host.join();

    eprintln!("the attacher was handed {} bytes", got.len());
    let text = String::from_utf8_lossy(&got);
    assert!(
        got.len() < 64 * 1024,
        "the attacher was poured {} bytes — the ring, not a redraw",
        got.len()
    );
    assert!(
        text.contains("DONE-MARK"),
        "tmux's redraw never reached the attacher: {:?}",
        &text[..text.len().min(200)]
    );
    let alt = text
        .find("\x1b[?1049h")
        .expect("the alternate screen is set for the attacher");
    assert!(
        alt < text.find("DONE-MARK").unwrap(),
        "the modes come before the screen"
    );
    assert!(
        !text.contains("flood-000000 "),
        "the first flood line is not in the attach: the ring was not replayed"
    );
    let _ = std::fs::remove_dir_all(&root);
}
