//! Chat: one CRDT document per channel, persisted as a stack of
//! append-only block files.
//!
//! Two transports carry the same bytes: a live stream between devices
//! ([`wire`]), or block files copied to a node that runs nothing
//! ([`plan`]). A channel is a machine's window — one machine, one
//! channel, every device writes into the same conversation.

pub mod doc;
pub mod plan;
pub mod store;
pub mod wire;

pub use doc::{ChatDoc, FileRef, Message, MsgBody, Sender};
pub use plan::{plan, Ledger, Plan, Side};
pub use store::{
    channel_dir, channel_of_machine, valid_channel, ChatStore, Loaded, REL_DIR,
};
pub use wire::{
    changes_since_b64, merge_b64, version_b64, Outgoing, Peer, Reply, Round,
    MAX_BYTES,
};

#[cfg(test)]
mod tests;
