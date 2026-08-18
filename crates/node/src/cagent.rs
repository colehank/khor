//! khor's own claude shim: an ACP agent over stdio whose brain is the
//! user's `claude` binary speaking its stream-json protocol.
//!
//! # Why this exists (会话身份批B, the skin ruling)
//!
//! `khor open --gui` hosts *an ACP agent* (`gui_host`). Claude does not
//! speak ACP; the published adapter is an npm package — a runtime
//! dependency on node that 零依赖 (docs/KHOR.md) rules out for a
//! first-class path. But everything the adapter does, claude's own
//! `--input-format stream-json` already offers, and khor already runs
//! the user's claude binary the way it already reads claude's files.
//! So the shim is khor's: `khor _cagent` serves ACP on stdio and holds
//! one claude child underneath. `gui_host`, `crates/acp`, and every
//! face stay byte-for-byte unchanged — to them this is just an agent.
//!
//! # The probed facts this is built on (2026-08-18, claude 2.1.234)
//!
//! - One `--input-format stream-json` process answers many turns.
//! - `--session-id <uuid>` names the session up front, so `session/new`
//!   can answer immediately without spending a turn.
//! - `--resume <id>` continues the *same* session id into the same
//!   transcript file — the 同源 ruling holds on this channel.
//! - Permission asks arrive only under `--permission-prompt-tool stdio`
//!   (a flag `--help` does not list): `control_request` /
//!   `can_use_tool` on stdout, answered by a `control_response` on
//!   stdin — allow echoes the input back as `updatedInput` (probed:
//!   the file really lands), deny really blocks.
//! - The `system`/`init` frame (first turn) carries `slash_commands`.
//!
//! # What is deliberately not translated
//!
//! Only what a face paints: message/thought chunks, tool calls, the
//! permission ask, the stop reason, and the command list. Everything
//! else in the stream is claude's own bookkeeping; relaying it would be
//! curating a protocol nobody reads.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, AvailableCommand, AvailableCommandsUpdate, ContentBlock, ContentChunk,
    InitializeRequest, InitializeResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind, PromptRequest,
    PromptResponse, RequestPermissionOutcome, RequestPermissionRequest, SessionNotification,
    SessionUpdate, StopReason, TextContent, ToolCall, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Stdio};
use khor_catalog::msg;
use tokio::sync::{mpsc, oneshot};

/// Where the claude binary is. The default asks PATH, which is the
/// user's own answer; the variable is the test door (a fake claude
/// drives the whole shim hermetically) and the odd-install escape,
/// `KHOR_TMUX`'s precedent.
const CLAUDE_ENV: &str = "KHOR_CLAUDE";

/// One claude child and the turn in flight against it.
struct Shim {
    root: PathBuf,
    /// Where the claude reader thread delivers parsed frames; `drain`
    /// on the runtime picks them up.
    frames: mpsc::UnboundedSender<serde_json::Value>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
    /// The claude child, so the shim's own ending can take it along.
    child_pid: Mutex<Option<u32>>,
    session: Mutex<Option<String>>,
    /// The pending `session/prompt`, resolved when claude's `result`
    /// frame arrives. One at a time — `gui_host` already refuses a
    /// second Say mid-turn, and the shim holding a queue would answer
    /// "who said what when" wrongly on every face.
    turn: Mutex<Option<oneshot::Sender<StopReason>>>,
    /// A cancel was sent for the current turn: its `result`, whatever
    /// its subtype, is the cancellation the client asked for.
    interrupted: AtomicBool,
}

/// The body of `khor _cagent` (and of `_cagent` in every host-capable
/// binary — `host::main_if_host`).
pub fn cagent_main(root: PathBuf) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(serve(root)).map_err(|e| e.to_string())
}

