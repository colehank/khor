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
//! on this socket, positional arrays, so the wire discipline holds:
//! new fields go at the tail with defaults, none in the middle.
//! **Live frames are broadcast; history is asked for.** An attacher
//! sees the conversation from now on, and one that wants the past sends
//! [`GuiOp::Replay`]: the host asks the agent itself (`session/load`
//! replays the whole conversation before it answers — probed on the
//! real adapter, and re-loading a live session works) and relays the
//! replayed updates as [`GuiNote::History`] frames **to that connection
//! only** — broadcast history would repaint every other face's past.
//! The host still buffers nothing: a second copy here would be a
//! transcript nobody owns. A replay asked for mid-turn waits for the
//! turn to end — the live updates already on screen are the same
//! content, and a load answered during a turn would interleave the two
//! indistinguishably (the protocol does not tag replayed updates).

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
    /// End the conversation: the agent dies with the connection. Held
    /// at its position — frames are positional msgpack, and an old host
    /// reading a new client must fail on the frame, never misread an
    /// earlier variant.
    Close,
    /// Ask for the conversation so far, replayed by the agent itself
    /// and answered as [`GuiNote::History`] frames **to this connection
    /// only**, closed by [`GuiNote::HistoryEnd`]. Sent mid-turn it
    /// waits for the turn to end (module head). Tail-appended.
    Replay,
    /// End the turn that is running — not the session. The agent stays,
    /// its conversation stays, and the ending is `Cancelled`, which is
    /// 空闲: stopping it was the user's own act, so nothing is broken.
    /// Tail-appended.
    Stop,
    /// One user turn with images attached.
    ///
    /// **Tail-appended, and a separate variant rather than a field on
    /// [`GuiOp::Say`]** — these frames are positional msgpack, so a
    /// field added inside `Say` would be read by an old host as a
    /// different `Say`, silently. A variant at the end fails to decode
    /// instead, which is the whole rule this enum's `Close` comment
    /// already states.
    ///
    /// `(mime, base64)` pairs. The data is carried rather than a path
    /// because the face that has the image may be on another machine
    /// entirely — a web face on a phone pasting a screenshot has no
    /// path this host could open.
    /// `images` are `(mime, base64)`; `files` are `(name, text)` and
    /// ride as the protocol's embedded resources.
    ///
    /// **Both lists on the one variant, and it never grows a third**:
    /// this frame is positional msgpack, so the shape it ships with is
    /// the shape it keeps. Anything else attachable later gets its own
    /// variant at the tail, which fails loudly on an old host instead
    /// of being misread as this one.
    SayWith {
        text: String,
        images: Vec<(String, String)>,
        files: Vec<(String, String)>,
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
}

/// How big one [`GuiOp::Part`] is: well under the host's frame cap, and
/// small enough that an op sent between two slices waits for at most
/// one of them.
pub const PART: usize = 256 * 1024;

/// The most an op sent in slices may come to. A turn's attachments are
/// pictures and text files a person picked; past this the face says
/// so before sending, and the host drops what it was sent.
pub const MAX_PARTS: usize = 64 * 1024 * 1024;

