//! The device-chat kind: a machine's window, ridden on the chat CRDT.
//! Which machines exist — and therefore which channels — is the device
//! table's business; this type takes resolved (channel, home) pairs.
//! Read state lives in the seen CRDT the node owns; this type takes the
//! watermark and answers with rows.

use std::path::PathBuf;

use khor_catalog::msg;
use khor_core::{DeviceId, Event, Kind, Millis, Session, SessionId, State, StateStamp, kind};
use khor_sync::chat::{ChatDoc, Message, Sender, channel_dir, open_channel};
use khor_sync::store::Loaded;

/// One channel's readout.
pub struct ChannelLog {
    pub messages: Vec<Message>,
    /// Blocks that would not read — a missing slice of conversation the
    /// caller must be able to mention.
    pub broken: usize,
}

/// What `tell` did. `at` is the new message's timestamp — the caller
/// persists it as the seen watermark (telling implies looking).
///
/// **`send` deliberately does not.** Telling implies looking because a
/// line's content *is* the line; an offer's is a payload that has not
/// arrived anywhere yet, so nobody has looked at anything. The full
/// reasoning is at the one site that drops it, `Node::send`.
pub struct Told {
    pub msg_id: String,
    pub at: i64,
    pub row: Session,
    pub event: Event,
}

pub struct ChatKind {
    root: PathBuf,
    me: Sender,
    /// This process's writer peer (one per live writer — see
    /// `ChatDoc::new`).
    peer: u64,
}

impl ChatKind {
    pub fn new(root: PathBuf, me: Sender) -> ChatKind {
        ChatKind {
            root,
            me,
            peer: ChatDoc::fresh_peer(),
        }
    }

