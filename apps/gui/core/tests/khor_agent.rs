//! Real Rust host -> compiled/source Pi -> MCP -> Node, using a local model.
//! No operator credentials or external model endpoints are read by this test.
use khor_node::{AccountKind, CommandRecord, Node, Provider};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tiny_http::{Header, Response, Server};

struct Fixture {
    root: PathBuf,
    stop: Arc<AtomicBool>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn receipt(node: &Node, id: &str, status: &str) -> CommandRecord {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(record) = node
            .wide_log(khor_node::wide::BROADCAST)
            .unwrap()
            .0
            .messages
            .into_iter()
            .filter_map(|m| m.command)
            .find(|c| c.id == id && c.status == status)
        {
            return record;
        }
        assert!(Instant::now() < until, "agent did not reach {status}");
        thread::sleep(Duration::from_millis(25));
    }
}
fn completion(delta: Value, finish: &str) -> String {
    let event = |delta, reason: Value| {
        format!(
            "data: {}\n\n",
            json!({"id":"fixture","object":"chat.completion.chunk","created":1,"model":"fixture","choices":[{"index":0,"delta":delta,"finish_reason":reason}]})
        )
    };
    format!(
        "{}{}data: [DONE]\n\n",
        event(delta, Value::Null),
        event(json!({}), json!(finish))
    )
}

#[test]
fn network_agent_uses_its_account_persists_context_and_authorizes_actual_tool_calls() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let stop = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
    let server_seen = seen.clone();
    let server_stop = stop.clone();
    let serving = thread::spawn(move || {
        while !server_stop.load(Ordering::SeqCst) {
            let Some(mut request) = server.recv_timeout(Duration::from_millis(100)).unwrap() else {
                continue;
            };
            assert!(
                request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("Authorization")
                        && h.value.as_str() == "Bearer fixture-selected-key")
            );
            let mut body = String::new();
            request.as_reader().read_to_string(&mut body).unwrap();
            let request_body: Value = serde_json::from_str(&body).unwrap();
            let mut seen = server_seen.lock().unwrap();
            seen.push(request_body);
            let count = seen.len();
            drop(seen);
            let response = match count {
                1 => completion(
                    json!({"role":"assistant","tool_calls":[{"index":0,"id":"devices-1","type":"function","function":{"name":"mcp__khor__devices","arguments":"{}"}}]}),
                    "tool_calls",
                ),
                2 => completion(
                    json!({"role":"assistant","content":"network ready"}),
                    "stop",
                ),
                3 => completion(
                    json!({"role":"assistant","tool_calls":[{"index":0,"id":"close-1","type":"function","function":{"name":"mcp__khor__close","arguments":"{\"session\":\"fixture-session\"}"}}]}),
                    "tool_calls",
                ),
                4 => completion(
                    json!({"role":"assistant","content":"operation declined"}),
                    "stop",
                ),
                _ => {
                    thread::sleep(Duration::from_secs(2));
                    completion(
                        json!({"role":"assistant","content":"late response"}),
                        "stop",
                    )
                }
            };
            let _ = request.respond(
                Response::from_string(response)
                    .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap()),
            );
        }
    });
    let root = std::env::temp_dir().join(format!(
        "khor-agent-integration-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    let fixture = Fixture {
        root: root.clone(),
        stop,
    };
    fs::create_dir_all(root.join(".khor")).unwrap();
    fs::write(
        root.join(".khor/relays.json"),
        r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
    )
    .unwrap();
    let node = Node::open_as(root.clone(), "fixture-home").unwrap();
    let account = node
        .register_account(
            "fixture",
            Provider::Newapi,
            AccountKind::Bill,
            Some(&url),
            "fixture-selected-key",
            None,
        )
        .unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let home = node.device_str();
    let configure = json!({"account":account.id,"model":"fixture"});
    let view = rt
        .block_on(khor_gui_core::khor_agent(
            &root,
            home,
            "configure",
            &configure,
        ))
        .unwrap();
    assert_eq!(view.config.unwrap().home, home);
    let first = json!({"token":"fixture-request-one-123456","text":"Remember fixture-task and list my devices"});
    let id = rt
        .block_on(khor_gui_core::khor_agent(&root, home, "prompt", &first))
        .unwrap()
        .run
        .unwrap();
    let done = receipt(&node, &id, "done");
    assert_eq!(done.stdout, "network ready");
    let updates: Vec<Value> = done
        .agent
        .as_ref()
        .unwrap()
        .updates
        .iter()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert!(updates.iter().any(|u| u["sessionUpdate"] == "tool_call"
        && u["toolCallId"] == "devices-1"
        && u["status"] == "completed"));
    assert!(
        updates
            .iter()
            .any(|u| u["sessionUpdate"] == "agent_message_chunk"
                && u["content"]["text"] == "network ready")
    );
    assert!(
        !serde_json::to_string(&done)
            .unwrap()
            .contains("fixture-selected-key")
    );
    assert_eq!(
        rt.block_on(khor_gui_core::khor_agent(&root, home, "prompt", &first))
            .unwrap()
            .run
            .as_deref(),
        Some(id.as_str())
    );
    let mut changed = first.clone();
    changed["text"] = json!("different task");
    assert!(
        rt.block_on(khor_gui_core::khor_agent(&root, home, "prompt", &changed))
            .is_err()
    );
    let second = json!({"token":"fixture-request-two-123456","text":"Close fixture-session"});
    let second_id = rt
        .block_on(khor_gui_core::khor_agent(&root, home, "prompt", &second))
        .unwrap()
        .run
        .unwrap();
    let blocked = receipt(&node, &second_id, "blocked");
    let ask = blocked.agent.unwrap().approval.unwrap();
    assert_eq!(ask.tool, "close");
    rt.block_on(khor_gui_core::khor_agent(&root,home,"configure",&configure)).unwrap();
    assert_eq!(receipt(&node,&second_id,"blocked").agent.unwrap().approval.unwrap().id,ask.id,"a model choice must not dismiss the active approval");
    assert!(
        rt.block_on(khor_gui_core::khor_agent(
            &root,
            home,
            "answer",
            &json!({"run":second_id,"ask":"stale","allow":true})
        ))
        .is_err()
    );
    // A human may need longer than pi-mcp's default 30s request timeout.
    // Progress must keep the real client alive, without continuing the model.
    thread::sleep(Duration::from_secs(32));
    assert_eq!(seen.lock().unwrap().len(), 3, "approval must not time out");
    assert_eq!(
        receipt(&node, &second_id, "blocked")
            .agent
            .unwrap()
            .approval
            .unwrap()
            .id,
        ask.id
    );
    rt.block_on(khor_gui_core::khor_agent(
        &root,
        home,
        "answer",
        &json!({"run":second_id,"ask":ask.id,"allow":false}),
    ))
    .unwrap();
    assert_eq!(
        receipt(&node, &second_id, "done").stdout,
        "operation declined"
    );
    let requests = seen.lock().unwrap();
    assert_eq!(requests.len(), 4, "a retry must not invoke the model twice");
    assert!(requests[0]["tools"].as_array().unwrap().iter().all(|t| {
        t["function"]["name"]
            .as_str()
            .unwrap()
            .starts_with("mcp__khor__")
    }));
    assert!(
        requests[2]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["role"] == "user" && m["content"].to_string().contains("fixture-task"))
    );
    assert!(
        requests[3]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["role"] == "tool" && m["content"].to_string().contains("fixture"))
    );
    drop(requests);
    let third = json!({"token":"fixture-request-stop-123456","text":"Wait for a slow response"});
    let third_id = rt
        .block_on(khor_gui_core::khor_agent(&root, home, "prompt", &third))
        .unwrap()
        .run
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while seen.lock().unwrap().len() < 5 {
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(20));
    }
    rt.block_on(khor_gui_core::khor_agent(&root, home, "configure", &json!({"account":account.id,"model":"fixture-next"}))).unwrap();
    assert_eq!(receipt(&node,&third_id,"running").agent.unwrap().model,"fixture","switching must leave the active request unchanged");
    assert_eq!(node.khor_agent_config().unwrap().unwrap().model,"fixture-next");
    rt.block_on(khor_gui_core::khor_agent(
        &root,
        home,
        "stop",
        &json!({"run":third_id}),
    ))
    .unwrap();
    let cancelled = receipt(&node, &third_id, "cancelled");
    assert!(cancelled.error.is_none());
    assert!(!cancelled.stderr.is_empty());
    // Exercise both GUI mappings and their unmodified JSON projection. This
    // libtest is not a native worker executable; R1's owned CLI tests establish
    // durability, rather than treating this routing check as a remote ACK.
    for action in ["cancel_status", "cancel_retry"] {
        let view = rt.block_on(khor_gui_core::khor_agent(&root, home, action, &json!({"run":third_id}))).unwrap();
        assert_eq!(view.run.as_deref(), Some(third_id.as_str()));
        let data = view.data.unwrap();
        assert_eq!(data["run"], third_id);
        assert!(data["durability"]["state"].is_string());
    }
    drop(fixture);
    serving.join().unwrap();
}

