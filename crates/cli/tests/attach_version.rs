//! Current terminal protocol rejection and host-kept clear, through the real
//! binary and PTY. Rejected clients receive no replay and cannot type; a
//! subsequent current client can still use that same host. All waits are bounded.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::host::{self, ClientOp, HostFile};

fn khor(home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION");
    c
}

/// Kills a pid when it goes out of scope, however it goes.
struct Reaper(u32);

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-9", &self.0.to_string()])
            .stderr(std::process::Stdio::null())
            .status();
    }
}

fn open_detached(home: &Path) -> (String, PathBuf, Reaper) {
    let out = khor(home)
        .args(["open", "-d", "--", "sh"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let sid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let dir = home
        .join(".khor")
        .join("sessions")
        .join(sid.replace('/', "-"));
    let hf: HostFile =
        serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    (sid, dir, Reaper(hf.host_pid))
}

/// Reads until `needle` shows up, or panics; answers all that was read.
fn read_until(stream: &mut std::net::TcpStream, needle: &str, secs: u64) -> String {
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut seen = Vec::new();
    let mut buf = [0u8; 4096];
    while Instant::now() < deadline {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => seen.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
        if String::from_utf8_lossy(&seen).contains(needle) {
            return String::from_utf8_lossy(&seen).into_owned();
        }
    }
    panic!(
        "never saw {needle:?} in: {}",
        String::from_utf8_lossy(&seen)
    );
}

#[test]
fn the_real_host_rejects_missing_empty_old_future_and_wrong_first_ops() {
    let home = std::env::temp_dir().join(format!("khor-attach-current-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let (sid, dir, _reap) = open_detached(&home);
    let hf = host::read_host_file(&dir).unwrap();
    let mut current = host::connect(&dir, 100, 30).unwrap();
    host::write_frame(&mut current, &ClientOp::Ready).unwrap();
    for case in 0..12 {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", hf.port)).unwrap();
        stream
            .set_read_timeout(Some(host::HANDSHAKE + Duration::from_secs(2)))
            .unwrap();
        host::write_frame(
            &mut stream,
            &host::Hello {
                cookie: hf.cookie.clone(),
                cols: 40,
                rows: 10,
            },
        )
        .unwrap();
        let welcome: host::Welcome = host::read_frame(&mut stream).unwrap();
        assert!(welcome.ok);
        assert_eq!(welcome.why, "khor-host/5");
        let op = match case {
            0 => None,
            1 => {
                stream.write_all(&[0, 0, 0, 0]).unwrap();
                None
            }
            2 => Some(ClientOp::Version(0)),
            3 => Some(ClientOp::Version(1)),
            4 => Some(ClientOp::Version(4)),
            5 => Some(ClientOp::Version(6)),
            6 => Some(ClientOp::Input(b"echo must-not-run\n".to_vec())),
            7 => Some(ClientOp::Resize { cols: 40, rows: 10 }),
            8 => Some(ClientOp::Forget),
            9 => Some(ClientOp::Scrollback(5000)),
            10 => Some(ClientOp::Ready),
            _ => {
                stream.write_all(&((1u32 << 20) + 1).to_be_bytes()).unwrap();
                None
            }
        };
        if let Some(op) = op {
            host::write_frame(&mut stream, &op).unwrap();
        }
        match stream.read(&mut [0u8; 1]) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) => {}
            other => panic!("first-op case {case} received replay or stayed open: {other:?}"),
        }
    }
    // Rejection closes the attachment, not the owned host or shell.
    host::write_frame(
        &mut current,
        &ClientOp::Input(b"stty size; printf 'current-client-%s\\n' ok\n".to_vec()),
    )
    .unwrap();
    let output = read_until(&mut current, "current-client-ok\r\n", 10);
    assert!(
        output.contains("30 100"),
        "a refused Hello changed the PTY size: {output}"
    );
    assert!(
        !output.contains("must-not-run"),
        "rejected input reached the PTY: {output}"
    );
    assert!(
        khor(&home)
            .args(["close", &sid])
            .status()
            .unwrap()
            .success()
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_current_client_clears_the_host_history_for_later_attachers() {
    let home = std::env::temp_dir().join(format!("khor-attachv-new-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let (sid, dir, _reap) = open_detached(&home);

    let mut s = host::connect(&dir, 100, 30).unwrap();
    host::write_frame(&mut s, &ClientOp::Ready).unwrap();
    host::write_frame(
        &mut s,
        &ClientOp::Input(b"printf 'before-the-%s\\n' clear\n".to_vec()),
    )
    .unwrap();
    read_until(&mut s, "before-the-clear\r\n", 10);

    // Control: a late attacher is handed it while nothing was cleared.
    let mut late = host::connect(&dir, 100, 30).unwrap();
    read_until(&mut late, "before-the-clear", 5);
    drop(late);

    host::write_frame(&mut s, &ClientOp::Forget).unwrap();
    host::write_frame(
        &mut s,
        &ClientOp::Input(b"printf 'after-the-%s\\n' clear\n".to_vec()),
    )
    .unwrap();
    read_until(&mut s, "after-the-clear\r\n", 10);

    let mut later = host::connect(&dir, 100, 30).unwrap();
    let replay = read_until(&mut later, "after-the-clear\r\n", 5);
    assert!(
        !replay.contains("before-the-clear"),
        "the cleared history came back: {replay:?}"
    );

    assert!(
        khor(&home)
            .args(["close", &sid])
            .status()
            .unwrap()
            .success()
    );
    let _ = std::fs::remove_dir_all(&home);
}
