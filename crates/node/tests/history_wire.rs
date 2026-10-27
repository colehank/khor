//! A machine's command history, searched over a real wire (roadmap 2.6;
//! 账本: 网络类改动必须真连,还要有对照组). One serve owns alpha's key and
//! alpha's `history.jsonl`; beta asks with `Node::history` and must get
//! exactly alpha's matching commands, newest first. The control half: an
//! unpaired key sending the same request is refused.
//!
//! The deadlines are `files_wire.rs`'s, for its reason (issue #73: a
//! fresh test binary's first network touch on this Mac can stall
//! 20-40 s).

use std::fs;
use std::time::Duration;

use khor_node::Node;
use khor_node::commands::{Command, HistoryAsk, append_history, history_file};
use khor_node::proto::{Request, Response};
use tokio::time::timeout;

mod util;
use util::raw_request;

/// A scratch tree that takes itself away however the test leaves
/// (`files_wire.rs` has the reasoning; a copy per file, the convention).
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
    let p = std::env::temp_dir().join(format!("khor-hist-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

fn ran(n: u64, text: &str, cwd: &str, exit: i32) -> Command {
    Command {
        n,
        text: Some(text.to_owned()),
        cwd: Some(cwd.to_owned()),
        started_ms: n * 1000,
        ended_ms: Some(n * 1000 + 10),
        exit: Some(exit),
        at: 0,
        end_at: None,
        pane: None,
        last_line: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_paired_machine_searches_the_far_history_and_an_unpaired_key_is_refused() {
    let ra = root("a");
    let rb = root("b");
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
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // Alpha's history, two sessions; beta's own is different, so an
    // answer read off the wrong disk cannot pass.
    let file = history_file(&ra);
    append_history(
        &file,
        "shell/one",
        &[
            ran(1, "cargo test", "/w/khor", 0),
            ran(2, "make", "/w/other", 2),
        ],
    );
    append_history(
        &file,
        "shell/two",
        &[
            ran(3, "cargo build", "/w/khor", 101),
            ran(4, "cargo test", "/w/khor", 0),
        ],
    );
    append_history(
        &history_file(&rb),
        "shell/b",
        &[ran(1, "beta only", "/b", 0)],
    );

    let ask = |text: &str, failed: bool| HistoryAsk {
        text: text.into(),
        dir: String::new(),
        failed,
    };
    let hits = timeout(
        Duration::from_secs(60),
        b.history("alpha", &ask("cargo", false)),
    )
    .await
    .expect("the search must not hang")
    .unwrap();
    let rows: Vec<(&str, u32)> = hits.iter().map(|h| (h.text.as_str(), h.runs)).collect();
    assert_eq!(
        rows,
        [("cargo test", 2), ("cargo build", 1)],
        "alpha's, newest first, counted"
    );
    let failed = timeout(Duration::from_secs(60), b.history("alpha", &ask("", true)))
        .await
        .expect("must not hang")
        .unwrap();
    let failed: Vec<&str> = failed.iter().map(|h| h.text.as_str()).collect();
    assert_eq!(failed, ["cargo build", "make"]);
    // And this machine's own, without a wire.
    let mine = b.history("", &ask("", false)).await.unwrap();
    assert_eq!(
        mine.iter().map(|h| h.text.as_str()).collect::<Vec<_>>(),
        ["beta only"]
    );

    // The control: an unpaired key, same request, same wire, against the
    // serve that just answered.
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
            &Request::History {
                ask: ask("", false),
            },
        ),
    )
    .await
    .expect("must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::not_paired()),
        other => panic!("an unpaired history search must be refused, got {other:?}"),
    }
    serve_a.abort();
}
