//! **A pull by path, over a slow road, cut and carried on** (round
//! NET-4): beta pulls a file off alpha's disk, relay-only behind a road
//! adding `PULL_RTT_MS` (default 300) of round trip. Timed whole; then
//! pulled again and cut halfway by stopping alpha's resident, which
//! comes back; the next pull carries on from the part file, and how much
//! it had to move is counted. The landed file must be the far file, byte
//! for byte, both times.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_pull_cut_halfway_carries_on_from_its_part_file() {
    let rtt: u64 = std::env::var("PULL_RTT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let (mut p, _road) = pair_on(
        "resume",
        Some(Impair {
            rtt: Duration::from_millis(rtt),
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    let size: usize = 16 << 20;
    let payload: Vec<u8> = (0..size as u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
    let src = p.ra.0.join("big.bin");
    std::fs::write(&src, &payload).unwrap();
    let src = src.display().to_string();

    // Whole.
    let whole = p.rb.0.join("whole");
    std::fs::create_dir_all(&whole).unwrap();
    let began = Instant::now();
    let (moved, dest) = p.beta.pull_path("alpha", &src, &whole).await.expect("a whole pull");
    let whole_took = began.elapsed();
    assert_eq!(moved as usize, size);
    assert_eq!(std::fs::read(&dest).unwrap(), payload);

    // Cut halfway: alpha's resident stops while the part file grows.
    let cut = p.rb.0.join("cut");
    std::fs::create_dir_all(&cut).unwrap();
    let part = cut.join(".khor-pull-big.bin");
    let pulling = {
        let (beta, src, cut) = (khor_node::Node::open_as(p.rb.0.clone(), "beta").unwrap(), src.clone(), cut.clone());
        tokio::spawn(async move { beta.pull_path("alpha", &src, &cut).await })
    };
    while std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0) < (size / 2) as u64 {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let old = serve_pid(&p.ra.0).unwrap();
    drop(p.a.take());
    let first = pulling.await.unwrap();
    assert!(first.is_err(), "a pull whose far end stopped does not land: {first:?}");
    let had = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    assert!(had > 0, "the part file stays, to carry on from");
    p.a = Some(serve_with(&p.ra.0, "alpha", &[("KHOR_RELAY_ONLY", "1")]));
    tokio::time::timeout(Duration::from_secs(60), async {
        while serve_pid(&p.ra.0).is_none_or(|pid| pid == old) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("alpha comes back");

    let began = Instant::now();
    let (moved, dest) = tokio::time::timeout(Duration::from_secs(180), p.beta.pull_path("alpha", &src, &cut))
        .await
        .expect("the carried-on pull must not hang")
        .expect("the pull carries on");
    let rest_took = began.elapsed();
    assert_eq!(std::fs::read(&dest).unwrap(), payload, "byte for byte the far file");
    assert!(
        (moved as usize) < size,
        "carried on, not started over: moved {moved} of {size}"
    );
    eprintln!(
        "[pull] added RTT {rtt} ms, {} MiB by path: whole in {whole_took:?} ({:.2} MB/s); cut at {} MiB, \
         the next pull moved {} MiB in {rest_took:?}",
        size >> 20,
        size as f64 / whole_took.as_secs_f64() / 1e6,
        had >> 20,
        moved >> 20,
    );
}
