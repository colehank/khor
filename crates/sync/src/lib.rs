//! Decision-class data that replicates across the whole network
//! (docs/NET.md): CRDT documents, their on-disk block stores, and
//! the two transports that move them.
//!
//! Why document CRDTs (loro) and not a live sync protocol: the network
//! has nodes that run nothing (reachable only over ssh). A loro increment
//! is bytes that can land as a file, so a dumb directory carries the same
//! sync a live stream does.

pub mod accounts;
pub mod agents;
pub mod chat;
pub mod devices;
pub mod dirpins;
pub mod doc_transfer;
mod glue;
pub mod pins;
pub mod plan;
pub mod recents;
pub mod roster;
pub mod seen;
pub mod store;
pub mod titles;
pub mod transfer;
pub mod webpins;
pub mod wire;
pub mod workspaces;

#[cfg(test)]
mod testutil;
