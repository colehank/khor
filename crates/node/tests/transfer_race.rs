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
//! **And the race is witnessed, not assumed.** The rule this file is
//! built on: **a negative assertion must be paired with a positive one
//! that would go red** — an absence is also what a probe that can find
//! nothing returns, and the two read identically in a passing run. So
//! the same probe is asked about the same id on both machines at the
//! same instant: alpha must say yes while beta says no. That is the
//! race itself, observed. And after the accept beta must say yes too,
//! which is what rules out a probe that only ever says no.
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
//!
//! # What this costs, and one wrong diagnosis worth keeping
//!
//! Measured 2026-08-27 on this machine: each real-connection binary in
//! this crate runs about 100 seconds (`transfer` 106s, `act` 102s,
//! `ipc` 96s, this one 99s), so "just run all the integration tests" is
//! **about seven minutes holding the cargo artifact lock** — the whole
//! machine's, not this shell's. Worth knowing before starting it.
//!
//! **The routed test below was a coin toss for an afternoon, and the
//! reason was not what it looked like.** It went red about half the
//! time, in setup, wearing two different faces — `连不上 beta(超时)` and
//! 「这台机器不在机器表里,先配对」 — and the obvious suspect was the two
//! tests in this file running at once (they do, by default) and
//! crowding each other's first touch of the network stack (#73). That
//! suspicion cost a round of measurement and was wrong: both faces are
//! **one defect in this file's own setup**, and the two-machine hops it
//! got wrong are written out beside the pairing below. Neither depends
//! on threading, and neither of the two green runs that seemed to
//! exonerate serial execution meant anything.
//!
//! Left running in parallel on purpose, because there is now no
//! measured reason not to: machinery kept for a refuted diagnosis is
//! worse than none — it explains away the next failure of the same
//! shape.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_node::{Node, SessionId};
use tokio::time::timeout;

/// A scratch tree that **takes itself away however the test leaves**
/// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
/// cleanup written at the end of a test is skipped by the assertion
/// that fails, so a red run leaks a directory every time it is red).
/// A copy per file, which is this repo's convention for test helpers —
/// it costs no dev-dependency.
struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-xrace-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// 90s, not 10, and multi_thread below for the same reason: on a Mac
/// operated over ssh a freshly compiled test binary is a new face to
/// macOS, and its first network-stack touch blocks 20-40s per process
/// on an authorization prompt nobody can click (#73). That hang is
/// synchronous, which on a current_thread runtime freezes tokio's clock
/// and silently stretches every timeout in here.
async fn wait_for_endpoint_file(root: &Path) {
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
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move {
        // Reported rather than swallowed: a serve that refuses to start
        // shows up downstream as "endpoint.json never appeared", which
        // reads like a slow machine and is not.
        if let Err(why) = server.serve().await {
            eprintln!("serve refused to start: {why}");
        }
    });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
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
    let began = std::time::Instant::now();
    let pending = timeout(Duration::from_secs(90), a.accept(&ghost_id()))
        .await
        .expect("a refused accept must not hang either")
        .unwrap_err();
    // **A machine that reported no roads costs nothing, and that is a
    // premise, not a detail.** The whole worst case quoted to the person
    // before this wait (`cli::syncing-for-offer` counts only machines
    // with roads) rests on `sync_device` refusing to dial one that has
    // none. If that early return ever goes, this round starts paying a
    // full dial budget for beta and the printed number becomes a lie —
    // the kind that is worse than printing nothing, because people wait
    // by it. Measured in milliseconds; the bar is one whole budget, so
    // the only way past it is that the early return is gone.
    let budget = Duration::from_secs(khor_node::link::dial_budget_seconds());
    assert!(
        began.elapsed() < budget,
        "a roadless peer must not be dialled: the round took {:?}, a dial budget is {budget:?}",
        began.elapsed()
    );
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
    assert_ne!(
        absent, pending,
        "the two misses must not collapse into one word again"
    );

    serve.abort();
}

