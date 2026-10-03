//! **A second diagnosis pass** (round NET-5, after audit/net.md): a
//! phone, a Mac resident and a far machine, all scratch, reaching each
//! other only through a loopback relay behind a road adding `DIAG_RTT_MS`
//! (default 200) of round trip and stalling `DIAG_LOSS_PCT` (default 2)
//! of chunks by one round trip. The phone is what the iPhone app is: a
//! node that hosts nothing, with a resumable resident in its own process.
//!
//! Phases, each printed as a timeline (`[diag] …`):
//!
//! - **A, quiet:** a minute of nothing — bytes each resident's wires carry,
//!   and every second the phone's word for the Mac and the far machine
//!   (the road is relay-only, so anything but 中继 is a lie).
//! - **B, the phone backgrounded and back:** suspended 20 s, resumed —
//!   how long until its ears answer, its round is done, a listing on the
//!   far machine answers; its words meanwhile.
//! - **C, the far machine gone:** its resident stopped — how long the
//!   phone and the Mac keep saying it is there, and what a listing on it
//!   costs while it is gone.
//! - **D, the far machine back:** how long until the words and a listing
//!   come back.
//! - **E, the Mac's network changes:** retirements, and whether a listing
//!   from the phone through it still answers.
//!
//! A probe, not a gate: `cargo test -p khor-cli --test diagnose -- --ignored --nocapture`.

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

