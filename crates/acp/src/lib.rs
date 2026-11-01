//! The ACP live channel: khor as an Agent Client Protocol client.
//!
//! # What this layer is, and what it deliberately is not
//!
//! This is the **live channel** for sessions khor starts as a GUI
//! session (the user's ruling: opening a session is TUI or GUI, two
//! forms over one store). ACP is a runtime protocol, not persistence —
//! the agent underneath keeps writing its own files wherever it always
//! writes them, so usage accounting and disk discovery cover ACP
//! sessions without a line changing (docs/handoff, the ACP rulings).
//! This crate therefore knows nothing about khor sessions, six words,
//! or rows: it speaks the protocol and hands the caller protocol-shaped
//! events. Mapping them onto khor's words is the caller's judgment.
//!
//! # v1 first, on purpose
//!
//! The official crate ships v1 (stable) and v2 (alpha at the time of
//! writing). Every adapter surveyed speaks v1; `initialize` carries the
//! version so a v2 upgrade is a negotiation change, not a rewrite.
//!
//! # Where the protocol's edge actually is (surveyed 2026-08-20)
//!
//! `session/load` (v1) IS history: the agent replays the whole past as
//! `session/update`s — khor's takeover rides it. **Enumerating sessions
//! is in v1 now**: `session/list` stabilised on 2026-03-09
//! (advertised as `sessionCapabilities.list`), and `session/delete`
//! with it — asked of a *running* agent, and still optional, so
//! adapters differ (20 of 32 registry agents answered it on
//! 2026-08-30; the rest say they cannot). khor asks it behind a press
//! and records the answer (`khor_sync::agents`), and those conversations
//! are import candidates (docs/SHELL.md 第八轮). **The disk road stays**
//! for that reason: claude's and codex's own files answer without
//! starting anything and whether or not an adapter lists, so discovery,
//! usage and the recorded-past view keep reading them, one file-format
//! per vendor in the adaptors. Stable v1 also offers capability-gated no-replay resume and close.
//! Fork remains experimental in the pinned SDK, as does choosing and
//! spawning the agent binary (client-side by the protocol's own
//! starting line).
//!
//! # Which way this can be wrong
//!
//! Everything here is request/response over one child process — there
//! is no arithmetic to get backwards. The open edges are liveness
//! shaped: an agent that never answers leaves [`Handle::prompt`]
//! pending (the caller owns timeouts, since only it knows what a
//! screen can wait for); an agent that dies mid-turn surfaces as the
//! connection ending, which arrives as [`Event::Closed`]. The child
//! cannot be leaked: the library kills the agent's **whole process
//! group** when the transport drops, `npx` wrappers included — the
//! orphan discipline this repo runs on, honoured by the dependency
//! itself.
//!
//! # The permission handler must not block the event loop
//!
//! Handlers run on the connection's event loop: while one runs, no
//! other message moves. A permission answer can take as long as a
//! person takes, so [`Ask`] waits on a spawned task, never in the
//! handler — otherwise every `session/update` would freeze behind an
//! unanswered prompt, which on a screen reads as the agent hanging.

use std::path::PathBuf;
use std::str::FromStr;

use agent_client_protocol::schema::v1::{
    AuthMethodId, AuthenticateRequest, ClientCapabilities, CloseSessionRequest,
    CreateElicitationRequest, CreateElicitationResponse, DeleteSessionRequest, ElicitationAction,
    LogoutRequest, McpServer, ResumeSessionRequest,
};
use agent_client_protocol::schema::v1::{
    CancelNotification, ContentBlock, EmbeddedResource, EmbeddedResourceResource, ImageContent,
    InitializeRequest, ListSessionsRequest, LoadSessionRequest, NewSessionRequest,
    PermissionOptionId, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionConfigOption,
    SessionConfigOptionValue, SessionId, SessionModeId, SessionModeState, SessionNotification,
    SetSessionConfigOptionRequest, SetSessionModeRequest, StopReason, TextContent,
    TextResourceContents,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{AcpAgent, Agent, Client, ConnectionTo, ErrorCode};
use tokio::sync::oneshot;
mod event_store;
use event_store::EventFence;
pub use event_store::{EventPrefix, EventReceiver};
mod interaction;
mod lifecycle;
use lifecycle::{CloseGate, Opening};
pub use lifecycle::{ProcessExit, ShutdownOutcome, TransportLifecycle};
use std::sync::Arc;
mod management;
pub use interaction::validate_elicitation;
pub use management::{manage, manage_on, Management};

/// What the handshake said about the agent on the other end.
///
/// `initialize`'s answer used to be thrown away here, which was
/// affordable only while every agent khor spoke to was one khor wrote
/// (both shims advertise `load_session`). An agent the user named can
/// advertise anything, and **a capability nobody read does not go
/// missing — it turns into its neighbour**: an agent that cannot replay
/// paints an empty past, which is pixel-identical to a conversation
/// where nothing was ever said.
///
/// Deliberately not the protocol's own `AgentCapabilities`: this is the
/// subset khor acts on, and a field here is a promise that some face
/// changes because of it.
#[derive(Debug, Clone)]
pub struct Facts {
    /// Complete handshake facts; additions never require a vendor-specific GUI.
    pub protocol: serde_json::Value,
    /// The agent advertises initial history loading (`session/load`). When
    /// false, a face must not present an unsupported restore as an empty past.
    pub replays: bool,
    /// The agent's own name for itself, when it gives one. Never used
    /// to decide anything — khor's answer to "whose session is this"
    /// is the user's, not the agent's self-description.
    pub name: Option<String>,
    /// The agent takes images in a prompt (`promptCapabilities.image`).
    ///
    /// **Read for the same reason `replays` is, and the reason is the
    /// whole shape of this struct**: a capability nobody reads does not
    /// grow a shape, so a face offers everything to everyone and finds
    /// out what is missing by having it fail. That was affordable while
    /// every agent khor spoke to was one khor wrote; it stops being
    /// affordable the moment somebody attaches an agent off the
    /// registry, because「支持 ACP」is a spectrum and not a switch —
    /// the registry's own probe on 2026-08-30 had 29 of 32 agents doing
    /// `session/load`, 20 doing `session/list`, and **1** doing
    /// `session/stop`.
    pub images: bool,
    /// The agent takes resource links / embedded context in a prompt
    /// (`promptCapabilities.embeddedContext`). What khor's file
    /// attachments ride on.
    pub embeds: bool,
    /// The agent can list its own sessions (`sessionCapabilities.list`).
    pub lists: bool,
    /// The agent said it needs somebody to log in before a session can
    /// open — carried from the handshake rather than discovered by
    /// failing, so a face can say so before the first turn instead of
    /// after it.
    ///
    /// Not the same fact as [`Refusal::Login`]: that one is a session
    /// that was actually refused, this one is the agent advertising
    /// that it has ways to log in at all.
    pub needs_login: bool,
}

/// Only the two lifecycle capabilities actually advertised by Initialize v1.
/// This is a Rust fact accessor, not another protocol or a vendor-name guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryCapabilities {
    pub load: bool,
    pub resume: bool,
}

