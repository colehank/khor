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
//! `session/update`s — khor's takeover rides it. What stays outside
//! the stable protocol: enumerating sessions (`session/list` is an
//! unaccepted RFD, answered by a *running* agent, with on-disk
//! persistence explicitly left undefined and no known adopters),
//! no-replay resume and fork (v2 alpha / draft), choosing and spawning
//! the agent binary (client-side by the protocol's own starting line),
//! and reading a dead agent's history without spawning anything —
//! which is exactly what khor's adaptors do for discovery, usage and
//! the recorded-past view. If `session/list` lands and vendors adopt
//! it, "whose session is this id" becomes askable over the wire; until
//! then the vendor fork lives in the adaptors, one file-format each.
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
use tokio::sync::{mpsc, oneshot};

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
    /// The agent replays its own past (`session/load`). When false,
    /// [`Handle::replay`] is not worth asking and a face must say the
    /// past is unavailable rather than paint an empty one.
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
}

/// What the connection tells the caller, in protocol shape.
#[derive(Debug)]
pub enum Event {
    /// The session exists; prompts may be sent.
    Ready { session: SessionId },
    /// One `session/update` — conversation chunks, tool calls, plans.
    /// Raw on purpose: this crate maps nothing onto khor's words.
    Note(SessionNotification),
    /// The agent asked permission and is now waiting on [`Ask`].
    Ask(Ask),
    /// The connection ended. `None` is a close the caller asked for;
    /// `Some` carries the error text of one it did not.
    Closed(Option<String>),
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

/// A live conversation with one agent process.
pub struct Handle {
    conn: ConnectionTo<Agent>,
    session: SessionId,
    /// Where the session lives — kept because `session/load` must name
    /// the same cwd the session was opened with (the real adapter
    /// refuses a mismatch).
    cwd: PathBuf,
    facts: Facts,
    settings: Settings,
    /// Dropping this ends the connection's main task, which drops the
    /// transport, which kills the agent's process group.
    _close: oneshot::Sender<()>,
}

impl Handle {
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

