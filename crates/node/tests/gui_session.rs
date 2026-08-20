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

/// The id, once the host has one. Spelled out rather than inlined a
/// third time.
fn wait_for_ready(ready: &std::path::Path) -> SessionId {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(text) = std::fs::read_to_string(ready) {
            return SessionId(text.trim().to_owned());
        }
        assert!(Instant::now() < deadline, "the host never wrote the ready file");
        std::thread::sleep(Duration::from_millis(50));
    }
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

    // **The first thing a face is told is what this agent can do.**
    // Before anything else, because whether asking for the past means
    // anything depends on it — and the answer to that question looks
    // the same either way (`GuiNote::Agent`).
    let GuiNote::Agent { replays, .. } = next_note(&mut conn) else {
        panic!("a face is told the agent's facts before anything else")
    };
    assert!(replays, "the stub advertises load_session");

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

    // The answer comes back as a frame of its own, to every face:
    // "answered" is a fact about the conversation, not a memory in
    // whichever face happened to press the button (`gui_host`).
    let GuiNote::Answered { ask: answered, option } = next_note(&mut conn) else {
        panic!("the answer is announced")
    };
    assert_eq!(answered, ask, "it names the ask it settles");
    assert_eq!(option.as_deref(), Some(options[0].0.as_str()), "and what was chosen");

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
        // A read deadline before the first read, not at the first
        // `next_note`: a frame that stops arriving must make this test
        // **fail**, and a blocking socket with no timeout makes it hang
        // instead — which reads on a terminal as a slow test rather
        // than as a broken one, and cannot be told from an infinite
        // loop. Found by tearing the announcement out to check this
        // suite noticed: it did not fail, it stopped.
        c.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
        write_frame(&mut c, &Hello { cookie: hf.cookie.clone(), cols: 0, rows: 0 }).unwrap();
        let w: Welcome = read_frame(&mut c).unwrap();
        assert!(w.ok, "{}", w.why);
        // Every face gets the agent's facts first (the test above pins
        // that); here it is only drained so the frames this test is
        // about line up.
        let GuiNote::Agent { .. } = read_frame::<GuiNote>(&mut c).unwrap() else {
            panic!("the facts come first")
        };
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
            // `Answered` rides the same broadcast as the ask itself —
            // both faces see it, which is the point of it existing.
            GuiNote::Note(_) | GuiNote::Answered { .. } => {}
            other => panic!("history before the turn ended: {}", kind_of(&other)),
        }
    }
    loop {
        match next_note(&mut bystander) {
            GuiNote::Turn(_) => break,
            GuiNote::Note(_) | GuiNote::Ask { .. } | GuiNote::Answered { .. } => {}
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
        GuiNote::Turning => "Turning",
        GuiNote::Answered { .. } => "Answered",
        GuiNote::Agent { .. } => "Agent",
        GuiNote::Refused { .. } => "Refused",
    }
}

/// The stub with one of its switches thrown, as a single argv element:
/// `gui_host_main` joins the command with spaces, and the protocol
/// crate reads a leading `{` as launch JSON — which is the only way to
/// hand a child an environment through a command *string*.
fn stub_with(key: &str, value: &str) -> Vec<String> {
    vec![serde_json::json!({
        "command": stub_path().to_string_lossy(),
        "env": { key: value },
    })
    .to_string()]
}

/// **An agent that cannot replay says so, and its empty history stays
/// distinguishable from an empty conversation.**
///
/// The two are byte-identical on the wire — an empty `HistoryEnd`
/// bracket either way — so the only thing that can tell them apart is
/// the fact announced up front. Without it a face paints "nothing was
/// said here" over a past that merely cannot be fetched, which is the
/// neighbouring answer rather than the missing one.
#[test]
fn an_agent_that_cannot_replay_is_not_a_conversation_with_nothing_in_it() {
    let root = std::env::temp_dir().join(format!("khor-guihost-noreplay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let host = {
        let (root, ready) = (root.clone(), ready.clone());
        let cmd = stub_with("KHOR_STUB_REPLAYS", "0");
        std::thread::spawn(move || gui_host_main(root, ready, "stub".into(), cmd))
    };
    let id = wait_for_ready(&ready);
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&k.dir_of(&id).unwrap()).expect("a host file");
    let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
    write_frame(&mut conn, &Hello { cookie: hf.cookie, cols: 0, rows: 0 }).unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);

    let GuiNote::Agent { replays, .. } = next_note(&mut conn) else {
        panic!("the facts come first")
    };
    assert!(!replays, "this stub advertises no load_session, and the face must be told");

    // Asking anyway closes the bracket rather than hanging — a face
    // that asks before reading the fact must not be left waiting.
    write_frame(&mut conn, &GuiOp::Replay).unwrap();
    match next_note(&mut conn) {
        GuiNote::HistoryEnd => {}
        other => panic!("an unanswerable replay still closes its bracket, got {}", kind_of(&other)),
    }

    write_frame(&mut conn, &GuiOp::Close).unwrap();
    host.join().expect("the host thread ends").expect("cleanly");
    let _ = std::fs::remove_dir_all(&root);
}

