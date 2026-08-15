//! The device-chat kind: a machine's window, ridden on the chat CRDT.
//! Which machines exist — and therefore which channels — is the device
//! table's business; this type takes resolved (channel, home) pairs.

use std::fs;
use std::path::PathBuf;

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
        channel_dir(&self.root, channel).ok_or_else(|| format!("频道名不合法: {channel:?}"))
    }

    fn load(&self, channel: &str) -> Result<Loaded<ChatDoc>, String> {
        open_channel(&self.dir(channel)?, self.peer)
    }

    fn session_id(channel: &str) -> SessionId {
        SessionId(format!("chat/{channel}"))
    }

    fn row_from(&self, channel: &str, home: DeviceId, doc: &ChatDoc) -> Session {
        let msgs = doc.messages();
        let total = msgs.len() as u64;
        let unread = total.saturating_sub(self.seen_count(channel));
        let at = msgs.last().map(|m| m.at.max(0) as u64).unwrap_or(0);
        Session {
            id: Self::session_id(channel),
            kind: Kind(kind::CHAT.to_owned()),
            title: channel.to_owned(),
            home,
            // docs/SESSION.md 对话·对设备: unseen content = Done, seen =
            // Idle; the first four words are unreachable (no process).
            state: StateStamp {
                state: if unread > 0 { State::Done } else { State::Idle },
                at: Millis(at),
            },
            unread,
        }
    }

    pub fn row(&self, channel: &str, home: DeviceId) -> Result<Session, String> {
        Ok(self.row_from(channel, home, &self.load(channel)?.doc))
    }

    pub fn tell(
        &self,
        channel: &str,
        home: DeviceId,
        text: &str,
    ) -> Result<(String, Session, Event), String> {
        let loaded = self.load(channel)?;
        let mut store = loaded.store;
        let msg_id = loaded.doc.tell(&self.me, text)?;
        store.flush(&loaded.doc)?;
        let total = loaded.doc.messages().len() as u64;
        // Telling implies looking at the channel: own words never count
        // as unread.
        self.write_seen(channel, total)?;
        let row = self.row_from(channel, home, &loaded.doc);
        let event = Event {
            session: row.id.clone(),
            seq: total,
            at: row.state.at,
            payload: serde_json::to_vec(&serde_json::json!({
                "text": text,
                "from": self.me.name,
            }))
            .map_err(|e| e.to_string())?,
        };
        Ok((msg_id, row, event))
    }

    pub fn log(&self, channel: &str) -> Result<ChannelLog, String> {
        let loaded = self.load(channel)?;
        Ok(ChannelLog {
            messages: loaded.doc.messages(),
            broken: loaded.broken.len(),
        })
    }

    pub fn seen(&self, channel: &str, home: DeviceId) -> Result<Session, String> {
        let loaded = self.load(channel)?;
        self.write_seen(channel, loaded.doc.messages().len() as u64)?;
        Ok(self.row_from(channel, home, &loaded.doc))
    }

    /// Deletes the conversation and everything it received — docs的判词:
    /// 删对设备的对话,连它收下的文件一起删。
    pub fn close(&self, channel: &str) -> Result<(), String> {
        let dir = self.dir(channel)?;
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| format!("删不掉: {e}"))?;
        }
        Ok(())
    }

    // The seen marker is a machine-local dotfile in the channel dir: no
    // `.loro` extension, so it never syncs (same trick as the merge
    // ledger). Cross-device clearing arrives with the read-state CRDT;
    // until then unread clears only where you cleared it.
    fn seen_path(&self, channel: &str) -> Result<PathBuf, String> {
        Ok(self.dir(channel)?.join(".seen"))
    }

    fn seen_count(&self, channel: &str) -> u64 {
        self.seen_path(channel)
            .ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    fn write_seen(&self, channel: &str, n: u64) -> Result<(), String> {
        // An empty channel leaves nothing on disk; tell() creates the dir
        // via flush before this runs.
        if !self.dir(channel)?.exists() {
            return Ok(());
        }
        fs::write(self.seen_path(channel)?, format!("{n}\n"))
            .map_err(|e| format!("记不了已读: {e}"))
    }
}
