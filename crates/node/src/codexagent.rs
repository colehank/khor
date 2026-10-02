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
//! # Permission modes (2026-10-03, codex-cli 0.157.1's own schema,
//! `codex app-server generate-ts`)
//!
//! Codex's permissions are two policies, not one mode: an **approval
//! policy** (`untrusted` / `on-request` / `never`, or granular) and a
//! **sandbox** (`read-only` / `workspace-write` / `danger-full-access`).
//! `thread/start` and `thread/resume` take both and answer with the ones
//! the thread actually runs on (`approvalPolicy`, `sandbox`) — the user's
//! config.toml included — so the mode a session starts in is read off
//! that answer, never assumed. **There is no request that changes them on
//! a live thread**: `turn/start` takes `approvalPolicy` and
//! `sandboxPolicy` "for this turn and subsequent turns", so a mode picked
//! here is applied from the next turn on, and a turn already running
//! keeps the policy it started with.
//!
//! The list offered is codex's own three presets (its `/approvals`):
//! read-only (read-only sandbox, asks on request), workspace (codex's
//! `auto`: workspace-write without network, asks on request — its
//! ordinary mode), and full
//! access (no sandbox, never asks). A thread on anything else is listed
//! as it is (`other`), in codex's words.
//!
//! # Attachments
//!
//! A turn's input is a list (`UserInput`): text, `image` by URL (a data
//! URL carries the bytes, so nothing is written to disk), `localImage`
//! by path, audio, skills and mentions. Images go as data-URL images; a
//! text file goes as a text item that names it (`cagent`'s fence). So
//! both prompt capabilities are advertised and ＋ is offered.
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
    AgentCapabilities, ContentBlock, ContentChunk, CurrentModeUpdate, EmbeddedResourceResource,
    InitializeRequest, InitializeResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PermissionOption, PermissionOptionKind,
    PromptCapabilities, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, SessionMode, SessionModeState, SessionNotification, SessionUpdate,
    SetSessionModeRequest, SetSessionModeResponse, StopReason, TextContent, ToolCall,
    ToolCallUpdate, ToolCallUpdateFields,
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
    /// The preset picked since the thread opened, if any: sent with the
    /// next `turn/start` and every one after (the protocol has no live
    /// change, module head). `None` leaves the thread's own policy.
    preset: Mutex<Option<Preset>>,
    /// The preset the thread opened in, read off codex's answer.
    opened: Mutex<Option<Preset>>,
    /// The thread's own policy when it is none of the presets — the
    /// `other` entry's words, kept so a second load can say it again.
    other: Mutex<Option<String>>,
}

/// One of codex's permission presets (its `/approvals` list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
    ReadOnly,
    Auto,
    FullAccess,
}

/// The presets in codex's order, by the ids the mode list wears.
const PRESETS: [(&str, Preset); 3] = [
    ("read-only", Preset::ReadOnly),
    // `workspace`, not codex's own `auto`: claude has an `auto` mode
    // too, and a face that words modes by id would call one by the
    // other's name.
    ("workspace", Preset::Auto),
    ("full-access", Preset::FullAccess),
];

/// The id a thread on none of the presets is listed under.
const OTHER: &str = "other";

impl Preset {
    fn id(self) -> &'static str {
        PRESETS.iter().find(|(_, p)| *p == self).map(|(id, _)| *id).unwrap_or("workspace")
    }

    fn of(id: &str) -> Option<Preset> {
        PRESETS.iter().find(|(i, _)| *i == id).map(|(_, p)| *p)
    }

    /// The two policies a `turn/start` carries for this preset, in the
    /// protocol's spelling (`AskForApproval`, `SandboxPolicy`).
    fn policies(self) -> (serde_json::Value, serde_json::Value) {
        match self {
            Preset::ReadOnly => (
                serde_json::json!("on-request"),
                serde_json::json!({ "type": "readOnly", "networkAccess": false }),
            ),
            Preset::Auto => (
                serde_json::json!("on-request"),
                serde_json::json!({
                    "type": "workspaceWrite",
                    "writableRoots": [],
                    "networkAccess": false,
                    "excludeTmpdirEnvVar": false,
                    "excludeSlashTmp": false,
                }),
            ),
            Preset::FullAccess => (
                serde_json::json!("never"),
                serde_json::json!({ "type": "dangerFullAccess" }),
            ),
        }
    }
}

