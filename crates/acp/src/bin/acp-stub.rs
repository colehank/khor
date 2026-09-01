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
    NewSessionRequest, NewSessionResponse,
    PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionConfigBoolean, SessionConfigKind,
    SessionConfigOption, SessionConfigOptionValue, SessionConfigSelect, SessionConfigSelectOption,
    SessionMode, SessionModeState, SessionNotification, SessionUpdate,
    SessionCapabilities, SessionInfo, SessionListCapabilities, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent, ToolCallUpdate, ToolCallUpdateFields,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent, Stdio};

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
        vec![SessionMode::new("careful", "Careful"), SessionMode::new("bold", "Bold")],
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
    // What the settable stub currently has set: (model, fast).
    let set = Arc::new(Mutex::new(("small".to_owned(), false)));
    let set_option = set.clone();
    Agent
        .builder()
        .name("acp-stub")
        .on_receive_request(
            async move |request: InitializeRequest, responder, _connection| {
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
                responder.respond(InitializeResponse::new(version).agent_capabilities(caps))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: ListSessionsRequest, responder, _connection| {
                // Only reachable when advertised — a client that asked
                // without checking would get an answer here and pass a
                // test it should fail, so this mirrors a real agent by
                // refusing when it never claimed the capability.
                let Some(has) = lists() else {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::method_not_found());
                };
                let sessions = if has {
                    vec![
                        SessionInfo::new("sess-a", PathBuf::from("/w/one")).title("先看看那个文件"),
                        // No title: an agent may have none, and the
                        // client must not turn that into an empty
                        // string on screen.
                        SessionInfo::new("sess-b", PathBuf::from("/w/two")),
                    ]
                } else {
                    vec![]
                };
                responder.respond(ListSessionsResponse::new(sessions))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: NewSessionRequest, responder, _connection| {
                if std::env::var("KHOR_STUB_LOGIN").is_ok() {
                    // The real spelling: `auth_required` is a code, not
                    // a phrase, and a client that matched on words
                    // would pass this test and fail every real agent.
                    return responder.respond_with_error(
                        agent_client_protocol::Error::auth_required(),
                    );
                }
                let mut opened = NewSessionResponse::new("stub-session-1");
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
                responder.respond(SetSessionConfigOptionResponse::new(options_of(&now.0, now.1)))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            // The replay contract, scripted: every replayed update goes
            // out **before** the response — which is the ordering the
            // real adapter honours and khor_acp::Handle::replay leans on.
            async move |request: LoadSessionRequest, responder, connection| {
                if !replays() {
                    return responder
                        .respond_with_error(agent_client_protocol::Error::method_not_found());
                }
                let session = request.session_id.clone();
                for text in ["played back: one", "played back: two"] {
                    connection
                        .send_notification(SessionNotification::new(session.clone(), chunk(text)))?;
                }
                responder.respond(LoadSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, connection| {
                let session = request.session_id.clone();
                for text in ["thinking it over", "about to act"] {
                    connection
                        .send_notification(SessionNotification::new(session.clone(), chunk(text)))?;
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
                    connection
                        .send_notification(SessionNotification::new(session.clone(), chunk(&word)))?;
                    responder.respond(PromptResponse::new(StopReason::EndTurn))
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}
