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

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    AgentCapabilities, AvailableCommand, AvailableCommandsUpdate, Content, ContentBlock,
    ContentChunk, Cost, CurrentModeUpdate, Diff, EmbeddedResourceResource, InitializeRequest,
    InitializeResponse, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,
    NewSessionResponse, PermissionOption, PermissionOptionKind, PromptCapabilities, PromptRequest,
    PromptResponse, RequestPermissionOutcome, RequestPermissionRequest, SessionMode,
    SessionInfoUpdate, SessionModeState, SessionNotification, SessionUpdate, SetSessionModeRequest,
    SetSessionModeResponse, StopReason, TextContent, ToolCall, ToolCallContent, ToolCallLocation,
    ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind, UsageUpdate,
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
    /// A fresh, unprompted session may not have a transcript yet.
    history_required: AtomicBool,
    /// Control requests this shim sent and is waiting on, by request
    /// id: claude answers each with a `control_response` naming the id,
    /// and `drain` hands the answer here. The interrupt is not in this
    /// map — nothing waits on it, the turn's own `result` is its answer.
    pending: Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>,
    /// The next control request's number, so two faces changing the
    /// mode at once get two ids rather than one answer for both.
    asked: AtomicU64,
}

/// The permission modes this shim offers, in claude's own spelling
/// (`--permission-mode`'s values, and what its init frame reports as
/// `permissionMode`). Four of claude's six: `dontAsk` and `auto` are
/// real but khor's face has no words for what they will do, and a mode
/// a person cannot read is not one to offer them.
const MODES: [&str; 4] = ["default", "plan", "acceptEdits", "bypassPermissions"];

/// The list a session opens with. `current` is what this shim
/// launched claude in — no flag, so claude's own default — and the
/// init frame's `permissionMode` corrects it the moment claude speaks
/// (`drain`, `current_mode_update`), which matters for a resumed
/// session that was left in another mode.
fn modes_listed(current: &str) -> SessionModeState {
    SessionModeState::new(
        current.to_owned(),
        MODES.iter().map(|m| SessionMode::new(*m, *m)).collect(),
    )
}

