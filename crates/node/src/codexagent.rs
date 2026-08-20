//! khor's own codex shim: an ACP agent over stdio whose brain is the
//! user's `codex` binary speaking its app-server JSON-RPC protocol.
//!
//! # Why this exists (批8; `cagent`'s reasons, second vendor)
//!
//! `khor open --gui` hosts *an ACP agent* (`gui_host`). Codex does not
//! speak ACP, and the ecosystem's adapters are npm packages — the same
//! runtime dependency 零依赖 (docs/KHOR.md) ruled out for claude. But
//! everything an adapter would do, codex's own `app-server` already
//! offers over stdio, and khor already reads codex's rollout files. So
//! the shim is khor's: `khor _codexagent` serves ACP on stdio and holds
//! one `codex app-server` child underneath. `gui_host`, `crates/acp`,
//! and every face stay byte-for-byte unchanged — to them this is just
//! an agent.
//!
//! # The probed facts this is built on (2026-08-20, codex-cli 0.146.1)
//!
//! - `codex app-server` is a long-lived JSON-RPC-over-stdio server:
//!   one process answers many turns. `initialize` → `initialized` →
//!   `thread/start` answers the thread's uuid at once, so
//!   `session/new` responds immediately (`--session-id`'s precedent on
//!   the claude side) — and that uuid is also the rollout file's name,
//!   which is what keeps 同源 (one conversation, one id) on this
//!   channel.
//! - `thread/resume { threadId }` in a fresh process continues the
//!   same thread: probed by asking the resumed thread for a string
//!   said before the restart, and getting it back verbatim.
//! - Approvals arrive as server→client JSON-RPC **requests**
//!   (`item/commandExecution/requestApproval`,
//!   `item/fileChange/requestApproval`), answered on the same id by
//!   `{"decision":"accept"|"decline"}`. Decline really blocks — the
//!   probe's file never appeared — and the turn continues.
//! - Every turn ends in exactly one `turn/completed`, whose
//!   `turn.status` distinguishes completed from interrupted; probed
//!   for the interrupt path with `turn/interrupt { threadId, turnId }`.
//!
//! # What is deliberately not translated
//!
//! The same judgment as `cagent`: only what a face paints — message
//! and thought chunks, tool calls, the permission ask, and the stop
//! reason. Codex's hook runs, MCP startup notices, rate-limit feeds
//! and skills bookkeeping stay its own. Codex has no slash-command
//! list, so no `available_commands_update` rides the first turn — an
//! empty "/" menu is the honest answer.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    LoadSessionRequest, LoadSessionResponse, NewSessionRequest, NewSessionResponse,
    PermissionOption, PermissionOptionKind, PromptRequest, PromptResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionNotification, SessionUpdate,
    StopReason, TextContent, ToolCall, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::{Agent, Client, ConnectionTo, Stdio};
use khor_catalog::msg;
use tokio::sync::{mpsc, oneshot};

/// Where the codex binary is. The default asks PATH; the variable is
/// the test door (a fake app-server drives the whole shim hermetically)
/// and the odd-install escape — `KHOR_CLAUDE`'s precedent.
const CODEX_ENV: &str = "KHOR_CODEX";

/// How long any single JSON-RPC answer may take. The answers awaited
/// here are all server-side bookkeeping (initialize, thread create,
/// turn create) — none waits on a model — so a minute of silence is a
/// wedged child, not a slow turn.
const ANSWER_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);

