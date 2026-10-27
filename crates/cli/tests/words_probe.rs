//! **The words under loss** (round NET-6, the lead's probe list; the
//! owner's first complaint was words flipping between unknown, 直连 and
//! 中继). A phone resident in this process and a Mac resident, scratch,
//! that can reach each other directly on loopback, with their relay
//! behind a road adding `DIAG_RTT_MS` (default 200) of round trip and
//! stalling `DIAG_LOSS_PCT` of chunks. The direct path is clean: loopback
//! has no loss to add without changing the system's network (pf, dnctl,
//! an interface alias), which a probe does not do — so this measures
//! whether a lossy relay shakes the word for a machine reached directly.
//!
//! For `DIAG_WATCH_SECS` (default 60) the phone asks the Mac for a
//! listing every 2 s and reads its word every 100 ms: how long until the
//! first 直连, every change of word after it, and the share of time in
//! each.
//!
//! `DIAG_LOSS_PCT=10 cargo test -p khor-cli --test words_probe -- --ignored --nocapture`.

mod scratch;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use khor_node::{Hop, Node};
use scratch::*;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

fn word_of(hop: Hop) -> &'static str {
    match hop {
        Hop::Direct => "直连",
        Hop::Relay => "中继",
        Hop::Idle => "未连接",
        Hop::NoServe => "无常驻",
        Hop::Ssh => "经 SSH",
        Hop::Forwarded => "转发",
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "a probe over a simulated road: run by hand"]
async fn the_word_for_a_machine_reached_directly_holds_while_the_relay_is_lossy() {
    let rtt = Duration::from_millis(env_u64("DIAG_RTT_MS", 200));
    let loss = env_u64("DIAG_LOSS_PCT", 5) as u32;
    let watch = Duration::from_secs(env_u64("DIAG_WATCH_SECS", 60));
    own_tmux();
    let (rm, rp, rr) = (
        scratch("words-mac"),
        scratch("words-phone"),
        scratch("words-relay"),
    );
    let (_relay, direct) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let (at, _road) = slow_road(
        to,
        Impair {
            rtt,
            loss_pct: loss,
            stall: rtt,
        },
    )
    .await;
    let url = format!("http://{at}");
    for h in [&rm.0, &rp.0] {
        home_on(h, &url);
    }
    let _mac_serve = serve_with(&rm.0, "mac", &[]);
    stood_up(&[&rm.0]).await;
    let phone_server = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    let _phone_serve = tokio::spawn(async move { phone_server.serve_resumable().await });
    stood_up(&[&rp.0]).await;
    let mac = Node::open_as(rm.0.clone(), "mac").unwrap();
    let phone = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    tokio::time::timeout(Duration::from_secs(90), phone.pair(&mac.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    phone.face_is_here(true).await.unwrap();
    let mac_id = mac.device_str().to_owned();
    let dir = rm.0.display().to_string();

    let began = Instant::now();
    let asking = {
        let phone = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
        let dir = dir.clone();
        tokio::spawn(async move {
            let (mut ok, mut failed, mut slowest) = (0u32, 0u32, Duration::ZERO);
            while began.elapsed() < watch {
                let t = Instant::now();
                match phone.ls_of("mac", &dir).await {
                    Ok(_) => ok += 1,
                    Err(_) => failed += 1,
                }
                slowest = slowest.max(t.elapsed());
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            (ok, failed, slowest)
        })
    };
    let mut first_direct = None;
    let mut changes: Vec<(u128, &'static str)> = Vec::new();
    let mut time_in: BTreeMap<&'static str, Duration> = BTreeMap::new();
    let mut last = Instant::now();
    let mut beat = 0u32;
    while began.elapsed() < watch {
        if beat.is_multiple_of(100) {
            let _ = phone.face_is_here(true).await;
        }
        beat += 1;
        let hop = phone.links().await.ok().and_then(|l| {
            l.into_iter()
                .find(|(i, _)| *i == mac_id)
                .map(|(_, l)| l.hop)
        });
        let w = hop.map(word_of).unwrap_or("（无行）");
        if w == "直连" && first_direct.is_none() {
            first_direct = Some(began.elapsed());
        }
        if changes.last().map(|(_, l)| *l) != Some(w) {
            changes.push((began.elapsed().as_millis(), w));
        }
        *time_in.entry(w).or_default() += last.elapsed();
        last = Instant::now();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (ok, failed, slowest) = asking.await.unwrap();
    let shares: Vec<String> = time_in
        .iter()
        .map(|(w, d)| format!("{w} {:.1}%", 100.0 * d.as_secs_f64() / watch.as_secs_f64()))
        .collect();
    eprintln!(
        "[words] relay loss {loss}% rtt {rtt:?}, {}s: first 直连 after {first_direct:?}; {} changes {:?}; time {}; \
         listings ok {ok} failed {failed}, slowest {slowest:?}",
        watch.as_secs(),
        changes.len().saturating_sub(1),
        changes.iter().take(12).collect::<Vec<_>>(),
        shares.join(", "),
    );
}
