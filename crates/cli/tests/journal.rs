//! **The connection journal** (NET-STABLE 第一步, `khor_node::journal`),
//! measured through a relay outage on real `khor serve` processes.
//!
//! Two scratch homes reach each other **only** through a `khor relay
//! --dev` on loopback, behind a road this test can cut
//! (`KHOR_RELAY_ONLY`, `cuttable_road`): cutting it is the relay going
//! away for both, healing it is the relay back. Fresh identities in
//! temporary homes, own tmux socket, no mDNS; nothing here touches a real
//! relay, store or key. Processes are stopped by pid.
//!
//! The outage lasts `KHOR_JOURNAL_OUTAGE_S` seconds (default 45). What
//! it prints is the measurement in audit/net-journal.md: how long the
//! wire took to be retired, the backoff the failed dials left, and how
//! long after the relay came back the two were connected again.

mod scratch;

use std::time::{Duration, Instant};

use khor_node::{Hop, Node};
use scratch::*;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_relay_outage_is_journalled_and_the_report_holds_no_secret() {
    let outage = Duration::from_secs(
        std::env::var("KHOR_JOURNAL_OUTAGE_S")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(45),
    );
    let (ra, rb, rr) = (scratch("journal-a"), scratch("journal-b"), scratch("journal-r"));
    let (_relay, url) = local_relay(&rr.0);
    let to: std::net::SocketAddr = url
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let quick = Impair {
        rtt: Duration::ZERO,
        loss_pct: 0,
        stall: Duration::ZERO,
    };
    let (at, cut, _road) = cuttable_road(to, quick).await;
    let road = format!("http://{at}");
    home_on(&ra.0, &road);
    home_on(&rb.0, &road);
    let env: &[(&str, &str)] = &[("KHOR_RELAY_ONLY", "1")];
    let _a = serve_with(&ra.0, "alpha", env);
    let _b = serve_with(&rb.0, "beta", env);
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    let ticket = alpha.invite().unwrap();
    let began = Instant::now();
    loop {
        match timeout(Duration::from_secs(60), beta.pair(&ticket)).await {
            Ok(Ok(_)) => break,
            other if began.elapsed() < Duration::from_secs(90) => {
                eprintln!("[journal] pairing again: {other:?}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            other => panic!("pairing: {other:?}"),
        }
    }
    let alpha_id = alpha.device_str().to_owned();
    until_reads(&beta, &alpha_id, Hop::Relay, Duration::from_secs(60)).await;
    // Past the settle window, so the journal has where it settled.
    tokio::time::sleep(Duration::from_secs(12)).await;

    cut.cut();
    let cut_at = Instant::now();
    let idle = until_reads(&beta, &alpha_id, Hop::Idle, Duration::from_secs(90)).await;
    if let Some(left) = outage.checked_sub(cut_at.elapsed()) {
        tokio::time::sleep(left).await;
    }
    cut.heal();
    let back = until_reads(&beta, &alpha_id, Hop::Relay, Duration::from_secs(420)).await;
    eprintln!(
        "[journal] relay cut: beta read 未连接 after {idle:?}; healed after {outage:?}; \
         beta read 中继 again {back:?} after the relay came back"
    );

    for (who, n) in [("beta", &beta), ("alpha", &alpha)] {
        eprintln!("[journal] ── {who} ──");
        for l in n.journal(None, 30) {
            let mark = if l.trouble { "!" } else { " " };
            eprintln!("{} {mark} {}\t{}", l.when, l.machine, l.text);
        }
    }
    let raw = khor_node::journal::read(&rb.0, Some(&alpha_id), 0);
    let has = |f: &dyn Fn(&khor_node::journal::What) -> bool| raw.iter().any(|e| f(&e.what));
    use khor_node::journal::What;
    assert!(has(&|w| matches!(w, What::Started)), "the resident's start");
    assert!(
        has(&|w| matches!(w, What::Dial { won: Some(_), .. })),
        "a dial that went through"
    );
    assert!(
        has(&|w| matches!(w, What::Settled { road, .. } if road == "relay")),
        "where the wire settled"
    );
    assert!(
        has(&|w| matches!(w, What::Retired { .. })),
        "the wire the outage killed"
    );
    assert!(
        has(&|w| matches!(w, What::Dial { won: None, retry_s: Some(_), .. })),
        "a failed dial with the backoff it left"
    );
    assert!(
        has(&|w| matches!(w, What::Home { state, .. } if state == "disconnected")),
        "the home relay lost"
    );

    let out = khor(&rb.0)
        .args(["journal", "--report", "--minutes", "30"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let said = String::from_utf8_lossy(&out.stdout);
    let path = said
        .split_once(['：', ':'])
        .map(|(_, p)| p.trim().to_owned())
        .expect("the report says where it is");
    let report = std::fs::read_to_string(&path).unwrap();
    assert!(report.contains("alpha"), "{report}");
    for secret in ["identity", "BEGIN", "token=", "Bearer"] {
        assert!(!report.contains(secret), "the report carries {secret:?}");
    }
    let key = std::fs::read(rb.0.join(".khor/identity.key")).unwrap();
    let key_hex: String = key.iter().map(|b| format!("{b:02x}")).collect();
    assert!(
        !report.contains(&key_hex[..16.min(key_hex.len())]),
        "the report carries the identity key"
    );
}
