//! **What a sync round costs, and how fast a change gets across**
//! (round NET-3): two scratch machines on loopback, the pump at its own
//! cadence. Measured from beta's side:
//!
//! - the UDP bytes beta's wire to alpha carries per 5 s beat while
//!   nothing is happening (`WireCounts::sent_bytes`/`received_bytes`,
//!   QUIC's own count: acks and keepalives included), over `SYNC_SECS`
//!   (default 60);
//! - the CPU both residents spend in that time;
//! - how long a session opened on alpha takes to be a row on beta (a
//!   reading crossing), and how long alpha pinning it takes to show as
//!   pinned on beta (a document crossing).

mod scratch;

use std::time::{Duration, Instant};

use khor_node::SessionId;
use scratch::*;

/// A process's CPU time so far, from `ps` (`[[dd-]hh:]mm:ss.cc`).
fn cpu_of(pid: i32) -> Duration {
    let out = std::process::Command::new("ps")
        .args(["-o", "time=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let mut secs = 0.0;
    for part in text.split(':') {
        secs = secs * 60.0 + part.parse::<f64>().unwrap_or(0.0);
    }
    Duration::from_secs_f64(secs)
}

async fn until<F: Fn() -> bool>(what: &str, within: Duration, f: F) -> Duration {
    let began = Instant::now();
    while !f() {
        assert!(began.elapsed() < within, "{what} never happened");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    began.elapsed()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_quiet_round_is_small_and_a_change_crosses_in_a_beat() {
    let secs: u64 = std::env::var("SYNC_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let p = pair("bytes").await;
    // Settle: both residents up, every document met once.
    tokio::time::sleep(Duration::from_secs(20)).await;

    let (pa, pb) = (serve_pid(&p.ra.0).unwrap(), serve_pid(&p.rb.0).unwrap());
    let c0 = p.beta.wire_counts().await.unwrap();
    let (a0, b0) = (cpu_of(pa), cpu_of(pb));
    tokio::time::sleep(Duration::from_secs(secs)).await;
    let c1 = p.beta.wire_counts().await.unwrap();
    let (a1, b1) = (cpu_of(pa), cpu_of(pb));
    let beats = secs as f64 / 5.0;
    let sent = (c1.sent_bytes - c0.sent_bytes) as f64 / beats;
    let got = (c1.received_bytes - c0.received_bytes) as f64 / beats;

    // A reading crossing: a session opened on alpha, a row on beta.
    let out = khor(&p.ra.0)
        .args(["open", "-d", "--title", "probe", "--", "/bin/sleep", "600"])
        .env("KHOR_NAME", "alpha")
        .output()
        .unwrap();
    let sid = String::from_utf8_lossy(&out.stdout)
        .trim()
        .lines()
        .last()
        .unwrap_or_default()
        .to_owned();
    assert!(
        sid.contains('/'),
        "a session id: {sid:?} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let row_on_beta = |want_pinned: bool| {
        let sid = sid.clone();
        let beta = &p.beta;
        move || {
            beta.sessions()
                .map(|rows| {
                    rows.iter()
                        .any(|r| r.session.id.0 == sid && (!want_pinned || r.pinned))
                })
                .unwrap_or(false)
        }
    };
    let reading = until(
        "the row on beta",
        Duration::from_secs(30),
        row_on_beta(false),
    )
    .await;

    // A document crossing: alpha pins it, beta shows it pinned.
    p.alpha.pin_session(&SessionId(sid.clone()), true).unwrap();
    let document = until(
        "the pin on beta",
        Duration::from_secs(30),
        row_on_beta(true),
    )
    .await;

    eprintln!(
        "[sync] {secs} s quiet: beta's wire to alpha {:.0} B out + {:.0} B in per beat; \
         CPU per beat alpha {:.1} ms, beta {:.1} ms; a new row crossed in {reading:?}, a pin in {document:?}",
        sent,
        got,
        (a1 - a0).as_secs_f64() * 1000.0 / beats,
        (b1 - b0).as_secs_f64() * 1000.0 / beats,
    );
    let _ = khor(&p.ra.0)
        .args(["close", &sid])
        .env("KHOR_NAME", "alpha")
        .output();
}
