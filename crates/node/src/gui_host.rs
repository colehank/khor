//! The GUI session's host: one detached process holding one ACP agent.
//!
//! # The second way to open a session (user ruling, 2026-08-17)
//!
//! `khor open --tui` gives the agent a terminal and khor reads its
//! screen; `khor open --gui` gives the agent a **protocol** and khor is
//! the client. Same store, same registry, same six words — what differs
//! is the channel, so this file is `host.rs`'s sibling, not its mode.
//!
//! # The row is registered under the vendor's own session id
//!
//! Registration waits until `session/new` answers, and the id is
//! [`crate::adaptor::id_for`] of the vendor's uuid — the same spelling
//! the disk sweep and the hook path mint. That single decision is what
//! keeps one conversation to one row with no dedup anywhere: the sweep
//! merges into this row by id, and a hook arriving without
//! `KHOR_SESSION` lands on this row too and fills the category khor
//! deliberately does not guess (the argv ruling: an adapter's command
//! name is not a vendor). The id is spelled `tui/…` because that is the
//! vendor-session agreement; **behaviour follows `Meta::kind`, which
//! says `gui`** — an id is an address, not a description.
//!
//! # Words, first-hand
//!
//! Every word here is [`crate::live::Source::Reported`]: khor is the
//! protocol client, so "a prompt is in flight" and "the agent asked
//! permission" are its own facts, not readings of anything. 空闲 while
//! nothing is asked of the agent; 忙碌 while a prompt is pending; 待批
//! while a permission request waits — and the answer clears it back to
//! 忙碌, because the turn is still running (待批答才清). The turn's end
//! maps `EndTurn` → 完成, `Cancelled` → 空闲 (stopping it was the
//! user's own act), and the refusals and ceilings → 中断: the session
//! is alive and continuing needs no restart, which is 中断's exact
//! boundary with 失败.
//!
//! 完成 from a protocol turn-end widens "`Stop` is the only source of
//! 完成": that sentence was written when hooks were the only party that
//! could *know* a turn ended. Here khor is the party the turn ends
//! *at*. Same fact, second first-hand witness — on the ledger.
//!
//! # What the socket speaks
//!
//! The host file and cookie handshake are `host.rs`'s, reused verbatim.
//! After the welcome the stream carries [`GuiNote`] frames out and
//! [`GuiOp`] frames in — length-prefixed msgpack like everything else
//! on this socket. Enum variants use names; their payload/struct fields are
//! positional. The GUI greeting gates incompatible capabilities/frame sets.
//! Live updates are broadcast. Each view asks for a token-bound canonical
//! Capture and reads its own bounded page/raw cursor; it never reloads the
//! running agent. The first supported Load/Resume commits its finite opening
//! batch atomically before the host publishes readiness.

use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    SessionConfigOption, SessionConfigOptionValue, SessionModeId, SessionUpdate,
};
use khor_catalog::msg;
use khor_core::{SessionId, State};
use serde::{Deserialize, Serialize};

use crate::host::{Hello, HostFile, Welcome, host_file_path, read_frame, write_frame};
use crate::live::{LiveKind, Source};
use crate::session_content::journal::{Identity as JournalIdentity, Journal, JournalReservation, Opening as JournalOpening, Outcome as JournalOutcome};

/// What a connected face may do to the conversation.
#[derive(Serialize, Deserialize)]
pub enum GuiOp {
    /// One user turn. The host runs it; updates come back as notes.
    Say(String),
    /// The answer to [`GuiNote::Ask`] with this number: the chosen
    /// option id, or `None` to refuse.
    Answer {
        ask: u64,
        option: Option<String>,
    },
    /// End the conversation: the agent dies with the connection.
    Close,
    /// End the turn that is running — not the session. The agent stays,
    /// its conversation stays, and the ending is `Cancelled`, which is
    /// 空闲: stopping it was the user's own act, so nothing is broken.
    /// Tail-appended.
    Stop,
    /// One user turn with images attached.
    ///
    /// A separate payload keeps [`GuiOp::Say`]'s ordinary text shape explicit.
    ///
    /// `(mime, base64)` pairs. The data is carried rather than a path
    /// because the face that has the image may be on another machine
    /// entirely — a web face on a phone pasting a screenshot has no
    /// path this host could open.
    /// `images` are `(mime, base64)`; `files` are `(name, text)` and
    /// ride as the protocol's embedded resources.
    ///
    /// The client token correlates its optimistic draft with the canonical
    /// accepted prompt. It does not identify or deduplicate an agent turn.
    SayWith {
        text: String,
        images: Vec<(String, String)>,
        files: Vec<(String, String)>,
        token: Option<String>,
    },
    /// Put the session in one of the modes the agent listed
    /// ([`GuiNote::Settings`]), by the agent's own id. Tail-appended.
    /// Answered by a fresh `Settings` to every face when it took, and by
    /// [`GuiNote::Refused`] (`refused::SETTING_REFUSED`) to the asker
    /// when the agent said no.
    SetMode(String),
    /// Set one of the agent's options to a listed value or a switch
    /// position. Tail-appended, and answered the same two ways.
    SetOption {
        id: String,
        value: Choice,
    },
    /// **One slice of an op too big for one frame** (a turn carrying
    /// photos): the op's own frame bytes, in order. The host keeps them
    /// per connection and runs the op at [`GuiOp::Parts`]. Ops sent
    /// between slices (a stop, an answer) run as they arrive — that is
    /// why a big op is sliced and not sent as one frame, which would hold
    /// the connection, and over a slow road the face, for its whole
    /// length (2026-10-04: two phone photos, 8.5 MB, held the face for
    /// minutes and were refused by the frame cap at the end). Tail-
    /// appended; written by [`write_op`].
    Part(serde_bytes::ByteBuf),
    /// The slices sent so far are one op: run it. Tail-appended.
    Parts,
    /// Authenticated local controller starts one durable A2A input. Tail-only;
    /// old hosts refuse it, and the controller reports that failure.
    Delegate {
        task: String,
        message: String,
    },
    ElicitAnswer {
        ask: u64,
        response: String,
    },
    Control {
        token: String,
        operation: String,
        payload: String,
    },
    /// A per-view read boundary; it never asks the agent to replay.
    Capture { token: [u8; 16] },
}

pub const CAPTURE_WIRE_BYTES: usize = 20 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureFence {
    pub binding: crate::session_content::ReadBinding,
    pub cursor: crate::adaptor::transcript_page::Cursor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureFailure {
    NotReady,
    Unavailable,
}

fn captured_note(token: [u8; 16], result: Result<CaptureFence, CaptureFailure>) -> GuiNote {
    let note = GuiNote::Captured { token, result };
    if crate::proto::encode(&note).is_ok_and(|bytes| bytes.len() <= CAPTURE_WIRE_BYTES) {
        note
    } else {
        GuiNote::Captured { token, result: Err(CaptureFailure::Unavailable) }
    }
}

/// How big one [`GuiOp::Part`] is: well under the host's frame cap, and
/// small enough that an op sent between two slices waits for at most
/// one of them.
pub const PART: usize = 256 * 1024;

/// The most an op sent in slices may come to. A turn's attachments are
/// pictures and text files a person picked; past this the face says
/// so before sending, and the host drops what it was sent.
pub const MAX_PARTS: usize = 64 * 1024 * 1024;

const GUI_GREETING: &str = "khor-gui/2";

pub struct Attached {
    pub stream: TcpStream,
}

/// GUI and terminal connections have distinct greetings and frame protocols.
pub fn connect_at(addr: &str, cookie: &str) -> Result<Attached, String> {
    let (stream, welcome) = crate::host::connect_welcomed(addr, cookie, 0, 0)?;
    if welcome.why != GUI_GREETING {
        return Err(msg::host_gui_protocol().into());
    }
    let _ = stream.set_read_timeout(None);
    let _ = stream.set_write_timeout(None);
    Ok(Attached { stream })
}

pub fn connect(dir: &std::path::Path) -> Result<Attached, String> {
    let host = crate::host::read_host_file(dir)?;
    connect_at(&format!("127.0.0.1:{}", host.port), &host.cookie)
}

#[derive(Clone, Copy)]
pub enum WriteStep {
    Whole,
    Part,
    Commit,
}

/// Writes one op to a host connection — **in slices when it is too big
/// for one frame** ([`GuiOp::Part`]), taking `conn` for each slice
/// alone, so other ops on the same connection go between them.
/// `progress` hears (bytes written, bytes in all) after each slice.
pub fn write_op(
    conn: &Mutex<TcpStream>,
    op: &GuiOp,
    progress: impl FnMut(usize, usize),
) -> Result<(), String> {
    write_op_guarded(conn, op, progress, |_| Ok(()))
}

/// The guard runs with the writer held, immediately before each frame. It can
/// cancel uncommitted input without racing the final Parts/whole-frame write.
pub fn write_op_guarded(
    conn: &Mutex<TcpStream>,
    op: &GuiOp,
    mut progress: impl FnMut(usize, usize),
    mut before: impl FnMut(WriteStep) -> Result<(), String>,
) -> Result<(), String> {
    let mut write = |op: &GuiOp, step| {
        let mut writer = conn.lock().unwrap_or_else(|p| p.into_inner());
        before(step)?;
        write_frame(&mut *writer, op)
    };
    let bytes = crate::proto::encode(op)?;
    if bytes.len() <= PART {
        write(op, WriteStep::Whole)?;
        progress(bytes.len(), bytes.len());
        return Ok(());
    }
    if bytes.len() > MAX_PARTS {
        return Err(msg::frame_too_big(bytes.len()));
    }
    let mut sent = 0;
    for slice in bytes.chunks(PART) {
        write(
            &GuiOp::Part(serde_bytes::ByteBuf::from(slice)),
            WriteStep::Part,
        )?;
        sent += slice.len();
        progress(sent, bytes.len());
    }
    write(&GuiOp::Parts, WriteStep::Commit)
}

/// One connection's ops as they are read, slices put back together
/// ([`GuiOp::Part`]): what the reader thread hands the conversation
/// loop. `None` for a slice, and for a reassembly that did not decode or
/// ran past [`MAX_PARTS`] — dropped whole at its `Parts`, never half-run.
#[derive(Default)]
struct Reassembly {
    held: Vec<u8>,
    over: bool,
}

impl Reassembly {
    fn has_upload(&self) -> bool {
        self.over || !self.held.is_empty()
    }

    fn take(&mut self, op: GuiOp) -> Option<GuiOp> {
        match op {
            GuiOp::Stop => {
                if self.has_upload() {
                    self.held = Vec::new();
                    self.over = true;
                }
                Some(GuiOp::Stop)
            }
            GuiOp::Part(slice) => {
                if self.over || self.held.len() + slice.len() > MAX_PARTS {
                    self.over = true;
                    self.held = Vec::new();
                } else {
                    self.held.extend_from_slice(&slice);
                }
                None
            }
            GuiOp::Parts => {
                let whole = std::mem::take(&mut self.held);
                if std::mem::take(&mut self.over) {
                    return None;
                }
                match crate::proto::decode::<GuiOp>(&whole) {
                    Ok(GuiOp::Part(_) | GuiOp::Parts) | Err(_) => None,
                    Ok(op) => Some(op),
                }
            }
            op => Some(op),
        }
    }
}

/// What an option is set to: a listed value's id, or a switch's
/// position — the protocol's two shapes, spelled as a khor enum so the
/// frame stays positional msgpack rather than carrying JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Choice {
    Value(String),
    Switch(bool),
}