/// One codex child and the turn in flight against it.
struct Shim {
    root: PathBuf,
    /// Server→client traffic (notifications and approval requests);
    /// `drain` on the runtime picks them up. Answers to the shim's own
    /// requests never land here — the reader routes those to
    /// [`Shim::pending`].
    frames: mpsc::UnboundedSender<serde_json::Value>,
    stdin: Mutex<Option<std::process::ChildStdin>>,
    child_pid: Mutex<Option<u32>>,
    /// The thread id — codex's own uuid, which is also the session id
    /// every ACP frame wears.
    session: Mutex<Option<String>>,
    /// The pending `session/prompt`, resolved when `turn/completed`
    /// arrives. One at a time (`cagent`'s reason).
    turn: Mutex<Option<oneshot::Sender<StopReason>>>,
    /// The running turn's id — `turn/interrupt` must name it.
    turn_id: Mutex<Option<String>>,
    /// A cancel was sent for the current turn; its completion, whatever
    /// its status, is the cancellation the client asked for.
    interrupted: AtomicBool,
    /// Answers the reader owes to requests the shim sent, by id.
    pending: Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>,
    next_id: AtomicU64,
}

/// The body of `khor _codexagent` (and of `_codexagent` in every
/// host-capable binary — `host::main_if_host`).
pub fn codexagent_main(root: PathBuf) -> Result<(), String> {
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
        turn_id: Mutex::new(None),
        interrupted: AtomicBool::new(false),
        pending: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(1),
    });
    let s_new = shim.clone();
    let s_prompt = shim.clone();
    let s_load = shim.clone();
    let s_cancel = shim.clone();
    let s_drain = shim.clone();

    Agent
        .builder()
        .name("khor-codexagent")
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
                match s_new.start_codex(&new.cwd).await {
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
                *s_prompt.turn_id.lock().unwrap() = None;
                let shim = s_prompt.clone();
                // The wait lives in a spawned task: the event loop must
                // keep moving updates while the turn runs. The turn's
                // own creation is awaited in the same task — its answer
                // arrives off the reader thread, not off this loop.
                connection.spawn(async move {
                    let sid = shim.session.lock().unwrap().clone().unwrap_or_default();
                    let started = shim
                        .request(
                            "turn/start",
                            serde_json::json!({
                                "threadId": sid,
                                "input": [{ "type": "text", "text": text }],
                            }),
                        )
                        .await;
                    match started {
                        Ok(v) => {
                            let tid = v["turn"]["id"].as_str().unwrap_or_default().to_owned();
                            *shim.turn_id.lock().unwrap() = Some(tid.clone());
                            // A cancel that raced the creation: honour
                            // it now that the turn has a name.
                            if shim.interrupted.load(Ordering::SeqCst) {
                                shim.interrupt(&sid, &tid);
                            }
                        }
                        Err(e) => {
                            if let Some(tx) = shim.turn.lock().unwrap().take() {
                                let _ = tx.send(StopReason::Refusal);
                            }
                            let _ = e;
                        }
                    }
                    let reason = rx.await.unwrap_or(StopReason::Cancelled);
                    responder.respond(PromptResponse::new(reason))
                })
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |load: LoadSessionRequest, responder, connection| {
                let sid = load.session_id.0.to_string();
                let started = s_load.session.lock().unwrap().clone();
                match started {
                    Some(cur) if cur == sid => {}
                    Some(_) => {
                        return responder.respond_with_internal_error(msg::CAGENT_ONE_SESSION)
                    }
                    None => {
                        // The takeover: resume the recorded thread. Its
                        // world (cwd) is the rollout's own record, not
                        // the loader's — the resumer may stand anywhere.
                        let home = crate::adaptor::vendor_home(&s_load.root).join(".codex");
                        let cwd = crate::adaptor::codex::Codex::at(home)
                            .recorded_cwd(&sid)
                            .unwrap_or_else(|| load.cwd.clone());
                        if let Err(e) = s_load.start_codex_resume(&cwd, &sid).await {
                            return responder.respond_with_internal_error(e);
                        }
                    }
                }
                // Replay is answered from the rollout — the same file
                // discovery reads, so both pasts read alike.
                let home = crate::adaptor::vendor_home(&s_load.root).join(".codex");
                let said = crate::adaptor::codex::Codex::at(home)
                    .transcript(&sid)
                    .unwrap_or_default();
                for u in said {
                    let update = crate::cagent::replayed(u);
                    let _ = connection
                        .send_notification(SessionNotification::new(sid.clone(), update));
                }
                responder.respond(LoadSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |_cancel: agent_client_protocol::schema::v1::CancelNotification, _cx| {
                // `turn/interrupt` ends the running turn with a
                // `turn/completed` whose status says interrupted
                // (probed); the flag is the belt, `cagent`'s reason:
                // however the turn ends now, it reads as the
                // cancellation the client asked for (取消 is 空闲;
                // 拒绝 is 中断).
                s_cancel.interrupted.store(true, Ordering::SeqCst);
                let sid = s_cancel.session.lock().unwrap().clone();
                let tid = s_cancel.turn_id.lock().unwrap().clone();
                if let (Some(sid), Some(tid)) = (sid, tid) {
                    s_cancel.interrupt(&sid, &tid);
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(Stdio::new(), move |connection: ConnectionTo<Client>| async move {
            // The client's EOF ends the shim (`cagent`'s arm, same
            // orphan discipline).
            tokio::select! {
                _ = drain(s_drain, frames_rx, &connection) => {}
                _ = connection.incoming_closed() => {}
            }
            Ok(())
        })
        .await
        .map(|_| ())?;
    // However serve ended, the codex child must not outlive the shim.
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
    /// Spawns the codex child and opens a fresh thread; the thread's
    /// own uuid is the session id.
    async fn start_codex(self: &Arc<Self>, cwd: &std::path::Path) -> Result<String, String> {
        self.start_appserver(cwd)?;
        self.handshake().await?;
        let mut params = serde_json::json!({ "cwd": cwd.display().to_string() });
        scheduler_into(&mut params)?;
        let started = self.request("thread/start", params).await?;
        let sid = started["thread"]["id"]
            .as_str()
            .ok_or_else(|| msg::codexagent_bad_answer("thread/start"))?
            .to_owned();
        *self.session.lock().unwrap() = Some(sid.clone());
        Ok(sid)
    }

    /// Spawns the codex child **resuming** an existing thread — the
    /// takeover path: the conversation already exists on disk, and
    /// `thread/resume` continues the same id into the same rollout
    /// (probed: it answered with words from before the restart).
    async fn start_codex_resume(
        self: &Arc<Self>,
        cwd: &std::path::Path,
        sid: &str,
    ) -> Result<(), String> {
        self.start_appserver(cwd)?;
        self.handshake().await?;
        let mut params =
            serde_json::json!({ "threadId": sid, "cwd": cwd.display().to_string() });
        scheduler_into(&mut params)?;
        let resumed = self.request("thread/resume", params).await?;
        if resumed["thread"]["id"].as_str() != Some(sid) {
            return Err(msg::codexagent_bad_answer("thread/resume"));
        }
        *self.session.lock().unwrap() = Some(sid.to_owned());
        Ok(())
    }

    fn start_appserver(self: &Arc<Self>, cwd: &std::path::Path) -> Result<(), String> {
        if self.session.lock().unwrap().is_some() {
            return Err(msg::CAGENT_ONE_SESSION.into());
        }
        let bin = std::env::var(CODEX_ENV).unwrap_or_else(|_| "codex".into());
        let mut c = std::process::Command::new(&bin);
        c.arg("app-server")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let mut child = c.spawn().map_err(|e| msg::wont_start(&bin, e))?;
        let stdout = child.stdout.take().ok_or(msg::CODEXAGENT_NO_PIPE)?;
        *self.child_pid.lock().unwrap() = Some(child.id());
        *self.stdin.lock().unwrap() = child.stdin.take();
        let shim = self.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
                // An id without a method is an answer to something the
                // shim asked; everything else is the server speaking.
                let answer_to = v
                    .get("method")
                    .is_none()
                    .then(|| v.get("id").and_then(|i| i.as_u64()))
                    .flatten();
                match answer_to {
                    Some(id) => {
                        if let Some(tx) = shim.pending.lock().unwrap().remove(&id) {
                            let _ = tx.send(v);
                        }
                    }
                    None => {
                        if shim.frames.send(v).is_err() {
                            break;
                        }
                    }
                }
            }
            // The child ended; a pending turn must not hang forever.
            let _ = shim.frames.send(serde_json::json!({ "khor_child_gone": true }));
        });
        Ok(())
    }

    /// The app-server hello: versioned like every JSON-RPC hello, and
    /// nothing moves before `initialized`.
    async fn handshake(self: &Arc<Self>) -> Result<(), String> {
        self.request(
            "initialize",
            serde_json::json!({
                "clientInfo": { "name": "khor", "title": "khor", "version": env!("CARGO_PKG_VERSION") }
            }),
        )
        .await?;
        self.notify("initialized", serde_json::Value::Null)
    }

    /// One request onto codex's stdin, one answer off the reader.
    async fn request(
        self: &Arc<Self>,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        self.write_line(&serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params,
        }))?;
        let answer = tokio::time::timeout(ANSWER_BUDGET, rx)
            .await
            .map_err(|_| msg::codexagent_bad_answer(method))?
            .map_err(|_| msg::codexagent_bad_answer(method))?;
        if let Some(e) = answer.get("error") {
            return Err(e["message"].as_str().unwrap_or("error").to_owned());
        }
        Ok(answer["result"].clone())
    }

    fn notify(&self, method: &str, params: serde_json::Value) -> Result<(), String> {
        let mut v = serde_json::json!({ "jsonrpc": "2.0", "method": method });
        if !params.is_null() {
            v["params"] = params;
        }
        self.write_line(&v)
    }

    /// Fire-and-forget `turn/interrupt`; its empty answer is routed to
    /// a throwaway waiter by the reader.
    fn interrupt(self: &Arc<Self>, sid: &str, tid: &str) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, _rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let _ = self.write_line(&serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "turn/interrupt",
            "params": { "threadId": sid, "turnId": tid },
        }));
    }

    fn write_line(&self, v: &serde_json::Value) -> Result<(), String> {
        let mut guard = self.stdin.lock().unwrap();
        let stdin = guard.as_mut().ok_or(msg::CAGENT_NOT_STARTED)?;
        writeln!(stdin, "{v}").map_err(|e| e.to_string())
    }
}

