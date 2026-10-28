//! The chat registry: live attachments to GUI-session hosts, held for a
//! frontend that can only poll.
//!
//! A webview cannot hold a TCP socket, and the dev bridge serves
//! requests one at a time — a poll that *waited* for frames would block
//! every other call behind it. So the socket lives here: one reader
//! thread per open chat collects frames into a cursor-addressed list,
//! and [`chat_poll`] answers instantly with whatever arrived since the
//! cursor. The frontend's cadence is its own business; nothing here is
//! lost between polls.
//!
//! This layer curates nothing (the host's rule, `gui_host` module
//! head): updates pass through as the protocol's own JSON, and what to
//! paint is the face's judgment.

use std::collections::HashMap;
#[cfg(test)]
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[path = "chat_send.rs"]
mod send;
use send::{Sending, Stage, StopAction, StopSignal, StopWriter};

use khor_node::gui_host::{Choice, GuiNote, GuiOp, write_op_guarded};
use khor_node::host::{read_frame, write_frame};
use khor_node::{Node, SessionId};
use serde::Serialize;
use ts_rs::TS;

/// One frame of a conversation, shaped for a screen. `update` carries
/// the `session/update` notification as parsed JSON — `unknown` on the
/// TS side, because its shape is the protocol's and the face narrows
/// it; this layer neither validates nor curates it.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatFrame {
    /// One live update.
    Note {
        #[ts(type = "unknown")]
        update: serde_json::Value,
    },
    /// One replayed update — this chat asked for history and this is
    /// part of the answer, older than every `Note`.
    History {
        #[ts(type = "unknown")]
        update: serde_json::Value,
    },
    /// The history is complete (possibly empty — no past to paint).
    HistoryEnd,
    /// The agent asked permission; answer with [`chat_answer`].
    Ask {
        #[ts(type = "number")]
        ask: u64,
        title: String,
        options: Vec<(String, String)>,
    },
    /// A turn was already running when this face attached.
    Turning,
    /// An [`ChatFrame::Ask`] has its answer: the chosen option id, or
    /// `None` for a refusal. The label is not carried — the `Ask` this
    /// answers is in the same stream and holds the words.
    Answered {
        #[ts(type = "number")]
        ask: u64,
        option: Option<String>,
    },
    /// The turn ended, with the protocol's stop reason.
    Turn { stop: String },
    /// The conversation is over; no more frames follow.
    Gone,
    /// What the agent on the other end can do — told once, when this
    /// face attaches (`khor_node::gui_host::GuiNote::Agent`).
    ///
    /// `replays` is the one a face must act on: an agent that does not
    /// replay answers a history request with an empty bracket, which is
    /// **the same bracket** a brand-new conversation answers with. A
    /// pane that only watched the answer would paint "nothing was said
    /// here" for an agent whose past simply cannot be fetched.
    Agent {
        name: Option<String>,
        replays: bool,
        images: bool,
        embeds: bool,
        lists: bool,
        needs_login: bool,
    },
    /// An op this face sent that did not take effect
    /// (`khor_node::gui_host::GuiNote::Refused`), and why — as a
    /// **key**, looked up at the last moment like every other word.
    /// Sent to the face that sent the op and to nobody else.
    Refused { why: String },
    /// **A turn that did not go** (NOTICE-BRIEF): its bytes could not be
    /// written to the host — the road to that machine broke under it —
    /// and why, in parts. The face takes the line back off its
    /// transcript and offers 重试 / 取消 on the session's pill. ⚠️ 墓碑:
    /// a `Refused` frame carrying the write's own error, which the face
    /// read as 「当前没有进行中的回合」.
    Unsent {
        miss: crate::miss::Miss,
        token: Option<String>,
    },
    /// An admitted input was cancelled before reaching the agent. The optional
    /// client token correlates the retained draft without keeping its payload.
    SendCancelled {
        token: Option<String>,
        reconnect: bool,
    },
    /// The complete input was written, including its final commit frame.
    /// This is not an acknowledgement of remote execution or agent completion.
    SendCommitted { token: String },
    /// What this session can be set to and what it is set to
    /// (`khor_node::gui_host::GuiNote::Settings`): `{ modes, options }`
    /// in the protocol's own JSON, `unknown` here for the reason
    /// `update` is. Told once on attach and again whenever it moves —
    /// a face repaints its controls from the latest and never merges.
    Settings {
        #[ts(type = "unknown")]
        settings: serde_json::Value,
    },
    Capabilities {
        #[ts(type = "unknown")]
        capabilities: serde_json::Value,
    },
    Elicit {
        #[ts(type = "number")]
        ask: u64,
        #[ts(type = "unknown")]
        request: serde_json::Value,
    },
    ElicitAnswered {
        #[ts(type = "number")]
        ask: u64,
        #[ts(type = "unknown")]
        response: serde_json::Value,
    },
    ControlResult {
        token: String,
        operation: String,
        #[ts(type = "unknown")]
        data: Option<serde_json::Value>,
        error: Option<String>,
    },
}

/// What a poll answers: the frames since the cursor, the next cursor,
/// and whether the conversation has ended — `gone` covers both the
/// host's own goodbye and a connection that simply broke, because to a
/// face those are one fact.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ChatBatch {
    pub frames: Vec<ChatFrame>,
    #[ts(type = "number")]
    pub next: u64,
    pub gone: bool,
    /// Frames older than this cursor are gone, so what arrived here is
    /// not the whole of what was missed. A face that sees this must ask
    /// for history rather than trust its own paint — see [`FRAME_CAP`].
    pub lost: bool,
    /// A turn with attachments on its way to the host: bytes sent so far
    /// and in all (`chat_say_with`, which sends a big turn in slices);
    /// `sending_of` is 0 when nothing is on its way. What lets a face say
    /// 正在发送 while a phone's photos cross a slow road, instead of a
    /// box that looks like nothing happened.
    #[ts(type = "number")]
    pub sending: u64,
    #[ts(type = "number")]
    pub sending_of: u64,
    /// The pipe to a far host ended and is being asked for again
    /// (`crate::rejoin`), and why, in parts — the terminal's
    /// [`crate::term::TermChunk::stall`], for a conversation.
    pub stall: Option<crate::miss::Miss>,
}

/// How many frames one attachment keeps.
///
/// **It has to be bounded, and bounding it has to be honest.** A
/// streaming agent emits a frame per chunk; a conversation left open all
/// day is an unbounded list nobody empties. But the list is also what a
/// *second* pane on the same conversation rebuilds its paint from
/// (`chat_open` answers `false` and that face polls from zero), so
/// dropping the front is dropping somebody's past.
///
/// Hence the pair: the oldest go, and the batch says `lost` to any
/// reader whose cursor was among them. The recovery is the one that
/// already exists — ask for history, exactly as a fresh attachment does
/// — so a trimmed list costs a replay and never a wrong picture.
///
/// Twenty thousand at roughly a couple of hundred bytes a frame is a
/// few megabytes, and reaching it unseen would take a face polling
/// slower than two thousand frames a second. That is not a promise it
/// cannot happen; it is why the `lost` path is written rather than
/// assumed unreachable.
const FRAME_CAP_DEFAULT: usize = 20_000;

/// How many frames one attachment keeps, `KHOR_CHAT_FRAME_CAP` if it is
/// set and sane.
///
/// **An operations knob, and verification is only its first customer.**
/// How much memory a face may spend on one conversation is the kind of
/// thing a small machine will eventually want to say something about,
/// and this repo already hands those to the environment. That it also
/// makes the trim reachable in a test is the reason it exists *now*
/// rather than later — said plainly here, because a knob whose only
/// caller is a test is a knob that gets deleted by the next person to
/// read it.
///
/// Read once: a cap that changed under a running attachment would move
/// the floor beneath a cursor mid-conversation.
pub fn frame_cap() -> usize {
    static CAP: OnceLock<usize> = OnceLock::new();
    *CAP.get_or_init(|| {
        std::env::var("KHOR_CHAT_FRAME_CAP")
            .ok()
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 1)
            .unwrap_or(FRAME_CAP_DEFAULT)
    })
}

