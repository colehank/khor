//! A late attacher's replay keeps the modes its trimmed head set.
//!
//! The host's ring holds the last [`RING`](khor_node::host) bytes, and a
//! terminal mode is set once: a tmux client sends `ESC [ ? 1049 h` when
//! it attaches and then draws for hours. Before 2026-10-01 a replay was
//! the bare tail, so once the ring had trimmed past that first line
//! every later attacher drew a full-screen program into the normal
//! buffer — the owner's「初次进入一个 tmux 会话的标签，滚鼠标滚轮页面没有
//! 变化；把窗口缩放一下就能滚了」(`khor_node::modes` has the measurement).
//!
//! A real host, a real PTY: a stand-in sets four modes, writes far past
//! the ring, and waits; an attacher reads the replay.

use std::io::Read;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::live::LiveKind;

/// ~450 KiB of lines after the modes — the ring is 256 KiB.
const PROGRAM: &str = r#"#!/bin/sh
printf '\033[?1049h\033[?1h\033=\033[?2004h'
i=0
while [ $i -lt 4000 ]; do
  printf 'flood-%06d 0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\n' $i
  i=$((i+1))
done
printf 'DONE-MARK\n'
while [ ! -f "$0.stop" ]; do sleep 0.1; done
"#;

#[test]
fn a_replay_trimmed_past_the_modes_still_starts_in_them() {
    let root = std::env::temp_dir().join(format!("khor-hostreplay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let program = root.join("flood");
    std::fs::write(&program, PROGRAM).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let id = SessionId("tui/hostreplay".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "tui", "flood", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        let cmd = vec![program.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let dir = k.dir_of(&id).expect("registered");

    // The program is done writing once its last line is the preview.
    let deadline = Instant::now() + Duration::from_secs(30);
    while std::fs::read_to_string(dir.join("last.txt")).unwrap_or_default() != "DONE-MARK" {
        assert!(
            Instant::now() < deadline,
            "the stand-in never finished writing"
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    let mut conn = khor_node::host::connect(&dir, 80, 24).expect("the host takes an attacher");
    conn.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut replay = Vec::new();
    let mut buf = [0u8; 65536];
    while !String::from_utf8_lossy(&replay).contains("DONE-MARK") {
        let n = conn.read(&mut buf).expect("the replay arrives");
        assert!(n > 0, "the host hung up mid-replay");
        replay.extend_from_slice(&buf[..n]);
    }
    std::fs::write(root.join("flood.stop"), "").unwrap();
    let _ = host.join();

    let text = String::from_utf8_lossy(&replay);
    assert!(
        !text.contains("flood-000000 "),
        "the precondition: the ring really trimmed past the program's first line ({} bytes replayed)",
        replay.len()
    );
    let first_line = text.find("flood-").expect("the flood is in the replay");
    let before = &text[..first_line];
    for (mode, what) in [
        ("\x1b[?1049h", "the alternate screen"),
        ("\x1b[?1h", "application cursor keys"),
        ("\x1b[?2004h", "bracketed paste"),
        ("\x1b=", "the application keypad"),
    ] {
        assert!(
            before.contains(mode),
            "{what} ({mode:?}) is not set before the replayed lines — the attacher starts in the wrong mode; the replay began {:?}",
            &text[..text.len().min(80)]
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}
