//! Khor calls an independent official A2A server: no remote Khor/ACP process.
mod util;
use a2a_server::{AgentExecutor, DefaultRequestHandler, ExecutorContext, InMemoryTaskStore};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
};
use futures::{
    StreamExt,
    stream::{self, BoxStream},
};
use khor_node::{
    Node,
    delegation::{Action, Answer, Record, protocol as p},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const TOKEN: &str = "fixture-token-never-in-tool-output";
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn scratch() -> Scratch {
    let path = std::env::temp_dir().join(format!("khor-native-a2a-{}", p::new_task_id()));
    util::hermetic(&path);
    Scratch(path)
}
struct Server {
    address: String,
    calls: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
struct Executor(Arc<AtomicUsize>);
impl AgentExecutor for Executor {
    fn execute(
        &self,
        ctx: ExecutorContext,
    ) -> BoxStream<'static, Result<p::StreamResponse, p::A2AError>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let message = ctx.message.unwrap();
        let text = match &message.parts[0].content {
            p::PartContent::Text(t) => t.as_str(),
            _ => "",
        };
        if matches!(
            text,
            "inline" | "inline-no-context" | "inline-invalid-context"
        ) {
            let mut reply = p::Message::new(p::Role::Agent, vec![p::Part::text("inline answer")]);
            reply.context_id = match text {
                "inline-no-context" => None,
                "inline-invalid-context" => Some("invalid\ncontext".into()),
                _ => Some(ctx.context_id),
            };
            return Box::pin(stream::iter([Ok(p::StreamResponse::Message(reply))]));
        }
        let state = match text {
            "ask" => p::TaskState::InputRequired,
            "auth" => p::TaskState::AuthRequired,
            "wait" | "progress" => p::TaskState::Working,
            _ => p::TaskState::Completed,
        };
        let mut history = ctx.stored_task.and_then(|t| t.history).unwrap_or_default();
        history.push(message.clone());
        let task = p::Task {
            id: ctx.task_id.clone(),
            context_id: ctx.context_id.clone(),
            status: p::TaskStatus {
                state: state.clone(),
                message: Some(p::Message::new(
                    p::Role::Agent,
                    vec![p::Part::text(if state == p::TaskState::InputRequired {
                        "Which branch?"
                    } else {
                        "accepted"
                    })],
                )),
                timestamp: None,
            },
            history: Some(history),
            artifacts: Some(vec![p::Artifact {
                artifact_id: "result".into(),
                name: Some("answer".into()),
                description: None,
                parts: vec![p::Part::text("native result")],
                metadata: None,
                extensions: None,
            }]),
            metadata: Some([("tenant".into(), serde_json::json!(ctx.tenant))].into()),
        };
        let first = stream::iter([Ok(p::StreamResponse::Task(task))]);
        if text == "wait" {
            return Box::pin(first.chain(stream::pending()));
        }
        if text == "progress" {
            let update = p::TaskStatusUpdateEvent {
                task_id: ctx.task_id,
                context_id: ctx.context_id,
                status: p::TaskStatus {
                    state: p::TaskState::Completed,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            };
            return Box::pin(first.chain(stream::once(async move {
                tokio::time::sleep(Duration::from_millis(250)).await;
                Ok(p::StreamResponse::StatusUpdate(update))
            })));
        }
        Box::pin(first)
    }
    fn cancel(
        &self,
        _: ExecutorContext,
    ) -> BoxStream<'static, Result<p::StreamResponse, p::A2AError>> {
        Box::pin(stream::empty())
    }
}
async fn auth(request: Request<Body>, next: Next) -> Response {
    if request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        != Some(&format!("Bearer {TOKEN}"))
    {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from(TOKEN))
            .unwrap();
    }
    next.run(request).await
}
fn card(address: &str, binding: &str) -> p::AgentCard {
    p::AgentCard {
        name: "Independent agent".into(),
        description: "Official A2A server without Khor".into(),
        version: "1".into(),
        supported_interfaces: vec![p::AgentInterface {
            url: address.into(),
            protocol_binding: binding.into(),
            protocol_version: "1.0".into(),
            tenant: (binding == p::TRANSPORT_PROTOCOL_JSONRPC).then(|| "research-team".into()),
        }],
        capabilities: Default::default(),
        default_input_modes: vec!["text/plain".into()],
        default_output_modes: vec!["text/plain".into()],
        skills: vec![],
        provider: None,
        documentation_url: None,
        icon_url: None,
        security_schemes: None,
        security_requirements: None,
        signatures: None,
    }
}
async fn server(binding: &str, protected: bool) -> Server {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", socket.local_addr().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let handler = Arc::new(DefaultRequestHandler::new(
        Executor(calls.clone()),
        InMemoryTaskStore::new(),
    ));
    let mut router = if binding == p::TRANSPORT_PROTOCOL_JSONRPC {
        a2a_server::jsonrpc::jsonrpc_router(handler)
    } else {
        a2a_server::rest::rest_router(handler)
    };
    router = router.merge(a2a_server::agent_card::agent_card_router(Arc::new(
        a2a_server::StaticAgentCard::new(card(&address, binding)),
    )));
    if protected {
        router = router.layer(middleware::from_fn(auth));
    }
    let task = tokio::spawn(async move {
        axum::serve(socket, router).await.unwrap();
    });
    Server {
        address,
        calls,
        task,
    }
}
async fn connect(root: &Path, server: &Server, token: Option<&str>) -> Result<(), String> {
    let root = root.to_path_buf();
    let address = server.address.clone();
    let token = token.map(str::to_owned);
    tokio::task::spawn_blocking(move || {
        Node::open(root)?
            .connect_a2a("fixture", &address, token.as_deref())
            .map(|_| ())
    })
    .await
    .unwrap()
}
async fn call(root: &Path, action: Action) -> Result<Answer, String> {
    Node::open(root.to_path_buf())?
        .delegation_on("", action)
        .await
}
fn send(id: &str, text: &str, context: Option<String>, task: Option<String>) -> Action {
    let mut message = p::Message::new(p::Role::User, vec![p::Part::text(text)]);
    message.message_id = id.into();
    message.context_id = context;
    message.task_id = task;
    Action::Send {
        agent: "a2a:fixture".into(),
        message,
        parent: None,
    }
}
fn record(answer: Answer) -> Record {
    if let Answer::Task { record } = answer {
        *record
    } else {
        panic!("expected task: {answer:?}")
    }
}
async fn read(root: &Path, id: &str) -> Record {
    record(
        call(
            root,
            Action::Get {
                agent: "a2a:fixture".into(),
                task: id.into(),
            },
        )
        .await
        .unwrap(),
    )
}