/// The 调度员 half (docs/AGENT.md), codex spelling: when the opener set
/// `KHOR_AGENT`, khor's verbs reach the session as an MCP server
/// injected through `thread/start`'s config override (probed
/// 2026-08-20: the khor server came up "ready" and the model listed
/// `mcp__khor__*` by name), and the brief rides `developerInstructions`
/// (`--append-system-prompt`'s twin — additive, unlike
/// `baseInstructions` which replaces codex's own).
///
/// One honest difference from the claude shim: codex has no
/// `--strict-mcp-config` twin — the override MERGES into the servers
/// the user configured in their own config.toml rather than replacing
/// them (probed: the user's other servers loaded beside khor's). Those
/// are servers the user pointed at codex sessions themselves, so a
/// khor-opened codex session wields what their own codex wields; the
/// claude strictness guards a different door.
fn scheduler_into(params: &mut serde_json::Value) -> Result<(), String> {
    if std::env::var(crate::cagent::AGENT_ENV).is_err() {
        return Ok(());
    }
    let exe = crate::self_exe()?;
    let root = crate::Node::root_from_env();
    params["config"] = serde_json::json!({
        "mcp_servers": {
            "khor": {
                "command": exe.display().to_string(),
                "args": ["mcp"],
                // The store is named rather than inherited (`cagent`'s
                // `khor_tools` has why: a scheduler pointed at the
                // wrong network is worse than one with no tools).
                "env": { "KHOR_HOME": root.display().to_string() }
            }
        }
    });
    params["developerInstructions"] = serde_json::Value::String(msg::AGENT_BRIEF.to_owned());
    Ok(())
}

