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

// RPC completion and the asynchronous event consumer are separate owners.
// One test-local request confirms only its already-accepted finite prefix;
// later traffic does not delay it and a closed/wrong receiver cannot ACK it.
type PrefixRequest = (
    khor_acp::EventPrefix,
    tokio::sync::oneshot::Sender<Result<(), String>>,
);
async fn next_fenced_event(
    events: &mut khor_acp::EventReceiver,
    requests: &mut tokio::sync::mpsc::Receiver<PrefixRequest>,
    pending: &mut Option<PrefixRequest>,
) -> Option<khor_acp::Event> {
    loop {
        if let Some((prefix, _)) = pending.as_ref() {
            let completed = match events.remaining_through(prefix) {
                Ok(0) => Some(Ok(())),
                Err(why) => Some(Err(why)),
                Ok(_) => None,
            };
            if let Some(result) = completed {
                let (_, reply) = pending.take().unwrap();
                let _ = reply.send(result);
            }
        }
        tokio::select! {
            Some(request) = requests.recv(), if pending.is_none() => { *pending = Some(request); },
            event = events.recv() => {
                if matches!(event.as_ref(), None | Some(khor_acp::Event::Closed(_))) {
                    // Close admission before draining so a late sender learns
                    // failure rather than leaving a reply queued without an owner.
                    requests.close();
                    if let Some((_, reply)) = pending.take() { let _ = reply.send(Err("event consumer closed before its prefix".into())); }
                    while let Ok((_, reply)) = requests.try_recv() { let _ = reply.send(Err("event consumer closed before its prefix".into())); }
                }
                return event;
            }
        }
    }
}
async fn consumed_prefix(
    handle: &khor_acp::Handle,
    requests: &tokio::sync::mpsc::Sender<PrefixRequest>,
) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let prefix = handle
            .fence_events()
            .await
            .expect("accepted turn prefix must be written");
        let (reply, completed) = tokio::sync::oneshot::channel();
        requests
            .send((prefix, reply))
            .await
            .expect("the original event pump must own this prefix");
        completed
            .await
            .expect("the pump must answer its prefix request")
            .expect("the pump must process the complete accepted prefix");
    })
    .await
    .expect("the event consumer must process its finite prefix within five seconds");
}

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
assert os.environ["CODEX_HOME"] == os.path.join(os.environ["KHOR_HOME"], ".codex"), "native child must use the same private store as discovery"
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
    elif method == "model/list":
        emit({"jsonrpc":"2.0","id":m["id"],"result":{"data":[{"model":"fixture-model","displayName":"Fixture model","defaultReasoningEffort":"low","supportedReasoningEfforts":[{"reasoningEffort":"low","description":"brief"},{"reasoningEffort":"high","description":"deep"}]}]}})
    elif method == "initialized":
        pass
    elif method == "thread/start":
        open("fake.mode", "w").write("new")
        open("fake.params", "w").write(json.dumps(m["params"]))
        # The policy the thread runs on, as codex answers it: the
        # user's config said workspace-write without network.
        emit({"jsonrpc": "2.0", "id": m["id"], "result": {
            "thread": {"id": SID}, "model":"fixture-model","reasoningEffort":"low", "approvalPolicy": "on-request",
            "sandbox": {"type": "workspaceWrite", "writableRoots": [],
                        "networkAccess": False, "excludeTmpdirEnvVar": False,
                        "excludeSlashTmp": False}}})
        note("thread/started", {"thread": {"id": SID}})
    elif method == "thread/resume":
        open("fake.mode", "w").write("resume")
        emit({"jsonrpc": "2.0", "id": m["id"],
              "result": {"thread": {"id": m["params"]["threadId"]}}})
    elif method == "turn/start":
        turn_no += 1
        tid = f"turn-{turn_no}"
        open("fake.turn", "w").write(json.dumps(m["params"]))
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
            note("item/commandExecution/outputDelta", {"threadId": SID, "turnId": tid, "itemId": "exec-0", "delta": "streamed result"})
            note("item/completed", {"threadId": SID, "turnId": tid, "item": {"id": "exec-0", "type": "commandExecution", "command": "true", "status": "completed", "exitCode": 0, "aggregatedOutput": "streamed result"}})
            note("turn/plan/updated", {"threadId": SID, "turnId": tid, "plan": [{"step": "verify", "status": "completed"}]})
            note("item/started", {"threadId": SID, "turnId": tid, "item": {"id": "edit-0", "type": "fileChange", "status": "inProgress", "changes": []}})
            note("item/completed", {"threadId": SID, "turnId": tid, "item": {"id": "edit-0", "type": "fileChange", "status": "completed", "changes": [{"path": "/work/file.ts", "kind": {"type": "update"}, "diff": "@@ -12 +12 @@\n-old\n+new"}]}})
            note("item/agentMessage/delta",
                 {"threadId": SID, "turnId": tid, "itemId": "msg-1",
                  "delta": f"echo: {text}"})
        note("thread/tokenUsage/updated",{"threadId":SID,"turnId":tid,"tokenUsage":{"last":{"totalTokens":4200},"total":{"totalTokens":990000},"modelContextWindow":128000}})
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
    let (handle, mut events) = khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd"))
        .await
        .unwrap();

    // An ordinary session carries no scheduler config.
    let params: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cwd/fake.params")).unwrap())
            .unwrap();
    assert!(
        params.get("config").is_none(),
        "no KHOR_AGENT, no tools: {params}"
    );

    // The session id is codex's own thread uuid — 同源: the rollout on
    // disk wears the same name.
    let opening = handle.fence_events().await.unwrap();
    let mut initial_notes = Vec::new();
    let mut ready = false;
    while events.remaining_through(&opening).unwrap() > 0 {
        match events.recv().await.expect("fresh opening remains readable") {
            khor_acp::Event::Note(note) => {
                let value = serde_json::to_value(&note).unwrap();
                assert!(
                    !matches!(
                        value["update"]["sessionUpdate"].as_str(),
                        Some("user_message_chunk" | "agent_message_chunk")
                    ),
                    "an unprompted new session must not invent past messages: {value}"
                );
                initial_notes.push(serde_json::to_string(&note).unwrap());
            }
            khor_acp::Event::Ready { .. } => {
                assert!(!ready);
                ready = true;
            }
            _ => panic!("a fresh empty opening unexpectedly asked or closed"),
        }
    }
    assert!(ready, "fresh opening must acknowledge its actual session");
    assert_eq!(handle.session().0.to_string(), SID);

    let notes = std::sync::Arc::new(std::sync::Mutex::new(initial_notes));
    let closed = std::sync::Arc::new(std::sync::Mutex::new(None::<Option<String>>));
    let n2 = notes.clone();
    let c2 = closed.clone();
    let (drain, mut drain_requests) = tokio::sync::mpsc::channel::<PrefixRequest>(1);
    let pump = tokio::spawn(async move {
        let mut pending = None;
        while let Some(e) = next_fenced_event(&mut events, &mut drain_requests, &mut pending).await
        {
            match e {
                khor_acp::Event::Note(n) => {
                    n2.lock().unwrap().push(serde_json::to_string(&n).unwrap());
                }
                khor_acp::Event::Ask(ask) => {
                    let ids: Vec<String> = ask
                        .request
                        .options
                        .iter()
                        .map(|o| o.option_id.0.to_string())
                        .collect();
                    assert_eq!(ids, vec!["allow", "deny"]);
                    ask.choose("allow");
                }
                khor_acp::Event::Closed(why) => {
                    *c2.lock().unwrap() = Some(why);
                    break;
                }
                khor_acp::Event::Ready { .. } => {}
                khor_acp::Event::Elicit(question) => drop(question),
                khor_acp::Event::ElicitDone(_) => {}
            }
        }
    });

    // The mode the thread opened in is read off codex's answer, and
    // the list is codex's three presets.
    let modes = handle
        .settings()
        .modes
        .clone()
        .expect("codex lists its modes");
    assert_eq!(modes.current_mode_id.to_string(), "workspace");
    let ids: Vec<String> = modes
        .available_modes
        .iter()
        .map(|m| m.id.to_string())
        .collect();
    assert_eq!(ids, ["read-only", "workspace", "full-access"]);
    // Both attachments are advertised, because both are carried.
    assert!(
        handle.facts().images && handle.facts().embeds,
        "＋ needs both"
    );

    assert_eq!(
        handle.settings().options.len(),
        2,
        "model and native reasoning choices are available before the first prompt"
    );
    handle
        .set_option(
            "thought_level",
            serde_json::from_value(serde_json::json!({"value":"high"})).unwrap(),
        )
        .await
        .unwrap();
    assert!(
        handle
            .set_option(
                "thought_level",
                serde_json::from_value(serde_json::json!({"value":"invented"})).unwrap()
            )
            .await
            .is_err()
    );
    // A plain turn: the reply streams through and the tool act shows.
    let stop = handle.prompt("hello there").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let params: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cwd/fake.turn")).unwrap()).unwrap();
    assert_eq!(params["effort"], "high");
    consumed_prefix(&handle, &drain).await;
    let said = notes.lock().unwrap().join("\n");
    assert!(
        said.contains("usage_update") && said.contains("4200") && !said.contains("990000"),
        "occupancy is the latest request, not billing total: {said}"
    );
    assert!(
        said.contains("echo: hello there"),
        "the reply must stream through: {said}"
    );
    assert!(
        said.contains("tool_call"),
        "the command item must surface as a tool act: {said}"
    );
    assert!(
        said.contains("tool_call_update")
            && said.contains("completed")
            && said.contains("streamed result"),
        "tool completion/output missing: {said}"
    );
    assert!(
        said.contains("plan") && said.contains("verify"),
        "plan missing: {said}"
    );
    assert!(
        said.contains("/work/file.ts") && said.contains("@@ -12 +12 @@"),
        "native patch missing: {said}"
    );

    // Until a mode is picked, a turn leaves the thread's own policy.
    let turn: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cwd/fake.turn")).unwrap()).unwrap();
    assert!(turn.get("approvalPolicy").is_none(), "{turn}");

    // A picked mode rides the next turn, and attachments ride with it:
    // the image as a data URL, the file as a text item naming it.
    handle.set_mode("read-only").await.unwrap();
    let stop = handle
        .prompt_with(
            "with things",
            &[("image/png".to_owned(), "QUJD".to_owned())],
            &[("notes.txt".to_owned(), "the notes".to_owned())],
        )
        .await
        .unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    let turn: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cwd/fake.turn")).unwrap()).unwrap();
    assert_eq!(turn["approvalPolicy"], "on-request", "{turn}");
    assert_eq!(turn["sandboxPolicy"]["type"], "readOnly", "{turn}");
    let kinds: Vec<&str> = turn["input"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["type"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["text", "text", "image"], "{turn}");
    assert_eq!(turn["input"][2]["url"], "data:image/png;base64,QUJD");
    assert!(
        turn["input"][1]["text"]
            .as_str()
            .unwrap()
            .contains("the notes")
    );
    assert!(
        handle.set_mode("bypassPermissions").await.is_err(),
        "claude's ids are not codex's"
    );

    // The approval round-trip: the server request surfaces as an ACP
    // ask, and the answer lands on codex's own JSON-RPC id as accept.
    let stop = handle.prompt("please ask-permission").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    consumed_prefix(&handle, &drain).await;
    let said = notes.lock().unwrap().join("\n");
    assert!(
        said.contains("verdict:accept"),
        "the fake must see the accept: {said}"
    );

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
    assert_eq!(
        format!("{stop:?}"),
        "Cancelled",
        "a stopped turn is 取消, not a refusal"
    );
    assert!(
        dir.join("cwd/fake.interrupt").exists(),
        "the cancel must reach codex as turn/interrupt"
    );
    let stop = handle.prompt("after the stop").await.unwrap();
    assert_eq!(format!("{stop:?}"), "EndTurn");
    consumed_prefix(&handle, &drain).await;
    let said = notes.lock().unwrap().join("\n");
    assert!(
        said.contains("echo: after the stop"),
        "the conversation survives a stop: {said}"
    );

    // Initial Load reads the rollout under the vendor home, by the thread id
    // in its name — write one and load it back.
    let day = dir
        .join("home")
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("08")
        .join("20");
    std::fs::create_dir_all(&day).unwrap();
    let mut f = std::fs::File::create(day.join(format!("rollout-2026-08-20T00-00-00-{SID}.jsonl")))
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
    // Supported initial Load below consumes this rollout after the old owner ends.

    // A dead agent must surface as a Closed the pump can break on.
    let fake_pid: i32 = std::fs::read_to_string(dir.join("cwd/fake.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    unsafe { libc::kill(fake_pid, libc::SIGKILL) };
    tokio::time::timeout(std::time::Duration::from_secs(10), pump)
        .await
        .expect("the pump must end when the agent dies")
        .unwrap();
    let why = closed
        .lock()
        .unwrap()
        .take()
        .expect("a Closed event arrived");
    assert!(
        why.is_some(),
        "an ending nobody asked for carries words, not None"
    );
    drop(handle);

    // Dropping the client must not leave the codex child running.
    let (handle2, mut events2) = khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd"))
        .await
        .unwrap();
    let pump2 = tokio::spawn(async move { while events2.recv().await.is_some() {} });
    handle2.prompt("hello again").await.unwrap();
    let fake_pid: i32 = std::fs::read_to_string(dir.join("cwd/fake.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        unsafe { libc::kill(fake_pid, 0) },
        0,
        "precondition: the fake is alive"
    );
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
    assert_eq!(
        handle3.session().0.to_string(),
        SID,
        "the resumed session keeps its id"
    );
    let opening = handle3.fence_events().await.unwrap();
    let mut past = Vec::new();
    let mut ready = false;
    while events3.remaining_through(&opening).unwrap() > 0 {
        match events3
            .recv()
            .await
            .expect("restored opening remains readable")
        {
            khor_acp::Event::Note(note) => past.push(serde_json::to_string(&note).unwrap()),
            khor_acp::Event::Ready { .. } => {
                assert!(!ready);
                ready = true;
            }
            _ => panic!("the supported initial Load unexpectedly asked or closed"),
        }
    }
    assert!(ready);
    let said = past.join("\n");
    assert!(
        said.contains("words from the past"),
        "initial Load must surface the recorded rollout before a new prompt: {said}"
    );
    assert!(
        said.contains("user_message_chunk"),
        "the restored past must retain its normalized user-message shape"
    );
    let notes3 = std::sync::Arc::new(std::sync::Mutex::new(past));
    let n3 = notes3.clone();
    let (drain3, mut drain_requests3) = tokio::sync::mpsc::channel::<PrefixRequest>(1);
    let pump3 = tokio::spawn(async move {
        let mut pending = None;
        while let Some(e) =
            next_fenced_event(&mut events3, &mut drain_requests3, &mut pending).await
        {
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
    consumed_prefix(&handle3, &drain3).await;
    let said = notes3.lock().unwrap().join("\n");
    assert!(
        said.contains("echo: hello resumed"),
        "the resumed session converses: {said}"
    );
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
    // The native peer accepts resume, but no transcript exists: opening must
    // fail with a diagnostic, rather than present an invented empty history.
    let missing = "00000000-0000-4000-8000-000000000099";
    match khor_acp::start_resume(&format!("{exe} _codexagent"), dir.join("cwd"), missing).await {
        Err(error) => assert!(error.said().contains("00000000"), "{error:?}"),
        Ok(_) => panic!("a missing recorded history cannot be restored as empty"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The real thing, opt-in — 真连 (账本: 网络类改动必须真连): the same
/// ACP client, a real `codex` from PATH, one real model answer. Run
/// with `cargo test -p khor-cli --test codexagent -- --ignored` on a
/// machine with an explicitly configured isolated KHOR_VENDOR_HOME; ignored
/// otherwise because it spends real tokens and needs authenticated settings.
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
    let (handle, mut events) = khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd"))
        .await
        .unwrap();
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
    assert!(
        said.contains("alive"),
        "the model's word must stream through: {said}"
    );

    // 同源: the thread this shim opened is a rollout file wearing the
    // same uuid, where discovery and usage accounting will find it.
    let sessions = khor_node::adaptor::codex_home(&dir.join("home")).join("sessions");
    let mut found = false;
    let mut stack = vec![sessions];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .file_name()
                .is_some_and(|n| n.to_string_lossy().contains(&sid))
            {
                found = true;
            }
        }
    }
    assert!(found, "the thread must land as a rollout named {sid}");
    drop(handle);
    let _ = pump.await;
    let _ = std::fs::remove_dir_all(&dir);
}
