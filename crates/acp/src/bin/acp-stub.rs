//! A scripted ACP agent for the tests — the `fake_sshd` pattern, ACP
//! shaped: real transport (child process, stdio JSON-RPC), scripted
//! behaviour, no model anywhere.
//!
//! One turn does, in order: two message chunks, one permission request
//! (two options: `go`, `stop`), then a final chunk that **names the
//! outcome it received** — which is what lets a test assert the answer
//! actually crossed the wire instead of asserting its own bookkeeping.

use agent_client_protocol::schema::v1::{
    ContentBlock, ContentChunk, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionNotification, SessionUpdate,
    StopReason, TextContent, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Stdio};

fn chunk(text: &str) -> SessionUpdate {
    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
        text.to_owned(),
    ))))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> agent_client_protocol::Result<()> {
    Agent
        .builder()
        .name("acp-stub")
        .on_receive_request(
            async move |request: InitializeRequest, responder, _connection| {
                responder.respond(InitializeResponse::new(request.protocol_version))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: NewSessionRequest, responder, _connection| {
                responder.respond(NewSessionResponse::new("stub-session-1"))
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
