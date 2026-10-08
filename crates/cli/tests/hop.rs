//! **The road by measured legs, end to end** (NET-DIRECT step 4): three
//! scratch nodes on one machine. Alpha binds UDP on `127.0.0.1` only and
//! gamma on `::1` only (`KHOR_BIND_IP`), so the two share no road and meet
//! on the relay alone; beta binds both wildcards and reaches each directly. The relay sits
//! behind a [`slow_road`], so the relay between alpha and gamma costs a
//! couple of hundred ms while beta's two legs cost about one.
//!
//! Alpha's resident holds its relay wire to gamma against beta's legs
//! every 30 s (`Node::compare_roads`) and opens the hop; the wire then
//! reads 通过 beta. With beta frozen (its leg degraded to nothing), the
//! wire goes back to the relay without ever going down.
//!
//! Loopback relay, temporary homes and identities, own tmux socket, no
//! mDNS; processes by pid.

mod scratch;

use std::time::{Duration, Instant};

use khor_node::{Hop, Node};
use scratch::*;
use tokio::time::timeout;

/// What alpha reads for gamma.
async fn read(alpha: &Node, gamma: &str) -> Option<khor_core::Link> {
    let links = alpha.links().await.ok()?;
    links.into_iter().find(|(id, _)| id == gamma).map(|(_, l)| l)
}

/// Polls `alpha`'s word for `gamma` until `want` holds; every change of
/// word is printed with its time, for the report.
async fn until_word(
    alpha: &Node,
    gamma: &str,
    within: Duration,
    what: &str,
    want: impl Fn(&khor_core::Link) -> bool,
) -> (Duration, khor_core::Link) {
    let started = Instant::now();
    let mut last: Option<(Hop, Option<String>)> = None;
    loop {
        if let Some(l) = read(alpha, gamma).await {
            let now = (l.hop, l.via.clone());
            if last.as_ref() != Some(&now) {
                eprintln!(
                    "[hop] {:>5.1}s {:?} via {:?} rtt {:?} down {:?}",
                    started.elapsed().as_secs_f32(),
                    l.hop,
                    l.via,
                    l.rtt_ms,
                    l.down_ms
                );
                last = Some(now);
            }
            assert_ne!(l.hop, Hop::Ssh, "nothing here rides ssh: a hop that ends reads as the hop, then the relay");
            if want(&l) {
                return (started.elapsed(), l);
            }
        }
        assert!(started.elapsed() < within, "{what} within {within:?}: last read {last:?}");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_hop_through_a_middle_beats_a_slow_relay_and_the_relay_takes_back_when_it_stalls() {
    let (ra, rb, rc, rr) = (scratch("hop3-a"), scratch("hop3-b"), scratch("hop3-c"), scratch("hop3-r"));
    let (_relay, direct_url) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct_url
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let (at, _road) = slow_road(
        to,
        Impair {
            rtt: Duration::from_millis(200),
            loss_pct: 0,
            stall: Duration::ZERO,
        },
    )
    .await;
    let url = format!("http://{at}");
    for r in [&ra, &rb, &rc] {
        home_on(&r.0, &url);
    }
    let _a = serve_with(&ra.0, "alpha", &[(khor_net::endpoint::BIND_IP_ENV, "127.0.0.1")]);
    let b = serve(&rb.0, "beta");
    let _c = serve_with(&rc.0, "gamma", &[(khor_net::endpoint::BIND_IP_ENV, "::1")]);
    stood_up(&[&ra.0, &rb.0, &rc.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    let gamma = Node::open_as(rc.0.clone(), "gamma").unwrap();
    // Every node on the slow relay before anyone is found there.
    tokio::time::sleep(Duration::from_secs(10)).await;
    for joiner in [&beta, &gamma] {
        let ticket = alpha.invite().unwrap();
        let began = Instant::now();
        loop {
            match timeout(Duration::from_secs(90), joiner.pair(&ticket))
                .await
                .expect("pairing must not hang")
            {
                Ok(_) => break,
                Err(why) if began.elapsed() < Duration::from_secs(120) => eprintln!("[pair] {why}"),
                Err(why) => panic!("pairing succeeds: {why}"),
            }
        }
    }
    let ids = [&alpha, &beta, &gamma].map(|n| n.device_str().to_owned());
    let [_, b_id, c_id] = ids.clone();
    let began = Instant::now();
    while ![&alpha, &beta, &gamma]
        .iter()
        .all(|n| n.devices().is_ok_and(|known| ids.iter().all(|id| known.iter().any(|d| &d.id == id))))
    {
        assert!(began.elapsed() < Duration::from_secs(90), "every machine knows the other two");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // The control: alpha's wire to gamma is on the slow relay.
    let (_, on_relay) = until_word(&alpha, &c_id, Duration::from_secs(90), "alpha reaches gamma by the relay", |l| {
        l.hop == Hop::Relay && l.rtt_ms.is_some()
    })
    .await;
    let relay_ms = on_relay.rtt_ms.unwrap();

    // The hop: opened by the 30 s compare once beta's row says it reaches
    // gamma directly.
    let (took, hopped) = until_word(&alpha, &c_id, Duration::from_secs(150), "alpha's wire to gamma goes through beta", |l| {
        l.hop == Hop::Forwarded
    })
    .await;
    assert_eq!(hopped.via.as_deref(), Some(b_id.as_str()), "the middle is beta");
    // Settled: iroh's own round trip on the hop, against the relay's.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let settled = read(&alpha, &c_id).await.expect("a link to gamma");
    eprintln!(
        "[hop] relay {relay_ms} ms; through beta after {took:?}: {:?} ms (settled {:?} ms, {:?})",
        hopped.rtt_ms, settled.rtt_ms, settled.hop
    );
    assert_eq!(settled.hop, Hop::Forwarded, "the hop holds once open");
    assert!(
        settled.rtt_ms.is_some_and(|ms| ms + 10 <= relay_ms),
        "the hop beats the relay by 10 ms: {:?} against {relay_ms}",
        settled.rtt_ms
    );

    // Beta frozen: the hop carries nothing, and the relay takes back the
    // wire it never let go of.
    let pid = serve_pid(&rb.0).expect("beta's resident");
    signal(pid, "-STOP");
    let back = timeout(
        Duration::from_secs(90),
        until_word(&alpha, &c_id, Duration::from_secs(90), "alpha's wire to gamma back on the relay", |l| {
            l.hop == Hop::Relay
        }),
    )
    .await;
    signal(pid, "-CONT");
    let (took_back, _) = back.expect("the relay takes back");
    eprintln!("[hop] back on the relay {took_back:?} after beta stopped");
    let after = read(&alpha, &c_id).await.expect("a link to gamma");
    assert!(after.down_ms.is_none(), "the wire never went down: {after:?}");
    drop(b);
}
