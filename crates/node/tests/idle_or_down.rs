//! **An idle wire is not an unreachable machine** (round NET-13: the
//! owner saw rows greyed as unreachable that opened at a click). Two residents in
//! this process, reachable through the tests' loopback relay only. A wire
//! let go of reads idle with no `down_ms` — it was not an attempt that
//! failed, and the next ask dials it fine; a dial that fails reads
//! `down_ms`; and anything heard from that machine clears it, here a
//! request it sends this way (the connection it opens is heard first;
//! beta's pump may dial back once alpha is heard, which is heard too).

mod util;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use khor_node::{Hop, Link, Node, link};
use tokio::time::timeout;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-idle-down-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

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

async fn link_to(n: &Node, id: &str) -> Link {
    n.links()
        .await
        .unwrap()
        .into_iter()
        .find(|(i, _)| i == id)
        .map(|(_, l)| l)
        .expect("a link for every machine in the table")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_idle_wire_is_not_down_a_failed_dial_is_and_a_byte_heard_clears_it() {
    // SAFETY: before any endpoint in this test binary binds.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let _serve_a = tokio::spawn(async move { sa.serve_resumable().await });
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let _serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&ra.0).await;
    wait_for_endpoint_file(&rb.0).await;
    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    timeout(Duration::from_secs(60), b.pair(&a.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let alpha_id = a.device_str().to_owned();
    let dir_a = ra.0.display().to_string();
    let dir_b = rb.0.display().to_string();
    let began = Instant::now();
    while b.ls_of("alpha", &dir_a).await.is_err() {
        assert!(began.elapsed() < Duration::from_secs(60), "beta reaches alpha");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let up = link_to(&b, &alpha_id).await;
    assert_eq!((up.hop, up.down_ms), (Hop::Relay, None), "a live wire");

    // Idle: the wire let go of, nothing tried since.
    b.let_go_of("alpha").await.unwrap();
    let idle = link_to(&b, &alpha_id).await;
    assert_eq!(idle.hop, Hop::Idle, "no live wire");
    assert_eq!(idle.down_ms, None, "a wire that closed idle is not an unreachable machine");
    assert!(idle.last_rtt_ms.is_some(), "the last round trip outlives the wire");

    // Down: alpha's ears close, and beta's next attempt fails.
    assert!(link::deafen(&ra.0).await, "alpha's resident deafens");
    let left = Instant::now();
    while b.ls_of("alpha", &dir_a).await.is_ok() {
        assert!(left.elapsed() < Duration::from_secs(10), "alpha stops answering");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let down = link_to(&b, &alpha_id).await;
    assert_eq!(down.hop, Hop::Idle);
    assert!(down.down_ms.is_some(), "a failed dial reads unreachable");
    let counts = b.wire_counts().await.unwrap();
    assert!(counts.read_down > 0, "the doctor counts it apart from idle");

    // Heard: alpha back, and it asks beta something. Beta's own ask is
    // not what clears it: its relay said alpha left, so its pump leaves
    // alpha alone until alpha is heard.
    let dialled = b.wire_counts().await.unwrap().dialed;
    link::resume(&ra.0).expect("resumable").synced().await.unwrap();
    let back = Instant::now();
    while a.ls_of("beta", &dir_b).await.is_err() {
        assert!(back.elapsed() < Duration::from_secs(30), "alpha reaches beta again");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let heard = link_to(&b, &alpha_id).await;
    assert_eq!(heard.down_ms, None, "a byte heard from alpha clears it");
    eprintln!(
        "[idle-or-down] idle {:?}; down {:?} ms after the failed dial; cleared in {:?}, beta dialled {} meanwhile",
        idle.hop,
        down.down_ms,
        back.elapsed(),
        b.wire_counts().await.unwrap().dialed - dialled
    );
}
