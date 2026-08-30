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
    PromptCapabilities, PromptResponse, RequestPermissionOutcome, RequestPermissionRequest,
    SessionNotification,
    Cost, SessionUpdate, StopReason, TextContent, ToolCall, ToolCallStatus, ToolCallUpdate,
    ToolCallUpdateFields, ToolKind, UsageUpdate,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Stdio};
use khor_catalog::msg;
use tokio::sync::{mpsc, oneshot};

/// Where the claude binary is. The default asks PATH, which is the
/// user's own answer; the variable is the test door (a fake claude
/// drives the whole shim hermetically) and the odd-install escape,
/// `KHOR_TMUX`'s precedent.
const CLAUDE_ENV: &str = "KHOR_CLAUDE";

/// Set by the opener when this session is meant to be the network's
/// scheduler (docs/AGENT.md). Env rather than argv for the reason every
/// door in this family is: the `_cagent` convention is one word, and
/// every binary that parses it stays as it is.
pub(crate) const AGENT_ENV: &str = "KHOR_AGENT";

/// One claude child and the turn in flight against it.
struct Shim {
    /// How many tokens fit in this model's context, as **claude last
    /// reported it** — not a table of model names to sizes here.
    ///
    /// A table would be the obvious way and it is the wrong one: it
    /// dates the moment a model ships, it is wrong rather than absent
    /// when it is out of date, and being wrong here draws a gauge that
    /// reads half-full when it is nearly full. claude puts the real
    /// number on the wire (`result.modelUsage[<model>].contextWindow`),
    /// so this is a memory of something measured.
    ///
    /// **`None` until the first turn ends**, and while it is `None` no
    /// usage is reported at all: the protocol's `UsageUpdate` requires a
    /// size, and inventing one to fill the field would put a number on
    /// the screen that no one measured.
    window: Mutex<Option<u64>>,
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
        window: Mutex::new(None),
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
                        .agent_capabilities(
                            AgentCapabilities::new()
                                .load_session(true)
                                // **Advertised because it is now true.**
                                // A capability advertised before it
                                // works is worse than one missing: a
                                // face offers the control and the turn
                                // fails with the picture already sent.
                                .prompt_capabilities(
                                    PromptCapabilities::new().image(true),
                                ),
                        ),
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
                // Images, in the order they were attached. Kept apart
                // from the text rather than interleaved because this
                // 垫片 writes one `user` line and the text is what the
                // person typed — see `Shim::say_with`.
                let images: Vec<(String, String)> = prompt
                    .prompt
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Image(i) => {
                            Some((i.mime_type.clone(), i.data.clone()))
                        }
                        _ => None,
                    })
                    .collect();
                let (tx, rx) = oneshot::channel::<StopReason>();
                {
                    let mut turn = s_prompt.turn.lock().unwrap();
                    if turn.is_some() {
                        return responder.respond_with_internal_error(msg::CAGENT_ONE_TURN);
                    }
                    *turn = Some(tx);
                }
                s_prompt.interrupted.store(false, Ordering::SeqCst);
                if let Err(e) = s_prompt.say_with(&text, &images) {
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
        let sid = crate::adaptor::uuid_v4()?;
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
        .args(khor_tools()?)
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
        self.say_with(text, &[])
    }

    /// One user turn with images, onto claude's stdin.
    ///
    /// **The block shape is claude's own, read off a real transcript
    /// rather than inferred from the docs** (2026-08-30):
    /// `{"type":"image","source":{"type":"base64","media_type":…,
    /// "data":…}}`. The protocol's own field is `mimeType` and claude's
    /// is `media_type`; they are the same fact under two spellings, and
    /// a 垫片 that passed the protocol's spelling through would have
    /// claude reject every image without saying why.
    ///
    /// **Text first.** The person wrote a sentence about a picture, not
    /// a picture about a sentence, and while claude reads either order
    /// only one of them is what was meant.
    fn say_with(&self, text: &str, images: &[(String, String)]) -> Result<(), String> {
        let mut content = vec![serde_json::json!({ "type": "text", "text": text })];
        for (mime, data) in images {
            content.push(serde_json::json!({
                "type": "image",
                "source": { "type": "base64", "media_type": mime, "data": data },
            }));
        }
        self.write_line(&serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": content },
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
                    if let Some(u) = said(block) {
                        let _ = conn.send_notification(SessionNotification::new(sid.clone(), u));
                    }
                }
                // **Live context, but only once the size is known.** The
                // window is learned from a `result` frame, so the very
                // first turn of a session reports nothing until it ends
                // — and that is the honest shape: a `used` with an
                // invented `size` is a gauge, and a gauge is a claim
                // about how much room is left.
                let size = *shim.window.lock().unwrap();
                if let (Some(size), Some(usage)) = (size, f["message"].get("usage")) {
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::UsageUpdate(UsageUpdate::new(tokens_in(usage), size)),
                    ));
                }
            }
            // The other half of a tool call. claude reports a finished
            // tool as a `tool_result` block riding a *user* line — the
            // machine answering itself — so this arm is what turns a
            // call that is drawn as running into one that is drawn as
            // finished. Without it every tool ever called stays
            // `InProgress` on the screen for the life of the
            // conversation, which is the shape the face had before.
            Some("user") => {
                for block in f["message"]["content"].as_array().into_iter().flatten() {
                    if let Some(u) = settled(block) {
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
                // The one frame that carries the window size, so it is
                // both where the size is learned and the only place the
                // first turn of a session can report usage at all.
                if let Some(w) = window_in(&f) {
                    *shim.window.lock().unwrap() = Some(w);
                }
                let size = *shim.window.lock().unwrap();
                if let (Some(size), Some(usage)) = (size, f.get("usage")) {
                    let mut u = UsageUpdate::new(tokens_in(usage), size);
                    if let Some(c) = cost_in(&f) {
                        u = u.cost(c);
                    }
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::UsageUpdate(u),
                    ));
                }
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
/// `pub(crate)`: the codex shim replays through the same mapping, so
/// the two vendors' pasts read alike too.
pub(crate) fn replayed(u: crate::adaptor::claude::Utterance) -> SessionUpdate {
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


#[cfg(test)]
mod wire_tests {
    use agent_client_protocol::schema::v1::{
        AvailableCommand, AvailableCommandsUpdate, SessionUpdate,
    };

    /// **The command list crosses the wire under the names the face
    /// reads.** The conversation pane narrows this JSON by hand
    /// (`ChatView`'s `Update`), so the two ends agree by convention
    /// and nothing else: a rename in the protocol crate would leave
    /// the "/" menu quietly empty, which reads as "this agent has no
    /// commands" rather than as a break.
    #[test]
    fn the_command_list_is_spelled_the_way_the_face_reads_it() {
        let update = SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(vec![
            AvailableCommand::new("compact", "squeeze the context"),
        ]));
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["sessionUpdate"], "available_commands_update");
        assert_eq!(json["availableCommands"][0]["name"], "compact");
        assert_eq!(json["availableCommands"][0]["description"], "squeeze the context");
    }

    /// **A tool call crosses the wire wearing both halves the face
    /// draws with — and the JSON going in is claude's, not ours.**
    /// `said` is fed the shape a real `assistant` line carries, so this
    /// fails if the shim stops setting either field. An earlier version
    /// of this test built the `ToolCall` itself and asserted its
    /// spelling; that assertion holds no matter what the shim does, and
    /// it stayed green through a run with `.kind()`/`.status()` deleted.
    /// Same names asserted, different subject, and the subject was the
    /// whole of the bug.
    #[test]
    fn a_tool_call_is_spelled_the_way_the_face_reads_it() {
        let block = serde_json::json!({
            "type": "tool_use", "id": "toolu_01", "name": "Read",
            "input": { "file_path": "/tmp/x" }
        });
        let json = serde_json::to_value(super::said(&block).expect("an update")).unwrap();
        assert_eq!(json["sessionUpdate"], "tool_call");
        assert_eq!(json["toolCallId"], "toolu_01");
        assert_eq!(json["title"], "Read");
        assert_eq!(json["kind"], "read");
        // Not `pending` — see `said`.
        assert_eq!(json["status"], "in_progress");
    }

    /// **And its closing half, from the line claude actually sends it
    /// on.** A `tool_result` rides a *user* line, which is also where a
    /// person's own words arrive, so this asserts both directions: the
    /// result becomes an update, and the text beside it does not.
    ///
    /// The frame carries a status and no title on purpose — the face
    /// reads absent as "leave it alone", so a frame that spelled an
    /// empty title would blank the line instead of settling it.
    #[test]
    fn a_finished_tool_call_says_only_what_changed() {
        let ok = serde_json::json!({
            "type": "tool_result", "tool_use_id": "toolu_01", "content": "…"
        });
        let json = serde_json::to_value(super::settled(&ok).expect("an update")).unwrap();
        assert_eq!(json["sessionUpdate"], "tool_call_update");
        assert_eq!(json["toolCallId"], "toolu_01");
        assert_eq!(json["status"], "completed");
        assert!(json.get("title").is_none() || json["title"].is_null());

        let bad = serde_json::json!({
            "type": "tool_result", "tool_use_id": "toolu_02",
            "content": "boom", "is_error": true
        });
        let json = serde_json::to_value(super::settled(&bad).expect("an update")).unwrap();
        assert_eq!(json["status"], "failed");

        // A person talking on the same kind of line is not a tool.
        let words = serde_json::json!({ "type": "text", "text": "look at the docs" });
        assert!(super::settled(&words).is_none());
    }

    /// **The numbers are the ones a real claude put on the wire**, not
    /// a shape invented to match the reader. Captured 2026-08-30 from
    /// `claude -p … --output-format stream-json` on this machine and
    /// trimmed, keeping every key the reader touches plus the ones that
    /// would trip a careless sum: `iterations` repeats the same four
    /// numbers a second time inside the object, so a reader that walked
    /// the tree instead of naming its four keys would double the answer.
    #[test]
    fn the_context_is_the_four_piles_added_once() {
        let usage = serde_json::json!({
            "input_tokens": 2,
            "cache_creation_input_tokens": 18707,
            "cache_read_input_tokens": 13413,
            "output_tokens": 4,
            "output_tokens_details": { "thinking_tokens": 0 },
            "cache_creation": { "ephemeral_1h_input_tokens": 18707, "ephemeral_5m_input_tokens": 0 },
            "iterations": [{
                "input_tokens": 2, "output_tokens": 4,
                "cache_read_input_tokens": 13413, "cache_creation_input_tokens": 18707
            }]
        });
        // 2 + 18707 + 13413 + 4. Written out rather than as a sum so
        // that a change to the rule has to change this number by hand.
        assert_eq!(super::tokens_in(&usage), 32126);

        // An empty object is 0, not a panic: claude omits `usage` on
        // some frames and a reader that unwrapped would take the shim
        // down mid-conversation.
        assert_eq!(super::tokens_in(&serde_json::json!({})), 0);
    }

    /// **The window is read off the frame, and the largest one wins.**
    /// A turn that used a subagent on a smaller model carries two
    /// entries; reporting the small one would draw a context with room
    /// left as nearly full, which is the reading that makes somebody
    /// compact a conversation they did not need to.
    #[test]
    fn the_window_comes_from_the_agent_and_the_largest_one_wins() {
        let one = serde_json::json!({
            "type": "result", "subtype": "success", "total_cost_usd": 0.38777300000000003,
            "modelUsage": { "claude-fable-5": {
                "inputTokens": 2, "outputTokens": 4, "costUSD": 0.38777300000000003,
                "contextWindow": 1_000_000, "maxOutputTokens": 64000
            }}
        });
        assert_eq!(super::window_in(&one), Some(1_000_000));
        assert_eq!(super::cost_in(&one).map(|c| c.amount), Some(0.38777300000000003));

        let two = serde_json::json!({ "modelUsage": {
            "big": { "contextWindow": 1_000_000 },
            "small": { "contextWindow": 200_000 },
        }});
        assert_eq!(super::window_in(&two), Some(1_000_000));

        // **No size is `None`, never a default.** The whole reason the
        // shim carries a learned window is that a made-up one draws a
        // gauge nobody measured; a zero would do the same while looking
        // like an answer.
        assert_eq!(super::window_in(&serde_json::json!({ "modelUsage": {} })), None);
        assert_eq!(super::window_in(&serde_json::json!({ "modelUsage": { "m": {} } })), None);
        assert_eq!(super::window_in(&serde_json::json!({ "modelUsage": { "m": { "contextWindow": 0 } } })), None);
        assert_eq!(super::window_in(&serde_json::json!({})), None);
        assert!(super::cost_in(&serde_json::json!({})).is_none());
    }

    /// **Named one by one, not「every name gets a kind」.** That weaker
    /// sentence is true of a function that answers `Other` to
    /// everything, so it would hold while the feature was gone. These
    /// are the mappings themselves, and the last two are the ones that
    /// keep the mapping honest rather than eager: an MCP tool can be
    /// anything and gets `Other`, and `Bash` stays `Execute` even
    /// though claude deletes through it — reading the shell line to
    /// promote it to `Delete` is the guess this deliberately does not
    /// make.
    #[test]
    fn each_claude_tool_lands_on_the_kind_it_actually_is() {
        use agent_client_protocol::schema::v1::ToolKind;
        for (name, want) in [
            ("Read", ToolKind::Read),
            ("Edit", ToolKind::Edit),
            ("Write", ToolKind::Edit),
            ("Bash", ToolKind::Execute),
            ("Grep", ToolKind::Search),
            ("Glob", ToolKind::Search),
            ("WebFetch", ToolKind::Fetch),
            ("Task", ToolKind::Think),
            ("ExitPlanMode", ToolKind::SwitchMode),
            ("mcp__github__create_issue", ToolKind::Other),
            ("rm -rf", ToolKind::Other),
        ] {
            assert_eq!(super::kind_of(name), want, "{name}");
        }
    }
}

