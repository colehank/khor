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
    CancelNotification, ContentBlock, InitializeRequest, LoadSessionRequest, NewSessionRequest,
    PermissionOptionId, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionId, SessionNotification,
    StopReason, TextContent,
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
        let _ = self
            .answer
            .send(RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)));
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
    /// Dropping this ends the connection's main task, which drops the
    /// transport, which kills the agent's process group.
    _close: oneshot::Sender<()>,
}

impl Handle {
    /// One turn: send the text, resolve when the agent finishes it.
    /// Updates stream through [`Event::Note`] while this is pending.
    pub async fn prompt(&self, text: &str) -> Result<StopReason, String> {
        let request = PromptRequest::new(
            self.session.clone(),
            vec![ContentBlock::Text(TextContent::new(text.to_owned()))],
        );
        let response =
            self.conn.send_request(request).block_task().await.map_err(|e| e.to_string())?;
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
        self.conn.send_request(request).block_task().await.map(|_| ()).map_err(|e| e.to_string())
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

async fn start_with(
    command: &str,
    cwd: PathBuf,
    resume: Option<String>,
) -> Result<(Handle, mpsc::UnboundedReceiver<Event>), Refusal> {
    let agent = AcpAgent::from_str(command).map_err(|e| Refusal::Wont(e.to_string()))?;
    let (events_tx, events_rx) = mpsc::unbounded_channel::<Event>();
    type Opened = Result<(ConnectionTo<Agent>, SessionId, Facts), Refusal>;
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
                    let _ = asks.send(Event::Ask(Ask { request, answer: tx }));
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
                    let _ = tx.send(
                        opened.map(|(session, facts)| (connection.clone(), session, facts)),
                    );
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

    let (conn, session, facts) = ready_rx
        .await
        .map_err(|_| Refusal::Wont("the agent ended before a session existed".to_owned()))??;
    let _ = events_tx.send(Event::Ready { session: session.clone() });
    Ok((Handle { conn, session, cwd: home, facts, _close: close_tx }, events_rx))
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
) -> Result<(SessionId, Facts), Refusal> {
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
    let facts = Facts {
        replays: hello.agent_capabilities.load_session,
        name: hello.agent_info.map(|i| i.name),
    };
    let session = match resume {
        Some(sid) => {
            let sid = SessionId::new(sid);
            connection
                .send_request(LoadSessionRequest::new(sid.clone(), cwd))
                .block_task()
                .await
                .map_err(refusal)?;
            sid
        }
        None => {
            connection
                .send_request(NewSessionRequest::new(cwd))
                .block_task()
                .await
                .map_err(refusal)?
                .session_id
        }
    };
    Ok((session, facts))
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