/// What a trim leaves behind: four fifths of the cap. Trimming in one
/// bite rather than one frame per push keeps the cost amortised — a
/// `drain` from the front is the length of what remains.
fn frame_keep() -> usize {
    (frame_cap() * 4 / 5).max(1)
}

/// The frames one attachment has collected, and how many it has let go.
///
/// **The cursor counts frames, not positions.** A face holds a number
/// and asks for everything after it; if that number were an index into
/// a list whose front can disappear, a trim would silently hand it
/// somebody else's frames. Counting every frame the attachment ever had
/// keeps the number meaning one thing for as long as the attachment
/// lives.
#[derive(Default)]
struct Frames {
    held: Vec<ChatFrame>,
    dropped: u64,
}

impl Frames {
    /// Where a reader's cursor lands in what is still held, and whether
    /// anything it had not seen is already gone.
    fn since(&self, cursor: u64) -> (usize, bool) {
        (
            (cursor.saturating_sub(self.dropped) as usize).min(self.held.len()),
            cursor < self.dropped,
        )
    }

    /// An attachment made again (`crate::rejoin`) starts a new life:
    /// nothing of the old one is held, and the front moves one past every
    /// cursor a face can hold in it — so every face's next poll says
    /// `lost`, and the face does what it already does for a trimmed
    /// list: clear, and ask for history (now on the new attachment).
    fn restart(&mut self) {
        let total = self.dropped + self.held.len() as u64;
        self.held.clear();
        self.dropped = total + 1;
    }

    fn push(&mut self, f: ChatFrame) {
        self.held.push(f);
        if self.held.len() > frame_cap() {
            let go = self.held.len() - frame_keep();
            self.held.drain(..go);
            self.dropped += go as u64;
        }
    }
}

struct Chat {
    /// The write half. One lock per op so frames never interleave.
    conn: crate::connection::Connection,
    commands: crate::command_queue::CommandQueue,
    turns: crate::command_queue::CommandQueue,
    sends: Mutex<Vec<std::sync::Weak<Sending>>>,
    frames: Mutex<Frames>,
    gone: AtomicBool,
    /// Between a far pipe ending and the next one ([`ChatBatch::stall`]).
    stall: Mutex<Option<crate::miss::Miss>>,
    /// The far machine it lives on, `None` when it is held here: the
    /// machine a turn that did not go names.
    far: Option<String>,
    /// Woken whenever a frame lands, the attachment is made again or
    /// ends: what a held [`chat_wait`] waits on.
    arrived: tokio::sync::Notify,
    /// Bytes of the turn on its way sent so far, and in all (0: none).
    sending: (AtomicU64, AtomicU64),
    /// How many opens hold this attachment. React mounts an effect,
    /// cleans it up, and mounts again — two holders of one id, in an
    /// order the wire does not preserve; without the count, whichever
    /// leave lands last closes the survivor's connection out from
    /// under it (seen live: the pane polled a registry its own first
    /// mount had emptied).
    holds: AtomicUsize,
    /// Nobody holds it any more: a pipe that ends now is not asked for
    /// again (`crate::rejoin`).
    left: AtomicBool,
}

struct SendJob {
    chat: Arc<Chat>,
    sending: Arc<Sending>,
    complete: bool,
}

impl Drop for SendJob {
    fn drop(&mut self) {
        if !self.complete {
            failed_send(&self.chat, &self.sending, "attachment is closed");
        }
        self.sending.finish();
    }
}

fn chats() -> &'static Mutex<HashMap<String, Arc<Chat>>> {
    static CHATS: OnceLock<Mutex<HashMap<String, Arc<Chat>>>> = OnceLock::new();
    CHATS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn frame_of(note: GuiNote) -> ChatFrame {
    // The host serialized these itself, so a parse failure is not a
    // path worth a word of its own: the raw text survives as a JSON
    // string and the face sees something is off instead of nothing.
    let parse =
        |s: String| serde_json::from_str(&s).unwrap_or_else(|_| serde_json::Value::String(s));
    match note {
        GuiNote::Note(s) => ChatFrame::Note { update: parse(s) },
        GuiNote::History(s) => ChatFrame::History { update: parse(s) },
        GuiNote::HistoryEnd => ChatFrame::HistoryEnd,
        GuiNote::Ask {
            ask,
            title,
            options,
        } => ChatFrame::Ask {
            ask,
            title,
            options,
        },
        GuiNote::Answered { ask, option } => ChatFrame::Answered { ask, option },
        GuiNote::Turning => ChatFrame::Turning,
        GuiNote::Turn(stop) => ChatFrame::Turn { stop },
        GuiNote::Gone => ChatFrame::Gone,
        GuiNote::Agent {
            name,
            replays,
            images,
            embeds,
            lists,
            needs_login,
        } => ChatFrame::Agent {
            name,
            replays,
            images,
            embeds,
            lists,
            needs_login,
        },
        GuiNote::Refused { why } => ChatFrame::Refused { why },
        GuiNote::Delegated { why: Some(why), .. } => ChatFrame::Refused { why },
        GuiNote::Delegated { why: None, .. } => ChatFrame::Turning,
        GuiNote::Settings(s) => ChatFrame::Settings { settings: parse(s) },
        GuiNote::Capabilities(s) => ChatFrame::Capabilities {
            capabilities: parse(s),
        },
        GuiNote::Elicit { ask, request } => ChatFrame::Elicit {
            ask,
            request: parse(request),
        },
        GuiNote::ElicitAnswered { ask, response } => ChatFrame::ElicitAnswered {
            ask,
            response: parse(response),
        },
        GuiNote::ControlResult {
            token,
            operation,
            data,
            error,
        } => ChatFrame::ControlResult {
            token,
            operation,
            data: data.map(parse),
            error,
        },
    }
}

