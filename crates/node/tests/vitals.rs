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
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// One machine's reading reaches another, and the trip does not stall
/// the connection it rides on.
///
/// **Two failures, one test.** The interesting one is the second:
/// `vitals::sample` blocks — measured at 1.9 s on the first call in a
/// process, most of it the one-time disk enumeration — and calling it
/// inline in the request handler stalled QUIC long enough that a
/// twenty-second sync timed out. It caught that for real (two unrelated
/// avatar tests went red before it existed). The first failure — no
/// reading arriving at all — is what the assertions below catch.
///
/// # The guard no longer watches a clock, because a clock cannot see it
///
/// It used to be the sync's own twenty-second timeout. **A wall clock
/// measures a block and a wait added together**, so anything slow read
/// as the bug — and on this machine something slow really did arrive:
/// #73's first-touch authorization stall (20-40 s, nothing to do with
/// khor) landed on that very sync. Measured 2026-08-21: bind 21 ms,
/// dial 1.1 s, the guarded sync 37 s, and nothing in the tree could say
/// which of the two the 37 s was. Widening the timeout would have
/// buried both.
///
/// So the guard is now [`khor_node::reactor`], which measures the
/// property itself: the worst gap between ticks on the serve's runtime.
/// A synchronous block starves the ticker; an `await` does not. That is
/// the line the clock could not draw, and it is **sharper** — a stall
/// of one second fails here, where the old shape needed twenty.
///
/// **What it no longer watches**, said out loud: whether the sync
/// finishes at all. An unreachable peer, a route that vanished, a
/// handler that awaits forever — those are caught by the ninety-second
/// wall clock below, which is now hygiene rather than the guard, and
/// they are no longer reported as "a sample stalled the reactor".
///
/// The default `#[tokio::test]` runtime is single-threaded, and that is
/// deliberate rather than incidental: a blocking call on a multi-threaded
/// runtime may be absorbed by another worker and never show. **The
/// cheapest reactor to stall is the one with a single thread**, so this
/// stays as it is — the watch above sees nothing on any other flavour,
/// so this design **keeps** that judgment rather than overturning it.
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
    timeout(Duration::from_secs(60), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    // The measurement window opens here: everything before it —
    // binding, pairing, and whatever #73 charged for the process's
    // first touch of the network stack — is somebody else's stall and
    // none of this test's business.
    khor_node::reactor::forget();
    timeout(Duration::from_secs(90), beta.sync_now())
        .await
        .expect("the sync must finish — hygiene, not the guard (module head)")
        .unwrap();
    // **The guard.** One second, because the bug's own signature is a
    // single 1.9 s sample and this has to sit below it with room, while
    // staying well above what an ordinary scheduling hiccup costs on a
    // busy machine.
    let stalled = khor_node::reactor::worst_stall_ms();
    assert!(
        stalled < 1000,
        "the serve's reactor was unavailable for {stalled} ms — a handler is blocking it \
         (the sample belongs on a blocking thread, not inline)"
    );

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
