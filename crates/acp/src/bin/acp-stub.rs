//! A scripted ACP agent for the tests — the `fake_sshd` pattern, ACP
//! shaped: real transport (child process, stdio JSON-RPC), scripted
//! behaviour, no model anywhere.
//!
//! One turn does, in order: two message chunks, one permission request
//! (two options: `go`, `stop`), then a final chunk that **names the
//! outcome it received** — which is what lets a test assert the answer
//! actually crossed the wire instead of asserting its own bookkeeping.
//!
//! # The switches, and why a stub needs to be able to lie
//!
//! The default stub is a *well-behaved* agent, and khor's two shims are
//! well-behaved too — so every agent the tests ever met could replay,
//! spoke v1, and needed no login. The behaviours worth guarding are all
//! on the other side of that: an agent that says it cannot replay, one
//! that answers with a version this client does not speak, one that
//! demands a login. Each is one environment variable, off by default,
//! so the well-behaved path stays the one the other tests exercise:
//!
//! | variable | the stub then |
//! |---|---|
//! | `KHOR_STUB_REPLAYS=0` | advertises no `load_session`, and refuses `session/load` |
//! | `KHOR_STUB_VERSION=<n>` | answers `initialize` with version `n`, whatever was asked |
//! | `KHOR_STUB_LOGIN=1` | answers `session/new` with `auth_required` (-32000) |
//! | `KHOR_STUB_LISTS=1` | advertises `sessionCapabilities.list`, and answers `session/list` |
//! | `KHOR_STUB_LISTS=empty` | advertises it and answers with **no** sessions |
//! | `KHOR_STUB_LIST_CWD=<dir>` | the listed conversations say they live in `dir` (default `/w/one`, `/w/two`) |
//!
//! `KHOR_STUB_LISTS` has three settings rather than two because the
//! fact being tested has three states: an agent that cannot list, one
//! that lists nothing, and one that lists something. **The first two
//! are what a `Vec` alone would flatten into each other**, and that
//! flattening is the bug the client's `Listing` enum exists to prevent
//! — so the stub has to be able to be each of them separately.
//!
//! | `KHOR_STUB_SETTINGS=1` | lists two modes (`careful`, `bold`), a select (`model`) and a switch (`fast`), and answers `session/set_mode` / `session/set_config_option` |
//!
//! `KHOR_STUB_SETTINGS` echoes every mode change as a chunk naming the
//! mode it received, for the same reason the permission answer is
//! echoed: the assertion has to be on the agent's account of what
//! crossed the wire, not on the client's own bookkeeping.
//!
//! `KHOR_STUB_REPLAYS` does **both** halves on purpose: an agent that
//! advertises a capability it does not have, or hides one it does, is a
//! third thing to test and not what these switches are for. What they
//! reproduce is an honest agent with less to offer.

use agent_client_protocol::schema::v1::{
    AgentCapabilities, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest,
    PromptResponse, RequestPermissionOutcome, RequestPermissionRequest, SessionCapabilities,
    SessionConfigBoolean, SessionConfigKind, SessionConfigOption, SessionConfigOptionValue,
    SessionConfigSelect, SessionConfigSelectOption, SessionInfo, SessionListCapabilities,
    SessionMode, SessionModeState, SessionNotification, SessionUpdate,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, StopReason, TextContent, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent, Stdio};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AuthenticateRequest, AuthenticateResponse, CancelNotification, CloseSessionRequest,
    CloseSessionResponse, CreateElicitationRequest, ResumeSessionRequest, ResumeSessionResponse,
};
fn extended() -> bool {
    std::env::var("KHOR_STUB_EXTENDED").is_ok() || std::env::args().any(|v| v == "--extended")
}
fn question(session: Option<&str>) -> CreateElicitationRequest {
    let mut v = serde_json::json!({"mode":"form","message":"Choose a count","requestedSchema":{"type":"object","properties":{"count":{"type":"integer","minimum":1,"maximum":3}},"required":["count"]}});
    if let Some(s) = session {
        v["sessionId"] = s.into();
    } else {
        v["requestId"] = "auth".into();
    }
    serde_json::from_value(v).unwrap()
}

fn chunk(text: &str) -> SessionUpdate {
    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
        text.to_owned(),
    ))))
}