/// Which preset a thread's own policies are, read off a `thread/start`
/// or `thread/resume` answer — `None` when they are none of the three.
/// Exact on both halves: a workspace-write sandbox *with* network is not
/// 「auto」, whose word says it asks before using the network.
fn preset_in(answer: &serde_json::Value) -> Option<Preset> {
    let approval = answer["approvalPolicy"].as_str()?;
    let sandbox = &answer["sandbox"];
    let network = sandbox["networkAccess"].as_bool().unwrap_or(false);
    match (approval, sandbox["type"].as_str()?) {
        ("on-request", "readOnly") if !network => Some(Preset::ReadOnly),
        ("on-request", "workspaceWrite") if !network => Some(Preset::Auto),
        ("never", "dangerFullAccess") => Some(Preset::FullAccess),
        _ => None,
    }
}

/// codex's own words for a policy that is none of the presets.
fn other_words(answer: &serde_json::Value) -> String {
    let approval = match &answer["approvalPolicy"] {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "?".to_owned(),
        v => v.as_object().and_then(|o| o.keys().next().cloned()).unwrap_or_else(|| v.to_string()),
    };
    let sandbox = answer["sandbox"]["type"].as_str().unwrap_or("?").to_owned();
    msg::codex_mode_other(&approval, &sandbox)
}

/// The mode list a session opens with: the three presets, each with its
/// line, and — when the thread runs on something else — that, current.
fn modes_listed(current: Option<Preset>, other: Option<&str>) -> SessionModeState {
    let mut listed: Vec<SessionMode> = PRESETS
        .iter()
        .map(|(id, p)| {
            let (name, note) = match p {
                Preset::ReadOnly => (msg::codex_mode_read_only(), msg::codex_mode_read_only_note()),
                Preset::Auto => (msg::codex_mode_auto(), msg::codex_mode_auto_note()),
                Preset::FullAccess => (msg::codex_mode_full_access(), msg::codex_mode_full_access_note()),
            };
            SessionMode::new(*id, name).description(note)
        })
        .collect();
    let current = match (current, other) {
        (Some(p), _) => p.id().to_owned(),
        (None, Some(words)) => {
            listed.push(SessionMode::new(OTHER, words.to_owned()));
            OTHER.to_owned()
        }
        (None, None) => Preset::Auto.id().to_owned(),
    };
    SessionModeState::new(current, listed)
}

