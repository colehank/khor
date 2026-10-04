//! **The phone's way back** (round NET-6, the lead's probe list): a
//! phone, a Mac resident and a far machine, all scratch, reaching each
//! other only through a loopback relay behind a road adding `DIAG_RTT_MS`
//! (default 200) of round trip and stalling `DIAG_LOSS_PCT` (default 2)
//! of chunks — the setting of `diagnose.rs`. Every word is read against
//! the road: it is relay-only, so 中继 is the honest word for a machine
//! that answers and 未连接 for one that does not.
//!
//! - **R, away:** the phone's app backgrounded for `DIAG_AWAY_SECS`
//!   (default 600) — wires let go (`suspending`) and its ears closed
//!   (`deafen`), which is what iOS freezing the process does to them —
//!   then resumed. Meanwhile, every minute: the Mac's word for the phone
//!   and what the absent phone costs the Mac (dials, bytes). Then, from
//!   the resume: new ears, the round, the first honest word for each
//!   machine, the first answer from the far machine, and the Mac's word
//!   for the phone coming back.
//! - **N, the network changes** as the app is told of it
//!   (`network_changed`, the phone's path-change call).
//! - **W, Wi-Fi ↔ cellular by rebinding:** the phone's endpoint bound
//!   again on a new port without a suspension (`resume`), the nearest a
//!   loopback test comes to the address under a connection changing.
//!
//! A probe, not a gate:
//! `DIAG_AWAY_SECS=600 cargo test -p khor-cli --test resume_probe -- --ignored --nocapture`.

mod scratch;

use std::time::{Duration, Instant};

use khor_node::{Hop, Node};
use scratch::*;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

async fn word(n: &Node, id: &str) -> String {
    match n.links().await {
        Ok(links) => links
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, l)| match l.hop {
                Hop::Direct => "直连".to_owned(),
                Hop::Relay => "中继".to_owned(),
                Hop::Idle => "未连接".to_owned(),
                Hop::NoServe => "无常驻".to_owned(),
            })
            .unwrap_or_else(|| "（无行）".to_owned()),
        Err(_) => "（读不到）".to_owned(),
    }
}