/// Attaches to the session's host, idempotently: an already-open chat
/// stays as it is (its frames and cursor survive a second open), a
/// dead one is replaced by a fresh attachment. Answers whether this
/// call attached anew — the caller that asks for history on open must
/// not ask twice for one attachment (React mounts effects twice in
/// dev, and a host answers every replay it is asked for).
///
/// **The registry is not held while the attachment is made**, for the
/// terminal's reason (`crate::term::term_open`): a far conversation is
/// reached over the network, and every other conversation's poll and
/// send take this same lock.
pub fn chat_open(root: &Path, id: &str) -> Result<bool, String> {
    {
        let mut registry = chats()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(chat) = registry.get(id) {
            if !chat.gone.load(Ordering::Relaxed) {
                chat.holds.fetch_add(1, Ordering::Relaxed);
                return Ok(false);
            }
            registry.remove(id);
        }
        landing()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .begin(id);
    }
    let attached = attach(root, id);
    let mut registry = chats()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // A leave that came while this open was reaching takes its hold
    // (`crate::landing`).
    let left = landing()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .land(id);
    let (attached, far) = attached?;
    let conn = attached.stream;
    if left {
        let _ = conn.shutdown(std::net::Shutdown::Both);
        return Ok(false);
    }
    if let Some(theirs) = registry.get(id)
        && !theirs.gone.load(Ordering::Relaxed)
    {
        // Another open made it while this one was reaching: theirs is
        // the attachment, and this connection goes unused.
        theirs.holds.fetch_add(1, Ordering::Relaxed);
        let _ = conn.shutdown(std::net::Shutdown::Both);
        return Ok(false);
    }
    let sid = SessionId(id.to_owned());
    let mut reading = conn.try_clone().map_err(|e| e.to_string())?;
    let chat = Arc::new(Chat {
        conn: crate::connection::Connection::new(conn)?,
        commands: Default::default(),
        turns: Default::default(),
        sends: Default::default(),
        frames: Mutex::new(Frames::default()),
        gone: AtomicBool::new(false),
        stall: Mutex::new(None),
        far: far.clone(),
        arrived: tokio::sync::Notify::new(),
        sending: (AtomicU64::new(0), AtomicU64::new(0)),
        holds: AtomicUsize::new(1),
        left: AtomicBool::new(false),
    });
    let reader = chat.clone();
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        loop {
            let mut said_goodbye = false;
            while let Ok(note) = read_frame::<GuiNote>(&mut reading) {
                let over = matches!(note, GuiNote::Gone);
                reader.frames.lock().unwrap().push(frame_of(note));
                reader.arrived.notify_waiters();
                if over {
                    said_goodbye = true;
                    break;
                }
            }
            // **A torn far pipe asks again where that is switched on**
            // (a phone, `crate::rejoin`); the host's own goodbye never
            // does — the conversation is over then, not the pipe.
            if said_goodbye {
                break;
            }
            let Some(machine) = far.as_deref().filter(|_| crate::rejoin::is_on()) else {
                break;
            };
            *reader.stall.lock().unwrap() = Some(crate::miss::Miss::new(
                crate::miss::MissKind::Unreached,
                Some(machine),
                "",
            ));
            reader.arrived.notify_waiters();
            let told = |miss: crate::miss::Miss| {
                *reader.stall.lock().unwrap() = Some(miss);
                reader.arrived.notify_waiters();
            };
            let Some(conn) = crate::rejoin::again(
                &root,
                machine,
                &sid,
                &reader.left,
                |addr, cookie| {
                    let attached = khor_node::gui_host::connect_at(addr, cookie)?;
                    Ok(attached.stream)
                },
                told,
            ) else {
                break;
            };
            let Ok(again) = conn.try_clone() else { break };
            *reader.stall.lock().unwrap() = None;
            // The write half first, so the history a face asks for on
            // seeing `lost` goes down the new attachment.
            if reader.conn.replace(conn).is_err() {
                break;
            }
            reader.frames.lock().unwrap().restart();
            reader.arrived.notify_waiters();
            reading = again;
        }
        // A goodbye and a torn connection land the same way (ChatBatch).
        *reader.stall.lock().unwrap() = None;
        reader.gone.store(true, Ordering::Relaxed);
        reader.commands.close();
        reader.turns.close();
        reader.conn.close();
        reader.arrived.notify_waiters();
    });
    registry.insert(id.to_owned(), chat);
    Ok(true)
}

/// The connection [`chat_open`] attaches with, and the far machine it
/// came through (`None` when the conversation is hosted here). No lock
/// is held: a far one waits on the network.
fn attach(
    root: &Path,
    id: &str,
) -> Result<(khor_node::gui_host::Attached, Option<String>), String> {
    let n = Node::open(root.to_path_buf())?;
    let sid = SessionId(id.to_owned());
    // **An ended conversation says so, in its own words** (UX-4, decided):
    // its host is gone (`State::Failed` is a process that is over), and
    // reaching for it answered a terminal's sentence from the far machine
    // (「没有可连接的终端：tui/…」) or a command line's from this one.
    // Read off the row both machines already share, so no wire changes.
    if n.sessions()?
        .iter()
        .any(|v| v.session.id == sid && v.session.state.state == khor_core::State::Failed)
    {
        return Err(khor_catalog::msg::chat_ended().into());
    }
    // **The fork is on the row, not on a flag** (`Node::far_machine`),
    // the terminal's own rule one door over: a conversation born on
    // another machine (the wizard's 在哪台) is reached through the pipe
    // the resident binds onto its host, and what arrives is the same
    // socket speaking the same frames — nothing below this line knows
    // there is a network under it. The GUI host ignores the terminal
    // size; 0×0 is what its own test clients send.
    let far = n.far_machine(&sid)?;
    let conn = match &far {
        Some(machine) => {
            let (addr, cookie) = crate::term::reach_blocking(root, machine, &sid)?;
            khor_node::gui_host::connect_at(&addr, &cookie)?
        }
        None => {
            let dir = n
                .session_dir(&sid)
                .ok_or_else(|| khor_catalog::msg::no_such_session(id))?;
            khor_node::gui_host::connect(&dir)?
        }
    };
    Ok((conn, far))
}

/// Conversation opens still reaching, and leaves that came before them.
/// Taken only while [`chats`] is held.
fn landing() -> &'static Mutex<crate::landing::Landing> {
    static LANDING: OnceLock<Mutex<crate::landing::Landing>> = OnceLock::new();
    LANDING.get_or_init(Default::default)
}

fn chat_of(id: &str) -> Result<Arc<Chat>, String> {
    chats()
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| khor_catalog::msg::no_such_session(id))
}

/// The frames since `since` — instantly, whatever has arrived so far.
pub fn chat_poll(id: &str, since: u64) -> Result<ChatBatch, String> {
    let chat = chat_of(id)?;
    let frames = chat.frames.lock().unwrap();
    // Everything this reader has not seen, in the list's own terms: its
    // cursor counts frames from the beginning of the attachment, and
    // `dropped` is how far the front has moved since.
    let (from, lost) = frames.since(since);
    Ok(ChatBatch {
        frames: frames.held[from..].to_vec(),
        next: frames.dropped + frames.held.len() as u64,
        gone: chat.gone.load(Ordering::Relaxed),
        lost,
        sending: chat.sending.0.load(Ordering::Relaxed),
        sending_of: chat.sending.1.load(Ordering::Relaxed),
        stall: chat.stall.lock().unwrap().clone(),
    })
}

/// [`chat_poll`], **held open until a frame lands** (or the attachment
/// is made again, or ends), and at the latest after `longest` with
/// nothing new — the terminal's [`crate::term::term_wait`], for a
/// conversation (round NET-2, piece 6).
///
/// ⚠️ 墓碑: the face asked `chat_poll` every 50 ms through a turn and
/// every 500 ms between, so a frame that had already crossed the network
/// waited on average half a beat more. The app's face holds one of these
/// open at a time instead; the bridge answers one request at a time, so
/// the browser face keeps the beat.
pub async fn chat_wait(
    id: &str,
    since: u64,
    longest: std::time::Duration,
) -> Result<ChatBatch, String> {
    let chat = chat_of(id)?;
    let woke = chat.arrived.notified();
    tokio::pin!(woke);
    woke.as_mut().enable();
    let next = {
        let frames = chat.frames.lock().unwrap();
        frames.dropped + frames.held.len() as u64
    };
    if next == since && !chat.gone.load(Ordering::Relaxed) {
        let _ = tokio::time::timeout(longest, woke).await;
    }
    chat_poll(id, since)
}

/// One user turn.
pub fn chat_say(id: &str, text: &str) -> Result<(), String> {
    chat_say_with(id, text, Vec::new(), Vec::new())
}

/// One user turn with things attached: images as `(mime, base64)`,
/// files as `(name, text)`.
///
/// **A different op on the wire, not a longer one** — `GuiOp` is
/// positional msgpack, so `Say` keeps its shape forever and
/// `SayWith` sits at the tail. A host too old to know the new variant
/// fails to decode the frame, which is loud; a host that misread an
/// extended `Say` would run the turn with the text silently mangled.
///
/// Sending this at all is gated on the agent having said it takes
/// images (`Facts::images`, carried to the face as `ChatFrame::Agent`).
/// The gate is up in the face rather than here because refusing after
/// the picture has crossed the wire is a refusal nobody can act on.
///
/// **It can take as long as the road does**, so no caller runs it on a
/// thread that draws (the app's command is `async`; the bridge hands it
/// to a thread). A turn too big for one frame goes in slices
/// (`khor_node::gui_host::write_op`), each taking the connection alone,
/// so a stop pressed meanwhile goes between them; the batch's `sending`
/// says how far it has got. When it cannot go, the face is told on the
/// conversation itself — an `Unsent` frame, the failure in parts — and
/// gets its words back.
pub fn chat_say_with(
    id: &str,
    text: &str,
    images: Vec<(String, String)>,
    files: Vec<(String, String)>,
) -> Result<(), String> {
    submit_say(id, text.to_owned(), images, files)?.wait()
}

