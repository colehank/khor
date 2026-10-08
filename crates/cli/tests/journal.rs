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
    use khor_node::journal::What;
    // **The relay back is the network back** (NET-STABLE 第二步): once
    // both homes are connected again, a dial goes through within seconds
    // — not after the backoff the outage left. Measured before the fix: a
    // 240 s outage, relay back at 16:33:37, the dial at 16:34:50.
    let homes_back = [&ra.0, &rb.0]
        .iter()
        .map(|h| {
            khor_node::journal::read(h, None, 0)
                .iter()
                .rev()
                .find(|e| e.peer.is_none() && matches!(&e.what, What::Home { state, .. } if state == "connected"))
                .map(|e| e.at)
                .expect("each home relay connected again")
        })
        .max()
        .unwrap();
    let dialled_back = [&ra.0, &rb.0]
        .iter()
        .filter_map(|h| {
            khor_node::journal::read(h, None, 0)
                .into_iter()
                .filter(|e| e.peer.is_some() && matches!(e.what, What::Dial { won: Some(_), .. }))
                .map(|e| e.at)
                .filter(|at| *at + 1_000 >= homes_back)
                .min()
        })
        .min()
        .expect("a dial after the relay came back");
    let late = dialled_back.saturating_sub(homes_back);
    eprintln!("[journal] the later home relay connected again; the first dial through came {late} ms after");
    assert!(late < 5_000, "the dial came {late} ms after the relay was back");
    // On the side that forgave: the other may have been dialled first.
    assert!(
        [&ra.0, &rb.0].iter().any(|h| {
            khor_node::journal::read(h, None, 0)
                .iter()
                .any(|e| matches!(&e.what, What::Network { source } if source == "relay"))
        }),
        "the relay coming back is journalled"
    );

    let raw = khor_node::journal::read(&rb.0, Some(&alpha_id), 0);
    let has = |f: &dyn Fn(&khor_node::journal::What) -> bool| raw.iter().any(|e| f(&e.what));
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

/// **One machine loses its home relay, the other's still answers it** —
/// alpha homes on the relay through the road this test cuts, beta on the
/// same relay server under its plain address. A measurement for
/// audit/net-journal.md, run by hand (`--ignored`). Measured 2026-10-08:
/// it does **not** isolate alpha — beta's relay address rides in beta's
/// row, alpha's dial offers it, and iroh connects to it beside the dead
/// home: the wire was retired after 16 s of silence and the next pump
/// round redialled through beta's relay in 8 ms. Cutting a machine off
/// entirely needs every relay address behind a cut.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn measure_one_side_losing_its_relay() {
    let outage = Duration::from_secs(
        std::env::var("KHOR_JOURNAL_OUTAGE_S")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120),
    );
    let (ra, rb, rr) = (scratch("journal1-a"), scratch("journal1-b"), scratch("journal1-r"));
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
    home_on(&ra.0, &format!("http://{at}"));
    home_on(&rb.0, &url);
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
                eprintln!("[journal1] pairing again: {other:?}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            other => panic!("pairing: {other:?}"),
        }
    }
    let (alpha_id, beta_id) = (alpha.device_str().to_owned(), beta.device_str().to_owned());
    until_reads(&beta, &alpha_id, Hop::Relay, Duration::from_secs(60)).await;
    until_reads(&alpha, &beta_id, Hop::Relay, Duration::from_secs(60)).await;
    tokio::time::sleep(Duration::from_secs(12)).await;

    cut.cut();
    tokio::time::sleep(outage).await;
    cut.heal();
    let healed = Instant::now();
    let a_back = until_reads(&alpha, &beta_id, Hop::Relay, Duration::from_secs(600)).await;
    let b_back = until_reads(&beta, &alpha_id, Hop::Relay, Duration::from_secs(600)).await;
    eprintln!(
        "[journal1] alpha's road cut for {outage:?}; after it healed alpha read 中继 in {a_back:?}, \
         beta in {:?}",
        b_back.max(healed.elapsed().saturating_sub(Duration::from_millis(100)))
    );
    for (who, n) in [("alpha", &alpha), ("beta", &beta)] {
        eprintln!("[journal1] ── {who} ──");
        for l in n.journal(None, 30) {
            let mark = if l.trouble { "!" } else { " " };
            eprintln!("{} {mark} {}\t{}", l.when, l.machine, l.text);
        }
    }
}
