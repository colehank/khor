//! One device's node: identity, the session surface (docs/SESSION.md),
//! and the live link to the rest of the network (docs/NET.md).
//!
//! No kind trait yet, on purpose: the second kind (transfer) derives its
//! rows from the chat document rather than owning state, so the two
//! kinds share only "produce rows" — a trait cut from that would freeze
//! an accident. Extraction waits for the first kind that is not derived
//! from chat (shell/tui, which own live processes).

pub mod chat;
pub mod ipc;
pub mod link;
pub mod proto;
pub mod transfer;

pub use khor_core::{Session, SessionId};
pub use khor_sync::chat::{FileRef, Message, MsgBody};
pub use khor_sync::devices::DeviceInfo;

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use khor_core::{DeviceId, Event};
use khor_sync::chat::{channel_dir, channel_of_machine, ChatDoc, Sender};
use khor_sync::devices::{devices_dir, DeviceDoc};
use khor_sync::seen::{seen_dir, SeenDoc};
use khor_sync::store::{load, Loaded};

use crate::chat::ChatKind;
use crate::transfer::TransferKind;

/// What subscribers receive. `watch()` is the one feed both faces
/// consume: the GUI repaints rows from it, the CLI prints from it —
/// polling is never the API.
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// A session's row changed (state, unread, title).
    Row(Session),
    /// A kind-namespaced event inside a session.
    Event(Event),
    /// The session is gone.
    Closed(SessionId),
}

pub struct Node {
    root: PathBuf,
    /// The identity key; also what the endpoint binds with. One live
    /// endpoint per key.
    key: iroh::SecretKey,
    device: DeviceId,
    /// The machine id as peers name it: the public key, hex.
    device_str: String,
    /// Own channel name.
    name: String,
    /// This process's writer peer, shared by every doc it writes.
    peer: u64,
    /// Relay roads this machine binds with and offers to others: the
    /// Khor tier plus `KHOR_RELAY` (docs/NET.md 中继).
    relays: Vec<String>,
    chat: ChatKind,
    transfer: TransferKind,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<NodeEvent>>>>,
}

impl Node {
    /// Opens the node rooted at `root`, named after the hostname (or
    /// `KHOR_NAME` when set — tests and same-machine dual instances).
    pub fn open(root: PathBuf) -> Result<Node, String> {
        let host = std::env::var("KHOR_NAME")
            .unwrap_or_else(|_| gethostname::gethostname().to_string_lossy().to_string());
        Self::open_as(root, &host)
    }

    /// Opens with an explicit machine name (pre-cleaning).
    pub fn open_as(root: PathBuf, name: &str) -> Result<Node, String> {
        let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
            .map_err(|e| e.to_string())?;
        let public = key.public();
        let device = DeviceId(*public.as_bytes());
        let device_str = public.to_string();
        let name = channel_of_machine(name)
            .ok_or_else(|| format!("这个名字进不了路径: {name:?}"))?;
        let chat = ChatKind::new(
            root.clone(),
            Sender { id: device_str.clone(), name: name.clone() },
        );
        let transfer = TransferKind::new(root.clone(), device_str.clone(), name.clone());
        let node = Node {
            root,
            key,
            device,
            device_str,
            name,
            peer: ChatDoc::fresh_peer(),
            relays: khor_net::endpoint::configured_relays(),
            chat,
            transfer,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        };
        node.register_self()?;
        Ok(node)
    }

