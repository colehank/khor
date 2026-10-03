//! **What a redial costs** (round NET-3): two machines that reach each
//! other only through a relay behind a road adding `REDIAL_RTT_MS`
//! (default 200) of round trip. Beta's resident lets its wires to alpha
//! go (`Node::let_go_of`), and the first answer afterwards is timed —
//! for a read (a listing) and for a write (a new directory), `REDIAL_N`
//! (default 5) times each. The road's own round trip is printed beside
//! them, measured on a held wire, so the redial's cost reads in round
//! trips.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn the_first_answer_after_a_redial() {
    let env = |k: &str, d: u64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let rtt = Duration::from_millis(env("REDIAL_RTT_MS", 200));
    let n = env("REDIAL_N", 5) as usize;
    let (p, _road) = pair_on(
        "redial",
        Some(Impair {
            rtt,
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    let dir = p.ra.0.join("tree");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.display().to_string();
    // Warm: a wire, and a ticket for the next one.
    p.beta.ls_of("alpha", &dir).await.expect("a first listing");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut held = Vec::new();
    for _ in 0..n {
        let began = Instant::now();
        p.beta.ls_of("alpha", &dir).await.expect("a listing on a held wire");
        held.push(began.elapsed());
    }
    let (mut read, mut write) = (Vec::new(), Vec::new());
    for i in 0..n {
        p.beta.let_go_of("alpha").await.expect("the resident lets go");
        let began = Instant::now();
        p.beta.ls_of("alpha", &dir).await.expect("a listing after a redial");
        read.push(began.elapsed());
        tokio::time::sleep(Duration::from_secs(1)).await;

        p.beta.let_go_of("alpha").await.expect("the resident lets go");
        let began = Instant::now();
        p.beta
            .mkdir("alpha", &format!("{dir}/made-{i}"))
            .await
            .expect("a directory after a redial");
        write.push(began.elapsed());
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    eprintln!(
        "[redial] added RTT {} ms: a listing on a held wire {:?}; after a redial, a listing {:?} (each {:?}), a new directory {:?} (each {:?})",
        rtt.as_millis(),
        median(held),
        median(read.clone()),
        read,
        median(write.clone()),
        write,
    );
}
