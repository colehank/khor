//! The files landing over a real wire (账本: 网络类改动必须真连,还要有
//! 对照组). One serve owns alpha's key; beta lists and pulls over UDP
//! and must see exactly what alpha's disk holds. The control half: an
//! unpaired key sending the same ops is refused — "it worked" on the
//! paired path proves nothing unless the unpaired one truly does not.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_node::proto::{Request, Response};
use khor_node::Node;
use tokio::time::timeout;

mod util;
use util::raw_request;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-ls-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
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

#[tokio::test]
async fn a_paired_machine_lists_and_pulls_the_far_disk_and_an_unpaired_key_is_refused() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(15), b.pair(&ticket))
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
        Duration::from_secs(15),
        b.ls_of("alpha", browse.to_str().unwrap()),
    )
    .await
    .expect("the listing must not hang")
    .unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["zoo", "apple", "Banana"], "the far disk, in the node's order");
    assert!(rows[0].dir && rows[2].size == 2 && !truncated);
    assert_eq!(at, browse.to_str().unwrap(), "the answer names where it is about");

    // A relative path is refused across the real wire too — the refusal
    // must survive the trip, not just the unit test.
    let err = timeout(Duration::from_secs(15), b.ls_of("alpha", "some/where"))
        .await
        .expect("must not hang")
        .unwrap_err();
    assert!(err.contains("some/where"), "the refusal names the path: {err}");

    // The pull: a file bigger than one slice, taken whole and byte-
    // identical — the loop is what a single-slice file cannot prove.
    let big: Vec<u8> = (0..(2 * khor_node::proto::SLICE + 1234) as u32)
        .map(|i| (i % 251) as u8)
        .collect();
    fs::write(browse.join("dataset.bin"), &big).unwrap();
    let dl = rb.join("landed");
    fs::create_dir_all(&dl).unwrap();
    let (moved, dest) = timeout(
        Duration::from_secs(30),
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

    // Pulling onto an existing name refuses before any byte moves —
    // overwriting is the one irreversible act on this path.
    let err = timeout(
        Duration::from_secs(15),
        b.pull_path("alpha", browse.join("dataset.bin").to_str().unwrap(), &dl),
    )
    .await
    .expect("must not hang")
    .unwrap_err();
    assert!(err.contains("dataset.bin"), "the refusal names the file: {err}");
    assert_eq!(fs::read(&dest).unwrap(), big, "and the existing file is untouched");

    // A directory is not a file, and the refusal crosses the wire.
    let err = timeout(
        Duration::from_secs(15),
        b.pull_path("alpha", browse.join("zoo").to_str().unwrap(), &dl),
    )
    .await
    .expect("must not hang")
    .unwrap_err();
    assert!(!err.is_empty());
    assert!(!dl.join(".khor-pull-zoo").exists(), "a refused pull leaves no dot-name");

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
        Duration::from_secs(15),
        raw_request(
            iroh::SecretKey::generate(),
            &alpha_info.id,
            &alpha_info.addrs,
            &Request::Ls { path: browse.to_str().unwrap().to_owned() },
        ),
    )
    .await
    .expect("must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::NOT_PAIRED),
        other => panic!("an unpaired ls must be refused, got {other:?}"),
    }
    let resp = timeout(
        Duration::from_secs(15),
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
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::NOT_PAIRED),
        other => panic!("an unpaired fetch-path must be refused, got {other:?}"),
    }

    // A directory pinned on beta names alpha's disk, and reaches alpha
    // over the same pump as every replicated decision — pin here,
    // listed there.
    b.pin_dir("alpha", browse.to_str().unwrap(), true).unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let pins = a.dir_pins().unwrap();
    let alpha_hex = a.device_str();
    assert!(
        pins.iter().any(|(d, p)| d == alpha_hex && p == browse.to_str().unwrap()),
        "a pin made on beta must reach alpha: {pins:?}"
    );

    serve_a.abort();
    let _ = fs::remove_dir_all(&ra);
    let _ = fs::remove_dir_all(&rb);
}