/// What the host tells every connected face.
///
/// `Clone` because a pending ask is **re-sent to a face that arrives
/// after it was raised**: the row says 待批, and a face that cannot see
/// what it is being asked has no way to unblock it (found live — a
/// second window on a session mid-ask showed a conversation with no
/// buttons and no way forward).
#[derive(Clone, Serialize, Deserialize)]
pub enum GuiNote {
    /// One `session/update`, as the protocol's own JSON. Passed through
    /// whole on purpose: this host relays a conversation, it does not
    /// curate one — what to paint is the face's judgment.
    Note(String),
    /// The agent asked permission. `options` are `(id, label)` pairs in
    /// the agent's own order; answer by number with [`GuiOp::Answer`].
    Ask {
        ask: u64,
        title: String,
        options: Vec<(String, String)>,
    },
    /// The turn ended, with the protocol's stop reason as a string.
    Turn(String),
    /// The conversation is over; no more frames follow.
    Gone,
    /// A turn is already running — told to a face the moment it
    /// attaches, and to nobody else. Without it, a second face opens
    /// on a conversation mid-turn with an inviting empty input, and
    /// what it says is dropped on the floor (one turn at a time is the
    /// host's rule, and silence is how it used to say so).
    /// Tail-appended.
    Turning,
    /// An [`GuiNote::Ask`] has its answer: the option id that was
    /// chosen, or `None` for a refusal. Broadcast, and tail-appended.
    ///
    /// Without it, "answered" lives only in the answering face's own
    /// memory — so the next face to read this conversation paints live
    /// buttons on a permission that was settled minutes ago, and
    /// pressing one answers a request that already has an answer.
    Answered {
        ask: u64,
        option: Option<String>,
    },
    /// What the agent on the other end said it can do, told to a face
    /// the moment it attaches (`khor_acp::Facts`). Tail-appended.
    ///
    /// **`replays` is here because its absence has no shape of its
    /// own.** An agent that cannot replay answers a history request
    /// with nothing, and nothing is exactly what a conversation where
    /// no one has spoken yet looks like — so a face with only the
    /// answer to go on paints "no past" for both, and one of those two
    /// is a lie. Told rather than inferred: the capability is on the
    /// handshake, which happened before any face existed.
    ///
    /// **The other three are here for the same reason and were added
    /// when the reason stopped being hypothetical.** khor now ships a
    /// snapshot of the ACP registry, so the agent on the other end can
    /// be one nobody here wrote — and「支持 ACP」is a spectrum, not a
    /// switch: the registry's own probe on 2026-08-30 had 31 of 32
    /// agents reach `initialize`, 29 do `session/load`, 20 do
    /// `session/list`, **1** do `session/stop`, and 19 refuse to open a
    /// session at all until somebody logs in. A face that offers every
    /// agent everything finds out which of those it is by failing.
    Agent {
        name: Option<String>,
        replays: bool,
        /// Images may be attached to a prompt.
        images: bool,
        /// Files may ride along as embedded context.
        embeds: bool,
        /// The agent can enumerate its own sessions.
        lists: bool,
        /// The agent advertises ways to log in, so「nothing opened」has
        /// a likely cause that a face can name **before** the first
        /// turn instead of after it.
        needs_login: bool,
    },
    /// An op that did not take effect, told to **the face that sent it
    /// and to nobody else**. Tail-appended.
    ///
    /// The two refusals this covers used to be a bare `continue` and a
    /// bare no-op: a face said something and the host dropped it in
    /// silence, so the screen agreed with the person while nothing had
    /// happened. One face cannot reach either (its own `inTurn` blocks
    /// a second line), but **two faces on one session can**, and there
    /// the loss compounds: the dropped `Say` never produces a `Turn`,
    /// so the stop that follows finds no turn and is dropped too, and
    /// the box never comes back.
    ///
    /// `why` is a **key, not a sentence**: keys travel the wire and
    /// words are looked up at the last moment (`zh.toml`'s own rule).
    /// A sentence here would freeze today's wording into the protocol
    /// and reach a face that cannot translate it.
    Refused {
        why: String,
    },
    /// What this session can be set to and what it is set to, as the
    /// protocol's own JSON: `{"modes": SessionModeState | null,
    /// "options": [SessionConfigOption]}` (`khor_acp::Settings`). Told
    /// to a face when it attaches, and to every face whenever it
    /// changes — by a face's own [`GuiOp::SetMode`] / [`GuiOp::SetOption`]
    /// or by the agent's `current_mode_update` / `config_option_update`.
    /// Only sent when there is something in it; an agent that offers
    /// nothing to set is most agents, and silence is their honest
    /// shape. Tail-appended.
    Settings(String),
    /// Sent only to a client which used Delegate; never broadcast to old faces.
    Delegated {
        task: String,
        why: Option<String>,
    },
    Capabilities(String),
    Elicit {
        ask: u64,
        request: String,
    },
    ElicitAnswered {
        ask: u64,
        response: String,
    },
    ControlResult {
        token: String,
        operation: String,
        data: Option<String>,
        error: Option<String>,
    },
    Captured {
        token: [u8; 16],
        result: Result<CaptureFence, CaptureFailure>,
    },
}

/// The two keys [`GuiNote::Refused`] travels under. Constants rather
/// than literals at the two call sites: the face matches on them, and a
/// typo on either end is a refusal that arrives wearing no reason.
pub mod refused {
    /// A turn is already running, so the line did not go in.
    pub const TURN_IN_FLIGHT: &str = "turn-in-flight";
    /// There is no turn to stop.
    pub const NO_TURN: &str = "no-turn";
    /// The agent would not take the mode or the value it was handed.
    pub const SETTING_REFUSED: &str = "setting-refused";
}

/// [`GuiNote::Settings`]'s body: the two lists as the protocol spells
/// them (camelCase, the shapes `crates/acp` carries through).
/// `model` rides beside them: the model id when the agent said it
/// (`khor_acp::MODEL_META`), held here like the rest so a face that
/// attaches after the turn that taught it is still told.
fn settings_json(s: &khor_acp::Settings) -> String {
    serde_json::json!({ "modes": s.modes, "options": s.options, "model": s.model }).to_string()
}

fn has_settings(s: &khor_acp::Settings) -> bool {
    s.modes.is_some() || !s.options.is_empty() || s.model.is_some()
}

/// The complete state returned by a setting change, including an empty list.
enum Setting {
    Mode(String),
    Options { list: Vec<SessionConfigOption> },
}

/// Where the freshly-minted session id is written for the opener —
/// which cannot know it in advance: the id exists only once the agent
/// has answered `session/new`.
pub fn spawn_gui_host(
    root: &std::path::Path,
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    vendor: Option<&str>,
    // `agent`: hand this session khor's own verbs (docs/AGENT.md
    // 调度员). Passed rather than read from the environment — mutating
    // the process environment to reach a child is not thread-safe, and
    // a GUI answering two openings at once would be exactly that race.
    agent: bool,
) -> Result<SessionId, String> {
    spawn_ghost(root, cwd, title, cmd, None, vendor, agent)
}

pub fn spawn_gui_host_context(
    root: &std::path::Path,
    cwd: &std::path::Path,
    title: &str,
    cmd: &[String],
    vendor: &str,
    environment: &khor_acp::Environment,
) -> Result<SessionId, String> {
    spawn_ghost_context(
        root,
        Some(cwd),
        title,
        cmd,
        None,
        Some(vendor),
        false,
        Some(environment),
    )
}

/// [`spawn_gui_host`], but resuming an existing vendor session — the
/// takeover (批C): the ghost loads the recorded conversation instead of
/// opening a fresh one, and re-seats the existing row (`LiveKind::reopen`)
/// instead of registering a new one.
pub fn spawn_gui_host_resume(
    root: &std::path::Path,
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    session: &str,
) -> Result<SessionId, String> {
    // No vendor: a resume re-seats a row that already has one, and
    // `LiveKind::reopen` does not touch a category anyway.
    spawn_ghost(root, cwd, title, cmd, Some(session), None, false)
}

/// [`spawn_gui_host`], but the conversation already exists on the agent
/// and is loaded rather than created.
///
/// **Separate from [`spawn_gui_host_resume`] because a takeover and a
/// reopen differ in what is already there.** A takeover re-seats a row
/// that exists (a live TUI it just ended) and so must not register a
/// vendor; a reopen has no row at all — the conversation was only ever
/// a record on the agent's side — so it registers one like any new
/// session, vendor and all.
pub fn spawn_gui_host_reopen(
    root: &std::path::Path,
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    vendor: Option<&str>,
    session: &str,
) -> Result<SessionId, String> {
    spawn_ghost(root, cwd, title, cmd, Some(session), vendor, false)
}

/// How `_ghost` learns it resumes: env, not argv — the argv convention
/// stays what every existing binary parses (`VIEWER_ENV`'s precedent).
const RESUME_ENV: &str = "KHOR_GHOST_RESUME";
const RECOVERY_ENV: &str = "KHOR_GHOST_RECOVERY";

/// Whose session this is, when the **opener** knows — the wizard's own
/// 智能体 field, which is the user saying it, not khor guessing from an
/// adapter's command name (the argv ruling in the module head covers
/// the guess; a declaration is not a guess). Absent for anything opened
/// without that answer, and a hook may still fill it in later:
/// `LiveKind::learn_category` never overwrites what is already there.
const VENDOR_ENV: &str = "KHOR_GHOST_VENDOR";

fn ghost_command(
    root: &std::path::Path,
    executable: &std::path::Path,
) -> Result<std::process::Command, String> {
    let root =
        std::fs::canonicalize(root).map_err(|error| msg::cant_read(root.display(), error))?;
    let mut command = std::process::Command::new(executable);
    // Only this child receives its session store; concurrent GUI requests keep
    // independent roots without changing the parent process environment.
    command.env("KHOR_HOME", root);
    Ok(command)
}

fn spawn_ghost(
    root: &std::path::Path,
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    resume: Option<&str>,
    vendor: Option<&str>,
    agent: bool,
) -> Result<SessionId, String> {
    spawn_ghost_context(root, cwd, title, cmd, resume, vendor, agent, None)
}
fn spawn_ghost_context(
    root: &std::path::Path,
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    resume: Option<&str>,
    vendor: Option<&str>,
    agent: bool,
    environment: Option<&khor_acp::Environment>,
) -> Result<SessionId, String> {
    spawn_ghost_prepared(root, cwd, title, cmd, resume, vendor, agent, environment, None)
}

/// Explicit stable recovery already owns this exact durable token. Do not pass
/// through the CLI convenience path which chooses a fresh token for a new ask.
pub(crate) fn spawn_gui_host_claimed(
    root: &std::path::Path, cwd: &std::path::Path, title: &str, cmd: &[String],
    native: &JournalIdentity, context: &khor_acp::Environment, claim: &str,
) -> Result<SessionId, String> {
    spawn_ghost_prepared(root, Some(cwd), title, cmd, Some(&native.native_id), Some(&native.agent), false, Some(context), Some(claim))
}

