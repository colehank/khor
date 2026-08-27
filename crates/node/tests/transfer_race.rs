//! The accept/sync race, and the two answers a miss now has.
//!
//! `khor send` writes the summary into the CRDT here; `khor accept`
//! reads it over there. Between those two the id is real and the
//! summary has not landed, and accept used to refuse it with the very
//! words it uses for an id that does not exist — so the only way past
//! was to guess which of the two had happened and run it again ten
//! seconds later (measured twice, 2026-08-26 空运升级).
//!
//! **The bug was already written down here, in green ink.** Every other
//! transfer test calls `b.sync_now()` in the one gap between `a.send`
//! and `b.accept` — `transfer.rs` twice, `ipc.rs`, `act.rs` — and that
//! call is not setup, it is the workaround: it is the human "wait ten
//! seconds and try again", spelled as an API call. Those tests are
//! right to keep it (they are about the payload, the resume and the
//! digest, and they want the summary already there), so nothing about
//! them changes; what changes is that the gap they step over now has a
//! test standing in it.
//!
//! **The race is reproduced without timing.** beta pairs from a
//! one-shot endpoint, so it reports no roads, so alpha's pump can never
//! push to it (`sync_device` refuses to dial a device with no roads).
//! Until beta asks, beta does not have the offer — not "probably not
//! yet", never. That is what makes phase 1 a gate rather than a coin
//! toss: it goes red on every run against the old accept.
//!
//! **And the race is witnessed, not assumed.** A precondition checked
//! only as an absence proves nothing — an absence is what a broken
//! probe returns too. So the same probe is asked about the same id on
//! both machines at the same instant: alpha must say yes while beta
//! says no. That is the race itself, observed; and after the accept
//! beta must say yes as well, which is what rules out a probe that
//! only ever says no.
//!
//! The probe is `Node::transfer_is_here` and not `sessions()`, and that
//! is not a style choice: since 传输并入对话批 (2026-08-23)
//! `TransferKind::rows` yields nothing, so "no row with this id" holds
//! for every id ever, and an absence asserted that way is a green that
//! cannot go red.
//!
//! The controls are the other half of the claim: the same made-up id,
//! asked on two machines, must come back with two different answers —
//! "absent" where every peer answered, "not here yet" where one did
//! not. A fix that merely retried would pass phase 1 and fail these.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_node::{Node, SessionId};
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-xrace-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// 90s, not 10, and multi_thread below for the same reason: on a Mac
/// operated over ssh a freshly compiled test binary is a new face to
/// macOS, and its first network-stack touch blocks 20-40s per process
/// on an authorization prompt nobody can click (#73). That hang is
/// synchronous, which on a current_thread runtime freezes tokio's clock
/// and silently stretches every timeout in here.
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

/// A made-up id of the right shape. Its shape is deliberately
/// plausible: accept must reach its verdict from the network, not from
/// a format check it would have to keep in step with `khor_sync::chat`.
fn ghost_id() -> SessionId {
    SessionId("transfer/00000000000000ff-1999999999-0".to_owned())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn accept_waits_out_the_sync_axis_and_says_which_kind_of_miss_it_hit() {
    let ra = root("a");
    let rb = root("b");
    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move {
        // Reported rather than swallowed: a serve that refuses to start
        // shows up downstream as "endpoint.json never appeared", which
        // reads like a slow machine and is not.
        if let Err(why) = server.serve().await {
            eprintln!("serve refused to start: {why}");
        }
    });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // ── phase 1: accept runs before the summary has been replicated ──
    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let src = ra.join("aloft.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src).unwrap();

    // The race, witnessed. One probe, one id, one instant, two
    // machines: alpha holds the offer and beta does not. The yes is
    // what makes the no worth anything — without it, "not here" is
    // equally the answer of a probe that cannot find anything at all.
    assert!(
        a.transfer_is_here(&tid),
        "the offer must be on alpha, or the probe below proves nothing"
    );
    assert!(
        !b.transfer_is_here(&tid),
        "the offer must NOT have reached beta yet, or phase 1 tests nothing"
    );
    // Nothing here asks `sessions()` whether the row is absent. Since
    // 传输并入对话批 (2026-08-23) `TransferKind::rows` returns nothing at
    // all, so "no row with this id" is true of every id, before and
    // after — a green that cannot go red, dressed as a precondition.

    // No sync_now here on purpose. Every other transfer test calls one
    // at this exact point — that call *is* the workaround this replaces.
    let (moved, landed) = timeout(Duration::from_secs(90), b.accept(&tid))
        .await
        .expect("accept must not hang while it waits out the sync axis")
        .expect("accept must wait for the offer, not refuse it");
    assert_eq!(moved, payload.len() as u64, "the whole payload should move");
    let [landing] = landed.as_slice() else {
        panic!("one file offered, one landing expected, got {landed:?}");
    };
    assert_eq!(
        fs::read(landing).unwrap(),
        payload,
        "bytes verbatim on the recipient"
    );
    // The other half of the witness: the probe that said no can say
    // yes, and the state it was reading really moved.
    assert!(
        b.transfer_is_here(&tid),
        "beta must hold the offer now — otherwise the earlier no was the probe, not the race"
    );

    // ── phase 2: every peer answered, and it is still not there ──
    // beta can reach alpha (alpha serves, and its roads are in the
    // table), so beta's round comes back complete: the id is absent,
    // and khor is entitled to say so.
    let absent = timeout(Duration::from_secs(90), b.accept(&ghost_id()))
        .await
        .expect("a refused accept must not hang either")
        .unwrap_err();
    assert_eq!(
        absent,
        khor_catalog::msg::transfer_absent_after_sync(&ghost_id().0),
        "with every peer answering, a missing id is missing"
    );

    // ── phase 3: the same id, on a machine that cannot rule it out ──
    // alpha's only peer is beta, which reported no roads at pairing and
    // therefore cannot be dialled at all. alpha did not learn anything
    // about this id, and must not pretend it did.
    let pending = timeout(Duration::from_secs(90), a.accept(&ghost_id()))
        .await
        .expect("a refused accept must not hang either")
        .unwrap_err();
    assert!(
        pending.contains("beta"),
        "the machine khor could not reach must be named: {pending}"
    );
    assert_eq!(
        pending,
        khor_catalog::msg::transfer_not_synced_yet(&ghost_id().0, 1, "beta"),
        "an unreachable peer means khor does not know, not that it is not there"
    );

    // The gate on the pair itself. Merging the two answers back into
    // one sentence is precisely the bug this batch closed, and neither
    // phase above would catch it alone: each only checks that its own
    // side reads right.
    assert_ne!(absent, pending, "the two misses must not collapse into one word again");

    serve.abort();
    for r in [&ra, &rb] {
        let _ = fs::remove_dir_all(r);
    }
}