/// The flags that turn this session into **the network's scheduler**
/// (docs/AGENT.md), when the opener asked for one.
///
/// khor's verbs arrive as MCP tools (`khor mcp`) rather than as
/// prose the model is told to obey: a tool is something the vendor
/// can gate, name in a permission prompt, and report back — all of
/// which khor already paints. `--strict-mcp-config` is not a
/// detail: without it this session would also inherit whatever MCP
/// servers the user has configured for their own work, which is a
/// different set of powers than the one they agreed to here.
///
/// Off by default. A khor session is an ordinary agent session
/// unless somebody asked for the other thing.
/// One block of a claude `assistant` line, as the update it is.
///
/// **A function rather than a `match` inside the pump, and the reason
/// is a test that was green while the feature was gone.** The first
/// version of the tool-status work asserted the wire spelling by
/// building a `ToolCall` in the test itself — which holds whether or
/// not this shim sets anything, and did: deleting `.status(...)` from
/// the pump left that assertion passing. Translation that only exists
/// inside an `async fn` holding a live connection cannot be asserted
/// on, so it was moved out here, where the claim can be "this JSON
/// produces this update" instead of "this protocol type spells itself
/// this way".
fn said(block: &serde_json::Value) -> Option<SessionUpdate> {
    match block.get("type").and_then(|t| t.as_str()) {
        Some("text") => block["text"].as_str().map(|t| {
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(t.to_owned()),
            )))
        }),
        Some("thinking") => block["thinking"].as_str().map(|t| {
            SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(t.to_owned()),
            )))
        }),
        Some("tool_use") => {
            let id = block["id"].as_str().unwrap_or("tool");
            let name = block["name"].as_str().unwrap_or("tool");
            // **`InProgress`, not the default.** A `ToolCall` built
            // without a status serialises as `pending`, and claude
            // announces a `tool_use` when it is about to run the tool,
            // not when it is queued — so the default would draw a
            // running tool as one that has not started. The other end
            // of this pair is `settled`.
            Some(SessionUpdate::ToolCall(
                ToolCall::new(id.to_owned(), name.to_owned())
                    .kind(kind_of(name))
                    .status(ToolCallStatus::InProgress),
            ))
        }
        _ => None,
    }
}