/// The routed half: **the machine that presses 收下 is the one that is
/// behind**, and the file is not even addressed to it.
///
/// This is the phone-shaped case and the realistic one — a device that
/// has been asleep approves a transfer between two machines that were
/// not. It reaches `accept_with`'s wait rather than `accept_local`'s,
/// and it is the only path on which the wait has to finish before khor
/// even knows *which machine to route to*: the recipient's name is read
/// off the offer.
///
/// Deterministic for the same reason as the first test, applied to a
/// different seat: gamma pairs from a one-shot endpoint and never
/// serves, so nothing syncs it but its own calls.
///
/// **The recipient-is-behind variant is deliberately absent, and it is
/// worth saying why rather than leaving a hole.** A recipient must be
/// serving to answer a routed Act at all, and a serving machine pumps
/// every `SYNC_EVERY` — so its window of being behind is bounded by the
/// pump and cannot be held open from a test without freezing it. The
/// guard for that case is in the code (`accept_local` waits too) and it
/// costs nothing on the way through; what cannot be had is a
/// deterministic test, and a flaky one here would be worse than none.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_routed_accept_waits_for_the_offer_before_it_knows_where_to_route() {
    let ra = root("ra");
    let rb = root("rb");
    let rg = root("rg");
    let alpha = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move {
        if let Err(why) = alpha.serve().await {
            eprintln!("alpha's serve refused to start: {why}");
        }
    });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();

    // beta serves: a routed accept lands on the recipient's serve.
    let beta = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let ticket_for_beta = a.invite().unwrap();
    timeout(Duration::from_secs(45), beta.pair(&ticket_for_beta))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let serve_b = tokio::spawn(async move {
        if let Err(why) = beta.serve().await {
            eprintln!("beta's serve refused to start: {why}");
        }
    });
    wait_for_endpoint_file(&rb).await;
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();

    // **gamma pairs with beta, not with alpha, and the reason is that
    // one of them is a coin toss.**
    //
    // 配对任意一台即入网 (docs/NET.md) is true *after* the table
    // replicates, and replicating takes a round — while the join gate is
    // absolute in the meantime: a machine answers nobody outside its own
    // table. Pair gamma to alpha and beta learns of it only on some
    // later pump, so the routed accept comes back
    // 「这台机器不在机器表里,先配对」 — from a machine that did pair.
    // (Measured both ways today; that refusal is what a coin-toss red
    // looks like here.)
    //
    // Pairing writes the newcomer into the ticket issuer's table
    // synchronously, so pairing gamma to **beta** — the machine that
    // must answer the routed action — makes the one hop that matters
    // deterministic instead of hoping for a pump.
    let g = Node::open_as(rg.to_path_buf(), "gamma").unwrap();
    timeout(Duration::from_secs(45), g.pair(&b.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    // The other hop, made the same way: gamma will sync with alpha on
    // its way to the offer, and alpha's gate would refuse a machine it
    // has not heard of yet. beta carries the news, because beta is
    // already in alpha's table and can dial it — alpha cannot dial beta
    // (beta paired from a one-shot endpoint and reported no roads, so
    // `sync_device` will not even try), which is exactly why the earlier
    // shape of this setup asked the wrong machine to sync.
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    // **Joining through beta is a request, not a membership** (docs/
    // CONNECT.md 根不在场): alpha made the root when it first opened and
    // beta does not hold it, so gamma waits until alpha approves it —
    // until then every machine refuses gamma by name. alpha has gamma's
    // row now (the sync above), approves, and beta takes the new roster
    // on one more round, which is the only way beta learns of it.
    a.approve_here("gamma").expect("alpha holds the root and sees gamma waiting");
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    let payload: Vec<u8> = (0..120_000u32).map(|i| (i % 241) as u8).collect();
    let src = ra.join("routed.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src).unwrap();

    // The witness, same shape as the first test: the offer exists where
    // it was written, and the machine about to approve it has never
    // heard of it. Pairing carries the device table, not the
    // conversations, so gamma is behind by construction.
    assert!(a.transfer_is_here(&tid), "the offer must be on alpha");
    assert!(
        !g.transfer_is_here(&tid),
        "gamma must be behind, or this test routes an offer it already had"
    );

    let (moved, landed) = timeout(Duration::from_secs(120), g.accept(&tid))
        .await
        .expect("a routed accept must not hang while it waits out the sync axis")
        .expect("a routed accept must wait for the offer, not refuse it");
    assert_eq!(moved, payload.len() as u64, "the whole payload should move");
    assert!(
        g.transfer_is_here(&tid),
        "gamma must hold the offer now — otherwise its earlier no was the probe"
    );

    // The pull ran on beta, off alpha, and the path that came back is
    // beta's own — gamma has no such file and no such directory, so
    // comparing the travelled answer against beta's disk is the only way
    // to tell it from a path gamma computed under its own root.
    let [landing] = landed.as_slice() else {
        panic!("one file offered, one landing expected, got {landed:?}");
    };
    assert!(
        !landing.starts_with(rg.to_str().unwrap()),
        "the payload must not land on the machine that merely asked: {landing}"
    );
    assert_eq!(
        fs::read(landing).unwrap(),
        payload,
        "bytes verbatim on the recipient"
    );
    let on_beta = rb.join("khor-drop");
    assert!(
        fs::read_dir(&on_beta)
            .unwrap()
            .flatten()
            .any(|e| !e.file_name().to_string_lossy().starts_with('.')),
        "the payload must be on the recipient's own disk"
    );

    serve_a.abort();
    serve_b.abort();
}
