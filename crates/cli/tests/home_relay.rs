//! **A machine homed on a relay in no list is still reachable** (net-f,
//! 2026-10-09). MacLab, with n0's tier on, homed on n0's aps1 while its
//! row published only Beijing, hk and pk; the iPhone, dialling those,
//! found nobody for twenty minutes.
//!
//! Here two scratch residents keep 「only mine」 with one relay nobody runs
//! (`127.0.0.1:9`), and a loopback relay stands in for n0's tier
//! (`KHOR_PUBLIC_RELAYS`), so each homes on a relay its own list does not
//! name. No direct road (`KHOR_RELAY_ONLY`). Each must publish the live
//! home first — in its row and in `endpoint.json`, which an invite reads —
//! and, after both residents restart with nothing remembered, each reaches
//! the other by the row alone.
//!
//! Loopback relay, temporary homes and identities, own tmux socket, no
//! mDNS; processes by pid.

mod scratch;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::Node;
use scratch::*;
use tokio::time::timeout;

/// A relay nobody runs: the only one either machine names.
const DEAD: &str = "http://127.0.0.1:9";

async fn until<F, Fut>(within: Duration, what: &str, mut f: F) -> Duration
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let started = Instant::now();
    while !f().await {
        assert!(started.elapsed() < within, "{what} within {within:?}");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    started.elapsed()
}

/// The relays `endpoint.json` hands an invite.
fn published(dir: &Path) -> Vec<String> {
    std::fs::read(dir.join(".khor/endpoint.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v["relays"].clone()).ok())
        .unwrap_or_default()
}

/// Stops a resident by pid and starts it again with nothing remembered.
async fn restart(dir: &Path, name: &str, env: &[(&str, &str)], was: Reaped) -> Reaped {
    let pid = serve_pid(dir).expect("a resident");
    drop(was);
    signal(pid, "-TERM");
    until(Duration::from_secs(20), "the resident stops", || async {
        !Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())
    })
    .await;
    let _ = std::fs::remove_file(dir.join(".khor/endpoint.json"));
    let again = serve_with(dir, name, env);
    stood_up(&[dir]).await;
    again
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_homed_on_a_relay_in_no_list_publishes_it_and_is_reached() {
    let (ra, rb, rr) = (scratch("home-a"), scratch("home-b"), scratch("home-r"));
    let (_relay, public) = local_relay(&rr.0);
    let public = public.trim_end_matches('/').to_owned();
    for r in [&ra, &rb] {
        home_on(&r.0, DEAD);
    }
    let env = [("KHOR_RELAY_ONLY", "1"), (khor_net::endpoint::PUBLIC_RELAYS_ENV, public.as_str())];
    let a = serve_with(&ra.0, "alpha", &env);
    let b = serve_with(&rb.0, "beta", &env);
    stood_up(&[&ra.0, &rb.0]).await;
    for (r, who) in [(&ra, "alpha"), (&rb, "beta")] {
        let took = until(Duration::from_secs(60), &format!("{who} publishes its home first"), || async {
            published(&r.0).first() == Some(&public)
        })
        .await;
        eprintln!("[home] {who} published {:?} {took:?} after it stood up", published(&r.0));
    }

    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(90), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds over the published home");
    let a_id = alpha.device_str().to_owned();
    until(Duration::from_secs(60), "beta's table has alpha's row with its home first", || async {
        beta.devices()
            .ok()
            .and_then(|all| all.into_iter().find(|d| d.id == a_id))
            .is_some_and(|d| d.addrs.iter().find(|r| r.starts_with("http")) == Some(&public))
    })
    .await;

    // Nothing remembered: each reaches the other by the row alone.
    let _a = restart(&ra.0, "alpha", &env, a).await;
    let _b = restart(&rb.0, "beta", &env, b).await;
    let there = until(Duration::from_secs(90), "beta lists alpha", || async {
        beta.ls_of("alpha", "/").await.is_ok()
    })
    .await;
    eprintln!("[home] beta reached alpha {there:?} after both restarted");
    let back = until(Duration::from_secs(90), "alpha lists beta", || async {
        alpha.ls_of("beta", "/").await.is_ok()
    })
    .await;
    eprintln!("[home] alpha reached beta {back:?} after that");
    // And the rows say so after the restarts: a resident that bound before
    // its first net report kept the home its row named.
    for (node, who) in [(&alpha, "alpha"), (&beta, "beta")] {
        let all = node.devices().unwrap();
        for d in &all {
            assert_eq!(
                d.addrs.iter().find(|r| r.starts_with("http")),
                Some(&public),
                "{who}'s table has {}'s row with the live home first: {:?}",
                d.shown(),
                d.addrs
            );
        }
    }
}
