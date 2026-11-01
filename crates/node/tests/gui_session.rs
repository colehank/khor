//! The GUI host end to end against the scripted ACP stub: one row under
//! the vendor's own id, six words driven by the protocol, ops and notes
//! over the host socket. The stub is khor-acp's (`fake_sshd`'s ACP
//! shape); the host runs in-process like `screen_face.rs`'s does — which
//! also means **nothing here may call `close_session`**: the host file
//! names this very test process, and the kill would land on the whole
//! test run. Closing goes through [`GuiOp::Close`], the op the product
//! wants anyway.

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId, State};
use khor_node::gui_host::{Choice, GuiNote, GuiOp, gui_host_main, refused};
use khor_node::host::{Hello, Welcome, read_frame, read_host_file, write_frame};
use khor_node::live::{LiveKind, Source};

/// The stub is another crate's binary, which `CARGO_BIN_EXE_` cannot
/// name across crates: it is built here, once, through the same cargo
/// that is running the tests, and found beside this test's own binary.
/// The build is a no-op when `cargo test --workspace` already made it.
fn stub_path() -> PathBuf {
    if let Some(path)=std::env::var_os("KHOR_ACP_STUB") {
        let path=PathBuf::from(path);
        let metadata=std::fs::symlink_metadata(&path).expect("the pinned stub must exist");
        assert!(path.is_absolute() && metadata.is_file() && !metadata.file_type().is_symlink()
            && path.canonicalize().unwrap()==path,
            "the supplied same-source stub must be an absolute regular file without symlink components");
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(metadata.permissions().mode() & 0o111,0,"the pinned stub must be executable");
        }
        return path;
    }
    assert!(std::env::var_os("KHOR_ACP_TEST_EVIDENCE").is_none(),
        "controlled acceptance requires an explicitly verified KHOR_ACP_STUB; implicit Cargo is forbidden");
    // Local convenience only; this branch is not an acceptance provenance proof.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = std::process::Command::new(cargo)
        .args(["build", "-q", "-p", "khor-acp", "--bin", "acp-stub"])
        .status()
        .expect("cargo runs");
    assert!(status.success(), "the stub builds");
    let me = std::env::current_exe().expect("a test binary");
    me.parent()
        .and_then(|d| d.parent())
        .expect("target/debug")
        .join("acp-stub")
}

