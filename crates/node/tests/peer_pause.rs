//! Pausing one other machine, over a real wire (SHELL.md 第五轮 第 3 条;
//! 账本: 网络类改动必须真连,还要有对照组).
//!
//! alpha serves, beta asks. Each side pauses the other in turn and the
//! ask must stop working **from that side's decision alone** — alpha by
//! not answering, beta by not dialling — and work again after resume.
//! The control is the first ask: a listing that works before any pause,
//! or the refusals after it prove nothing.
//!
//! Temporary stores and temporary identities only.

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
    let p = std::env::temp_dir().join(format!("khor-peerpause-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
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
async fn a_paused_machine_is_neither_answered_nor_dialled_and_resume_puts_it_back() {
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
    let ask = || async {
        timeout(Duration::from_secs(60), b.ls_of("alpha", &path))
            .await
            .expect("an ask must come back, answered or refused")
    };

    // Control: the listing works before anybody pauses anybody.
    assert!(ask().await.is_ok(), "control: alpha answers beta");

    // alpha pauses beta: alpha's decision alone must stop the answers.
    khor_node::pause::set_peer(&ra.0, b.device_str(), "beta", true).unwrap();
    assert!(
        ask().await.is_err(),
        "alpha has paused beta, so alpha must not answer it"
    );
    khor_node::pause::set_peer(&ra.0, b.device_str(), "beta", false).unwrap();
    assert!(ask().await.is_ok(), "and after resuming, it answers again");

    // beta pauses alpha: beta must not even dial, and say why.
    khor_node::pause::set_peer(&rb.0, a.device_str(), "alpha", true).unwrap();
    let why = ask().await.unwrap_err();
    assert!(
        why.contains(khor_catalog::msg::peer_paused_refusal()),
        "a paused machine is refused by name on this side, not dialled: {why}"
    );
    khor_node::pause::set_peer(&rb.0, a.device_str(), "alpha", false).unwrap();
    assert!(ask().await.is_ok(), "resume on beta's side puts it back");

    serve_a.abort();
}