/// One bounded observation shared with the owner of this single startup. It is
/// also readable after a refused opening, without changing the refusal type.
#[derive(Clone, Default)]
pub struct InitializeFacts(Arc<std::sync::OnceLock<RecoveryCapabilities>>);
impl InitializeFacts {
    pub fn recovery(&self) -> Option<RecoveryCapabilities> {
        self.0.get().copied()
    }
    fn initialized(&self, value: RecoveryCapabilities) -> Result<(), Refusal> {
        self.0.set(value).map_err(|_| Refusal::Wont(
            "Initialization facts cannot be reused for another startup".into(),
        ))
    }
}

/// Why no session exists.
///
/// Three kinds because a person does three different things about them,
/// and one string for all of them meant the commonest of the three —
/// "this agent wants you to log in first" — arrived wearing the same
/// sentence as a typo in the command.
///
/// The words are the caller's: this crate carries the agent's own
/// message as evidence ([`Refusal::said`]) and names which kind it is;
/// choosing what a person reads is khor's judgment, not the protocol's.
#[derive(Debug)]
pub enum Refusal {
    /// `auth_required` (-32000): the agent runs, speaks, and refuses to
    /// open a session until somebody logs in. The one refusal whose fix
    /// is not in khor's hands.
    Login(String),
    /// The agent answered `initialize` with a version this client does
    /// not speak. Carries what it said it speaks.
    ///
    /// Unchecked, this is not an error at all until later: the protocol
    /// has an agent answer with the highest version it supports, so the
    /// handshake "succeeds" and the *first real request* fails — the
    /// shape that costs the most to diagnose, because everything up to
    /// it looked healthy.
    Version(String),
    /// Everything else: the command did not start, it does not speak
    /// the protocol, it died, it said no.
    Wont(String),
}

impl Refusal {
    /// The agent's own words, verbatim. Evidence for a face to show
    /// under khor's sentence — never a substitute for one, since an
    /// agent may say nothing at all.
    pub fn said(&self) -> &str {
        match self {
            Refusal::Login(s) | Refusal::Version(s) | Refusal::Wont(s) => s,
        }
    }
}

/// What this session can be set to, as the agent described it when
/// the session opened.
///
/// Two lists because the protocol has two: **modes** are the one axis
/// every agent that has any axis has (claude's permission modes, a
/// registry agent's "ask / auto"), named separately so a face can put
/// the one that changes what the agent *will do* where a person looks
/// before typing; **options** are everything else — a model, a
/// thinking level, a switch — and arrive already shaped as selects and
/// booleans, categorised by the agent.
///
/// Protocol types on purpose, like [`Event::Note`]: this crate hands
/// the caller protocol-shaped facts and maps none of them. Both are
/// "as of opening"; changes arrive as `current_mode_update` and
/// `config_option_update` notes, and as the answer to
/// [`Handle::set_option`]. Empty for an agent that offers nothing to
/// set, which is most of them — and that emptiness has a shape a face
/// can read (no modes listed), unlike a capability nobody carried.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub modes: Option<SessionModeState>,
    pub options: Vec<SessionConfigOption>,
    /// The model the session runs on, as the provider spells it, when
    /// the agent said so in the session's `_meta` ([`MODEL_META`]):
    /// khor's own shims do — codex knows it when the thread starts,
    /// claude on its first turn's init frame (`session_info_update`).
    /// An agent off the registry names its model, when it does, as a
    /// `model` option instead; a face reads that one from `options`.
    pub model: Option<String>,
}

/// The `_meta` key khor's shims put the model id under — on the
/// session's opening answer, or on a `session_info_update` when it is
/// learned later. ACP has no stable field for it (`models` is an
/// unstable extension), and `_meta` is the protocol's own place for
/// what it does not spell.
pub const MODEL_META: &str = "modelId";

/// The model id a `_meta` carries ([`MODEL_META`]), if any.
pub fn model_in(meta: Option<&serde_json::Map<String, serde_json::Value>>) -> Option<String> {
    meta?
        .get(MODEL_META)?
        .as_str()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
}

/// A `_meta` that says the model id ([`MODEL_META`]).
pub fn model_meta(model: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut meta = serde_json::Map::new();
    meta.insert(
        MODEL_META.to_owned(),
        serde_json::Value::String(model.to_owned()),
    );
    meta
}

/// What the connection tells the caller, in protocol shape.
#[derive(Debug)]
pub enum Event {
    /// The session exists; prompts may be sent.
    Ready {
        session: SessionId,
    },
    /// One `session/update` — conversation chunks, tool calls, plans.
    /// Raw on purpose: this crate maps nothing onto khor's words.
    Note(SessionNotification),
    /// The agent asked permission and is now waiting on [`Ask`].
    Ask(Ask),
    Elicit(Elicitation),
    ElicitDone(String),
    /// The connection ended. `None` is a close the caller asked for;
    /// `Some` carries the error text of one it did not.
    /// An aborted event stream can report its error before process cleanup.
    /// Only [`Handle::shutdown`] confirms the driver's/provider's exit fact.
    Closed(Option<String>),
}

