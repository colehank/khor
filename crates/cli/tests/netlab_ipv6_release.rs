//! Original physical IPv6 mobility contracts in an ordinary Release test entry.
#![cfg(target_os = "linux")]

mod scratch;

#[path = "netlab/attempts.rs"]
mod attempts;
#[path = "netlab/lab.rs"]
mod lab;
#[path = "netlab/paths_ipv6_release.rs"]
mod paths;

#[ctor::ctor(unsafe)]
fn userns() {
    patchbay::init_userns().expect("an unprivileged user namespace (docs/NETLAB.md)");
}
