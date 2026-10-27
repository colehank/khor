//! A tmux pane's history, asked over a real wire (item 31; 账本: 网络类
//! 改动必须真连,还要有对照组). Alpha hosts and has a tmux server of this
//! test's own with khor's grouped session in it (`@khor_row`); beta is a
//! phone (it cannot host, so it discovers nothing of its own) and asks
//! with `Node::pane_history`. It must get alpha's pane, whole, then only
//! what followed. The control half: an unpaired key sending the same
//! request is refused.
//!
//! The deadlines are `files_wire.rs`'s, for its reason (issue #73).

use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_node::Node;
use khor_node::proto::{Request, Response};
use tokio::time::timeout;

mod util;
use util::raw_request;

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
    let p = std::env::temp_dir().join(format!("khor-panehist-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// This test's own server, never the person's and never the shared
/// scratch one the other tests only ask: killed however the test leaves.
struct Server(String);

impl Server {
    fn tmux(&self, args: &[&str]) -> String {
        let out = Command::new("tmux")
            .arg("-L")
            .arg(&self.0)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn until_shown(&self, line: &str) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !self
            .tmux(&["capture-pane", "-p", "-t", "one"])
            .lines()
            .any(|l| l == line)
        {
            assert!(Instant::now() < deadline, "{line} was never shown");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.tmux(&["kill-server"]);
    }
}

fn numbers(lines: &[String]) -> Vec<u32> {
    lines.iter().filter_map(|l| l.trim().parse().ok()).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_phone_reads_a_far_tmux_pane_whole_then_from_its_anchor_and_an_unpaired_key_is_refused() {
    if Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .status()
        .is_err()
    {
        println!("skipped: no tmux on this machine");
        return;
    }
    let ra = root("a");
    let rb = root("b");
    let sock = format!("khor-panewire-{}", std::process::id());
    // SAFETY: the one test in this binary, before its nodes open; it only
    // points this process's discovery at a server of its own.
    unsafe {
        std::env::set_var("KHOR_TMUX_SOCKET", &sock);
    }
    let s = Server(sock);
    s.tmux(&[
        "-f",
        "/dev/null",
        "new-session",
        "-d",
        "-s",
        "one",
        "-x",
        "60",
        "-y",
        "20",
        "/bin/sh",
    ]);
    s.tmux(&["set-option", "-t", "one", "@khor_row", "shell/9-9"]);
    s.tmux(&["send-keys", "-t", "one", "clear; seq 1 300", "Enter"]);
    s.until_shown("300");

    let sa = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    let endpoint = ra.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !endpoint.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_hosting(rb.to_path_buf(), "beta", false).unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let first = timeout(
        Duration::from_secs(60),
        b.pane_history("alpha", "shell/9-9", None),
    )
    .await
    .expect("the read must not hang")
    .unwrap();
    assert!(first.refill && !first.alternate, "{first:?}");
    let mut seen = numbers(&first.lines);
    seen.extend(numbers(&first.screen));
    assert_eq!(
        seen,
        (1..=300).collect::<Vec<_>>(),
        "alpha's pane, history and screen, once"
    );

    s.tmux(&["send-keys", "-t", "one", "seq 301 400", "Enter"]);
    s.until_shown("400");
    let next = timeout(
        Duration::from_secs(60),
        b.pane_history("alpha", "shell/9-9", Some(first.anchor.clone())),
    )
    .await
    .expect("must not hang")
    .unwrap();
    assert!(!next.refill, "{next:?}");
    let mut held = numbers(&first.lines);
    held.extend(numbers(&next.lines));
    held.extend(numbers(&next.screen));
    assert_eq!(
        held,
        (1..=400).collect::<Vec<_>>(),
        "only what followed crossed"
    );

    // The phone itself has no tmux: asked of itself, it says so.
    assert!(b.pane_history("", "shell/9-9", None).await.is_err());

    // The control: an unpaired key, same request, same wire.
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
            &Request::PaneHistory {
                session: "shell/9-9".into(),
                since: None,
            },
        ),
    )
    .await
    .expect("must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::not_paired()),
        other => panic!("an unpaired read must be refused, got {other:?}"),
    }
    serve_a.abort();
}
