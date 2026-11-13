//! Physical sole-neighbor recovery with unusable/stale cached metadata.
#![cfg(target_os = "linux")]

mod scratch;

#[path = "netlab/attempts.rs"]
mod attempts;
#[path = "netlab/lab.rs"]
mod lab;
#[path = "netlab/hop/sole_cache.rs"]
mod sole_cache;
#[path = "netlab/hop/sole_peer.rs"]
mod sole_peer;

use std::time::Duration;

use khor_core::Hop;
use patchbay::{IpSupport, Lab, Nat, Router};

use lab::{mesh, node, relay, result, slow};

const LEG: Duration = Duration::from_millis(20);

enum Net {
    Strict,
}

async fn router(lab: &Lab, name: &str, _net: Net) -> Router {
    lab.add_router(name)
        .nat(Nat::Strict)
        .ip_support(IpSupport::V4Only)
        .build()
        .await
        .unwrap()
}

#[ctor::ctor(unsafe)]
fn userns() {
    patchbay::init_userns().expect("an unprivileged user namespace (docs/NETLAB.md)");
}

fn nft(dev: &patchbay::Device, rules: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(rules.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success(), "nft took:\n{rules}");
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn path_diagnostics(n: &crate::lab::LabNode, peer: &crate::lab::LabNode) {
    let rows: Vec<_> = n
        .journal(Some(peer))
        .into_iter()
        .filter(|e| {
            matches!(
                e["e"].as_str(),
                Some("hop" | "dial" | "settled" | "path" | "retired")
            )
        })
        .rev()
        .take(50)
        .collect();
    for row in rows.into_iter().rev() {
        eprintln!("[netlab-path-diagnostic] {} -> {} {row}", n.name, peer.name);
    }
}

fn hop_diagnostics(x: &crate::lab::LabNode, y: &crate::lab::LabNode, m: &crate::lab::LabNode) {
    for (n, peer) in [(x, y), (y, x), (x, m), (m, x), (y, m), (m, y)] {
        path_diagnostics(n, peer);
    }
}
