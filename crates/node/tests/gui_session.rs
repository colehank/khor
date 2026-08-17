//! The GUI host end to end against the scripted ACP stub: one row under
//! the vendor's own id, six words driven by the protocol, ops and notes
//! over the host socket. The stub is khor-acp's (`fake_sshd`'s ACP
//! shape); the host runs in-process like `screen_face.rs`'s does — which
//! also means **nothing here may call `close_session`**: the host file
//! names this very test process, and the kill would land on the whole
//! test run. Closing goes through [`GuiOp::Close`], the op the product
//! wants anyway.

use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId, State};
use khor_node::gui_host::{gui_host_main, GuiNote, GuiOp};
use khor_node::host::{read_frame, read_host_file, write_frame, Hello, Welcome};
use khor_node::live::{LiveKind, Source};

/// The stub is another crate's binary, which `CARGO_BIN_EXE_` cannot
/// name across crates: it is built here, once, through the same cargo
/// that is running the tests, and found beside this test's own binary.
/// The build is a no-op when `cargo test --workspace` already made it.
fn stub_path() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = std::process::Command::new(cargo)
        .args(["build", "-q", "-p", "khor-acp", "--bin", "acp-stub"])
        .status()
        .expect("cargo runs");
    assert!(status.success(), "the stub builds");
    let me = std::env::current_exe().expect("a test binary");
    me.parent().and_then(|d| d.parent()).expect("target/debug").join("acp-stub")
}