fn spawn_ghost_prepared(
    root: &std::path::Path,
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    resume: Option<&str>,
    vendor: Option<&str>,
    agent: bool,
    environment: Option<&khor_acp::Environment>,
    prepared_claim: Option<&str>,
) -> Result<SessionId, String> {
    if prepared_claim.is_some() && resume.is_none() { return Err("a recovery claim cannot start a New session".into()); }
    let exe = crate::self_exe()?;
    let ready = std::env::temp_dir().join(format!(
        "khor-gui-ready-{}-{}",
        std::process::id(),
        crate::link::fresh_hex()?
    ));
    let mut c = ghost_command(root, &exe)?;
    // A fresh New must never inherit another invocation's claim.
    c.env_remove(RECOVERY_ENV);
    match environment {
        Some(context) => {
            c.env(
                "KHOR_GHOST_CONTEXT",
                serde_json::to_string(context).map_err(|e| e.to_string())?,
            );
        }
        None => {
            c.env_remove("KHOR_GHOST_CONTEXT");
        }
    }
    if let Some(cwd) = cwd {
        c.current_dir(cwd);
    }
    match resume {
        Some(sid) => {
            c.env(RESUME_ENV, sid);
        }
        None => {
            c.env_remove(RESUME_ENV);
        }
    }
    match vendor {
        Some(v) => {
            c.env(VENDOR_ENV, v);
        }
        None => {
            c.env_remove(VENDOR_ENV);
        }
    }
    // The scheduler flag reaches the shim as `cagent`'s `KHOR_AGENT`.
    // **Removed when false**, never merely left alone: a host spawned
    // by a scheduler session inherits its environment, and powers that
    // arrive by inheritance are powers nobody granted.
    if agent {
        c.env("KHOR_AGENT", "1");
    } else {
        c.env_remove("KHOR_AGENT");
    }
    // **The environment launchd does not give a GUI process**, handed
    // down here rather than at each caller. Two of the callers build
    // their argv by hand and so never pass through `Node::agent_launch`
    // (the takeover is one), and what a ghost spawns next is khor's own
    // shim — which then spawns `claude` or `codex`, both of them in
    // `~/.local/bin` and neither of them on launchd's four directories.
    // Putting it on the spawn closes the hole structurally instead of
    // once per call site (`Node::user_path`, `Node::user_locale`).
    c.env(crate::PATH_ENV, crate::Node::user_path());
    c.env("PATH", crate::Node::user_path());
    for (name, value) in crate::Node::user_locale() {
        c.env(name, value);
    }
    c.arg("_ghost")
        .arg(&ready)
        .arg(title)
        .arg("--")
        .args(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    crate::mcp_worker::prepare_durable_child(&mut c);
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let key = khor_net::identity::load_or_create(&root.join(".khor/identity.key"))
        .map_err(|error| error.to_string())?;
    let live = LiveKind::new(root.to_path_buf(), khor_core::DeviceId(*key.public().as_bytes()));
    let mut expected_agent = Some(gui_new_agent(cmd, vendor));
    let claim = if let Some(native_id) = resume {
        if agent { return Err("known-native recovery cannot inject a new scheduler context".into()); }
        let native = match prepared_claim {
            Some(claim) => {
                let (_, native) = live.decode_gui_target(claim, native_id)?;
                if vendor.is_some_and(|vendor| vendor != native.agent) { return Err("prepared claim names another configured agent".into()); }
                native
            }
            None => gui_source_identity_as(&live, khor_core::DeviceId(*key.public().as_bytes()), cmd, native_id, vendor)?,
        };
        expected_agent = Some(native.agent.clone());
        let context = match environment {
            Some(context) => context.clone(),
            None => saved_environment(root, native_id)?,
        };
        let standing = cwd.map(std::path::Path::to_path_buf).or_else(crate::cwd::standing)
            .ok_or("known session directory is unavailable")?;
        let standing = std::fs::canonicalize(standing).map_err(|error| error.to_string())?;
        let launch = serde_json::to_vec(&(&standing, title, cmd, &context)).map_err(|error| error.to_string())?;
        // Freeze exactly the context/cwd hashed into this claim. The child must
        // not reread a newer saved context and silently change the operation.
        c.current_dir(&standing);
        c.env("KHOR_GHOST_CONTEXT", serde_json::to_string(&context).map_err(|error| error.to_string())?);
        let claim = match prepared_claim {
            Some(claim) => claim.to_owned(),
            None => live.prepare_gui_recovery(native, &launch)?,
        };
        c.env(RECOVERY_ENV, &claim);
        Some(claim)
    } else { None };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let child = c.spawn().map_err(|error| {
        let why = msg::host_wont_start(error);
        match claim.as_deref().map(|claim| live.gui_ghost_spawn_failed(claim)).transpose() {
            Ok(_) => why,
            Err(failed) => format!("{why}; recovery reservation is uncertain: {failed}"),
        }
    })?;
    let ghost_pid = child.id();
    drop(child);
    let mut ready_issue = None;
    // Longer than host.rs's wait on purpose: before the marker can
    // exist an agent process has to boot and answer two requests.
    while std::time::Instant::now() < deadline {
        if let Ok(id) = std::fs::read_to_string(&ready) {
            let id = SessionId(id.trim().to_owned());
            if !id.0.is_empty() {
                match live.confirm_gui_ready(&id, ghost_pid, resume, expected_agent.as_deref(), claim.as_deref()) {
                    Ok(()) => {
                        let _ = std::fs::remove_file(&ready);
                        return Ok(id);
                    }
                    // Publishing/busy/error is never permission to return
                    // Opened. Keep the marker and only the original deadline.
                    Err(why) => ready_issue = Some(why),
                }
            }
        }
        // The ghost's own account of why there is no session. Without
        // it the only thing this loop can say is that thirty seconds
        // passed — so an agent that refused in the first half second
        // for a reason it stated plainly ("log in first") was reported
        // as a host that never came up. The ghost's stderr goes to
        // /dev/null and its exit code is not waited on; a file beside
        // the marker is the one channel that already exists here.
        if let Ok(why) = std::fs::read_to_string(refusal_path(&ready)) {
            let _ = std::fs::remove_file(refusal_path(&ready));
            return Err(why);
        }
        std::thread::sleep(deadline.saturating_duration_since(std::time::Instant::now())
            .min(std::time::Duration::from_millis(50)));
    }
    Err(ready_issue.unwrap_or_else(|| msg::ghost_never_ready().into()))
}

/// Where the ghost writes why it will never be ready. Beside the marker
/// rather than instead of it: "there is a session" and "there is not,
/// because…" are two different messages, and a reader that had to tell
/// them apart by parsing one file would have to guess at a session id
/// that happened to look like a sentence.
fn refusal_path(ready: &std::path::Path) -> std::path::PathBuf {
    let mut p = ready.as_os_str().to_owned();
    p.push(".why");
    std::path::PathBuf::from(p)
}

/// khor's sentence for one [`khor_acp::Refusal`].
///
/// **The kind picks the sentence; the agent's own words ride along as
/// evidence.** All three used to be one string, and the commonest of
/// them — an agent that runs fine and wants a login — read exactly like
/// a typo in the command, which sends a person to re-check something
/// that was never wrong.
fn refusal_words(r: &khor_acp::Refusal) -> String {
    match r {
        khor_acp::Refusal::Login(said) => msg::agent_wants_a_login(said),
        khor_acp::Refusal::Version(said) => msg::agent_speaks_another_acp(said),
        khor_acp::Refusal::Wont(said) => msg::agent_wont_talk(said),
    }
}

#[path = "gui_host_fanout.rs"]
mod packet_fanout;

struct Fanout {
    packets: packet_fanout::Fanout,
}
impl Fanout {
    fn new() -> Self {
        Self {
            packets: packet_fanout::Fanout::new(),
        }
    }
    fn encode(note: &GuiNote) -> Result<Arc<[u8]>, String> {
        let body = crate::proto::encode(note)?;
        let length = u32::try_from(body.len())
            .map_err(|_| "GUI frame exceeds the wire length".to_owned())?;
        let mut frame = Vec::with_capacity(4 + body.len());
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(&body);
        Ok(frame.into())
    }
    fn send(&self, note: &GuiNote) {
        match Self::encode(note) {
            Ok(frame) => self.packets.send(frame),
            Err(why) => eprintln!("GUI frame encoding failed: {why}"),
        }
    }
    fn send_to(&self, who: u64, note: &GuiNote) {
        match Self::encode(note) {
            Ok(frame) => self.packets.send_to(who, frame),
            Err(why) => eprintln!("GUI frame encoding failed: {why}"),
        }
    }
}

#[derive(Clone, Copy)]
struct HostTimeouts {
    handshake: std::time::Duration,
    control: std::time::Duration,
    cancel: std::time::Duration,
}
impl Default for HostTimeouts {
    fn default() -> Self {
        Self {
            handshake: std::time::Duration::from_secs(28),
            control: std::time::Duration::from_secs(120),
            cancel: std::time::Duration::from_secs(10),
        }
    }
}

fn gui_environment(root: &std::path::Path, resume: Option<&str>) -> Result<khor_acp::Environment, String> {
    let saved_context = resume
        .and_then(|sid| std::fs::read_to_string(context_path(&root, sid)).ok());
    let mut environment: khor_acp::Environment = std::env::var("KHOR_GHOST_CONTEXT")
        .ok()
        .or(saved_context)
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| format!("Invalid session context: {e}"))?
        .unwrap_or_default();
    if std::env::var(crate::cagent::AGENT_ENV).is_ok() {
        environment.mcp_servers.retain(|s| {
            serde_json::to_value(s)
                .ok()
                .is_none_or(|v| v["name"] != "khor")
        });
        environment.mcp_servers.push(serde_json::from_value(serde_json::json!({"name":"khor","command":crate::self_exe()?.display().to_string(),"args":["mcp"],"env":[{"name":"KHOR_HOME","value":root.display().to_string()}]})).map_err(|e|e.to_string())?);
    }
    Ok(environment)
}

/// The detached process `_ghost` runs. Blocks until the conversation is
/// over, then reports the ending like any host: a recorded exit, even
/// if nobody is watching.
pub fn gui_host_main(
    root: PathBuf,
    ready: PathBuf,
    title: String,
    cmd: Vec<String>,
) -> Result<(), String> {
    gui_host_main_with_timeouts(root, ready, title, cmd, HostTimeouts::default())
}

