//! The codex shim, driven end to end through the real binary: a fake
//! codex app-server (a script speaking canned JSON-RPC) sits where the
//! real one would, and `khor _codexagent` is spoken to over real ACP by
//! the very client the GUI host uses (`khor_acp`). Hermetic — no API,
//! no real codex — yet every seam is the production one: the argv
//! convention, the line framing, the server-request approval
//! round-trip. The frames the fake speaks are the probed ones
//! (2026-08-20, codex-cli 0.146.1; `codexagent`'s module head).
//!
//! One test function on purpose: the fake home and `KHOR_CODEX` are
//! process env, and a second test in this binary would race them.

use std::io::Write as _;

/// The thread id the fake mints — a well-formed uuid because the row id
/// and the rollout name both wear it.
const SID: &str = "01a01e00-0000-7000-8000-000000000042";

#[tokio::test]
async fn the_shim_speaks_acp_for_a_fake_codex() {
    let dir = std::env::temp_dir().join(format!("khor-codexagent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cwd")).unwrap();

    let fake = dir.join("codex.py");
    std::fs::write(
        &fake,
        r#"#!/usr/bin/env python3
import json, sys, os
open("fake.pid", "w").write(str(os.getpid()))
assert sys.argv[1] == "app-server", sys.argv

SID = "01a01e00-0000-7000-8000-000000000042"
def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()
def note(method, params):
    emit({"jsonrpc": "2.0", "method": method, "params": params})

turn_no = 0
for line in sys.stdin:
    m = json.loads(line)
    method = m.get("method")
    if method == "initialize":
        emit({"jsonrpc": "2.0", "id": m["id"], "result": {}})
    elif method == "initialized":
        pass
    elif method == "thread/start":
        open("fake.mode", "w").write("new")
        open("fake.params", "w").write(json.dumps(m["params"]))
        emit({"jsonrpc": "2.0", "id": m["id"], "result": {"thread": {"id": SID}}})
        note("thread/started", {"thread": {"id": SID}})
    elif method == "thread/resume":
        open("fake.mode", "w").write("resume")
        emit({"jsonrpc": "2.0", "id": m["id"],
              "result": {"thread": {"id": m["params"]["threadId"]}}})
    elif method == "turn/start":
        turn_no += 1
        tid = f"turn-{turn_no}"
        text = m["params"]["input"][0]["text"]
        emit({"jsonrpc": "2.0", "id": m["id"], "result": {"turn": {"id": tid}}})
        note("turn/started", {"threadId": SID, "turn": {"id": tid}})
        if "hang" in text:
            # Ends only when the client's stop arrives as turn/interrupt.
            for line in sys.stdin:
                c = json.loads(line)
                if c.get("method") == "turn/interrupt":
                    assert c["params"]["turnId"] == tid, c
                    open("fake.interrupt", "w").write("seen")
                    emit({"jsonrpc": "2.0", "id": c["id"], "result": {}})
                    break
            note("turn/completed",
                 {"threadId": SID, "turn": {"id": tid, "status": "interrupted"}})
            continue
        if "ask-permission" in text:
            emit({"jsonrpc": "2.0", "id": 1000 + turn_no,
                  "method": "item/commandExecution/requestApproval",
                  "params": {"threadId": SID, "turnId": tid, "itemId": "exec-1",
                             "reason": "may I touch x", "command": "touch x"}})
            resp = json.loads(sys.stdin.readline())
            assert resp["id"] == 1000 + turn_no, resp
            decision = resp["result"]["decision"]
            note("item/agentMessage/delta",
                 {"threadId": SID, "turnId": tid, "itemId": "msg-1",
                  "delta": f"verdict:{decision}"})
        else:
            note("item/started",
                 {"threadId": SID, "turnId": tid,
                  "item": {"type": "commandExecution", "id": "exec-0",
                           "command": "true", "status": "inProgress"}})
            note("item/agentMessage/delta",
                 {"threadId": SID, "turnId": tid, "itemId": "msg-1",
                  "delta": f"echo: {text}"})
        note("turn/completed",
             {"threadId": SID, "turn": {"id": tid, "status": "completed"}})
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
        std::env::set_var("KHOR_CODEX", &fake);
        // Scheduler powers arrive only when the opener grants them —
        // an inherited KHOR_AGENT would be a power nobody granted.
        std::env::remove_var("KHOR_AGENT");
    }

    let exe = env!("CARGO_BIN_EXE_khor");
    let (handle, mut events) =
        khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd")).await.unwrap();

    // An ordinary session carries no scheduler config.
    let params: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cwd/fake.params")).unwrap())
            .unwrap();
    assert!(params.get("config").is_none(), "no KHOR_AGENT, no tools: {params}");

    // The session id is codex's own thread uuid — 同源: the rollout on
    // disk wears the same name.
    assert_eq!(handle.session().0.to_string(), SID);

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

    // A plain turn: the reply streams through and the tool act shows.
    let stop = handle.prompt("hello there").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("echo: hello there"), "the reply must stream through: {said}");
    assert!(said.contains("tool_call"), "the command item must surface as a tool act: {said}");

    // The approval round-trip: the server request surfaces as an ACP
    // ask, and the answer lands on codex's own JSON-RPC id as accept.
    let stop = handle.prompt("please ask-permission").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("verdict:accept"), "the fake must see the accept: {said}");

    // A turn that will not end on its own ends when the client says so,
    // and the wire word is turn/interrupt with the running turn's id.
    let handle = std::sync::Arc::new(handle);
    let hung = {
        let handle = handle.clone();
        tokio::spawn(async move { handle.prompt("hang for a while").await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    handle.cancel().unwrap();
    let stop = tokio::time::timeout(std::time::Duration::from_secs(10), hung)
        .await
        .expect("a cancelled turn must end")
        .unwrap()
        .unwrap();
    assert_eq!(format!("{stop:?}"), "Cancelled", "a stopped turn is 取消, not a refusal");
    assert!(
        dir.join("cwd/fake.interrupt").exists(),
        "the cancel must reach codex as turn/interrupt"
    );
    let stop = handle.prompt("after the stop").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("echo: after the stop"), "the conversation survives a stop: {said}");

    // Replay reads the rollout under the vendor home, by the thread id
    // in its name — write one and load it back.
    let day = dir.join("home").join(".codex").join("sessions").join("2026").join("08").join("20");
    std::fs::create_dir_all(&day).unwrap();
    let mut f =
        std::fs::File::create(day.join(format!("rollout-2026-08-20T00-00-00-{SID}.jsonl")))
            .unwrap();
    writeln!(
        f,
        r#"{{"type":"session_meta","payload":{{"session_id":"{SID}","cwd":"{}"}}}}"#,
        dir.join("cwd").display()
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"words from the past"}}}}"#
    )
    .unwrap();
    handle.replay().await.unwrap();
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("words from the past"), "the replay must surface the rollout: {said}");
    assert!(said.contains("user_message_chunk"), "in the shape history() uses");

    // A dead agent must surface as a Closed the pump can break on.
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

    // Dropping the client must not leave the codex child running.
    let (handle2, mut events2) =
        khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd")).await.unwrap();
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
    assert!(gone, "the codex child must die with the shim");
    let _ = pump2.await;

    // The takeover path: resuming an existing thread must reach codex
    // as thread/resume, replay the rollout on the way up, and then
    // converse as usual.
    let (handle3, mut events3) =
        khor_acp::start_resume(&format!("{exe} _codexagent"), dir.join("cwd"), SID)
            .await
            .unwrap();
    assert_eq!(handle3.session().0.to_string(), SID, "the resumed session keeps its id");
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
    assert_eq!(
        std::fs::read_to_string(dir.join("cwd/fake.mode")).unwrap(),
        "resume",
        "codex must be asked to resume, not to start fresh"
    );
    let said = notes3.lock().unwrap().join("\n");
    assert!(
        said.contains("words from the past"),
        "the load must replay the rollout before conversing: {said}"
    );
    drop(handle3);
    let _ = pump3.await;
    let _ = std::fs::remove_dir_all(&dir);
}

