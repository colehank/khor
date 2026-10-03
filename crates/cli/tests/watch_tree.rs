//! **A far directory tree, pushed** (round NET-4): beta's resident has
//! alpha watch the levels a tree has open (`Node::dirs_changed`), relay-
//! only behind a road adding `WATCH_RTT_MS` (default 200) of round trip.
//! Measured:
//!
//! - how long after a file appears on alpha beta hears which directory
//!   changed, without asking;
//! - what a tree costs on coming back to it (the remount the face does
//!   when a person walks back): before, every open level asked again
//!   (`Ls` over the road); now, one question to the resident, which
//!   answers from the watch without crossing the road at all;
//! - what an idle minute of watching costs on the wire.

mod scratch;

use std::time::{Duration, Instant};

use scratch::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_far_tree_is_told_what_changed_and_a_remount_asks_nothing() {
    let rtt: u64 = std::env::var("WATCH_RTT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
    let (p, _road) = pair_on(
        "watch",
        Some(Impair {
            rtt: Duration::from_millis(rtt),
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    // Five open levels, two hundred names each.
    let root = p.ra.0.join("tree");
    let dirs: Vec<String> = (0..5)
        .map(|i| {
            let d = root.join(format!("level{i}"));
            std::fs::create_dir_all(&d).unwrap();
            for f in 0..200 {
                std::fs::write(d.join(format!("file-{f:03}.txt")), b"x").unwrap();
            }
            d.display().to_string()
        })
        .collect();

    // The first answer: every level is news to a face that holds nothing.
    let began = Instant::now();
    let first = p
        .beta
        .dirs_changed("alpha", &dirs, 0, Duration::from_secs(20))
        .await
        .expect("the resident answers");
    assert!(!first.unavailable, "alpha can watch");
    assert_eq!(first.dirs.len(), dirs.len());
    let started = began.elapsed();

    // A file appears in level 3; beta is waiting.
    let waiting = {
        let (beta, dirs, since) = (
            khor_node::Node::open_as(p.rb.0.clone(), "beta").unwrap(),
            dirs.clone(),
            first.seq,
        );
        tokio::spawn(async move { beta.dirs_changed("alpha", &dirs, since, Duration::from_secs(20)).await })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    let made = Instant::now();
    std::fs::write(std::path::Path::new(&dirs[3]).join("new.txt"), b"hello").unwrap();
    let told = waiting.await.unwrap().expect("the wait answers");
    let heard = made.elapsed();
    assert_eq!(told.dirs, vec![dirs[3].clone()], "only the level that changed");

    // Coming back to the tree: before, every open level asked again.
    let c0 = p.beta.wire_counts().await.unwrap();
    let began = Instant::now();
    for d in &dirs {
        p.beta.ls_of("alpha", d).await.expect("a listing");
    }
    let asked_took = began.elapsed();
    let c1 = p.beta.wire_counts().await.unwrap();
    // Now: one question to the resident.
    let began = Instant::now();
    let now = p
        .beta
        .dirs_changed("alpha", &dirs, told.seq, Duration::ZERO)
        .await
        .expect("the resident answers");
    let watched_took = began.elapsed();
    let c2 = p.beta.wire_counts().await.unwrap();
    assert!(now.dirs.is_empty(), "nothing changed since: nothing to ask");

    // An idle minute of watching, on the wire.
    let quiet = 30u64;
    let c3 = p.beta.wire_counts().await.unwrap();
    let idle = p.beta.dirs_changed("alpha", &dirs, now.seq, Duration::from_secs(20));
    let _ = tokio::time::timeout(Duration::from_secs(quiet), async {
        let _ = idle.await;
        tokio::time::sleep(Duration::from_secs(quiet)).await;
    })
    .await;
    let c4 = p.beta.wire_counts().await.unwrap();
    let bytes = |a: &khor_core::WireCounts, b: &khor_core::WireCounts| {
        (b.sent_bytes - a.sent_bytes) + (b.received_bytes - a.received_bytes)
    };
    eprintln!(
        "[watch] added RTT {rtt} ms, {} levels of 200: first answer {started:?}; a new file heard in {heard:?}; \
         coming back to the tree: asked again {} B in {asked_took:?}, from the watch {} B in {watched_took:?}; \
         {quiet} s of quiet (pump included) {} B",
        dirs.len(),
        bytes(&c0, &c1),
        bytes(&c1, &c2),
        bytes(&c3, &c4),
    );
}