    /// The table must know this machine before anyone else can. Name
    /// only — addresses belong to a live endpoint (`serve` writes them),
    /// and clobbering them with `[]` on every open would erase the hints
    /// other devices dial by.
    fn register_self(&self) -> Result<(), String> {
        let loaded = self.devices_loaded()?;
        let keep = loaded
            .doc
            .get(&self.device_str)
            .map(|d| d.addrs)
            .unwrap_or_default();
        loaded.doc.upsert(&self.device_str, &self.name, &keep)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// `KHOR_HOME` if set, else the home directory. The override exists
    /// for tests and dual-instance verification on one machine.
    pub fn root_from_env() -> PathBuf {
        std::env::var_os("KHOR_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::home_dir().unwrap_or_else(|| PathBuf::from(".")))
    }

    pub fn device(&self) -> DeviceId {
        self.device
    }

    /// The machine id, printable (public key, hex).
    pub fn device_str(&self) -> &str {
        &self.device_str
    }

    /// This machine's channel name.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn root(&self) -> &PathBuf {
        &self.root
    }

    pub(crate) fn writer_peer(&self) -> u64 {
        self.peer
    }

    pub(crate) fn secret_key(&self) -> &iroh::SecretKey {
        &self.key
    }

    pub(crate) fn relays(&self) -> &[String] {
        &self.relays
    }

    pub(crate) fn devices_loaded(&self) -> Result<Loaded<DeviceDoc>, String> {
        load(&devices_dir(&self.root), self.peer)
    }

    pub(crate) fn seen_loaded(&self) -> Result<Loaded<SeenDoc>, String> {
        load(&seen_dir(&self.root), self.peer)
    }

    /// Raises a session's seen watermark and persists it. The watermark
    /// travels the network (docs/NET.md): clear here, clear
    /// everywhere on the next sync.
    fn mark_seen(&self, session: &SessionId, at: i64) -> Result<(), String> {
        let loaded = self.seen_loaded()?;
        loaded.doc.mark(&session.0, at)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// Everyone in the network, this machine included.
    pub fn devices(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self.devices_loaded()?.doc.all())
    }

    /// name → (channel, home). Refuses unknown machines listing what the
    /// table has — "not found" alone sends people guessing.
    fn resolve(&self, machine: &str) -> Result<(String, DeviceId), String> {
        let ch = channel_of_machine(machine)
            .ok_or_else(|| format!("这个机器名进不了路径: {machine:?}"))?;
        let all = self.devices()?;
        match all.iter().find(|d| d.name == ch) {
            Some(d) => Ok((ch, device_id_from_hex(&d.id)?)),
            None => {
                let names: Vec<&str> = all.iter().map(|d| d.name.as_str()).collect();
                Err(format!(
                    "机器不存在: {machine}。网里现在有: {}",
                    names.join("、")
                ))
            }
        }
    }

    fn channel_of_session(&self, id: &SessionId) -> Result<(String, DeviceId), String> {
        let Some(ch) = id.0.strip_prefix("chat/") else {
            return Err(format!("没有这个 session: {}", id.0));
        };
        self.resolve(ch)
            .map_err(|e| format!("没有这个 session: {}({e})", id.0))
    }

    /// The list: one row per session, each answering the five questions.
    pub fn sessions(&self) -> Result<Vec<Session>, String> {
        let seen = self.seen_loaded()?;
        let mut rows = Vec::new();
        for d in self.devices()? {
            let wm = seen.doc.watermark(&format!("chat/{}", d.name));
            rows.push(self.chat.row(&d.name, device_id_from_hex(&d.id)?, wm)?);
            let dir = channel_dir(&self.root, &d.name)
                .ok_or_else(|| format!("频道名不合法: {:?}", d.name))?;
            let msgs = self.chat.log(&d.name)?.messages;
            rows.extend(self.transfer.rows(&d.name, &dir, &msgs, |sid| seen.doc.watermark(sid)));
        }
        Ok(rows)
    }

    /// Offers a file to a machine's window: the summary (name, size,
    /// digest) travels the CRDT to everyone; the bytes wait here for the
    /// far side's approval. Returns the transfer session id.
    pub fn send(&self, machine: &str, path: &Path) -> Result<SessionId, String> {
        let (channel, home) = self.resolve(machine)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| format!("这不是一个文件路径: {}", path.display()))?;
        let (digest, size) = transfer::digest_file(path)?;
        let f = FileRef { name: name.clone(), size: size as i64, digest: digest.clone() };
        let told = self.chat.send_files(&channel, home, &[f])?;
        transfer::save_offer(
            &self.root,
            &digest,
            &transfer::Offer {
                path: path
                    .canonicalize()
                    .map_err(|e| format!("定不下这个路径: {e}"))?,
                name,
                size,
                started: false,
                done: false,
            },
        )?;
        self.mark_seen(&told.row.id, told.at)?;
        self.emit(NodeEvent::Event(told.event));
        self.emit(NodeEvent::Row(told.row));
        Ok(TransferKind::session_id(&told.msg_id))
    }

    /// The file message behind `transfer/<msg_id>`, with its channel.
    pub(crate) fn find_transfer(&self, msg_id: &str) -> Result<(String, Message), String> {
        for d in self.devices()? {
            let log = self.chat.log(&d.name)?;
            if let Some(m) = log
                .messages
                .into_iter()
                .find(|m| m.id == msg_id && matches!(m.body, MsgBody::Files(_)))
            {
                return Ok((d.name, m));
            }
        }
        Err(format!("没有这个传输: transfer/{msg_id}"))
    }