/// The real thing, opt-in — 真连 (账本: 网络类改动必须真连): the same
/// ACP client, a real `codex` from PATH, one real model answer. Run
/// with `cargo test -p khor-cli --test codexagent -- --ignored` on a
/// machine whose codex gateway is alive; ignored otherwise because it
/// spends real tokens and needs the network.
#[tokio::test]
#[ignore]
async fn the_shim_answers_through_a_real_codex() {
    let dir = std::env::temp_dir().join(format!("khor-codexreal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cwd")).unwrap();
    unsafe {
        std::env::set_var("KHOR_HOME", dir.join("home"));
        std::env::remove_var("KHOR_CODEX");
    }
    let exe = env!("CARGO_BIN_EXE_khor");
    let (handle, mut events) =
        khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd")).await.unwrap();
    let sid = handle.session().0.to_string();
    assert_eq!(sid.len(), 36, "codex's own thread uuid: {sid}");

    let notes = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let n2 = notes.clone();
    let pump = tokio::spawn(async move {
        while let Some(e) = events.recv().await {
            match e {
                khor_acp::Event::Note(n) => {
                    n2.lock().unwrap().push(serde_json::to_string(&n).unwrap());
                }
                khor_acp::Event::Closed(_) => break,
                _ => {}
            }
        }
    });
    let stop = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        handle.prompt("Reply with exactly one word: alive"),
    )
    .await
    .expect("a real turn must end inside the budget")
    .unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let said = notes.lock().unwrap().join("\n");
    assert!(said.contains("alive"), "the model's word must stream through: {said}");

    // 同源: the thread this shim opened is a rollout file wearing the
    // same uuid, where discovery and usage accounting will find it.
    let sessions = std::env::home_dir().unwrap().join(".codex/sessions");
    let mut found = false;
    let mut stack = vec![sessions];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|n| n.to_string_lossy().contains(&sid)) {
                found = true;
            }
        }
    }
    assert!(found, "the thread must land as a rollout named {sid}");
    drop(handle);
    let _ = pump.await;
    let _ = std::fs::remove_dir_all(&dir);
}
