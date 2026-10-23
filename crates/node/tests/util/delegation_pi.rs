//! Pi SDK -> private MCP -> Rust approval -> detached ACP -> task result.
use super::*;
use khor_node::{AccountKind, CommandRecord, Provider, khor_agent};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tiny_http::{Header, Response, Server};

fn receipt(node: &Node, run: &str) -> Option<CommandRecord> {
    node.wide_log(khor_node::wide::BROADCAST)
        .unwrap()
        .0
        .messages
        .into_iter()
        .filter_map(|m| m.command)
        .find(|c| c.id == run)
}
fn sse(delta: Value, finish: &str) -> String {
    let chunk = |delta: Value, reason: Value| {
        format!(
            "data: {}\n\n",
            json!({"id":"fixture","object":"chat.completion.chunk","created":1,"model":"fixture","choices":[{"index":0,"delta":delta,"finish_reason":reason}]})
        )
    };
    format!(
        "{}{}data: [DONE]\n\n",
        chunk(delta, Value::Null),
        chunk(json!({}), json!(finish))
    )
}
fn model_reply(body: &Value, count: u64) -> String {
    let messages = body["messages"].as_array().unwrap();
    let user = messages.iter().rposition(|m| m["role"] == "user").unwrap();
    let cancel = serde_json::to_string(&messages[user])
        .unwrap()
        .contains("cancel-flow");
    let tool = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(i, m)| *i > user && m["role"] == "tool");
    let (name, args) = match tool {
        None => (
            "delegate",
            json!({"machine":"alpha","agent":"fixture","prompt":"ACP fixture work","request":if cancel {"pi-stop-child"} else {"pi-complete-child"}}),
        ),
        Some((_, result)) => {
            let text = result["content"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    result["content"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|c| c["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("")
                });
            let value: Value = serde_json::from_str(&text)
                .unwrap_or_else(|_| panic!("unexpected tool output: {text}"));
            let record = &value["Task"]["record"];
            assert!(record.is_object(), "task tool failed: {value}");
            let task = record["task"]["id"].as_str().unwrap();
            let state = record["task"]["status"]["state"].as_str().unwrap();
            if state == "TASK_STATE_COMPLETED" {
                assert!(serde_json::to_string(record).unwrap().contains("picked:go"));
                return sse(
                    json!({"role":"assistant","content":"delegation completed: picked:go"}),
                    "stop",
                );
            }
            if state == "TASK_STATE_CANCELED" {
                return sse(
                    json!({"role":"assistant","content":"child stopped"}),
                    "stop",
                );
            }
            let pending = &record["task"]["status"]["message"]["metadata"]["khor.approval"];
            if pending.is_object() && !cancel {
                (
                    "task_answer",
                    json!({"machine":"alpha","agent":"fixture","task":task,"ask":pending["ask"].as_u64().unwrap().to_string(),"option":"go"}),
                )
            } else {
                // Wait returns promptly for an approval. Avoid a synthetic
                // model flooding its own pending permission in stop coverage.
                if cancel && pending.is_object() {
                    std::thread::sleep(Duration::from_millis(150));
                }
                (
                    "task_wait",
                    json!({"machine":"alpha","agent":"fixture","task":task,"revision":record["revision"].as_u64().unwrap().to_string()}),
                )
            }
        }
    };
    sse(
        json!({"role":"assistant","tool_calls":[{"index":0,"id":format!("pi-call-{count}"),"type":"function","function":{"name":format!("mcp__khor__{name}"),"arguments":args.to_string()}}]}),
        "tool_calls",
    )
}
struct ModelServer {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for ModelServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub async fn run(node: &Node) {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = stop.clone();
    let thread = std::thread::spawn(move || {
        let mut count = 0;
        while !server_stop.load(Ordering::SeqCst) {
            let Some(mut request) = server.recv_timeout(Duration::from_millis(100)).unwrap() else {
                continue;
            };
            assert!(
                request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("Authorization")
                        && h.value.as_str() == "Bearer fixture-pi-selected-key")
            );
            let mut text = String::new();
            request.as_reader().read_to_string(&mut text).unwrap();
            let body: Value = serde_json::from_str(&text).unwrap();
            count += 1;
            let response = model_reply(&body, count);
            let _ = request.respond(
                Response::from_string(response)
                    .with_header(Header::from_bytes("Content-Type", "text/event-stream").unwrap()),
            );
        }
    });
    let _model = ModelServer {
        stop,
        thread: Some(thread),
    };
    let account = node
        .register_account(
            "fixture-pi",
            Provider::Newapi,
            AccountKind::Bill,
            Some(&url),
            "fixture-pi-selected-key",
            None,
        )
        .unwrap();
    node.khor_agent_on(
        "alpha",
        khor_agent::Action::Configure {
            account: account.id,
            model: "fixture".into(),
        },
    )
    .await
    .unwrap();
    for cancel in [false, true] {
        let report = node
            .khor_agent_on(
                "alpha",
                khor_agent::Action::Prompt {
                    token: if cancel {
                        "pi-stop-fixture-01"
                    } else {
                        "pi-complete-fixture-01"
                    }
                    .into(),
                    text: if cancel {
                        "cancel-flow"
                    } else {
                        "complete-flow"
                    }
                    .into(),
                },
            )
            .await
            .unwrap();
        let run = report.run.unwrap();
        let until = Instant::now() + Duration::from_secs(40);
        let mut approved = std::collections::HashSet::new();
        let mut stopping = false;
        loop {
            let row = receipt(node, &run).unwrap();
            if row.status == "blocked" {
                let ask = row.agent.as_ref().unwrap().approval.as_ref().unwrap();
                if approved.insert(ask.id.clone()) {
                    node.khor_agent_on(
                        "alpha",
                        khor_agent::Action::Answer {
                            run: run.clone(),
                            ask: ask.id.clone(),
                            allow: true,
                        },
                    )
                    .await
                    .unwrap();
                }
            }
            if cancel && !stopping {
                let Answer::Tasks { records, .. } = node
                    .delegation_on(
                        "alpha",
                        Action::List {
                            agent: "fixture".into(),
                            context: None,
                            after: None,
                        },
                    )
                    .await
                    .unwrap()
                else {
                    panic!()
                };
                if records
                    .iter()
                    .any(|r| r.parent.as_ref().is_some_and(|p| p.id == run) && pending(r).is_some())
                {
                    node.khor_agent_on("alpha", khor_agent::Action::Stop { run: run.clone() })
                        .await
                        .unwrap();
                    stopping = true;
                }
            }
            if row.ended_at_ms.is_some() {
                if cancel {
                    assert!(stopping);
                    assert_eq!(row.status, "cancelled", "{row:?}");
                    let Answer::Tasks { records, .. } = node
                        .delegation_on(
                            "alpha",
                            Action::List {
                                agent: "fixture".into(),
                                context: None,
                                after: None,
                            },
                        )
                        .await
                        .unwrap()
                    else {
                        panic!()
                    };
                    let child = records
                        .iter()
                        .find(|r| r.parent.as_ref().is_some_and(|p| p.id == run))
                        .unwrap();
                    assert_eq!(
                        super::wait(node, "alpha", &child.task.id, true)
                            .await
                            .task
                            .status
                            .state,
                        TaskState::Canceled
                    );
                } else {
                    assert_eq!(row.status, "done", "{row:?}");
                    assert!(row.stdout.contains("delegation completed: picked:go"));
                    assert_eq!(approved.len(), 2);
                }
                break;
            }
            assert!(Instant::now() < until, "Pi delegation stalled: {row:?}");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}