/// A structured user interaction. As with permissions, dropping cancels it.
#[derive(Debug)]
pub struct Elicitation {
    pub request: CreateElicitationRequest,
    answer: oneshot::Sender<CreateElicitationResponse>,
    _store_credit: event_store::InteractionCredit,
}
impl Elicitation {
    pub fn validate(&self, value: &serde_json::Value) -> Result<CreateElicitationResponse, String> {
        validate_elicitation(&self.request, value)
    }
    pub fn answer(self, response: CreateElicitationResponse) -> Result<(), String> {
        self.answer
            .send(response)
            .map_err(|_| "Interaction is no longer pending".to_owned())
    }
}

/// Session execution context belongs to the execution node, never the GUI device.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Environment {
    pub mcp_servers: Vec<McpServer>,
    pub additional_directories: Vec<PathBuf>,
}

/// A `session/request_permission`, waiting for exactly one answer.
///
/// Dropping it unanswered cancels the request — the agent sees a
/// dismissal, never a hang: the responder lives in a spawned task that
/// answers `Cancelled` when this side goes away.
#[derive(Debug)]
pub struct Ask {
    pub request: RequestPermissionRequest,
    answer: oneshot::Sender<RequestPermissionOutcome>,
    _store_credit: event_store::InteractionCredit,
}

impl Ask {
    /// Approve by picking one of the offered options, by its id.
    pub fn choose(self, id: &str) {
        let id = PermissionOptionId::new(id.to_owned());
        let _ = self.answer.send(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(id),
        ));
    }

    /// Refuse without picking anything.
    pub fn dismiss(self) {
        let _ = self.answer.send(RequestPermissionOutcome::Cancelled);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenedBy {
    New,
    Load,
    Resume,
}

/// A live conversation with one agent process.
pub struct Handle {
    conn: ConnectionTo<Agent>,
    session: SessionId,
    facts: Facts,
    settings: Settings,
    opened_by: OpenedBy,
    close: Arc<CloseGate>,
    event_fence: EventFence,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.request_shutdown();
    }
}

impl Handle {
    pub fn opened_by(&self) -> OpenedBy { self.opened_by }
    /// Forward the finite batch of events already accepted before this call.
    /// Call after an RPC response before publishing its UI completion. This is
    /// independent of live close admission: a delivered response remains delivered.
    /// The receiver uses this store-bound prefix to drain exactly those events.
    pub async fn fence_events(&self) -> Result<EventPrefix, String> {
        self.event_fence.wait().await
    }

    pub fn request_shutdown(&self) {
        self.close.request();
    }

    /// A shared confirmation. DriverClosed alone does not prove a child was reaped.
    pub async fn shutdown(&self) -> Result<ShutdownOutcome, String> {
        self.request_shutdown();
        self.close.completed().await
    }

    /// Standard lifecycle controls remain capability-gated.
    pub async fn control(
        &self,
        operation: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.close
            .while_open(self.control_inner(operation, payload))
            .await
    }
    async fn control_inner(
        &self,
        operation: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let caps = &self.facts.protocol["capabilities"];
        match operation {
            "close" if !caps["sessionCapabilities"]["close"].is_null() => self
                .conn
                .send_request(CloseSessionRequest::new(self.session.clone()))
                .block_task()
                .await
                .map(|_| serde_json::json!({}))
                .map_err(|e| e.to_string()),
            "delete" if !caps["sessionCapabilities"]["delete"].is_null() => self
                .conn
                .send_request(DeleteSessionRequest::new(self.session.clone()))
                .block_task()
                .await
                .map(|_| serde_json::json!({}))
                .map_err(|e| e.to_string()),
            "logout" if !caps["auth"]["logout"].is_null() => self
                .conn
                .send_request(LogoutRequest::new())
                .block_task()
                .await
                .map(|_| serde_json::json!({}))
                .map_err(|e| e.to_string()),
            "authenticate" => {
                let id = payload["methodId"]
                    .as_str()
                    .ok_or("Missing authentication method")?;
                let method = self.facts.protocol["authMethods"]
                    .as_array()
                    .and_then(|xs| xs.iter().find(|x| x["id"] == id))
                    .ok_or("Unknown authentication method")?;
                if method["type"].as_str().is_some_and(|t| t != "agent") {
                    return Err(
                        "This authentication method requires its advertised login flow".into(),
                    );
                }
                self.conn
                    .send_request(AuthenticateRequest::new(AuthMethodId::new(id.to_owned())))
                    .block_task()
                    .await
                    .map(|_| serde_json::json!({}))
                    .map_err(|e| e.to_string())
            }
            _ => Err("The agent does not support this session operation".into()),
        }
    }
    /// One turn: send the text, resolve when the agent finishes it.
    /// Updates stream through [`Event::Note`] while this is pending.
    pub async fn prompt(&self, text: &str) -> Result<StopReason, String> {
        self.prompt_with(text, &[], &[]).await
    }