    /// Subscribes to everything that happens. Events push; the returned
    /// receiver ends when the node drops.
    pub fn watch(&self) -> mpsc::Receiver<NodeEvent> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    /// Tells a machine's window a line. Returns the message id.
    pub fn tell(&self, machine: &str, text: &str) -> Result<String, String> {
        let (channel, home) = self.resolve(machine)?;
        let told = self.chat.tell(&channel, home, text)?;
        // Telling implies looking: own words never count as unread — on
        // any device, once the watermark syncs.
        self.mark_seen(&told.row.id, told.at)?;
        self.emit(NodeEvent::Event(told.event));
        self.emit(NodeEvent::Row(told.row));
        Ok(told.msg_id)
    }

    /// A channel's readout, oldest first.
    pub fn log(&self, machine: &str) -> Result<chat::ChannelLog, String> {
        let (channel, _) = self.resolve(machine)?;
        self.chat.log(&channel)
    }

    /// Marks a session seen; unread drops to zero — here and, once the
    /// watermark syncs, on every device.
    pub fn seen(&self, id: &SessionId) -> Result<(), String> {
        if id.0.starts_with("chat/") {
            let (channel, home) = self.channel_of_session(id)?;
            let (last_at, row) = self.chat.seen(&channel, home)?;
            self.mark_seen(id, last_at)?;
            self.emit(NodeEvent::Row(row));
            return Ok(());
        }
        if let Some(msg_id) = id.0.strip_prefix("transfer/") {
            let (channel, m) = self.find_transfer(msg_id)?;
            let dir = channel_dir(&self.root, &channel)
                .ok_or_else(|| format!("频道名不合法: {channel:?}"))?;
            // What "looked at" covers here is the landed payload; before
            // it lands there is nothing unread to clear (待批 clears by
            // answering, not by looking — docs/UX.md 角标).
            let MsgBody::Files(files) = &m.body else {
                return Err(format!("没有这个传输: {}", id.0));
            };
            let at = files
                .iter()
                .filter_map(|f| transfer::mtime_ms(&transfer::payload_path(&dir, f)))
                .max()
                .unwrap_or(0);
            self.mark_seen(id, at)?;
            self.emit_transfer_row(&channel, &m)?;
            return Ok(());
        }
        Err(format!("没有这个 session: {}", id.0))
    }

    /// Closes a session. A device chat deletes the payloads it received;
    /// a transfer deletes just its own payload (the summary stays in the
    /// CRDT — the row falls back to 待批 and can be pulled again).
    pub fn close(&self, id: &SessionId) -> Result<(), String> {
        if id.0.starts_with("chat/") {
            let (channel, _) = self.channel_of_session(id)?;
            self.chat.close(&channel)?;
            self.emit(NodeEvent::Closed(id.clone()));
            return Ok(());
        }
        if let Some(msg_id) = id.0.strip_prefix("transfer/") {
            let (channel, m) = self.find_transfer(msg_id)?;
            let dir = channel_dir(&self.root, &channel)
                .ok_or_else(|| format!("频道名不合法: {channel:?}"))?;
            let MsgBody::Files(files) = &m.body else {
                return Err(format!("没有这个传输: {}", id.0));
            };
            for f in files {
                self.transfer.close(&dir, f)?;
            }
            self.emit_transfer_row(&channel, &m)?;
            return Ok(());
        }
        Err(format!("没有这个 session: {}", id.0))
    }

    /// Recomputes one transfer's row and pushes it to watchers.
    fn emit_transfer_row(&self, channel: &str, m: &Message) -> Result<(), String> {
        let dir = channel_dir(&self.root, channel)
            .ok_or_else(|| format!("频道名不合法: {channel:?}"))?;
        let seen = self.seen_loaded()?;
        let rows = self
            .transfer
            .rows(channel, &dir, std::slice::from_ref(m), |sid| seen.doc.watermark(sid));
        if let Some(row) = rows.into_iter().next() {
            self.emit(NodeEvent::Row(row));
        }
        Ok(())
    }

    fn emit(&self, event: NodeEvent) {
        // Dead receivers drop out here; send failure is disconnection,
        // not an error.
        self.subscribers
            .lock()
            .unwrap()
            .retain(|tx| tx.send(event.clone()).is_ok());
    }
}

/// hex → the 32 key bytes. The table stores ids as hex; iroh wants bytes.
pub(crate) fn device_id_from_hex(s: &str) -> Result<DeviceId, String> {
    let s = s.trim();
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("这不是一个机器 id: {s:?}"));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).unwrap() as u8;
        let lo = (chunk[1] as char).to_digit(16).unwrap() as u8;
        out[i] = (hi << 4) | lo;
    }
    Ok(DeviceId(out))
}

#[cfg(test)]
mod tests;
