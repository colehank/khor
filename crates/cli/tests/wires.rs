//! **The wires** (round NET-FIX, audit/net.md F1–F8): one long-lived
//! connection per machine, deadlines that retire a dead one, far streams
//! answered at once, a parallel pump with backoff, and an honest road
//! word with an age beside it — each measured on real `khor serve`
//! processes.
//!
//! Every home here binds with **only** a `khor relay --dev` this test
//! starts on loopback (`relays.json`, only-mine), and every identity is
//! a fresh one in a temporary home: nothing here touches a real relay,
//! a real store or a real key. Processes are stopped by pid.
//!
//! Each test prints the reading it took (`--nocapture`), beside the one
//! audit/net.md measured before the change.

mod scratch;

use std::time::{Duration, Instant};

use khor_node::{Hop, Node};
use scratch::*;
use tokio::time::timeout;

/// **F1 / F2: one wire, held.** A healthy pair dials once and then rides
/// the wire it holds: the sync pump visits every five seconds without a
/// single new connection, and the road word does not move.
///
/// Before (audit/net.md, probe A): 13–14 fresh dials a minute per side —
/// one per pump round, each dropped after its round.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_healthy_pair_dials_once_and_its_word_holds() {
    let p = pair("held").await;
    let alpha_id = p.alpha.device_str().to_owned();
    until_reads(&p.beta, &alpha_id, Hop::Direct, Duration::from_secs(30)).await;
    // Past the pairing's own dials and the first rounds.
    tokio::time::sleep(Duration::from_secs(6)).await;
    let before = p.beta.wire_counts().await.expect("beta's resident answers");
    let started = Instant::now();
    let mut words = Vec::new();
    while started.elapsed() < Duration::from_secs(60) {
        let links = p.beta.links().await.unwrap();
        if let Some((_, l)) = links.iter().find(|(id, _)| *id == alpha_id) {
            words.push(l.hop);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let after = p.beta.wire_counts().await.unwrap();
    let dialed = after.dialed - before.dialed;
    let flips = words.windows(2).filter(|w| w[0] != w[1]).count();
    eprintln!(
        "[F1] 60 s on a healthy pair: {dialed} new wires (before: 13–14), {} samples, {flips} flips",
        words.len()
    );
    assert!(
        dialed <= 1,
        "a healthy pair dialled {dialed} times in a minute: the wire is not held"
    );
    assert_eq!(flips, 0, "the word moved on a healthy wire: {words:?}");
    assert!(words.iter().all(|h| *h == Hop::Direct), "{words:?}");
}

/// **F5: a slow request does not hold a quick one.** Far streams are
/// answered at once, so a listing beside a five-second search on the
/// same wire answers in milliseconds.
///
/// Before (probe H): `git status` 2–13 ms alone, 4706 ms behind the
/// search on the same wire.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_slow_request_does_not_hold_a_quick_one() {
    let p = pair("slow").await;
    let tmp = std::env::temp_dir().display().to_string();
    // Warm the wire, and take the quick one's own time.
    let began = Instant::now();
    p.beta
        .ls_of("alpha", &tmp)
        .await
        .expect("a listing answers");
    let alone = began.elapsed();
    let rb = p.rb.0.clone();
    let slow = tokio::spawn(async move {
        let n = Node::open_as(rb, "beta").unwrap();
        let began = Instant::now();
        let found = n.search("alpha", "/System/Library", "zq9xk7nothere").await;
        (began.elapsed(), found.is_ok())
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    let began = Instant::now();
    p.beta
        .ls_of("alpha", &tmp)
        .await
        .expect("a listing answers");
    let beside = began.elapsed();
    let (search_took, search_ok) = slow.await.unwrap();
    eprintln!(
        "[F5] listing alone {alone:?}, beside a {search_took:?} search {beside:?} (before: 4706 ms)"
    );
    assert!(search_ok, "the search answered");
    assert!(
        search_took >= Duration::from_secs(2),
        "control: the search must really be slow ({search_took:?}) for this to mean anything"
    );
    assert!(
        beside < Duration::from_secs(1),
        "the listing waited {beside:?} behind the search"
    );
}

/// **F6 / F7: a restarted peer costs one ack wait, not thirty seconds.**
///
/// A reading on a wire the far serve's restart left dead is asked again
/// on a fresh wire — the person sees an answer, a few seconds late. A
/// request that is not a reading is not asked twice: it fails within the
/// ack wait, and the next one goes through.
///
/// Before (probe K): three asks in a row hung 29.5 s each; the fourth
/// failed "connection lost" at about 100 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_restarted_peer_costs_one_ack_wait() {
    let mut p = pair("restart").await;
    let tmp = std::env::temp_dir().display().to_string();
    p.beta
        .ls_of("alpha", &tmp)
        .await
        .expect("control: a listing answers");

    // A reading, right after a restart.
    let old = serve_pid(&p.ra.0).unwrap();
    drop(p.a.take());
    p.a = Some(serve(&p.ra.0, "alpha"));
    timeout(Duration::from_secs(60), async {
        while serve_pid(&p.ra.0).is_none_or(|pid| pid == old) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("alpha comes back");
    let began = Instant::now();
    let read = p.beta.ls_of("alpha", &tmp).await;
    let read_took = began.elapsed();

    // A write, right after another restart.
    let old = serve_pid(&p.ra.0).unwrap();
    drop(p.a.take());
    p.a = Some(serve(&p.ra.0, "alpha"));
    timeout(Duration::from_secs(60), async {
        while serve_pid(&p.ra.0).is_none_or(|pid| pid == old) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("alpha comes back");
    let dir = p.ra.0.join("made");
    let began = Instant::now();
    let first = p.beta.mkdir("alpha", &dir.display().to_string()).await;
    let first_took = began.elapsed();
    let began = Instant::now();
    let second = p.beta.mkdir("alpha", &dir.display().to_string()).await;
    let second_took = began.elapsed();
    eprintln!(
        "[F7] after a restart: a reading answered {} in {read_took:?}; a write {} in {first_took:?}, \
         the next {} in {second_took:?} (before: 29.5 s × 3, then an error at ~100 s)",
        if read.is_ok() { "ok" } else { "ERR" },
        if first.is_ok() { "ok" } else { "failed" },
        if second.is_ok() { "ok" } else { "failed" },
    );
    assert!(
        read.is_ok(),
        "the reading is asked again on a fresh wire: {read:?}"
    );
    assert!(
        read_took < Duration::from_secs(12),
        "the reading took {read_took:?}: more than one ack wait and a dial"
    );
    // The write either went through on a wire that was already fresh (the
    // pump got there first) or failed within the ack wait — never the
    // ipc deadline.
    assert!(
        first_took < Duration::from_secs(8),
        "the first write after a restart took {first_took:?}"
    );
    assert!(second.is_ok(), "the next write goes through: {second:?}");
    assert!(second_took < Duration::from_secs(12), "{second_took:?}");
}

/// **F3 / F6: a frozen peer reads not connected, its age grows, and a
/// question to it ends in seconds.** The wire is retired once nothing is
/// heard on it, and the word follows within a few seconds; thawed, the
/// next round dials it again.
///
/// Before (probe S): 直连 in all 225 samples across a 40 s freeze; a
/// `git status` on the held wire waited the whole 20 s freeze.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_frozen_peer_reads_not_connected_and_its_age_grows() {
    let p = pair("frozen").await;
    let alpha_id = p.alpha.device_str().to_owned();
    until_reads(&p.beta, &alpha_id, Hop::Direct, Duration::from_secs(30)).await;
    let pid = serve_pid(&p.ra.0).unwrap();
    signal(pid, "-STOP");
    let stopped = Instant::now();
    let rb = p.rb.0.clone();
    let tmp = std::env::temp_dir().display().to_string();
    let asked = tokio::spawn(async move {
        let n = Node::open_as(rb, "beta").unwrap();
        let began = Instant::now();
        let r = n.ls_of("alpha", &tmp).await;
        (began.elapsed(), r.is_ok())
    });
    let went_quiet = until_reads(&p.beta, &alpha_id, Hop::Idle, Duration::from_secs(30)).await;
    let first_age = p
        .beta
        .links()
        .await
        .unwrap()
        .into_iter()
        .find(|(id, _)| *id == alpha_id)
        .and_then(|(_, l)| l.heard_ms)
        .expect("alpha was heard before it froze");
    tokio::time::sleep(Duration::from_secs(5)).await;
    let later_age = p
        .beta
        .links()
        .await
        .unwrap()
        .into_iter()
        .find(|(id, _)| *id == alpha_id)
        .and_then(|(_, l)| l.heard_ms)
        .unwrap();
    let (ask_took, ask_ok) = asked.await.unwrap();
    signal(pid, "-CONT");
    let back = until_reads(&p.beta, &alpha_id, Hop::Direct, Duration::from_secs(60)).await;
    eprintln!(
        "[F3] frozen: 未连接 after {went_quiet:?}, last heard {first_age} ms then {later_age} ms; \
         a listing ended in {ask_took:?} ({}); thawed, 直连 again after {back:?} \
         (before: 直连 throughout a 40 s freeze; a held ask waited the whole freeze)",
        if ask_ok { "answered" } else { "not reached" }
    );
    assert!(
        went_quiet < Duration::from_secs(20),
        "the word took {went_quiet:?} to say not connected"
    );
    assert!(
        later_age >= first_age + 4_000,
        "the age grows while nothing is heard: {first_age} then {later_age}"
    );
    assert!(
        stopped.elapsed() > ask_took && ask_took < Duration::from_secs(16),
        "a question to a frozen machine took {ask_took:?}"
    );
    assert!(!ask_ok, "control: a frozen machine does not answer");
}

/// **F8: a dark machine costs its own budget, not the round's.** Three
/// machines; one is stopped. An explicit sync answers in about one dial
/// budget — the dark one's — with the live one synced, and the pump
/// backs off the dark one instead of dialling it every five seconds.
///
/// Before (probe S): `sync_now` with one frozen peer 10 004 ms, every
/// round, serial; the pump dialled it every round.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_dark_machine_costs_its_own_budget_and_is_backed_off() {
    let p = pair("dark").await;
    let rc = scratch("dark-c");
    let relay_url = {
        let doc: serde_json::Value = serde_json::from_slice(
            &std::fs::read(p.ra.0.join(".khor").join("relays.json")).unwrap(),
        )
        .unwrap();
        doc["relays"][0]["url"].as_str().unwrap().to_owned()
    };
    home_on(&rc.0, &relay_url);
    let c = serve(&rc.0, "gamma");
    stood_up(&[&rc.0]).await;
    let gamma = Node::open_as(rc.0.clone(), "gamma").unwrap();
    let ticket = p.alpha.invite().unwrap();
    timeout(Duration::from_secs(90), gamma.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("gamma pairs with alpha");
    // Until beta has gamma, admitted, through alpha.
    timeout(Duration::from_secs(60), async {
        loop {
            let out = p.beta.sync_now().await.unwrap_or_default();
            if out.iter().any(|(name, r)| name == "gamma" && r.is_ok()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .expect("beta reaches gamma through the table alpha shared");

    drop(c);
    tokio::time::sleep(Duration::from_secs(1)).await;
    let began = Instant::now();
    let out = p.beta.sync_now().await.expect("a sync comes back");
    let took = began.elapsed();
    let alpha_ok = out.iter().any(|(n, r)| n == "alpha" && r.is_ok());
    let gamma_failed = out.iter().any(|(n, r)| n == "gamma" && r.is_err());
    // The pump's account of gamma over the next minute.
    let before = p.beta.wire_counts().await.unwrap();
    tokio::time::sleep(Duration::from_secs(60)).await;
    let after = p.beta.wire_counts().await.unwrap();
    let failed_dials = after.dial_failed - before.dial_failed;
    eprintln!(
        "[F8] sync with one dark machine: {took:?}, alpha {}, gamma {}; failed dials to it in the \
         next minute: {failed_dials} (before: 10 004 ms serial per round, a dial every round = 12)",
        if alpha_ok { "synced" } else { "NOT synced" },
        if gamma_failed { "unreached" } else { "?" },
    );
    assert!(alpha_ok && gamma_failed, "{out:?}");
    assert!(
        took < Duration::from_secs(12),
        "one dark machine cost the sync {took:?}"
    );
    assert!(
        failed_dials <= 5,
        "the pump dialled a dark machine {failed_dials} times in a minute"
    );
}

/// **A phone going to the background lets its wires go at once**
/// (`link::suspending`, the app's `WindowEvent::Suspended`): the desk
/// reads it not connected in a moment, not after its silence limit, and
/// the phone's way back (`link::resume`) is direct again.
///
/// The phone here is an in-process resumable serve — what the iPhone app
/// runs — on a scratch home with the loopback relay.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_phone_going_to_the_background_reads_not_connected_at_once() {
    let (ra, rb, rr) = (scratch("phone-a"), scratch("phone-b"), scratch("phone-r"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let phone = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { phone.serve_resumable().await });
    let _b = serve(&rb.0, "beta");
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(90), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds");
    let alpha_id = alpha.device_str().to_owned();
    until_reads(&beta, &alpha_id, Hop::Direct, Duration::from_secs(30)).await;

    assert!(
        khor_node::link::suspending(&ra.0).await,
        "the phone's resident takes the suspension"
    );
    let quiet = until_reads(&beta, &alpha_id, Hop::Idle, Duration::from_secs(30)).await;
    let mut resuming = khor_node::link::resume(&ra.0).expect("the phone's resident resumes");
    resuming.ears().await.expect("new ears");
    let back = until_reads(&beta, &alpha_id, Hop::Direct, Duration::from_secs(30)).await;
    eprintln!(
        "[phone] suspended: the desk read 未连接 after {quiet:?} (silence limit: 15 s); resumed: 直连 after {back:?}"
    );
    assert!(
        quiet < Duration::from_secs(3),
        "the desk took {quiet:?} to see the phone go"
    );
    serve_a.abort();
}
