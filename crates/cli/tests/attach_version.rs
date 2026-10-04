//! The host protocol's version handshake, through the real binary
//! (audit/term-attach.md). A client from before the handshake still
//! attaches, types and reads. The other direction — a versioned
//! client against a host from before the handshake — is in
//! `khor_node::host`'s handshake tests, against a host that answers the
//! old way. Every wait has a deadline.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::host::{self, ClientOp, HOST_VERSION, HostFile};

fn khor(home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home).env("KHOR_NAME", "box").env_remove("KHOR_SESSION");
    c
}

/// Kills a pid when it goes out of scope, however it goes.
struct Reaper(u32);

impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = Command::new("kill").args(["-9", &self.0.to_string()]).stderr(std::process::Stdio::null()).status();
    }
}

fn open_detached(home: &Path) -> (String, PathBuf, Reaper) {
    let out = khor(home).args(["open", "-d", "--", "sh"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let sid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let dir = home.join(".khor").join("sessions").join(sid.replace('/', "-"));
    let hf: HostFile = serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();
    (sid, dir, Reaper(hf.host_pid))
}

/// Reads until `needle` shows up, or panics; answers all that was read.
fn read_until(stream: &mut std::net::TcpStream, needle: &str, secs: u64) -> String {
    stream.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
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
    panic!("never saw {needle:?} in: {}", String::from_utf8_lossy(&seen));
}

#[test]
fn a_client_from_before_the_handshake_still_attaches_types_and_reads() {
    let home = std::env::temp_dir().join(format!("khor-attachv-old-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let (sid, dir, _reap) = open_detached(&home);
    let hf: HostFile = serde_json::from_str(&std::fs::read_to_string(dir.join("host.json")).unwrap()).unwrap();

    // The old client's whole conversation: Hello, Welcome, then only
    // Input and Resize, never a Version.
    let mut s = std::net::TcpStream::connect(("127.0.0.1", hf.port)).unwrap();
    host::write_frame(&mut s, &host::Hello { cookie: hf.cookie.clone(), cols: 80, rows: 24 }).unwrap();
    let w: host::Welcome = host::read_frame(&mut s).unwrap();
    assert!(w.ok, "{}", w.why);
    assert_eq!(w.why, format!("khor-host/{HOST_VERSION}"), "the version is said where an old client does not look");
    host::write_frame(&mut s, &ClientOp::Resize { cols: 100, rows: 30 }).unwrap();
    host::write_frame(&mut s, &ClientOp::Input(b"echo old-client-ok\n".to_vec())).unwrap();
    read_until(&mut s, "old-client-ok", 10);

    assert!(khor(&home).args(["close", &sid]).status().unwrap().success());
    let _ = std::fs::remove_dir_all(&home);
}