async fn serve(root: PathBuf) -> Result<(), agent_client_protocol::Error> {
    let (frames_tx, frames_rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let shim = Arc::new(Shim {
        root,
        frames: frames_tx,
        stdin: Mutex::new(None),
        child_pid: Mutex::new(None),
        session: Mutex::new(None),
        turn: Mutex::new(None),
        interrupted: AtomicBool::new(false),
    });
    let s_new = shim.clone();
    let s_prompt = shim.clone();
    let s_load = shim.clone();
    let s_cancel = shim.clone();
    let s_drain = shim.clone();

    Agent
        .builder()
        .name("khor-cagent")
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _connection| {
                responder.respond(
                    InitializeResponse::new(initialize.protocol_version)
                        .agent_capabilities(AgentCapabilities::new().load_session(true)),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |new: NewSessionRequest, responder, _connection| {
                match s_new.start_claude(&new.cwd) {
                    Ok(sid) => responder.respond(NewSessionResponse::new(sid)),
                    Err(e) => responder.respond_with_internal_error(e),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |prompt: PromptRequest, responder, connection| {
                let text = prompt
                    .prompt
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let (tx, rx) = oneshot::channel::<StopReason>();
                {
                    let mut turn = s_prompt.turn.lock().unwrap();
                    if turn.is_some() {
                        return responder.respond_with_internal_error(msg::CAGENT_ONE_TURN);
                    }
                    *turn = Some(tx);
                }
                s_prompt.interrupted.store(false, Ordering::SeqCst);
                if let Err(e) = s_prompt.say(&text) {
                    *s_prompt.turn.lock().unwrap() = None;
                    return responder.respond_with_internal_error(e);
                }
                // The wait lives in a spawned task: the event loop must
                // keep moving updates while the turn runs (the same
                // judgment as `crates/acp`'s permission handler).
                connection.spawn(async move {
                    let reason = rx.await.unwrap_or(StopReason::Cancelled);
                    responder.respond(PromptResponse::new(reason))
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |load: LoadSessionRequest, responder, connection| {
                // Replay is answered from the transcript — the same
                // file, and the very translation the GUI's history()
                // uses for discovered rows, so both pasts read alike.
                let sid = load.session_id.0.to_string();
                let started = s_load.session.lock().unwrap().clone();
                match started {
                    Some(cur) if cur == sid => {}
                    Some(_) => {
                        return responder.respond_with_internal_error(msg::CAGENT_ONE_SESSION)
                    }
                    None => {
                        // A load before any new is the takeover: resume
                        // the recorded conversation. Its world (cwd) is
                        // the transcript's own record, not the loader's
                        // — the resumer may stand anywhere.
                        let home =
                            crate::adaptor::vendor_home(&s_load.root).join(".claude");
                        let cwd = crate::adaptor::claude::Claude::at(home)
                            .recorded_cwd(&crate::live::clean_leaf(&sid))
                            .unwrap_or_else(|| load.cwd.clone());
                        if let Err(e) = s_load.start_claude_resume(&cwd, &sid) {
                            return responder.respond_with_internal_error(e);
                        }
                    }
                }
                let home = crate::adaptor::vendor_home(&s_load.root).join(".claude");
                let said = crate::adaptor::claude::Claude::at(home)
                    .transcript(&crate::live::clean_leaf(&sid))
                    .unwrap_or_default();
                for u in said {
                    let update = replayed(u);
                    let _ = connection
                        .send_notification(SessionNotification::new(sid.clone(), update));
                }
                responder.respond(LoadSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |_cancel: agent_client_protocol::schema::v1::CancelNotification, _cx| {
                // The control-protocol interrupt the SDK sends, and
                // claude honours it: measured on a live turn through
                // the app's own 停止 — a running turn ended in 401 ms
                // and the session took the next line. The flag is the
                // belt: if a vendor ever ignores the interrupt the turn
                // runs out on its own, and its result still reads as
                // the cancellation the client asked for rather than as
                // a refusal (取消 is 空闲; 拒绝 is 中断).
                s_cancel.interrupted.store(true, Ordering::SeqCst);
                let _ = s_cancel.control(&serde_json::json!({
                    "type": "control_request",
                    "request_id": format!("khor-int-{}", std::process::id()),
                    "request": { "subtype": "interrupt" },
                }));
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(Stdio::new(), move |connection: ConnectionTo<Client>| async move {
            // The client's EOF ends the shim (the crate observes a clean
            // EOF, it does not act on it — `connect_with`'s contract);
            // without this arm a killed gui host left the shim, and the
            // claude under it, alive forever (the orphan discipline's
            // exact shape, found live as three ppid=1 leftovers).
            tokio::select! {
                _ = drain(s_drain, frames_rx, &connection) => {}
                _ = connection.incoming_closed() => {}
            }
            Ok(())
        })
        .await
        .map(|_| ())?;
    // However serve ended — client EOF, claude death, a protocol error —
    // the claude child must not outlive the shim.
    if let Some(pid) = shim.child_pid.lock().unwrap().take() {
        #[cfg(unix)]
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGTERM);
        }
        #[cfg(not(unix))]
        let _ = pid;
    }
    Ok(())
}

impl Shim {
    /// Spawns the claude child for one freshly-minted session id.
    fn start_claude(self: &Arc<Self>, cwd: &std::path::Path) -> Result<String, String> {
        let sid = uuid_v4()?;
        self.start_claude_with(cwd, &["--session-id", &sid])?;
        *self.session.lock().unwrap() = Some(sid.clone());
        Ok(sid)
    }

    /// Spawns the claude child **resuming** an existing session — the
    /// takeover path (批C): the conversation already exists on disk, and
    /// `--resume` continues the same id into the same transcript
    /// (probed: same file, and it works across cwd).
    fn start_claude_resume(self: &Arc<Self>, cwd: &std::path::Path, sid: &str) -> Result<(), String> {
        self.start_claude_with(cwd, &["--resume", sid])?;
        *self.session.lock().unwrap() = Some(sid.to_owned());
        Ok(())
    }

    fn start_claude_with(self: &Arc<Self>, cwd: &std::path::Path, tail: &[&str]) -> Result<(), String> {
        if self.session.lock().unwrap().is_some() {
            return Err(msg::CAGENT_ONE_SESSION.into());
        }
        let bin = std::env::var(CLAUDE_ENV).unwrap_or_else(|_| "claude".into());
        let mut c = std::process::Command::new(&bin);
        c.args([
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-prompt-tool",
            "stdio",
        ])
        .args(tail)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        // The nested-session guard: khor's GUI may itself be running
        // inside a claude (the dev bridge always is), and the guard
        // reads these to refuse. This child is not a nested session —
        // it is the session.
        .env_remove("CLAUDECODE")
        .env_remove("CLAUDE_CODE_ENTRYPOINT")
        // Inherited from a dev bridge started inside a claude session,
        // this marker turns transcript saving OFF — which silently
        // severs the whole 同源 story (found live: a probe session with
        // nothing on disk to resume). khor's sessions are never anyone's
        // child sessions.
        .env_remove("CLAUDE_CODE_CHILD_SESSION");
        let mut child = c.spawn().map_err(|e| msg::wont_start(&bin, e))?;
        let stdout = child.stdout.take().ok_or(msg::CAGENT_NO_PIPE)?;
        *self.child_pid.lock().unwrap() = Some(child.id());
        *self.stdin.lock().unwrap() = child.stdin.take();
        let frames = self.frames.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                    if frames.send(v).is_err() {
                        break;
                    }
                }
            }
            // The child ended; a pending turn must not hang forever.
            let _ = frames.send(serde_json::json!({ "type": "khor_child_gone" }));
        });
        Ok(())
    }

    /// One user turn onto claude's stdin.
    fn say(&self, text: &str) -> Result<(), String> {
        self.write_line(&serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
        }))
    }

    fn control(&self, v: &serde_json::Value) -> Result<(), String> {
        self.write_line(v)
    }

    fn write_line(&self, v: &serde_json::Value) -> Result<(), String> {
        let mut guard = self.stdin.lock().unwrap();
        let stdin = guard.as_mut().ok_or(msg::CAGENT_NOT_STARTED)?;
        writeln!(stdin, "{v}").map_err(|e| e.to_string())
    }
}

/// The claude→ACP pump: every stream-json frame, routed. Runs on the
/// connection's runtime because everything it produces is async ACP
/// traffic. The permission round-trip is awaited inline on purpose:
/// claude itself is paused on that very answer, so nothing else is
/// moving on the stream behind it.
async fn drain(
    shim: Arc<Shim>,
    mut frames: mpsc::UnboundedReceiver<serde_json::Value>,
    conn: &ConnectionTo<Client>,
) {
    while let Some(f) = frames.recv().await {
        let sid = match shim.session.lock().unwrap().clone() {
            Some(s) => s,
            None => continue,
        };
        match f.get("type").and_then(|t| t.as_str()) {
            Some("assistant") => {
                for block in f["message"]["content"].as_array().into_iter().flatten() {
                    let update = match block.get("type").and_then(|t| t.as_str()) {
                        Some("text") => block["text"].as_str().map(|t| {
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                ContentBlock::Text(TextContent::new(t.to_owned())),
                            ))
                        }),
                        Some("thinking") => block["thinking"].as_str().map(|t| {
                            SessionUpdate::AgentThoughtChunk(ContentChunk::new(
                                ContentBlock::Text(TextContent::new(t.to_owned())),
                            ))
                        }),
                        Some("tool_use") => {
                            let id = block["id"].as_str().unwrap_or("tool");
                            let name = block["name"].as_str().unwrap_or("tool");
                            Some(SessionUpdate::ToolCall(ToolCall::new(id.to_owned(), name.to_owned())))
                        }
                        _ => None,
                    };
                    if let Some(u) = update {
                        let _ = conn.send_notification(SessionNotification::new(sid.clone(), u));
                    }
                }
            }
            Some("system") if f.get("subtype").and_then(|s| s.as_str()) == Some("init") => {
                let commands: Vec<AvailableCommand> = f["slash_commands"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c.as_str())
                    .map(|c| AvailableCommand::new(c, ""))
                    .collect();
                if !commands.is_empty() {
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(
                            commands,
                        )),
                    ));
                }
            }
            Some("control_request")
                if f["request"]["subtype"].as_str() == Some("can_use_tool") =>
            {
                answer_permission(&shim, conn, &sid, &f).await;
            }
            Some("result") => {
                let reason = if shim.interrupted.swap(false, Ordering::SeqCst) {
                    StopReason::Cancelled
                } else {
                    match f.get("subtype").and_then(|s| s.as_str()) {
                        Some("success") => StopReason::EndTurn,
                        Some(s) if s.contains("max_turns") => StopReason::MaxTurnRequests,
                        _ => StopReason::Refusal,
                    }
                };
                if let Some(tx) = shim.turn.lock().unwrap().take() {
                    let _ = tx.send(reason);
                }
            }
            Some("khor_child_gone") => {
                if let Some(tx) = shim.turn.lock().unwrap().take() {
                    let _ = tx.send(StopReason::Refusal);
                }
                break;
            }
            _ => {}
        }
    }
}