fn read_state(root: &Path, id: &SessionId) -> Option<(State, Option<Source>)> {
    let k = LiveKind::new(root.to_path_buf(), DeviceId([1; 32]));
    let dir = k.dir_of(id)?;
    let raw = std::fs::read(dir.join("state.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    let word: State = serde_json::from_value(v.get("word")?.clone()).ok()?;
    let source = v
        .get("source")
        .and_then(|s| serde_json::from_value(s.clone()).ok());
    Some((word, source))
}

fn wait_for(root: &Path, id: &SessionId, want: State) -> Option<(State, Option<Source>)> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last = None;
    while Instant::now() < deadline {
        last = read_state(root, id);
        if let Some((word, _)) = &last
            && *word == want
        {
            return last;
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
        assert!(
            Instant::now() < deadline,
            "the host never wrote the ready file"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn next_note(conn: &mut TcpStream) -> GuiNote {
    conn.set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    loop {
        let note = read_frame::<GuiNote>(conn).expect("a frame within fifteen seconds");
        if !matches!(note, GuiNote::Capabilities(_)) {
            return note;
        }
    }
}

#[test]
fn a_complete_empty_option_list_removes_the_previous_controls() {
    let root = std::env::temp_dir().join(format!(
        "khor-gui-empty-settings-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let command=serde_json::json!({"command":stub_path(),"env":{"KHOR_STUB_SETTINGS":"1","KHOR_STUB_EMPTY_OPTIONS":"1"}}).to_string();
    let host = {
        let root = root.clone();
        let ready = ready.clone();
        std::thread::spawn(move || gui_host_main(root, ready, "options".into(), vec![command]))
    };
    let id = wait_for_ready(&ready);
    let live = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&live.dir_of(&id).unwrap()).unwrap();
    let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).unwrap();
    write_frame(
        &mut conn,
        &Hello {
            cookie: hf.cookie,
            cols: 0,
            rows: 0,
        },
    )
    .unwrap();
    let _: Welcome = read_frame(&mut conn).unwrap();
    let _ = next_note(&mut conn);
    let GuiNote::Settings(first) = next_note(&mut conn) else {
        panic!("initial settings")
    };
    assert!(
        !serde_json::from_str::<serde_json::Value>(&first).unwrap()["options"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    write_frame(
        &mut conn,
        &GuiOp::SetOption {
            id: "fast".into(),
            value: Choice::Switch(true),
        },
    )
    .unwrap();
    let GuiNote::Settings(next) = next_note(&mut conn) else {
        panic!("updated settings")
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&next).unwrap()["options"],
        serde_json::json!([])
    );
    write_frame(&mut conn, &GuiOp::Close).unwrap();
    host.join().unwrap().unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_prompt_keeps_answering_interactions_across_two_faces() {
    let root = std::env::temp_dir().join(format!(
        "khor-gui-elicit-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let command =
        serde_json::json!({"command":stub_path(),"env":{"KHOR_STUB_EXTENDED":"1"}}).to_string();
    let host = {
        let root = root.clone();
        let ready = ready.clone();
        std::thread::spawn(move || gui_host_main(root, ready, "interactive".into(), vec![command]))
    };
    let id = wait_for_ready(&ready);
    let live = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&live.dir_of(&id).unwrap()).unwrap();
    let connect = || {
        let mut c = TcpStream::connect(("127.0.0.1", hf.port)).unwrap();
        write_frame(
            &mut c,
            &Hello {
                cookie: hf.cookie.clone(),
                cols: 0,
                rows: 0,
            },
        )
        .unwrap();
        let _: Welcome = read_frame(&mut c).unwrap();
        c
    };
    let mut a = connect();
    let _ = next_note(&mut a);
    write_frame(&mut a, &GuiOp::Say("interactive prompt".into())).unwrap();
    let ask = loop {
        if let GuiNote::Elicit { ask, .. } = next_note(&mut a) {
            break ask;
        }
    };
    write_frame(
        &mut a,
        &GuiOp::ElicitAnswer {
            ask,
            response: serde_json::json!({"action":"accept","content":{"count":99}}).to_string(),
        },
    )
    .unwrap();
    assert!(matches!(next_note(&mut a), GuiNote::Refused { .. }));
    let mut b = connect();
    let _ = next_note(&mut b);
    loop {
        if let GuiNote::Elicit { ask: pending, .. } = next_note(&mut b) {
            assert_eq!(pending, ask);
            break;
        }
    }
    write_frame(
        &mut b,
        &GuiOp::ElicitAnswer {
            ask,
            response: serde_json::json!({"action":"accept","content":{"count":2}}).to_string(),
        },
    )
    .unwrap();
    assert!(matches!(next_note(&mut b),GuiNote::ElicitAnswered{ask:answered,..}if answered==ask));
    let mut answer=String::new();
    loop {
        match next_note(&mut a) {
            GuiNote::Note(note)=>answer.push_str(&note),
            GuiNote::Turn(stop)=> { assert_eq!(stop,"EndTurn");break; },
            _=>{},
        }
    }
    assert!(answer.contains("prompt-answer"));
    assert!(answer.contains("accept"));
    // Current prompt output is live on both views, unlike the retired private
    // history bracket. Drain and verify it before probing a stale answer.
    let mut mirrored=String::new();
    loop {
        match next_note(&mut b) {
            GuiNote::Note(note)=>mirrored.push_str(&note),
            GuiNote::Turn(stop)=> { assert_eq!(stop,"EndTurn");break; },
            _=>{},
        }
    }
    assert!(mirrored.contains("prompt-answer") && mirrored.contains("accept"));
    write_frame(
        &mut b,
        &GuiOp::ElicitAnswer {
            ask,
            response: "{\"action\":\"accept\"}".into(),
        },
    )
    .unwrap();
    assert!(
        matches!(next_note(&mut b), GuiNote::Refused { .. }),
        "late answer does not execute again"
    );
    write_frame(&mut a, &GuiOp::Close).unwrap();
    host.join().unwrap().unwrap();
    let _ = std::fs::remove_dir_all(root);
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
        assert!(
            Instant::now() < deadline,
            "the host never wrote the ready file"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(
        id.0, "tui/stub-session-1",
        "the row lives at the vendor-session agreement"
    );

    // Behaviour follows Meta::kind, and that says gui.
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let meta = std::fs::read_to_string(k.dir_of(&id).unwrap().join("meta.json"))
        .expect("a registered row");
    assert!(
        meta.contains("\"gui\""),
        "kind is gui, not the id's tui spelling: {meta}"
    );

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
    write_frame(
        &mut conn,
        &Hello {
            cookie: hf.cookie,
            cols: 0,
            rows: 0,
        },
    )
    .unwrap();
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

    let GuiNote::Note(first) = next_note(&mut conn) else {
        panic!("a first chunk")
    };
    assert!(first.contains("thinking it over"), "{first}");
    let GuiNote::Note(_second) = next_note(&mut conn) else {
        panic!("a second chunk")
    };

    let GuiNote::Ask { ask, options, .. } = next_note(&mut conn) else {
        panic!("the stub asks permission")
    };
    assert_eq!(options.len(), 2);
    assert!(matches!(
        wait_for(&root, &id, State::Blocked),
        Some((State::Blocked, Some(Source::Reported)))
    ));

    write_frame(
        &mut conn,
        &GuiOp::Answer {
            ask,
            option: Some(options[0].0.clone()),
        },
    )
    .unwrap();
    // (No 忙碌 wait here — see the anchor note above.)

    // The answer comes back as a frame of its own, to every face:
    // "answered" is a fact about the conversation, not a memory in
    // whichever face happened to press the button (`gui_host`).
    let GuiNote::Answered {
        ask: answered,
        option,
    } = next_note(&mut conn)
    else {
        panic!("the answer is announced")
    };
    assert_eq!(answered, ask, "it names the ask it settles");
    assert_eq!(
        option.as_deref(),
        Some(options[0].0.as_str()),
        "and what was chosen"
    );

    // The stub's own account of what crossed the wire, relayed whole.
    let GuiNote::Note(echo) = next_note(&mut conn) else {
        panic!("the echo chunk")
    };
    assert!(echo.contains("picked:go"), "{echo}");

    let GuiNote::Turn(stop) = next_note(&mut conn) else {
        panic!("the turn ends")
    };
    assert_eq!(stop, "EndTurn");
    assert!(matches!(
        wait_for(&root, &id, State::Done),
        Some((State::Done, Some(Source::Reported)))
    ));

    // Close is an op, and the ending is recorded like any host's.
    write_frame(&mut conn, &GuiOp::Close).unwrap();
    let GuiNote::Gone = next_note(&mut conn) else {
        panic!("a goodbye")
    };
    host.join().expect("the host thread ends").expect("cleanly");
    let state = std::fs::read_to_string(dir.join("state.json")).unwrap_or_default();
    assert!(
        state.contains("\"exit\":0"),
        "a close the user asked for exits clean: {state}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Each view receives the live turn but only its own Capture response. The
/// captured canonical cursor includes the completed turn and is readable without
/// asking the agent to load again. The negative view first proves its live path.
#[test]
fn capture_answers_its_own_view_and_contains_the_completed_turn() {
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
        assert!(
            Instant::now() < deadline,
            "the host never wrote the ready file"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let dir = k.dir_of(&id).unwrap();
    let hf = read_host_file(&dir).expect("a host file");

    let attach = || -> TcpStream {
        let mut c = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
        // A read deadline before the first read, not at the first
        // `next_note`: a frame that stops arriving must make this test
        // **fail**, and a blocking socket with no timeout makes it hang
        // instead — which reads on a terminal as a slow test rather
        // than as a broken one, and cannot be told from an infinite
        // loop. Found by tearing the announcement out to check this
        // suite noticed: it did not fail, it stopped.
        c.set_read_timeout(Some(Duration::from_secs(15))).unwrap();
        write_frame(
            &mut c,
            &Hello {
                cookie: hf.cookie.clone(),
                cols: 0,
                rows: 0,
            },
        )
        .unwrap();
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

    write_frame(&mut asker, &GuiOp::Say("hello".into())).unwrap();
    // Both views see the same genuine turn and permission/answer broadcasts.
    loop {
        match next_note(&mut asker) {
            GuiNote::Ask { ask, options, .. } => {
                write_frame(
                    &mut asker,
                    &GuiOp::Answer {
                        ask,
                        option: Some(options[0].0.clone()),
                    },
                )
                .unwrap();
            }
            GuiNote::Turn(stop) => {
                assert_eq!(stop, "EndTurn");
                break;
            }
            // `Answered` rides the same broadcast as the ask itself —
            // both faces see it, which is the point of it existing.
            GuiNote::Note(_) | GuiNote::Answered { .. } | GuiNote::Capabilities(_) | GuiNote::Settings(_) | GuiNote::Turning => {}
            other => panic!("unexpected turn frame: {}", kind_of(&other)),
        }
    }
    loop {
        match next_note(&mut bystander) {
            GuiNote::Turn(_) => break,
            GuiNote::Note(_) | GuiNote::Ask { .. } | GuiNote::Answered { .. } | GuiNote::Capabilities(_) | GuiNote::Settings(_) | GuiNote::Turning => {}
            other => panic!("unexpected turn frame: {}", kind_of(&other)),
        }
    }

    let token=[43;16];
    write_frame(&mut asker,&GuiOp::Capture{token}).unwrap();
    let fence=receive_capture(&mut asker,token);
    let page=canonical_page(&root,&fence);
    assert!(page.events.iter().any(|event|matches!(&event.payload,
        khor_node::session_content::ContentEvent::Update { update: note, .. }
            if note.to_string().contains("picked:go"))),
        "canonical history must contain the answered, completed turn");
    // The bystander proved its read path on the turn above; the same
    // read now finds silence where broadcast history would be.
    bystander
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    assert!(
        read_frame::<GuiNote>(&mut bystander).is_err(),
        "a capture response must never be broadcast"
    );

    write_frame(&mut asker, &GuiOp::Close).unwrap();
    host.join().expect("the host thread ends").expect("cleanly");
    let _ = std::fs::remove_dir_all(&root);
}

fn receive_capture(conn: &mut TcpStream, expected: [u8; 16]) -> khor_node::gui_host::CaptureFence {
    loop {
        if let GuiNote::Captured { token, result } = next_note(conn) {
            assert_eq!(token, expected, "a view received another capture token");
            return result.expect("the current canonical capture is available");
        }
    }
}
fn canonical_page(
    root: &Path,
    fence: &khor_node::gui_host::CaptureFence,
) -> khor_node::adaptor::transcript_page::Page<khor_node::session_content::ContentEvent> {
    let node = khor_node::Node::open(root.to_owned()).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let key = node.content_begin(&fence.binding).await.unwrap();
            let request = khor_node::session_content::remote::ClientRead::claim(
                root,
                fence.binding.clone(),
                &key,
            )
            .unwrap();
            node.content_page(&request, Some(fence.cursor.clone()), false)
                .await
                .unwrap()
        })
}

fn kind_of(n: &GuiNote) -> &'static str {
    match n {
        GuiNote::Note(_) => "Note",
        GuiNote::Ask { .. } => "Ask",
        GuiNote::Turn(_) => "Turn",
        GuiNote::Gone => "Gone",
        GuiNote::Turning => "Turning",
        GuiNote::Answered { .. } => "Answered",
        GuiNote::Agent { .. } => "Agent",
        GuiNote::Refused { .. } => "Refused",
        GuiNote::Settings(_) => "Settings",
        GuiNote::Capabilities(_) => "Capabilities",
        GuiNote::Elicit { .. } => "Elicit",
        GuiNote::ElicitAnswered { .. } => "ElicitAnswered",
        GuiNote::ControlResult { .. } => "ControlResult",
        GuiNote::Delegated { .. } => "Delegated",
        GuiNote::Captured { .. } => "Captured",
    }
}

/// The stub with one of its switches thrown, as a single argv element:
/// `gui_host_main` joins the command with spaces, and the protocol
/// crate reads a leading `{` as launch JSON — which is the only way to
/// hand a child an environment through a command *string*.
fn stub_with(key: &str, value: &str) -> Vec<String> {
    vec![
        serde_json::json!({
            "command": stub_path().to_string_lossy(),
            "env": { key: value },
        })
        .to_string(),
    ]
}

/// NoLoad is an explicit restore capability fact. A new local canonical
/// capture remains available and must retain its honest new/empty origin.
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
    write_frame(
        &mut conn,
        &Hello {
            cookie: hf.cookie,
            cols: 0,
            rows: 0,
        },
    )
    .unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);

    let GuiNote::Agent { replays, .. } = next_note(&mut conn) else {
        panic!("the facts come first")
    };
    assert!(
        !replays,
        "this stub advertises no load_session, and the face must be told"
    );

    let token=[44;16];
    write_frame(&mut conn,&GuiOp::Capture{token}).unwrap();
    let fence=receive_capture(&mut conn,token);
    let page=canonical_page(&root,&fence);
    assert!(page.events.is_empty() && page.references.is_empty());
    let node=khor_node::Node::open(root.clone()).unwrap();
    assert!(matches!(node.session_content(&id,None).unwrap(),
        khor_node::session_content::Availability::Available{capture:Some(capture),..}
            if capture.opening=="new" && capture.gaps==0),
        "fresh captured content must not masquerade as an unsupported restored past");

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
    assert_eq!(
        why, expected,
        "the login refusal gets its own sentence, not a generic one"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("ready.why")).ok(),
        Some(expected),
        "and it is on disk beside the marker, which is all the opener can read"
    );
    assert!(
        !ready.exists(),
        "no session id was written: there is no session"
    );
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
    let why = node
        .takeover_term(&id)
        .expect_err("a stub has no terminal form");
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
    conn.set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    write_frame(
        &mut conn,
        &Hello {
            cookie: hf.cookie,
            cols: 0,
            rows: 0,
        },
    )
    .unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);
    write_frame(&mut conn, &GuiOp::Close).unwrap();
    host.join().expect("the host thread ends").expect("cleanly");
    let _ = std::fs::remove_dir_all(&root);
}

/// **What can be set reaches a face when it attaches, and a change goes
/// the whole way round: face → host → agent → every face.**
///
/// Read off the agent's account where there is one (the stub names the
/// mode it was put in) and off the host's fresh `Settings` for the
/// rest — and a face that attaches *after* the changes is told where
/// things stand, not where they started, the way a pending ask is
/// re-sent. The refusal is the control: a mode the agent never listed
/// comes back as `Refused` to the asker and moves nothing.
#[test]
fn settings_reach_a_face_and_a_change_goes_round() {
    let root = std::env::temp_dir().join(format!("khor-guihost-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ready = root.join("ready");
    let host = {
        let (root, ready) = (root.clone(), ready.clone());
        let cmd = stub_with("KHOR_STUB_SETTINGS", "1");
        std::thread::spawn(move || gui_host_main(root, ready, "stub".into(), cmd))
    };
    let id = wait_for_ready(&ready);
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&k.dir_of(&id).unwrap()).expect("a host file");
    let attach = || {
        let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
        write_frame(
            &mut conn,
            &Hello {
                cookie: hf.cookie.clone(),
                cols: 0,
                rows: 0,
            },
        )
        .unwrap();
        let w: Welcome = read_frame(&mut conn).unwrap();
        assert!(w.ok, "{}", w.why);
        let GuiNote::Agent { .. } = next_note(&mut conn) else {
            panic!("the facts come first")
        };
        let GuiNote::Settings(json) = next_note(&mut conn) else {
            panic!("then what can be set, before anything else")
        };
        (
            conn,
            serde_json::from_str::<serde_json::Value>(&json).expect("the protocol's own JSON"),
        )
    };
    let (mut conn, s) = attach();
    assert_eq!(s["modes"]["currentModeId"], "careful");
    assert_eq!(s["modes"]["availableModes"][1]["id"], "bold");
    assert_eq!(s["options"][0]["id"], "model");
    assert_eq!(s["options"][1]["currentValue"], false);

    write_frame(&mut conn, &GuiOp::SetMode("bold".into())).unwrap();
    // The agent's own word first (events before answers in the loop),
    // then the host telling every face.
    let GuiNote::Note(chunk) = next_note(&mut conn) else {
        panic!("the agent names the mode")
    };
    assert!(chunk.contains("mode:bold"), "{chunk}");
    let GuiNote::Settings(json) = next_note(&mut conn) else {
        panic!("every face learns it")
    };
    let s: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(s["modes"]["currentModeId"], "bold");

    write_frame(&mut conn, &GuiOp::SetMode("reckless".into())).unwrap();
    let GuiNote::Refused { why } = next_note(&mut conn) else {
        panic!("a mode the agent never listed comes back refused, to the asker")
    };
    assert_eq!(why, refused::SETTING_REFUSED);

    write_frame(
        &mut conn,
        &GuiOp::SetOption {
            id: "fast".into(),
            value: Choice::Switch(true),
        },
    )
    .unwrap();
    let GuiNote::Settings(json) = next_note(&mut conn) else {
        panic!("the list as the agent has it")
    };
    let s: serde_json::Value = serde_json::from_str(&json).unwrap();
    let fast = s["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == "fast")
        .unwrap();
    assert_eq!(fast["currentValue"], true);
    assert_eq!(
        s["modes"]["currentModeId"], "bold",
        "the earlier change is still there"
    );

    // A face arriving now.
    let (_late, s) = attach();
    assert_eq!(
        s["modes"]["currentModeId"], "bold",
        "told where things stand, not where they started"
    );
    let fast = s["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == "fast")
        .unwrap();
    assert_eq!(fast["currentValue"], true);

    write_frame(&mut conn, &GuiOp::Close).unwrap();
    host.join().expect("the host thread ends").expect("cleanly");
    let _ = std::fs::remove_dir_all(&root);
}
