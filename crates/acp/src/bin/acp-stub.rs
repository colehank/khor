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
//!
//! `KHOR_STUB_REPLAYS` does **both** halves on purpose: an agent that
//! advertises a capability it does not have, or hides one it does, is a
//! third thing to test and not what these switches are for. What they
//! reproduce is an honest agent with less to offer.

use agent_client_protocol::schema::v1::{
    AgentCapabilities, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse,
    PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionNotification, SessionUpdate,
    StopReason, TextContent, ToolCallUpdate, ToolCallUpdateFields,
};
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> agent_client_protocol::Result<()> {
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
                responder.respond(
                    InitializeResponse::new(version)
                        .agent_capabilities(AgentCapabilities::new().load_session(replays())),
                )
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
                responder.respond(NewSessionResponse::new("stub-session-1"))
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
