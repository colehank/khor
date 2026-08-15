//! Decision-class data that replicates across the whole network
//! (docs/NET.md 同步): CRDT documents and their on-disk block stores.
//!
//! Why a document CRDT (loro) and not a live sync protocol: the network
//! has nodes that run nothing (reachable only over ssh). A loro increment
//! is bytes that can land as a file, so a dumb directory carries the same
//! sync a live stream does.

pub mod chat;
