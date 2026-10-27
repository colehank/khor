//! **An older khor and this one, syncing** (round NET-3): alpha's
//! resident is the khor named by `OLD_KHOR` (a build from before
//! `Request::Versions`), beta's is this one. Beta asks in the new words,
//! alpha refuses them, and beta syncs the way it always did; alpha asks
//! in the old words and beta answers them. Both ways a reading and a
//! document must cross. Without `OLD_KHOR` the test says so and passes.

mod scratch;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use khor_node::SessionId;
use scratch::*;

async fn until<F: Fn() -> bool>(what: &str, f: F) -> Duration {
    let began = Instant::now();
    while !f() {
        assert!(
            began.elapsed() < Duration::from_secs(40),
            "{what} never happened"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    began.elapsed()
}

fn open_probe(home: &std::path::Path, name: &str, bin: &std::path::Path) -> String {
    let out = khor_from(home, bin)
        .args(["open", "-d", "--title", "probe", "--", "/bin/sleep", "600"])
        .env("KHOR_NAME", name)
        .output()
        .unwrap();
    let sid = String::from_utf8_lossy(&out.stdout)
        .trim()
        .lines()
        .last()
        .unwrap_or_default()
        .to_owned();
    assert!(
        sid.contains('/'),
        "a session id: {sid:?} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    sid
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_older_khor_and_this_one_still_sync_both_ways() {
    let Some(old) = std::env::var_os("OLD_KHOR").map(PathBuf::from) else {
        eprintln!("[mixed] OLD_KHOR not set: nothing to compare against");
        return;
    };
    let new = PathBuf::from(env!("CARGO_BIN_EXE_khor"));
    let (p, _) = pair_built("mixed", None, &old).await;

    let seen_on = |node: &khor_node::Node, sid: &str, pinned: bool| {
        node.sessions()
            .map(|rows| {
                rows.iter()
                    .any(|r| r.session.id.0 == sid && (!pinned || r.pinned))
            })
            .unwrap_or(false)
    };

    // Old alpha to new beta: beta asks in new words, falls back.
    let on_alpha = open_probe(&p.ra.0, "alpha", &old);
    let row = until("alpha's row on beta", || seen_on(&p.beta, &on_alpha, false)).await;
    p.alpha
        .pin_session(&SessionId(on_alpha.clone()), true)
        .unwrap();
    let pin = until("alpha's pin on beta", || seen_on(&p.beta, &on_alpha, true)).await;

    // New beta to old alpha: alpha asks in old words, beta answers.
    let on_beta = open_probe(&p.rb.0, "beta", &new);
    let back_row = until("beta's row on alpha", || seen_on(&p.alpha, &on_beta, false)).await;
    p.beta
        .pin_session(&SessionId(on_beta.clone()), true)
        .unwrap();
    let back_pin = until("beta's pin on alpha", || seen_on(&p.alpha, &on_beta, true)).await;

    eprintln!(
        "[mixed] older alpha, newer beta: alpha's row on beta in {row:?}, its pin in {pin:?}; \
         beta's row on alpha in {back_row:?}, its pin in {back_pin:?}"
    );
    let _ = khor_from(&p.ra.0, &old)
        .args(["close", &on_alpha])
        .env("KHOR_NAME", "alpha")
        .output();
    let _ = khor(&p.rb.0)
        .args(["close", &on_beta])
        .env("KHOR_NAME", "beta")
        .output();
}