/// Capture this attachment and the end of its admitted controls before dispatching.
/// Turns wait for those controls, while later Stop can pass between their slices.
pub fn submit_say(
    id: &str,
    text: String,
    images: Vec<(String, String)>,
    files: Vec<(String, String)>,
) -> Result<crate::command_queue::Pending<()>, String> {
    submit_say_with_token(id, text, images, files, None)
}

pub fn validate_send_token(token: Option<&str>) -> Result<(), String> {
    if token.is_some_and(|token| {
        token.is_empty()
            || token.len() > 128
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    }) {
        return Err("Invalid client send token".into());
    }
    Ok(())
}

pub fn submit_say_with_token(
    id: &str,
    text: String,
    images: Vec<(String, String)>,
    files: Vec<(String, String)>,
    token: Option<String>,
) -> Result<crate::command_queue::Pending<()>, String> {
    validate_send_token(token.as_deref())?;
    let chat = chat_of(id)?;
    let mut sends = chat.sends.lock().unwrap();
    sends.retain(|send| send.strong_count() > 0);
    let sending = Arc::new(Sending::new(token));
    sends.push(Arc::downgrade(&sending));
    let mut job = SendJob {
        chat: chat.clone(),
        sending,
        complete: false,
    };
    let preceding_controls = chat.commands.submit(|| Ok(()))?;
    chat.turns.submit(move || {
        let result = preceding_controls
            .wait()
            .and_then(|_| send_turn(&job.chat, text, images, files, &job.sending));
        if let Err(why) = &result {
            failed_send(&job.chat, &job.sending, why);
        } else if let Some(token) = &job.sending.token
            && job.sending.claim_receipt()
        {
            job.chat
                .frames
                .lock()
                .unwrap()
                .push(ChatFrame::SendCommitted {
                    token: token.clone(),
                });
            job.chat.arrived.notify_waiters();
        }
        job.complete = true;
        result
    })
}

fn send_turn(
    chat: &Chat,
    text: String,
    images: Vec<(String, String)>,
    files: Vec<(String, String)>,
    sending: &Sending,
) -> Result<(), String> {
    let op = GuiOp::SayWith {
        text,
        images,
        files,
    };
    let mut sent = write_op_guarded(
        &chat.conn,
        &op,
        |sent, all| {
            chat.sending.1.store(all as u64, Ordering::Relaxed);
            chat.sending.0.store(sent as u64, Ordering::Relaxed);
            chat.arrived.notify_waiters();
        },
        |step| sending.before(step),
    );
    let cancelled = sending.cancellation();
    if let Some((stage, stop)) = &cancelled {
        let reconnect = match stop.wait() {
            Ok(StopAction::Written) if *stage == Stage::Uploading => {
                // Stop is already on this stream. Parts now ends the host's
                // discard state instead of committing the cancelled payload.
                if write_frame(&mut *chat.conn.lock().unwrap(), &GuiOp::Parts).is_err() {
                    cancelled_notice(chat, sending, true);
                    retire_cancelled_upload(chat);
                    true
                } else {
                    false
                }
            }
            Err(_) if *stage == Stage::Uploading => {
                cancelled_notice(chat, sending, true);
                retire_cancelled_upload(chat);
                true
            }
            _ => false,
        };
        cancelled_notice(chat, sending, reconnect);
        sent = Err(khor_node::drops::cancelled());
    }
    chat.sending.0.store(0, Ordering::Relaxed);
    chat.sending.1.store(0, Ordering::Relaxed);
    chat.arrived.notify_waiters();
    sent
}

/// Controls share one FIFO; whole turns deliberately use another one.
pub enum Control {
    Answer {
        ask: u64,
        option: Option<String>,
    },
    Respond {
        ask: u64,
        response: serde_json::Value,
    },
    Operation {
        token: String,
        operation: String,
        payload: serde_json::Value,
    },
    SetMode(String),
    SetOption {
        option: String,
        value: serde_json::Value,
    },
    Stop,
    Replay,
}

fn retire_cancelled_upload(chat: &Chat) {
    chat.left.store(true, Ordering::Relaxed);
    chat.gone.store(true, Ordering::Relaxed);
    chat.commands.close();
    chat.turns.close();
    chat.conn.close();
    chat.arrived.notify_waiters();
}

fn cancelled_notice(chat: &Chat, send: &Sending, reconnect: bool) {
    if send.claim_notice() {
        chat.frames.lock().unwrap().push(ChatFrame::SendCancelled {
            token: send.token.clone(),
            reconnect,
        });
        chat.arrived.notify_waiters();
    }
}

fn failed_send(chat: &Chat, send: &Sending, why: &str) {
    if send.cancellation().is_some() {
        cancelled_notice(
            chat,
            send,
            chat.left.load(Ordering::Relaxed) || chat.gone.load(Ordering::Relaxed),
        );
    } else if send.claim_receipt() {
        chat.frames.lock().unwrap().push(ChatFrame::Unsent {
            miss: crate::miss::read(why, chat.far.as_deref()),
            token: send.token.clone(),
        });
        chat.arrived.notify_waiters();
    }
}

fn cancelled_notices(chat: &Chat, sends: &[(Arc<Sending>, Stage)], reconnect: bool) {
    let mut frames = chat.frames.lock().unwrap();
    for (send, _) in sends {
        if send.claim_notice() {
            frames.push(ChatFrame::SendCancelled {
                token: send.token.clone(),
                reconnect,
            });
        }
    }
    drop(frames);
    chat.arrived.notify_waiters();
}

fn submit_stop(id: &str) -> Result<crate::command_queue::Pending<()>, String> {
    let chat = chat_of(id)?;
    let mut sends = chat.sends.lock().unwrap();
    sends.retain(|send| send.strong_count() > 0);
    let signal = Arc::new(StopSignal::default());
    let writer = StopWriter(signal.clone());
    let plan = Arc::new(Mutex::new(None::<Vec<(Arc<Sending>, Stage)>>));
    let mut preparing = plan.lock().unwrap();
    let ready = plan.clone();
    let target = chat.clone();
    let pending = chat.commands.submit(move || {
        let plan = ready
            .lock()
            .unwrap()
            .take()
            .expect("Stop admitted before its worker");
        let uploading = plan.iter().any(|(_, stage)| *stage == Stage::Uploading);
        let needs_wire = plan.is_empty()
            || plan
                .iter()
                .any(|(_, stage)| matches!(stage, Stage::Uploading | Stage::Committed));
        let result = if needs_wire {
            write_frame(&mut *target.conn.lock().unwrap(), &GuiOp::Stop)
                .map(|_| StopAction::Written)
        } else {
            Ok(StopAction::Local)
        };
        if result.is_err() && uploading {
            cancelled_notices(&target, &plan, true);
            retire_cancelled_upload(&target);
        } else {
            for (send, stage) in &plan {
                if *stage != Stage::Uploading {
                    cancelled_notice(&target, send, false);
                }
            }
        }
        writer.complete(result.clone());
        result.map(|_| ())
    })?;
    *preparing = Some(
        sends
            .iter()
            .filter_map(std::sync::Weak::upgrade)
            .filter_map(|send| {
                let stage = send.cancel(&signal);
                (stage != Stage::Finished).then_some((send, stage))
            })
            .collect(),
    );
    drop(preparing);
    Ok(pending)
}

