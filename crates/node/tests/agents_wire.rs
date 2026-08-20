//! The agent registry over a real wire (批⑥; 账本: 网络类改动必须真连,
//! 还要有对照组). Two nodes on one machine, real UDP: a registration made
//! on one is offerable on the other, and forgetting it travels back.
//!
//! # Why this is not in the registry's own unit test
//!
//! `khor_sync::agents` merges two documents in one process, which proves
//! the CRDT and nothing about the transport. The document is named by a
//! **string** in two places on this side of the fence — the serve's
//! `Request::Sync` dispatch and the sync round — and a name that is
//! wrong, or missing from one of them, would leave every in-process test
//! green while no registration ever travelled. The whole reason this is
//! a document rather than a preference file is the sentence "say it
//! once, true on every machine", and that sentence is exactly what only
//! a wire can check.
//!
//! # Its own file, with its own clock (issue #73)
//!
//! On a Mac operated over ssh a freshly compiled test binary is a new
//! face to macOS, and its first network-stack touch (interface
//! enumeration, SystemConfiguration) hangs 20-40s per process waiting on
//! an authorization prompt nobody can ever click. `transfer.rs` already
//! carries 45-90s deadlines for it; `pairing.rs` and `files_wire.rs`
//! still carry 10-20s ones and time out here for that reason and no
//! other (checked 2026-08-21 01:35 — removing this batch's own sync
//! round left them red exactly as before).
//!
//! So the deadlines here are transfer.rs's, and the runtime is
//! multi-thread for transfer.rs's other reason: that stall is
//! synchronous, and on a current-thread runtime it freezes tokio's clock
//! so every timeout in the file silently stretches with it.

use std::path::PathBuf;
use std::time::Duration;

use khor_node::Node;
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-agentwire-{tag}-{}", std::process::id()));
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

/// **A registered agent really crosses the wire, and forgetting it
/// crosses back.**
///
/// The assertion *before* the sync is the load-bearing half. Without it
/// this passes on two nodes that shared a store, on a sync that did
/// nothing, and on a registry that was somehow global — every way of
/// being wrong except the one it is checking.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_registered_agent_reaches_the_other_machine() {
    let ra = root("a");
    let rb = root("b");

    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    b.register_agent("gemini", &["gemini".to_owned(), "--acp".to_owned()]).unwrap();
    assert_eq!(
        a.agent("gemini").unwrap(),
        None,
        "control: alpha has not heard of it before a sync — without this line \
         the test proves nothing about the wire"
    );

    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    let landed = a.agent("gemini").unwrap().expect("the registration reached alpha");
    assert_eq!(
        landed.typed(),
        "gemini --acp",
        "and it arrived whole — a command with its arguments, not just a name"
    );

    // Forgetting travels too. A registry that only ever grew would leave
    // every other machine offering an agent the person removed.
    b.forget_agent("gemini").unwrap();
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    assert_eq!(a.agent("gemini").unwrap(), None, "forgetting reaches alpha too");

    serve.abort();
    let _ = std::fs::remove_dir_all(&ra);
    let _ = std::fs::remove_dir_all(&rb);
}
