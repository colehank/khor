//! The files landing over a real wire (账本: 网络类改动必须真连,还要有
//! 对照组). One serve owns alpha's key; beta lists and pulls over UDP
//! and must see exactly what alpha's disk holds. The control half: an
//! unpaired key sending the same ops is refused — "it worked" on the
//! paired path proves nothing unless the unpaired one truly does not.
//!
//! # The clock here is issue #73's, not this file's subject
//!
//! Every deadline below is sized to swallow one full first-touch stall
//! (#73: 20-40s per process on this machine, the comment in
//! `wait_for_endpoint_file` has the mechanism). **Uniformly**, because
//! which call pays it is not deterministic — on 2026-08-21 `pair`
//! cleared 15s here while `sync_now` blew 20s, and a per-call guess
//! would be exactly that. The runtime is multi-thread for the other
//! half of #73: that stall is *synchronous*, and on a current-thread
//! runtime it freezes tokio's clock, so every timeout in the file
//! silently stretches with it and a real hang stops looking like one.

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_node::Node;
use khor_node::proto::{Request, Response};
use tokio::time::timeout;

mod util;
use util::raw_request;

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
    let p = std::env::temp_dir().join(format!("khor-ls-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90s, not 10 (issue #73, sampled 2026-08-20; re-confirmed on this
    // machine 2026-08-21 — this file's one real-connection test timed out
    // at `sync_now` for this reason and no other). On a Mac operated over
    // ssh, a freshly compiled test binary is a new face to macOS: its
    // first network-stack touch (interface enumeration,
    // SystemConfiguration) hangs **20-40s per process**, waiting on an
    // authorization prompt nobody can ever click.
    //
    // What the budgets here are distinguishing themselves from: a real
    // hang, which is unbounded. 90s against a measured 40s ceiling is
    // rather more than double — deliberately, because a timeout that
    // never fires costs nothing when the test passes, and a flaky red
    // costs a person a trip to find out it was the machine.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_paired_machine_lists_and_pulls_the_far_disk_and_an_unpaired_key_is_refused() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // What alpha's disk actually holds — the fixture is asymmetric the
    // same way the unit test's is, so a wrong order cannot pass.
    let browse = ra.join("browse");
    fs::create_dir_all(browse.join("zoo")).unwrap();
    fs::write(browse.join("apple"), b"x").unwrap();
    fs::write(browse.join("Banana"), b"xy").unwrap();

    let (at, rows, truncated) = timeout(
        Duration::from_secs(60),
        b.ls_of("alpha", browse.to_str().unwrap()),
    )
    .await
    .expect("the listing must not hang")
    .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        ["zoo", "apple", "Banana"],
        "the far disk, in the node's order"
    );
    assert!(rows[0].dir && rows[2].size == 2 && !truncated);
    // Resolved, because the answer is the far machine's physical
    // spelling (`files::list_dir`), and `temp_dir()` on a Mac is a link.
    assert_eq!(
        at,
        browse.canonicalize().unwrap().to_str().unwrap(),
        "the answer names where it is about, links resolved"
    );

    // A relative path is refused across the real wire too — the refusal
    // must survive the trip, not just the unit test.
    let err = timeout(Duration::from_secs(60), b.ls_of("alpha", "some/where"))
        .await
        .expect("must not hang")
        .unwrap_err();
    assert!(
        err.contains("some/where"),
        "the refusal names the path: {err}"
    );

    // The pull: a file bigger than one slice, taken whole and byte-
    // identical — the loop is what a single-slice file cannot prove.
    let big: Vec<u8> = (0..(2 * khor_node::proto::SLICE + 1234) as u32)
        .map(|i| (i % 251) as u8)
        .collect();
    fs::write(browse.join("dataset.bin"), &big).unwrap();
    let dl = rb.join("landed");
    fs::create_dir_all(&dl).unwrap();
    let (moved, dest) = timeout(
        Duration::from_secs(60),
        b.pull_path("alpha", browse.join("dataset.bin").to_str().unwrap(), &dl),
    )
    .await
    .expect("the pull must not hang")
    .unwrap();
    assert_eq!(moved, big.len() as u64);
    assert_eq!(fs::read(&dest).unwrap(), big, "every byte, in order");
    assert!(
        !dl.join(".khor-pull-dataset.bin").exists(),
        "the dot-name must not survive a finished pull"
    );

    // A relative landing dir means "relative to the caller", and the
    // reported dest says so absolutely — a serve with a different cwd
    // once landed a pull in *its* directory while the caller read
    // "落在 ./x" (2026-08-19). `../..` from the crate dir is target/,
    // which is gitignored, so a panicking run litters nothing tracked.
    let rel = std::path::PathBuf::from(format!("../../target/pull-rel-{}", std::process::id()));
    fs::create_dir_all(&rel).unwrap();
    let (rel_moved, rel_dest) = timeout(
        Duration::from_secs(60),
        b.pull_path("alpha", browse.join("dataset.bin").to_str().unwrap(), &rel),
    )
    .await
    .expect("the relative pull must not hang")
    .unwrap();
    assert_eq!(rel_moved, big.len() as u64);
    assert!(
        rel_dest.is_absolute(),
        "the landing must be told absolutely: {rel_dest:?}"
    );
    assert_eq!(
        rel_dest,
        std::path::absolute(rel.join("dataset.bin")).unwrap(),
        "and it must be the caller's dir, not somebody else's cwd"
    );
    assert_eq!(
        fs::read(&rel_dest).unwrap(),
        big,
        "same bytes at the said place"
    );
    fs::remove_dir_all(&rel).unwrap();

    // Pulling onto an existing name refuses before any byte moves —
    // overwriting is the one irreversible act on this path.
    let err = timeout(
        Duration::from_secs(60),
        b.pull_path("alpha", browse.join("dataset.bin").to_str().unwrap(), &dl),
    )
    .await
    .expect("must not hang")
    .unwrap_err();
    assert!(
        err.contains("dataset.bin"),
        "the refusal names the file: {err}"
    );
    assert_eq!(
        fs::read(&dest).unwrap(),
        big,
        "and the existing file is untouched"
    );

    // A directory is not a file, and the refusal crosses the wire.
    let err = timeout(
        Duration::from_secs(60),
        b.pull_path("alpha", browse.join("zoo").to_str().unwrap(), &dl),
    )
    .await
    .expect("must not hang")
    .unwrap_err();
    assert!(!err.is_empty());
    assert!(
        !dl.join(".khor-pull-zoo").exists(),
        "a refused pull leaves no dot-name"
    );

    // The control: an unpaired key, same op, same wire. Proved against
    // a live gate first — the paired listing above used this very
    // serve — so a refusal here is the gate and not a dead endpoint.
    let alpha_info = b
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .expect("pairing put alpha in beta's table");
    let resp = timeout(
        Duration::from_secs(60),
        raw_request(
            iroh::SecretKey::generate(),
            &alpha_info.id,
            &alpha_info.addrs,
            &Request::Ls {
                path: browse.to_str().unwrap().to_owned(),
            },
        ),
    )
    .await
    .expect("must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::not_paired()),
        other => panic!("an unpaired ls must be refused, got {other:?}"),
    }
    let resp = timeout(
        Duration::from_secs(60),
        raw_request(
            iroh::SecretKey::generate(),
            &alpha_info.id,
            &alpha_info.addrs,
            &Request::FetchPath {
                path: browse.join("dataset.bin").to_str().unwrap().to_owned(),
                offset: 0,
            },
        ),
    )
    .await
    .expect("must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::not_paired()),
        other => panic!("an unpaired fetch-path must be refused, got {other:?}"),
    }

    // A directory pinned on beta names alpha's disk, and reaches alpha
    // over the same pump as every replicated decision — pin here,
    // listed there.
    b.pin_dir("alpha", browse.to_str().unwrap(), true).unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let pins = a.dir_pins().unwrap();
    let alpha_hex = a.device_str();
    assert!(
        pins.iter()
            .any(|(d, p)| d == alpha_hex && p == browse.to_str().unwrap()),
        "a pin made on beta must reach alpha: {pins:?}"
    );

    // A web page pinned against alpha as its exit travels the same pump —
    // the fourth pins table, keyed URL-and-machine (docs/NET.md 借网).
    b.pin_web("alpha", "https://example.com/read", true)
        .unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let webs = a.web_pins().unwrap();
    assert!(
        webs.iter()
            .any(|(d, u)| d == alpha_hex && u == "https://example.com/read"),
        "a web pin made on beta must reach alpha: {webs:?}"
    );

    // Where this person has been rides the same pump — and so does
    // "stop suggesting this", which is the half a per-device store
    // cannot do at all: dismissing on the desk has to reach the phone,
    // or the phone keeps offering what was already waved away.
    let place = browse.to_str().unwrap();
    b.recent_visit(khor_node::PlaceKind::Dir, "alpha", place)
        .unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let seen = a.recents(khor_node::PlaceKind::Dir, 10).unwrap();
    assert!(
        seen.iter()
            .any(|p| p.machine == alpha_hex && p.place == place),
        "a visit recorded on beta must reach alpha: {seen:?}"
    );

    b.forget_recent(khor_node::PlaceKind::Dir, "alpha", place)
        .unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let left = a.recents(khor_node::PlaceKind::Dir, 10).unwrap();
    assert!(
        !left.iter().any(|p| p.place == place),
        "the dismissal must reach alpha too, not just the device that made it: {left:?}"
    );

    // **A peer that cannot answer for this document must not cost the
    // rounds behind it.** That is the whole reason the recents round is
    // best-effort: it is a new name on the wire, and an older khor
    // refuses one by name. An older khor cannot be built here, so alpha
    // is made to refuse this one document for its own reason — a file
    // where its block directory belongs — which reaches beta as exactly
    // the same `Refused`.
    let recents_dir = ra.join(".khor").join("recents");
    let _ = fs::remove_dir_all(&recents_dir);
    fs::write(&recents_dir, b"not a directory").unwrap();
    assert!(
        a.recents(khor_node::PlaceKind::Dir, 10).is_err(),
        "control: the refusal has to be real, or this proves nothing"
    );

    b.recent_visit(khor_node::PlaceKind::Page, "alpha", "https://after.example")
        .unwrap();
    b.tell("alpha", "still talking").unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let said: Vec<String> = a
        .log("alpha")
        .unwrap()
        .messages
        .iter()
        .filter_map(|m| match &m.body {
            khor_node::MsgBody::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert!(
        said.iter().any(|t| t == "still talking"),
        "the chat round runs after recents and must survive its refusal: {said:?}"
    );
    let _ = fs::remove_file(&recents_dir);

    serve_a.abort();
    let _ = fs::remove_dir_all(&*ra);
}
