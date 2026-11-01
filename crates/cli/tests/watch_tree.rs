//! **A far directory tree, pushed** (round NET-4): beta's resident has
//! alpha watch the levels a tree has open (`Node::dirs_begin`/`dirs_wait`/`dirs_leave`), relay-
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
    let rtt: u64 = std::env::var("WATCH_RTT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
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

    // More streams than the ordinary RPC admission pool: held observations
    // must not prevent the next control request from being decoded.
    let mut scopes: Vec<_> = dirs
        .iter()
        .map(|path| khor_node::watch::Scope::Entries { path: path.clone() })
        .collect();
    for i in 0..33 {
        let dir = root.join(format!("control-{i}"));
        std::fs::create_dir_all(&dir).unwrap();
        scopes.push(khor_node::watch::Scope::Entries {
            path: dir.display().to_string(),
        });
    }
    // Initial coverage is pending, followed by an explicit registration fence.
    let began = Instant::now();
    let opened = p
        .beta
        .dirs_begin("alpha", scopes.clone())
        .await
        .expect("resident opens lease");
    let mut first = opened.batch.clone();
    tokio::time::timeout(Duration::from_secs(15), async {
        while first
            .coverage
            .iter()
            .any(|c| c.mode == khor_node::watch::CoverageMode::Pending)
        {
            first = p
                .beta
                .dirs_wait(&opened.machine_id, &opened.id, Some(&first.cursor), 20_000)
                .await
                .unwrap();
        }
    })
    .await
    .expect("all source initialization is bounded");
    assert!(
        first
            .coverage
            .iter()
            .all(|c| c.mode == khor_node::watch::CoverageMode::Native),
        "{first:?}"
    );
    assert!(first.reset, "registration emits a fresh-read fence");
    let second = p.beta.dirs_begin("alpha", scopes.clone()).await.unwrap();
    assert!(
        second
            .batch
            .coverage
            .iter()
            .all(|c| c.mode == khor_node::watch::CoverageMode::Native)
    );
    let started = began.elapsed();

    // A file appears in level 3; beta is waiting.
    let waiting = {
        let (beta, machine, id, cursor) = (
            khor_node::Node::open_as(p.rb.0.clone(), "beta").unwrap(),
            opened.machine_id.clone(),
            opened.id.clone(),
            first.cursor.clone(),
        );
        tokio::spawn(async move { beta.dirs_wait(&machine, &id, Some(&cursor), 20_000).await })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    let made = Instant::now();
    std::fs::write(std::path::Path::new(&dirs[3]).join("new.txt"), b"hello").unwrap();
    let told = waiting.await.unwrap().expect("the wait answers");
    let heard = made.elapsed();
    assert_eq!(
        told.dirs,
        vec![dirs[3].clone()],
        "only the level that changed"
    );

    p.beta
        .dirs_leave(&opened.machine_id, &opened.id)
        .await
        .unwrap();
    assert!(
        p.beta
            .dirs_wait(&opened.machine_id, &opened.id, None, 0)
            .await
            .is_err(),
        "closed leases never recreate a watcher"
    );

    let held = {
        let (beta, machine, id, cursor) = (
            khor_node::Node::open_as(p.rb.0.clone(), "beta").unwrap(),
            second.machine_id.clone(),
            second.id.clone(),
            told.cursor.clone(),
        );
        tokio::spawn(async move { beta.dirs_wait(&machine, &id, Some(&cursor), 20_000).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !held.is_finished(),
        "a native wait really is held during control probes"
    );
    let control = Instant::now();
    let third = p.beta.dirs_begin("alpha", scopes).await.unwrap();
    p.beta
        .dirs_leave(&third.machine_id, &third.id)
        .await
        .unwrap();
    p.beta
        .ls_of("alpha", root.to_str().unwrap())
        .await
        .expect("ordinary RPC remains admitted with 38 watch streams");
    assert!(
        control.elapsed() < Duration::from_secs(3),
        "begin/leave/control were starved"
    );
    std::fs::write(
        std::path::Path::new(&dirs[2]).join("after-one-owner-left.txt"),
        b"still observed",
    )
    .unwrap();
    let survived = held.await.unwrap().unwrap();
    assert!(
        survived.dirs.contains(&dirs[2]),
        "closing one owner keeps the other's source alive"
    );

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
        .dirs_wait(&second.machine_id, &second.id, Some(&survived.cursor), 0)
        .await
        .expect("the resident answers");
    let watched_took = began.elapsed();
    let c2 = p.beta.wire_counts().await.unwrap();
    assert!(now.dirs.is_empty(), "nothing changed since: nothing to ask");

    // An idle minute of watching, on the wire.
    let quiet = 30u64;
    let c3 = p.beta.wire_counts().await.unwrap();
    let idle = p
        .beta
        .dirs_wait(&second.machine_id, &second.id, Some(&now.cursor), 20_000);
    let _ = tokio::time::timeout(Duration::from_secs(quiet), async {
        let _ = idle.await;
        tokio::time::sleep(Duration::from_secs(quiet)).await;
    })
    .await;
    let c4 = p.beta.wire_counts().await.unwrap();
    p.beta
        .dirs_leave(&second.machine_id, &second.id)
        .await
        .unwrap();
    p.beta
        .dirs_leave(&second.machine_id, &second.id)
        .await
        .unwrap();
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
