//! **A slow road is not a dead one** (round NET-2, item 1): two nodes
//! that can reach each other only through a relay (`KHOR_RELAY_ONLY`),
//! with the relay behind a road that adds a round trip and stalls a share
//! of chunks the way a retransmission would (`scratch::slow_road`). Both
//! machines stay up for the whole run, so **every wire retired is a false
//! retirement**, and every 未连接 on the row is a false word.
//!
//! Loopback relay, temporary homes and identities; processes by pid.
//! `SLOW_RTT_MS` (default 300), `SLOW_LOSS_PCT` (default 2) and
//! `SLOW_SECS` (default 120) set the road and the length of the run.

mod scratch;

use std::time::{Duration, Instant};

use khor_node::Hop;
use scratch::*;

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_slow_lossy_relay_retires_nothing_and_reads_relay_throughout() {
    let rtt = Duration::from_millis(env_u64("SLOW_RTT_MS", 300));
    let loss = env_u64("SLOW_LOSS_PCT", 2) as u32;
    let secs = env_u64("SLOW_SECS", 120);
    let impair = Impair {
        rtt,
        loss_pct: loss,
        // A loss under TCP costs a fast retransmit: about one round trip.
        stall: rtt.max(Duration::from_millis(50)),
    };
    let (p, _road) = pair_on("slow", Some(impair)).await;
    let alpha_id = p.alpha.device_str().to_owned();
    until_reads(&p.beta, &alpha_id, Hop::Relay, Duration::from_secs(60)).await;
    tokio::time::sleep(Duration::from_secs(5)).await;

    let a0 = p.alpha.wire_counts().await.unwrap();
    let b0 = p.beta.wire_counts().await.unwrap();
    let tmp = std::env::temp_dir().display().to_string();
    let started = Instant::now();
    let (mut asks, mut failed, mut slowest) = (0u32, 0u32, Duration::ZERO);
    let (mut samples, mut not_relay) = (0u32, 0u32);
    while started.elapsed() < Duration::from_secs(secs) {
        // What a face does with a machine page open: a listing, and the
        // row's word, every two seconds.
        let began = Instant::now();
        asks += 1;
        if p.beta.ls_of("alpha", &tmp).await.is_err() {
            failed += 1;
        }
        slowest = slowest.max(began.elapsed());
        if let Ok(links) = p.beta.links().await {
            samples += 1;
            if !links
                .iter()
                .any(|(id, l)| *id == alpha_id && l.hop == Hop::Relay)
            {
                not_relay += 1;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    let a1 = p.alpha.wire_counts().await.unwrap();
    let b1 = p.beta.wire_counts().await.unwrap();
    let retired = |c0: &khor_node::WireCounts, c1: &khor_node::WireCounts| {
        (c1.retired_silent + c1.retired_unanswered + c1.retired_closed + c1.retired_network)
            - (c0.retired_silent + c0.retired_unanswered + c0.retired_closed + c0.retired_network)
    };
    let false_retired = retired(&a0, &a1) + retired(&b0, &b1);
    let per_minute = false_retired as f64 * 60.0 / secs as f64;
    let dial_failed = (a1.dial_failed - a0.dial_failed) + (b1.dial_failed - b0.dial_failed);
    eprintln!(
        "[slow] {}ms added RTT, {loss}% stalled {}ms, {secs}s: false retirements {false_retired} \
         ({per_minute:.1}/min), failed dials {dial_failed}, listings {asks} ({failed} failed, slowest \
         {slowest:?}), word samples {samples} ({not_relay} not 中继)",
        rtt.as_millis(),
        impair.stall.as_millis(),
    );
    assert_eq!(
        false_retired, 0,
        "a wire to a machine that never went away was retired"
    );
    assert_eq!(failed, 0, "a listing failed on a slow but answering road");
    assert_eq!(
        not_relay, 0,
        "the word left 中继 on a road that never broke"
    );
}

/// **首选中继 wins whenever it is reachable** (round NET-2, item 3): a
/// node with two relays homes on the nearer by itself; told to prefer the
/// farther one, it homes there — and with the preferred one gone, it
/// homes on the other rather than on nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_preferred_relay_is_home_while_it_answers() {
    let (rn, rf, home) = (
        scratch("pref-near"),
        scratch("pref-far"),
        scratch("pref-node"),
    );
    let (_near, near) = local_relay(&rn.0);
    let (far_relay, far_direct) = local_relay(&rf.0);
    let to: std::net::SocketAddr = far_direct
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let (at, _road) = slow_road(
        to,
        Impair {
            rtt: Duration::from_millis(400),
            loss_pct: 0,
            stall: Duration::ZERO,
        },
    )
    .await;
    let far = format!("http://{at}");
    let write = |preferred: Option<&str>| {
        std::fs::create_dir_all(home.0.join(".khor")).unwrap();
        std::fs::write(
            home.0.join(".khor").join("relays.json"),
            serde_json::json!({
                "relays": [{ "url": near }, { "url": far }],
                "only_mine": true,
                "preferred": preferred,
            })
            .to_string(),
        )
        .unwrap();
    };
    let home_after = |label: &'static str| {
        let root = home.0.clone();
        async move {
            let s = serve(&root, "solo");
            stood_up(&[&root]).await;
            let n = khor_node::Node::open_as(root.clone(), "solo").unwrap();
            let began = Instant::now();
            let mut home = None;
            while began.elapsed() < Duration::from_secs(40) {
                if let Ok(c) = n.connections().await
                    && c.home_connected
                {
                    home = c.home_relay;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            drop(s);
            let _ = std::fs::remove_file(root.join(".khor/endpoint.json"));
            tokio::time::sleep(Duration::from_secs(1)).await;
            eprintln!("[pref] {label}: home {home:?}");
            home
        }
    };
    let same = |a: &Option<String>, b: &str| {
        a.as_deref().map(|u| u.trim_end_matches('/')) == Some(b.trim_end_matches('/'))
    };

    write(None);
    let by_itself = home_after("no preference").await;
    write(Some(&far));
    let preferring = home_after("prefers the far one").await;
    drop(far_relay);
    let far_gone = home_after("the far one gone").await;

    assert!(
        same(&by_itself, &near),
        "by itself it homes on the nearer: {by_itself:?}"
    );
    assert!(
        same(&preferring, &far),
        "a reachable preference is home: {preferring:?}"
    );
    assert!(
        same(&far_gone, &near),
        "an unreachable preference is not: {far_gone:?}"
    );
}