/// One block of a claude `user` line, as the update it is — which is
/// only ever a tool call closing.
///
/// claude reports a finished tool as a `tool_result` block riding a
/// *user* line: the machine answering itself. Everything else on such a
/// line is a person talking, and this face paints the user's own words
/// first-hand, so nothing else here becomes an update.
fn settled(block: &serde_json::Value) -> Option<SessionUpdate> {
    if block.get("type").and_then(|t| t.as_str()) != Some("tool_result") {
        return None;
    }
    let id = block["tool_use_id"].as_str()?;
    // **Absent `is_error` means success.** claude writes the key only
    // when the tool failed. Of the two ways to be wrong about a missing
    // key, reading it as failure paints every healthy tool as broken,
    // which is the louder and the more expensive one.
    let status = if block["is_error"].as_bool().unwrap_or(false) {
        ToolCallStatus::Failed
    } else {
        ToolCallStatus::Completed
    };
    Some(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        id.to_owned(),
        ToolCallUpdateFields::new().status(status),
    )))
}

/// The tokens sitting in the context after a claude `usage` object.
///
/// **All four are summed, and none of them double-counts.** claude's
/// `input_tokens` excludes the cached part — the opposite of codex,
/// which includes it (`crate::usage` carries that difference and the
/// readings behind it). So input, cache-read, cache-write and output
/// are four disjoint piles, and the context is their sum.
fn tokens_in(usage: &serde_json::Value) -> u64 {
    let n = |k: &str| usage.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
    n("input_tokens")
        + n("cache_read_input_tokens")
        + n("cache_creation_input_tokens")
        + n("output_tokens")
}

