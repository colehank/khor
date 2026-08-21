//! The one real call to Anthropic's usage endpoint.
//!
//! **`#[ignore]` on purpose, and the reason is the feature.** Everything
//! `khor_node::quota` does is about not asking this endpoint more often
//! than a person is owed — five minutes of cache, a shared file so three
//! processes are one request, ten minutes of silence after a 429. A test
//! that ran on every `cargo test` would be the first thing to violate
//! all three, on the developer's own account.
//!
//! So it runs by name, deliberately, when somebody wants to know whether
//! the real thing still answers:
//!
//! ```text
//! cargo test -p khor-node --test quota_live -- --ignored --nocapture
//! ```
//!
//! It spends exactly one request. The property that matters most — that
//! a *second* process does not spend a second one — is not checked by
//! calling twice; it is checked by the artifact this leaves behind, and
//! by the unit test that reads such an artifact back
//! (`a_fresh_process_reads_the_machines_last_fetch`).

use khor_core::QuotaTrouble;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "spends a real request against the user's own Claude account"]
async fn the_endpoint_answers_and_leaves_the_machine_a_shared_cache() {
    // A home of its own, so a manual verification never writes into the
    // real one. The path is computed exactly as production computes it —
    // what changes is the root, which is the knob khor already has.
    let home = std::env::temp_dir().join(format!("khor-quota-live-{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("a home to point at");
    unsafe { std::env::set_var("KHOR_HOME", &home) };

    let answer = khor_node::quota::read().await;
    let cache = home.join(".khor").join("claude-usage.json");

    match answer {
        Ok(quota) => {
            // Never the token, never anything but the numbers the person
            // is being shown anyway.
            println!("windows: {:?}", quota.windows);
            println!("as_of: {:?}", quota.as_of);
            assert!(
                !quota.windows.is_empty(),
                "a successful read with no windows means the shape moved and the parser gave up \
                 quietly — which is exactly the failure this endpoint is expected to have one day"
            );
            for w in &quota.windows {
                assert!(
                    (0.0..=100.0).contains(&w.used_pct),
                    "{:?} came back at {}%, outside a percentage",
                    w.kind,
                    w.used_pct
                );
            }
            // **The point of the whole exercise.** One process asked; the
            // next one on this machine must not have to.
            assert!(
                cache.exists(),
                "no shared cache at {} — the next process would ask again",
                cache.display()
            );
            let back = std::fs::read_to_string(&cache).expect("the cache reads back");
            assert!(
                back.contains("fetched_at"),
                "the cache has no timestamp, so it can never be judged stale"
            );
        }
        // These three are answers, not failures of this test: they mean
        // the machine cannot ask right now, and each names why.
        Err(QuotaTrouble::NoLogin) => println!("no Claude Code login here — nothing was asked"),
        Err(QuotaTrouble::Stale) => println!("the stored credential is expired — nothing was asked"),
        Err(QuotaTrouble::Cooling { minutes }) => {
            println!("still cooling down, {minutes} minute(s) left — nothing was asked");
        }
        Err(other) => panic!("the endpoint did not answer: {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&home);
}
