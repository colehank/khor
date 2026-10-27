//! **How steady the machine row's latency reads** (round NET-8): two
//! scratch machines, relay-only through a road adding `DIAG_RTT_MS`
//! (default 200) and stalling `DIAG_LOSS_PCT` (default 2) of chunks by
//! one round trip. Beta's resident is asked for its links every 5 s, the
//! pace a face's list refreshes at, for `DIAG_WATCH_SECS` (default 120)
//! while the pump keeps the wire in use; what `rtt_ms` read each time,
//! and how far apart neighbouring samples were.
//!
//! `cargo test -p khor-cli --test rtt_probe -- --ignored --nocapture`.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a probe over a simulated road: run by hand"]
async fn the_rows_latency_over_five_second_samples() {
    let rtt = Duration::from_millis(env_u64("DIAG_RTT_MS", 200));
    let loss = env_u64("DIAG_LOSS_PCT", 2) as u32;
    let watch = Duration::from_secs(env_u64("DIAG_WATCH_SECS", 120));
    let (p, _road) = pair_on(
        "rtt",
        Some(Impair {
            rtt,
            loss_pct: loss,
            stall: rtt,
        }),
    )
    .await;
    let alpha_id = p.alpha.device_str().to_owned();
    let tmp = std::env::temp_dir().display().to_string();
    // A wire up before the first sample.
    let _ = p.beta.ls_of("alpha", &tmp).await;
    let began = Instant::now();
    let mut samples: Vec<Option<u32>> = Vec::new();
    while began.elapsed() < watch {
        let r = p
            .beta
            .links()
            .await
            .ok()
            .and_then(|l| l.into_iter().find(|(id, _)| *id == alpha_id))
            .and_then(|(_, l)| l.rtt_ms);
        samples.push(r);
        // A listing now and then, as a face with a machine open does.
        if samples.len() % 3 == 0 {
            let _ = p.beta.ls_of("alpha", &tmp).await;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    let got: Vec<u32> = samples.iter().flatten().copied().collect();
    let (min, max) = (got.iter().min().copied(), got.iter().max().copied());
    let mean = got.iter().map(|&v| f64::from(v)).sum::<f64>() / got.len().max(1) as f64;
    let steps: Vec<u32> = got.windows(2).map(|w| w[0].abs_diff(w[1])).collect();
    let mut sorted = steps.clone();
    sorted.sort_unstable();
    let median_step = sorted.get(sorted.len() / 2).copied();
    eprintln!(
        "[rtt] road {}ms {loss}%: {} samples ({} none), min {min:?} max {max:?} mean {mean:.0}, \
         step between samples median {median_step:?} max {:?}; series {:?}",
        rtt.as_millis(),
        samples.len(),
        samples.iter().filter(|s| s.is_none()).count(),
        sorted.last(),
        samples
    );
}