fn gui_host_main_with_timeouts(
    root: PathBuf,
    ready: PathBuf,
    title: String,
    cmd: Vec<String>,
    timeouts: HostTimeouts,
) -> Result<(), String> {
    let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
        .map_err(|e| e.to_string())?;
    let live = LiveKind::new(root.clone(), khor_core::DeviceId(*key.public().as_bytes()));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    let command = cmd.join(" ");
    // The opener spawned this ghost standing in the person's own
    // directory; `cwd::standing` is that directory even after a node
    // has pinned the process elsewhere (第十四轮 第 4 条).
    let cwd = crate::cwd::standing().unwrap_or_else(std::env::temp_dir);
    let resume = std::env::var(RESUME_ENV).ok();
    let environment = gui_environment(&root, resume.as_deref())?;
    let retained_environment = environment.clone();
    let launch = serde_json::to_vec(&(&cwd, &title, &cmd, &retained_environment))
        .map_err(|error| error.to_string())?;
    // A known native conversation owns the journal's physical writer lock
    // before any upstream process exists. This is not an inherited-FD handoff:
    // the ghost acquires it and later moves that same File into its Journal.
    let mut reservation = resume.as_ref().map(|native_id| {
        let claim = std::env::var(RECOVERY_ENV).map_err(|_| "known-native opening has no preghost recovery claim".to_owned())?;
        let (id, identity) = live.decode_gui_target(&claim, native_id)?;
        if std::env::var(VENDOR_ENV).ok().as_deref().is_some_and(|vendor| vendor != identity.agent) {
            return Err("known opening claim names another configured agent".into());
        }
        let reservation = JournalReservation::take(&root, identity)?;
        let dir = live.dir_of(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        reservation.check_binding(&dir)?;
        Ok::<_, String>(reservation)
    }).transpose().map_err(|why| {
        let _ = std::fs::write(refusal_path(&ready), &why);
        why
    })?;
    let mut recovery = if resume.is_some() {
        let claim = std::env::var(RECOVERY_ENV).map_err(|_| "known-native opening has no preghost recovery claim".to_owned())?;
        Some(live.claim_gui_recovery(
            reservation.as_ref().ok_or("known-native opening has no Journal reservation")?,
            &launch, &claim,
        ).map_err(|why| {
            let failure = live.gui_ghost_spawn_failed(&claim);
            let why = match failure { Ok(()) => why, Err(error) => format!("{why}; claim remains fenced: {error}") };
            let _ = std::fs::write(refusal_path(&ready), &why);
            why
        })?)
    } else { None };
    let (transport, birth) = match crate::acp_process::spawn_observed(&command, |pid, _control| {
        crate::live::observe_gui_birth(pid)
    }) {
        Ok(spawned) => spawned,
        Err(error) => {
            let why = refusal_words(&khor_acp::Refusal::Wont(error));
            let settled = recovery.as_ref().map(|owner| live.settle_gui_recovery(owner, Err(()), 1)).transpose();
            let why = match &settled { Ok(_) => why, Err(failed) => format!("{why}; cleanup publication is uncertain: {failed}") };
            let _ = std::fs::write(refusal_path(&ready), &why);
            if settled.is_err() && reservation.as_ref().is_some_and(|value| value.holds_file()) {
                hold_uncertain_gui_owner(reservation.take(), None, None, recovery.take());
            }
            return Err(why);
        }
    };
    let provider_pid = transport.provider_pid;
    let process_lifecycle = transport.lifecycle.clone();
    let process_control = transport.control.clone();
    let stderr = transport.stderr.clone();
    let mut birth = Some(birth);
    if let Some(owner) = recovery.as_mut() {
        if let Err(why) = live.attach_gui_recovery(owner, &process_control, provider_pid, birth.take().expect("one observation")) {
            let (why, retain) = retire_failed_opening(&rt, &live, &process_lifecycle, recovery.as_ref(), None, why);
            let _ = std::fs::write(refusal_path(&ready), &why);
            if retain { hold_uncertain_gui_owner(reservation.take(), None, None, recovery.take()); }
            return Err(why);
        }
    }
    let initialization = khor_acp::InitializeFacts::default();
    let opened = rt.block_on(async {
        tokio::time::timeout(
            timeouts.handshake,
            khor_acp::start_configured_on_with_facts(
                transport.streams,
                transport.lifecycle,
                cwd.clone(),
                resume.clone(),
                environment,
                initialization.clone(),
            ),
        )
        .await
        .unwrap_or_else(|_| {
            Err(khor_acp::Refusal::Wont(
                "The agent did not finish the ACP handshake within 28 seconds".into(),
            ))
        })
    });
    if let (Some(owner), Some(support)) = (recovery.as_ref(), initialization.recovery()) {
        if let Err(why) = live.record_gui_recovery_support(owner, support) {
            let (why, retain) = retire_failed_opening(&rt, &live, &process_lifecycle, recovery.as_ref(), Some(&initialization), why);
            let _ = std::fs::write(refusal_path(&ready), &why);
            if retain { hold_uncertain_gui_owner(reservation.take(), None, None, recovery.take()); }
            return Err(why);
        }
    }
    let (handle, mut events) = match opened {
        Ok(opened) => opened,
        Err(refusal) => {
            let (why, retain) = retire_failed_opening(&rt, &live, &process_lifecycle, recovery.as_ref(), Some(&initialization), refusal_words(&refusal));
            let _ = std::fs::write(refusal_path(&ready), &why);
            if retain { hold_uncertain_gui_owner(reservation.take(), None, None, recovery.take()); }
            return Err(why);
        }
    };

    // These owners outlive every local setup error and the async loop. In
    // particular, a successful handshake never drops its Journal before the
    // outer same-Transport cleanup has been confirmed or marked Unknown.
    let mut retained_journal: Option<Journal> = None;
    let mut initial: Option<crate::live::GuiInitial> = None;
    let mut new_collision = false;
    let result = (|| -> Result<(), String> {
        // Keep the full provider identity; the row's clean_leaf is only a
        // retained spelling, never authority to replace another conversation.
        let is_new = handle.opened_by() == khor_acp::OpenedBy::New;
        let id = recovery.as_ref().map(|owner| owner.logical_id().clone()).unwrap_or_else(|| crate::adaptor::id_for(&handle.session().0));
        let dir = live.dir_of(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let identity = if is_new {
            // A New response cannot inherit an unrelated old row's agent.
            gui_new_source_identity(khor_core::DeviceId(*key.public().as_bytes()), &cmd, &handle.session().0)
        } else {
            let identity = reservation.as_ref().ok_or("known opening lost its physical Journal")?.identity().clone();
            if identity.native_id != handle.session().0.as_ref() { return Err("known opening returned another native session".into()); }
            identity
        };
        if is_new {
            reservation = Some(JournalReservation::take(&root, identity.clone())?);
            new_collision = true;
        }
        // Check the complete source before binding or appending anything.
        if is_new { live.check_gui_source(&identity)?; }
        let opening=match handle.opened_by() {
            khor_acp::OpenedBy::New=>JournalOpening::New,
            khor_acp::OpenedBy::Load=>JournalOpening::Load,
            khor_acp::OpenedBy::Resume=>JournalOpening::Resume,
        };
        if is_new {
            initial = Some(live.seed_gui_new(
                reservation.as_ref().ok_or("New has no exact journal reservation")?,
                &launch, &process_control, provider_pid,
                birth.take().ok_or("initial process observation was already consumed")?,
                initialization.recovery(),
            )?);
        }
        let admission = reservation.as_mut().ok_or("ACP opening has no physical journal reservation")?;
        if admission.identity() != &identity {
            return Err("ACP opened another identity than its reservation".into());
        }
        retained_journal = Some(admission.open_journal(opening)?);
        new_collision = false;
        let journal = retained_journal.as_mut().expect("journal retained until process cleanup");
        let vendor = std::env::var(VENDOR_ENV).ok();
        // Keep the previous published row during the finite opening prefix.
        // A new execution becomes visible only after its display base commits.
        let mut run = RunPublication::Opening(State::Idle);
        let context = context_path(&root, handle.session().0.as_ref());
        if !retained_environment.mcp_servers.is_empty()
            || !retained_environment.additional_directories.is_empty()
        {
            let parent = context.parent().expect("context parent");
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                    .map_err(|e| e.to_string())?;
            }
            crate::link::write_private(
                &context,
                &serde_json::to_vec(&retained_environment).map_err(|e| e.to_string())?,
            )?;
        }
        let initial_place = if resume.is_none() { crate::cwd::standing() } else { None };
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(msg::cant_listen_loopback)?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let cookie = crate::link::fresh_hex()?;
        crate::link::write_private(
            &host_file_path(&dir),
            &serde_json::to_vec(&HostFile {
                port,
                cookie: cookie.clone(),
                host_pid: std::process::id(),
                // `close` kills `-child_pid` then the host: naming the host
                // twice is deliberate. The agent is not in the host's
                // process group (the library gives it its own), so the kill
                // lands here, and dropping the connection takes the agent's
                // whole group with it — the child dies *because* the host
                // does, which is the ordering close wants anyway.
                child_pid: std::process::id(),
            })
            .map_err(|e| e.to_string())?,
        )?;

        let fanout = Arc::new(Fanout::new());
        let (ops_tx, mut ops_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, GuiOp, bool)>();
        // A face that just arrived, so the loop can hand it what is
        // outstanding. Its own channel rather than a `GuiOp`: joining is
        // not something a face *does*, it is something that happened to
        // this host, and a face must not be able to spell it.
        let (joined_tx, mut joined_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();

        // Attachers: cookie in, frames both ways after the welcome. Each
        // gets a number so a capture can be answered to its asker alone;
        // every other op is the same from anyone.
        {
            let fanout = fanout.clone();
            let cookie = cookie.clone();
            let ops_tx = ops_tx.clone();
            let joined_tx = joined_tx.clone();
            std::thread::spawn(move || {
                let mut next_client: u64 = 0;
                for conn in listener.incoming().flatten() {
                    let mut conn = conn;
                    let Ok(hello) = read_frame::<Hello>(&mut conn) else {
                        continue;
                    };
                    if hello.cookie != cookie {
                        let _ = write_frame(
                            &mut conn,
                            &Welcome {
                                ok: false,
                                why: msg::wrong_cookie().into(),
                            },
                        );
                        continue;
                    }
                    if write_frame(
                        &mut conn,
                        &Welcome {
                            ok: true,
                            why: GUI_GREETING.into(),
                        },
                    )
                    .is_err()
                    {
                        continue;
                    }
                    let Ok(mut readable) = conn.try_clone() else {
                        continue;
                    };
                    let who = next_client;
                    next_client += 1;
                    if fanout.packets.add(who, conn).is_err() {
                        continue;
                    }
                    let _ = joined_tx.send(who);
                    let ops_tx = ops_tx.clone();
                    let fanout = fanout.clone();
                    std::thread::spawn(move || {
                        let mut parts = Reassembly::default();
                        while let Ok(op) = read_frame::<GuiOp>(&mut readable) {
                            let stopped_upload = matches!(op, GuiOp::Stop) && parts.has_upload();
                            let Some(op) = parts.take(op) else {
                                continue;
                            };
                            if ops_tx.send((who, op, stopped_upload)).is_err() {
                                break;
                            }
                        }
                        fanout.packets.remove(who);
                    });
                }
            });
        }

        // The conversation loop: one task, owning every state change, so
        // the words cannot race each other.
        let loop_live = live.clone();
        let loop_id = id.clone();
        let loop_fanout = fanout.clone();
        let loop_ready = ready.clone();
        // block_on drops this future before final exit publication. The outer
        // owner retains the exact key, if the opening actually publishes one.
        let run_in_loop = &mut run;
        let initial_in_loop = initial.as_ref();
        let recovery_in_loop = recovery.as_ref();
        let ending = rt.block_on(async move {
        let (live, id, fanout) = (loop_live, loop_id, loop_fanout);
        // The pending asks, each with the very note it was announced
        // by: a face that arrives later must be told the same sentence
        // the first face got, not a second rendering of it.
        let mut asks: Vec<(u64, khor_acp::Ask, GuiNote)> = Vec::new();
        let mut elicits:Vec<(u64,khor_acp::Elicitation,GuiNote)>=Vec::new();
        let mut next_ask: u64 = 0;
        let (completion_tx, mut completion_rx) = tokio::sync::mpsc::unbounded_channel::<HostProgress>();
        let mut pending_completion: Option<PendingCompletion> = None;
        let mut deferred_ending: Option<Option<String>> = None;
        let mut in_turn: Option<tokio::task::AbortHandle> = None;
        let mut tasks = tokio::task::JoinSet::new();
        let mut active_operation = None;
        let mut bootstrap=Bootstrap::AwaitReady;
        let mut journal_turn:Option<String>=None;
        // Read once here rather than off the handle at each use: this
        // is what the handshake said, and it cannot change while the
        // connection lives.
        let agent_note = GuiNote::Agent {
            name: handle.facts().name.clone(),
            replays: handle.facts().replays,
            images: handle.facts().images,
            embeds: handle.facts().embeds,
            lists: handle.facts().lists,
            needs_login: handle.facts().needs_login,
        };
        // What the session can be set to, as of opening, then moved
        // by every change that took — the agent's own updates and the
        // faces' answered ops alike. Held here so a face that attaches
        // later is told where things stand, the way a pending ask is.
        let mut settings = handle.settings().clone();
        // The agent's own command list, as it last published it — kept
        // for the same reason a pending ask and the settings are: a
        // face that attaches later has to be told. The list rides one
        // `available_commands_update` that claude's shim sends off the
        // init frame, once, on the first turn; a conversation opened
        // again an hour later replays its transcript and the transcript
        // has no such frame, so the "/" menu came up empty on every
        // re-attach (found by the smoke, 2026-09-02, the moment a block
        // left a conversation and came back to it).
        let mut commands: Option<String> = None;
        // The last context reading, for the same reason: it rides the
        // turn that measured it, and a face attaching afterwards would
        // show no dial until the next turn ends.
        let mut usage: Option<String> = None;
        let (set_tx, mut set_rx) =
            tokio::sync::mpsc::unbounded_channel::<(u64, Result<Setting, String>)>();
        let handle = Arc::new(handle);
        let mut controlling=false;
        let setting_gate=Arc::new(tokio::sync::Mutex::new(()));
        let mut delegated: Option<crate::delegation::HostedTask> = None;
        let mut cancellation_sent: Option<std::time::Instant> = None;
        let mut delegation_tick = tokio::time::interval(std::time::Duration::from_millis(200));
        delegation_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut event_burst = 0usize;
        let ending = 'conversation: loop {
            if bootstrap.is_drained() {
                let key = if let Some(initial) = initial_in_loop {
                    let word = match &*run_in_loop {
                        RunPublication::Opening(word) => *word,
                        RunPublication::Live(_) => break Some("GUI opening published twice".into()),
                    };
                    match live.publish_gui_new(
                        initial, journal, &title, vendor.as_deref(), word,
                        initial_place.as_deref(), &loop_ready,
                    ) { Ok(key) => key, Err(why) => break Some(why) }
                } else if let Some(recovery) = recovery_in_loop {
                    let word = match &*run_in_loop {
                        RunPublication::Opening(word) => *word,
                        RunPublication::Live(_) => break Some("GUI recovery published twice".into()),
                    };
                    match live.publish_gui_recovery(recovery, journal, &title, vendor.as_deref(), word, &loop_ready) {
                        Ok(key) => key, Err(why) => break Some(why),
                    }
                } else {
                    break Some("GUI opening has no current publication authority".into());
                };
                if let Err(why) = run_in_loop.publish(&live, key) { break Some(why); }
                bootstrap=Bootstrap::Live;
            }
            if pending_completion.as_ref().is_some_and(|pending| pending.remaining == Some(0)) {
                let pending = pending_completion.take().unwrap();
                match pending.completion {
HostCompletion::Turn { result, .. } => {
                    if let Some(turn)=journal_turn.take() {
                        let (outcome,detail)=match &result {
                            Ok(stop) if stop=="Cancelled"=>(JournalOutcome::Cancelled,Some(stop.as_str())),
                            Ok(stop)=>(JournalOutcome::Ended,Some(stop.as_str())),
                            Err(why)=>(JournalOutcome::Unknown,Some(why.as_str())),
                        };
                        if let Err(why)=journal.finish(&turn,outcome,detail) { break Some(why); }
                    }
let turn = Some(result);
                    in_turn = None;
                    cancellation_sent = None;
                    if let Some(mut task) = delegated.take() {
                        let stop = match &turn { Some(Ok(stop)) => stop.as_str(), Some(Err(why)) => why.as_str(), None => "ACP turn disconnected" };
                        if let Err(error) = task.finish(stop) { break Some(error); }
                    }
                    asks.clear();
                    cancel_elicits(&mut elicits,&fanout);
                    match turn {
                        Some(Ok(stop)) => {
                            let word = match stop.as_str() {
                                "EndTurn" => State::Done,
                                "Cancelled" => State::Idle,
                                _ => State::Errored,
                            };
                            let _ = run_in_loop.report(&live, word, Source::Reported);
                            fanout.send(&GuiNote::Turn(stop));
                        }
                        Some(Err(e)) => {
                            let _ = run_in_loop.report(&live, State::Errored, Source::Reported);
                            fanout.send(&GuiNote::Turn(e));
                        }
                        None => {}
                    }
                }
HostCompletion::Control { who, token, operation, result, .. } => {
                        controlling=false;
                        let end=result.is_ok()&&matches!(operation.as_str(),"close"|"delete"|"logout");
                        let(data,error)=match result{Ok(value)=>(Some(value.to_string()),None),Err(e)=>(None,Some(e))};
                        let deleting=operation=="delete";
                        fanout.send_to(who,&GuiNote::ControlResult{token,operation,data,error});
                        if end{if deleting{let _=std::fs::remove_file(&context);}break None;}
                    }

                }
                if let Some(ending) = deferred_ending.take() { break ending; }
                continue;
            }
            while let Some(joined) = tasks.try_join_next() {
                if let Err(why) = joined { break 'conversation Some(format!("ACP request task failed: {why}")); }
            }
            tokio::select! {
                // A bounded event burst keeps Stop responsive while preserving
                // updates before a turn's end. Its last update and its end
                // become ready together, and a face must read the words
                // before the goodbye — an unbiased pick would reorder
                // them on some runs and every screen would inherit the
                // dice.
                biased;
                progress = completion_rx.recv() => {
                    match progress {
                        Some(HostProgress::BootstrapFenced { result })=> {
                            let remaining=match result.and_then(|prefix| events.remaining_through(&prefix)) {
                                Ok(remaining)=>remaining,
                                Err(why)=>break Some(format!("ACP bootstrap event fence failed: {why}")),
                            };
                            if let Err(why)=bootstrap.fenced(remaining) { break Some(why); }
                        },
                        Some(HostProgress::Resolved { completion, acknowledged }) => {
                            if active_operation.as_ref() != Some(completion.operation()) || pending_completion.is_some() {
                                break Some("ACP completion belongs to an inactive operation".into());
                            }
                            if let HostCompletion::Control { abandoned: true, who, token, operation, result, .. } = &completion {
                                fanout.send_to(*who, &GuiNote::ControlResult { token: token.clone(), operation: operation.clone(), data: None, error: result.as_ref().err().cloned() });
                                break Some("ACP operation timed out; its completion is unknown and the process is retiring".into());
                            }
                            if !acknowledged { break Some("ACP cancellation grace expired; retiring the process".into()); }
                            active_operation = None;
                            cancellation_sent = None;
                            pending_completion = Some(PendingCompletion { completion, remaining: None });
                        }
                        Some(HostProgress::Fenced { operation, result }) => {
                            let Some(pending) = pending_completion.as_mut().filter(|pending| pending.completion.operation() == &operation) else {
                                break Some("ACP event fence belongs to an inactive operation".into());
                            };
                            pending.remaining = Some(match result.and_then(|prefix| events.remaining_through(&prefix)) {
                                Ok(remaining)=>remaining,
                                Err(why)=>break Some(format!("ACP response was received but its event fence failed: {why}")),
                            });
                        }
                        None => break Some("ACP completion channel closed".into()),
                    }
                }
                _ = delegation_tick.tick(), if delegated.is_some() => {
                    if active_operation.is_some() && cancellation_sent.is_some_and(|at| at.elapsed() >= timeouts.cancel) {
                        break Some("ACP did not acknowledge cancellation; closing its host".into());
                    }
                    if active_operation.is_some() && cancellation_sent.is_none() && delegated.as_ref().is_some_and(|task| task.cancel_requested()) {
                        cancellation_sent = Some(std::time::Instant::now());
                        if let Some(operation) = &active_operation { process_control.cancel_operation(operation, timeouts.cancel); }
                        let _ = handle.cancel();
                        asks.clear(); // Dropping permission senders cancels pending asks.
                    }
                }
                incoming = next_host_input(&mut events, &mut ops_rx, event_burst >= 32, pause_closed_events(deferred_ending.is_some(), pending_completion.as_ref().and_then(|pending| pending.remaining))) => match incoming {
                    HostInput::Event(event) => { event_burst = event_burst.saturating_add(1);
                        if let Some(remaining) = pending_completion.as_mut().and_then(|pending| pending.remaining.as_mut()) { *remaining = remaining.saturating_sub(1); }
                        bootstrap.consume_event();
match event {
                    Some(khor_acp::Event::Note(n)) => {
                        if n.session_id!=*handle.session() { break Some("ACP update belongs to another session".into()); }
                        // Load is a full new base; Resume is not a replay.
                        // Both still own every notification actually received.
                        if let Err(why)=journal.note(&n) { break Some(why); }
                        let json = serde_json::to_string(&n).unwrap_or_default();
                        if matches!(n.update, SessionUpdate::AvailableCommandsUpdate(_)) {
                            commands = Some(json.clone());
                        }
                        if matches!(n.update, SessionUpdate::UsageUpdate(_)) {
                            usage = Some(json.clone());
                        }
                        if let Some(task) = &mut delegated {
                            if let Err(error) = task.note(&json) { break Some(error); }
                        }
                        fanout.send(&GuiNote::Note(json));
                        // The agent moving its own settings: the note
                        // went through whole above, and this is the
                        // copy the next face to attach will be told.
                        let moved = match &n.update {
                            SessionUpdate::CurrentModeUpdate(u) => match settings.modes.as_mut() {
                                Some(m) => {
                                    m.current_mode_id = u.current_mode_id.clone();
                                    true
                                }
                                None => false,
                            },
                            SessionUpdate::ConfigOptionUpdate(u) => {
                                settings.options = u.config_options.clone();
                                true
                            }
                            // The model, learned after opening (claude's
                            // shim says it off the first turn's init).
                            SessionUpdate::SessionInfoUpdate(u) => match khor_acp::model_in(u.meta.as_ref()) {
                                Some(m) if settings.model.as_deref() != Some(m.as_str()) => {
                                    settings.model = Some(m);
                                    true
                                }
                                _ => false,
                            },
                            _ => false,
                        };
                        if moved {
                            fanout.send(&GuiNote::Settings(settings_json(&settings)));
                        }
                    }
                    Some(khor_acp::Event::Ask(ask)) => {
                        if ask.request.session_id!=*handle.session(){drop(ask);continue;}
                        let n = next_ask;
                        next_ask += 1;
                        let title = ask.request.tool_call.fields.title.clone().unwrap_or_default();
                        let options: Vec<(String, String)> = ask
                            .request
                            .options
                            .iter()
                            .map(|o| (o.option_id.0.to_string(), o.name.clone()))
                            .collect();
                        if let Some(task) = &mut delegated {
                            if let Err(error) = task.ask(n, &title, &options) { break Some(error); }
                        }
                        let note = GuiNote::Ask { ask: n, title, options };
                        fanout.send(&note);
                        asks.push((n, ask, note));
                        let _ = run_in_loop.report(&live, State::Blocked, Source::Reported);
                    }
                    Some(khor_acp::Event::Elicit(request))=>{
                        let encoded=serde_json::to_value(&request.request).unwrap_or_default();
                        if encoded["sessionId"].as_str().is_some_and(|s|s!=handle.session().0.as_ref()) {drop(request);continue;}
                        let ask=next_ask;next_ask+=1;
                        let note=GuiNote::Elicit{ask,request:encoded.to_string()};
                        fanout.send(&note);elicits.push((ask,request,note));
                        let _=run_in_loop.report(&live,State::Blocked,Source::Reported);
                    }
                    Some(khor_acp::Event::ElicitDone(elicitation_id))=>{
                        if let Some(at)=elicits.iter().position(|(_,q,_)|serde_json::to_value(&q.request).is_ok_and(|v|v["mode"]=="url"&&v["elicitationId"]==elicitation_id)){
                            let(ask,q,_)=elicits.remove(at);let value=serde_json::json!({"action":"accept"});if let Ok(response)=q.validate(&value){let _=q.answer(response);}
                            fanout.send(&GuiNote::ElicitAnswered{ask,response:value.to_string()});
                            let _=run_in_loop.report(&live,if asks.is_empty()&&elicits.is_empty(){if in_turn.is_some(){State::Busy}else{State::Idle}}else{State::Blocked},Source::Reported);
                        }
                    }
                    Some(khor_acp::Event::Ready { session }) => {
                        if session!=*handle.session() { break Some("ACP opening belongs to another session".into()); }
                        if let Err(why)=bootstrap.begin_fence() { break Some(why); }
                        let handle=handle.clone(); let tx=completion_tx.clone();
                        tasks.spawn(async move { let _=tx.send(HostProgress::BootstrapFenced { result:handle.fence_events().await }); });
                    }
                    // A channel that just stops is an ending too, and
                    // painting it any other way would need a word khor
                    // does not have.
                    Some(khor_acp::Event::Closed(err)) => {
                        if pending_completion.is_some() { deferred_ending = Some(err); } else { break err; }
                    }
                    None => {
                        if pending_completion.is_some() { deferred_ending = Some(None); } else { break None; }
                    }
                }
                    },
                    HostInput::Op(op) => { event_burst = 0;
{
                    let (op, stopped_upload) = match op {
                        Some((who, op, stopped)) => (Some((who, op)), stopped),
                        None => (None, false),
                    };
                    if bootstrap.is_running() {
                        match &op {
                            Some((who,GuiOp::Capture{token}))=> { fanout.send_to(*who, &captured_note(*token, Err(CaptureFailure::NotReady))); continue; },
                            Some((_,GuiOp::Close|GuiOp::Stop))|None=>{},
                            Some((who,_))=> { fanout.send_to(*who,&GuiNote::Refused { why:refused::TURN_IN_FLIGHT.to_owned() }); continue; },
                        }
                    }
                    match op {
                    Some((who, GuiOp::Capture { token })) => {
                        let result = journal.capture(id.clone())
                            .map(|boundary| CaptureFence { binding: boundary.binding, cursor: boundary.cursor })
                            .map_err(|_| CaptureFailure::Unavailable);
                        fanout.send_to(who, &captured_note(token, result));
                    }
                    Some((who,GuiOp::ElicitAnswer{ask,response}))=>{
                        let accepted=elicits.iter().position(|(id,_,_)|*id==ask).and_then(|at|{
                            let value:serde_json::Value=serde_json::from_str(&response).ok()?;
                            elicits[at].1.validate(&value).ok().map(|answer|(at,answer))
                        });
                        if let Some((at,answer))=accepted{
                            let(_,request,_)=elicits.remove(at);
                            let action=serde_json::to_value(&answer).unwrap_or_default()["action"].clone();
                            let accepted=request.answer(answer).is_ok();
                            fanout.send(&GuiNote::ElicitAnswered{ask,response:serde_json::json!({"action":if accepted{action}else{serde_json::json!("cancel")}}).to_string()});
                            let _=run_in_loop.report(&live,if asks.is_empty()&&elicits.is_empty(){if in_turn.is_some(){State::Busy}else{State::Idle}}else{State::Blocked},Source::Reported);
                        }else{fanout.send_to(who,&GuiNote::Refused{why:refused::SETTING_REFUSED.into()});}
                    }
                    Some((who,GuiOp::Control{token,operation,payload}))=>{
                        if in_turn.is_some()||controlling{fanout.send_to(who,&GuiNote::ControlResult{token,operation,data:None,error:Some(refused::TURN_IN_FLIGHT.into())});continue;}
                        let parsed=serde_json::from_str(&payload);
                        if token.is_empty()||token.len()>128||payload.len()>16*1024||operation.len()>64||parsed.is_err(){fanout.send_to(who,&GuiNote::ControlResult{token,operation,data:None,error:Some("Invalid session operation".into())});continue;}
                        active_operation = process_control.begin_operation();
                        if active_operation.is_none() { break Some("ACP process is retiring".into()); }
                        controlling=true;
                        let h=handle.clone();let tx=completion_tx.clone();
                        let generation = active_operation.clone().unwrap(); let control = process_control.clone();
                        tasks.spawn(async move {
                            let (result, abandoned) = match tokio::time::timeout(timeouts.control, h.control(&operation, &parsed.unwrap())).await {
                                Ok(result) => (result, false),
                                Err(_) => { h.request_shutdown(); (Err("The agent did not answer this operation; completion is unknown and its transport is closing".into()), true) }
                            };
                            finish_host_operation(&h, &control, &tx, HostCompletion::Control { generation, who, token, operation, result, abandoned }).await;
                        });
                    }
                    Some((who, GuiOp::Delegate { task, message })) => {
                        if in_turn.is_some() || controlling {
                            fanout.send_to(who, &GuiNote::Delegated { task, why: Some("ACP session already has a running turn".into()) });
                            continue;
                        }
                        match crate::delegation::HostedTask::claim(&root, &id, &task, &message) {
                            Ok((mut execution, text)) => {
                                active_operation = process_control.begin_operation();
                                if active_operation.is_none() { break Some("ACP process is retiring".into()); }
                                let turn=match crate::link::fresh_hex() { Ok(turn)=>turn, Err(why)=>break Some(why) };
                                if let Err(why)=journal.accept(&turn,&text,&[],&[],None) { let _=execution.finish(&why); break Some(why); }
                                journal_turn=Some(turn);
                                delegated = Some(execution);
                                cancellation_sent = None;
                                let _ = run_in_loop.report(&live, State::Busy, Source::Reported);
                                let handle = handle.clone();
                                let tx = completion_tx.clone();
                                let generation = active_operation.clone().unwrap(); let control = process_control.clone();
                                in_turn = Some(tasks.spawn(async move {
                                    let out = handle.prompt(&text).await.map(|stop| format!("{stop:?}"));
                                    finish_host_operation(&handle, &control, &tx, HostCompletion::Turn { generation, result: out }).await;
                                }));
                                fanout.send_to(who, &GuiNote::Delegated { task, why: None });
                                fanout.send(&GuiNote::Turning);
                            }
                            Err(why) => fanout.send_to(who, &GuiNote::Delegated { task, why: Some(why) }),
                        }
                    }
                    // **One arm for both.** `SayWith` is `Say` with a
                    // list that is usually empty, and splitting the
                    // handler would be two copies of the turn-in-flight
                    // refusal, the 忙碌 report and the spawn — three
                    // things that have each been got wrong once already
                    // and must not exist twice.
                    Some((who, op @ (GuiOp::Say(_) | GuiOp::SayWith { .. }))) => {
                        let (text, images, files, token) = match op {
                            GuiOp::Say(t) => (t, Vec::new(), Vec::new(), None),
                            GuiOp::SayWith { text, images, files, token } => (text, images, files, token),
                            _ => unreachable!("the pattern above admits nothing else"),
                        };
                        if in_turn.is_some() || controlling {
                            // One turn at a time; a queue would answer
                            // "who said what when" wrongly on every face.
                            // **Said back, though.** Dropping it in
                            // silence let a second face paint a line
                            // that never went anywhere — and then its
                            // stop found no turn and was dropped too.
                            fanout.send_to(
                                who,
                                &GuiNote::Refused { why: refused::TURN_IN_FLIGHT.to_owned() },
                            );
                            continue;
                        }
                        active_operation = process_control.begin_operation();
                        if active_operation.is_none() { break Some("ACP process is retiring".into()); }
                        let turn=match crate::link::fresh_hex() { Ok(turn)=>turn, Err(why)=>break Some(why) };
                        if let Err(why)=journal.accept(&turn,&text,&images,&files,token.as_deref()) { break Some(why); }
                        journal_turn=Some(turn);
                        let _ = run_in_loop.report(&live, State::Busy, Source::Reported);
                        let handle = handle.clone();
                        let tx = completion_tx.clone();
                        let generation = active_operation.clone().unwrap(); let control = process_control.clone();
                        in_turn = Some(tasks.spawn(async move {
                            let out = handle
                                .prompt_with(&text, &images, &files)
                                .await
                                .map(|stop| format!("{stop:?}"));
                            finish_host_operation(&handle, &control, &tx, HostCompletion::Turn { generation, result: out }).await;
                        }));
                    }
                    Some((_, GuiOp::Answer { ask, option })) => {
                        let Some(seat) = asks.iter().position(|(n, _, _)| *n == ask) else {
                            continue;
                        };
                        let (_, pending, _) = asks.swap_remove(seat);
                        match &option {
                            Some(oid) => pending.choose(oid),
                            None => pending.dismiss(),
                        }
                        // Every face learns the answer, including the
                        // ones that arrive afterwards: the record of a
                        // settled permission outlives the answering.
                        if let Some(task) = &mut delegated {
                            let result = match asks.last() {
                                Some((n, _, GuiNote::Ask { title, options, .. })) => task.ask(*n, title, options),
                                _ => task.answered(),
                            };
                            if let Err(error) = result { break Some(error); }
                        }
                        fanout.send(&GuiNote::Answered { ask, option });
                        if asks.is_empty() {
                            // 待批答才清 — and it clears to 忙碌: the
                            // turn the ask interrupted is still running.
                            let _ = run_in_loop.report(&live, State::Busy, Source::Reported);
                        }
                    }
                    Some((who, GuiOp::Stop)) => {
                        cancel_elicits(&mut elicits,&fanout);
                        // The cancel goes to the agent, not to the task:
                        // aborting the join handle here would leave the
                        // agent still working with nobody reading it.
                        // The turn ends the way every turn ends, through
                        // its own `Turn` note.
                        if in_turn.is_some() || controlling {
                            if let Some(operation) = &active_operation {
                                process_control.cancel_operation(operation, timeouts.cancel);
                                let _ = handle.cancel();
                            }
                            for(ask,_,_)in asks.drain(..){fanout.send(&GuiNote::Answered{ask,option:None});}
                        } else if stopped_upload {
                            let _ = run_in_loop.report(&live, State::Idle, Source::Reported);
                            fanout.send_to(who, &GuiNote::Turn("Cancelled".into()));
                        } else {
                            // Nothing to stop. A face that got here is
                            // holding a stop button over a turn that is
                            // already over, and silence would leave it
                            // holding it.
                            fanout.send_to(
                                who,
                                &GuiNote::Refused { why: refused::NO_TURN.to_owned() },
                            );
                        }
                    }
                    // Both asked of the agent in a task of their own,
                    // like a turn: the loop owns every state change
                    // and must keep moving updates while the agent
                    // thinks — a `set_mode` on the claude shim is a
                    // control request answered by a process that may
                    // be busy.
                    Some((who, GuiOp::SetMode(mode))) => {
                        let handle = handle.clone();
                        let set_tx = set_tx.clone();let gate=setting_gate.clone();
                        tasks.spawn(async move {let _guard=gate.lock().await;
                            let out = handle.set_mode(&mode).await.map(|()| Setting::Mode(mode));
                            let _ = set_tx.send((who, out));
                        });
                    }
                    Some((who, GuiOp::SetOption { id: option, value })) => {
                        let handle = handle.clone();
                        let set_tx = set_tx.clone();let gate=setting_gate.clone();
                        tasks.spawn(async move {let _guard=gate.lock().await;
                            let wire = match &value {
                                Choice::Value(v) => SessionConfigOptionValue::value_id(v.clone()),
                                Choice::Switch(on) => SessionConfigOptionValue::boolean(*on),
                            };
                            let out = handle
                                .set_option(&option, wire)
                                .await
                                .map(|list| Setting::Options { list });
                            let _ = set_tx.send((who, out));
                        });
                    }
                    Some((_, GuiOp::Close)) => {asks.clear();elicits.clear();let _=handle.cancel();let _=tokio::time::timeout(std::time::Duration::from_secs(2),handle.control("close",&serde_json::json!({}))).await;break None;},
                    // Put back together by the reader (`Reassembly`).
                    Some((_, GuiOp::Part(_) | GuiOp::Parts)) => {}
                    None => break None,
                    }
                }
                    },
                },
                // A face that arrived mid-ask gets the ask. Nothing
                // else is re-sent: the canonical past belongs to each view
                // through its own Capture, and the row already carries the word.
                who = joined_rx.recv() => {
                    if let Some(who) = who {
                        // First, because everything else a face does
                        // with this conversation depends on it — most
                        // of all whether asking for the past means
                        // anything.
                        fanout.send_to(who, &agent_note);
                        fanout.send_to(who,&GuiNote::Capabilities(handle.facts().protocol.to_string()));
                        if has_settings(&settings) {
                            fanout.send_to(who, &GuiNote::Settings(settings_json(&settings)));
                        }
                        if let Some(json) = &commands {
                            fanout.send_to(who, &GuiNote::Note(json.clone()));
                        }
                        if let Some(json) = &usage {
                            fanout.send_to(who, &GuiNote::Note(json.clone()));
                        }
                        if in_turn.is_some() {
                            fanout.send_to(who, &GuiNote::Turning);
                        }
                        for (_, _, note) in &asks {
                            fanout.send_to(who, note);
                        }
                        for(_,_,note) in &elicits{fanout.send_to(who,note);}
                    }
                }
                set = set_rx.recv() => {
                    let Some((who, out)) = set else { continue };
                    match out {
                        Ok(Setting::Mode(mode)) => {
                            // The agent took it. Its own
                            // `current_mode_update`, if it sends one,
                            // lands on the same field and changes
                            // nothing.
                            if let Some(m) = settings.modes.as_mut() {
                                m.current_mode_id = SessionModeId::new(mode);
                            }
                            fanout.send(&GuiNote::Settings(settings_json(&settings)));
                        }
                        Ok(Setting::Options { list }) => {
                            settings.options = list;
                            fanout.send(&GuiNote::Settings(settings_json(&settings)));
                        }
                        // The agent said no; the screen keeps what it
                        // had, and the asker is told rather than left
                        // looking at a control that flipped for nobody.
                        Err(_) => fanout.send_to(
                            who,
                            &GuiNote::Refused { why: refused::SETTING_REFUSED.to_owned() },
                        ),
                    }
                }

            }
        };

        asks.clear(); elicits.clear();
        let mut ending=if let Some(turn)=journal_turn.take() {
            match journal.finish(&turn,JournalOutcome::Unknown,ending.as_deref()) {
                Ok(())=>ending,
                Err(why)=>Some(match ending { Some(previous)=>format!("{previous}; {why}"),None=>why }),
            }
        } else { ending };
        if let Err(why)=journal.abort_base() {
            ending=Some(match ending { Some(previous)=>format!("{previous}; {why}"),None=>why });
        }
        handle.request_shutdown();

        tasks.abort_all();
        while tasks.join_next().await.is_some() {}

        match handle.shutdown().await {
            Ok(khor_acp::ShutdownOutcome::ProcessReaped(_)) => ending,
            Ok(khor_acp::ShutdownOutcome::DriverClosed) => Some("ACP driver closed without a process receipt".into()),
            Err(why) => Some(format!("ACP process cleanup is unconfirmed: {why}")),
        }
    });

        process_lifecycle.request_shutdown();
        rt.block_on(process_lifecycle.confirmed_exit())
            .map_err(|why| format!("ACP process cleanup is unconfirmed: {why}"))?;

        if !ready.exists() {
            std::fs::write(refusal_path(&ready),ending.as_deref().unwrap_or("ACP closed before the initial capture committed"))
                .map_err(|why|why.to_string())?;
        }

        fanout.send(&GuiNote::Gone);

        fanout.packets.close();

        // 0 for an ending nobody minded, 1 for a torn connection: the exit
        // code is how `record_exit` tells 完成-shaped endings from 失败.
        let _ = run.record_exit(&live, if ending.is_none() { 0 } else { 1 });
        Ok(())
    })();
    process_lifecycle.request_shutdown();
    let retired = rt.block_on(process_lifecycle.confirmed_exit());
    let settled = if let Some(initial) = &initial {
        live.settle_gui_new(initial, retired.as_ref().map_err(|_| ()), if result.is_ok() { 0 } else { 1 })
    } else if let Some(recovery) = &recovery {
        live.settle_gui_recovery(recovery, retired.as_ref().map_err(|_| ()), if result.is_ok() { 0 } else { 1 })
    } else if retired.is_err() && new_collision {
        // Only the exact actual-New agent/native reservation can quarantine
        // its old bound row. A same short SID or missing reservation is never
        // authority to freeze someone else's source.
        match &reservation {
            Some(reservation) => live.quarantine_gui_new(reservation),
            None => Err("unconfirmed New has no exact reservation".into()),
        }
    } else {
        Ok(())
    };
    if retired.is_err() && settled.is_err()
        && (retained_journal.is_some() || reservation.as_ref().is_some_and(|value| value.holds_file()))
    {
        let why = format!("ACP cleanup and its durable Unknown record are unconfirmed: {}; {}",
            retired.as_ref().err().expect("cleanup error"),
            settled.as_ref().err().expect("publication error"));
        // Refuse the opener within its existing budget; the ghost itself keeps
        // its exact resource rather than returning and silently unlocking it.
        let _ = std::fs::write(refusal_path(&ready), &why);
        eprintln!("{why}");
        hold_uncertain_gui_owner(reservation.take(), retained_journal.take(), initial.take(), recovery.take());
    }
    // Release only after actual cleanup or a durable Unknown. A failed write
    // plus unconfirmed cleanup parks the existing ghost above, without retry.
    let result = match (result, settled) {
        (outcome, Ok(())) => outcome,
        (outcome, Err(why)) => Err(format!("{}GUI cleanup publication is uncertain: {why}",
            outcome.err().map(|why| format!("{why}; ")).unwrap_or_default())),
    };
    match (result, retired) {
        (Ok(()), Ok(_)) => Ok(()),
        (Err(why), Ok(_)) => Err(why),
        (outcome, Err(why)) => Err(format!(
            "{}ACP process cleanup is unconfirmed: {why} ({} stderr bytes captured)",
            outcome
                .err()
                .map(|why| format!("{why}; "))
                .unwrap_or_default(),
            stderr.lock().unwrap_or_else(|p| p.into_inner()).len()
        )),
    }
}

fn retire_failed_opening(
    rt: &tokio::runtime::Runtime,
    live: &LiveKind,
    process: &khor_acp::TransportLifecycle,
    recovery: Option<&crate::live::GuiRecovery>,
    initialization: Option<&khor_acp::InitializeFacts>,
    mut why: String,
) -> (String, bool) {
    process.request_shutdown();
    let retired = rt.block_on(process.confirmed_exit());
    let facts = match (recovery, initialization.and_then(khor_acp::InitializeFacts::recovery)) {
        (Some(owner), Some(support)) => live.record_gui_recovery_support(owner, support),
        _ => Ok(()),
    };
    let settled = recovery.map(|owner| live.settle_gui_recovery(owner, retired.as_ref().map_err(|_| ()), 1)).transpose();
    if let Err(error) = &retired { why = format!("{why}; ACP cleanup is unconfirmed: {error}"); }
    if let Err(error) = &settled { why = format!("{why}; durable cleanup is unconfirmed: {error}"); }
    if let Err(error) = &facts { why = format!("{why}; Initialize fact publication is unconfirmed: {error}"); }
    (why, retired.is_err() && (settled.is_err() || facts.is_err()))
}

/// No listener or GuiOp::Close is promised on this pre-publication failure.
/// Retaining this existing ghost is fail-closed only for its current lifetime:
/// explicit OS termination/crash releases the File, and a failed durable write
/// cannot provide cross-crash safety. No retry, timer, new control plane or
/// second resource registry is created here.
fn hold_uncertain_gui_owner(
    _reservation: Option<JournalReservation>,
    _journal: Option<Journal>,
    _initial: Option<crate::live::GuiInitial>,
    _recovery: Option<crate::live::GuiRecovery>,
) -> ! {
    loop {
        std::thread::park();
    }
}

/// Before publication this owns only the latest opening word. Afterwards the
/// GUI row's atomic Meta is the sole state owner; no second live snapshot is kept.
enum RunPublication {
    Opening(State),
    Live(crate::live::RunKey),
}
impl RunPublication {
    fn report(&mut self, live: &LiveKind, word: State, source: Source) -> Result<(), String> {
        match self {
            Self::Opening(current) => {
                *current = word;
                Ok(())
            }
            Self::Live(key) => live.report_run(key, word, source),
        }
    }
    fn publish(&mut self, live: &LiveKind, key: crate::live::RunKey) -> Result<(), String> {
        let Self::Opening(word) = self else {
            return Err("GUI execution has already published".into());
        };
        let word = *word;
        *self = Self::Live(key);
        self.report(live, word, Source::Reported)
    }
    fn record_exit(&self, live: &LiveKind, code: i32) -> Result<(), String> {
        match self {
            Self::Opening(_) => Ok(()),
            Self::Live(key) => live.record_exit_run(key, code),
        }
    }
}

enum Bootstrap {
    AwaitReady,
    Fencing,
    Draining(u64),
    Live,
}
impl Bootstrap {
    fn is_running(&self) -> bool { !matches!(self,Self::Live) }
    fn is_drained(&self) -> bool { matches!(self,Self::Draining(0)) }
    fn begin_fence(&mut self) -> Result<(),String> {
        if !matches!(self,Self::AwaitReady) { return Err("ACP Ready belongs to an inactive opening".into()); }
        *self=Self::Fencing; Ok(())
    }
    fn fenced(&mut self, remaining:u64) -> Result<(),String> {
        if !matches!(self,Self::Fencing) { return Err("ACP event fence belongs to an inactive opening".into()); }
        *self=Self::Draining(remaining); Ok(())
    }
    fn consume_event(&mut self) {
        if let Self::Draining(remaining)=self { *remaining=remaining.saturating_sub(1); }
    }
}

enum HostCompletion {
    Turn {
        generation: crate::agent_process::Operation,
        result: Result<String, String>,
    },
    Control {
        generation: crate::agent_process::Operation,
        who: u64,
        token: String,
        operation: String,
        result: Result<serde_json::Value, String>,
        abandoned: bool,
    },
}
impl HostCompletion {
    fn operation(&self) -> &crate::agent_process::Operation {
        match self {
            Self::Turn { generation, .. }
            | Self::Control { generation, .. } => generation,
        }
    }
}
enum HostProgress {
    BootstrapFenced { result:Result<khor_acp::EventPrefix,String> },
    Resolved {
        completion: HostCompletion,
        acknowledged: bool,
    },
    Fenced {
        operation: crate::agent_process::Operation,
        result: Result<khor_acp::EventPrefix, String>,
    },
}
struct PendingCompletion {
    completion: HostCompletion,
    remaining: Option<u64>,
}
async fn finish_host_operation(
    handle: &khor_acp::Handle,
    control: &crate::agent_process::Control,
    tx: &tokio::sync::mpsc::UnboundedSender<HostProgress>,
    completion: HostCompletion,
) {
    let operation = completion.operation().clone();
    let abandoned = matches!(
        completion,
        HostCompletion::Control {
            abandoned: true,
            ..
        }
    );
    // A real response acknowledges only its own opaque ticket, before any
    // forwarding work or UI backlog. Timeout is unknown and never an ACK.
    let acknowledged = !abandoned && control.complete_operation(&operation);
    if tx
        .send(HostProgress::Resolved {
            completion,
            acknowledged,
        })
        .is_err()
        || abandoned
        || !acknowledged
    {
        return;
    }
    let result = handle.fence_events().await;
    let _ = tx.send(HostProgress::Fenced { operation, result });
}

enum HostInput {
    Event(Option<khor_acp::Event>),
    Op(Option<(u64, GuiOp, bool)>),
}

fn pause_closed_events(closed: bool, remaining: Option<u64>) -> bool {
    // Closed can itself lie inside an already-captured finite batch. Drain
    // that accepted tail before pausing the receiver and publishing its end.
    closed && !remaining.is_some_and(|remaining| remaining > 0)
}

fn poll_host_input(
    cx: &mut std::task::Context<'_>,
    mut poll_events: impl FnMut(&mut std::task::Context<'_>) -> std::task::Poll<Option<khor_acp::Event>>,
    ops: &mut tokio::sync::mpsc::UnboundedReceiver<(u64, GuiOp, bool)>,
    prefer_ops: bool,
    events_closed: bool,
) -> std::task::Poll<HostInput> {
    if prefer_ops {
        if let std::task::Poll::Ready(op) = ops.poll_recv(cx) {
            return std::task::Poll::Ready(HostInput::Op(op));
        }
    }
    if !events_closed {
        if let std::task::Poll::Ready(event) = poll_events(cx) {
            return std::task::Poll::Ready(HostInput::Event(event));
        }
    }
    ops.poll_recv(cx).map(HostInput::Op)
}
async fn next_host_input(
    events: &mut khor_acp::EventReceiver,
    ops: &mut tokio::sync::mpsc::UnboundedReceiver<(u64, GuiOp, bool)>,
    prefer_ops: bool,
    events_closed: bool,
) -> HostInput {
    std::future::poll_fn(|cx| {
        poll_host_input(cx, |cx| events.poll_recv(cx), ops, prefer_ops, events_closed)
    }).await
}

fn gui_new_source_identity(
    home: khor_core::DeviceId,
    cmd: &[String],
    native_id: &str,
) -> JournalIdentity {
    let vendor = std::env::var(VENDOR_ENV).ok();
    JournalIdentity { home, agent: gui_new_agent(cmd, vendor.as_deref()), native_id: native_id.to_owned() }
}

fn gui_new_agent(cmd: &[String], vendor: Option<&str>) -> String {
    vendor.map(str::to_owned).unwrap_or_else(|| {
        let launch = serde_json::to_vec(cmd).expect("launcher identity serializes");
        format!("launcher:{}", blake3::hash(&launch).to_hex())
    })
}

#[cfg(test)]
fn gui_source_identity(
    live: &LiveKind,
    home: khor_core::DeviceId,
    cmd: &[String],
    native_id: &str,
) -> Result<JournalIdentity, String> {
    let vendor = std::env::var(VENDOR_ENV).ok();
    gui_source_identity_as(live, home, cmd, native_id, vendor.as_deref())
}

fn gui_source_identity_as(
    live: &LiveKind,
    home: khor_core::DeviceId,
    cmd: &[String],
    native_id: &str,
    vendor: Option<&str>,
) -> Result<JournalIdentity, String> {
    let id = crate::adaptor::id_for(native_id);
    let dir = live
        .dir_of(&id)
        .ok_or_else(|| msg::not_a_session_id(&id.0))?;
    let bound = crate::session_content::journal::bound_identity(&dir)?;
    let meta = match std::fs::read(dir.join("meta.json")) {
        Ok(bytes) => {
            Some(serde_json::from_slice::<crate::live::Meta>(&bytes).map_err(|e| e.to_string())?)
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => return Err(why.to_string()),
    };
    let recorded = bound.or_else(|| meta.as_ref().and_then(|meta| meta.gui_native.clone()));
    if let Some(recorded) = recorded {
        if recorded.home != home
            || recorded.native_id != native_id
            || vendor.is_some_and(|agent| agent != recorded.agent)
        {
            return Err("GUI row already names another full native identity".into());
        }
        return Ok(recorded);
    }
    let agent = vendor.map(str::to_owned)
        .or_else(|| meta.and_then(|meta| meta.category))
        .unwrap_or_else(|| {
            let launch = serde_json::to_vec(cmd).expect("launcher identity serializes");
            format!("launcher:{}", blake3::hash(&launch).to_hex())
        });
    Ok(JournalIdentity {
        home,
        agent,
        native_id: native_id.into(),
    })
}

pub(crate) fn saved_environment(root: &std::path::Path, native_id: &str) -> Result<khor_acp::Environment, String> {
    match std::fs::read_to_string(context_path(root, native_id)) {
        Ok(context) => serde_json::from_str(&context).map_err(|error| format!("Invalid session context: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(khor_acp::Environment::default()),
        Err(error) => Err(error.to_string()),
    }
}
fn context_path(root: &std::path::Path, session: &str) -> PathBuf {
    root.join(".khor/acp-context").join(format!(
        "{}.json",
        blake3::hash(session.as_bytes()).to_hex()
    ))
}

fn cancel_elicits(elicits: &mut Vec<(u64, khor_acp::Elicitation, GuiNote)>, fanout: &Fanout) {
    for (ask, _, _) in elicits.drain(..) {
        fanout.send(&GuiNote::ElicitAnswered {
            ask,
            response: serde_json::json!({"action":"cancel"}).to_string(),
        });
    }
}

#[cfg(test)]
mod parts_tests {
    use super::*;

    #[test]
    fn an_uncancelled_sliced_turn_retains_the_complete_payload() {
        let picture = "x".repeat(3 * PART + 17);
        let op = GuiOp::SayWith {
            text: "keep this".into(),
            images: vec![("image/png".into(), picture.clone())],
            files: vec![],
            token: None,
        };
        let bytes = crate::proto::encode(&op).unwrap();
        let mut parts = Reassembly::default();
        for slice in bytes.chunks(PART) {
            assert!(parts.take(GuiOp::Part(slice.to_vec().into())).is_none());
        }
        assert!(
            matches!(parts.take(GuiOp::Parts), Some(GuiOp::SayWith { text, images, .. }) if text == "keep this" && images[0].1 == picture)
        );
    }

    /// A Stop cancels its partial upload; the following turn still crosses
    /// the same connection normally. A reassembly
    /// that runs past the cap is dropped whole, not run half.
    #[test]
    fn a_stop_discards_the_upload_and_the_next_turn_still_runs() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = Mutex::new(TcpStream::connect(listener.local_addr().unwrap()).unwrap());
        let (mut far, _) = listener.accept().unwrap();
        let picture = "x".repeat(3 * PART + 17);
        let op = GuiOp::SayWith {
            text: "look".into(),
            images: vec![("image/png".into(), picture.clone())],
            files: vec![],
            token: None,
        };
        let reader = std::thread::spawn(move || {
            let mut parts = Reassembly::default();
            let mut seen = Vec::new();
            while let Ok(op) = read_frame::<GuiOp>(&mut far) {
                if let Some(op) = parts.take(op) {
                    let done = matches!(op, GuiOp::SayWith { .. });
                    seen.push(op);
                    if done {
                        break;
                    }
                }
            }
            seen
        });
        let mut slices = 0;
        write_op(&conn, &op, |sent, all| {
            slices += 1;
            assert!(sent <= all);
            if slices == 2 {
                write_frame(&mut *conn.lock().unwrap(), &GuiOp::Stop).unwrap();
            }
        })
        .unwrap();
        write_frame(
            &mut *conn.lock().unwrap(),
            &GuiOp::SayWith {
                text: "next".into(),
                images: vec![],
                files: vec![],
                token: None,
            },
        )
        .unwrap();
        let seen = reader.join().unwrap();
        assert!(slices >= 4, "{slices} slices");
        assert!(
            matches!(seen[0], GuiOp::Stop),
            "the stop ran before the turn was whole"
        );
        match &seen[1] {
            GuiOp::SayWith { text, images, .. } => {
                assert_eq!(text, "next");
                assert!(images.is_empty());
            }
            _ => panic!("the turn came out whole"),
        }

        let mut parts = Reassembly::default();
        for _ in 0..=(MAX_PARTS / PART) {
            assert!(
                parts
                    .take(GuiOp::Part(serde_bytes::ByteBuf::from(vec![0u8; PART])))
                    .is_none()
            );
        }
        assert!(
            parts.take(GuiOp::Parts).is_none(),
            "past the cap, dropped whole"
        );
        assert!(
            matches!(parts.take(GuiOp::Stop), Some(GuiOp::Stop)),
            "and the next op runs"
        );
    }
}

#[cfg(test)]
mod command_store_root_tests {
    use super::*;
    #[test]
    fn ghost_child_receives_the_requested_store_before_changing_directory() {
        let root = std::env::temp_dir().join(format!(
            "khor-ghost-root-{}",
            crate::link::fresh_hex().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut command = ghost_command(&root, std::path::Path::new("unused-executable")).unwrap();
        command.current_dir(std::env::temp_dir());
        let configured = command
            .get_envs()
            .find(|(key, _)| *key == "KHOR_HOME")
            .and_then(|(_, value)| value)
            .unwrap();
        assert_eq!(
            std::path::PathBuf::from(configured),
            std::fs::canonicalize(&root).unwrap()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, unix))]
#[path = "gui_host_cancel_tests.rs"]
mod cancel_tests;
