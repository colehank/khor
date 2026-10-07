//! Khor-wide sessions (STORE-BRIEF foundation B): a session that belongs
//! to the whole khor, not to one machine.
//!
//! **The broadcast** (`wide/all`, phase 3) is the first: the group
//! conversation of every machine. Its document is a conversation
//! (`khor_sync::chat::ChatDoc`) every machine keeps and syncs under the
//! wire name `wide/all`, so a message spreads machine to machine at each
//! round and outlives its sender going offline. Every machine derives
//! the row from its own copy (`WideKind`), the way a machine's window is
//! derived — nothing about it travels as a report.
//!
//! **A file in it is an offer.** Each machine answers it on its own: it
//! accepts (the transfer's `take`, here), then pulls the bytes from the
//! sender into its own receive location, writing how that stands under
//! its own key on the message (`ChatDoc::set_fate_of`). So every device
//! reads every machine's state, and the sender counts the machines that
//! have it. The pull asks the sender by digest; the road is
//! content-addressed, so a later step can ask any machine that already
//! holds the digest instead (`road::Source::Offer`'s `from`).
//!
//! **The row is this machine's seat.** An offer this machine has not
//! answered is 待批 — its yes is the only way on — one it is pulling is
//! 忙碌, one that broke is 中断 (a retry resumes); otherwise unseen
//! messages make 完成 and the rest 空闲. It stands on every machine as
//! soon as the khor has two machines that run khor (a satellite has no
//! khor to answer an offer with), with nothing to create first.
//!
//! **How the khor agent fits** (phase 4, not built): another board,
//! `wide/<name>`, the same document with its head naming the machine
//! that runs it (`ChatDoc::home`). Its row is derived the same way on
//! every machine; what differs is that the agent's answers come from its
//! home, and a move of home is a write to the head.

use std::path::Path;

use khor_catalog::msg;
use khor_core::{Kind, Millis, Session, SessionId, State, StateStamp, kind};
use khor_sync::chat::{Fate, Message, MsgBody, Step};
use khor_sync::devices::DeviceInfo;

use crate::chat::ChannelLog;
use crate::transfer::TransferKind;
use crate::{KindSurface, Node, NodeEvent};

/// The broadcast's board, which is also its session id and the name its
/// document syncs under.
pub const BROADCAST: &str = "wide/all";

/// The prefix every khor-wide board and session id carries.
pub use khor_sync::chat::WIDE;

/// The board a caller named — refused unless this build keeps one by
/// that name (only the broadcast, so far). A typo must not open a new
/// khor-wide conversation that every machine then syncs.
pub(crate) fn board(name: &str) -> Result<&'static str, String> {
    if name == BROADCAST {
        Ok(BROADCAST)
    } else {
        Err(msg::bad_channel_name(format_args!("{name:?}")))
    }
}

/// One broadcast file, as this machine reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WideFace {
    /// This machine's own answer to the offer; `None` while it has not
    /// answered, and on the machine that sent it.
    pub here: Option<Fate>,
    /// The machines that have it (their fate is landed) …
    pub got: u32,
    /// … out of the machines it was offered to: every member but its
    /// sender.
    pub of: u32,
}

/// A khor-wide conversation's readout, each message with its face —
/// `None` for anything that is not a file.
pub type WideLog = (ChannelLog, Vec<Option<WideFace>>);

/// The face of one file message, read against the members.
fn face_of(m: &Message, me: &str, members: &[DeviceInfo]) -> Option<WideFace> {
    if !matches!(m.body, MsgBody::Files(_)) || m.retracted {
        return None;
    }
    let to: Vec<&DeviceInfo> = members.iter().filter(|d| d.id != m.from.id).collect();
    Some(WideFace {
        here: (m.from.id != me).then(|| m.fates.get(me).cloned()).flatten(),
        got: to
            .iter()
            .filter(|d| m.fates.get(&d.id).is_some_and(|f| f.step == Step::Landed))
            .count() as u32,
        of: to.len() as u32,
    })
}