/// **A refusal leaves the ghost as words, on disk, at once.**
///
/// The opener waits on a file and nothing else: the ghost is detached,
/// its stderr goes to /dev/null and its exit code is never collected.
/// So an agent that declined in half a second — for a reason it stated
/// plainly — used to be reported thirty seconds later as a host that
/// never came up, which sends a person to look at khor instead of at
/// the thing that actually said no.
///
/// The literal `.why` is here on purpose: this asserts the reason is
/// *left behind*, not merely returned. A ghost that only returned it
/// would pass every in-process assertion and tell the opener nothing.
#[test]
fn an_agent_that_wants_a_login_says_so_where_the_opener_is_looking() {
    let root = std::env::temp_dir().join(format!("khor-guihost-login-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let cmd = stub_with("KHOR_STUB_LOGIN", "1");
    let why = gui_host_main(root.clone(), ready.clone(), "stub".into(), cmd)
        .expect_err("an agent demanding a login opens no session");

    let expected = khor_catalog::msg::agent_wants_a_login("Authentication required");
    assert_eq!(why, expected, "the login refusal gets its own sentence, not a generic one");
    assert_eq!(
        std::fs::read_to_string(root.join("ready.why")).ok(),
        Some(expected),
        "and it is on disk beside the marker, which is all the opener can read"
    );
    assert!(!ready.exists(), "no session id was written: there is no session");
    let _ = std::fs::remove_dir_all(&root);
}

/// The control for the one above: a command that is not an ACP agent at
/// all must not borrow the login sentence. Without this, mapping every
/// refusal to `Login` would pass that test.
#[test]
fn a_command_that_is_not_an_agent_does_not_borrow_the_login_sentence() {
    let root = std::env::temp_dir().join(format!("khor-guihost-nocmd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let why = gui_host_main(
        root.clone(),
        ready.clone(),
        "nothing".into(),
        vec!["/nonexistent/khor-no-such-agent".to_owned()],
    )
    .expect_err("there is no such binary");
    assert!(
        why.starts_with(khor_catalog::msg::agent_wont_talk("").trim_end_matches(':')),
        "a missing binary reads as one, not as a login: {why}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A conversation with an agent khor did not ship cannot move into a
/// terminal, and says why in the right words** (批⑥).
///
/// It was already refused before this test existed — but with
/// 「没有它的对话记录」, which is the refusal for a *claude or codex*
/// row whose transcript went missing. On a generic agent that sentence
/// sends a person looking for a file that was never supposed to exist:
/// there is no vendor CLI khor knows how to resume and no transcript in
/// a format it reads, and the limit is khor's rather than the disk's.
///
/// The neighbouring-answer trap in miniature: the refusal was correct
/// and its words were not, which is the half a `is_err()` assertion
/// cannot see. So this asserts the sentence.
#[test]
fn a_generic_agents_conversation_says_it_has_no_terminal_form() {
    let root = std::env::temp_dir().join(format!("khor-guihost-noterm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let host = {
        let (root, ready) = (root.clone(), ready.clone());
        let cmd = vec![stub_path().to_string_lossy().to_string()];
        std::thread::spawn(move || gui_host_main(root, ready, "stub".into(), cmd))
    };
    let id = wait_for_ready(&ready);

    let node = khor_node::Node::open(root.clone()).expect("a node on this store");
    let why = node.takeover_term(&id).expect_err("a stub has no terminal form");
    assert_eq!(
        why,
        khor_catalog::msg::no_terminal_form(&id.0),
        "the refusal names khor's own limit, not a missing file"
    );
    assert_ne!(
        why,
        khor_catalog::msg::takeover_no_record(&id.0),
        "and it is not the sentence for a vendor row whose record went missing"
    );

    // Still a live conversation afterwards: a refusal must not have
    // been half a takeover. `GuiOp::Close` rather than `close_session`
    // — the host file names this test process (module head).
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&k.dir_of(&id).unwrap()).expect("the host is still there");
    let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host still listens");
    conn.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
    write_frame(&mut conn, &Hello { cookie: hf.cookie, cols: 0, rows: 0 }).unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);
    write_frame(&mut conn, &GuiOp::Close).unwrap();
    host.join().expect("the host thread ends").expect("cleanly");
    let _ = std::fs::remove_dir_all(&root);
}
