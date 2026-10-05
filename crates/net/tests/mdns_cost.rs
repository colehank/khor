//! **What mDNS costs a long-lived process** (TERM 38, term-mdns.md): the
//! owner's app had rebound its endpoint many times, and every bind had
//! left its mDNS service running. This probe stands up that process: one
//! key, `MDNS_COST_BINDS` binds closed behind it (default 20), one live
//! endpoint, then this process's CPU in three windows of `MDNS_COST_SECS`
//! (default 30), while the segment's other khor nodes answer.
//!
//! Run by hand, with scratch nodes announcing on the segment:
//! `cargo test --profile local -p khor-net --test mdns_cost -- --ignored --nocapture`
//! It binds with no relay at all, so it touches nothing but the segment.

use khor_net::endpoint;
use std::time::Duration;

fn env_or(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// This process's user plus system time, in seconds, as `ps` reads it.
fn cpu_seconds() -> f64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "time=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps runs");
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .split(':')
        .fold(0.0, |acc, part| acc * 60.0 + part.parse::<f64>().unwrap_or(0.0))
}

/// Sockets this process holds on UDP 5353, as `lsof` reads them.
fn mdns_sockets() -> usize {
    let out = std::process::Command::new("lsof")
        .args(["-nP", "-a", "-p", &std::process::id().to_string(), "-iUDP:5353"])
        .output()
        .expect("lsof runs");
    String::from_utf8_lossy(&out.stdout).lines().skip(1).count()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "probe: needs the segment, run by hand"]
async fn mdns_cost_of_a_long_lived_process() {
    // `KHOR_NO_MDNS` set is the control: what the closed binds cost
    // without any mDNS service.
    let mdns = std::env::var_os(endpoint::NO_MDNS_ENV).is_none();
    let binds = env_or("MDNS_COST_BINDS", 20);
    let secs = env_or("MDNS_COST_SECS", 30);
    let secret = iroh::SecretKey::generate();
    for _ in 0..binds {
        let ep = endpoint::bind(secret.clone(), &[]).await.unwrap();
        ep.close().await;
    }
    let live = endpoint::bind(secret, &[]).await.unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    let sockets = mdns_sockets();
    // Three windows, so a cost that is only the closed binds' tear-down
    // shows as the first window's and not as the steady state.
    let mut windows = Vec::new();
    for _ in 0..3 {
        let before = cpu_seconds();
        tokio::time::sleep(Duration::from_secs(secs)).await;
        windows.push(format!("{:.1} %", (cpu_seconds() - before) * 100.0 / secs as f64));
    }
    println!(
        "mdns_cost: mdns {mdns}, binds closed {binds}, sockets on 5353 {sockets}, cpu per {secs} s window: {}",
        windows.join(", ")
    );
    live.close().await;
}