fn read_state(root: &PathBuf, id: &SessionId) -> Option<(State, Option<Source>)> {
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let dir = k.dir_of(id)?;
    let raw = std::fs::read(dir.join("state.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    let word: State = serde_json::from_value(v.get("word")?.clone()).ok()?;
    let source = v.get("source").and_then(|s| serde_json::from_value(s.clone()).ok());
    Some((word, source))
}

fn wait_for(root: &PathBuf, id: &SessionId, want: State) -> Option<(State, Option<Source>)> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last = None;
    while Instant::now() < deadline {
        last = read_state(root, id);
        if let Some((word, _)) = &last {
            if *word == want {
                return last;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    last
}

fn next_note(conn: &mut TcpStream) -> GuiNote {
    conn.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
    read_frame::<GuiNote>(conn).expect("a frame within fifteen seconds")
}

#[test]
fn a_gui_session_is_one_row_wearing_the_protocols_words() {
    let root = std::env::temp_dir().join(format!("khor-guihost-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");

    let host = {
        let root = root.clone();
        let ready = ready.clone();
        let cmd = vec![stub_path().to_string_lossy().to_string()];
        std::thread::spawn(move || gui_host_main(root, ready, "stub".into(), cmd))
    };

    // The id arrives through the ready file, and it is the vendor's own
    // uuid through `id_for` — the whole merge story in one assertion:
    // the disk sweep and the hook path mint exactly this spelling.
    let deadline = Instant::now() + Duration::from_secs(20);
    let id = loop {
        if let Ok(text) = std::fs::read_to_string(&ready) {
            break SessionId(text.trim().to_owned());
        }
        assert!(Instant::now() < deadline, "the host never wrote the ready file");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(id.0, "tui/stub-session-1", "the row lives at the vendor-session agreement");

    // Behaviour follows Meta::kind, and that says gui.
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let meta =
        std::fs::read_to_string(k.dir_of(&id).unwrap().join("meta.json")).expect("a registered row");
    assert!(meta.contains("\"gui\""), "kind is gui, not the id's tui spelling: {meta}");

    // …and the row is IN THE LIST. This line was owed from the start:
    // everything below reads the row through `dir_of`, which kept
    // working while the kind-prefix rebuild dropped the row from every
    // list (`Meta::id`) — a session that answered every op and
    // appeared nowhere.
    assert!(
        k.rows(|_| 0).iter().any(|r| r.id == id),
        "the GUI row must appear in the session list"
    );

    // A fresh GUI session is a prompt nobody has typed into: 空闲, said
    // first-hand.
    let (word, source) = wait_for(&root, &id, State::Idle).expect("a state");
    assert_eq!((word, source), (State::Idle, Some(Source::Reported)));

    let dir = k.dir_of(&id).unwrap();
    let hf = read_host_file(&dir).expect("a host file");
    let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
    write_frame(&mut conn, &Hello { cookie: hf.cookie, cols: 0, rows: 0 }).unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);

    // One turn, asserted at its **stable anchors** only: 空闲 before,
    // 待批 while the ask waits on us, 完成 after. The two 忙碌 reports
    // between them are real but transient — the stub runs its script in
    // milliseconds, and a poll that must catch a word faster than its
    // own cadence measures the machine, not the host (the tmux lesson,
    // 账本). What pins them instead: 待批 can only be *reached* through
    // the Say path that reports 忙碌 first, and the ask-answered report
    // is the same line of code either way.
    write_frame(&mut conn, &GuiOp::Say("hello".into())).unwrap();

    let GuiNote::Note(first) = next_note(&mut conn) else { panic!("a first chunk") };
    assert!(first.contains("thinking it over"), "{first}");
    let GuiNote::Note(_second) = next_note(&mut conn) else { panic!("a second chunk") };

    let GuiNote::Ask { ask, options, .. } = next_note(&mut conn) else {
        panic!("the stub asks permission")
    };
    assert_eq!(options.len(), 2);
    assert!(matches!(wait_for(&root, &id, State::Blocked), Some((State::Blocked, Some(Source::Reported)))));

    write_frame(&mut conn, &GuiOp::Answer { ask, option: Some(options[0].0.clone()) }).unwrap();
    // (No 忙碌 wait here — see the anchor note above.)

    // The stub's own account of what crossed the wire, relayed whole.
    let GuiNote::Note(echo) = next_note(&mut conn) else { panic!("the echo chunk") };
    assert!(echo.contains("picked:go"), "{echo}");

    let GuiNote::Turn(stop) = next_note(&mut conn) else { panic!("the turn ends") };
    assert_eq!(stop, "EndTurn");
    assert!(matches!(wait_for(&root, &id, State::Done), Some((State::Done, Some(Source::Reported)))));

    // Close is an op, and the ending is recorded like any host's.
    write_frame(&mut conn, &GuiOp::Close).unwrap();
    let GuiNote::Gone = next_note(&mut conn) else { panic!("a goodbye") };
    host.join().expect("the host thread ends").expect("cleanly");
    let state = std::fs::read_to_string(dir.join("state.json")).unwrap_or_default();
    assert!(state.contains("\"exit\":0"), "a close the user asked for exits clean: {state}");

    let _ = std::fs::remove_dir_all(&root);
}

/// Replay in one test, three claims: asked mid-turn it waits for the
/// turn (the `Turn` frame arrives before any `History`), the replayed
/// updates and the end marker reach the asker, and **none of it reaches
/// anyone else** — with the negative half proved live first: the other
/// connection read the whole turn's broadcast through the very frames
/// it is then expected not to get (the grep-scope lesson: an empty read
/// only means something after the same read has found something).
#[test]
fn history_answers_the_asker_alone_and_after_the_turn() {
    let root = std::env::temp_dir().join(format!("khor-guireplay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");

    let host = {
        let root = root.clone();
        let ready = ready.clone();
        let cmd = vec![stub_path().to_string_lossy().to_string()];
        std::thread::spawn(move || gui_host_main(root, ready, "stub".into(), cmd))
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    let id = loop {
        if let Ok(text) = std::fs::read_to_string(&ready) {
            break SessionId(text.trim().to_owned());
        }
        assert!(Instant::now() < deadline, "the host never wrote the ready file");
        std::thread::sleep(Duration::from_millis(50));
    };
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let dir = k.dir_of(&id).unwrap();
    let hf = read_host_file(&dir).expect("a host file");

    let mut attach = || -> TcpStream {
        let mut c = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
        write_frame(&mut c, &Hello { cookie: hf.cookie.clone(), cols: 0, rows: 0 }).unwrap();
        let w: Welcome = read_frame(&mut c).unwrap();
        assert!(w.ok, "{}", w.why);
        c
    };
    let mut asker = attach();
    let mut bystander = attach();

    // Say, then ask for history in the same breath: the ops arrive in
    // order on one stream, so the host sees the turn start first and
    // must hold the replay.
    write_frame(&mut asker, &GuiOp::Say("hello".into())).unwrap();
    write_frame(&mut asker, &GuiOp::Replay).unwrap();

    // The whole turn plays out first, on both connections — anything
    // History-shaped before the turn's end would be the mid-turn
    // interleaving the host exists to prevent. The asker answers the
    // stub's permission ask; the bystander only listens.
    loop {
        match next_note(&mut asker) {
            GuiNote::Ask { ask, options, .. } => {
                write_frame(&mut asker, &GuiOp::Answer { ask, option: Some(options[0].0.clone()) })
                    .unwrap();
            }
            GuiNote::Turn(stop) => {
                assert_eq!(stop, "EndTurn");
                break;
            }
            GuiNote::Note(_) => {}
            other => panic!("history before the turn ended: {}", kind_of(&other)),
        }
    }
    loop {
        match next_note(&mut bystander) {
            GuiNote::Turn(_) => break,
            GuiNote::Note(_) | GuiNote::Ask { .. } => {}
            other => panic!("history before the turn ended: {}", kind_of(&other)),
        }
    }

    // Now the held replay: the asker gets the stub's canned playback
    // and the end marker, in order.
    let mut played = Vec::new();
    loop {
        match next_note(&mut asker) {
            GuiNote::History(json) => played.push(json),
            GuiNote::HistoryEnd => break,
            other => panic!("expected history frames, got {}", kind_of(&other)),
        }
    }
    assert!(
        played.iter().any(|j| j.contains("played back: one"))
            && played.iter().any(|j| j.contains("played back: two")),
        "the replayed conversation reached the asker: {played:?}"
    );

    // The bystander proved its read path on the turn above; the same
    // read now finds silence where broadcast history would be.
    bystander.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
    assert!(
        read_frame::<GuiNote>(&mut bystander).is_err(),
        "history frames must never be broadcast"
    );

    write_frame(&mut asker, &GuiOp::Close).unwrap();
    host.join().expect("the host thread ends").expect("cleanly");
    let _ = std::fs::remove_dir_all(&root);
}

fn kind_of(n: &GuiNote) -> &'static str {
    match n {
        GuiNote::Note(_) => "Note",
        GuiNote::Ask { .. } => "Ask",
        GuiNote::Turn(_) => "Turn",
        GuiNote::Gone => "Gone",
        GuiNote::History(_) => "History",
        GuiNote::HistoryEnd => "HistoryEnd",
    }
}