    /// Replays the whole conversation so far: every replayed update
    /// arrives as [`Event::Note`], and **all of them are already on the
    /// receiver when this resolves** — the protocol sends them before
    /// the `session/load` response, and the connection delivers the
    /// stream in order, so a caller may drain without waiting.
    ///
    /// Loading a session that is live on this very connection works
    /// (probed on the real adapter, 2026-08-17: a second load answers
    /// with the same replay) — but an agent replays from its own
    /// transcript, so a session that has never had a turn may refuse
    /// with "not found" rather than replay nothing.
    pub async fn replay(&self) -> Result<(), String> {
        let request = LoadSessionRequest::new(self.session.clone(), self.cwd.clone());
        self.conn
            .send_request(request)
            .block_task()
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Interrupt the current turn without ending the session.
    pub fn cancel(&self) -> Result<(), String> {
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
) -> Result<(Handle, mpsc::UnboundedReceiver<Event>), Refusal> {
    start_with(command, cwd, None).await
}

/// [`start`], but the session is an existing one the agent resumes
/// (`session/load` instead of `session/new`) — the takeover path (批C).
/// The load's replayed updates arrive as [`Event::Note`]s before
/// [`Event::Ready`]; a caller with nobody attached yet simply drops
/// them, and the past stays askable through [`Handle::replay`].
pub async fn start_resume(
    command: &str,
    cwd: PathBuf,
    session: &str,
) -> Result<(Handle, mpsc::UnboundedReceiver<Event>), Refusal> {
    start_with(command, cwd, Some(session.to_owned())).await
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
    let mut request = ListSessionsRequest::new();
    if let Some(cwd) = cwd {
        request = request.cwd(cwd);
    }
    let answer = connection
        .send_request(request)
        .block_task()
        .await
        .map_err(refusal)?;
    Ok(Listing::Sessions(
        answer
            .sessions
            .into_iter()
            .map(|s| Recorded {
                id: s.session_id.to_string(),
                cwd: s.cwd,
                title: s.title.filter(|t| !t.trim().is_empty()),
                updated_at: s.updated_at,
            })
            .collect(),
    ))
}

async fn start_with(
    command: &str,
    cwd: PathBuf,
    resume: Option<String>,
) -> Result<(Handle, mpsc::UnboundedReceiver<Event>), Refusal> {
    let agent = AcpAgent::from_str(command).map_err(|e| Refusal::Wont(e.to_string()))?;
    let (events_tx, events_rx) = mpsc::unbounded_channel::<Event>();
    type Opened = Result<(ConnectionTo<Agent>, SessionId, Facts, Settings), Refusal>;
    let (ready_tx, ready_rx) = oneshot::channel::<Opened>();
    let (close_tx, close_rx) = oneshot::channel::<()>();

    let notes = events_tx.clone();
    let asks = events_tx.clone();
    let closed = events_tx.clone();
    let home = cwd.clone();

    tokio::spawn(async move {
        let mut ready_tx = Some(ready_tx);
        let outcome = Client
            .builder()
            .on_receive_notification(
                async move |n: SessionNotification, _cx| {
                    let _ = notes.send(Event::Note(n));
                    Ok(())
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                async move |request: RequestPermissionRequest, responder, connection| {
                    // The wait lives in a spawned task: a person can take
                    // minutes to answer, and the event loop must keep
                    // moving `session/update`s meanwhile (module head).
                    let (tx, rx) = oneshot::channel::<RequestPermissionOutcome>();
                    let _ = asks.send(Event::Ask(Ask {
                        request,
                        answer: tx,
                    }));
                    connection.spawn(async move {
                        // A dropped `Ask` answers `Cancelled`: the agent
                        // gets a refusal, never a hang.
                        let outcome = rx.await.unwrap_or(RequestPermissionOutcome::Cancelled);
                        responder.respond(RequestPermissionResponse::new(outcome))
                    })
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(agent, |connection: ConnectionTo<Agent>| async move {
                let opened = open_one(&connection, resume, cwd).await;
                // The refusal has to leave through the ready channel,
                // because the alternative is what this used to do: fail
                // out of the closure, land the text on `Event::Closed`
                // — a receiver the caller never got, since it only
                // arrives with the Handle — and answer the caller with
                // a sentence about a channel. Every way an agent can
                // decline then read as "it ended", and the opener's own
                // timeout became the only thing a person ever saw.
                let refused = opened.as_ref().err().map(|r| r.said().to_owned());
                if let Some(tx) = ready_tx.take() {
                    let _ = tx.send(opened.map(|(session, facts, settings)| {
                        (connection.clone(), session, facts, settings)
                    }));
                }
                if let Some(said) = refused {
                    return Err(agent_client_protocol::util::internal_error(said));
                }
                // Park until the Handle goes away **or the agent's side
                // of the pipe does**. The crate treats a clean incoming
                // EOF as observable, never terminal (`connect_with`'s
                // contract), so without the second arm a dead agent
                // leaves this closure parked and the caller's loop
                // waiting for a Closed that never comes — found live: a
                // gui host whose agent died hung in its conversation
                // loop until killed by hand. The EOF arm is an error on
                // purpose: the caller did not ask for this ending, and
                // Closed(None) is reserved for the one it did.
                tokio::select! {
                    _ = close_rx => Ok(()),
                    _ = connection.incoming_closed() => {
                        Err(agent_client_protocol::util::internal_error(
                            "the agent closed the connection",
                        ))
                    }
                }
            })
            .await;
        let _ = closed.send(Event::Closed(outcome.err().map(|e| e.to_string())));
    });

    let (conn, session, facts, settings) = ready_rx
        .await
        .map_err(|_| Refusal::Wont("the agent ended before a session existed".to_owned()))??;
    let _ = events_tx.send(Event::Ready {
        session: session.clone(),
    });
    Ok((
        Handle {
            conn,
            session,
            cwd: home,
            facts,
            settings,
            _close: close_tx,
        },
        events_rx,
    ))
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
) -> Result<(SessionId, Facts, Settings), Refusal> {
    let hello = connection
        .send_request(InitializeRequest::new(ProtocolVersion::V1))
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
    let caps = &hello.agent_capabilities;
    let (replays, images, embeds, lists) = (
        caps.load_session,
        caps.prompt_capabilities.image,
        caps.prompt_capabilities.embedded_context,
        caps.session_capabilities.list.is_some(),
    );
    let needs_login = !hello.auth_methods.is_empty();
    let facts = Facts {
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
    let (session, settings) = match resume {
        Some(sid) => {
            let sid = SessionId::new(sid);
            let loaded = connection
                .send_request(LoadSessionRequest::new(sid.clone(), cwd))
                .block_task()
                .await
                .map_err(refusal)?;
            (
                sid,
                Settings {
                    modes: loaded.modes,
                    options: loaded.config_options.unwrap_or_default(),
                },
            )
        }
        None => {
            let opened = connection
                .send_request(NewSessionRequest::new(cwd))
                .block_task()
                .await
                .map_err(refusal)?;
            (
                opened.session_id,
                Settings {
                    modes: opened.modes,
                    options: opened.config_options.unwrap_or_default(),
                },
            )
        }
    };
    Ok((session, facts, settings))
}

/// One JSON-RPC error, sorted. Only `auth_required` gets its own kind:
/// it is the only code in the protocol whose remedy belongs to the
/// person rather than to khor.
fn refusal(e: agent_client_protocol::Error) -> Refusal {
    match e.code {
        ErrorCode::AuthRequired => Refusal::Login(e.message),
        _ => Refusal::Wont(e.message),
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
        let hello = InitializeRequest::new(ProtocolVersion::V1);
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

        // Measured 2026-09-01, the whole of what khor grants:
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
