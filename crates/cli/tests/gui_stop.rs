//! **The stop button, over and over.** A turn that will not end on its
//! own, stopped through the very op a face sends, twenty times in a row.
//!
//! Written to chase a flake the smoke run sees (`the turn ended and the
//! box back`, ~2 in 7): after a stop, the `Turn` frame sometimes never
//! comes back and the box stays gone. The shim alone does not flake —
//! `cagent.rs` drives the same cancel through the same fake and it went
//! 30 for 30 — so this drives the layer above it, the GUI host, which
//! is where a face's stop actually lands.
//!
//! # What a miss here can tell apart
//!
//! The fake writes `fake.interrupt` the moment the control protocol's
//! interrupt reaches it, and `fake.hanging` when it enters the branch
//! that waits for one. A missing `Turn` frame therefore splits three
//! ways, and they have different owners:
//!
//! - **no `fake.hanging`** — the fake never got the prompt, so the test
//!   stopped a turn that was not running: test debris.
//! - **hanging, no `fake.interrupt`** — the stop never left khor. A real
//!   person's stop button would go mute the same way: a product bug in
//!   the host or the shim's cancel path.
//! - **interrupt seen, no frame** — claude was interrupted and the
//!   answer was lost coming back. Also a product bug, one layer on.
//!
//! Written as one test with a loop rather than twenty tests: the fake
//! home and `KHOR_CLAUDE` are process env, and a second test in this
//! binary would race them (`cagent.rs`'s reason).

use std::net::TcpStream;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::gui_host::{gui_host_main, GuiNote, GuiOp};
use khor_node::host::{read_frame, read_host_file, write_frame, Hello, Welcome};
use khor_node::live::LiveKind;

const FAKE: &str = r#"#!/usr/bin/env python3
import json, sys, os
args = sys.argv[1:]
sid = args[args.index("--session-id") + 1] if "--session-id" in args else args[args.index("--resume") + 1]
def emit(o):
    sys.stdout.write(json.dumps(o) + "\n"); sys.stdout.flush()
first = True
for line in sys.stdin:
    m = json.loads(line)
    if m.get("type") != "user":
        continue
    text = m["message"]["content"][0]["text"]
    if first:
        first = False
        emit({"type": "system", "subtype": "init", "session_id": sid, "slash_commands": ["compact"]})
    if "hang" in text:
        open("fake.hanging", "w").write(text)
        while True:
            line = sys.stdin.readline()
            if not line:
                break
            c = json.loads(line)
            if c.get("type") == "control_request" and c.get("request", {}).get("subtype") == "interrupt":
                open("fake.interrupt", "w").write(text)
                break
        emit({"type": "result", "subtype": "interrupted", "session_id": sid})
        continue
    emit({"type": "assistant", "message": {"content": [{"type": "text", "text": "echo: " + text}]}})
    emit({"type": "result", "subtype": "success", "session_id": sid})
"#;

const ROUNDS: usize = 20;

#[test]
fn a_stop_always_brings_the_turn_to_an_end() {
    let dir = std::env::temp_dir().join(format!("khor-guistop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cwd = dir.join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let fake = dir.join("claude.py");
    std::fs::write(&fake, FAKE).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    // Process env, in a single-test binary on purpose (module head).
    unsafe {
        std::env::set_var("KHOR_HOME", dir.join("home"));
        std::env::set_var("KHOR_CLAUDE", &fake);
    }
    // The ghost inherits this, and the fake writes its markers relative
    // to it — so the markers land where this test looks for them.
    std::env::set_current_dir(&cwd).unwrap();

    let root = dir.join("home");
    std::fs::create_dir_all(&root).unwrap();
    let ready = dir.join("ready");
    let exe = env!("CARGO_BIN_EXE_khor");
    let host = {
        let (root, ready) = (root.clone(), ready.clone());
        let cmd = vec![exe.to_owned(), "_cagent".to_owned()];
        std::thread::spawn(move || gui_host_main(root, ready, "stop-probe".into(), cmd))
    };

    let deadline = Instant::now() + Duration::from_secs(30);
    let id = loop {
        if let Ok(text) = std::fs::read_to_string(&ready) {
            break SessionId(text.trim().to_owned());
        }
        assert!(Instant::now() < deadline, "the host never wrote the ready file");
        std::thread::sleep(Duration::from_millis(50));
    };

    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&k.dir_of(&id).unwrap()).expect("a host file");
    let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
    // A deadline before the first read: a frame that stops arriving has
    // to make this **fail**, not hang — a hang reads as a slow test.
    conn.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    write_frame(&mut conn, &Hello { cookie: hf.cookie, cols: 0, rows: 0 }).unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);
    let GuiNote::Agent { .. } = read_frame::<GuiNote>(&mut conn).unwrap() else {
        panic!("the facts come first")
    };

    let mut misses = Vec::new();
    for round in 0..ROUNDS {
        let _ = std::fs::remove_file(cwd.join("fake.hanging"));
        let _ = std::fs::remove_file(cwd.join("fake.interrupt"));
        write_frame(&mut conn, &GuiOp::Say(format!("hang {round}"))).unwrap();

        // **Wait for the turn to be the thing that is running**, rather
        // than for a clock: the fake announces itself by writing, and a
        // stop sent before the prompt reached claude is a different
        // experiment from the one this is running.
        let started = Instant::now() + Duration::from_secs(10);
        while !cwd.join("fake.hanging").exists() {
            assert!(Instant::now() < started, "round {round}: the fake never got the prompt");
            std::thread::sleep(Duration::from_millis(10));
        }

        write_frame(&mut conn, &GuiOp::Stop).unwrap();
        let mut ended = None;
        loop {
            match read_frame::<GuiNote>(&mut conn) {
                Ok(GuiNote::Turn(stop)) => {
                    ended = Some(stop);
                    break;
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        match ended {
            Some(stop) => assert_eq!(
                stop, "Cancelled",
                "round {round}: a stopped turn is 取消, not a refusal"
            ),
            None => misses.push(format!(
                "round {round}: no Turn frame — hanging={} interrupt={}",
                cwd.join("fake.hanging").exists(),
                cwd.join("fake.interrupt").exists()
            )),
        }
    }

    write_frame(&mut conn, &GuiOp::Close).unwrap();
    let _ = host.join();
    assert!(misses.is_empty(), "{} of {ROUNDS} stops never ended:\n{}", misses.len(), misses.join("\n"));
    let _ = std::fs::remove_dir_all(&dir);
}