/// The context window claude reported on a `result` frame, if it did.
///
/// `modelUsage` is keyed by model name and this takes the largest
/// window in it rather than picking a key: a turn can touch more than
/// one model (a subagent on a smaller one), and of the two ways to be
/// wrong, reporting the small window makes a half-full context look
/// nearly full and would have people compacting a conversation that has
/// room. Same-model turns — every ordinary one — have a single entry
/// and the max is that entry.
fn window_in(f: &serde_json::Value) -> Option<u64> {
    let models = f.get("modelUsage")?.as_object()?;
    models
        .values()
        .filter_map(|m| m.get("contextWindow").and_then(serde_json::Value::as_u64))
        .max()
        .filter(|w| *w > 0)
}

/// What a turn's `result` frame says was spent on it, in USD.
///
/// **Cumulative for the session, not for the turn** — claude's own
/// field is `total_cost_usd`, and the protocol's `Cost` says the same
/// ("total cumulative cost for session"), so the two agree and nothing
/// here has to add anything up.
fn cost_in(f: &serde_json::Value) -> Option<Cost> {
    let amount = f.get("total_cost_usd").and_then(serde_json::Value::as_f64)?;
    Some(Cost::new(amount, "USD"))
}

/// Which of the protocol's tool kinds a claude tool name is.
///
/// **The kind is what a face can draw before it knows anything else.**
/// A title is a string and reads as one; a kind is a small closed set,
/// so an icon or a shape can stand for it and a reader can tell "it
/// read a file" from "it ran something" without parsing words.
///
/// **What this cannot do is be right about tools it has never seen.**
/// The names below are claude's own; an MCP tool arrives as
/// `mcp__<server>__<tool>` and could be anything, so it lands in
/// `Other` — which is the honest answer, not a gap. The protocol has
/// `Other` for exactly this, and a guess from substrings ("everything
/// containing `read`") would produce confident wrong icons on other
/// people's tools, which is worse than a plain one.
///
/// **`Delete` and `Move` are deliberately unreachable here.** claude
/// deletes and moves through `Bash`, so those two kinds would be a
/// claim about the *contents* of a shell line, which this shim does not
/// read. They stay `Execute`, and a `rm` shows up as something that ran
/// rather than as a deletion. Said out loud because the alternative —
/// grepping the command — is the kind of thing that looks like an
/// improvement right up until it mislabels `grep -r delete`.
fn kind_of(name: &str) -> ToolKind {
    match name {
        "Read" | "NotebookRead" => ToolKind::Read,
        "Write" | "Edit" | "NotebookEdit" => ToolKind::Edit,
        "Bash" | "BashOutput" | "KillShell" => ToolKind::Execute,
        "Glob" | "Grep" => ToolKind::Search,
        "WebFetch" | "WebSearch" => ToolKind::Fetch,
        "Task" => ToolKind::Think,
        "ExitPlanMode" => ToolKind::SwitchMode,
        _ => ToolKind::Other,
    }
}

