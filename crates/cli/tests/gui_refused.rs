//! **A dropped op says so, to the face that sent it** (#88).
//!
//! Two places in `gui_host` used to drop a face's op in silence: a
//! `Say` while a turn is already running, and a `Stop` with no turn to
//! stop. One face cannot reach either — `ChatView.say()` blocks on its
//! own `inTurn`, set synchronously — so this opens **two**, which is
//! not a contrived arrangement: two windows on one conversation is the
//! case `GuiNote::Ask` is already re-sent for.
//!
//! What the silence cost, in order: B's line is dropped while A holds
//! the turn, so no `Turn` frame ever answers B; B's stop then finds no
//! turn and is dropped as well; B's box never comes back. Each step is
//! invisible on its own, and together they read as the session having
//! died.
//!
//! **This is not the smoke flake.** The backend chain went 250 rounds
//! without dropping a frame (`gui_stop.rs`), and neither site was ever
//! shown to be reachable from a single face — which is the whole reason
//! this test needs two. It is fixed because a silent drop is wrong on
//! its own terms, not because it explains anything.
//!
//! Its own binary, `cagent.rs`'s reason: the fake home, `KHOR_CLAUDE`
//! and the working directory are process-wide, and a second test in one
//! binary races them. Tried it the other way first and the second test
//! found the first one's marker files.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::gui_host::{gui_host_main, refused, GuiNote, GuiOp};
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
                break
        emit({"type": "result", "subtype": "interrupted", "session_id": sid})
        continue
    emit({"type": "assistant", "message": {"content": [{"type": "text", "text": "echo: " + text}]}})
    emit({"type": "result", "subtype": "success", "session_id": sid})
"#;

fn kind_of(n: &GuiNote) -> &'static str {
    match n {
        GuiNote::Note(_) => "Note",
        GuiNote::Ask { .. } => "Ask",
        GuiNote::Turn(_) => "Turn",
        GuiNote::Gone => "Gone",
        GuiNote::History(_) => "History",
        GuiNote::HistoryEnd => "HistoryEnd",
        GuiNote::Turning => "Turning",
        GuiNote::Answered { .. } => "Answered",
        GuiNote::Agent { .. } => "Agent",
        GuiNote::Refused { .. } => "Refused",
    }
}

#[test]
fn a_dropped_op_is_said_back_to_the_face_that_sent_it() {
    let dir = std::env::temp_dir().join(format!("khor-refused-{}", std::process::id()));
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
    unsafe {
        std::env::set_var("KHOR_HOME", dir.join("home"));
        std::env::set_var("KHOR_CLAUDE", &fake);
    }
    // The ghost inherits this, and the fake writes its marker relative
    // to it — so the marker lands where this test looks for it.
    std::env::set_current_dir(&cwd).unwrap();

    let root = dir.join("home");
    std::fs::create_dir_all(&root).unwrap();
    let ready = dir.join("ready");
    let exe = env!("CARGO_BIN_EXE_khor");
    let host = {
        let (root, ready) = (root.clone(), ready.clone());
        let cmd = vec![exe.to_owned(), "_cagent".to_owned()];
        std::thread::spawn(move || gui_host_main(root, ready, "refused-probe".into(), cmd))
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

    let attach = || -> TcpStream {
        let mut c = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
        // A read deadline before the first read: a frame that stops
        // arriving must make this fail, not hang.
        c.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        write_frame(&mut c, &Hello { cookie: hf.cookie.clone(), cols: 0, rows: 0 }).unwrap();
        let w: Welcome = read_frame(&mut c).unwrap();
        assert!(w.ok, "{}", w.why);
        let GuiNote::Agent { .. } = read_frame::<GuiNote>(&mut c).unwrap() else {
            panic!("the facts come first")
        };
        c
    };
    let mut a = attach();
    let mut b = attach();

    // **Stop with nothing running** — the simpler of the two, and it
    // needs no timing at all.
    write_frame(&mut b, &GuiOp::Stop).unwrap();
    match read_frame::<GuiNote>(&mut b).expect("a frame") {
        GuiNote::Refused { why } => assert_eq!(why, refused::NO_TURN),
        other => panic!("a stop with no turn must say so, got {}", kind_of(&other)),
    }

    // A takes the turn and holds it. Waited for by the fake's own
    // marker rather than by a clock: a `Say` sent before the prompt
    // reached claude is a different experiment.
    write_frame(&mut a, &GuiOp::Say("hang here".into())).unwrap();
    let started = Instant::now() + Duration::from_secs(20);
    while !cwd.join("fake.hanging").exists() {
        assert!(Instant::now() < started, "the fake never got the prompt");
        std::thread::sleep(Duration::from_millis(10));
    }

    // B speaks into a session that is busy. **At once** — not after the
    // turn ends, which is when it used to find out by inference.
    write_frame(&mut b, &GuiOp::Say("mine as well".into())).unwrap();
    let told = loop {
        match read_frame::<GuiNote>(&mut b).expect("a frame within twenty seconds") {
            GuiNote::Refused { why } => break why,
            _ => continue,
        }
    };
    assert_eq!(told, refused::TURN_IN_FLIGHT, "B is told why its line did not go in");

    // And it went to B alone: A is holding a turn and has no business
    // hearing about B's mistake.
    //
    // **Asserted as "no `Refused` among them", not as "nothing at
    // all".** A is on the same conversation and legitimately receives
    // what is broadcast — the first turn carries the agent's command
    // list, and the first spelling of this assertion read that as the
    // refusal leaking and failed. Silence would have been the wrong
    // claim: it says this face is deaf, and it is not.
    a.set_read_timeout(Some(Duration::from_millis(400))).unwrap();
    let mut heard = Vec::new();
    while let Ok(note) = read_frame::<GuiNote>(&mut a) {
        heard.push(kind_of(&note));
    }
    assert!(
        !heard.contains(&"Refused"),
        "a refusal is for the face that earned it, never a broadcast: {heard:?}"
    );

    // The turn still ends normally afterwards — a refusal must not have
    // been half a cancel.
    a.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    write_frame(&mut a, &GuiOp::Stop).unwrap();
    loop {
        match read_frame::<GuiNote>(&mut a).expect("a frame") {
            GuiNote::Turn(_) => break,
            _ => continue,
        }
    }
    write_frame(&mut a, &GuiOp::Close).unwrap();
    let _ = host.join();
    let _ = std::fs::remove_dir_all(&dir);
}