fn lose_response(root: &Path, request: &str, task: &str) {
    let ledger = root
        .join(".khor/a2a/http")
        .join(blake3::hash(b"a2a:fixture").to_hex().as_str())
        .join("ledger.json");
    let mut data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&ledger).unwrap()).unwrap();
    for input in data["inputs"].as_object_mut().unwrap().values_mut() {
        if input["request"] == request {
            input["response"] = serde_json::Value::Null;
        }
    }
    data["tasks"]
        .as_object_mut()
        .unwrap()
        .retain(|_, r| r["record"]["task"]["id"] != task);
    std::fs::write(&ledger, serde_json::to_vec(&data).unwrap()).unwrap();
}

async fn interop(binding: &str) {
    let service = server(binding, true).await;
    let root = scratch();
    let error = connect(&root.0, &service, Some("wrong")).await.unwrap_err();
    assert!(error.contains("401"));
    assert!(!error.contains(TOKEN));
    connect(&root.0, &service, Some(TOKEN)).await.unwrap();
    let Answer::Agents { agents } = call(&root.0, Action::Agents).await.unwrap() else {
        panic!()
    };
    let target = agents.iter().find(|a| a.name == "a2a:fixture").unwrap();
    assert_eq!(target.protocol.as_deref(), Some(binding));
    assert!(target.card.is_some());
    assert!(!serde_json::to_string(&agents).unwrap().contains(TOKEN));
    let initial = record(
        call(&root.0, send("ask-1", "ask", None, None))
            .await
            .unwrap(),
    );
    let initial = read(&root.0, &initial.task.id).await;
    assert_eq!(initial.task.status.state, p::TaskState::InputRequired);
    if binding == p::TRANSPORT_PROTOCOL_JSONRPC {
        assert_eq!(
            initial.task.metadata.as_ref().unwrap()["tenant"],
            "research-team"
        );
    }
    assert!(initial.context_session.is_none());
    // Re-open Node: durable input dedup returns the same receipt, no extra work.
    let retry = record(
        call(&root.0, send("ask-1", "ask", None, None))
            .await
            .unwrap(),
    );
    assert_eq!(retry.task.id, initial.task.id);
    assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    assert!(
        call(&root.0, send("ask-1", "different", None, None))
            .await
            .is_err()
    );
    let resumed = record(
        call(
            &root.0,
            send(
                "reply-1",
                "main",
                Some(initial.task.context_id.clone()),
                Some(initial.task.id.clone()),
            ),
        )
        .await
        .unwrap(),
    );
    let resumed = read(&root.0, &resumed.task.id).await;
    assert_eq!(resumed.task.id, initial.task.id);
    assert_eq!(resumed.task.status.state, p::TaskState::Completed);
    assert_eq!(
        resumed.task.artifacts.as_ref().unwrap()[0].parts[0],
        p::Part::text("native result")
    );
    let follow = record(
        call(
            &root.0,
            send(
                "follow-1",
                "done",
                Some(initial.task.context_id.clone()),
                None,
            ),
        )
        .await
        .unwrap(),
    );
    assert_ne!(follow.task.id, initial.task.id);
    assert_eq!(follow.task.context_id, initial.task.context_id);
    // Fault injection: the service accepted work but the process died before
    // saving its response. ListTasks reconciles by the scoped wire message ID.
    lose_response(&root.0, "follow-1", &follow.task.id);
    let before = service.calls.load(Ordering::SeqCst);
    let recovered = record(
        call(
            &root.0,
            send(
                "follow-1",
                "done",
                Some(initial.task.context_id.clone()),
                None,
            ),
        )
        .await
        .unwrap(),
    );
    assert_eq!(recovered.task.id, follow.task.id);
    assert_eq!(service.calls.load(Ordering::SeqCst), before);
    let other_caller = scratch();
    connect(&other_caller.0, &service, Some(TOKEN))
        .await
        .unwrap();
    assert!(
        call(
            &other_caller.0,
            Action::Get {
                agent: "a2a:fixture".into(),
                task: initial.task.id.clone()
            }
        )
        .await
        .is_err()
    );
    assert!(
        call(
            &other_caller.0,
            send(
                "not-my-context",
                "done",
                Some(initial.task.context_id.clone()),
                None
            )
        )
        .await
        .is_err()
    );
    let separate = record(
        call(&other_caller.0, send("ask-1", "done", None, None))
            .await
            .unwrap(),
    );
    assert_ne!(
        separate.task.history.as_ref().unwrap()[0].message_id,
        initial.task.history.as_ref().unwrap()[0].message_id
    );
    let auth_required = record(
        call(&root.0, send("auth-1", "auth", None, None))
            .await
            .unwrap(),
    );
    assert_eq!(
        read(&root.0, &auth_required.task.id)
            .await
            .task
            .status
            .state,
        p::TaskState::AuthRequired
    );
    let progress = record(
        call(&root.0, send("progress-1", "progress", None, None))
            .await
            .unwrap(),
    );
    let progressed = record(
        call(
            &root.0,
            Action::Wait {
                agent: "a2a:fixture".into(),
                task: progress.task.id.clone(),
                revision: progress.revision,
            },
        )
        .await
        .unwrap(),
    );
    assert_eq!(progressed.task.status.state, p::TaskState::Completed);
    let waiting = record(
        call(&root.0, send("wait-1", "wait", None, None))
            .await
            .unwrap(),
    );
    // Parent Stop may only know the request ID after a lost response.
    // Listing reconciles the pending intent before cancellation by task ID.
    lose_response(&root.0, "wait-1", &waiting.task.id);
    let before = service.calls.load(Ordering::SeqCst);
    let Answer::Tasks { records, .. } = call(
        &root.0,
        Action::List {
            agent: "a2a:fixture".into(),
            context: None,
            after: None,
        },
    )
    .await
    .unwrap() else {
        panic!()
    };
    assert!(records.iter().any(|r| r.task.id == waiting.task.id));
    assert_eq!(service.calls.load(Ordering::SeqCst), before);
    let canceled = record(
        call(
            &root.0,
            Action::Cancel {
                agent: "a2a:fixture".into(),
                task: waiting.task.id.clone(),
            },
        )
        .await
        .unwrap(),
    );
    assert_eq!(canceled.task.status.state, p::TaskState::Canceled);
    assert!(canceled.cancellation_requested);
    let inline = call(&root.0, send("inline-1", "inline", None, None))
        .await
        .unwrap();
    let inline_context = match inline {
        Answer::Message { message, .. } => message.context_id.expect("inline context"),
        other => panic!("expected Message, got {other:?}"),
    };
    let before = service.calls.load(Ordering::SeqCst);
    assert!(matches!(
        call(&root.0, send("inline-1", "inline", None, None))
            .await
            .unwrap(),
        Answer::Message { .. }
    ));
    assert_eq!(service.calls.load(Ordering::SeqCst), before);
    // A direct Message owns a context without fabricating a Task. A new
    // request may continue it, and reopening the connector retains ownership.
    let follow = call(
        &root.0,
        send("inline-2", "inline", Some(inline_context.clone()), None),
    )
    .await
    .unwrap();
    assert!(matches!(follow, Answer::Message { message, .. }
        if message.context_id.as_deref() == Some(inline_context.as_str())));
    assert_eq!(service.calls.load(Ordering::SeqCst), before + 1);
    continue_in_another_process(&root.0, &inline_context).await;
    let after_restart = service.calls.load(Ordering::SeqCst);
    assert_eq!(after_restart, before + 2);
    call(
        &root.0,
        send(
            "inline-after-restart",
            "inline",
            Some(inline_context.clone()),
            None,
        ),
    )
    .await
    .unwrap();
    assert_eq!(
        service.calls.load(Ordering::SeqCst),
        after_restart,
        "restart retry must not send twice"
    );

    assert!(
        call(
            &other_caller.0,
            send(
                "not-my-inline",
                "inline",
                Some(inline_context.clone()),
                None
            )
        )
        .await
        .unwrap_err()
        .contains("does not belong")
    );
    let other_root = root.0.clone();
    let address = service.address.clone();
    tokio::task::spawn_blocking(move || {
        Node::open(other_root)
            .unwrap()
            .connect_a2a("other", &address, Some(TOKEN))
            .unwrap()
    })
    .await
    .unwrap();
    let mut wrong_agent = send(
        "wrong-agent-inline",
        "inline",
        Some(inline_context.clone()),
        None,
    );
    if let Action::Send { agent, .. } = &mut wrong_agent {
        *agent = "a2a:other".into();
    }
    assert!(
        call(&root.0, wrong_agent)
            .await
            .unwrap_err()
            .contains("does not belong")
    );
    assert_eq!(service.calls.load(Ordering::SeqCst), after_restart);

    // A stale/mismatched endpoint receipt cannot authorize the current route.
    // Only this scratch ledger is fault-injected, then restored byte for byte.
    let ledger_path = root
        .0
        .join(".khor/a2a/http")
        .join(blake3::hash(b"a2a:fixture").to_hex().as_str())
        .join("ledger.json");
    let original = std::fs::read(&ledger_path).unwrap();
    for boundary in ["endpoint", "agent", "unconfirmed", "task-endpoint"] {
        let mut ledger: serde_json::Value = serde_json::from_slice(&original).unwrap();
        if boundary == "task-endpoint" {
            for receipt in ledger["tasks"].as_object_mut().unwrap().values_mut() {
                receipt["endpoint"]["card_url"] =
                    "http://127.0.0.1:1/different-service.json".into();
            }
        } else {
            for input in ledger["inputs"].as_object_mut().unwrap().values_mut() {
                match boundary {
                    "endpoint" => {
                        input["endpoint"]["card_url"] =
                            "http://127.0.0.1:1/different-service.json".into()
                    }
                    "agent" => input["endpoint"]["name"] = "a2a:other".into(),
                    "unconfirmed" => input["response"] = serde_json::Value::Null,
                    _ => unreachable!(),
                }
            }
        }
        std::fs::write(&ledger_path, serde_json::to_vec(&ledger).unwrap()).unwrap();
        let context = if boundary == "task-endpoint" {
            initial.task.context_id.clone()
        } else {
            inline_context.clone()
        };
        let mismatch = call(
            &root.0,
            send(
                &format!("mismatched-{boundary}-inline"),
                "inline",
                Some(context),
                None,
            ),
        )
        .await;
        std::fs::write(&ledger_path, &original).unwrap();
        assert!(
            mismatch.unwrap_err().contains("does not belong"),
            "boundary {boundary}"
        );
    }
    assert!(
        call(
            &root.0,
            send(
                "unknown-inline-context",
                "inline",
                Some("never-confirmed-context".into()),
                None
            )
        )
        .await
        .unwrap_err()
        .contains("does not belong")
    );
    assert_eq!(service.calls.load(Ordering::SeqCst), after_restart);

    let without_context = call(
        &root.0,
        send("inline-without-context", "inline-no-context", None, None),
    )
    .await
    .unwrap();
    assert!(
        matches!(without_context, Answer::Message { message, .. } if message.context_id.is_none())
    );
    assert!(
        call(
            &root.0,
            send(
                "inline-invalid-context",
                "inline-invalid-context",
                None,
                None
            )
        )
        .await
        .unwrap_err()
        .contains("A2A ID")
    );

    let Answer::Tasks {
        records,
        cached,
        next,
    } = call(
        &root.0,
        Action::List {
            agent: "a2a:fixture".into(),
            context: Some(initial.task.context_id.clone()),
            after: None,
        },
    )
    .await
    .unwrap()
    else {
        panic!()
    };
    assert!(cached);
    assert_eq!(records.len(), 2);
    assert!(next.is_none());
    // Removing discovery does not strand task receipts or their credential route.
    call(
        &root.0,
        Action::Disconnect {
            agent: "a2a:fixture".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        read(&root.0, &canceled.task.id).await.task.status.state,
        p::TaskState::Canceled
    );
    assert!(
        call(&root.0, send("new-after-disconnect", "done", None, None))
            .await
            .is_err()
    );
    assert!(
        call(
            &root.0,
            Action::Get {
                agent: "a2a:fixture".into(),
                task: "not-owned".into()
            }
        )
        .await
        .is_err()
    );
    // Disconnect removes discovery only. Confirmed Message context receipts
    // survive a same-service reconnect and its fresh credential reference.
    connect(&root.0, &service, Some(TOKEN)).await.unwrap();
    let reconnected = call(
        &root.0,
        send(
            "inline-reconnected",
            "inline",
            Some(inline_context.clone()),
            None,
        ),
    )
    .await
    .unwrap();
    assert!(matches!(reconnected, Answer::Message { message, .. }
        if message.context_id.as_deref() == Some(inline_context.as_str())));
    call(
        &root.0,
        Action::Disconnect {
            agent: "a2a:fixture".into(),
        },
    )
    .await
    .unwrap();
    // A name cannot silently be rebound to a different service.
    let other = server(binding, false).await;
    assert!(
        connect(&root.0, &other, None)
            .await
            .unwrap_err()
            .contains("another service")
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn jsonrpc_native_agent_interop() {
    interop(p::TRANSPORT_PROTOCOL_JSONRPC).await
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rest_native_agent_interop() {
    interop(p::TRANSPORT_PROTOCOL_HTTP_JSON).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_rejects_cross_origin_versions_redirects_and_secret_echoes() {
    for case in ["origin", "version", "redirect", "echo"] {
        let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", socket.local_addr().unwrap());
        let mut card = card(&address, p::TRANSPORT_PROTOCOL_JSONRPC);
        match case {
            "origin" => card.supported_interfaces[0].url = "https://other.example.com".into(),
            "version" => card.supported_interfaces[0].protocol_version = "2.0".into(),
            "echo" => card.description = TOKEN.into(),
            _ => {}
        }
        let router = if case == "redirect" {
            Router::new().route(
                "/.well-known/agent-card.json",
                axum::routing::get(|| async {
                    axum::response::Redirect::temporary(
                        "https://other.example.com/.well-known/agent-card.json",
                    )
                }),
            )
        } else {
            a2a_server::agent_card::agent_card_router(Arc::new(a2a_server::StaticAgentCard::new(
                card,
            )))
        };
        let task = tokio::spawn(async move {
            axum::serve(socket, router).await.unwrap();
        });
        let service = Server {
            address,
            calls: Arc::new(AtomicUsize::new(0)),
            task,
        };
        let root = scratch();
        let error = connect(&root.0, &service, Some(TOKEN)).await.unwrap_err();
        assert!(!error.contains(TOKEN));
        assert!(
            Node::open(root.0.clone())
                .unwrap()
                .a2a_agents()
                .unwrap()
                .is_empty()
        );
    }
}

struct Serving(tokio::task::JoinHandle<Result<(), String>>);
impl Drop for Serving {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn paired_connector_keeps_credentials_local_and_scopes_callers() {
    let service = server(p::TRANSPORT_PROTOCOL_JSONRPC, true).await;
    let home = scratch();
    let caller = scratch();
    connect(&home.0, &service, Some(TOKEN)).await.unwrap();
    let local = record(
        call(&home.0, send("local-1", "done", None, None))
            .await
            .unwrap(),
    );
    let local_inline = call(&home.0, send("local-inline", "inline", None, None))
        .await
        .unwrap();
    let Answer::Message {
        message: local_inline,
        ..
    } = local_inline
    else {
        panic!("expected Message")
    };
    let host = Node::open_as(home.0.clone(), "connector").unwrap();
    let home_id = host.device_str().to_owned();
    let serving = Serving(tokio::spawn(async move { host.serve().await }));
    tokio::time::timeout(Duration::from_secs(30), async {
        while !home.0.join(".khor/endpoint.json").exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let invite = Node::open(home.0.clone()).unwrap().invite().unwrap();
    let client = Node::open_as(caller.0.clone(), "caller").unwrap();
    tokio::time::timeout(Duration::from_secs(45), client.pair(&invite))
        .await
        .unwrap()
        .unwrap();
    let agents = client
        .delegation_on(&home_id, Action::Agents)
        .await
        .unwrap();
    assert!(!serde_json::to_string(&agents).unwrap().contains(TOKEN));
    for action in [
        Action::Get {
            agent: "a2a:fixture".into(),
            task: local.task.id.clone(),
        },
        Action::Cancel {
            agent: "a2a:fixture".into(),
            task: local.task.id.clone(),
        },
        send(
            "other-context",
            "done",
            Some(local.task.context_id.clone()),
            None,
        ),
    ] {
        assert!(client.delegation_on(&home_id, action).await.is_err());
    }
    assert!(
        client
            .delegation_on(
                &home_id,
                send("stolen-inline", "inline", local_inline.context_id, None)
            )
            .await
            .unwrap_err()
            .contains("does not belong")
    );
    let remote_inline = client
        .delegation_on(&home_id, send("remote-inline", "inline", None, None))
        .await
        .unwrap();
    let Answer::Message {
        message: remote_inline,
        ..
    } = remote_inline
    else {
        panic!("expected Message")
    };
    let remote_context = remote_inline.context_id.expect("remote inline context");
    let remote_follow = client
        .delegation_on(
            &home_id,
            send(
                "remote-inline-next",
                "inline",
                Some(remote_context.clone()),
                None,
            ),
        )
        .await
        .unwrap();
    assert!(matches!(remote_follow, Answer::Message { message, .. }
        if message.context_id.as_deref() == Some(remote_context.as_str())));
    assert!(
        call(
            &home.0,
            send("home-steals-inline", "inline", Some(remote_context), None)
        )
        .await
        .unwrap_err()
        .contains("does not belong")
    );
    let Answer::Tasks { records, .. } = client
        .delegation_on(
            &home_id,
            Action::List {
                agent: "a2a:fixture".into(),
                context: None,
                after: None,
            },
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(records.is_empty());
    let task = record(
        client
            .delegation_on(&home_id, send("caller-1", "wait", None, None))
            .await
            .unwrap(),
    );
    assert_eq!(task.caller, client.device());
    assert_eq!(task.target.home, local.target.home);
    assert!(
        call(
            &home.0,
            Action::Get {
                agent: "a2a:fixture".into(),
                task: task.task.id.clone()
            }
        )
        .await
        .is_err()
    );
    let before = service.calls.load(Ordering::SeqCst);
    let restarted = Node::open(caller.0.clone()).unwrap();
    let retry = record(
        restarted
            .delegation_on(&home_id, send("caller-1", "wait", None, None))
            .await
            .unwrap(),
    );
    assert_eq!(retry.task.id, task.task.id);
    assert_eq!(service.calls.load(Ordering::SeqCst), before);
    let canceled = record(
        client
            .delegation_on(
                &home_id,
                Action::Cancel {
                    agent: "a2a:fixture".into(),
                    task: task.task.id,
                },
            )
            .await
            .unwrap(),
    );
    assert_eq!(canceled.task.status.state, p::TaskState::Canceled);
    assert!(!caller.0.join(".khor/a2a/http/credentials").exists());
    drop(serving);
}

async fn continue_in_another_process(root: &Path, context: &str) {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "continue_saved_message_child", "--ignored"])
        .env("KHOR_A2A_CONTEXT_ROOT", root)
        .env("KHOR_A2A_CONTEXT_ID", context)
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(20), command.output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[ignore = "fresh process used by the Message-context persistence regression"]
async fn continue_saved_message_child() {
    let root = PathBuf::from(std::env::var_os("KHOR_A2A_CONTEXT_ROOT").unwrap());
    let context = std::env::var("KHOR_A2A_CONTEXT_ID").unwrap();
    let answer = call(
        &root,
        send(
            "inline-after-restart",
            "inline",
            Some(context.clone()),
            None,
        ),
    )
    .await
    .unwrap();
    assert!(matches!(answer, Answer::Message { message, .. }
        if message.context_id.as_deref() == Some(context.as_str())));
}