    fn dir(&self, channel: &str) -> Result<PathBuf, String> {
        channel_dir(&self.root, channel)
            .ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))
    }

    fn load(&self, channel: &str) -> Result<Loaded<ChatDoc>, String> {
        open_channel(&self.dir(channel)?, self.peer)
    }

    fn session_id(channel: &str) -> SessionId {
        SessionId(format!("chat/{channel}"))
    }

    /// A file message **this machine offered**. It is not unread here,
    /// and no watermark says so — the watermark is the person's decision
    /// and it replicates, while this is a fact about a seat.
    ///
    /// The sender's seat already refuses to speak the six words about
    /// its own file messages ([`crate::transfer::TransferKind::message_delivery`]
    /// answers `Delivery`, "not a state word", 发端中性词批). Unread is
    /// the same question asked of the row: 完成/未看 on a file message
    /// means *a file arrived here and nobody looked*, and on the seat
    /// that offered it nothing arrived. Same seat test as over there
    /// (`m.from.id` against this device), and it must stay the same one:
    /// two answers to "is this my own offer" is how the chip and the row
    /// come to disagree.
    ///
    /// Text is untouched on purpose — for a line, the watermark already
    /// does this job and does it person-wide, which is the tested rule
    /// (`tests/pairing.rs`: own words never count as unread, on any
    /// device).
    fn i_offered(&self, m: &Message) -> bool {
        matches!(m.body, khor_sync::chat::MsgBody::Files(_)) && m.from.id == self.me.id
    }

    /// The row a channel's messages make, **taking the messages rather
    /// than the document**: the caller that reads this document on the
    /// list's beat wants the messages too, and one read has to serve
    /// both (`row_and_log`). Handed a document it would re-derive the
    /// message list a second time, which is the cheap half of the same
    /// mistake `row` used to make with the expensive one.
    fn row_from(&self, channel: &str, home: DeviceId, msgs: &[Message], watermark: i64) -> Session {
        let unread = msgs
            .iter()
            .filter(|m| m.at > watermark && !self.i_offered(m))
            .count() as u64;
        let at = msgs.last().map(|m| m.at.max(0) as u64).unwrap_or(0);
        Session {
            id: Self::session_id(channel),
            kind: Kind(kind::CHAT.to_owned()),
            title: channel.to_owned(),
            home,
            // docs/SESSION.md device chat: unseen content = Done, seen =
            // Idle; the first four words are unreachable (no process).
            state: StateStamp {
                state: if unread > 0 { State::Done } else { State::Idle },
                at: Millis(at),
            },
            unread,
            // Never dead: the other machine can always speak again, so
            // there is no state this row cannot move out of on its own.
            dead: false,
            // Always vouched-for: the words here (完成/空闲) are derived
            // from the replicated document itself, which cannot go
            // stale-behind-a-crash the way a process's last report can.
            unbacked: false,
            // Khor's own, not a vendor's: a device chat is a thing this
            // product makes, and there is no agent behind it to name.
            category: Some(khor_core::category::KHOR.to_owned()),
            // The row's preview: the newest message still standing. A
            // retracted one is skipped (its slot stays, its words must
            // not); a kind this version cannot read previews as nothing
            // rather than as blank text (MsgBody::Unknown's rule).
            last: msgs
                .iter()
                .rev()
                .filter(|m| !m.retracted)
                .find_map(|m| match &m.body {
                    khor_sync::chat::MsgBody::Text(t) => Some(khor_core::preview(t)),
                    khor_sync::chat::MsgBody::Files(fs) => Some(khor_core::preview(
                        &fs.iter()
                            .map(|f| f.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                    )),
                    khor_sync::chat::MsgBody::Unknown(_) => None,
                }),
            via: None,
            term: false,
            // **No place, and that is not a gap.** A conversation with
            // another machine does not happen in a directory — nothing
            // was recorded because there was nothing to record, and the
            // list groups these together as such rather than pretending
            // somebody forgot.
            place: None,
            mux: false,
            doing: None,
        }
    }

    /// One channel's row. For the **event** paths — after a message is
    /// told or a file offered — where one channel is read once and the
    /// messages are not wanted afterwards.
    pub fn row(&self, channel: &str, home: DeviceId, watermark: i64) -> Result<Session, String> {
        Ok(self.row_from(
            channel,
            home,
            &self.load(channel)?.doc.messages(),
            watermark,
        ))
    }

    /// The row **and** the messages behind it, from one read of the
    /// document.
    ///
    /// ⚠️ **This is the list's path, and the list is why it exists.**
    /// `ChatKind::rows` used to ask for the log and then for the row,
    /// which is two calls to [`Self::load`] — and a load is not a
    /// lookup: it opens an empty document, reads **every block file** in
    /// the channel's directory, merges each one into it, and writes the
    /// ledger back. Twice per machine, every 2 s, for as long as the app
    /// is open. Measured 2026-09-11 on a real home: `ChatKind::rows` was
    /// 37–53 ms of the session list's 95–131 ms.
    ///
    /// The same argument `log_faced` already makes for a different pair:
    /// one read, one order. Here it is also one read, one cost.
    pub fn row_and_log(
        &self,
        channel: &str,
        home: DeviceId,
        watermark: i64,
    ) -> Result<(Session, ChannelLog), String> {
        let loaded = self.load(channel)?;
        let messages = loaded.doc.messages();
        let row = self.row_from(channel, home, &messages, watermark);
        Ok((
            row,
            ChannelLog {
                messages,
                broken: loaded.broken.len(),
            },
        ))
    }

    pub fn tell(&self, channel: &str, home: DeviceId, text: &str) -> Result<Told, String> {
        self.push(
            channel,
            home,
            |doc| doc.tell(&self.me, text),
            serde_json::json!({ "text": text, "from": self.me.name }),
        )
    }

    /// Sends file summaries — names, sizes, digests, and the directory
    /// on the receiver (`into`, empty for its receive location). The
    /// bytes never enter the document; the receiver pulls them at once.
    pub fn send_files(
        &self,
        channel: &str,
        home: DeviceId,
        files: &[khor_sync::chat::FileRef],
        into: &str,
    ) -> Result<Told, String> {
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        self.push(
            channel,
            home,
            |doc| doc.send_files(&self.me, files, into),
            serde_json::json!({ "files": names, "from": self.me.name }),
        )
    }

    fn push(
        &self,
        channel: &str,
        home: DeviceId,
        write: impl FnOnce(&ChatDoc) -> Result<String, String>,
        payload: serde_json::Value,
    ) -> Result<Told, String> {
        let loaded = self.load(channel)?;
        let mut store = loaded.store;
        let msg_id = write(&loaded.doc)?;
        store.flush(&loaded.doc)?;
        let msgs = loaded.doc.messages();
        // The watermark is this message's own `at` — not the list max: a
        // merged-in line stamped ahead of us must not get auto-seen.
        let at = msgs
            .iter()
            .find(|m| m.id == msg_id)
            .map(|m| m.at)
            .unwrap_or(0);
        let row = self.row_from(channel, home, &msgs, at);
        let event = Event {
            session: row.id.clone(),
            seq: msgs.len() as u64,
            at: row.state.at,
            payload: serde_json::to_vec(&payload).map_err(|e| e.to_string())?,
        };
        Ok(Told {
            msg_id,
            at,
            row,
            event,
        })
    }

    /// Writes how a file message's move stands (its receiver only).
    pub fn set_fate(&self, channel: &str, msg_id: &str, fate: &khor_sync::chat::Fate) -> Result<(), String> {
        let loaded = self.load(channel)?;
        let mut store = loaded.store;
        loaded.doc.set_fate(msg_id, fate)?;
        store.flush(&loaded.doc).map(|_| ())
    }

    pub fn log(&self, channel: &str) -> Result<ChannelLog, String> {
        let loaded = self.load(channel)?;
        Ok(ChannelLog {
            messages: loaded.doc.messages(),
            broken: loaded.broken.len(),
        })
    }
}