impl Node {
    /// The machines a broadcast reaches: every machine that runs khor.
    /// A satellite is left out — it has no khor of its own to answer an
    /// offer with, and counting it would leave every count one short
    /// forever.
    pub fn wide_members(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self
            .devices()?
            .into_iter()
            .filter(|d| d.sponsor.is_none())
            .collect())
    }

    /// Tells a khor-wide conversation a line. Telling is looking, as in
    /// a machine's window (`Node::tell`).
    pub fn wide_tell(&self, name: &str, text: &str) -> Result<String, String> {
        let board = board(name)?;
        let told = self.chat.tell(board, self.device(), text)?;
        self.mark_seen(&told.row.id, told.at)?;
        self.emit(NodeEvent::Event(told.event));
        let _ = self.emit_row_of(&told.row.id);
        Ok(told.msg_id)
    }

    /// Offers a file to every machine: the summary travels the document,
    /// the bytes wait here until a machine accepts and pulls them.
    /// Offering is not looking (`Node::send`'s note), so no watermark
    /// moves. Returns the transfer session id.
    pub fn wide_send(&self, name: &str, path: &Path) -> Result<SessionId, String> {
        let board = board(name)?;
        let f = self.offer_file(path)?;
        let told = self.chat.send_files(board, self.device(), &[f], "")?;
        self.emit(NodeEvent::Event(told.event));
        let _ = self.emit_row_of(&told.row.id);
        Ok(TransferKind::session_id(&told.msg_id))
    }

    /// The readout with each file's face, from one read of the document.
    pub fn wide_log(&self, name: &str) -> Result<WideLog, String> {
        let board = board(name)?;
        let log = self.chat.log(board)?;
        let members = self.wide_members()?;
        let faces = log
            .messages
            .iter()
            .map(|m| face_of(m, self.device_str(), &members))
            .collect();
        Ok((log, faces))
    }
}

/// The khor-wide rows (module head).
pub(crate) struct WideKind;

impl KindSurface for WideKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        id.0 == BROADCAST
    }

    fn rows(&self, node: &Node) -> Result<Vec<Session>, String> {
        if node.wide_members()?.len() < 2 {
            return Ok(Vec::new());
        }
        let seen = node.seen_loaded()?;
        let watermark = seen.doc.watermark(BROADCAST);
        let msgs = node.chat.log(BROADCAST)?.messages;
        let mut row: Session = node.chat.row_from(BROADCAST, node.device(), &msgs, watermark);
        row.kind = Kind(kind::BROADCAST.to_owned());
        row.title = msg::broadcast_title().to_owned();
        // This machine's seat on each offer from another machine: the
        // most pressing word wins, and among offers waiting for the same
        // word the oldest says how long it has waited.
        let me = node.device_str();
        let mut pressed: Option<(State, i64)> = None;
        for m in &msgs {
            if m.retracted || m.from.id == me || !matches!(m.body, MsgBody::Files(_)) {
                continue;
            }
            let word = match m.fates.get(me).map(|f| f.step) {
                None => State::Blocked,
                Some(Step::Moving) => State::Busy,
                Some(Step::Failed) => State::Errored,
                Some(Step::Landed | Step::Stopped) => continue,
            };
            if pressed.is_none_or(|(w, at)| word.rank() < w.rank() || (word == w && m.at < at)) {
                pressed = Some((word, m.at));
            }
        }
        if let Some((word, at)) = pressed
            && word.rank() < row.state.state.rank()
        {
            row.state = StateStamp {
                state: word,
                at: Millis(at.max(0) as u64),
            };
        }
        // Put away like a machine's window (`ChatKind`'s rows) — but an
        // offer still waiting on this machine keeps it out.
        if pressed.is_none()
            && let Some(closed) = seen.doc.closed_watermark(BROADCAST)
            && msgs.iter().map(|m| m.at).max().unwrap_or(0) <= closed
        {
            return Ok(Vec::new());
        }
        Ok(vec![row])
    }

    /// Every machine derives its own, from its own copy.
    fn reportable(&self) -> bool {
        false
    }

    fn seen_at(&self, node: &Node, _id: &SessionId) -> Result<i64, String> {
        Ok(node
            .chat
            .log(BROADCAST)?
            .messages
            .iter()
            .map(|m| m.at)
            .max()
            .unwrap_or(0))
    }

    /// Put away, never deleted — a machine's window's close
    /// (docs/SESSION.md 动作): the next message brings it back.
    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String> {
        let newest = self.seen_at(node, id)?;
        let loaded = node.seen_loaded()?;
        loaded.doc.mark(&id.0, newest)?;
        loaded.doc.put_away(&id.0, newest)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }
}