    /// The same turn, with images riding along.
    ///
    /// **Separate from [`Handle::prompt`] rather than one call with an
    /// empty slice everywhere**, because the caller that has no images
    /// should not have to say so — and because the two are gated
    /// differently upstream: a face may only offer an attachment when
    /// the agent said it takes one (`Facts::images`), and text is not
    /// gated on anything.
    ///
    /// **The blocks go text-first.** A prompt that led with an image
    /// would have the agent reading the picture before the sentence
    /// asking about it, and while every agent here tolerates either
    /// order, only one of them is the order a person wrote in.
    pub async fn prompt_with(
        &self,
        text: &str,
        images: &[(String, String)],
        files: &[(String, String)],
    ) -> Result<StopReason, String> {
        self.close
            .while_open(self.prompt_with_inner(text, images, files))
            .await
    }
    async fn prompt_with_inner(
        &self,
        text: &str,
        images: &[(String, String)],
        files: &[(String, String)],
    ) -> Result<StopReason, String> {
        let mut blocks = vec![ContentBlock::Text(TextContent::new(text.to_owned()))];
        for (mime, data) in images {
            blocks.push(ContentBlock::Image(ImageContent::new(
                data.clone(),
                mime.clone(),
            )));
        }
        // `(name, text)` as an embedded resource, which is what the
        // protocol calls a file riding along with the question rather
        // than a path the agent is told to go and read.
        //
        // **The difference is not cosmetic on this product.** khor is a
        // mesh: the face holding the file may be a phone, and a path
        // means nothing to an agent on another machine. Carrying the
        // contents is the only version of "here, look at this" that
        // works across the net khor exists to be.
        for (name, body) in files {
            blocks.push(ContentBlock::Resource(EmbeddedResource::new(
                EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
                    body.clone(),
                    format!("file://{name}"),
                )),
            )));
        }
        let request = PromptRequest::new(self.session.clone(), blocks);
        let response = self
            .conn
            .send_request(request)
            .block_task()
            .await
            .map_err(|e| e.to_string())?;
        Ok(response.stop_reason)
    }

    /// Interrupt the current turn without ending the session.
    pub fn cancel(&self) -> Result<(), String> {
        self.close.check_open()?;
        self.conn
            .send_notification(CancelNotification::new(self.session.clone()))
            .map_err(|e| e.to_string())
    }

    pub fn session(&self) -> &SessionId {
        &self.session
    }

    /// What the handshake said this agent can do. Read it before
    /// offering a person something the agent cannot deliver — asking
    /// and swallowing the error looks identical to succeeding at
    /// nothing ([`Facts`]).
    pub fn facts(&self) -> &Facts {
        &self.facts
    }

    /// What the agent said this session can be set to, when it opened
    /// ([`Settings`]). A snapshot: the live value is whatever the last
    /// `current_mode_update` / `config_option_update` note said.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Puts the session in one of its listed modes (`session/set_mode`).
    /// The id is the agent's own, off [`Settings::modes`]; an agent
    /// answers a mode it did not list with an error, which comes back
    /// here as its words.
    pub async fn set_mode(&self, id: &str) -> Result<(), String> {
        self.close.while_open(self.set_mode_inner(id)).await
    }
    async fn set_mode_inner(&self, id: &str) -> Result<(), String> {
        let request =
            SetSessionModeRequest::new(self.session.clone(), SessionModeId::new(id.to_owned()));
        self.conn
            .send_request(request)
            .block_task()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Sets one option (`session/set_config_option`) and answers with
    /// **the whole list as the agent now has it** — the protocol has
    /// the agent send every option back, because one change can move
    /// another (a model that has no "fast" switch), and a client that
    /// updated only the one it touched would be drawing the rest from
    /// memory.
    pub async fn set_option(
        &self,
        id: &str,
        value: SessionConfigOptionValue,
    ) -> Result<Vec<SessionConfigOption>, String> {
        self.close
            .while_open(self.set_option_inner(id, value))
            .await
    }
    async fn set_option_inner(
        &self,
        id: &str,
        value: SessionConfigOptionValue,
    ) -> Result<Vec<SessionConfigOption>, String> {
        let request =
            SetSessionConfigOptionRequest::new(self.session.clone(), id.to_owned(), value);
        self.conn
            .send_request(request)
            .block_task()
            .await
            .map(|r| r.config_options)
            .map_err(|e| e.to_string())
    }
}

/// Starts `command` as an ACP agent and opens one session in `cwd`.
///
/// Resolves once the session exists; conversation and permission
/// traffic then arrives on the returned receiver until [`Event::Closed`].
/// Dropping the [`Handle`] closes everything, agent process included.
pub async fn start(
    command: &str,
    cwd: PathBuf,
) -> Result<(Handle, EventReceiver), Refusal> {
    start_with(command, cwd, None).await
}

/// [`start`], but the session is an existing one the agent resumes
/// (`session/load` instead of `session/new`) — the takeover path (批C).
/// The load's replayed updates arrive as [`Event::Note`]s before
/// [`Event::Ready`]; a caller with nobody attached yet simply drops
/// them. Later views read the canonical capture rather than reloading the agent.
pub async fn start_resume(
    command: &str,
    cwd: PathBuf,
    session: &str,
) -> Result<(Handle, EventReceiver), Refusal> {
    start_with(command, cwd, Some(session.to_owned())).await
}

async fn start_with(
    command: &str,
    cwd: PathBuf,
    resume: Option<String>,
) -> Result<(Handle, EventReceiver), Refusal> {
    start_configured(command, cwd, resume, Environment::default()).await
}

/// One session an agent says it has, in khor's own words.
///
/// The protocol's `SessionInfo` narrowed to what a person picking from
/// a list needs. Narrowed here rather than passed through so that
/// callers — a CLI verb, a face — do not each depend on the schema
/// crate for a struct they only read four fields of.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub id: String,
    pub cwd: PathBuf,
    /// The agent's own name for it, when it has one.
    pub title: Option<String>,
    /// RFC 3339, as the agent wrote it. Kept as text: khor does not
    /// need to do arithmetic on it, and parsing a timestamp only to
    /// print it back is a place to be wrong about time zones.
    pub updated_at: Option<String>,
}

/// What asking an agent for its sessions answered.
///
/// **Three answers, not two, and the third is the whole reason this is
/// an enum.** "This agent cannot list its sessions" and "this agent has
/// no sessions" are different facts that an empty `Vec` would render
/// identical — the same trap `Facts::replays` exists for, where a face
/// with only the answer to go on paints "no past" for both and one of
/// those is a lie.
///
/// It matters more here than there: `session/list` is **not** in the
/// protocol's mandatory set (only `session/new`, `session/prompt`,
/// `session/cancel` and `session/update` are), and the registry's own
/// probe on 2026-08-30 had 20 of 32 agents implement it. So a third of
/// the agents khor can now start will answer `Cannot`, and that is
/// normal rather than broken.
#[derive(Debug, Clone, PartialEq)]
pub enum Listing {
    /// The agent lists, and this is what it has. Possibly empty, which
    /// here really does mean "none".
    Sessions(Vec<Recorded>),
    /// The agent does not implement `session/list`.
    Cannot,
}