/// Writes one op to a host connection — **in slices when it is too big
/// for one frame** ([`GuiOp::Part`]), taking `conn` for each slice
/// alone, so other ops on the same connection go between them.
/// `progress` hears (bytes written, bytes in all) after each slice.
pub fn write_op(
    conn: &Mutex<TcpStream>,
    op: &GuiOp,
    mut progress: impl FnMut(usize, usize),
) -> Result<(), String> {
    let bytes = crate::proto::encode(op)?;
    if bytes.len() <= PART {
        write_frame(&mut *conn.lock().unwrap_or_else(|p| p.into_inner()), op)?;
        progress(bytes.len(), bytes.len());
        return Ok(());
    }
    if bytes.len() > MAX_PARTS {
        return Err(msg::frame_too_big(bytes.len()));
    }
    let mut sent = 0;
    for slice in bytes.chunks(PART) {
        write_frame(
            &mut *conn.lock().unwrap_or_else(|p| p.into_inner()),
            &GuiOp::Part(serde_bytes::ByteBuf::from(slice)),
        )?;
        sent += slice.len();
        progress(sent, bytes.len());
    }
    write_frame(
        &mut *conn.lock().unwrap_or_else(|p| p.into_inner()),
        &GuiOp::Parts,
    )
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
    fn take(&mut self, op: GuiOp) -> Option<GuiOp> {
        match op {
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
    /// One replayed update (`session/update` JSON, [`GuiNote::Note`]'s
    /// shape), answering this connection's [`GuiOp::Replay`] — never
    /// broadcast. Tail-appended.
    History(String),
    /// The replay is complete. Arrives with no [`GuiNote::History`]
    /// before it when there is nothing to replay **or the load failed**
    /// — to a face the two are one fact: no past to paint, the live
    /// stream unaffected. Tail-appended.
    HistoryEnd,
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
    let exe = crate::self_exe()?;
    let ready = std::env::temp_dir().join(format!(
        "khor-gui-ready-{}-{}",
        std::process::id(),
        crate::link::fresh_hex()?
    ));
    let mut c = ghost_command(root, &exe)?;
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
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    c.spawn().map_err(msg::host_wont_start)?;
    // Longer than host.rs's wait on purpose: before the marker can
    // exist an agent process has to boot and answer two requests.
    for _ in 0..600 {
        if let Ok(id) = std::fs::read_to_string(&ready) {
            let _ = std::fs::remove_file(&ready);
            return Ok(SessionId(id.trim().to_owned()));
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
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(msg::ghost_never_ready().into())
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

struct Fanout {
    clients: Mutex<Vec<(u64, std::net::TcpStream)>>,
}

impl Fanout {
    fn send(&self, note: &GuiNote) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|(_, c)| write_frame(c, note).is_ok());
    }

    /// To one connection — history frames only ever travel this way.
    fn send_to(&self, who: u64, note: &GuiNote) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|(id, c)| *id != who || write_frame(c, note).is_ok());
    }
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
    let saved_context = resume
        .as_ref()
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
    let retained_environment = environment.clone();
    let (handle, mut events) = rt
        .block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(28),khor_acp::start_configured(&command,cwd,resume.clone(),environment)).await.unwrap_or_else(|_|Err(khor_acp::Refusal::Wont("The agent did not finish the ACP handshake within 28 seconds".into())))
        })
        // The opener is waiting on a file, and this is the only moment
        // that knows why there will never be one. Written before
        // returning, so the wait ends at the refusal rather than at its
        // own timeout thirty seconds later.
        .map_err(|r| {
            let why = refusal_words(&r);
            let _ = std::fs::write(refusal_path(&ready), &why);
            why
        })?;

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
    // The vendor's uuid is the id — the whole merge story hangs on this
    // line (module head).
    let id = crate::adaptor::id_for(&handle.session().0);
    if resume.is_some() {
        // The takeover re-seats a row that already exists (and clears
        // the ending the takeover itself just caused).
        live.reopen(&id, khor_core::kind::GUI, &title, std::process::id())?;
    } else {
        // The vendor the opener declared, if it did: the wizard's own
        // 智能体 field is the user saying whose session this is, which
        // is not the guess the argv ruling forbids.
        let vendor = std::env::var(VENDOR_ENV).ok();
        live.register(
            &id,
            khor_core::kind::GUI,
            &title,
            Some(std::process::id()),
            vendor.as_deref(),
        )?;
        // **This process was started standing in it** — the opener
        // spawned the ghost with `current_dir`, so the answer is right
        // here and needs no argument passed down
        // (`khor_core::Session::place`).
        if let Some(cwd) = crate::cwd::standing() {
            live.learn_place(&id, &cwd)?;
        }
        // Register wrote the spawn's 忙碌; a fresh GUI session is a
        // prompt nobody has typed into yet.
        live.report(&id, State::Idle, Source::Reported)?;
    }
    let dir = live
        .dir_of(&id)
        .ok_or_else(|| msg::not_a_session_id(&id.0))?;

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
    std::fs::write(&ready, &id.0).map_err(|e| e.to_string())?;

    let fanout = Arc::new(Fanout {
        clients: Mutex::new(Vec::new()),
    });
    let (ops_tx, mut ops_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, GuiOp)>();
    // A face that just arrived, so the loop can hand it what is
    // outstanding. Its own channel rather than a `GuiOp`: joining is
    // not something a face *does*, it is something that happened to
    // this host, and a face must not be able to spell it.
    let (joined_tx, mut joined_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();

    // Attachers: cookie in, frames both ways after the welcome. Each
    // gets a number so a replay can be answered to its asker alone;
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
                        why: String::new(),
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
                fanout.clients.lock().unwrap().push((who, conn));
                let _ = joined_tx.send(who);
                let ops_tx = ops_tx.clone();
                std::thread::spawn(move || {
                    let mut parts = Reassembly::default();
                    while let Ok(op) = read_frame::<GuiOp>(&mut readable) {
                        let Some(op) = parts.take(op) else {
                            continue;
                        };
                        if ops_tx.send((who, op)).is_err() {
                            break;
                        }
                    }
                });
            }
        });
    }

    // The conversation loop: one task, owning every state change, so
    // the words cannot race each other.
    let loop_live = live.clone();
    let loop_id = id.clone();
    let loop_fanout = fanout.clone();
    let ending = rt.block_on(async move {
        let (live, id, fanout) = (loop_live, loop_id, loop_fanout);
        // The pending asks, each with the very note it was announced
        // by: a face that arrives later must be told the same sentence
        // the first face got, not a second rendering of it.
        let mut asks: Vec<(u64, khor_acp::Ask, GuiNote)> = Vec::new();
        let mut elicits:Vec<(u64,khor_acp::Elicitation,GuiNote)>=Vec::new();
        let mut next_ask: u64 = 0;
        // Who asked for history while a turn was running: answered when
        // the turn ends (module head — a mid-turn load would interleave
        // replayed and live updates indistinguishably).
        let mut replay_wanted: Vec<u64> = Vec::new();
        let mut replay: Option<Replay> = None;
        let (replay_tx,mut replay_rx)=tokio::sync::mpsc::unbounded_channel::<Result<(),String>>();
        let mut in_turn: Option<tokio::task::JoinHandle<()>> = None;
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
        let (turn_tx, mut turn_rx) =
            tokio::sync::mpsc::unbounded_channel::<Result<String, String>>();
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
        let (control_tx,mut control_rx)=tokio::sync::mpsc::unbounded_channel::<(u64,String,String,Result<serde_json::Value,String>)>();
        let handle = Arc::new(handle);
        let mut controlling=false;
        let setting_gate=Arc::new(tokio::sync::Mutex::new(()));
        let mut delegated: Option<crate::delegation::HostedTask> = None;
        let mut cancellation_sent: Option<std::time::Instant> = None;
        let mut delegation_tick = tokio::time::interval(std::time::Duration::from_millis(200));
        delegation_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                // Biased, events first: a turn's last update and its end
                // become ready together, and a face must read the words
                // before the goodbye — an unbiased pick would reorder
                // them on some runs and every screen would inherit the
                // dice.
                biased;
                event = events.recv() => match event {
                    Some(khor_acp::Event::Note(n)) => {
                        let json = serde_json::to_string(&n).unwrap_or_default();
                        if let Some(replay)=replay.as_mut(){replay.push(json);continue;}
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
                        let _ = live.report(&id, State::Blocked, Source::Reported);
                    }
                    Some(khor_acp::Event::Elicit(request))=>{
                        let encoded=serde_json::to_value(&request.request).unwrap_or_default();
                        if encoded["sessionId"].as_str().is_some_and(|s|s!=handle.session().0.as_ref()) {drop(request);continue;}
                        let ask=next_ask;next_ask+=1;
                        let note=GuiNote::Elicit{ask,request:encoded.to_string()};
                        fanout.send(&note);elicits.push((ask,request,note));
                        let _=live.report(&id,State::Blocked,Source::Reported);
                    }
                    Some(khor_acp::Event::ElicitDone(elicitation_id))=>{
                        if let Some(at)=elicits.iter().position(|(_,q,_)|serde_json::to_value(&q.request).is_ok_and(|v|v["mode"]=="url"&&v["elicitationId"]==elicitation_id)){
                            let(ask,q,_)=elicits.remove(at);let value=serde_json::json!({"action":"accept"});if let Ok(response)=q.validate(&value){let _=q.answer(response);}
                            fanout.send(&GuiNote::ElicitAnswered{ask,response:value.to_string()});
                            let _=live.report(&id,if asks.is_empty()&&elicits.is_empty(){if in_turn.is_some(){State::Busy}else{State::Idle}}else{State::Blocked},Source::Reported);
                        }
                    }
                    Some(khor_acp::Event::Ready { .. }) => {}
                    // A channel that just stops is an ending too, and
                    // painting it any other way would need a word khor
                    // does not have.
                    Some(khor_acp::Event::Closed(err)) => break err,
                    None => break None,
                },
                // A face that arrived mid-ask gets the ask. Nothing
                // else is re-sent: the conversation's past is what
                // `Replay` is for, and the row already carries the word.
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
                _ = delegation_tick.tick(), if delegated.is_some() => {
                    if cancellation_sent.is_some_and(|at| at.elapsed() >= std::time::Duration::from_secs(10)) {
                        break Some("ACP did not acknowledge cancellation; closing its host".into());
                    }
                    if cancellation_sent.is_none() && delegated.as_ref().is_some_and(|task| task.cancel_requested()) {
                        cancellation_sent = Some(std::time::Instant::now());
                        let _ = handle.cancel();
                        asks.clear(); // Dropping permission senders cancels pending asks.
                    }
                }
                op = ops_rx.recv() => match op {
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
                            let _=live.report(&id,if asks.is_empty()&&elicits.is_empty(){if in_turn.is_some(){State::Busy}else{State::Idle}}else{State::Blocked},Source::Reported);
                        }else{fanout.send_to(who,&GuiNote::Refused{why:refused::SETTING_REFUSED.into()});}
                    }
                    Some((who,GuiOp::Control{token,operation,payload}))=>{
                        if in_turn.is_some()||replay.is_some()||controlling{fanout.send_to(who,&GuiNote::ControlResult{token,operation,data:None,error:Some(refused::TURN_IN_FLIGHT.into())});continue;}
                        let parsed=serde_json::from_str(&payload);
                        if token.is_empty()||token.len()>128||payload.len()>16*1024||operation.len()>64||parsed.is_err(){fanout.send_to(who,&GuiNote::ControlResult{token,operation,data:None,error:Some("Invalid session operation".into())});continue;}
                        controlling=true;
                        let h=handle.clone();let tx=control_tx.clone();
                        tokio::spawn(async move{let result=tokio::time::timeout(std::time::Duration::from_secs(120),h.control(&operation,&parsed.unwrap())).await.unwrap_or_else(|_|Err("The agent did not answer this operation".into()));let _=tx.send((who,token,operation,result));});
                    }
                    Some((who, GuiOp::Delegate { task, message })) => {
                        if in_turn.is_some() || replay.is_some() || controlling {
                            fanout.send_to(who, &GuiNote::Delegated { task, why: Some("ACP session already has a running turn".into()) });
                            continue;
                        }
                        match crate::delegation::HostedTask::claim(&root, &id, &task, &message) {
                            Ok((execution, text)) => {
                                delegated = Some(execution);
                                cancellation_sent = None;
                                let _ = live.report(&id, State::Busy, Source::Reported);
                                let handle = handle.clone();
                                let turn_tx = turn_tx.clone();
                                in_turn = Some(tokio::spawn(async move {
                                    let out = handle.prompt(&text).await.map(|stop| format!("{stop:?}"));
                                    let _ = turn_tx.send(out);
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
                        let (text, images, files) = match op {
                            GuiOp::Say(t) => (t, Vec::new(), Vec::new()),
                            GuiOp::SayWith { text, images, files } => (text, images, files),
                            _ => unreachable!("the pattern above admits nothing else"),
                        };
                        if in_turn.is_some() || replay.is_some() || controlling {
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
                        let _ = live.report(&id, State::Busy, Source::Reported);
                        let handle = handle.clone();
                        let turn_tx = turn_tx.clone();
                        in_turn = Some(tokio::spawn(async move {
                            let out = handle
                                .prompt_with(&text, &images, &files)
                                .await
                                .map(|stop| format!("{stop:?}"));
                            let _ = turn_tx.send(out);
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
                            let _ = live.report(&id, State::Busy, Source::Reported);
                        }
                    }
                    Some((who, GuiOp::Stop)) => {
                        cancel_elicits(&mut elicits,&fanout);
                        // The cancel goes to the agent, not to the task:
                        // aborting the join handle here would leave the
                        // agent still working with nobody reading it.
                        // The turn ends the way every turn ends, through
                        // its own `Turn` note.
                        if in_turn.is_some() || replay.is_some() || controlling {
                            let _ = handle.cancel();
                            for(ask,_,_)in asks.drain(..){fanout.send(&GuiNote::Answered{ask,option:None});}
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
                        tokio::spawn(async move {let _guard=gate.lock().await;
                            let out = handle.set_mode(&mode).await.map(|()| Setting::Mode(mode));
                            let _ = set_tx.send((who, out));
                        });
                    }
                    Some((who, GuiOp::SetOption { id: option, value })) => {
                        let handle = handle.clone();
                        let set_tx = set_tx.clone();let gate=setting_gate.clone();
                        tokio::spawn(async move {let _guard=gate.lock().await;
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
                    Some((who, GuiOp::Replay)) => {
                        // **The capability decides what a face is
                        // told, never whether the agent is asked.** An
                        // agent that under-advertises is a real animal
                        // — this repo's own stub was one for months,
                        // implementing `session/load` while claiming
                        // nothing — and skipping the request on its say
                        // so would withhold a past it actually has. So
                        // the load always goes out; `GuiNote::Agent` is
                        // what keeps an empty answer from reading as an
                        // empty conversation.
                        if in_turn.is_some() {
                            replay_wanted.push(who);
                        } else if let Some(active)=replay.as_mut(){
                            if !active.who.contains(&who){active.who.push(who);}
                        } else {
                            replay=Some(Replay::start(vec![who],handle.clone(),replay_tx.clone()));
                        }
                    }
                    None => break None,
                },
                turn = turn_rx.recv() => {
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
                            let _ = live.report(&id, word, Source::Reported);
                            fanout.send(&GuiNote::Turn(stop));
                        }
                        Some(Err(e)) => {
                            let _ = live.report(&id, State::Errored, Source::Reported);
                            fanout.send(&GuiNote::Turn(e));
                        }
                        None => {}
                    }
                    // The turn is over: the replays that waited on it,
                    // in the order they were asked for.
                    if !replay_wanted.is_empty(){
                        replay=Some(Replay::start(std::mem::take(&mut replay_wanted),handle.clone(),replay_tx.clone()));
                    }
                }
                result=replay_rx.recv()=>{
                    if let(Some(active),Some(result))=(replay.take(),result){
                        cancel_elicits(&mut elicits,&fanout);
                        for who in active.who{
                            if result.is_ok()&&!active.overflow{for note in &active.notes{fanout.send_to(who,&GuiNote::History(note.clone()));}}
                            else{fanout.send_to(who,&GuiNote::ControlResult{token:"replay".into(),operation:"replay".into(),data:None,error:Some(result.as_ref().err().cloned().unwrap_or_else(||"Session history exceeds the replay limit".into()))});}
                            fanout.send_to(who,&GuiNote::HistoryEnd);
                        }
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
                result=control_rx.recv()=>{
                    if let Some((who,token,operation,result))=result {
                        controlling=false;
                        let end=result.is_ok()&&matches!(operation.as_str(),"close"|"delete"|"logout");
                        let(data,error)=match result{Ok(value)=>(Some(value.to_string()),None),Err(e)=>(None,Some(e))};
                        let deleting=operation=="delete";
                        fanout.send_to(who,&GuiNote::ControlResult{token,operation,data,error});
                        if end{if deleting{let _=std::fs::remove_file(&context);}break None;}
                    }
                }
            }
        }
    });

    fanout.send(&GuiNote::Gone);
    for (_, c) in fanout.clients.lock().unwrap().drain(..) {
        let _ = c.shutdown(std::net::Shutdown::Both);
    }
    // 0 for an ending nobody minded, 1 for a torn connection: the exit
    // code is how `record_exit` tells 完成-shaped endings from 失败.
    let _ = live.record_exit(&id, if ending.is_none() { 0 } else { 1 });
    Ok(())
}

fn context_path(root: &std::path::Path, session: &str) -> PathBuf {
    root.join(".khor/acp-context").join(format!(
        "{}.json",
        blake3::hash(session.as_bytes()).to_hex()
    ))
}

/// History is buffered while the main loop continues answering interactions.
/// A failed/oversized load never paints an apparently complete partial past.
struct Replay {
    who: Vec<u64>,
    notes: Vec<String>,
    bytes: usize,
    overflow: bool,
}
impl Replay {
    fn start(
        who: Vec<u64>,
        handle: Arc<khor_acp::Handle>,
        tx: tokio::sync::mpsc::UnboundedSender<Result<(), String>>,
    ) -> Self {
        tokio::spawn(async move {
            let result = handle.replay().await;
            let _ = tx.send(result);
        });
        Self {
            who,
            notes: Vec::new(),
            bytes: 0,
            overflow: false,
        }
    }
    fn push(&mut self, json: String) {
        // Transient notices must never become history, even from a broken adapter.
        if serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .is_some_and(|v| v["update"]["sessionUpdate"] == "notice")
        {
            return;
        }
        self.bytes = self.bytes.saturating_add(json.len());
        if self.bytes > 32 * 1024 * 1024 {
            self.overflow = true;
            self.notes.clear();
        }
        if !self.overflow {
            self.notes.push(json);
        }
    }
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

    /// **A big op crosses in slices and comes out whole, with a stop
    /// between the slices running as it arrives** — and a reassembly
    /// that runs past the cap is dropped whole, not run half.
    #[test]
    fn a_sliced_turn_comes_back_whole_and_a_stop_goes_between() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = Mutex::new(TcpStream::connect(listener.local_addr().unwrap()).unwrap());
        let (mut far, _) = listener.accept().unwrap();
        let picture = "x".repeat(3 * PART + 17);
        let op = GuiOp::SayWith {
            text: "look".into(),
            images: vec![("image/png".into(), picture.clone())],
            files: vec![],
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
        let seen = reader.join().unwrap();
        assert!(slices >= 4, "{slices} slices");
        assert!(
            matches!(seen[0], GuiOp::Stop),
            "the stop ran before the turn was whole"
        );
        match &seen[1] {
            GuiOp::SayWith { text, images, .. } => {
                assert_eq!(text, "look");
                assert_eq!(images[0].1, picture);
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
