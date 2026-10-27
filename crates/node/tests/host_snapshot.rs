//! Current terminal attachers receive the host model's bounded snapshot,
//! including styled history and modes. A real host and PTY produce more
//! than the ring's budget before a client attaches.
use std::io::Read;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::live::LiveKind;

/// ~1.15 MiB of lines after the modes — past the ring (`host::RING`, 1 MiB).
const PROGRAM: &str = r#"#!/bin/sh
printf '\033[?1h\033=\033[?2004h'
i=0
while [ $i -lt 10000 ]; do
  printf '\033[3%dmflood-%06d\033[0m 0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\n' $((i % 7 + 1)) $i
  i=$((i+1))
done
printf 'DONE-MARK\n'
while [ ! -f "$0.stop" ]; do sleep 0.1; done
"#;

#[test]
fn a_current_attacher_gets_the_snapshot_not_the_ring() {
    let root = std::env::temp_dir().join(format!("khor-hostsnap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let program = root.join("flood");
    std::fs::write(&program, PROGRAM).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let id = SessionId("shell/hostsnap".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "shell", "flood", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        let cmd = vec![program.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let dir = k.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(30);
    while std::fs::read_to_string(dir.join("last.txt")).unwrap_or_default() != "DONE-MARK" {
        assert!(
            Instant::now() < deadline,
            "the stand-in never finished writing"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(300));

    let started = Instant::now();
    let mut conn =
        khor_node::host::connect(&dir, 80, 24).expect("the host takes a current attacher");
    conn.set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let mut got = Vec::new();
    let mut buf = [0u8; 65536];
    let until = Instant::now() + Duration::from_secs(2);
    let mut first_byte_at = None;
    while Instant::now() < until {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                first_byte_at.get_or_insert_with(|| started.elapsed());
                got.extend_from_slice(&buf[..n]);
            }
            Err(_) => {}
        }
    }
    std::fs::write(root.join("flood.stop"), "").unwrap();
    let _ = host.join();

    let text = String::from_utf8_lossy(&got);
    eprintln!(
        "the attacher was handed {} bytes, the first after {:?}",
        got.len(),
        first_byte_at
    );
    assert!(
        got.len() < 400 * 1024,
        "the snapshot is bounded by the model's scrollback, not the ring: {} bytes",
        got.len()
    );
    assert!(
        text.contains("DONE-MARK"),
        "the screen's last line is there"
    );
    assert!(
        text.contains("flood-009999"),
        "the newest history line is there"
    );
    assert!(
        !text.contains("flood-005000"),
        "a line 5 000 rows up is past the model's history"
    );
    let done = text.find("DONE-MARK").unwrap();
    for (mode, what) in [
        ("\x1b[?1h", "application cursor keys"),
        ("\x1b[?2004h", "bracketed paste"),
        ("\x1b=", "the application keypad"),
    ] {
        assert!(
            text.contains(mode),
            "{what} ({mode:?}) is set for the attacher"
        );
    }
    assert!(
        text[..done].contains("\x1b[32m") || text[..done].contains("\x1b[0;32m"),
        "the history keeps its colours"
    );
    assert!(
        first_byte_at.unwrap() < Duration::from_millis(250),
        "a current attacher is not made to wait out the first-frame window: {:?}",
        first_byte_at
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Waits for `$0.go`, then 8 000 short lines (one row each) and a mark.
const LATER: &str = r#"#!/bin/sh
while [ ! -f "$0.go" ]; do sleep 0.05; done
i=0
while [ $i -lt 8000 ]; do
  printf 'flood-%06d\n' $i
  i=$((i+1))
done
printf 'DONE-MARK\n'
while [ ! -f "$0.stop" ]; do sleep 0.1; done
"#;

/// **The re-attach keeps the history the face asked for** (item 56): a
/// face that says 5 000 rows on attach (`ClientOp::Scrollback`) finds
/// 5 000 rows of history when it comes back, not the host's
/// 2 000 (the cap is `host::last_line_tests`').
#[test]
fn a_re_attach_keeps_the_history_the_face_asked_for() {
    use khor_node::host::{ClientOp, write_frame};
    let root = std::env::temp_dir().join(format!("khor-hostkeep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let program = root.join("later");
    std::fs::write(&program, LATER).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let id = SessionId("shell/hostkeep".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "shell", "later", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        let cmd = vec![program.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let dir = k.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(10);
    let first = loop {
        if let Ok(a) = khor_node::host::connect(&dir, 80, 24) {
            break a;
        }
        assert!(Instant::now() < deadline, "the host never took an attacher");
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut conn = first;
    write_frame(&mut conn, &ClientOp::Scrollback(5000)).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    drop(conn);
    std::fs::write(root.join("later.go"), "").unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while std::fs::read_to_string(dir.join("last.txt")).unwrap_or_default() != "DONE-MARK" {
        assert!(
            Instant::now() < deadline,
            "the stand-in never finished writing"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(300));

    let again = khor_node::host::connect(&dir, 80, 24).expect("the face comes back");
    let mut conn = again;
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
    std::fs::write(root.join("later.stop"), "").unwrap();
    let _ = host.join();
    let text = String::from_utf8_lossy(&got);
    let rows = text.matches("flood-").count();
    eprintln!(
        "the face came back to {rows} rows of flood ({} bytes)",
        got.len()
    );
    assert!(
        text.contains("DONE-MARK") && text.contains("flood-007999"),
        "the newest rows are there"
    );
    assert!(
        text.contains("flood-003100"),
        "a row 4 900 up is kept: the face asked for 5 000, not the host's 2 000"
    );
    assert!(
        !text.contains("flood-002900"),
        "a row 5 100 up is past what the face asked for"
    );
    let _ = std::fs::remove_dir_all(&root);
}
