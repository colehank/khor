//! Real-connection acceptance for machine readings: vitals are **asked
//! for**, not replicated (`khor_core::Vitals`), so the only way to know
//! they work is to make one machine ask another over real UDP.
//!
//! Two nodes on one machine, real pairing, real sync. Every await is
//! under a timeout — and here the timeouts are not just hygiene, they
//! are the assertion: sampling blocks, and a sample taken on the reactor
//! stalls the very connection its answer travels on.

use std::path::PathBuf;
use std::time::Duration;

use khor_node::Node;
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-vitals-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
}

/// One machine's reading reaches another, and the trip does not stall
/// the connection it rides on.
///
/// **Two failures, one test, and they cannot be separated here.** The
/// interesting one is the second: `vitals::sample` blocks — measured at
/// 1.9 s on the first call in a process, most of it the one-time disk
/// enumeration — and calling it inline in the request handler stalled
/// QUIC long enough that a twenty-second sync timed out. That is what
/// the timeout below catches, and it caught it for real (two unrelated
/// avatar tests went red before it existed). The first failure — no
/// reading arriving at all — is what the assertions catch.
///
/// The default `#[tokio::test]` runtime is single-threaded, and that is
/// deliberate rather than incidental: a blocking call on a multi-threaded
/// runtime may be absorbed by another worker and never show. **The
/// cheapest reactor to stall is the one with a single thread**, so this
/// stays as it is; a `flavor = "multi_thread"` here would quietly turn
/// the guard off.
#[tokio::test]
async fn a_machine_reports_its_readings_without_stalling_the_link() {
    let ra = root("a");
    let rb = root("b");

    let alpha = Node::open_as(ra.clone(), "alpha").unwrap();
    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let _serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let beta = Node::open_as(rb.clone(), "beta").unwrap();

    // The control: before pairing, beta has heard from nobody but
    // itself, so the reading it finds below cannot be one it already
    // had. Without this the assertion after the sync could be satisfied
    // by beta's own row under another name.
    let alpha_id = alpha.device_str().to_owned();
    assert_eq!(
        beta.vitals_of(&alpha_id),
        None,
        "beta must not have a reading for a machine it has never met"
    );

    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(15), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(20), beta.sync_now())
        .await
        .expect("sync must not hang — a sample taken on the reactor is what makes it")
        .unwrap();

    let (v, age) = beta
        .vitals_of(&alpha_id)
        .expect("alpha's reading should have arrived on the sync visit");
    assert!(v.cores >= 1, "a real machine answered: {v:?}");
    assert!(v.mem.total > 0, "a real machine answered: {v:?}");
    // Under a minute, i.e. taken on this visit rather than dug out of
    // something older. Not "== 0": the cache stamps when it was written,
    // and the read happens afterwards.
    assert!(age < 60_000, "the reading should be from this visit, not {age} ms ago");

    // …and beta's own reading is taken now, which is the one place
    // `age_ms` is zero. This is what tells "sampled for you" apart from
    // "remembered from a visit" at the far end, so it is asserted rather
    // than assumed.
    let (_, mine) = beta.vitals_of(beta.device_str()).expect("this machine can always answer");
    assert_eq!(mine, 0, "this machine samples on demand, so its reading has no age");
}
