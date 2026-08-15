//! One device's node: identity plus the session surface (docs/SESSION.md)
//! over the kinds this build ships. The network arrives in the next
//! batch; everything here is honest without it.
//!
//! The kind trait gets extracted into core when the second kind lands —
//! an interface frozen against one implementor is guesswork.

pub mod chat;

pub use khor_core::{Session, SessionId};
pub use khor_sync::chat::{Message, MsgBody};

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use khor_core::{DeviceId, Event};

use crate::chat::ChatKind;

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
    device: DeviceId,
    /// The machine id as peers will name it: the public key, printable.
    device_str: String,
    /// Own channel name (docs: one machine, one window).
    name: String,
    chat: ChatKind,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<NodeEvent>>>>,
}

impl Node {
    /// Opens the node rooted at `root` — the OS home directory in real
    /// use; tests and same-machine dual instances override it via
    /// [`Self::root_from_env`].
    pub fn open(root: PathBuf) -> Result<Node, String> {
        let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
            .map_err(|e| e.to_string())?;
        let public = key.public();
        let device = DeviceId(*public.as_bytes());
        let host = gethostname::gethostname().to_string_lossy().to_string();
        let name = khor_sync::chat::channel_of_machine(&host)
            .ok_or_else(|| format!("主机名进不了路径: {host:?}"))?;
        let chat = ChatKind::new(&root, device, public.to_string(), name.clone());
        Ok(Node {
            device,
            device_str: public.to_string(),
            name,
            chat,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        })
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

    /// The machine id, printable (public key).
    pub fn device_str(&self) -> &str {
        &self.device_str
    }

    /// This machine's channel name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The list: one row per session, each answering the five questions.
    pub fn sessions(&self) -> Result<Vec<Session>, String> {
        self.chat.sessions()
    }

    /// Subscribes to everything that happens. Events push; the returned
    /// receiver ends when the node drops.
    pub fn watch(&self) -> mpsc::Receiver<NodeEvent> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    /// Says a line into a machine's channel. Returns the message id.
    pub fn say(&self, machine: &str, text: &str) -> Result<String, String> {
        let (id, row, event) = self.chat.say(machine, text)?;
        self.emit(NodeEvent::Event(event));
        self.emit(NodeEvent::Row(row));
        Ok(id)
    }

    /// A channel's readout, oldest first.
    pub fn log(&self, machine: &str) -> Result<chat::ChannelLog, String> {
        self.chat.log(machine)
    }

    /// Marks a session seen; unread drops to zero.
    pub fn seen(&self, id: &SessionId) -> Result<(), String> {
        let row = self.chat.seen(id)?;
        self.emit(NodeEvent::Row(row));
        Ok(())
    }

    /// Closes a session. For a device chat this deletes the history and
    /// the files it received (docs/SESSION.md 动作).
    pub fn close(&self, id: &SessionId) -> Result<(), String> {
        self.chat.close(id)?;
        self.emit(NodeEvent::Closed(id.clone()));
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

#[cfg(test)]
mod tests;