/// The control request that moves claude to another mode. The spelling
/// is claude's (measured off the 2.1.252 bundle: `subtype:
/// "set_permission_mode", mode: <id>`), and the test below pins it —
/// a misspelt subtype is not an error, it is a request claude ignores.
fn mode_request(request_id: &str, mode: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "control_request",
        "request_id": request_id,
        "request": { "subtype": "set_permission_mode", "mode": mode },
    })
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
        history_required: AtomicBool::new(false),
        window: Mutex::new(None),
        pending: Mutex::new(HashMap::new()),
        asked: AtomicU64::new(0),
    });
    let s_mode = shim.clone();
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
                    InitializeResponse::new(initialize.protocol_version).agent_capabilities(
                        AgentCapabilities::new()
                            .load_session(true)
                            // **Advertised because it is now true.**
                            // A capability advertised before it
                            // works is worse than one missing: a
                            // face offers the control and the turn
                            // fails with the picture already sent.
                            .prompt_capabilities(
                                PromptCapabilities::new()
                                    .image(true)
                                    // A file rides along as text —
                                    // see the prompt handler for
                                    // the translation and why it is
                                    // written down.
                                    .embedded_context(true),
                            ),
                    ),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |new: NewSessionRequest, responder, _connection| match s_new
                .start_claude(&new.cwd)
            {
                Ok(sid) => {
                    let mut opened = NewSessionResponse::new(sid);
                    opened.modes = Some(modes_listed("default"));
                    responder.respond(opened)
                }
                Err(e) => responder.respond_with_internal_error(e),
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |prompt: PromptRequest, responder, connection| {
                s_prompt.history_required.store(true, Ordering::SeqCst);
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
                        ContentBlock::Image(i) => Some((i.mime_type.clone(), i.data.clone())),
                        _ => None,
                    })
                    .collect();
                // **A translation, not a passthrough — say so.** The
                // protocol has an embedded resource; claude's
                // stream-json has `text`, `image`, `tool_result` and
                // `document`, and nothing shaped like a named file of
                // text. So it is inlined as text, fenced and labelled
                // with its uri, which is what a person pasting a file
                // into a chat does by hand anyway.
                //
                // Written out rather than done quietly because the
                // fence is *this shim's invention*: an agent reading it
                // sees markup khor chose, and the next person to wonder
                // where it came from should find the answer here.
                let files: Vec<String> = prompt
                    .prompt
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Resource(r) => match &r.resource {
                            EmbeddedResourceResource::TextResourceContents(t) => {
                                Some(format!("<file uri=\"{}\">\n{}\n</file>", t.uri, t.text))
                            }
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect();
                let text = if files.is_empty() {
                    text
                } else {
                    format!("{}\n\n{}", files.join("\n\n"), text)
                };
                let (tx, rx) = oneshot::channel::<StopReason>();
                {
                    let mut turn = s_prompt.turn.lock().unwrap();
                    if turn.is_some() {
                        return responder.respond_with_internal_error(msg::cagent_one_turn());
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
                        return responder.respond_with_internal_error(msg::cagent_one_session());
                    }
                    None => {
                        s_load.history_required.store(true, Ordering::SeqCst);
                        // A load before any new is the takeover: resume
                        // the recorded conversation. Its world (cwd) is
                        // the transcript's own record, not the loader's
                        // — the resumer may stand anywhere.
                        let home = crate::adaptor::claude_home(&s_load.root);
                        let cwd = crate::adaptor::claude::Claude::at(home)
                            .recorded_cwd(&crate::live::clean_leaf(&sid))
                            .unwrap_or_else(|| load.cwd.clone());
                        if let Err(e) = s_load.start_claude_resume(&cwd, &sid) {
                            return responder.respond_with_internal_error(e);
                        }
                    }
                }
                let home = crate::adaptor::claude_home(&s_load.root);
                let said = match crate::adaptor::claude::Claude::at(home)
                    .transcript(&crate::live::clean_leaf(&sid)) {
                    Ok(said) => said,
                    Err(_) if !s_load.history_required.load(Ordering::SeqCst) => Vec::new(),
                    Err(error) => return responder.respond_with_internal_error(error),
                };
                for u in said {
                    // `None` for a recorded block with nothing to
                    // paint; skipped rather than turned into an empty
                    // line about a tool nobody can see.
                    let Some(update) = replayed(u) else { continue };
                    let _ =
                        connection.send_notification(SessionNotification::new(sid.clone(), update));
                }
                let mut loaded = LoadSessionResponse::new();
                loaded.modes = Some(modes_listed("default"));
                responder.respond(loaded)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |set: SetSessionModeRequest, responder, connection| {
                let mode = set.mode_id.to_string();
                if !MODES.contains(&mode.as_str()) {
                    return responder.respond_with_internal_error(msg::cagent_unknown_mode(&mode));
                }
                let request_id =
                    format!("khor-mode-{}", s_mode.asked.fetch_add(1, Ordering::SeqCst));
                let (tx, rx) = oneshot::channel::<serde_json::Value>();
                s_mode
                    .pending
                    .lock()
                    .unwrap()
                    .insert(request_id.clone(), tx);
                if let Err(e) = s_mode.control(&mode_request(&request_id, &mode)) {
                    s_mode.pending.lock().unwrap().remove(&request_id);
                    return responder.respond_with_internal_error(e);
                }
                // Answered when claude answers — in a task, so the
                // pump keeps moving frames meanwhile (the prompt
                // handler's judgment). **Success is claude's word, not
                // this shim's**: the `current_mode_update` below goes
                // out only on its `success`, because a mode drawn as
                // changed on the strength of having asked is the
                // marker lying.
                let shim = s_mode.clone();
                let sid = set.session_id.clone();
                let inner = connection.clone();
                connection.spawn(async move {
                    let answer = tokio::time::timeout(std::time::Duration::from_secs(10), rx).await;
                    match answer {
                        Ok(Ok(r)) if r["subtype"].as_str() == Some("success") => {
                            let _ = inner.send_notification(SessionNotification::new(
                                sid,
                                SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(mode)),
                            ));
                            responder.respond(SetSessionModeResponse::new())
                        }
                        Ok(Ok(r)) => responder.respond_with_internal_error(
                            r["error"]
                                .as_str()
                                .unwrap_or(msg::cagent_mode_refused())
                                .to_owned(),
                        ),
                        _ => {
                            shim.pending.lock().unwrap().remove(&request_id);
                            responder.respond_with_internal_error(msg::cagent_mode_unanswered())
                        }
                    }
                })
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
        .connect_with(
            Stdio::new(),
            move |connection: ConnectionTo<Client>| async move {
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
            },
        )
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
    fn start_claude_resume(
        self: &Arc<Self>,
        cwd: &std::path::Path,
        sid: &str,
    ) -> Result<(), String> {
        self.start_claude_with(cwd, &["--resume", sid])?;
        *self.session.lock().unwrap() = Some(sid.to_owned());
        Ok(())
    }

    fn start_claude_with(
        self: &Arc<Self>,
        cwd: &std::path::Path,
        tail: &[&str],
    ) -> Result<(), String> {
        if self.session.lock().unwrap().is_some() {
            return Err(msg::cagent_one_session().into());
        }
        let named = std::env::var(CLAUDE_ENV).unwrap_or_else(|_| "claude".into());
        // Looked up by khor rather than left to the spawn, so a missing
        // vendor binary is something khor reports by name and by where
        // it searched — not an `ENOENT` a reader has to guess at
        // (`crate::spawn_path`, and the launchd `PATH` it exists for).
        let path = crate::spawn_path(&named);
        let bin = crate::find_program(&named, &path)
            .ok_or_else(|| msg::command_not_found(&named, &path))?;
        let mut c = std::process::Command::new(&bin);
        c.env("PATH", &path);
        c.env("CLAUDE_CONFIG_DIR", crate::adaptor::claude_home(&self.root));
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
        let stdout = child.stdout.take().ok_or(msg::cagent_no_pipe())?;
        *self.child_pid.lock().unwrap() = Some(child.id());
        *self.stdin.lock().unwrap() = child.stdin.take();
        let frames = self.frames.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
                    && frames.send(v).is_err()
                {
                    break;
                }
            }
            // The child ended; a pending turn must not hang forever.
            let _ = frames.send(serde_json::json!({ "type": "khor_child_gone" }));
        });
        Ok(())
    }

    /// One user turn onto claude's stdin.
    // No caller today: every turn so far carries the images argument.
    // Kept rather than deleted because it is the no-images half of a
    // pair, and deleting it is a change to this shim's shape, not a
    // lint fix.
    #[allow(dead_code)]
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
        let stdin = guard.as_mut().ok_or(msg::cagent_not_started())?;
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
                // **The mode the agent is actually in.** claude puts
                // `permissionMode` on its init frame and it is the real
                // one — launched with `--permission-mode plan` it comes
                // back `plan` (measured 2026-08-30). It changes what
                // the agent will *do*, and until now a face had no way
                // to know: a session that will not edit anything and
                // one that edits without asking looked identical.
                if let Some(mode) = f["permissionMode"].as_str().filter(|m| !m.is_empty()) {
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(mode.to_owned())),
                    ));
                }
                // **The model, the moment claude names it.** The init
                // frame carries the one it runs on; nothing before the
                // first turn does (claude starts on the first input).
                // `_meta` is where it rides (`khor_acp::MODEL_META`): ACP
                // has no stable field for a model.
                if let Some(model) = f["model"].as_str().filter(|m| !m.is_empty()) {
                    let _ = conn.send_notification(SessionNotification::new(
                        sid.clone(),
                        SessionUpdate::SessionInfoUpdate(
                            SessionInfoUpdate::new().meta(khor_acp::model_meta(model)),
                        ),
                    ));
                }
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
            Some("control_request") if f["request"]["subtype"].as_str() == Some("can_use_tool") => {
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
            // claude's answer to a control request this shim sent —
            // handed to whoever is waiting on that id (the mode
            // change). One never asked for, or one whose asker gave up,
            // is dropped here rather than logged: there is nobody to
            // tell.
            Some("control_response") => {
                let answer = &f["response"];
                if let Some(id) = answer["request_id"].as_str()
                    && let Some(tx) = shim.pending.lock().unwrap().remove(id)
                {
                    let _ = tx.send(answer.clone());
                }
            }
            Some("khor_child_gone") => {
                if let Some(tx) = shim.turn.lock().unwrap().take() {
                    let _ = tx.send(StopReason::Refusal);
                }
                // Every waiting control request learns the same way the
                // turn does: the sender goes, the waiter reads an
                // ending rather than waiting out its clock.
                shim.pending.lock().unwrap().clear();
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
            PermissionOption::new(
                "allow",
                msg::cagent_allow(),
                PermissionOptionKind::AllowOnce,
            ),
            PermissionOption::new("deny", msg::cagent_deny(), PermissionOptionKind::RejectOnce),
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
        serde_json::json!({ "behavior": "deny", "message": msg::cagent_denied() })
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
///
/// **Tool calls now go through the very same [`said`] and [`settled`]
/// the live stream does, and that is the whole point of this shape.**
/// They used to have their own answer here — `ToolCall::new("replayed",
/// name)` — and it drifted the moment the live path was improved: the
/// live one grew titles out of the tool's input, kinds, statuses,
/// diffs and outputs, while every *replayed* conversation went on
/// painting line after line reading `Bash`, each wearing the same id
/// `"replayed"`, so no update could ever match one either.
///
/// **That is what a person actually looks at**: almost everything on
/// screen in an existing conversation is replayed, so a fix to the live
/// path alone changes nothing they can see. A conversation read from
/// the record and the same conversation watched live are the same
/// conversation, and one translation is the only way that stays true.
pub fn replayed(u: crate::adaptor::claude::Utterance) -> Option<SessionUpdate> {
    use crate::adaptor::claude::Utterance;
    match u {
        Utterance::User(t) => Some(SessionUpdate::UserMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(t)),
        ))),
        Utterance::Agent(t) => Some(SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(t)),
        ))),
        Utterance::Thought(t) => Some(SessionUpdate::AgentThoughtChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(t)),
        ))),
        // `None` where the block carries nothing paintable — a
        // `tool_result` with no content, say. Dropping it is right:
        // the alternative is a line about a tool nobody can see.
        Utterance::Tool(block) => said(&block),
        Utterance::ToolDone(block) => settled(&block),
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
            let input = &block["input"];
            let mut call = ToolCall::new(id.to_owned(), titled(name, input))
                .kind(kind_of(name))
                // **`InProgress`, not the default.** A `ToolCall` built
                // without a status serialises as `pending`, and claude
                // announces a `tool_use` when it is about to run the
                // tool, not when it is queued — so the default would
                // draw a running tool as one that has not started. The
                // other end of this pair is `settled`.
                .status(ToolCallStatus::InProgress)
                // The whole input, so a face can open a line up and
                // show what was actually run. The title is a summary
                // and summaries lose things; this is the thing itself.
                .raw_input(input.clone());
            if let Some(path) = input.get("file_path").and_then(|p| p.as_str()) {
                call = call.locations(vec![ToolCallLocation::new(path)]);
            }
            // An edit carries its own before and after, so the protocol
            // can carry the diff rather than the sentence "a file was
            // edited". `Edit` is the only claude tool that hands over
            // both sides; `Write` gives only the new text, which the
            // protocol accepts (`old_text` is optional) and which reads
            // correctly as "all of this is new".
            if let (Some(path), Some(new_text)) = (
                input.get("file_path").and_then(|p| p.as_str()),
                input
                    .get("new_string")
                    .or_else(|| input.get("content"))
                    .and_then(|t| t.as_str()),
            ) {
                let mut diff = Diff::new(path, new_text.to_owned());
                if let Some(old) = input.get("old_string").and_then(|t| t.as_str()) {
                    diff = diff.old_text(old.to_owned());
                }
                call = call.content(vec![ToolCallContent::Diff(diff)]);
            }
            Some(SessionUpdate::ToolCall(call))
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
    let mut fields = ToolCallUpdateFields::new().status(status);
    // **What the tool actually said.** A finished tool used to settle
    // with nothing but a colour, so「跑完了」and「跑完了,输出是这个」
    // looked identical — and for a failed one the whole of what a
    // person needs is the message it failed with.
    //
    // claude writes `content` as a string most of the time and as a
    // list of blocks the rest, so both are read. A tool that returned
    // nothing gets no content rather than an empty block: an empty
    // panel that opens onto nothing is worse than no panel.
    if let Some(text) = result_text(block) {
        fields = fields.content(vec![ToolCallContent::Content(Content::new(
            ContentBlock::Text(TextContent::new(text)),
        ))]);
    }
    Some(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
        id.to_owned(),
        fields,
    )))
}

