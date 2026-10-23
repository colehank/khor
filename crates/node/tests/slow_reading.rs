//! **A reading that cannot be had in time does not hold up the round**
//! (fix/front, 2026-10-23).
//!
//! The opening [`Request::Versions`] of every round names the quotas
//! reading, and the far side used to work out each reading in turn and
//! answer only when the last was done. A machine whose quota read hung
//! on a provider — 5 accounts read one after another, a 15 s HTTP
//! timeout each — kept every asker's round waiting past the opening's
//! own budget: a phone back from the background waited 4–5 s, 15–24 s
//! with the cache out, and `khor sync` between desks said
//! 「didn't finish answering within 15s」.
//!
//! Here beta's one account is a new-api gateway on loopback that takes
//! the connection and never answers, and alpha's round with beta must
//! still come back, good, in well under the time that gateway costs.

mod util;

use std::time::{Duration, Instant};

use khor_node::Node;
use khor_sync::accounts::{AccountKind, Provider};
use tokio::time::timeout;

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
    let p = std::env::temp_dir().join(format!("khor-slow-reading-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// A gateway that takes every connection and sits on it for 20 s
/// without a byte — what a provider looks like from here when it hangs.
fn hanging_gateway() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(conn) = conn else { continue };
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(20));
                drop(conn);
            });
        }
    });
    format!("http://{addr}")
}

async fn wait_for_endpoint_file(root: &std::path::Path) {
    // 90 s: a fresh test binary's first network touch on macOS (usage.rs).
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hanging_quota_read_over_there_does_not_hold_up_the_round() {
    let ra = root("a");
    let rb = root("b");
    let gateway = hanging_gateway();

    // Registered before beta serves, and before anything asks: the
    // round under the clock below is the first quota read beta makes,
    // so no cooldown from an earlier failure can answer it fast.
    let beta = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    beta.register_account(
        "hangs",
        Provider::Newapi,
        AccountKind::Bill,
        Some(&gateway),
        "sk-test",
        None,
    )
    .unwrap();
    let server = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&rb).await;

    let alpha = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = beta.invite().unwrap();
    timeout(Duration::from_secs(60), alpha.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let begun = Instant::now();
    let out = timeout(Duration::from_secs(60), alpha.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let took = begun.elapsed();
    println!("alpha's round took {took:?}: {out:?}");
    let (_, beta_round) = out
        .iter()
        .find(|(name, _)| name == "beta")
        .unwrap_or_else(|| panic!("alpha's round visited beta: {out:?}"));
    assert!(
        beta_round.is_ok(),
        "the round with beta came back good, not {beta_round:?} (after {took:?})"
    );
    assert!(
        took < Duration::from_secs(3),
        "the round with beta waited {took:?} on a quota read that was never going to answer in time"
    );

    serve.abort();
}