/// Validate before enqueueing, retaining the core errors used by every skin.
pub fn submit_control(
    id: &str,
    command: Control,
) -> Result<crate::command_queue::Pending<()>, String> {
    if matches!(command, Control::Stop) {
        return submit_stop(id);
    }
    let op = match command {
        Control::Answer { ask, option } => GuiOp::Answer { ask, option },
        Control::Respond { ask, response } => {
            let response = serde_json::to_string(&response).map_err(|e| e.to_string())?;
            if response.len() > 128 * 1024 {
                return Err("Interaction response is too large".into());
            }
            GuiOp::ElicitAnswer { ask, response }
        }
        Control::Operation {
            token,
            operation,
            payload,
        } => {
            if token.is_empty()
                || token.len() > 128
                || !matches!(
                    operation.as_str(),
                    "close" | "delete" | "authenticate" | "logout"
                )
            {
                return Err("Invalid session operation".into());
            }
            let payload = payload.to_string();
            if payload.len() > 16 * 1024 {
                return Err("Session operation is too large".into());
            }
            GuiOp::Control {
                token,
                operation,
                payload,
            }
        }
        Control::SetMode(mode) => GuiOp::SetMode(mode),
        Control::SetOption { option, value } => {
            let value = match value {
                serde_json::Value::String(v) => Choice::Value(v),
                serde_json::Value::Bool(on) => Choice::Switch(on),
                other => {
                    return Err(format!(
                        "an option is set to a word or a switch, not {other}"
                    ));
                }
            };
            GuiOp::SetOption { id: option, value }
        }
        Control::Stop => GuiOp::Stop,
        Control::Replay => GuiOp::Replay,
    };
    let chat = chat_of(id)?;
    let target = chat.clone();
    chat.commands
        .submit(move || write_frame(&mut *target.conn.lock().unwrap(), &op))
}

/// Chosen option id, or None to refuse. Synchronous bridge callers still wait
/// until the op left; native IPC awaits the same submitted command instead.
pub fn chat_answer(id: &str, ask: u64, option: Option<String>) -> Result<(), String> {
    submit_control(id, Control::Answer { ask, option })?.wait()
}
pub fn chat_respond(id: &str, ask: u64, response: &serde_json::Value) -> Result<(), String> {
    submit_control(
        id,
        Control::Respond {
            ask,
            response: response.clone(),
        },
    )?
    .wait()
}
pub fn chat_control(
    id: &str,
    token: &str,
    operation: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    submit_control(
        id,
        Control::Operation {
            token: token.into(),
            operation: operation.into(),
            payload: payload.clone(),
        },
    )?
    .wait()
}
/// The agent's Settings/Refused frame carries the outcome; this result says
/// only that the op left, as it did before commands acquired a queue.
pub fn chat_set_mode(id: &str, mode: &str) -> Result<(), String> {
    submit_control(id, Control::SetMode(mode.into()))?.wait()
}
pub fn chat_set_option(id: &str, option: &str, value: &serde_json::Value) -> Result<(), String> {
    submit_control(
        id,
        Control::SetOption {
            option: option.into(),
            value: value.clone(),
        },
    )?
    .wait()
}
/// Ends the running turn, not the session; never waits for a whole turn's queue.
pub fn chat_stop(id: &str) -> Result<(), String> {
    submit_control(id, Control::Stop)?.wait()
}
pub fn chat_replay(id: &str) -> Result<(), String> {
    submit_control(id, Control::Replay)?.wait()
}

/// The recorded past of a session khor did not open: the vendor's own
/// transcript, translated into the frames a live replay wears — which
/// is the very translation claude-code-acp's `loadSession` performs,
/// one process closer. Static and whole: no attachment, no cursor, and
/// the answer ends with [`ChatFrame::HistoryEnd`] like any replay, so
/// one fold paints both kinds of past.
pub fn history(root: &Path, id: &str) -> Result<Vec<ChatFrame>, String> {
    let n = Node::open(root.to_path_buf())?;
    let said = n.transcript_of(&SessionId(id.to_owned()))?;
    let mut frames: Vec<ChatFrame> = said
        .into_iter()
        // **The one translation, shared with the live stream.** This
        // used to be a third hand-written copy of it, and the copies
        // drifted the moment the live one learned anything: tool lines
        // here went on saying `Bash` while a watched conversation
        // showed the command, its status and its diff. Almost
        // everything a person reads in an existing conversation comes
        // through here, so the copy that mattered most was the one that
        // never got fixed.
        .filter_map(khor_node::replayed_update)
        .map(|update| {
            let update = serde_json::to_value(&update).unwrap_or(serde_json::Value::Null);
            ChatFrame::History {
                update: serde_json::json!({ "sessionId": id, "update": update }),
            }
        })
        .collect();
    frames.push(ChatFrame::HistoryEnd);
    Ok(frames)
}

/// A discovered conversation, followed: whatever its transcript has
/// said since `from`, and where to continue.
///
/// **Appended lines come back as `note` frames, not `history` ones**,
/// and that is what makes this work without touching the fold: a
/// `history` batch is closed by `HistoryEnd` and *replaces* the past,
/// which is right for a first read and wrong for a continuation — it
/// would leave the pane holding only the newest paragraph. Notes
/// append, which is exactly what an appended line is.
///
/// A rewound file (compaction, rotation) comes back the other way:
/// whole, as `history`, ending in `HistoryEnd`, so the pane repaints.
/// Byte 400 of a new file is not byte 400 of the old one, and appending
/// across that seam would splice two conversations together.
pub fn history_follow(root: &Path, id: &str, from: u64) -> Result<Followed, String> {
    let n = Node::open(root.to_path_buf())?;
    let follow = n.transcript_follow(&SessionId(id.to_owned()), from)?;
    let kind_of = |update: serde_json::Value| {
        let wrapped = serde_json::json!({ "sessionId": id, "update": update });
        if follow.rewound {
            ChatFrame::History { update: wrapped }
        } else {
            ChatFrame::Note { update: wrapped }
        }
    };
    let mut frames: Vec<ChatFrame> = follow
        .said
        .into_iter()
        // The one translation, shared with the live stream — see
        // `history` above for why there is exactly one.
        .filter_map(khor_node::replayed_update)
        .map(|u| kind_of(serde_json::to_value(&u).unwrap_or(serde_json::Value::Null)))
        .collect();
    if follow.rewound {
        frames.push(ChatFrame::HistoryEnd);
    }
    Ok(Followed {
        frames,
        at: follow.at,
        following: follow.following,
    })
}