/// The text a `tool_result` block carries, in either of the two shapes
/// claude writes it in.
///
/// **Bounded at 16 KiB.** A `Read` of a large file comes back whole,
/// and every byte of it would cross khor's wire, sit in a face's memory
/// and re-render on every poll — for a panel nobody scrolls to the
/// bottom of. The cut is marked so a reader knows there was more; an
/// unmarked truncation is a lie about what the tool returned.
fn result_text(block: &serde_json::Value) -> Option<String> {
    const MAX: usize = 16 * 1024;
    let raw = match block.get("content")? {
        serde_json::Value::String(t) => t.clone(),
        serde_json::Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.len() <= MAX {
        return Some(raw.to_owned());
    }
    // On a char boundary: `raw` is UTF-8 and slicing mid-codepoint
    // panics, which for a Chinese-heavy transcript is not a rare case.
    let mut cut = MAX;
    while cut > 0 && !raw.is_char_boundary(cut) {
        cut -= 1;
    }
    Some(format!("{}\n… {}", &raw[..cut], msg::tool_output_cut()))
}

/// The tokens sitting in the context after a claude `usage` object.
///
/// **All four are summed, and none of them double-counts.** claude's
/// `input_tokens` excludes the cached part — the opposite of codex,
/// which includes it (`crate::usage` carries that difference and the
/// readings behind it). So input, cache-read, cache-write and output
/// are four disjoint piles, and the context is their sum.
fn tokens_in(usage: &serde_json::Value) -> u64 {
    let n = |k: &str| {
        usage
            .get(k)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    };
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
    let amount = f
        .get("total_cost_usd")
        .and_then(serde_json::Value::as_f64)?;
    Some(Cost::new(amount, "USD"))
}

/// What one line about this tool call should say.
///
/// **The bug this exists for: a conversation full of lines reading
/// `Bash`.** The title used to be the tool's *name*, so twenty shell
/// commands were twenty identical lines and the one thing a reader
/// wants — which command — was nowhere on screen.
///
/// The keys below are claude's own, read off this machine's transcripts
/// (2026-08-30) rather than guessed:
///
/// | tool | the key carrying the information |
/// |---|---|
/// | `Bash`, `Monitor`, `Agent` | `description` — claude writes a short human line itself |
/// | `Read`, `Edit`, `Write`, `NotebookEdit` | `file_path` |
/// | `Grep`, `Glob` | `pattern` |
/// | `WebFetch`, `WebSearch` | `url` / `query` |
/// | `Skill` | `skill` |
/// | `Task*`, `ListAgents` | ids or nothing, so the name stands |
///
/// **`description` wins wherever it exists**, because it is the agent's
/// own sentence about what it is doing and no rule written here beats
/// that. Everything unanticipated falls back to the bare name — the
/// state this replaces — so a tool nobody thought of is no worse off.
///
/// A path shows its last two segments rather than all of them: an
/// absolute path in a 736-wide column pushes the part that identifies
/// the file off the end, and the tail is the half a reader recognises.
fn titled(name: &str, input: &serde_json::Value) -> String {
    let s = |k: &str| {
        input
            .get(k)
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
    };
    if let Some(d) = s("description") {
        return d.to_owned();
    }
    let detail = match name {
        "Read" | "Edit" | "Write" | "NotebookEdit" => s("file_path").map(tail_of),
        "Grep" | "Glob" => s("pattern").map(str::to_owned),
        "WebFetch" | "WebSearch" => s("url").or_else(|| s("query")).map(str::to_owned),
        "Skill" => s("skill").map(str::to_owned),
        "Bash" | "BashOutput" => s("command").map(str::to_owned),
        _ => None,
    };
    match detail {
        Some(d) => format!("{name} {d}"),
        None => name.to_owned(),
    }
}

/// The last two segments of a path, which is the part a reader knows.
fn tail_of(path: &str) -> String {
    if path.matches('/').count() < 2 {
        return path.trim_start_matches('/').to_owned();
    }
    let parts: Vec<&str> = path.rsplit('/').take(2).collect();
    format!("{}/{}", parts[1], parts[0])
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
    std::fs::write(
        &path,
        serde_json::to_vec(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut args = vec![
        "--mcp-config".into(),
        path.display().to_string(),
        "--strict-mcp-config".into(),
        "--append-system-prompt".into(),
        msg::agent_brief().into(),
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
        assert_eq!(
            json["availableCommands"][0]["description"],
            "squeeze the context"
        );
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
        // **Was `"Read"`, and the change is the point of this batch.**
        // The title is the tool's *name* no longer: a conversation of
        // twenty shell commands was twenty lines reading `Bash`.
        assert_eq!(json["title"], "Read tmp/x");
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
        assert_eq!(
            super::cost_in(&one).map(|c| c.amount),
            Some(0.38777300000000003)
        );

        let two = serde_json::json!({ "modelUsage": {
            "big": { "contextWindow": 1_000_000 },
            "small": { "contextWindow": 200_000 },
        }});
        assert_eq!(super::window_in(&two), Some(1_000_000));

        // **No size is `None`, never a default.** The whole reason the
        // shim carries a learned window is that a made-up one draws a
        // gauge nobody measured; a zero would do the same while looking
        // like an answer.
        assert_eq!(
            super::window_in(&serde_json::json!({ "modelUsage": {} })),
            None
        );
        assert_eq!(
            super::window_in(&serde_json::json!({ "modelUsage": { "m": {} } })),
            None
        );
        assert_eq!(
            super::window_in(&serde_json::json!({ "modelUsage": { "m": { "contextWindow": 0 } } })),
            None
        );
        assert_eq!(super::window_in(&serde_json::json!({})), None);
        assert!(super::cost_in(&serde_json::json!({})).is_none());
    }

    /// **A conversation full of lines reading `Bash` was the bug.**
    /// Each case here is one real tool's real input shape, and the
    /// assertion is the line a person would read.
    ///
    /// The last two are the ones that keep it from over-reaching: a
    /// tool nobody anticipated keeps its bare name (the state this
    /// replaces, so nothing regresses), and `description` beats every
    /// rule below it because it is the agent's own sentence.
    #[test]
    fn a_tool_line_says_which_one_it_was() {
        let t = |name: &str, input: serde_json::Value| super::titled(name, &input);
        assert_eq!(
            t(
                "Bash",
                serde_json::json!({ "command": "cargo test", "description": "跑测试" })
            ),
            "跑测试",
            "the agent's own sentence wins"
        );
        assert_eq!(
            t("Bash", serde_json::json!({ "command": "ls -la" })),
            "Bash ls -la",
            "no description, so the command is the point"
        );
        assert_eq!(
            t(
                "Read",
                serde_json::json!({ "file_path": "/Users/z/khor/crates/node/src/cagent.rs" })
            ),
            "Read src/cagent.rs",
            "the tail is the half a reader recognises"
        );
        assert_eq!(
            t("Read", serde_json::json!({ "file_path": "cagent.rs" })),
            "Read cagent.rs"
        );
        assert_eq!(
            t("Grep", serde_json::json!({ "pattern": "kind_of" })),
            "Grep kind_of"
        );
        // A tool this shim has never heard of, and one whose input says
        // nothing: both keep the bare name rather than inventing a line.
        assert_eq!(
            t("SomeFutureTool", serde_json::json!({ "x": 1 })),
            "SomeFutureTool"
        );
        assert_eq!(t("Bash", serde_json::json!({})), "Bash");
        // An empty description is not a description.
        assert_eq!(
            t(
                "Bash",
                serde_json::json!({ "description": "  ", "command": "id" })
            ),
            "Bash id"
        );
    }

    /// **An edit crosses as a diff, and a call carries what it was
    /// given.** Before this the whole of a tool line was a word, so a
    /// person could see that a file had been edited and never what the
    /// edit was.
    #[test]
    fn an_edit_crosses_the_wire_as_its_own_diff() {
        let block = serde_json::json!({
            "type": "tool_use", "id": "toolu_e", "name": "Edit",
            "input": {
                "file_path": "/Users/z/khor/src/a.rs",
                "old_string": "let x = 1;",
                "new_string": "let x = 2;",
                "replace_all": false
            }
        });
        let json = serde_json::to_value(super::said(&block).expect("an update")).unwrap();
        assert_eq!(json["title"], "Edit src/a.rs");
        assert_eq!(json["kind"], "edit");
        assert_eq!(json["locations"][0]["path"], "/Users/z/khor/src/a.rs");
        assert_eq!(json["content"][0]["type"], "diff");
        assert_eq!(json["content"][0]["oldText"], "let x = 1;");
        assert_eq!(json["content"][0]["newText"], "let x = 2;");
        // The input is carried whole beside the summary of it.
        assert_eq!(json["rawInput"]["replace_all"], false);
    }

    /// **A finished tool says what it said.** Both content shapes claude
    /// writes, the empty case, and the cut — an unmarked truncation is a
    /// lie about what the tool returned, and a reader would act on the
    /// incomplete output believing it whole.
    #[test]
    fn a_finished_tool_carries_its_own_output() {
        let one = serde_json::json!({
            "type": "tool_result", "tool_use_id": "t", "content": "3 files changed"
        });
        let json = serde_json::to_value(super::settled(&one).expect("an update")).unwrap();
        assert_eq!(json["content"][0]["type"], "content");
        assert_eq!(json["content"][0]["content"]["text"], "3 files changed");

        // The other shape: a list of blocks.
        let many = serde_json::json!({
            "type": "tool_result", "tool_use_id": "t",
            "content": [{ "type": "text", "text": "line one" }, { "type": "text", "text": "line two" }]
        });
        let json = serde_json::to_value(super::settled(&many).expect("an update")).unwrap();
        assert_eq!(json["content"][0]["content"]["text"], "line one\nline two");

        // Nothing said is no panel, not an empty one.
        let empty =
            serde_json::json!({ "type": "tool_result", "tool_use_id": "t", "content": "  " });
        let json = serde_json::to_value(super::settled(&empty).expect("an update")).unwrap();
        assert!(json.get("content").is_none() || json["content"].is_null());

        // A long output is cut **and says so**. The sample is CJK on
        // purpose: the cut lands mid-codepoint at 16 KiB and slicing a
        // Rust string there panics.
        let long = "上下文".repeat(20_000);
        let big = serde_json::json!({ "type": "tool_result", "tool_use_id": "t", "content": long });
        let json = serde_json::to_value(super::settled(&big).expect("an update")).unwrap();
        let text = json["content"][0]["content"]["text"].as_str().unwrap();
        assert!(text.len() < 17 * 1024, "cut, {} bytes", text.len());
        assert!(
            text.ends_with(khor_catalog::msg::tool_output_cut()),
            "and marked as cut"
        );
    }

    /// **The request that moves claude's mode is spelled the way claude
    /// reads it.** Measured off the 2.1.252 bundle, where the SDK's own
    /// `setPermissionMode` sends `{subtype: "set_permission_mode",
    /// mode}` — a subtype claude does not know is not refused, it is
    /// ignored, and the face would then draw a mode that never moved.
    #[test]
    fn the_mode_change_is_spelled_the_way_claude_reads_it() {
        let f = super::mode_request("khor-mode-7", "plan");
        assert_eq!(f["type"], "control_request");
        assert_eq!(f["request_id"], "khor-mode-7");
        assert_eq!(f["request"]["subtype"], "set_permission_mode");
        assert_eq!(f["request"]["mode"], "plan");
    }

    /// A session opens with claude's modes listed, current first as
    /// claude's own default, so a face has something to offer before
    /// claude has said a word. The four are the ones the face has words
    /// for (`MODES`).
    #[test]
    fn a_session_opens_with_claudes_modes_listed() {
        let mut opened = super::NewSessionResponse::new("s");
        opened.modes = Some(super::modes_listed("default"));
        let json = serde_json::to_value(&opened).unwrap();
        assert_eq!(json["modes"]["currentModeId"], "default");
        let ids: Vec<&str> = json["modes"]["availableModes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, super::MODES);
    }

    /// **The mode crosses the wire under the name the face reads**, and
    /// it is the agent's own word rather than one this shim renames.
    /// claude puts `permissionMode` on its init frame and it is the
    /// real one — launched with `--permission-mode plan` it comes back
    /// `plan` (measured 2026-08-30 off a real stream-json run).
    #[test]
    fn the_mode_the_agent_is_in_is_spelled_the_way_the_face_reads_it() {
        use agent_client_protocol::schema::v1::CurrentModeUpdate;
        let update = SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new("plan"));
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["sessionUpdate"], "current_mode_update");
        assert_eq!(json["currentModeId"], "plan");
    }

    /// **A replayed tool call and a live one are the same call**, and
    /// this asserts it by comparing the two paths' output rather than
    /// by checking each against a written expectation — two
    /// expectations can both be updated, and the bug was exactly that
    /// only one of them ever was.
    ///
    /// What a person looks at in an existing conversation is almost
    /// entirely replayed, so「the live path improved」and「the screen
    /// changed」were different facts for as long as there were two
    /// translations. There is now one, and this is what says so.
    #[test]
    fn a_replayed_tool_call_is_the_same_call_as_a_live_one() {
        use crate::adaptor::claude::Utterance;
        let block = serde_json::json!({
            "type": "tool_use", "id": "toolu_x", "name": "Bash",
            "input": { "command": "cargo test", "description": "跑测试" }
        });
        let live = serde_json::to_value(super::said(&block).expect("live")).unwrap();
        let past = serde_json::to_value(
            super::replayed(Utterance::Tool(block.clone())).expect("replayed"),
        )
        .unwrap();
        assert_eq!(live, past, "one translation, or the copies drift");
        // And it is the good answer, not merely a matching one: a test
        // that only compared the two would pass with both broken.
        assert_eq!(past["title"], "跑测试");
        assert_eq!(past["status"], "in_progress");
        assert_eq!(past["toolCallId"], "toolu_x");

        let done = serde_json::json!({
            "type": "tool_result", "tool_use_id": "toolu_x", "content": "ok", "is_error": true
        });
        let live = serde_json::to_value(super::settled(&done).expect("live")).unwrap();
        let past = serde_json::to_value(
            super::replayed(Utterance::ToolDone(done.clone())).expect("replayed"),
        )
        .unwrap();
        assert_eq!(live, past);
        assert_eq!(past["status"], "failed");
        // **The ids match across the pair**, which is what lets a
        // replayed failure settle the replayed call it belongs to.
        // Every replayed call used to wear the id `"replayed"`, so no
        // update could match one — twenty tools, one id.
        assert_eq!(past["toolCallId"], "toolu_x");
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
