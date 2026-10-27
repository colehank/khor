//! **What a machine card open on a far machine costs** (round NET-4):
//! beta asks alpha's `Load` (the top 200 and the last day) the way the
//! card does, relay-only behind a road adding `LOAD_RTT_MS` (default
//! 200) of round trip. Alpha's day is seeded full — five series of 288
//! buckets, a day of a busy machine — so the day weighs what it does on
//! a real one. Measured: the UDP bytes beta's wire carries per ask
//! (`WireCounts`, both ways) and the time per ask.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

/// A full day in the recorder's own file format (`history::CpuHistory`):
/// one line per five-minute bucket, sums over five readings.
fn seed_day(home: &std::path::Path) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let newest = now / (5 * 60 * 1000);
    let mut text = String::new();
    for i in 0..287u64 {
        let k = newest - 287 + i;
        let f = |base: f64| base + (i % 17) as f64 * 1.3;
        text.push_str(&format!(
            "{k} {} 5 {} {} {} {}\n",
            f(100.0),
            f(200.0),
            f(50.0),
            f(300.0),
            f(120.0)
        ));
    }
    let dir = home.join(".khor").join("history");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cpu"), text).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_card_on_a_far_machine_gets_the_day_once() {
    let rtt: u64 = std::env::var("LOAD_RTT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let (p, _road) = pair_on(
        "loadday",
        Some(Impair {
            rtt: Duration::from_millis(rtt),
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    seed_day(&p.ra.0);
    let ask = || {
        p.beta
            .load_on("alpha", 200, khor_core::TopBy::Busiest, true)
    };
    let first = ask().await.expect("a first load");
    assert!(
        first.cpu_day.buckets.iter().filter(|b| b.is_some()).count() >= 280,
        "the seeded day crossed"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;

    let n = 10;
    let c0 = p.beta.wire_counts().await.unwrap();
    let began = Instant::now();
    for _ in 0..n {
        let load = ask().await.expect("a load");
        assert!(
            load.cpu_day.buckets.iter().filter(|b| b.is_some()).count() >= 280,
            "every answer the card gets has the whole day"
        );
    }
    let took = began.elapsed() / n;
    let c1 = p.beta.wire_counts().await.unwrap();
    let per = |a: u64, b: u64| (b - a) as f64 / n as f64;
    eprintln!(
        "[load] added RTT {rtt} ms, top 200 and a full day: per ask {:.0} B out + {:.0} B in, {took:?}",
        per(c0.sent_bytes, c1.sent_bytes),
        per(c0.received_bytes, c1.received_bytes),
    );
}