/// The codex→ACP pump: every server frame, routed. The permission
/// round-trip is awaited inline on purpose — codex itself is paused on
/// that very answer.
async fn drain(
    shim: Arc<Shim>,
    mut frames: mpsc::UnboundedReceiver<serde_json::Value>,
    conn: &ConnectionTo<Client>,
) {
    while let Some(f) = frames.recv().await {
        if f.get("khor_child_gone").is_some() {
            if let Some(tx) = shim.turn.lock().unwrap().take() {
                let _ = tx.send(StopReason::Refusal);
            }
            break;
        }
        let sid = match shim.session.lock().unwrap().clone() {
            Some(s) => s,
            None => continue,
        };
        let method = f.get("method").and_then(|m| m.as_str()).unwrap_or("");
        match method {
            "item/agentMessage/delta" => {
                if let Some(t) = f["params"]["delta"].as_str() {
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                            TextContent::new(t.to_owned()),
                        ))),
                    ));
                }
            }
            "item/reasoning/summaryTextDelta" | "item/reasoning/textDelta" => {
                if let Some(t) = f["params"]["delta"].as_str() {
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(
                            TextContent::new(t.to_owned()),
                        ))),
                    ));
                }
            }
            "item/started" => {
                let item = &f["params"]["item"];
                let kind = item["type"].as_str().unwrap_or("");
                // Conversation items already stream as deltas; what a
                // face paints as a tool call is everything else that
                // acts.
                if matches!(kind, "userMessage" | "agentMessage" | "reasoning" | "") {
                    continue;
                }
                let id = item["id"].as_str().unwrap_or("tool");
                let name = item["command"]
                    .as_str()
                    .or_else(|| item["title"].as_str())
                    .or_else(|| item["name"].as_str())
                    .unwrap_or(kind);
                let _ = conn.send_notification(SessionNotification::new(
                    sid.clone(),
                    SessionUpdate::ToolCall(ToolCall::new(id.to_owned(), name.to_owned())),
                ));
            }
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
                answer_permission(&shim, conn, &sid, &f).await;
            }
            "turn/completed" => {
                let status = f["params"]["turn"]["status"].as_str().unwrap_or("");
                let reason = if shim.interrupted.swap(false, Ordering::SeqCst)
                    || status == "interrupted"
                {
                    StopReason::Cancelled
                } else if status == "completed" {
                    StopReason::EndTurn
                } else {
                    StopReason::Refusal
                };
                *shim.turn_id.lock().unwrap() = None;
                if let Some(tx) = shim.turn.lock().unwrap().take() {
                    let _ = tx.send(reason);
                }
            }
            _ => {}
        }
    }
}

/// One approval request → one `session/request_permission` → one
/// JSON-RPC answer on the server's own id. Allow is accept; everything
/// else — deny, dismiss, a client that went away — is decline (the
/// probed "agent continues the turn" kind, not the interrupting kind).
async fn answer_permission(
    shim: &Arc<Shim>,
    conn: &ConnectionTo<Client>,
    sid: &str,
    f: &serde_json::Value,
) {
    let request_id = f["id"].clone();
    let p = &f["params"];
    let title = p["reason"]
        .as_str()
        .or_else(|| p["command"].as_str())
        .unwrap_or("codex")
        .to_owned();
    let ask = RequestPermissionRequest::new(
        sid.to_owned(),
        ToolCallUpdate::new(
            p["itemId"].as_str().unwrap_or("tool").to_owned(),
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
    let decision = if allowed { "accept" } else { "decline" };
    let _ = shim.write_line(&serde_json::json!({
        "jsonrpc": "2.0", "id": request_id, "result": { "decision": decision },
    }));
}
