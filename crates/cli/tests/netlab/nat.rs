//! Scenarios a and d: two machines behind the NATs this week's STUN
//! measurements found (audit/net-d.md §1.1), on one plain relay.
//!
//! a. Both ends behind endpoint-dependent ("strict") NAT, the campus and
//!    the carrier: on IPv4 they cannot punch, so the relay must carry
//!    them without a stuck dial. With global IPv6 on both sides they
//!    should go direct over v6, as turing ↔ iPhone does; on one side only,
//!    it is the relay again.
//! d. One end's NAT gives every flow a port of its own (MacLab's proxy
//!    TUN), the other end public, behind a home router, or behind a
//!    full-cone NAT. Plus the control: two home routers.
//!
//! The relays here answer no QUIC address discovery, like khor's own
//! today (net-d §5.2): a machine behind NAT learns no public address.

use std::time::Duration;

use khor_core::Hop;
use patchbay::{IpSupport, Lab, Nat, Router, RouterPreset};

use crate::lab::{LabNode, mesh, node, relay, result, word};

/// The network a machine sits behind.
#[derive(Clone, Copy, Debug)]
pub enum Net {
    /// A public address, no NAT, no firewall (turing).
    Public,
    /// A home router: one port for every destination, replies only from
    /// where it sent (EIM + APDF).
    Home,
    /// Full cone: one port for every destination, anyone may send to it.
    Open,
    /// A new random port per destination (EDM + APDF), v4 only: the
    /// campus's commercial links, the carrier, a proxy's TUN.
    Strict,
    /// Strict v4 plus global v6 with no firewall (the campus on CERNET).
    Campus,
    /// Strict v4 plus v6 behind a stateful firewall (the phone).
    Cellular,
}

pub async fn router(lab: &Lab, name: &str, net: Net) -> Router {
    let b = lab.add_router(name);
    let b = match net {
        Net::Public => b.preset(RouterPreset::PublicV4),
        Net::Home => b.nat(Nat::Moderate).ip_support(IpSupport::V4Only),
        Net::Open => b.nat(Nat::Open).ip_support(IpSupport::V4Only),
        Net::Strict => b.nat(Nat::Strict).ip_support(IpSupport::V4Only),
        Net::Campus => b.nat(Nat::Strict).ip_support(IpSupport::DualStack),
        Net::Cellular => b.preset(RouterPreset::IspCgnatSymmetric),
    };
    b.build().await.unwrap()
}

/// What a pair should end up on.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Want {
    /// Direct (punched) within a punching round.
    Direct,
    /// Anything that carries bytes; never a dial that hangs.
    Carried,
}

/// How long after the first road a pair may take to go direct: iroh
/// starts a round at once and again every 60 s (`UPGRADE_INTERVAL`).
const PUNCH: Duration = Duration::from_secs(75);

/// The longest a dial may take before it counts as stuck.
const STUCK: u64 = 10_000;

async fn two(scenario: &str, left: Net, right: Net, want: Want) {
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, scenario, "relay", None).await;
    let (ra, rb) = (router(&lab, "left", left).await, router(&lab, "right", right).await);
    let da = lab.add_device("a").uplink(ra.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rb.id()).build().await.unwrap();
    let a = node(scenario, da, "alpha", &[&r], &[]).await;
    let b = node(scenario, db, "beta", &[&r], &[]).await;
    // Both on the relay before either is looked for there.
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&a, &b]).await;

    let first = a.until(&b, Duration::from_secs(60), |l| l.rtt_ms.is_some()).await;
    let Some((took, first)) = first else {
        result(scenario, false, format!("{left:?} × {right:?}: no road with a round trip in 60 s"));
        panic!("{scenario}: alpha reaches beta");
    };
    let direct = a.until(&b, PUNCH, |l| l.hop == Hop::Direct).await;
    let last = a.link(&b).await.expect("a link");
    let (dials, failed, slowest) = dials(&a, &b);
    let (dials_b, failed_b, slowest_b) = dials(&b, &a);
    let ok = match want {
        Want::Direct => direct.is_some(),
        Want::Carried => last.rtt_ms.is_some() && last.down_ms.is_none(),
    } && slowest.max(slowest_b) <= STUCK;
    result(
        scenario,
        ok,
        format!(
            "{left:?} × {right:?}: first {} after {took:.1?}; direct {}; now {}; dials {}/{} failed, slowest {} ms",
            word(&first),
            direct.as_ref().map_or("never".into(), |(t, l)| format!("after {t:.1?} ({})", word(l))),
            word(&last),
            failed + failed_b,
            dials + dials_b,
            slowest.max(slowest_b),
        ),
    );
    assert!(slowest.max(slowest_b) <= STUCK, "{scenario}: no dial over {STUCK} ms");
    match want {
        Want::Direct => assert!(direct.is_some(), "{scenario}: {left:?} × {right:?} goes direct"),
        Want::Carried => assert!(last.rtt_ms.is_some(), "{scenario}: {left:?} × {right:?} is carried"),
    }
}

/// Dials `from` made to `to` (journal `dial` lines): how many, how many
/// failed, the slowest in ms.
fn dials(from: &LabNode, to: &LabNode) -> (usize, usize, u64) {
    let lines: Vec<_> = from.journal(Some(to)).into_iter().filter(|e| e["e"] == "dial").collect();
    let failed = lines.iter().filter(|e| e["won"].is_null()).count();
    let slowest = lines.iter().filter_map(|e| e["ms"].as_u64()).max().unwrap_or(0);
    (lines.len(), failed, slowest)
}

// a. Both behind endpoint-dependent NAT.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a1_strict_both_v4_rides_the_relay() {
    two("a1", Net::Strict, Net::Strict, Want::Carried).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a2_strict_both_v6_on_one_side_rides_the_relay() {
    two("a2", Net::Campus, Net::Strict, Want::Carried).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a3_strict_both_v6_on_both_goes_direct_over_v6() {
    two("a3", Net::Campus, Net::Cellular, Want::Direct).await;
}

// d. A port of its own per flow (the proxy TUN) against each other kind.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d1_port_per_flow_to_a_public_peer_goes_direct() {
    two("d1", Net::Strict, Net::Public, Want::Direct).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d2_port_per_flow_to_full_cone_goes_direct() {
    two("d2", Net::Strict, Net::Open, Want::Direct).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d3_port_per_flow_to_a_home_router_rides_the_relay() {
    two("d3", Net::Strict, Net::Home, Want::Carried).await;
}

/// The control: two home routers punch through in every other stack.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn d4_two_home_routers_go_direct() {
    two("d4", Net::Home, Net::Home, Want::Direct).await;
}