/// Starts `command`, asks what sessions it has, and stops.
///
/// **No session is opened.** The protocol allows `session/list` right
/// after `initialize`, and opening one to ask would leave the agent
/// holding a conversation nobody wanted — which for an agent that
/// charges per session is not a free mistake.
///
/// `cwd` narrows the answer to sessions rooted there when the agent
/// honours it; `None` asks for everything it knows.
///
/// # Why khor needs this at all
///
/// khor discovers sessions by reading vendors' files on disk, which
/// works only for the two vendors khor wrote readers for. An agent out
/// of the ACP registry writes its records wherever it likes, in
/// whatever shape it likes, and **`session/list` is the only way khor
/// can know it has a past at all**. The alternative is that every
/// registry agent looks brand new every time it is opened.
pub async fn list(command: &str, cwd: Option<PathBuf>) -> Result<Listing, Refusal> {
    let agent = AcpAgent::from_str(command).map_err(|e| Refusal::Wont(e.to_string()))?;
    let (ready_tx, ready_rx) = oneshot::channel::<Result<Listing, Refusal>>();

    tokio::spawn(async move {
        let mut ready_tx = Some(ready_tx);
        let _ = Client
            .builder()
            // A listing connection answers nothing: it asks one
            // question and leaves. An agent that sends updates or asks
            // permission before being prompted is not a case to
            // support — dropping them is what "no session was opened"
            // means.
            .connect_with(agent, |connection: ConnectionTo<Agent>| async move {
                let out = ask_for_sessions(&connection, cwd).await;
                let refused = out.as_ref().err().map(|r| r.said().to_owned());
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(out);
                }
                match refused {
                    Some(said) => Err(agent_client_protocol::util::internal_error(said)),
                    // Returning ends the connection, which drops the
                    // transport, which kills the agent's process group.
                    // That is the whole lifetime of a listing.
                    None => Ok(()),
                }
            })
            .await;
    });

    ready_rx
        .await
        .map_err(|_| Refusal::Wont("the agent ended before it answered".to_owned()))?
}

/// `initialize`, then `session/list` **only if the handshake said so**.
async fn ask_for_sessions(
    connection: &ConnectionTo<Agent>,
    cwd: Option<PathBuf>,
) -> Result<Listing, Refusal> {
    let hello = connection
        .send_request(InitializeRequest::new(ProtocolVersion::V1))
        .block_task()
        .await
        .map_err(refusal)?;
    if hello.protocol_version != ProtocolVersion::V1 {
        return Err(Refusal::Version(hello.protocol_version.to_string()));
    }
    // **Asked only when advertised.** Sending `session/list` to an
    // agent that does not implement it comes back as a method-not-found
    // error, and an error is what khor shows when something went wrong
    // — while nothing went wrong here at all.
    if hello.agent_capabilities.session_capabilities.list.is_none() {
        return Ok(Listing::Cannot);
    }
    let mut cursor = None;
    let mut cursors = std::collections::HashSet::new();
    let mut ids = std::collections::HashSet::new();
    let mut rows = Vec::new();
    loop {
        let mut request = ListSessionsRequest::new();
        request.cwd = cwd.clone();
        request.cursor = cursor;
        let answer = connection
            .send_request(request)
            .block_task()
            .await
            .map_err(refusal)?;
        for s in answer.sessions {
            if ids.insert(s.session_id.to_string()) {
                rows.push(Recorded {
                    id: s.session_id.to_string(),
                    cwd: s.cwd,
                    title: s.title.filter(|t| !t.trim().is_empty()),
                    updated_at: s.updated_at,
                });
            }
        }
        let Some(next) = answer.next_cursor else {
            break;
        };
        if !cursors.insert(next.clone()) {
            return Err(Refusal::Wont(
                "the agent repeated a session-list cursor".into(),
            ));
        }
        cursor = Some(next);
    }
    Ok(Listing::Sessions(rows))
}

pub async fn start_configured(
    command: &str,
    cwd: PathBuf,
    resume: Option<String>,
    environment: Environment,
) -> Result<(Handle, EventReceiver), Refusal> {
    let agent = AcpAgent::from_str(command).map_err(|e| Refusal::Wont(e.to_string()))?;
    start_transport(agent, None, cwd, resume, environment).await
}

/// Connect through caller-owned streams without giving the SDK ownership of the child.
/// A returned refusal waits for owner cleanup. If this future is externally cancelled,
/// the caller must retain the lifecycle and request/confirm shutdown before retrying.
pub async fn start_configured_on(
    transport: impl agent_client_protocol::ConnectTo<Client>,
    lifecycle: TransportLifecycle,
    cwd: PathBuf,
    resume: Option<String>,
    environment: Environment,
) -> Result<(Handle, EventReceiver), Refusal> {
    start_configured_on_with_facts(transport, lifecycle, cwd, resume, environment, InitializeFacts::default()).await
}