/// Polls every 50 ms until `f` answers true, up to `within`; how long it took.
async fn until<F, Fut>(within: Duration, mut f: F) -> Option<Duration>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let began = Instant::now();
    while began.elapsed() < within {
        if f().await {
            return Some(began.elapsed());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
}

/// From `began`: the first honest word for each of `ids`, and the first
/// answer from `far` — polled side by side, so neither waits on the other.
async fn the_way_back(
    phone: &Node,
    ids: [(&str, &str); 2],
    dir: &str,
    began: Instant,
) -> (Vec<(String, Option<Duration>)>, Option<Duration>, Vec<String>) {
    let mut seen = Vec::new();
    let words = async {
        let mut firsts = vec![None; ids.len()];
        while began.elapsed() < Duration::from_secs(120) && firsts.iter().any(Option::is_none) {
            let mut now = Vec::new();
            for (k, (_, id)) in ids.iter().enumerate() {
                let w = word(phone, id).await;
                if w == "中继" && firsts[k].is_none() {
                    firsts[k] = Some(began.elapsed());
                }
                now.push(w);
            }
            let line = now.join("/");
            if seen.last() != Some(&line) {
                seen.push(line);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        ids.iter()
            .zip(firsts)
            .map(|((name, _), t)| ((*name).to_owned(), t))
            .collect::<Vec<_>>()
    };
    let answer = async {
        until(Duration::from_secs(120), || async { phone.ls_of("far", dir).await.is_ok() })
            .await
            .map(|_| began.elapsed())
    };
    let (words, answer) = tokio::join!(words, answer);
    (words, answer, seen)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "a probe over a simulated road, minutes to an hour long: run by hand"]
async fn the_phone_comes_back_after_being_away_and_after_its_network_changes() {
    let rtt = Duration::from_millis(env_u64("DIAG_RTT_MS", 200));
    let loss = env_u64("DIAG_LOSS_PCT", 2) as u32;
    let away = Duration::from_secs(env_u64("DIAG_AWAY_SECS", 600));
    // SAFETY: before any endpoint in this process binds.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    own_tmux();
    let (rm, rf, rp, rr) = (scratch("back-mac"), scratch("back-far"), scratch("back-phone"), scratch("back-relay"));
    let (_relay, direct) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct.trim_start_matches("http://").trim_end_matches('/').parse().unwrap();
    let (at, _road) = slow_road(to, Impair { rtt, loss_pct: loss, stall: rtt }).await;
    let url = format!("http://{at}");
    for h in [&rm.0, &rf.0, &rp.0] {
        home_on(h, &url);
    }
    let env = [("KHOR_RELAY_ONLY", "1")];
    // A copy of khor the build cannot touch (`DIAG_KHOR_BIN`): a resident
    // whose binary is rebuilt under it restarts by design, and an hour
    // is long enough for somebody to build.
    let bin = std::env::var_os("DIAG_KHOR_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_khor").into());
    let _mac_serve = serve_from(&rm.0, "mac", &env, &bin);
    let _far_serve = serve_from(&rf.0, "far", &env, &bin);
    stood_up(&[&rm.0, &rf.0]).await;
    let phone_server = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    let _phone_serve = tokio::spawn(async move { phone_server.serve_resumable().await });
    stood_up(&[&rp.0]).await;

    let mac = Node::open_as(rm.0.clone(), "mac").unwrap();
    let far = Node::open_as(rf.0.clone(), "far").unwrap();
    let phone = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    for n in [&far, &phone] {
        let ticket = mac.invite().unwrap();
        let began = Instant::now();
        loop {
            match tokio::time::timeout(Duration::from_secs(90), n.pair(&ticket)).await {
                Ok(Ok(_)) => break,
                other if began.elapsed() < Duration::from_secs(150) => {
                    eprintln!("[back] pairing: {other:?}");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                other => panic!("pairs: {other:?}"),
            }
        }
    }
    let (mac_id, far_id, phone_id) = (
        mac.device_str().to_owned(),
        far.device_str().to_owned(),
        phone.device_str().to_owned(),
    );
    let dir = rf.0.join("tree");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.display().to_string();
    let first = until(Duration::from_secs(90), || async { phone.ls_of("far", &dir).await.is_ok() }).await;
    eprintln!("[back] the phone's first listing on the far machine answered after {first:?}");
    // The face is on the phone, as it is when the app goes away.
    phone.face_is_here(true).await.unwrap();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let ids = [("mac", mac_id.as_str()), ("far", far_id.as_str())];

    // R: away.
    phone.face_is_here(false).await.unwrap();
    assert!(khor_node::link::suspending(&rp.0).await, "the phone's resident suspends");
    assert!(khor_node::link::deafen(&rp.0).await, "and its ears close");
    let m0 = mac.wire_counts().await.unwrap();
    let left = Instant::now();
    let mut mac_words = Vec::new();
    while left.elapsed() < away {
        tokio::time::sleep(Duration::from_secs(60).min(away.saturating_sub(left.elapsed()))).await;
        let w = word(&mac, &phone_id).await;
        if mac_words.last().map(|(_, l)| l) != Some(&w) {
            mac_words.push((left.elapsed().as_secs(), w));
        }
    }
    let m1 = mac.wire_counts().await.unwrap();
    let mins = away.as_secs_f64() / 60.0;
    eprintln!(
        "[back] R away {}s: the Mac's word for the phone (from s) {mac_words:?}; the Mac meanwhile dialled {} times, \
         {:.0} B/min, rounds {}",
        away.as_secs(),
        m1.dialed.saturating_sub(m0.dialed),
        (m1.sent_bytes + m1.received_bytes).saturating_sub(m0.sent_bytes + m0.received_bytes) as f64 / mins,
        m1.rounds.saturating_sub(m0.rounds),
    );
    let began = Instant::now();
    phone.face_is_here(true).await.unwrap();
    let mut resuming = khor_node::link::resume(&rp.0).expect("a resumable resident");
    let ears = resuming.ears().await.map(|_| began.elapsed());
    let back = the_way_back(&phone, ids, &dir, began);
    let mac_back = until(Duration::from_secs(120), || async { word(&mac, &phone_id).await == "中继" });
    let synced = async { resuming.synced().await.map(|o| (began.elapsed(), o.iter().filter(|(_, r)| r.is_ok()).count())) };
    let ((words, answer, seen), mac_back, synced) = tokio::join!(back, mac_back, synced);
    eprintln!(
        "[back] R resumed after {}s: ears {ears:?}, round {synced:?}; first 中继 {words:?}; first far answer \
         {answer:?}; the Mac's 中继 for the phone after {mac_back:?}; the phone's words in order {seen:?}",
        away.as_secs(),
    );
    tokio::time::sleep(Duration::from_secs(10)).await;

    // N: the network changes, as the app is told of it.
    let began = Instant::now();
    assert!(khor_node::link::network_changed(&rp.0));
    let (words, answer, seen) = the_way_back(&phone, ids, &dir, began).await;
    eprintln!("[back] N network_changed: first 中继 {words:?}; first far answer {answer:?}; words in order {seen:?}");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // W: Wi-Fi ↔ cellular, by rebinding on a new port.
    let began = Instant::now();
    let mut rebinding = khor_node::link::resume(&rp.0).expect("a resumable resident");
    let ears = rebinding.ears().await.map(|_| began.elapsed());
    let (words, answer, seen) = the_way_back(&phone, ids, &dir, began).await;
    let mac_back = until(Duration::from_secs(60), || async { word(&mac, &phone_id).await == "中继" }).await;
    eprintln!(
        "[back] W rebound: ears {ears:?}; first 中继 {words:?}; first far answer {answer:?}; the Mac's 中继 for the \
         phone {mac_back:?} after; words in order {seen:?}"
    );
}