/// What one turn sends codex as its input: the typed text first, then
/// each text file as a text item naming it, then each image as a data
/// URL — the order a person wrote in (`khor_acp::Handle::prompt_with`).
fn turn_input(prompt: &[ContentBlock]) -> Vec<serde_json::Value> {
    let text = prompt
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut input = vec![serde_json::json!({ "type": "text", "text": text, "text_elements": [] })];
    for b in prompt {
        if let ContentBlock::Resource(r) = b
            && let EmbeddedResourceResource::TextResourceContents(t) = &r.resource
        {
            input.push(serde_json::json!({
                "type": "text",
                "text": format!("<file uri=\"{}\">\n{}\n</file>", t.uri, t.text),
                "text_elements": [],
            }));
        }
    }
    for b in prompt {
        if let ContentBlock::Image(i) = b {
            input.push(serde_json::json!({
                "type": "image",
                "url": format!("data:{};base64,{}", i.mime_type, i.data),
            }));
        }
    }
    input
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
        preset: Mutex::new(None),
        opened: Mutex::new(None),
        other: Mutex::new(None),
    });
    let s_new = shim.clone();
    let s_prompt = shim.clone();
    let s_load = shim.clone();
    let s_cancel = shim.clone();
    let s_mode = shim.clone();
    let s_drain = shim.clone();

    Agent
        .builder()
        .name("khor-codexagent")
        .on_receive_request(
            async move |initialize: InitializeRequest, responder, _connection| {
                responder.respond(
                    InitializeResponse::new(initialize.protocol_version).agent_capabilities(
                        AgentCapabilities::new().load_session(true).prompt_capabilities(
                            // Both carried (`turn_input`): images as
                            // data-URL images, a text file as a text item
                            // that names it.
                            PromptCapabilities::new().image(true).embedded_context(true),
                        ),
                    ),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |new: NewSessionRequest, responder, _connection| match s_new
                .start_codex(&new.cwd)
                .await
            {
                // The model codex said the thread runs on, in the
                // session's `_meta` (`khor_acp::MODEL_META`): known the
                // moment the thread starts. Beside it the modes, current
                // as the thread's own answer says.
                Ok((sid, model, _)) => {
                    let mut opened = NewSessionResponse::new(sid);
                    if let Some(m) = model {
                        opened = opened.meta(khor_acp::model_meta(&m));
                    }
                    opened.modes = Some(s_new.modes());
                    responder.respond(opened)
                }
                Err(e) => responder.respond_with_internal_error(e),
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |prompt: PromptRequest, responder, connection| {
                let input = turn_input(&prompt.prompt);
                let (tx, rx) = oneshot::channel::<StopReason>();
                {
                    let mut turn = s_prompt.turn.lock().unwrap();
                    if turn.is_some() {
                        return responder.respond_with_internal_error(msg::cagent_one_turn());
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
                    let mut params = serde_json::json!({ "threadId": sid, "input": input });
                    // A picked mode rides every turn from here on (the
                    // protocol's only door, module head).
                    if let Some(p) = *shim.preset.lock().unwrap() {
                        let (approval, sandbox) = p.policies();
                        params["approvalPolicy"] = approval;
                        params["sandboxPolicy"] = sandbox;
                    }
                    let started = shim.request("turn/start", params).await;
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
                let mut model = None;
                match started {
                    Some(cur) if cur == sid => {}
                    Some(_) => {
                        return responder.respond_with_internal_error(msg::cagent_one_session());
                    }
                    None => {
                        // The takeover: resume the recorded thread. Its
                        // world (cwd) is the rollout's own record, not
                        // the loader's — the resumer may stand anywhere.
                        let home = crate::adaptor::vendor_home(&s_load.root).join(".codex");
                        let cwd = crate::adaptor::codex::Codex::at(home)
                            .recorded_cwd(&sid)
                            .unwrap_or_else(|| load.cwd.clone());
                        match s_load.start_codex_resume(&cwd, &sid).await {
                            Ok((m, _)) => model = m,
                            Err(e) => return responder.respond_with_internal_error(e),
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
                    let Some(update) = crate::cagent::replayed(u) else {
                        continue;
                    };
                    let _ =
                        connection.send_notification(SessionNotification::new(sid.clone(), update));
                }
                let mut loaded = LoadSessionResponse::new();
                if let Some(m) = model {
                    loaded = loaded.meta(khor_acp::model_meta(&m));
                }
                // A second load of the live thread lists the mode it is
                // in now: the one picked, else the one it opened in.
                loaded.modes = Some(s_load.modes());
                responder.respond(loaded)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |set: SetSessionModeRequest, responder, connection| {
                let id = set.mode_id.to_string();
                let Some(p) = Preset::of(&id) else {
                    // `other` is what the thread already runs on: picking
                    // it again before anything else was picked is no
                    // change; after one, codex has no way back to it.
                    if id == OTHER && s_mode.preset.lock().unwrap().is_none() {
                        return responder.respond(SetSessionModeResponse::new());
                    }
                    return responder.respond_with_internal_error(msg::codexagent_unknown_mode(&id));
                };
                // **Applied from the next turn** (module head): there is
                // nothing to ask codex now, so the mode is this shim's to
                // hold and the update says what the next turn will run.
                *s_mode.preset.lock().unwrap() = Some(p);
                let _ = connection.send_notification(SessionNotification::new(
                    set.session_id.clone(),
                    SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(p.id().to_owned())),
                ));
                responder.respond(SetSessionModeResponse::new())
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
        .connect_with(
            Stdio::new(),
            move |connection: ConnectionTo<Client>| async move {
                // The client's EOF ends the shim (`cagent`'s arm, same
                // orphan discipline).
                tokio::select! {
                    _ = drain(s_drain, frames_rx, &connection) => {}
                    _ = connection.incoming_closed() => {}
                }
                Ok(())
            },
        )
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
    /// own uuid is the session id, and beside it the model the answer
    /// names (`thread/start`'s `model`), when it names one.
    async fn start_codex(
        self: &Arc<Self>,
        cwd: &std::path::Path,
    ) -> Result<(String, Option<String>, Option<Preset>), String> {
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
        Ok((sid, model_of(&started), self.policy_from(&started)))
    }

    /// Spawns the codex child **resuming** an existing thread — the
    /// takeover path: the conversation already exists on disk, and
    /// `thread/resume` continues the same id into the same rollout
    /// (probed: it answered with words from before the restart).
    async fn start_codex_resume(
        self: &Arc<Self>,
        cwd: &std::path::Path,
        sid: &str,
    ) -> Result<(Option<String>, Option<Preset>), String> {
        self.start_appserver(cwd)?;
        self.handshake().await?;
        let mut params = serde_json::json!({ "threadId": sid, "cwd": cwd.display().to_string() });
        scheduler_into(&mut params)?;
        let resumed = self.request("thread/resume", params).await?;
        if resumed["thread"]["id"].as_str() != Some(sid) {
            return Err(msg::codexagent_bad_answer("thread/resume"));
        }
        *self.session.lock().unwrap() = Some(sid.to_owned());
        Ok((model_of(&resumed), self.policy_from(&resumed)))
    }

    /// The preset a start or resume answer runs on, and — when it is
    /// none — codex's words for what it is, kept for the mode list.
    fn policy_from(&self, answer: &serde_json::Value) -> Option<Preset> {
        let p = preset_in(answer);
        *self.opened.lock().unwrap() = p;
        if p.is_none() && answer.get("approvalPolicy").is_some() {
            *self.other.lock().unwrap() = Some(other_words(answer));
        }
        p
    }

    /// The mode list, current as the session is now: the preset picked,
    /// else the one the thread opened in, else codex's own words for
    /// what it runs on. (An answer that named no policy at all — a codex
    /// older than the field — lists `auto`, codex's default preset.)
    fn modes(&self) -> SessionModeState {
        let current = self.preset.lock().unwrap().or(*self.opened.lock().unwrap());
        modes_listed(current, self.other.lock().unwrap().as_deref())
    }

    fn start_appserver(self: &Arc<Self>, cwd: &std::path::Path) -> Result<(), String> {
        if self.session.lock().unwrap().is_some() {
            return Err(msg::cagent_one_session().into());
        }
        let named = std::env::var(CODEX_ENV).unwrap_or_else(|_| "codex".into());
        // Looked up by khor rather than left to the spawn, so a missing
        // vendor binary is something khor reports by name and by where
        // it searched — not an `ENOENT` a reader has to guess at
        // (`crate::spawn_path`, and the launchd `PATH` it exists for).
        let path = crate::spawn_path(&named);
        let bin = crate::find_program(&named, &path)
            .ok_or_else(|| msg::command_not_found(&named, &path))?;
        let mut c = std::process::Command::new(&bin);
        c.env("PATH", &path);
        c.arg("app-server")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let mut child = c.spawn().map_err(|e| msg::wont_start(&bin, e))?;
        let stdout = child.stdout.take().ok_or(msg::codexagent_no_pipe())?;
        *self.child_pid.lock().unwrap() = Some(child.id());
        *self.stdin.lock().unwrap() = child.stdin.take();
        let shim = self.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
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
            let _ = shim
                .frames
                .send(serde_json::json!({ "khor_child_gone": true }));
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
        let stdin = guard.as_mut().ok_or(msg::cagent_not_started())?;
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
    params["developerInstructions"] = serde_json::Value::String(msg::agent_brief().to_owned());
    Ok(())
}

/// The codex→ACP pump: every server frame, routed. The permission
/// round-trip is awaited inline on purpose — codex itself is paused on
/// that very answer.
/// The model a `thread/start` or `thread/resume` answer names.
fn model_of(answer: &serde_json::Value) -> Option<String> {
    answer["model"].as_str().filter(|m| !m.is_empty()).map(str::to_owned)
}

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
                let reason =
                    if shim.interrupted.swap(false, Ordering::SeqCst) || status == "interrupted" {
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
    let decision = if allowed { "accept" } else { "decline" };
    let _ = shim.write_line(&serde_json::json!({
        "jsonrpc": "2.0", "id": request_id, "result": { "decision": decision },
    }));
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::schema::v1::{
        ContentBlock, EmbeddedResource, EmbeddedResourceResource, ImageContent, TextContent,
        TextResourceContents,
    };

    use super::{Preset, modes_listed, preset_in, turn_input};

    /// **The mode a session opens in is read off codex's answer**, both
    /// halves exact: a workspace sandbox with the network open is not
    /// 「auto」, whose words say it asks before going online.
    #[test]
    fn the_opening_mode_is_the_threads_own_policy() {
        let answer = |approval: serde_json::Value, sandbox: serde_json::Value| {
            serde_json::json!({ "approvalPolicy": approval, "sandbox": sandbox })
        };
        let ws = |net: bool| serde_json::json!({ "type": "workspaceWrite", "networkAccess": net });
        assert_eq!(preset_in(&answer("on-request".into(), ws(false))), Some(Preset::Auto));
        assert_eq!(preset_in(&answer("on-request".into(), ws(true))), None);
        assert_eq!(
            preset_in(&answer("on-request".into(), serde_json::json!({ "type": "readOnly", "networkAccess": false }))),
            Some(Preset::ReadOnly)
        );
        assert_eq!(
            preset_in(&answer("never".into(), serde_json::json!({ "type": "dangerFullAccess" }))),
            Some(Preset::FullAccess)
        );
        assert_eq!(preset_in(&answer("untrusted".into(), ws(false))), None);
        assert_eq!(preset_in(&serde_json::json!({})), None);
    }

    /// The policies a picked preset sends are spelt the way codex's
    /// schema reads them (`AskForApproval`, `SandboxPolicy`), and each
    /// reads back as the preset it came from.
    #[test]
    fn a_preset_sends_what_codex_reads_back_as_itself() {
        for p in [Preset::ReadOnly, Preset::Auto, Preset::FullAccess] {
            let (approval, sandbox) = p.policies();
            let back = serde_json::json!({ "approvalPolicy": approval, "sandbox": sandbox });
            assert_eq!(preset_in(&back), Some(p), "{back}");
            assert_eq!(Preset::of(p.id()), Some(p));
        }
        assert_eq!(Preset::of("bypassPermissions"), None, "claude's ids are not codex's");
    }

    /// Three presets listed, current as given; a thread on none of them
    /// is listed as it is and is the current one — never rounded.
    #[test]
    fn the_list_says_the_current_mode_truthfully() {
        let ids = |s: &agent_client_protocol::schema::v1::SessionModeState| {
            s.available_modes.iter().map(|m| m.id.to_string()).collect::<Vec<_>>()
        };
        let plain = modes_listed(Some(Preset::ReadOnly), None);
        assert_eq!(plain.current_mode_id.to_string(), "read-only");
        assert_eq!(ids(&plain), ["read-only", "workspace", "full-access"]);
        assert!(plain.available_modes.iter().all(|m| m.description.is_some()), "each mode has its line");
        let other = modes_listed(None, Some("other words"));
        assert_eq!(other.current_mode_id.to_string(), "other");
        assert_eq!(ids(&other), ["read-only", "workspace", "full-access", "other"]);
    }

    /// A turn's input: the typed text, then each text file as a text
    /// item naming it, then each image as a data URL.
    #[test]
    fn attachments_ride_the_turn_in_the_order_written() {
        let blocks = vec![
            ContentBlock::Text(TextContent::new("look".to_owned())),
            ContentBlock::Image(ImageContent::new("QUJD".to_owned(), "image/png".to_owned())),
            ContentBlock::Resource(EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(
                TextResourceContents::new("fn main() {}".to_owned(), "file://main.rs".to_owned()),
            ))),
        ];
        let input = turn_input(&blocks);
        assert_eq!(input.len(), 3);
        assert_eq!(input[0]["type"], "text");
        assert_eq!(input[0]["text"], "look");
        assert_eq!(input[1]["type"], "text");
        let file = input[1]["text"].as_str().unwrap();
        assert!(file.contains("file://main.rs") && file.contains("fn main() {}"), "{file}");
        assert_eq!(input[2]["type"], "image");
        assert_eq!(input[2]["url"], "data:image/png;base64,QUJD");
    }
}