/// Caller-owned facts are filled only by this startup's real Initialize v1
/// response. Missing/version-refused/unfinished initialization stays unknown.
pub async fn start_configured_on_with_facts(
    transport: impl agent_client_protocol::ConnectTo<Client>,
    lifecycle: TransportLifecycle,
    cwd: PathBuf,
    resume: Option<String>,
    environment: Environment,
    initialization: InitializeFacts,
) -> Result<(Handle, EventReceiver), Refusal> {
    let cleanup = lifecycle.clone();
    match start_transport_with_facts(transport, Some(lifecycle), cwd, resume, environment, initialization).await {
        Ok(opened) => Ok(opened),
        Err(refusal) => {
            cleanup.request_shutdown();
            match cleanup.confirmed_exit().await {
                Ok(_) => Err(refusal),
                Err(error) => {
                    let append = |said: String| {
                        format!("{said}\nACP process cleanup was not confirmed: {error}")
                    };
                    Err(match refusal {
                        Refusal::Login(said) => Refusal::Login(append(said)),
                        Refusal::Version(said) => Refusal::Version(append(said)),
                        Refusal::Wont(said) => Refusal::Wont(append(said)),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "event_fence_tests.rs"]
mod event_fence_tests;
#[cfg(all(test, unix))]
mod startup_pressure_tests;

async fn start_transport(
    agent: impl agent_client_protocol::ConnectTo<Client>,
    lifecycle: Option<TransportLifecycle>,
    cwd: PathBuf,
    resume: Option<String>,
    environment: Environment,
) -> Result<(Handle, EventReceiver), Refusal> {
    start_transport_with_facts(agent, lifecycle, cwd, resume, environment, InitializeFacts::default()).await
}

async fn start_transport_with_facts(
    agent: impl agent_client_protocol::ConnectTo<Client>,
    lifecycle: Option<TransportLifecycle>,
    cwd: PathBuf,
    resume: Option<String>,
    environment: Environment,
    initialization: InitializeFacts,
) -> Result<(Handle, EventReceiver), Refusal> {
    let join_sdk_driver = lifecycle.is_none();
    let (events, receiver, event_fence) = event_store::channel();
    type Opened = Result<(ConnectionTo<Agent>, SessionId, Facts, Settings, OpenedBy), Refusal>;
    let (ready_tx, mut ready_rx) = oneshot::channel::<Opened>();
    let (refuse_tx, mut refuse_rx) = oneshot::channel::<Refusal>();
    let refusals = Arc::new(std::sync::Mutex::new(Some(refuse_tx)));
    let (close_tx, close_rx) = oneshot::channel::<()>();
    let (result_tx, result_rx) = tokio::sync::watch::channel(None);
    let close = Arc::new(CloseGate::new(close_tx, result_rx, lifecycle));
    #[cfg(all(test, unix))]
    let startup_pressure = startup_pressure_tests::current();
    #[cfg(all(test, unix))]
    if let Some(pressure) = &startup_pressure {
        pressure.attach(&close);
    }
    let mut opening = Opening(Some(close.clone()));
    let driver_close = close.clone();
    let driver_cleanup = Opening(Some(close.clone()));

    let notes = events.clone();
    #[cfg(all(test, unix))]
    let notes_pressure = startup_pressure.clone();
    let asks = events.clone();
    let ask_refusal = refusals.clone();
    let elicits = events.clone();
    let elicit_refusal = refusals.clone();
    let completions = events.clone();
    let opening_events = events.clone();
    let closed = events.clone();
    let failed = events.clone();

    tokio::spawn(async move {
        let _driver_cleanup = driver_cleanup;
        let mut ready_tx = Some(ready_tx);
        let transport = Client.builder()
            .on_receive_notification(
                async move |note: SessionNotification, _cx| {
                    #[cfg(all(test, unix))]
                    if let Some(pressure) = &notes_pressure { pressure.note().await; }
                    notes.note(&note).await.map_err(agent_client_protocol::util::internal_error)
                }, agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_notification(
                async move |note:agent_client_protocol::schema::v1::CompleteElicitationNotification,_cx| {
                    completions.elicitation_done(&note.elicitation_id.to_string()).await
                        .map_err(agent_client_protocol::util::internal_error)
                }, agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                async move |request:CreateElicitationRequest,responder,connection| {
                    if !elicits.accepts_interactions() {
                        refuse_startup_once(&elicit_refusal, "This agent requires interactive session creation. Complete its setup before opening a session.");
                        return responder.respond(CreateElicitationResponse::new(ElicitationAction::Cancel));
                    }
                    let credit = elicits.interaction_credit(&request).map_err(agent_client_protocol::util::internal_error)?;
                    let(tx,rx)=oneshot::channel();
                    elicits.interaction(Event::Elicit(Elicitation { request, answer:tx, _store_credit:credit }))
                        .map_err(agent_client_protocol::util::internal_error)?;
                    connection.spawn(async move {responder.respond(rx.await.unwrap_or_else(|_|CreateElicitationResponse::new(ElicitationAction::Cancel)))})
                }, agent_client_protocol::on_receive_request!(),
            )
            .on_receive_request(
                async move |request:RequestPermissionRequest,responder,connection| {
                    if !asks.accepts_interactions() {
                        refuse_startup_once(&ask_refusal, "The agent requested permission before a session was available");
                        return responder.respond(RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled));
                    }
                    let credit = asks.interaction_credit(&request).map_err(agent_client_protocol::util::internal_error)?;
                    let(tx,rx)=oneshot::channel::<RequestPermissionOutcome>();
                    asks.interaction(Event::Ask(Ask { request, answer:tx, _store_credit:credit }))
                        .map_err(agent_client_protocol::util::internal_error)?;
                    // A human answer remains outside the protocol event loop.
                    // The question itself owns its memory credit until answer/drop.
                    connection.spawn(async move {
                        let outcome=rx.await.unwrap_or(RequestPermissionOutcome::Cancelled);
                        responder.respond(RequestPermissionResponse::new(outcome))
                    })
                }, agent_client_protocol::on_receive_request!(),
            )
            .connect_with(agent, |connection:ConnectionTo<Agent>| async move {
                let mut opened=open_one(&connection,resume,cwd,environment,&initialization).await;
                if let Ok((session,..))=&opened {
                    if let Err(error)=opening_events.accept_session(session.clone()) {
                        opened=Err(Refusal::Wont(error));
                    }
                }
                // Keep the precise protocol refusal on this bounded ready handoff.
                let refused=opened.as_ref().err().map(|error|error.said().to_owned());
                if let Some(tx)=ready_tx.take() {
                    let _=tx.send(opened.map(|(session,facts,settings,opened_by)| {
                        (connection.clone(),session,facts,settings,opened_by)
                    }));
                }
                if let Some(error)=refused { return Err(agent_client_protocol::util::internal_error(error)); }
                connection.incoming_closed().await;
                Err::<(),_>(agent_client_protocol::util::internal_error("the agent closed the connection"))
            });
        let (cancelled, error) = tokio::select! {
            biased;
            _=close_rx=>(true,None),
            error=failed.failed()=>(true,Some(error)),
            result=transport=>(false,result.err().map(|error|error.to_string())),
        };
        let abort_io = cancelled && (error.is_some() || !closed.receiver_released());
        let io_error = if abort_io {
            closed.cancel(
                error
                    .clone()
                    .unwrap_or_else(|| "ACP transport closing".into()),
            );
            closed.shutdown().await.err()
        } else {
            // Reserve the logical Closed position before awaiting actual process
            // confirmation. It is not publishable until those facts are known.
            closed.begin_close();
            None
        };
        let mut completion = driver_close.driver_closed().await;
        if let Some(error) = &io_error {
            completion = Err(error.clone());
        }
        let error = error
            .or(io_error)
            .or_else(|| completion.as_ref().err().cloned());
        if !abort_io {
            closed.closed(error);
        }
        result_tx.send_replace(Some(completion));
    });

    let opened = tokio::select! {
        biased;
        result=&mut ready_rx=>match result {
            Ok(opened)=>opened,
            Err(_)=>Err(Refusal::Wont(events.opening_ended().await)),
        },
        refusal=&mut refuse_rx=>Err(refusal.unwrap_or_else(|_|Refusal::Wont("The agent disconnected".into()))),
        error=events.opening_ended()=>Err(Refusal::Wont(error)),
    };
    let (conn, session, facts, settings, opened_by) = match opened {
        Ok(opened) => opened,
        Err(refusal) => {
            return Err(refuse_and_close(&events, &close, join_sdk_driver, refusal).await)
        }
    };
    if let Ok(refusal) = refuse_rx.try_recv() {
        return Err(refuse_and_close(&events, &close, join_sdk_driver, refusal).await);
    }
    if let Err(error) = events.ready(session.clone()) {
        return Err(refuse_and_close(&events, &close, join_sdk_driver, Refusal::Wont(error)).await);
    }
    #[cfg(all(test, unix))]
    if let Some(pressure) = &startup_pressure {
        pressure.opened();
    }
    Ok((
        Handle {
            conn,
            session,
            facts,
            settings,
            opened_by,
            close: opening.0.take().expect("opening owns its close gate"),
            event_fence,
        },
        receiver,
    ))
}

fn refuse_startup_once(
    signal: &Arc<std::sync::Mutex<Option<oneshot::Sender<Refusal>>>>,
    message: &str,
) {
    let sender = signal
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take();
    if let Some(sender) = sender {
        let _ = sender.send(Refusal::Wont(message.to_owned()));
    }
}

async fn refuse_and_close(
    events: &event_store::Sender,
    close: &CloseGate,
    join_sdk_driver: bool,
    mut refusal: Refusal,
) -> Refusal {
    close.request();
    events.cancel("ACP opening was refused".into());
    if let Err(error) = events.shutdown().await {
        refusal = append_refusal(
            refusal,
            format!("ACP event spool cleanup was not confirmed: {error}"),
        );
    }
    // Caller-owned process confirmation remains with start_configured_on's
    // existing lifecycle wrapper. The SDK-owned path joins its driver here.
    if join_sdk_driver {
        if let Err(error) = close.completed().await {
            refusal = append_refusal(
                refusal,
                format!("ACP driver cleanup was not confirmed: {error}"),
            );
        }
    }
    refusal
}
fn append_refusal(refusal: Refusal, context: String) -> Refusal {
    let append = |said: String| format!("{said}\n{context}");
    match refusal {
        Refusal::Login(said) => Refusal::Login(append(said)),
        Refusal::Version(said) => Refusal::Version(append(said)),
        Refusal::Wont(said) => Refusal::Wont(append(said)),
    }
}


fn client_capabilities() -> ClientCapabilities {
    serde_json::from_value(serde_json::json!({"session":{"configOptions":{"boolean":{}},"compaction":{},"notices":{}},"elicitation":{"form":{},"url":{}}})).expect("client capabilities")
}

/// The handshake and the one session, or the reason there is neither.
///
/// Split out of the connection closure so every exit is a [`Refusal`]
/// with a kind on it: inside the closure the natural spelling is `?`,
/// which flattens all three kinds into one protocol error and loses
/// exactly the distinction this exists to keep.
async fn open_one(
    connection: &ConnectionTo<Agent>,
    resume: Option<String>,
    cwd: PathBuf,
    environment: Environment,
    initialization: &InitializeFacts,
) -> Result<(SessionId, Facts, Settings, OpenedBy), Refusal> {
    if environment
        .additional_directories
        .iter()
        .any(|p| !p.is_absolute())
    {
        return Err(Refusal::Wont(
            "Workspace roots must be absolute on the execution node".into(),
        ));
    }
    let mut initialize = InitializeRequest::new(ProtocolVersion::V1);
    initialize.client_capabilities = client_capabilities();
    let hello = connection
        .send_request(initialize)
        .block_task()
        .await
        .map_err(refusal)?;
    // **Version is negotiated, not assumed.** The protocol has the
    // agent answer with the version it will actually speak, which need
    // not be the one asked for; an unchecked mismatch is a handshake
    // that succeeds and a first request that cannot, with nothing in
    // between to look at.
    if hello.protocol_version != ProtocolVersion::V1 {
        return Err(Refusal::Version(hello.protocol_version.to_string()));
    }
    let protocol = serde_json::json!({"capabilities":hello.agent_capabilities,"authMethods":hello.auth_methods,"agentInfo":hello.agent_info});
    let caps = &hello.agent_capabilities;
    initialization.initialized(RecoveryCapabilities {
        load: caps.load_session,
        resume: caps.session_capabilities.resume.is_some(),
    })?;
    if !environment.additional_directories.is_empty()
        && caps.session_capabilities.additional_directories.is_none()
    {
        return Err(Refusal::Wont(
            "The agent does not support additional workspace roots".into(),
        ));
    }
    for server in &environment.mcp_servers {
        let value = serde_json::to_value(server).expect("MCP server");
        if value["type"] == "http" && !caps.mcp_capabilities.http {
            return Err(Refusal::Wont(
                "The agent does not support HTTP MCP servers".into(),
            ));
        }
        if value["type"] == "sse" && !caps.mcp_capabilities.sse {
            return Err(Refusal::Wont(
                "The agent does not support SSE MCP servers".into(),
            ));
        }
    }
    let (replays, images, embeds, lists) = (
        caps.load_session,
        caps.prompt_capabilities.image,
        caps.prompt_capabilities.embedded_context,
        caps.session_capabilities.list.is_some(),
    );
    let needs_login = !hello.auth_methods.is_empty();
    let facts = Facts {
        protocol,
        replays,
        name: hello.agent_info.map(|i| i.name),
        images,
        embeds,
        lists,
        // **Advertising a way in means there is a door.** An agent with
        // no auth methods at all cannot be asking anyone to log in;
        // one that lists them is telling a client to expect it. Read as
        // a boolean rather than carried whole because khor cannot walk
        // anybody through a vendor's login anyway — what a face can do
        // with this is say why nothing opened, which needs one bit.
        needs_login,
    };
    // **The settings ride on the same answer the session id does**, and
    // they used to be dropped with it — `session_id` was read and the
    // rest of the response discarded. Nothing broke, because an agent
    // that lists modes and is never asked about them behaves exactly
    // like one that lists none; what was missing was the whole surface
    // (a face could see the mode a session was in, and had no way to
    // put it in another).
    let (session, settings, opened_by) = match resume {
        Some(sid) => {
            let sid = SessionId::new(sid);
            if caps.session_capabilities.resume.is_some() {
                let mut request = ResumeSessionRequest::new(sid.clone(), cwd.clone());
                request.mcp_servers = environment.mcp_servers;
                request.additional_directories = environment.additional_directories;
                let loaded = connection
                    .send_request(request)
                    .block_task()
                    .await
                    .map_err(refusal)?;
                return Ok((
                    sid,
                    Facts { ..facts },
                    Settings {
                        model: model_in(loaded.meta.as_ref()),
                        modes: loaded.modes,
                        options: loaded.config_options.unwrap_or_default(),
                    },
                    OpenedBy::Resume,
                ));
            }
            if !caps.load_session {
                return Err(Refusal::Wont(
                    "The agent does not support loading or resuming this session".into(),
                ));
            }
            let mut request = LoadSessionRequest::new(sid.clone(), cwd);
            request.mcp_servers = environment.mcp_servers;
            request.additional_directories = environment.additional_directories;
            let loaded = connection
                .send_request(request)
                .block_task()
                .await
                .map_err(refusal)?;
            (
                sid,
                Settings {
                    model: model_in(loaded.meta.as_ref()),
                    modes: loaded.modes,
                    options: loaded.config_options.unwrap_or_default(),
                },
                OpenedBy::Load,
            )
        }
        None => {
            let mut request = NewSessionRequest::new(cwd);
            request.mcp_servers = environment.mcp_servers;
            request.additional_directories = environment.additional_directories;
            let opened = connection
                .send_request(request)
                .block_task()
                .await
                .map_err(refusal)?;
            (
                opened.session_id,
                Settings {
                    model: model_in(opened.meta.as_ref()),
                    modes: opened.modes,
                    options: opened.config_options.unwrap_or_default(),
                },
                OpenedBy::New,
            )
        }
    };
    Ok((session, facts, settings, opened_by))
}

/// Preserve the diagnostic carried in ACP error.data. SDK 3 rejects a wire
/// version mismatch before returning InitializeResponse to our own guard.
fn refusal(e: agent_client_protocol::Error) -> Refusal {
    let detail = e.data.as_ref().and_then(|data| data.as_str());
    let said = match detail {
        Some(detail) => format!("{}: {detail}", e.message),
        None => e.message.clone(),
    };
    match e.code {
        ErrorCode::AuthRequired => Refusal::Login(said),
        ErrorCode::InvalidRequest
            if detail.is_some_and(|detail| {
                detail.starts_with("unsupported ACP protocol version ")
                    || detail.starts_with("required ACP protocol version ")
            }) =>
        {
            Refusal::Version(said)
        }
        _ => Refusal::Wont(said),
    }
}

#[cfg(test)]
mod what_khor_grants {
    use super::*;

    /// **khor does not let an agent run commands on this machine**, and
    /// this is the assertion that says so out loud.
    ///
    /// # What the capability actually grants
    ///
    /// `terminal/create`, `terminal/output`, `terminal/kill` and their
    /// siblings are **agent→client** requests: the agent asks *khor* to
    /// start a process here and hands back a `TerminalId`. The v1
    /// schema is explicit that they exist only when the client set
    /// `terminal: true`. So the flag is not a rendering option — it is
    /// the difference between an agent that talks and an agent that
    /// runs things on the user's computer through khor.
    ///
    /// # Why it is a test and not a comment
    ///
    /// This started life as a line on a ledger reading "`ToolCallContent
    /// ::Terminal` — a tool whose output is a live terminal rather than
    /// text, unpainted". Read that way it sounds like a missing view,
    /// and the obvious next step is to go paint it.
    ///
    /// **It is not reachable at all**: a conforming agent cannot put a
    /// terminal id in a tool call without having been given one, and
    /// khor gives none. What the ledger line described as a gap in the
    /// face is a door that is shut, and the thing worth guarding is the
    /// door — a `true` written here one day, for a reason that sounded
    /// small, would open it with nothing to notice.
    ///
    /// Asserted on the **serialized request**, because that is what an
    /// agent reads. A default that changed meaning in a future schema
    /// would pass a test that only read the Rust field.
    #[test]
    fn the_handshake_does_not_offer_an_agent_a_terminal_on_this_machine() {
        let mut hello = InitializeRequest::new(ProtocolVersion::V1);
        hello.client_capabilities = client_capabilities();
        let wire = serde_json::to_value(&hello).expect("the handshake must serialize");
        let caps = &wire["clientCapabilities"];

        // The live probe that earns the negative below: the object khor
        // sends really does carry capabilities, so "terminal is not
        // true" is a reading of something rather than of nothing.
        assert!(
            caps.is_object(),
            "probe dead — the handshake carries no capabilities at all: {wire}"
        );
        assert!(
            caps.get("terminal").is_some(),
            "probe dead — the key being asserted on is not in the object at all: {wire}"
        );

        assert!(caps["session"]["configOptions"]["boolean"].is_object());
        assert!(caps["session"]["compaction"].is_object());
        assert!(caps["session"]["notices"].is_object());
        assert!(caps["elicitation"]["form"].is_object());
        assert!(caps["elicitation"]["url"].is_object());
        // Execution on the GUI remains disabled:
        //   {"fs":{"readTextFile":false,"writeTextFile":false},"terminal":false}
        assert_eq!(
            caps["terminal"],
            serde_json::json!(false),
            "khor must not offer an agent a terminal on this machine: {caps}",
        );

        // **The same door, one hinge over.** `fs/read_text_file` and
        // `fs/write_text_file` are agent→client too: they let the agent
        // read and write this machine's files through khor. Nothing
        // here needs them either — an ACP agent khor starts is already
        // running on this machine with its own hands — and a capability
        // granted "so it works" is granted for as long as nobody looks.
        assert_eq!(
            caps["fs"],
            serde_json::json!({ "readTextFile": false, "writeTextFile": false }),
            "khor must not offer an agent this machine's files: {caps}",
        );
    }
}