#[test]
fn restarting_an_executor_interrupts_its_receipt_without_starting_pi() {
    let root = std::env::temp_dir().join(format!(
        "khor-agent-recovery-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    let fixture = Fixture {
        root: root.clone(),
        stop: Arc::new(AtomicBool::new(false)),
    };
    let node = Node::open_as(root.clone(), "recovery-fixture").unwrap();
    let record = CommandRecord {
        id: "agent-recovery-fixture-123456".into(),
        owner: node.device_str().into(),
        owner_name: node.name().into(),
        line: "do not replay this task".into(),
        status: "running".into(),
        started_at_ms: 1,
        agent: Some(khor_node::AgentTurn {
            model: "fixture".into(),
            approval: None,
            updated_at_ms: 1,
            updates: Vec::new(),
            updates_truncated: false,
        }),
        ..Default::default()
    };
    let dir = root.join(".khor/agent/runs");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{}.json", record.id)),
        serde_json::to_vec(&json!({"fingerprint":"fixture","record":record})).unwrap(),
    )
    .unwrap();
    node.record_command(&record).unwrap();
    node.reconcile_agent().unwrap();
    assert!(
        receipt(&node, &record.id, "interrupted")
            .ended_at_ms
            .is_some()
    );
    assert!(!root.join(".khor/agent/sessions").exists());
    drop(fixture);
}