/// One `can_use_tool` → one `session/request_permission` → one
/// `control_response`. Allow echoes the input back as `updatedInput`
/// (the probed contract); everything else — deny, dismiss, a client
/// that went away — is a deny with words.
async fn answer_permission(
    shim: &Arc<Shim>,
    conn: &ConnectionTo<Client>,
    sid: &str,
    f: &serde_json::Value,
) {
    let request_id = f["request_id"].clone();
    let req = &f["request"];
    let tool = req["display_name"]
        .as_str()
        .or_else(|| req["tool_name"].as_str())
        .unwrap_or("tool");
    let title = match req["description"].as_str() {
        Some(d) if !d.is_empty() => format!("{tool}: {d}"),
        _ => tool.to_owned(),
    };
    let ask = RequestPermissionRequest::new(
        sid.to_owned(),
        ToolCallUpdate::new(
            req["tool_use_id"].as_str().unwrap_or("tool").to_owned(),
            ToolCallUpdateFields::new().title(title),
        ),
        vec![
            PermissionOption::new("allow", msg::CAGENT_ALLOW, PermissionOptionKind::AllowOnce),
            PermissionOption::new("deny", msg::CAGENT_DENY, PermissionOptionKind::RejectOnce),
        ],
    );
    let outcome = match conn.send_request(ask).block_task().await {
        Ok(r) => r.outcome,
        Err(_) => RequestPermissionOutcome::Cancelled,
    };
    let allowed = matches!(
        &outcome,
        RequestPermissionOutcome::Selected(s) if s.option_id.0.as_ref() == "allow"
    );
    let response = if allowed {
        serde_json::json!({ "behavior": "allow", "updatedInput": req["input"] })
    } else {
        serde_json::json!({ "behavior": "deny", "message": msg::CAGENT_DENIED })
    };
    let _ = shim.control(&serde_json::json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": response },
    }));
}

