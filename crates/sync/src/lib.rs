//! Decision-class data that replicates across the whole network
//! (docs/NET.md): CRDT documents, their on-disk block stores, and
//! the two transports that move them.
//!
//! Why document CRDTs (loro) and not a live sync protocol: the network
//! has nodes that run nothing (reachable only over ssh). A loro increment
//! is bytes that can land as a file, so a dumb directory carries the same
//! sync a live stream does.

pub mod agents;
pub mod chat;
pub mod devices;
mod glue;
pub mod dirpins;
pub mod pins;
pub mod plan;
pub mod seen;
pub mod store;
pub mod webpins;
pub mod wire;

#[cfg(test)]
mod testutil;
