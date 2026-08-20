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
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use khor_node::gui_host::{GuiNote, GuiOp};
use khor_node::host::{connect, read_frame, write_frame};
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
    Agent { name: Option<String>, replays: bool },
    /// An op this face sent that did not take effect
    /// (`khor_node::gui_host::GuiNote::Refused`), and why — as a
    /// **key**, looked up at the last moment like every other word.
    /// Sent to the face that sent the op and to nobody else.
    Refused { why: String },
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
    conn: Mutex<TcpStream>,
    frames: Mutex<Frames>,
    gone: AtomicBool,
    /// How many opens hold this attachment. React mounts an effect,
    /// cleans it up, and mounts again — two holders of one id, in an
    /// order the wire does not preserve; without the count, whichever
    /// leave lands last closes the survivor's connection out from
    /// under it (seen live: the pane polled a registry its own first
    /// mount had emptied).
    holds: AtomicUsize,
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
        GuiNote::Ask { ask, title, options } => ChatFrame::Ask { ask, title, options },
        GuiNote::Answered { ask, option } => ChatFrame::Answered { ask, option },
        GuiNote::Turning => ChatFrame::Turning,
        GuiNote::Turn(stop) => ChatFrame::Turn { stop },
        GuiNote::Gone => ChatFrame::Gone,
        GuiNote::Agent { name, replays } => ChatFrame::Agent { name, replays },
        GuiNote::Refused { why } => ChatFrame::Refused { why },
    }
}

/// Attaches to the session's host, idempotently: an already-open chat
/// stays as it is (its frames and cursor survive a second open), a
/// dead one is replaced by a fresh attachment. Answers whether this
/// call attached anew — the caller that asks for history on open must
/// not ask twice for one attachment (React mounts effects twice in
/// dev, and a host answers every replay it is asked for).
pub fn chat_open(root: &Path, id: &str) -> Result<bool, String> {
    let mut registry = chats().lock().unwrap();
    if let Some(chat) = registry.get(id) {
        if !chat.gone.load(Ordering::Relaxed) {
            chat.holds.fetch_add(1, Ordering::Relaxed);
            return Ok(false);
        }
        registry.remove(id);
    }
    let n = Node::open(root.to_path_buf())?;
    let dir = n
        .session_dir(&SessionId(id.to_owned()))
        .ok_or_else(|| khor_catalog::msg::no_such_session(id))?;
    // The GUI host ignores the terminal size; 0×0 is what its own test
    // clients send.
    let conn = connect(&dir, 0, 0)?;
    let mut reading = conn.try_clone().map_err(|e| e.to_string())?;
    let chat = Arc::new(Chat {
        conn: Mutex::new(conn),
        frames: Mutex::new(Frames::default()),
        gone: AtomicBool::new(false),
        holds: AtomicUsize::new(1),
    });
    let reader = chat.clone();
    std::thread::spawn(move || {
        while let Ok(note) = read_frame::<GuiNote>(&mut reading) {
            let over = matches!(note, GuiNote::Gone);
            reader.frames.lock().unwrap().push(frame_of(note));
            if over {
                break;
            }
        }
        // A goodbye and a torn connection land the same way (ChatBatch).
        reader.gone.store(true, Ordering::Relaxed);
    });
    registry.insert(id.to_owned(), chat);
    Ok(true)
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
    })
}

/// One user turn.
pub fn chat_say(id: &str, text: &str) -> Result<(), String> {
    let chat = chat_of(id)?;
    let mut conn = chat.conn.lock().unwrap();
    write_frame(&mut *conn, &GuiOp::Say(text.to_owned()))
}

/// The answer to an [`ChatFrame::Ask`]: the chosen option id, or `None`
/// to refuse.
pub fn chat_answer(id: &str, ask: u64, option: Option<String>) -> Result<(), String> {
    let chat = chat_of(id)?;
    let mut conn = chat.conn.lock().unwrap();
    write_frame(&mut *conn, &GuiOp::Answer { ask, option })
}

/// Ends the running turn — not the session (`GuiOp::Stop`).
pub fn chat_stop(id: &str) -> Result<(), String> {
    let chat = chat_of(id)?;
    let mut conn = chat.conn.lock().unwrap();
    write_frame(&mut *conn, &GuiOp::Stop)
}

/// Asks for the conversation so far; it arrives as
/// [`ChatFrame::History`] frames closed by [`ChatFrame::HistoryEnd`].
pub fn chat_replay(id: &str) -> Result<(), String> {
    let chat = chat_of(id)?;
    let mut conn = chat.conn.lock().unwrap();
    write_frame(&mut *conn, &GuiOp::Replay)
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
        .map(|u| {
            use khor_node::adaptor::claude::Utterance;
            let update = match u {
                Utterance::User(text) => serde_json::json!({
                    "sessionUpdate": "user_message_chunk",
                    "content": { "type": "text", "text": text },
                }),
                Utterance::Agent(text) => serde_json::json!({
                    "sessionUpdate": "agent_message_chunk",
                    "content": { "type": "text", "text": text },
                }),
                Utterance::Thought(text) => serde_json::json!({
                    "sessionUpdate": "agent_thought_chunk",
                    "content": { "type": "text", "text": text },
                }),
                Utterance::Tool(name) => serde_json::json!({
                    "sessionUpdate": "tool_call",
                    "title": name,
                }),
            };
            ChatFrame::History { update: serde_json::json!({ "sessionId": id, "update": update }) }
        })
        .collect();
    frames.push(ChatFrame::HistoryEnd);
    Ok(frames)
}

/// Detaches one holder; the socket closes when the last one leaves
/// (`Chat::holds`). **Not a close**: the session and its agent live
/// on, and closing is [`crate::close_session`]'s job — a detail view
/// going away must not take the conversation with it.
pub fn chat_leave(id: &str) -> Result<(), String> {
    let mut registry = chats().lock().unwrap();
    if let Some(chat) = registry.get(id) {
        if chat.holds.fetch_sub(1, Ordering::Relaxed) <= 1 {
            registry.remove(id);
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
        ChatFrame::Turn { stop: n.to_string() }
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
        assert_eq!(f.dropped, (frame_cap() + 1 - frame_keep()) as u64, "one trim, one bite");
        assert_eq!(f.held.len(), frame_keep());

        // A reader at the very front: what it missed is gone, and it is
        // told so rather than handed the tail as if it were the whole.
        let (from, lost) = f.since(0);
        assert!(lost, "a cursor older than the trim must be told it lost frames");
        assert_eq!(stop_of(&f.held[from]), f.dropped.to_string(), "and gets the oldest still held");

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
        assert_eq!(from, f.held.len(), "a cursor at the end reads an empty slice");
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
