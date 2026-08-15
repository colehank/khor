//! The device-chat kind: a machine's window, ridden on the chat CRDT.
//!
//! This batch the device table holds only this machine, so the only
//! reachable channel is our own (notes to self). Foreign machines join
//! when pairing lands; telling one now is refused by name, not
//! silently written into a channel nobody serves.

use std::fs;
use std::path::{Path, PathBuf};

use khor_core::{kind, DeviceId, Event, Kind, Millis, Session, SessionId, State, StateStamp};
use khor_sync::chat::{channel_dir, channel_of_machine, ChatDoc, ChatStore, Message, Sender};

/// One channel's readout.
pub struct ChannelLog {
    pub messages: Vec<Message>,
    /// Blocks that would not read — a missing slice of conversation the
    /// caller must be able to mention.
    pub broken: usize,
}

pub struct ChatKind {
    /// Our own channel's directory.
    dir: PathBuf,
    device: DeviceId,
    me: Sender,
    /// Own channel name.
    own: String,
    /// This process's writer peer (one per live writer — see
    /// `ChatDoc::new`).
    peer: u64,
}

impl ChatKind {
    pub fn new(root: &Path, device: DeviceId, device_str: String, own: String) -> ChatKind {
        let dir = channel_dir(root, &own)
            .expect("channel_of_machine output always passes the whitelist");
        ChatKind {
            dir,
            device,
            me: Sender { id: device_str, name: own.clone() },
            own,
            peer: ChatDoc::fresh_peer(),
        }
    }

    fn session_id(&self) -> SessionId {
        SessionId(format!("chat/{}", self.own))
    }

    /// Refuses names that resolve to no known device, listing what
    /// exists — "not found" alone sends people guessing.
    fn resolve(&self, machine: &str) -> Result<(), String> {
        let ch = channel_of_machine(machine)
            .ok_or_else(|| format!("这个机器名进不了路径: {machine:?}"))?;
        if ch == self.own {
            return Ok(());
        }
        Err(format!(
            "机器不存在: {machine}。设备表这一批只有本机({}),配对落地后才有别的",
            self.own
        ))
    }

    fn check(&self, id: &SessionId) -> Result<(), String> {
        let want = self.session_id();
        if *id == want {
            return Ok(());
        }
        Err(format!("没有这个 session: {}(有的是 {})", id.0, want.0))
    }

    fn load(&self) -> Result<khor_sync::chat::Loaded, String> {
        ChatStore::load(&self.dir, self.peer)
    }

    fn row_from(&self, doc: &ChatDoc) -> Session {
        let msgs = doc.messages();
        let total = msgs.len() as u64;
        let unread = total.saturating_sub(self.seen_count());
        let at = msgs.last().map(|m| m.at.max(0) as u64).unwrap_or(0);
        Session {
            id: self.session_id(),
            kind: Kind(kind::CHAT.to_owned()),
            title: self.own.clone(),
            home: self.device,
            // docs/SESSION.md 对话·对设备: unseen content = Done, seen =
            // Idle; the first four words are unreachable (no process).
            state: StateStamp {
                state: if unread > 0 { State::Done } else { State::Idle },
                at: Millis(at),
            },
            unread,
        }
    }

    pub fn sessions(&self) -> Result<Vec<Session>, String> {
        Ok(vec![self.row_from(&self.load()?.doc)])
    }

    pub fn tell(&self, machine: &str, text: &str) -> Result<(String, Session, Event), String> {
        self.resolve(machine)?;
        let loaded = self.load()?;
        let mut store = loaded.store;
        let msg_id = loaded.doc.tell(&self.me, text)?;
        store.flush(&loaded.doc)?;
        let total = loaded.doc.messages().len() as u64;
        // Telling implies looking at the channel: own words never count
        // as unread.
        self.write_seen(total)?;
        let row = self.row_from(&loaded.doc);
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

    pub fn log(&self, machine: &str) -> Result<ChannelLog, String> {
        self.resolve(machine)?;
        let loaded = self.load()?;
        Ok(ChannelLog {
            messages: loaded.doc.messages(),
            broken: loaded.broken.len(),
        })
    }

    pub fn seen(&self, id: &SessionId) -> Result<Session, String> {
        self.check(id)?;
        let loaded = self.load()?;
        self.write_seen(loaded.doc.messages().len() as u64)?;
        Ok(self.row_from(&loaded.doc))
    }

    /// Deletes the conversation and everything it received — docs的判词:
    /// 删对设备的对话,连它收下的文件一起删。
    pub fn close(&self, id: &SessionId) -> Result<(), String> {
        self.check(id)?;
        if self.dir.exists() {
            fs::remove_dir_all(&self.dir).map_err(|e| format!("删不掉: {e}"))?;
        }
        Ok(())
    }

    // The seen marker is a machine-local dotfile in the channel dir: no
    // `.loro` extension, so it never syncs (same trick as the merge
    // ledger). Cross-device clearing arrives with the read-state CRDT;
    // until then unread clears only where you cleared it.
    fn seen_path(&self) -> PathBuf {
        self.dir.join(".seen")
    }

    fn seen_count(&self) -> u64 {
        fs::read_to_string(self.seen_path())
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    fn write_seen(&self, n: u64) -> Result<(), String> {
        // An empty channel leaves nothing on disk; tell() creates the dir
        // via flush before this runs.
        if !self.dir.exists() {
            return Ok(());
        }
        fs::write(self.seen_path(), format!("{n}\n")).map_err(|e| format!("记不了已读: {e}"))
    }
}
