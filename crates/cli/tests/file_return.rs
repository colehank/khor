//! **Coming back to a window with a far file open** (round NET-4):
//! what it costs to learn whether the file changed. Before, the face
//! read the file again (one slice, up to 256 KiB) and compared digests;
//! now it asks `Stat` and reads only when that answer moved. Relay-only
//! behind a road adding `RETURN_RTT_MS` (default 200) of round trip, a
//! 200 KB text file: the UDP bytes beta's wire carries per check, and
//! the time, for each.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_window_return_asks_what_the_file_is_not_what_it_says() {
    let rtt: u64 = std::env::var("RETURN_RTT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let (p, _road) = pair_on(
        "return",
        Some(Impair {
            rtt: Duration::from_millis(rtt),
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    let text: String = (0..4000)
        .map(|i| format!("line {i} of a file somebody is reading\n"))
        .collect();
    let file = p.ra.0.join("notes.md");
    std::fs::write(&file, &text).unwrap();
    let file = file.display().to_string();
    let (_, _, first) = p
        .beta
        .read_head("alpha", &file)
        .await
        .expect("a first read");
    let held = khor_node::files::digest(&first);

    let n = 8u32;
    let measure = |label: &'static str,
                   c0: khor_core::WireCounts,
                   c1: khor_core::WireCounts,
                   took: Duration| {
        eprintln!(
            "[return] added RTT {rtt} ms, a {} KB file, {label}: per check {:.0} B out + {:.0} B in, {:?}",
            text.len() / 1000,
            (c1.sent_bytes - c0.sent_bytes) as f64 / f64::from(n),
            (c1.received_bytes - c0.received_bytes) as f64 / f64::from(n),
            took / n,
        );
    };

    let c0 = p.beta.wire_counts().await.unwrap();
    let began = Instant::now();
    for _ in 0..n {
        let (_, _, again) = p.beta.read_head("alpha", &file).await.expect("a read");
        assert_eq!(khor_node::files::digest(&again), held);
    }
    let took = began.elapsed();
    let c1 = p.beta.wire_counts().await.unwrap();
    measure("read again (before)", c0, c1, took);

    let c0 = p.beta.wire_counts().await.unwrap();
    let began = Instant::now();
    for _ in 0..n {
        let (_, _, digest) = p.beta.stat_of("alpha", &file).await.expect("a stat");
        assert_eq!(digest, held, "unchanged: the same digest, no bytes");
    }
    let took = began.elapsed();
    let c1 = p.beta.wire_counts().await.unwrap();
    measure("stat (now)", c0, c1, took);

    // And a change is seen.
    std::fs::write(&file, format!("{text}one more line\n")).unwrap();
    let (_, _, digest) = p.beta.stat_of("alpha", &file).await.expect("a stat");
    assert_ne!(digest, held, "a changed file answers a different digest");
}