/// A replayed utterance in the very shapes the live stream uses — and
/// the same choices `gui-core`'s `history()` makes, so a session's past
/// reads identically whether it was loaded here or read from disk.
fn replayed(u: crate::adaptor::claude::Utterance) -> SessionUpdate {
    use crate::adaptor::claude::Utterance;
    match u {
        Utterance::User(t) => SessionUpdate::UserMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(t)),
        )),
        Utterance::Agent(t) => SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(t)),
        )),
        Utterance::Thought(t) => SessionUpdate::AgentThoughtChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(t)),
        )),
        Utterance::Tool(name) => SessionUpdate::ToolCall(ToolCall::new("replayed", name)),
    }
}

/// 16 random bytes worn as a proper v4 uuid — claude refuses
/// `--session-id` values that are not valid uuids, so the version and
/// variant nibbles are set rather than left to chance.
fn uuid_v4() -> Result<String, String> {
    let hex = crate::link::fresh_hex()?;
    let b: Vec<char> = hex.chars().collect();
    let mut s = String::with_capacity(36);
    for (i, c) in b.iter().enumerate() {
        match i {
            8 | 12 | 16 | 20 => s.push('-'),
            _ => {}
        }
        match i {
            12 => s.push('4'),
            16 => s.push(['8', '9', 'a', 'b'][(c.to_digit(16).unwrap_or(0) % 4) as usize]),
            _ => s.push(*c),
        }
    }
    Ok(s)
}