/// One bounded external-history response. Record references explicitly retain
/// content that cannot safely be expanded into an inline response.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub older: Option<String>,
    pub references: Vec<HistoryRecordRef>,
    pub reset: bool,
    pub encoded_bytes: usize,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct HistoryEntry {
    pub key: String,
    pub frame: ChatFrame,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct HistoryRecordRef {
    #[ts(type = "number")]
    pub start: u64,
    #[ts(type = "number")]
    pub end: u64,
    pub missing_start: bool,
    pub missing_end: bool,
    pub item: Option<usize>,
    pub cursor: String,
}

const HISTORY_PAYLOAD_BYTES: usize = 512 * 1024;
const HISTORY_FRAME_BYTES: usize = HISTORY_PAYLOAD_BYTES - 16 * 1024;

struct EncodedSize(usize);
impl std::io::Write for EncodedSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > HISTORY_PAYLOAD_BYTES {
            return Err(std::io::Error::other(
                "history response exceeds its byte budget",
            ));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encoded_size(value: &impl Serialize) -> Result<usize, String> {
    let mut counter = EncodedSize(0);
    serde_json::to_writer(&mut counter, value).map_err(|e| e.to_string())?;
    Ok(counter.0)
}

pub fn history_page(root: &Path, id: &str, cursor: Option<&str>) -> Result<HistoryPage, String> {
    if cursor.is_some_and(|cursor| cursor.len() > 4096) {
        return Err("history cursor exceeds its byte budget".to_owned());
    }
    let cursor: Option<khor_node::adaptor::transcript_page::Cursor> = cursor
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| format!("invalid history cursor: {e}"))?;
    let node = Node::open(root.to_path_buf())?;
    let page = node.transcript_page(&SessionId(id.to_owned()), cursor.as_ref())?;
    history_page_response(id, page)
}

fn history_page_response(
    id: &str,
    mut page: khor_node::adaptor::transcript_page::Page,
) -> Result<HistoryPage, String> {
    let mut entries = Vec::new();
    let mut frame_bytes = 0;
    let mut first_position = None;
    let mut expanded_references = Vec::new();
    // Traverse backwards to prioritize recent content while keeping a cursor
    // before the oldest admitted event. No discarded event is skipped forever.
    for event in std::mem::take(&mut page.events).into_iter().rev() {
        let Some(update) = khor_node::replayed_update(event.utterance) else {
            continue;
        };
        let update = serde_json::to_value(update).map_err(|e| e.to_string())?;
        let frame = ChatFrame::Note {
            update: serde_json::json!({ "sessionId": id, "update": update }),
        };
        let entry = HistoryEntry {
            key: format!("{}:{}", event.position.start, event.position.item),
            frame,
        };
        let bytes = encoded_size(&entry).unwrap_or(HISTORY_PAYLOAD_BYTES + 1);
        if bytes > HISTORY_FRAME_BYTES {
            expanded_references.push(HistoryRecordRef {
                start: event.position.start,
                end: event.position.end,
                missing_start: false,
                missing_end: false,
                item: Some(event.position.item),
                cursor: serde_json::to_string(&page.cursor_before(event.position.clone()))
                    .map_err(|e| e.to_string())?,
            });
            first_position = Some(event.position);
            continue;
        }
        if frame_bytes + bytes > HISTORY_FRAME_BYTES {
            break;
        }
        frame_bytes += bytes;
        first_position = Some(event.position.clone());
        entries.push(entry);
    }
    entries.reverse();
    let older = if let Some(position) = &first_position {
        if position.start == 0 && position.item == 0 {
            None
        } else {
            Some(page.cursor_before(position.clone()))
        }
    } else {
        page.older.clone()
    };
    let mut references: Vec<_> = page
        .references
        .iter()
        .filter(|reference| {
            first_position
                .as_ref()
                .is_none_or(|first| reference.range.end > first.start)
        })
        .map(|reference| {
            let position = khor_node::adaptor::transcript_page::Position {
                start: reference.range.start,
                end: reference.range.end,
                item: 0,
            };
            Ok(HistoryRecordRef {
                start: reference.range.start,
                end: reference.range.end,
                missing_start: reference.missing_start,
                missing_end: reference.missing_end,
                item: None,
                cursor: serde_json::to_string(&page.cursor_before(position))
                    .map_err(|e| e.to_string())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    references.extend(expanded_references);
    let mut response = HistoryPage {
        entries,
        older: older
            .map(|cursor| serde_json::to_string(&cursor))
            .transpose()
            .map_err(|e| e.to_string())?,
        references,
        reset: page.reset,
        encoded_bytes: 0,
    };
    loop {
        let encoded_bytes = encoded_size(&response)?;
        if encoded_bytes == response.encoded_bytes {
            break;
        }
        response.encoded_bytes = encoded_bytes;
    }
    Ok(response)
}

/// [`history_follow`]'s answer: what to paint, and where to continue.
#[derive(Serialize, TS)]
#[ts(export)]
pub struct Followed {
    pub frames: Vec<ChatFrame>,
    /// Byte offset to hand back next time. **Carried by the caller
    /// rather than remembered here**: this function opens the node and
    /// answers, and a cursor kept on this side would be one per
    /// process, shared by every face watching every session.
    pub at: u64,
    /// Whether asking again can say anything new. `false` is a record
    /// nothing can seek: this answer is the whole of it, and a caller
    /// that kept asking would re-read the file forever.
    pub following: bool,
}

/// Detaches one holder; the socket closes when the last one leaves
/// (`Chat::holds`). **Not a close**: the session and its agent live
/// on, and closing is [`crate::close_session`]'s job — a detail view
/// going away must not take the conversation with it.
pub fn chat_leave(id: &str) -> Result<(), String> {
    let mut registry = chats().lock().unwrap();
    match registry.get(id) {
        Some(chat) => {
            if chat.holds.fetch_sub(1, Ordering::Relaxed) <= 1 {
                chat.left.store(true, Ordering::Relaxed);
                let chat = registry.remove(id).unwrap();
                drop(registry);
                chat.commands.close();
                chat.turns.close();
                chat.conn.close();
                return Ok(());
            }
        }
        // Its open may still be reaching: the hold it would take is
        // this leave's (`crate::landing`).
        None => {
            landing()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .leave_early(id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod frame_cap_tests {
    use super::*;

    /// Each frame carries its own number, so an assertion can name
    /// *which* frame came back rather than only how many.
    fn numbered(n: u64) -> ChatFrame {
        ChatFrame::Turn {
            stop: n.to_string(),
        }
    }

    fn stop_of(f: &ChatFrame) -> &str {
        match f {
            ChatFrame::Turn { stop } => stop,
            other => panic!("not a numbered frame: {other:?}"),
        }
    }

    /// **A cursor keeps meaning the same thing after the front is
    /// trimmed**, which is the whole reason it counts frames instead of
    /// indexing a list.
    ///
    /// The assertion is on *identity*, not on counts: a cursor that
    /// forgot to account for the dropped frames would still return a
    /// plausible number of them, just the wrong ones — and a wrong
    /// picture of a conversation is exactly what nobody would notice.
    #[test]
    fn a_cursor_survives_the_front_being_dropped() {
        let mut f = Frames::default();
        for n in 0..frame_cap() as u64 + 1 {
            f.push(numbered(n));
        }
        assert_eq!(
            f.dropped,
            (frame_cap() + 1 - frame_keep()) as u64,
            "one trim, one bite"
        );
        assert_eq!(f.held.len(), frame_keep());

        // A reader at the very front: what it missed is gone, and it is
        // told so rather than handed the tail as if it were the whole.
        let (from, lost) = f.since(0);
        assert!(
            lost,
            "a cursor older than the trim must be told it lost frames"
        );
        assert_eq!(
            stop_of(&f.held[from]),
            f.dropped.to_string(),
            "and gets the oldest still held"
        );

        // A reader that kept up: no loss, and it lands exactly on the
        // frame it has not seen.
        let caught_up = f.dropped + 5;
        let (from, lost) = f.since(caught_up);
        assert!(!lost);
        assert_eq!(stop_of(&f.held[from]), caught_up.to_string());

        // The end: nothing new, and nothing lost.
        let end = f.dropped + f.held.len() as u64;
        let (from, lost) = f.since(end);
        assert!(!lost);
        assert_eq!(
            from,
            f.held.len(),
            "a cursor at the end reads an empty slice"
        );
    }

    /// **An attachment made again starts a new life** (`crate::rejoin`):
    /// a face that had read everything is told it `lost` frames — its
    /// way back is history on the new attachment — and only the new
    /// life's frames are handed out, never the old ones twice.
    #[test]
    fn a_rejoined_conversation_tells_every_face_it_lost_frames() {
        let mut f = Frames::default();
        for n in 0..10u64 {
            f.push(numbered(n));
        }
        let caught_up = f.dropped + f.held.len() as u64;
        f.restart();
        let (from, lost) = f.since(caught_up);
        assert!(lost, "a face that kept up is told to rebuild");
        assert_eq!(from, 0);
        assert!(f.held.is_empty(), "nothing of the old life is held");
        f.push(numbered(100));
        let (from, lost) = f.since(caught_up);
        assert!(lost);
        assert_eq!(
            stop_of(&f.held[from]),
            "100",
            "and it gets the new life's frames"
        );
        let (_, lost) = f.since(0);
        assert!(lost, "a face from the very start rebuilds too");
    }

    /// Below the cap nothing moves — so the test above is measuring the
    /// trim rather than something the type does all the time.
    #[test]
    fn an_untrimmed_list_drops_nothing() {
        let mut f = Frames::default();
        for n in 0..10u64 {
            f.push(numbered(n));
        }
        assert_eq!(f.dropped, 0);
        let (from, lost) = f.since(4);
        assert!(!lost);
        assert_eq!(stop_of(&f.held[from]), "4");
    }
}

#[cfg(test)]
mod old_host_tests {
    use super::*;
    use serde::Serialize;

    /// **The host's note as the binary before the redesign wrote it**
    /// (`khor_node::gui_host::GuiNote` at ee144e3), spelled out here so
    /// the wire stays held when the live enum moves. The owner's running
    /// sessions are hosted by that binary (`khor-gui _host`), and the new
    /// face attaches to them: every frame below is one such a host sends.
    #[derive(Serialize)]
    #[allow(dead_code)]
    enum OldNote {
        Note(String),
        Ask {
            ask: u64,
            title: String,
            options: Vec<(String, String)>,
        },
        Turn(String),
        Gone,
        History(String),
        HistoryEnd,
        Turning,
        Answered {
            ask: u64,
            option: Option<String>,
        },
        Agent {
            name: Option<String>,
            replays: bool,
            images: bool,
            embeds: bool,
            lists: bool,
            needs_login: bool,
        },
        Refused {
            why: String,
        },
        Settings(String),
    }

    /// Through the real codec, as bytes: length-prefixed MessagePack.
    fn over_the_wire(old: &OldNote) -> ChatFrame {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, old).expect("encode");
        let note: GuiNote =
            read_frame(&mut bytes.as_slice()).expect("the new reader takes the old frame");
        frame_of(note)
    }

    /// **An old host's frames parse**: its settings carry no `model` (the
    /// redesign added it), it sends no stored usage on attach, and its
    /// handshake and settings frames are the old shapes. Each arrives as
    /// the frame the face expects, with nothing invented.
    #[test]
    fn an_old_hosts_frames_parse() {
        let agent = over_the_wire(&OldNote::Agent {
            name: Some("claude".into()),
            replays: true,
            images: true,
            embeds: true,
            lists: false,
            needs_login: false,
        });
        assert!(
            matches!(
                agent,
                ChatFrame::Agent {
                    replays: true,
                    images: true,
                    ..
                }
            ),
            "{agent:?}"
        );

        // The old `settings_json`: modes and options, nothing else.
        let old_settings = serde_json::json!({
            "modes": {
                "currentModeId": "default",
                "availableModes": [
                    { "id": "default", "name": "Default" },
                    { "id": "acceptEdits", "name": "Accept Edits" },
                    { "id": "plan", "name": "Plan" }
                ]
            },
            "options": []
        })
        .to_string();
        match over_the_wire(&OldNote::Settings(old_settings)) {
            ChatFrame::Settings { settings } => {
                assert_eq!(settings["modes"]["currentModeId"], "default");
                assert!(
                    settings.get("model").is_none(),
                    "an old host names no model: {settings}"
                );
            }
            other => panic!("not a settings frame: {other:?}"),
        }

        let said = serde_json::json!({
            "sessionId": "s",
            "update": { "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "hello" } }
        })
        .to_string();
        assert!(matches!(
            over_the_wire(&OldNote::Note(said.clone())),
            ChatFrame::Note { .. }
        ));
        assert!(matches!(
            over_the_wire(&OldNote::History(said)),
            ChatFrame::History { .. }
        ));
        assert!(matches!(
            over_the_wire(&OldNote::HistoryEnd),
            ChatFrame::HistoryEnd
        ));
        assert!(matches!(
            over_the_wire(&OldNote::Turning),
            ChatFrame::Turning
        ));
        assert!(matches!(
            over_the_wire(&OldNote::Turn("end_turn".into())),
            ChatFrame::Turn { .. }
        ));
        assert!(matches!(
            over_the_wire(&OldNote::Ask {
                ask: 1,
                title: "Run".into(),
                options: vec![("y".into(), "Yes".into())]
            }),
            ChatFrame::Ask { ask: 1, .. }
        ));
        assert!(matches!(
            over_the_wire(&OldNote::Answered {
                ask: 1,
                option: Some("y".into())
            }),
            ChatFrame::Answered { ask: 1, .. }
        ));
        assert!(matches!(
            over_the_wire(&OldNote::Refused {
                why: "no-turn".into()
            }),
            ChatFrame::Refused { .. }
        ));
        assert!(matches!(over_the_wire(&OldNote::Gone), ChatFrame::Gone));
    }
}

#[cfg(test)]
#[path = "chat_io_tests.rs"]
mod io_tests;

#[cfg(test)]
mod wait_tests {
    use super::*;

    /// **A held wait answers when the frame lands, not on a beat.**
    #[tokio::test]
    async fn a_wait_answers_when_a_frame_arrives() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let chat = Arc::new(Chat {
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            turns: Default::default(),
            sends: Default::default(),
            frames: Mutex::new(Frames::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            far: None,
            arrived: tokio::sync::Notify::new(),
            sending: (AtomicU64::new(0), AtomicU64::new(0)),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
        });
        let id = format!("chat-wait-{}", std::process::id());
        chats().lock().unwrap().insert(id.clone(), chat.clone());

        let mut lags = Vec::new();
        for n in 0..20u64 {
            let feeder = chat.clone();
            let arrives = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                let at = std::time::Instant::now();
                feeder.frames.lock().unwrap().push(ChatFrame::Turning);
                feeder.arrived.notify_waiters();
                at
            });
            let batch = chat_wait(&id, n, std::time::Duration::from_secs(2))
                .await
                .unwrap();
            let answered = std::time::Instant::now();
            let at = arrives.join().unwrap();
            assert_eq!((batch.frames.len(), batch.next), (1, n + 1));
            lags.push(answered.duration_since(at));
        }
        lags.sort();
        let median = lags[lags.len() / 2];
        eprintln!(
            "[chat-wait] arrival -> answer median {median:?}, worst {:?} (the beat it replaces: 0-50 ms in a turn, 0-500 between)",
            lags[lags.len() - 1]
        );
        assert!(median < std::time::Duration::from_millis(5), "{lags:?}");

        // Nothing new: it waits, then answers with nothing.
        let began = std::time::Instant::now();
        let batch = chat_wait(&id, 20, std::time::Duration::from_millis(200))
            .await
            .unwrap();
        assert!(
            batch.frames.is_empty() && began.elapsed() >= std::time::Duration::from_millis(190)
        );
        chats().lock().unwrap().remove(&id);
    }
}

// ── files for an agent conversation (round CHAT) ────────────────────
//
// **One rule for where a file goes**: the machine the conversation runs
// on keeps what it is handed in its receive directory (`~/khor-drop`
// unless set, `Node::receive_dir`) under its own name — the very place
// an accepted `send` lands (`khor_node::road`), so a file handed to an
// agent and a file sent to that machine are found in one place, and the
// same bytes handed twice are one file (found again by digest). The agent
// is told the path (that is what Claude Code and Codex read), never the
// bytes.
//
// A conversation running here keeps the file here; one running on
// another machine has it delivered there first (`send`, the file road's
// direct mode, waited out by `term::deliver_blocking`). Either way a
// directory the person chose on that machine (`into`) takes the place of
// the receive directory, and the path the agent gets is where it landed.

/// Files that are already on this machine (a picker's or a drop's
/// paths), put where the conversation's machine keeps them, or into the
/// directory `into` names on it (empty: its receive directory). Answers
/// each one's path **on that machine**, in order.
pub fn chat_put_paths(
    root: &Path,
    id: &str,
    paths: &[String],
    into: &str,
) -> Result<Vec<String>, String> {
    let n = Node::open(root.to_path_buf())?;
    if let Some(machine) = n.far_machine(&SessionId(id.to_owned()))? {
        return crate::term::deliver_blocking(root, &machine, paths, into);
    }
    let into = khor_node::road::tidy_setting(into)?;
    let dir = if into.is_empty() {
        n.receive_dir()?
    } else {
        khor_node::drops::named_dir(&khor_node::road::expand(root, &into).display().to_string())?
    };
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let src = Path::new(p);
        let (digest, size) = khor_node::transfer::digest_file(src)?;
        // The same bytes are the same file: kept, not copied again.
        if let Some(r) = khor_node::road::receipt(root, &digest)
            .filter(|r| r.path.parent() == Some(dir.as_path()))
        {
            out.push(r.path.display().to_string());
            continue;
        }
        let name = khor_node::transfer::safe_name(
            src.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        );
        let at = khor_node::drops::landing(&dir, &name)?;
        let copied = std::fs::copy(src, &at.part)
            .map_err(|e| khor_catalog::msg::cant_read(src.display(), e))
            .and_then(|_| std::fs::rename(&at.part, &at.dest).map_err(|e| e.to_string()));
        if let Err(why) = copied {
            let _ = std::fs::remove_file(&at.part);
            return Err(why);
        }
        khor_node::road::record(
            root,
            &khor_node::road::Receipt {
                digest,
                name: name.clone(),
                path: at.dest.clone(),
                size,
                from: n.device_str().to_owned(),
                at_ms: khor_node::road::now_ms(),
                mode: khor_node::road::Mode::Direct,
            },
        )?;
        out.push(at.dest.display().to_string());
    }
    Ok(out)
}

/// Bytes from a face (a phone's photo, a browser's pick), named `name`:
/// staged on this machine (`save_upload`, with its cap), then put where
/// the conversation's machine keeps files (`chat_put_paths`). Answers the
/// path on that machine. The staging copy goes once the file is placed:
/// unlike a `send` offer, nothing reads it afterwards.
pub fn chat_put(
    root: &Path,
    id: &str,
    name: &str,
    into: &str,
    body: &mut dyn std::io::Read,
) -> Result<String, String> {
    let staged = crate::save_upload(root, name, body)?;
    // Placed under its own name, not the staging one's stamp.
    let named = staged.with_file_name(khor_node::transfer::safe_name(name));
    let named = if named.exists() {
        staged.clone()
    } else {
        std::fs::rename(&staged, &named).map_err(|e| e.to_string())?;
        named
    };
    // Placed here by a copy, or there by a delivery that has finished
    // fetching when it returns (`deliver_blocking` accepts before it
    // answers): either way the staged bytes are read by nobody now.
    let out = chat_put_paths(root, id, &[named.display().to_string()], into);
    let _ = std::fs::remove_file(&named);
    out?.into_iter()
        .next()
        .ok_or_else(|| "nothing placed".to_owned())
}

#[cfg(test)]
mod put_tests {
    use super::*;

    /// **The one folder rule** (round CHAT): a file this machine already
    /// has, put for a conversation that runs here, lands in this
    /// machine's receive directory under its own name, where an accepted
    /// `send` lands, and the same bytes put twice are one file.
    #[test]
    fn a_local_file_lands_in_the_receive_directory_once() {
        let root = std::env::temp_dir().join(format!("khor-chat-put-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("notes.md");
        std::fs::write(&src, "# notes\n").unwrap();
        let given = [src.display().to_string()];

        let first = chat_put_paths(&root, "gui/nowhere", &given, "").expect("placed");
        let again = chat_put_paths(&root, "gui/nowhere", &given, "").expect("placed again");

        let want = root.join("khor-drop").join("notes.md");
        assert_eq!(first, vec![want.display().to_string()]);
        assert_eq!(again, first);
        assert_eq!(std::fs::read_to_string(&want).unwrap(), "# notes\n");
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod bounded_history_response_tests {
    use super::*;
    use std::io::Write;

    struct RecordFixture(std::path::PathBuf);
    impl RecordFixture {
        fn new(name: &str, text: &[u8]) -> Self {
            let path =
                std::env::temp_dir().join(format!("khor-history-{name}-{}", std::process::id()));
            std::fs::write(&path, text).unwrap();
            Self(path)
        }
        fn page(
            &self,
            cursor: Option<&str>,
            parse: fn(&str) -> Vec<khor_node::adaptor::claude::Utterance>,
        ) -> HistoryPage {
            let cursor = cursor.map(serde_json::from_str).transpose().unwrap();
            let mut file = std::fs::File::open(&self.0).unwrap();
            let page = khor_node::adaptor::transcript_page::read_page(
                &mut file,
                "session",
                cursor.as_ref(),
                parse,
            )
            .unwrap();
            history_page_response("session", page).unwrap()
        }
    }
    impl Drop for RecordFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn serialized_budget_retains_a_cursor_before_the_unadmitted_event() {
        // Exercise response accounting independently of the vendor parser's
        // current expansion ratio; a future parser must not bypass the cap.
        fn expand(line: &str) -> Vec<khor_node::adaptor::claude::Utterance> {
            vec![khor_node::adaptor::claude::Utterance::Agent(
                line.trim().repeat(200 * 1024),
            )]
        }
        let fixture = RecordFixture::new("wire-budget", b"a\nb\nc\n");
        let tail = fixture.page(None, expand);
        assert_eq!(tail.entries.len(), 2);
        assert!(tail.encoded_bytes <= HISTORY_PAYLOAD_BYTES);
        assert_eq!(tail.encoded_bytes, serde_json::to_vec(&tail).unwrap().len());
        assert_eq!(
            tail.entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            ["2:0", "4:0"]
        );
        let older = fixture.page(tail.older.as_deref(), expand);
        assert_eq!(older.entries.len(), 1);
        assert_eq!(older.entries[0].key, "0:0");
        assert!(older.older.is_none());
    }

    #[test]
    fn over_budget_expanded_event_is_an_identity_bound_reference() {
        fn expand(_: &str) -> Vec<khor_node::adaptor::claude::Utterance> {
            vec![khor_node::adaptor::claude::Utterance::Agent(
                "x".repeat(HISTORY_PAYLOAD_BYTES),
            )]
        }
        let fixture = RecordFixture::new("expanded-reference", b"a\n");
        let page = fixture.page(None, expand);
        assert!(page.entries.is_empty());
        assert_eq!(page.references.len(), 1);
        assert_eq!(page.references[0].item, Some(0));
        assert!(!page.references[0].cursor.is_empty());
        assert!(page.encoded_bytes < HISTORY_PAYLOAD_BYTES);
    }

    #[test]
    fn giant_raw_record_is_never_offered_to_the_inline_parser() {
        fn forbidden(_: &str) -> Vec<khor_node::adaptor::claude::Utterance> {
            panic!("oversized record parsed inline")
        }
        let mut text = vec![b'x'; 128 * 1024];
        text.write_all(b"\n").unwrap();
        let fixture = RecordFixture::new("raw-reference", &text);
        let page = fixture.page(None, forbidden);
        assert!(page.entries.is_empty());
        assert_eq!(page.references.len(), 1);
        assert!(!page.references[0].missing_start);
        assert!(!page.references[0].missing_end);
    }
}
