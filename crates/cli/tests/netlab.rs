//! **The network lab** (docs/NETLAB.md): real khor residents in Linux
//! network namespaces, behind the NATs and links this week's measurements
//! found (audit/net-d.md), on n0's patchbay. A connection bug becomes a
//! scenario here instead of the owner holding a phone.
//!
//! Each scenario is written as **today's result**: it asserts what khor
//! should do, and is red where khor does not do it yet. The numbers it
//! measured are printed as `[netlab-result]` lines, which
//! `scripts/netlab.sh` collects.
//!
//! Linux only, unprivileged: the test process enters a user namespace of
//! its own before the harness starts a thread (`userns`). It needs `nft`
//! and `tc` on PATH; on turing `scripts/netlab.sh` brings both.
//!
//! Scratch homes and identities, own tmux socket, no mDNS
//! (`scratch::khor`), `KHOR_NETLAB` on every resident
//! (`endpoint::LAB_ENV`: patchbay's internet is 198.18.0.0/15).
#![cfg(target_os = "linux")]

mod scratch;

#[path = "netlab/hop.rs"]
mod hop;
#[path = "netlab/lab.rs"]
mod lab;
#[path = "netlab/nat.rs"]
mod nat;
#[path = "netlab/paths.rs"]
mod paths;
#[path = "netlab/relay.rs"]
mod relay;
#[path = "netlab/uplink.rs"]
mod uplink;

/// Enters a user namespace of this process's own before any thread
/// exists: every namespace the lab makes after is owned by it.
#[ctor::ctor(unsafe)]
fn userns() {
    patchbay::init_userns().expect("an unprivileged user namespace (docs/NETLAB.md)");
}
