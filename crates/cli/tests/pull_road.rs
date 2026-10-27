//! **A whole file over a slow road** (round NET-2, piece 6): alpha
//! offers a file and beta takes it, then beta previews another of
//! alpha's files whole — both relay-only behind a road that adds
//! `PULL_RTT_MS` (default 300) of round trip (`scratch::slow_road`).
//! Timed, so a pull that asks one slice per round trip shows as one.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_file_pulls_at_the_roads_pace_not_a_slice_per_round_trip() {
    let rtt: u64 = std::env::var("PULL_RTT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let (p, _road) = pair_on(
        "pull",
        Some(Impair {
            rtt: Duration::from_millis(rtt),
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;

    // Eight MiB offered in the broadcast and taken: the clock covers the
    // post, the sync and the pull together.
    let payload: Vec<u8> = (0..8u32 << 20).map(|i| (i % 251) as u8).collect();
    let src = p.ra.0.join("big.bin");
    std::fs::write(&src, &payload).unwrap();
    let began = Instant::now();
    let id = tokio::time::timeout(
        Duration::from_secs(120),
        p.alpha.post("", std::slice::from_ref(&src)),
    )
    .await
    .expect("a post must not hang")
    .unwrap();
    let tid = khor_node::transfer::TransferKind::session_id(&id);
    tokio::time::timeout(Duration::from_secs(120), p.beta.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let (_, landed) = tokio::time::timeout(Duration::from_secs(300), p.beta.take(&tid))
        .await
        .expect("take must not hang")
        .unwrap();
    let transfer = began.elapsed();
    assert_eq!(std::fs::read(&landed[0]).unwrap(), payload);

    // Four MiB read whole, the way a preview reads.
    let view = p.ra.0.join("view.bin");
    std::fs::write(&view, &payload[..4 << 20]).unwrap();
    let began = Instant::now();
    let (total, _, bytes, cut) = tokio::time::timeout(
        Duration::from_secs(300),
        p.beta
            .read_all("alpha", &view.display().to_string(), 4 << 20),
    )
    .await
    .expect("a preview must not hang")
    .unwrap();
    let preview = began.elapsed();
    assert_eq!((total, bytes.len(), cut), (4 << 20, 4 << 20, false));
    assert_eq!(&bytes[..], &payload[..4 << 20]);

    let mb_s = |b: u64, d: Duration| b as f64 / d.as_secs_f64() / 1e6;
    eprintln!(
        "[pull] added RTT {rtt} ms: 8 MiB transfer in {transfer:?} ({:.2} MB/s), \
         4 MiB preview in {preview:?} ({:.2} MB/s)",
        mb_s(8 << 20, transfer),
        mb_s(4 << 20, preview),
    );
}
