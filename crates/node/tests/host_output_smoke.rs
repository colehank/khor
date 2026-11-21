//! The host reads what its program writes, on a Windows parent with no console.
//!
//! `portable-pty` used to create the pseudoconsole with
//! `PSUEDOCONSOLE_INHERIT_CURSOR`; on a console-less parent that flag makes
//! conhost deliver nothing, so this asserts the line the terminal panel and the
//! replay ring are built on. See `vendor/portable-pty/KHOR.md`.
#![cfg(windows)]

use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::live::LiveKind;

#[test]
fn the_host_reads_its_programs_output() {
    let root = std::env::temp_dir().join(format!("khor-hostout-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let id = SessionId("tui/hostout".into());
    let kind = LiveKind::new(root.clone(), DeviceId([2; 32]));
    kind.register(&id, "tui", "cmd", None, None).unwrap();
    let dir = kind.dir_of(&id).expect("registered");
    let host = {
        let root = root.clone();
        let id = id.clone();
        std::thread::spawn(move || {
            khor_node::host::host_main(
                root,
                id,
                (80, 24),
                vec![
                    "cmd.exe".to_owned(),
                    "/d".to_owned(),
                    "/c".to_owned(),
                    "echo hello-host".to_owned(),
                ],
            )
        })
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut seen = String::new();
    while Instant::now() < deadline {
        seen = std::fs::read_to_string(dir.join("last.txt")).unwrap_or_default();
        if seen.contains("hello-host") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        seen.contains("hello-host"),
        "the host never saw its program's line; last.txt = {seen:?}"
    );
    let _ = std::fs::write(root.join("stop"), "");
    let _ = host.join();
}
