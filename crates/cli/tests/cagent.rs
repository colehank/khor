//! The claude shim, driven end to end through the real binary: a fake
//! claude (a script speaking canned stream-json) sits where the real
//! one would, and `khor _cagent` is spoken to over real ACP by the very
//! client the GUI host uses (`khor_acp`). Hermetic — no API, no real
//! claude — yet every seam is the production one: the argv convention,
//! the stdio framing, the control-protocol permission round-trip.
//!
//! One test function on purpose: the fake home and `KHOR_CLAUDE` are
//! process env, and a second test in this binary would race them.

use std::io::Write as _;

#[tokio::test]
async fn the_shim_speaks_acp_for_a_fake_claude() {
    let dir = std::env::temp_dir().join(format!("khor-cagent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cwd")).unwrap();

    // The fake claude: answers the exact frames the probes recorded.
    let fake = dir.join("claude.py");
    std::fs::write(
        &fake,
        r#"#!/usr/bin/env python3
import json, sys, os
open("fake.pid", "w").write(str(os.getpid()))

args = sys.argv[1:]
assert "--permission-prompt-tool" in args, "the shim must ask for the control protocol"
if "--session-id" in args:
    sid = args[args.index("--session-id") + 1]
    open("fake.mode", "w").write("new")
else:
    sid = args[args.index("--resume") + 1]
    open("fake.mode", "w").write("resume")

def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()

first = True
for line in sys.stdin:
    m = json.loads(line)
    if m.get("type") != "user":
        continue
    text = m["message"]["content"][0]["text"]
    if first:
        first = False
        emit({"type": "system", "subtype": "init", "session_id": sid,
              "slash_commands": ["compact", "model"]})
    if "ask-permission" in text:
        emit({"type": "control_request", "request_id": "req-1",
              "request": {"subtype": "can_use_tool", "tool_name": "Write",
                          "display_name": "Write", "description": "x.txt",
                          "tool_use_id": "toolu_1",
                          "input": {"file_path": "x.txt", "content": "hi"}}})
        resp = json.loads(sys.stdin.readline())
        assert resp["type"] == "control_response"
        behavior = resp["response"]["response"]["behavior"]
        emit({"type": "assistant", "message": {"content": [
            {"type": "text", "text": f"verdict:{behavior}"}]}})
    else:
        emit({"type": "assistant", "message": {"content": [
            {"type": "text", "text": f"echo: {text}"}]}})
    emit({"type": "result", "subtype": "success", "session_id": sid})
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Process-global env, in a single-test binary on purpose (module
    // head); unsafe is the 2024-edition spelling of that fact.
    unsafe {
        std::env::set_var("KHOR_HOME", dir.join("home"));
        std::env::set_var("KHOR_CLAUDE", &fake);
    }

    let exe = env!("CARGO_BIN_EXE_khor");
    let (handle, mut events) =
        khor_acp::start(&format!("{exe} _cagent"), dir.join("cwd")).await.unwrap();

    let sid = handle.session().0.to_string();
    assert_eq!(sid.len(), 36, "a minted v4 uuid, dashes and all: {sid}");
    assert_eq!(&sid[14..15], "4", "the version nibble is set, or claude would refuse it");

    // Notes and asks are handled beside the turns: an ask arrives while
    // its prompt is still pending, so a serial drain would deadlock.
    let notes = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let closed = std::sync::Arc::new(std::sync::Mutex::new(None::<Option<String>>));
    let n2 = notes.clone();
    let c2 = closed.clone();
    let pump = tokio::spawn(async move {
        while let Some(e) = events.recv().await {
            match e {
                khor_acp::Event::Note(n) => {
                    n2.lock().unwrap().push(serde_json::to_string(&n).unwrap());
                }
                khor_acp::Event::Ask(ask) => {
                    // The labels are khor's catalog words, and the ids are
                    // the shim's contract.
                    let ids: Vec<String> =
                        ask.request.options.iter().map(|o| o.option_id.0.to_string()).collect();
                    assert_eq!(ids, vec!["allow", "deny"]);
                    ask.choose("allow");
                }
                khor_acp::Event::Closed(why) => {
                    *c2.lock().unwrap() = Some(why);
                    break;
                }
                khor_acp::Event::Ready { .. } => {}
            }
        }
    });

    // A plain turn: the reply streams through, the command list rides
    // the first turn, and the stop reason is the turn's end.
    let stop = handle.prompt("hello there").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("echo: hello there"), "the reply must stream through: {said}");
    assert!(said.contains("available_commands_update"), "the command list must ride along");
    assert!(said.contains("compact"), "with claude's own words in it");

    // The permission round-trip: the ask surfaces as ACP, the answer
    // goes back as a control_response, and the fake sees `allow`.
    let stop = handle.prompt("please ask-permission").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("verdict:allow"), "the fake must see the allow: {said}");

    // Replay reads the vendor transcript through the same lossy leaf
    // the id wears — write one for this session and load it back.
    let slug = dir.join("home").join(".claude").join("projects").join("-x");
    std::fs::create_dir_all(&slug).unwrap();
    let mut f = std::fs::File::create(slug.join(format!("{sid}.jsonl"))).unwrap();
    writeln!(
        f,
        r#"{{"type":"user","message":{{"role":"user","content":"words from the past"}}}}"#
    )
    .unwrap();
    handle.replay().await.unwrap();
    let said = notes.lock().unwrap().join("\n");
    assert!(
        said.contains("words from the past"),
        "the replay must surface the transcript: {said}"
    );
    assert!(said.contains("user_message_chunk"), "in the shape history() uses");

    // A dead agent must surface as a Closed the pump can break on —
    // the hang this guards against was found live: a gui host whose
    // agent died sat in its loop forever (`crates/acp`'s EOF arm).
    let fake_pid: i32 =
        std::fs::read_to_string(dir.join("cwd/fake.pid")).unwrap().trim().parse().unwrap();
    unsafe { libc::kill(fake_pid, libc::SIGKILL) };
    tokio::time::timeout(std::time::Duration::from_secs(10), pump)
        .await
        .expect("the pump must end when the agent dies")
        .unwrap();
    let why = closed.lock().unwrap().take().expect("a Closed event arrived");
    assert!(why.is_some(), "an ending nobody asked for carries words, not None");
    drop(handle);

    // And the other direction: dropping the client must not leave the
    // claude child running (the shim kills it as serve ends — three
    // ppid=1 leftovers taught this the hard way).
    let (handle2, mut events2) =
        khor_acp::start(&format!("{exe} _cagent"), dir.join("cwd")).await.unwrap();
    let pump2 = tokio::spawn(async move { while events2.recv().await.is_some() {} });
    handle2.prompt("hello again").await.unwrap();
    let fake_pid: i32 =
        std::fs::read_to_string(dir.join("cwd/fake.pid")).unwrap().trim().parse().unwrap();
    assert_eq!(unsafe { libc::kill(fake_pid, 0) }, 0, "precondition: the fake is alive");
    drop(handle2);
    let mut gone = false;
    for _ in 0..50 {
        if unsafe { libc::kill(fake_pid, 0) } != 0 {
            gone = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(gone, "the claude child must die with the shim");
    let _ = pump2.await;

    // The takeover path (批C): resuming an existing session must reach
    // claude as `--resume <sid>`, replay the transcript on the way up,
    // and then converse as usual.
    let (handle3, mut events3) =
        khor_acp::start_resume(&format!("{exe} _cagent"), dir.join("cwd"), &sid).await.unwrap();
    assert_eq!(handle3.session().0.to_string(), sid, "the resumed session keeps its id");
    let notes3 = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let n3 = notes3.clone();
    let pump3 = tokio::spawn(async move {
        while let Some(e) = events3.recv().await {
            match e {
                khor_acp::Event::Note(n) => {
                    n3.lock().unwrap().push(serde_json::to_string(&n).unwrap());
                }
                khor_acp::Event::Closed(_) => break,
                _ => {}
            }
        }
    });
    let stop = handle3.prompt("hello resumed").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes3.lock().unwrap().join("\n");
    assert!(said.contains("echo: hello resumed"), "the resumed session converses: {said}");
    // Read after the round-trip — the interpreter needs a beat to write
    // the marker, and a turn proves it is long past started.
    assert_eq!(
        std::fs::read_to_string(dir.join("cwd/fake.mode")).unwrap(),
        "resume",
        "claude must be asked to resume, not to start fresh"
    );
    drop(handle3);
    let _ = pump3.await;
    let _ = std::fs::remove_dir_all(&dir);
}
