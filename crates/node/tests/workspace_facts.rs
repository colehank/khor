//! **A workspace row's facts, kept where the files are and carried by a
//! round** (round NET-8, `crate::wsfacts`; 账本: 网络类改动必须真连). Alpha
//! holds two workspaces — a git checkout and a plain folder — and its
//! resident keeps their facts; beta, paired, learns them in a round.
//! Then a file is written in the checkout and one is changed: alpha's
//! watcher marks it stale, alpha recomputes, and beta's next round
//! carries the new count and the later time. The plain folder has a time
//! and nothing else. Nothing here asks a machine for a status: the
//! numbers move because a file did.

use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::Node;
use tokio::time::timeout;

mod util;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-wsfacts-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &std::path::Path) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main"])
        .args(args)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

/// Waits until `f` holds, up to `within`.
async fn until(within: Duration, what: &str, mut f: impl FnMut() -> bool) -> Duration {
    let began = Instant::now();
    while !f() {
        assert!(began.elapsed() < within, "{what} never happened");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    began.elapsed()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_workspaces_facts_move_when_a_file_does_and_reach_the_other_machine() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let _serve_a = tokio::spawn(async move { sa.serve().await });
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let _serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&ra.0).await;
    wait_for_endpoint_file(&rb.0).await;
    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    timeout(Duration::from_secs(60), b.pair(&a.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let alpha = a.device_str().to_owned();

    // A checkout with one commit, and a plain folder.
    let repo = ra.0.join("repo");
    let plain = ra.0.join("plain");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&plain).unwrap();
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    git(&repo, &["add", "a.txt"]);
    git(&repo, &["commit", "-q", "-m", "one"]);
    fs::write(plain.join("notes.txt"), "x").unwrap();
    a.ws_add("", &repo.display().to_string()).await.unwrap();
    a.ws_add("", &plain.display().to_string()).await.unwrap();
    let repo = fs::canonicalize(&repo).unwrap().display().to_string();
    let plain = fs::canonicalize(&plain).unwrap().display().to_string();

    // Alpha's resident takes them up on its next beat.
    let fact = |n: &Node, path: &str| n.workspace_facts_of(&alpha).into_iter().find(|f| f.path == path);
    let kept = until(Duration::from_secs(30), "alpha keeping both", || {
        fact(&a, &repo).is_some() && fact(&a, &plain).is_some()
    })
    .await;
    let first = fact(&a, &repo).unwrap();
    assert_eq!(first.branch.as_deref(), Some("main"));
    assert_eq!(first.changed, Some(0), "a clean checkout");
    let folder = fact(&a, &plain).unwrap();
    assert_eq!((folder.branch.as_deref(), folder.changed), (None, None), "a folder has only a time");
    assert!(folder.changed_ms > 0, "and has one");

    // Beta learns them in two rounds: the first brings the table that
    // says alpha holds workspaces, the next asks alpha for their facts —
    // a machine with none is not asked at all (the reading's name would
    // cost a phone half a KB a minute for nothing).
    timeout(Duration::from_secs(60), b.sync_now()).await.unwrap().unwrap();
    timeout(Duration::from_secs(60), b.sync_now()).await.unwrap().unwrap();
    assert_eq!(fact(&b, &repo), Some(first.clone()), "beta holds alpha's facts");

    // A file changed and one added: the watcher, a recompute, the next round.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let written = Instant::now();
    fs::write(std::path::Path::new(&repo).join("a.txt"), "two\n").unwrap();
    fs::write(std::path::Path::new(&repo).join("b.txt"), "new\n").unwrap();
    let moved = until(Duration::from_secs(30), "alpha counting two", || {
        fact(&a, &repo).is_some_and(|f| f.changed == Some(2))
    })
    .await;
    let now = fact(&a, &repo).unwrap();
    assert!(now.changed_ms > first.changed_ms, "the time moved: {} → {}", first.changed_ms, now.changed_ms);
    timeout(Duration::from_secs(60), b.sync_now()).await.unwrap().unwrap();
    assert_eq!(fact(&b, &repo), Some(now), "and beta has the new count");
    eprintln!(
        "[wsfacts] kept after {kept:?}; a write counted on alpha {moved:?} after it ({:?} with the round)",
        written.elapsed()
    );
}