/// Whether this stub replays its past. On unless told otherwise — the
/// existing tests were written against an agent that does.
fn replays() -> bool {
    std::env::var("KHOR_STUB_REPLAYS").as_deref() != Ok("0")
}

/// Whether this stub lists its sessions, and whether it has any.
/// Off unless asked — `session/list` is optional in the protocol, and
/// the default stub is a plain one.
fn lists() -> Option<bool> {
    match std::env::var("KHOR_STUB_LISTS").as_deref() {
        Ok("1") => Some(true),
        Ok("empty") => Some(false),
        _ => None,
    }
}

/// Whether this stub has anything to set. Off unless asked: most
/// agents offer nothing, and the plain stub must stay one of them so
/// the tests that read「no settings」are reading an agent and not a
/// default.
fn settable() -> bool {
    std::env::var("KHOR_STUB_SETTINGS").as_deref() == Ok("1")
}

fn modes_of(current: &str) -> SessionModeState {
    SessionModeState::new(
        current.to_owned(),
        vec![
            SessionMode::new("careful", "Careful"),
            SessionMode::new("bold", "Bold"),
        ],
    )
}

/// The whole list, as the protocol has an agent answer every change
/// with — one select and one switch, so both value shapes cross.
fn options_of(model: &str, fast: bool) -> Vec<SessionConfigOption> {
    vec![
        SessionConfigOption::new(
            "model",
            "Model",
            SessionConfigKind::Select(SessionConfigSelect::new(
                model.to_owned(),
                vec![
                    SessionConfigSelectOption::new("small", "Small"),
                    SessionConfigSelectOption::new("large", "Large"),
                ],
            )),
        ),
        SessionConfigOption::new(
            "fast",
            "Fast",
            SessionConfigKind::Boolean(SessionConfigBoolean::new(fast)),
        ),
    ]
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> agent_client_protocol::Result<()> {
    // SDK 3 normalizes outgoing initialization versions. Use a deliberately
    // malformed raw peer for the unsupported-wire-version regression instead.
    if let Ok(version) = std::env::var("KHOR_STUB_VERSION") {
        use std::io::{BufRead, Write};
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let request: serde_json::Value = serde_json::from_str(&line).unwrap();
            if request["method"] == "initialize" {
                println!(
                    "{}",
                    serde_json::json!({"jsonrpc":"2.0", "id":request["id"], "result": {
                        "protocolVersion": version.parse::<u16>().unwrap_or(0), "agentCapabilities": {}
                    }})
                );
                std::io::stdout().flush().unwrap();
            }
        }
        return Ok(());
    }
    // What the settable stub currently has set: (model, fast).
    let set = Arc::new(Mutex::new(("small".to_owned(), false)));
    let set_option = set.clone();
    // The scripted agent retains the actual context accepted by session/new.
    let new_context = Arc::new(Mutex::new(String::new()));
    let prompt_context = new_context.clone();
    let canceled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let prompt_canceled = canceled.clone();
    Agent
        .builder()
        .on_receive_notification(
            async move |_request: CancelNotification, _connection| {
                canceled.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .name("acp-stub")
        .on_receive_request(
            async move |request: InitializeRequest, responder, connection| {
                if let Ok(marker)=std::env::var("KHOR_STUB_WAIT_INIT"){
                    std::fs::write(marker,std::process::id().to_string()).unwrap();
                    return connection.spawn(async move{tokio::time::sleep(std::time::Duration::from_secs(600)).await;responder.respond(InitializeResponse::new(request.protocol_version))});
                }
                // The version answered is the agent's to choose — which
                // is exactly why a client must read it back.
                let version = match std::env::var("KHOR_STUB_VERSION") {
                    Ok(n) => ProtocolVersion::from(n.parse::<u16>().unwrap_or(0)),
                    Err(_) => request.protocol_version,
                };
                let mut caps = AgentCapabilities::new().load_session(replays());
                if lists().is_some() {
                    caps = caps.session_capabilities(
                        SessionCapabilities::new().list(SessionListCapabilities::new()),
                    );
                }
                let mut hello=serde_json::to_value(InitializeResponse::new(version).agent_capabilities(caps)).unwrap();
                if extended(){hello["agentCapabilities"]["sessionCapabilities"]=serde_json::json!({"resume":{},"close":{},"delete":{},"additionalDirectories":{}});hello["agentCapabilities"]["mcpCapabilities"]=serde_json::json!({"http":true,"sse":true});hello["authMethods"]=serde_json::json!([{"type":"agent","id":"login","name":"Fixture login"}]);}
                responder.respond(serde_json::from_value::<InitializeResponse>(hello).unwrap())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ListSessionsRequest, responder, _connection| {
                // Only reachable when advertised — a client that asked
                // without checking would get an answer here and pass a
                // test it should fail, so this mirrors a real agent by
                // refusing when it never claimed the capability.
                let Some(has) = lists() else {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::method_not_found());
                };
                // Where the listed conversations say they live. A fixed
                // path by default; a gate that goes on to *open* one
                // (`session/load` wants a directory that exists) names a
                // real one (`KHOR_STUB_LIST_CWD`).
                let at = |fixed: &str| {
                    std::env::var("KHOR_STUB_LIST_CWD")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| PathBuf::from(fixed))
                };
                if let Ok(mode) = std::env::var("KHOR_STUB_PAGES") {
                    if request.cwd.as_deref() != Some(std::path::Path::new("/filtered")) {
                        return responder
                            .respond_with_error(agent_client_protocol::Error::invalid_params())
                    }
                    let page = request.cursor.as_deref();
                    if page.is_some() && mode == "fail" {
                        return responder.respond_with_error(agent_client_protocol::Error::internal_error());
                    }
                    let (ids, next) = match page {
                        None => (vec!["sess-a", "sess-b"], Some("page-two")),
                        Some("page-two") => (vec!["sess-b", "sess-c"], Some(if mode == "loop" { "page-two" } else { "page-three" })),
                        Some("page-three") => (vec!["sess-d"], None),
                        _ => {
                            return responder
                                .respond_with_error(agent_client_protocol::Error::invalid_params())
                        }
                    };
                    let mut response = ListSessionsResponse::new(ids.into_iter().map(|id| SessionInfo::new(id, "/filtered")).collect());
                    response.next_cursor = next.map(Into::into);
                    return responder.respond(response);
                }
                let sessions = if has {
                    vec![
                        SessionInfo::new("sess-a", at("/w/one")).title("先看看那个文件"),
                        // No title: an agent may have none, and the
                        // client must not turn that into an empty
                        // string on screen.
                        SessionInfo::new("sess-b", at("/w/two")),
                    ]
                } else {
                    vec![]
                };
                responder.respond(ListSessionsResponse::new(sessions))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: NewSessionRequest, responder, _connection| {
                if std::env::var("KHOR_STUB_LOGIN").is_ok() {
                    // The real spelling: `auth_required` is a code, not
                    // a phrase, and a client that matched on words
                    // would pass this test and fail every real agent.
                    return responder
                        .respond_with_error(agent_client_protocol::Error::auth_required());
                }
                let session = if std::env::args().any(|arg| arg == "--unique-session") {
                    format!("stub-session-{}", std::process::id())
                } else { "stub-session-1".to_owned() };
                let mut opened = NewSessionResponse::new(session);
                if extended(){let context=serde_json::to_string(&request).unwrap();*new_context.lock().unwrap()=context.clone();let mut meta=serde_json::Map::new();meta.insert("modelId".into(),context.into());opened.meta=Some(meta);}
                if settable() {
                    let (model, fast) = set.lock().unwrap().clone();
                    opened.modes = Some(modes_of("careful"));
                    opened.config_options = Some(options_of(&model, fast));
                }
                responder.respond(opened)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: SetSessionModeRequest, responder, connection| {
                if !settable() {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::method_not_found());
                }
                let id = request.mode_id.to_string();
                // A mode never listed is refused, the way a real agent
                // refuses one — a client that set anything and read
                // back success would be trusting its own request.
                if id != "careful" && id != "bold" {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::invalid_params());
                }
                connection.send_notification(SessionNotification::new(
                    request.session_id.clone(),
                    chunk(&format!("mode:{id}")),
                ))?;
                responder.respond(SetSessionModeResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: SetSessionConfigOptionRequest, responder, _connection| {
                if !settable() {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::method_not_found());
                }
                let mut now = set_option.lock().unwrap();
                match (request.config_id.to_string().as_str(), &request.value) {
                    ("model", SessionConfigOptionValue::ValueId { value }) => {
                        now.0 = value.to_string();
                    }
                    ("fast", SessionConfigOptionValue::Boolean { value }) => now.1 = *value,
                    _ => {
                        return responder
                            .respond_with_error(agent_client_protocol::Error::invalid_params())
                    }
                }
                responder.respond(SetSessionConfigOptionResponse::new(if std::env::var("KHOR_STUB_EMPTY_OPTIONS").is_ok(){vec![]}else{options_of(&now.0,now.1)}))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // Supported initial Load emits the recorded past before its response.
            async move |request: LoadSessionRequest, responder, connection| {
                if !replays() {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::method_not_found());
                }
                let session = request.session_id.clone();
                if extended(){connection.send_notification(SessionNotification::new(session.clone(),chunk(&format!("load-context:{}",serde_json::to_string(&request).unwrap()))))?;return responder.respond(LoadSessionResponse::new());}
                for text in ["played back: one", "played back: two"] {
                    connection.send_notification(SessionNotification::new(
                        session.clone(),
                        chunk(text),
                    ))?;
                }
                responder.respond(LoadSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(async move|request:AuthenticateRequest,responder,connection|{
            if !extended()||request.method_id.0.as_ref()!="login"{return responder.respond_with_error(agent_client_protocol::Error::invalid_params());}
            let inner=connection.clone();connection.spawn(async move{let response=inner.send_request(question(None)).block_task().await?;if serde_json::to_value(response).unwrap()["action"]!="accept"{return responder.respond_with_error(agent_client_protocol::Error::auth_required());}responder.respond(AuthenticateResponse::new())})
        },agent_client_protocol::on_receive_request!())
        .on_receive_request(async move|request:ResumeSessionRequest,responder,connection|{
            connection.send_notification(SessionNotification::new(request.session_id,chunk("no-replay resume")))?;responder.respond(ResumeSessionResponse::new())
        },agent_client_protocol::on_receive_request!())
        .on_receive_request(async move|_request:CloseSessionRequest,responder,_connection|responder.respond(CloseSessionResponse::new()),agent_client_protocol::on_receive_request!())
        .on_receive_request(
            async move |request: PromptRequest, responder, connection| {
                if extended() {
                    let context=prompt_context.lock().unwrap().clone();
                    let inner=connection.clone();
                    return connection.spawn(async move {
                        let answer=inner.send_request(question(Some(request.session_id.0.as_ref()))).block_task().await?;
                        inner.send_notification(SessionNotification::new(request.session_id.clone(),chunk(&format!("prompt-answer:{} context:{}",serde_json::to_string(&answer).unwrap(),context))))?;
                        responder.respond(PromptResponse::new(StopReason::EndTurn))
                    });
                }
                prompt_canceled.store(false, std::sync::atomic::Ordering::SeqCst);
                let canceled = prompt_canceled.clone();
                let session = request.session_id.clone();
                for text in ["thinking it over", "about to act"] {
                    connection.send_notification(SessionNotification::new(
                        session.clone(),
                        chunk(text),
                    ))?;
                }
                let asked = connection.send_request(RequestPermissionRequest::new(
                    session.clone(),
                    ToolCallUpdate::new("tool-1", ToolCallUpdateFields::new()),
                    vec![
                        PermissionOption::new("go", "run it", PermissionOptionKind::AllowOnce),
                        PermissionOption::new("stop", "do not", PermissionOptionKind::RejectOnce),
                    ],
                ));
                let inner = connection.clone();
                connection.spawn(async move {
                    let connection = inner;
                    let outcome = asked.block_task().await?;
                    let word = match outcome.outcome {
                        RequestPermissionOutcome::Selected(s) => format!("picked:{}", s.option_id),
                        _ => "dismissed".to_owned(),
                    };
                    connection.send_notification(SessionNotification::new(
                        session.clone(),
                        chunk(&word),
                    ))?;
                    let stop = if canceled.load(std::sync::atomic::Ordering::SeqCst) {
                        StopReason::Cancelled
                    } else {
                        StopReason::EndTurn
                    };
                    responder.respond(PromptResponse::new(stop))
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}
