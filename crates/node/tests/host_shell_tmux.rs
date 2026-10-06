//! **A tmux the person started in a khor shell is not handed out black**
//! (item 57; the owner: a shell with `tmux` typed in it opened black, and
//! a drag brought the picture back).
//!
//! The host's screen model (item 46) is vt100, and vt100's resize cuts a
//! row without clearing a wide character it splits at the new last
//! column; the next erase or write on that cell panics (`row.rs`
//! `clear_wide`). A tmux client redraws every row after a resize, so a
//! pane with CJK reaching the right edge and a face one column narrower
//! is enough. The model was replaced by a blank one on the normal screen
//! and nothing asked tmux to draw again, so every later face joining at
//! that size was handed a blank alternate screen — until a drag folded
//! the deck, which is a resize, which is tmux's redraw.
//!
//! A real host, a real pty, a private tmux server inside the shell: the
//! face narrows the pty by one column, leaves, and a face joins again at
//! that size and must see what tmux shows.
#![cfg(unix)]
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::host::{ClientOp, write_frame};
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

fn read_for(conn: &mut std::net::TcpStream, d: Duration) -> Vec<u8> {
    conn.set_read_timeout(Some(Duration::from_millis(100))).unwrap();
    let mut got = Vec::new();
    let mut buf = [0u8; 65536];
    let until = Instant::now() + d;
    while Instant::now() < until {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
    }
    got
}

#[test]
fn a_tmux_typed_in_a_shell_is_not_handed_out_black_after_a_narrowing() {
    if Command::new("tmux").arg("-V").stdout(Stdio::null()).stderr(Stdio::null()).status().map_or(true, |s| !s.success()) {
        eprintln!("skipped: no tmux on this machine");
        return;
    }
    let root = std::env::temp_dir().join(format!("khor-shelltmux-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let sock = format!("khor-test-57-{}", std::process::id());
    let id = SessionId("shell/57-0".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "shell", "sh", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), vec!["/bin/sh".into()]))
    };
    let dir = k.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut face = loop {
        if let Ok(a) = khor_node::host::connect_host_versioned(&dir, 80, 24) {
            break a.stream;
        }
        assert!(Instant::now() < deadline, "the host never took an attacher");
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = read_for(&mut face, Duration::from_millis(300));
    // The person types tmux at the shell's prompt.
    let typed = format!("tmux -L {sock} -f /dev/null new -s one /bin/sh\r");
    write_frame(&mut face, &ClientOp::Input(typed.into_bytes())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while tmux(&sock, &["list-clients"]).is_empty() {
        assert!(Instant::now() < deadline, "tmux never started in the shell");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    // CJK to the pane's right edge: 40 wide characters fill 80 columns.
    let wide = "\u{4e2d}".repeat(40);
    let line = format!("clear; for i in 1 2 3 4 5 6; do printf '%s\\n' '{wide}'; done; echo MARK-57");
    tmux(&sock, &["send-keys", "-t", "one", "-l", &line]);
    tmux(&sock, &["send-keys", "-t", "one", "Enter"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !tmux(&sock, &["capture-pane", "-p", "-t", "one"]).lines().any(|l| l == "MARK-57") {
        assert!(Instant::now() < deadline, "the pane never showed the mark");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    let _ = read_for(&mut face, Duration::from_millis(500));
    // The face is one column narrower (another device, a fold): tmux
    // redraws every row at 79 over cells the resize split.
    write_frame(&mut face, &ClientOp::Resize { cols: 79, rows: 24 }).unwrap();
    let _ = read_for(&mut face, Duration::from_millis(2000));
    drop(face);
    std::thread::sleep(Duration::from_millis(300));

    // A face joins again at the size the pty has: no SIGWINCH, so what it
    // is handed is the host's model and nothing else.
    let mut again = khor_node::host::connect_host_versioned(&dir, 79, 24).expect("the face comes back").stream;
    let got = read_for(&mut again, Duration::from_millis(1500));
    let shown = tmux(&sock, &["capture-pane", "-p", "-t", "one"]);
    tmux(&sock, &["kill-server"]);
    // The shell ends, and the host with it.
    let _ = read_for(&mut again, Duration::from_millis(300));
    write_frame(&mut again, &ClientOp::Input(b"exit\r".to_vec())).unwrap();
    let _ = read_for(&mut again, Duration::from_millis(300));
    drop(again);
    let _ = host.join();
    let _ = std::fs::remove_dir_all(&root);

    let mut face = vt100::Parser::new(24, 79, 0);
    face.process(&got);
    let screen = face.screen();
    let rows: Vec<String> = screen.rows(0, 79).map(|r| r.trim_end().to_owned()).collect();
    eprintln!("the face was handed {} bytes:\n{}", got.len(), rows.join("\n"));
    assert!(screen.alternate_screen(), "the face is on tmux's alternate screen");
    let want: Vec<&str> = shown.lines().map(str::trim_end).collect();
    assert!(want.iter().any(|l| l.contains('\u{4e2d}')), "control: tmux shows the wide rows: {shown}");
    for (i, w) in want.iter().enumerate() {
        assert_eq!(rows[i], *w, "row {i}: the face shows what tmux shows, not a blank");
    }
}
