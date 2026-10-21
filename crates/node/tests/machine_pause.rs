//! Pausing this whole machine is 「不收不发」 (docs/MORE.md 暂停), for
//! what is already open as well (net-dial B2).
//!
//! It used to stop only what came next: a new connection was refused and
//! the pump stopped, while a wire another machine already held kept being
//! answered, and the paused machine's own verbs still dialled. alpha
//! serves, beta holds a wire to it; alpha pauses and that wire must close
//! within a pause beat, then beta pauses and must not dial at all. The
//! control is the first ask over the same wire.
//!
//! Temporary stores and temporary identities only.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_node::Node;
use tokio::time::timeout;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-machinepause-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// 90 s for issue #73's first-touch stall; `files_wire.rs` has why.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pausing_closes_what_is_open_and_dials_nothing() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra.0).await;

    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let dir = ra.0.join("browse");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("apple"), b"x").unwrap();
    let path = dir.to_str().unwrap().to_owned();
    // beta's resident holds the wire (a one-shot node lets its go).
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let watch = sb.wire_watch();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&rb.0).await;
    let ask = || async {
        timeout(Duration::from_secs(60), b.ls_of("alpha", &path))
            .await
            .expect("an ask must come back, answered or refused")
    };

    // Control: answered, on a wire beta now holds.
    assert!(ask().await.is_ok(), "control: alpha answers beta");
    assert_eq!(watch.is_open("alpha").await, Some(true), "control: beta holds a wire to alpha");

    // alpha pauses: the wire beta holds is closed from alpha's side.
    khor_node::pause::set_on(&ra.0, true).unwrap();
    let closed = timeout(Duration::from_secs(5), async {
        while watch.is_open("alpha").await == Some(true) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(closed.is_ok(), "a paused alpha let go of the wire beta already held");
    assert!(ask().await.is_err(), "and does not answer on a new one");

    khor_node::pause::set_on(&ra.0, false).unwrap();
    let back = timeout(Duration::from_secs(30), async {
        loop {
            if ask().await.is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    })
    .await;
    assert!(back.is_ok(), "resumed, alpha answers again");

    // beta pauses: nothing is dialled, and the refusal says so.
    khor_node::pause::set_on(&rb.0, true).unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let why = ask().await.unwrap_err();
    assert!(
        why.contains(khor_catalog::msg::paused_no_dial()),
        "a paused machine does not dial, and says so: {why}"
    );
    assert_eq!(watch.is_open("alpha").await, Some(false), "and the wire it held is let go");

    serve_a.abort();
    serve_b.abort();
}