fn khor_tools() -> Result<Vec<String>, String> {
    if std::env::var(AGENT_ENV).is_err() {
        return Ok(Vec::new());
    }
    let exe = crate::self_exe()?;
    let root = crate::Node::root_from_env();
    let config = serde_json::json!({
        "mcpServers": {
            "khor": {
                "command": exe.display().to_string(),
                "args": ["mcp"],
                // The store is named rather than inherited: this
                // child is spawned by a host that may itself have
                // been started with another one, and a scheduler
                // pointed at the wrong network is worse than one
                // with no tools at all.
                "env": { "KHOR_HOME": root.display().to_string() }
            }
        }
    });
    let path = std::env::temp_dir().join(format!("khor-mcp-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&config).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut args = vec![
        "--mcp-config".into(),
        path.display().to_string(),
        "--strict-mcp-config".into(),
        "--append-system-prompt".into(),
        msg::AGENT_BRIEF.into(),
    ];
    // 读免批 (docs/AGENT.md). Looking at the network is not an act, and
    // asking permission for it teaches a person to press 允许 without
    // reading — which is the same as having no gate on the ones that
    // matter. The names come from the tool table itself, so a tool
    // added on the reading side is free the day it lands and one added
    // on the acting side is gated the day it lands.
    args.push("--allowedTools".into());
    args.extend(crate::mcp::free_tools());
    Ok(args)
}
