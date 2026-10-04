//! **A shell never gets the mouse reports a program left on** (item 25;
//! the owner's zsh prompt filled with `35;6;5M35;12;6M…` after he came back
//! to a session).
//!
//! A real host and a real PTY: a stand-in shell turns SGR any-motion mouse
//! tracking on (`?1003h ?1006h`) and never turns it off, then reads lines
//! the way a shell does. The host must write the modes off into the stream
//! once the shell has the terminal again, so a replay ends with them off;
//! and a face still sending reports must not have them typed into the
//! shell.

use std::io::Read;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::host::{ClientOp, write_frame};
use khor_node::live::LiveKind;

const SHELL: &str = r#"#!/bin/sh
printf '\033[?1003h\033[?1006h'
echo SET-MARK
while IFS= read -r line; do echo "GOT:$line"; done
"#;

fn read_until(conn: &mut impl Read, want: &str, into: &mut Vec<u8>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut buf = [0u8; 65536];
    while !String::from_utf8_lossy(into).contains(want) {
        assert!(Instant::now() < deadline, "never saw {want:?} in {:?}", String::from_utf8_lossy(into));
        match conn.read(&mut buf) {
            Ok(0) => panic!("the host hung up"),
            Ok(n) => into.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
    }
}

#[test]
fn a_mouse_mode_left_on_is_turned_off_and_its_reports_never_reach_the_shell() {
    let root = std::env::temp_dir().join(format!("khor-mousereports-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let shell = root.join("stand-in-shell");
    std::fs::write(&shell, SHELL).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let id = SessionId("shell/mousereports".into());
    let k = LiveKind::new(root.clone(), DeviceId([3; 32]));
    k.register(&id, "shell", "mouse", None, None).unwrap();
    let host = {
        let (root, id) = (root.clone(), id.clone());
        let cmd = vec![shell.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let dir = k.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(20);
    while std::fs::read_to_string(dir.join("last.txt")).unwrap_or_default() != "SET-MARK" {
        assert!(Instant::now() < deadline, "the stand-in never started");
        std::thread::sleep(Duration::from_millis(50));
    }

    // The replay: the program's `h`, and after it the host's `l`.
    let mut conn = khor_node::host::connect(&dir, 80, 24).expect("the host takes an attacher");
    conn.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let mut seen = Vec::new();
    read_until(&mut conn, "SET-MARK", &mut seen);
    let text = String::from_utf8_lossy(&seen).into_owned();
    let on = text.find("\x1b[?1003h").expect("the program's mode in the replay");
    let off = text.find("\x1b[?1003l").expect("the host turned the mode off in the stream");
    assert!(off > on, "the `l` comes after the `h`: {text:?}");

    // A stale face still sends the owner's reports, then a line.
    let mut writer = conn.try_clone().unwrap();
    write_frame(&mut writer, &ClientOp::Input(b"\x1b[<35;6;5M\x1b[<35;12;6Mhello\r".to_vec())).unwrap();
    read_until(&mut conn, "GOT:", &mut seen);
    std::thread::sleep(Duration::from_millis(300));
    let mut rest = [0u8; 4096];
    if let Ok(n) = conn.read(&mut rest) {
        seen.extend_from_slice(&rest[..n]);
    }
    let text = String::from_utf8_lossy(&seen).into_owned();
    let got = &text[text.rfind("GOT:").unwrap()..];
    assert!(got.starts_with("GOT:hello"), "the shell read reports as typing: {got:?}");

    drop(conn);
    drop(writer);
    let _ = std::process::Command::new("pkill").args(["-f", &shell.to_string_lossy()]).status();
    let _ = host.join();
    let _ = std::fs::remove_dir_all(&root);
}
