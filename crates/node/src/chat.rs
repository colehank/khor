//! The device-chat kind: a machine's window, ridden on the chat CRDT.
//! Which machines exist — and therefore which channels — is the device
//! table's business; this type takes resolved (channel, home) pairs.
//! Read state lives in the seen CRDT the node owns; this type takes the
//! watermark and answers with rows.

use std::fs;
use std::path::PathBuf;

use khor_catalog::msg;
use khor_core::{kind, DeviceId, Event, Kind, Millis, Session, SessionId, State, StateStamp};
use khor_sync::chat::{channel_dir, open_channel, ChatDoc, Message, Sender};
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
        channel_dir(&self.root, channel).ok_or_else(|| msg::bad_channel_name(format_args!("{channel:?}")))
    }

    fn load(&self, channel: &str) -> Result<Loaded<ChatDoc>, String> {
        open_channel(&self.dir(channel)?, self.peer)
    }

    fn session_id(channel: &str) -> SessionId {
        SessionId(format!("chat/{channel}"))
    }

    fn row_from(&self, channel: &str, home: DeviceId, doc: &ChatDoc, watermark: i64) -> Session {
        let msgs = doc.messages();
        let unread = msgs.iter().filter(|m| m.at > watermark).count() as u64;
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
            // Khor's own, not a vendor's: a device chat is a thing this
            // product makes, and there is no agent behind it to name.
            category: Some(khor_core::category::KHOR.to_owned()),
        }
    }

    pub fn row(&self, channel: &str, home: DeviceId, watermark: i64) -> Result<Session, String> {
        Ok(self.row_from(channel, home, &self.load(channel)?.doc, watermark))
    }

    pub fn tell(&self, channel: &str, home: DeviceId, text: &str) -> Result<Told, String> {
        self.push(channel, home, |doc| doc.tell(&self.me, text), serde_json::json!({ "text": text, "from": self.me.name }))
    }

    /// Sends file summaries — names, sizes, digests. The bytes never
    /// enter the document; they travel a transfer after approval.
    pub fn send_files(
        &self,
        channel: &str,
        home: DeviceId,
        files: &[khor_sync::chat::FileRef],
    ) -> Result<Told, String> {
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        self.push(channel, home, |doc| doc.send_files(&self.me, files), serde_json::json!({ "files": names, "from": self.me.name }))
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
        let row = self.row_from(channel, home, &loaded.doc, at);
        let event = Event {
            session: row.id.clone(),
            seq: msgs.len() as u64,
            at: row.state.at,
            payload: serde_json::to_vec(&payload).map_err(|e| e.to_string())?,
        };
        Ok(Told { msg_id, at, row, event })
    }

    pub fn log(&self, channel: &str) -> Result<ChannelLog, String> {
        let loaded = self.load(channel)?;
        Ok(ChannelLog {
            messages: loaded.doc.messages(),
            broken: loaded.broken.len(),
        })
    }

    /// Deletes what the conversation received — the `files/` payloads —
    /// and nothing else. The history blocks stay: they replicate
    /// network-wide, so a local delete would only be pulled right back on
    /// the next sync. Slimming history is an explicit cleanup, not close
    /// (docs/SESSION.md 动作, docs/NET.md 同步).
    pub fn close_channel(&self, channel: &str) -> Result<(), String> {
        let files = self.dir(channel)?.join("files");
        if files.exists() {
            fs::remove_dir_all(&files).map_err(msg::cant_delete)?;
        }
        Ok(())
    }
}