/// Polls until `f` answers true, up to `within`; how long it took.
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
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    None
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "a diagnosis probe over a simulated road: run by hand"]
async fn a_phone_a_mac_and_a_far_machine_over_a_slow_lossy_relay() {
    let rtt = Duration::from_millis(env_u64("DIAG_RTT_MS", 200));
    let loss = env_u64("DIAG_LOSS_PCT", 2) as u32;
    let impair = Impair {
        rtt,
        loss_pct: loss,
        stall: rtt,
    };
    // The phone's resident is in this process: it must be relay-only too.
    // SAFETY: before any endpoint in this process binds.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    own_tmux();
    let (rm, rf, rp, rr) = (scratch("diag-mac"), scratch("diag-far"), scratch("diag-phone"), scratch("diag-relay"));
    let (_relay, direct) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct.trim_start_matches("http://").trim_end_matches('/').parse().unwrap();
    let (at, _road) = slow_road(to, impair).await;
    let url = format!("http://{at}");
    for h in [&rm.0, &rf.0, &rp.0] {
        home_on(h, &url);
    }
    let env = [("KHOR_RELAY_ONLY", "1")];
    let mac_serve = serve_with(&rm.0, "mac", &env);
    let mut far_serve = Some(serve_with(&rf.0, "far", &env));
    stood_up(&[&rm.0, &rf.0]).await;
    // The phone: hosts nothing, its resident resumable, in this process.
    let phone_server = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    let _phone_serve = tokio::spawn(async move { phone_server.serve_resumable().await });
    stood_up(&[&rp.0]).await;
    tokio::time::sleep(Duration::from_secs(10)).await;

    let mac = Node::open_as(rm.0.clone(), "mac").unwrap();
    let far = Node::open_as(rf.0.clone(), "far").unwrap();
    let phone = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    for (who, n) in [("far", &far), ("phone", &phone)] {
        let ticket = mac.invite().unwrap();
        let began = Instant::now();
        loop {
            match tokio::time::timeout(Duration::from_secs(90), n.pair(&ticket)).await {
                Ok(Ok(_)) => break,
                other if began.elapsed() < Duration::from_secs(150) => {
                    eprintln!("[diag] {who} pairing: {other:?}");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                other => panic!("{who} pairs: {other:?}"),
            }
        }
        eprintln!("[diag] {who} paired in {:?}", began.elapsed());
    }
    let (mac_id, far_id) = (mac.device_str().to_owned(), far.device_str().to_owned());
    // Everybody meets everybody through the Mac's table.
    let met = until(Duration::from_secs(90), || async {
        phone.devices().map(|d| d.iter().any(|x| x.id == far_id)).unwrap_or(false)
    })
    .await;
    eprintln!("[diag] the phone learned of the far machine in {met:?}");
    let dir = rf.0.join("tree");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.display().to_string();
    let first = until(Duration::from_secs(90), || async { phone.ls_of("far", &dir).await.is_ok() }).await;
    eprintln!("[diag] the phone's first listing on the far machine answered after {first:?}");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // A: a quiet minute.
    let quiet = env_u64("DIAG_QUIET_SECS", 60);
    let (p0, m0, f0) = (
        phone.wire_counts().await.unwrap(),
        mac.wire_counts().await.unwrap(),
        far.wire_counts().await.unwrap(),
    );
    let mut lies: Vec<String> = Vec::new();
    for s in 0..quiet {
        for (name, id) in [("mac", &mac_id), ("far", &far_id)] {
            let w = word(&phone, id).await;
            if w != "中继" {
                lies.push(format!("{s}s phone→{name} {w}"));
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let (p1, m1, f1) = (
        phone.wire_counts().await.unwrap(),
        mac.wire_counts().await.unwrap(),
        far.wire_counts().await.unwrap(),
    );
    let per_min = |a: &khor_core::WireCounts, b: &khor_core::WireCounts| {
        ((b.sent_bytes - a.sent_bytes + b.received_bytes - a.received_bytes) as f64 * 60.0 / quiet as f64) as u64
    };
    let retired = |a: &khor_core::WireCounts, b: &khor_core::WireCounts| {
        (b.retired_silent + b.retired_unanswered + b.retired_closed + b.retired_network)
            - (a.retired_silent + a.retired_unanswered + a.retired_closed + a.retired_network)
    };
    eprintln!(
        "[diag] A quiet {quiet}s: bytes/min phone {} mac {} far {}; retired phone {} mac {} far {}; \
         dials phone {} mac {} far {}; words not 中继: {} {:?}",
        per_min(&p0, &p1),
        per_min(&m0, &m1),
        per_min(&f0, &f1),
        retired(&p0, &p1),
        retired(&m0, &m1),
        retired(&f0, &f1),
        p1.dialed - p0.dialed,
        m1.dialed - m0.dialed,
        f1.dialed - f0.dialed,
        lies.len(),
        lies.iter().take(8).collect::<Vec<_>>()
    );

    // B: the phone backgrounded 20 s, then back.
    let pb0 = phone.wire_counts().await.unwrap();
    assert!(khor_node::link::suspending(&rp.0).await, "the phone's resident suspends");
    let mut while_away = Vec::new();
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_secs(5)).await;
        while_away.push(format!("{}/{}", word(&phone, &mac_id).await, word(&phone, &far_id).await));
    }
    let pb1 = phone.wire_counts().await.unwrap();
    eprintln!(
        "[diag] B while backgrounded 20 s: the phone dialled {} times, its wires carried {} B",
        pb1.dialed - pb0.dialed,
        (pb1.sent_bytes + pb1.received_bytes) - (pb0.sent_bytes + pb0.received_bytes)
    );
    let began = Instant::now();
    let mut resuming = khor_node::link::resume(&rp.0).expect("a resumable resident");
    let ears = resuming.ears().await.map(|_| began.elapsed());
    let synced = resuming.synced().await.map(|o| (began.elapsed(), o));
    let listed = until(Duration::from_secs(60), || async { phone.ls_of("far", &dir).await.is_ok() })
        .await
        .map(|_| began.elapsed());
    let mut after = Vec::new();
    for _ in 0..10 {
        after.push(format!("{}/{}", word(&phone, &mac_id).await, word(&phone, &far_id).await));
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    eprintln!(
        "[diag] B backgrounded 20s, words meanwhile (mac/far) {while_away:?}; resumed: ears {ears:?}, round {:?}, \
         a far listing answered by {listed:?}; words over the next 5 s {after:?}",
        synced.as_ref().map(|(d, o)| (d, o.iter().map(|(n, r)| (n.clone(), r.is_ok())).collect::<Vec<_>>())),
    );

    // C: the far machine gone.
    let gone_at = Instant::now();
    drop(far_serve.take());
    let ls_gone = {
        let began = Instant::now();
        let r = phone.ls_of("far", &dir).await;
        (began.elapsed(), r.err())
    };
    let phone_says = until(Duration::from_secs(120), || async { word(&phone, &far_id).await != "中继" }).await;
    let mac_says = until(Duration::from_secs(120), || async { word(&mac, &far_id).await != "中继" }).await;
    eprintln!(
        "[diag] C far stopped: a listing right after took {:?} ({:?}); the phone stopped saying 中继 after {:?} \
         (from the stop: {:?}), the Mac after {:?}; now phone says {}",
        ls_gone.0,
        ls_gone.1,
        phone_says,
        gone_at.elapsed(),
        mac_says,
        word(&phone, &far_id).await,
    );

    // D: the far machine back.
    let back_at = Instant::now();
    far_serve = Some(serve_with(&rf.0, "far", &env));
    let listed = until(Duration::from_secs(120), || async { phone.ls_of("far", &dir).await.is_ok() }).await;
    let worded = until(Duration::from_secs(120), || async { word(&phone, &far_id).await == "中继" }).await;
    eprintln!(
        "[diag] D far back: a listing from the phone answered after {listed:?}, the phone's word 中继 after {worded:?} \
         (from the start: {:?})",
        back_at.elapsed()
    );

    // E: the phone's network changes (its resident is the in-process one).
    let p2 = phone.wire_counts().await.unwrap();
    assert!(khor_node::link::network_changed(&rp.0));
    tokio::time::sleep(Duration::from_secs(5)).await;
    let began = Instant::now();
    let through = phone.ls_of("far", &dir).await;
    let p3 = phone.wire_counts().await.unwrap();
    eprintln!(
        "[diag] E the phone's network changed: its wires retired {} (network {}), a far listing {} in {:?}; \
         words {}/{}",
        retired(&p2, &p3),
        p3.retired_network - p2.retired_network,
        if through.is_ok() { "answered" } else { "FAILED" },
        began.elapsed(),
        word(&phone, &mac_id).await,
        word(&phone, &far_id).await,
    );
    drop(mac_serve);
    drop(far_serve);
}
