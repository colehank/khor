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
    let seg = first.git.clone().expect("a checkout has a git segment");
    assert_eq!((seg.branch.as_deref(), seg.subject.as_deref()), (Some("main"), Some("one")));
    assert_eq!(seg.head.as_ref().map(String::len), Some(8), "HEAD's short id");
    assert_eq!((seg.staged, seg.unstaged, seg.untracked), (0, 0, 0));
    assert!(seg.since_ms > 0, "dated by git's own files on first sight");
    let folder = fact(&a, &plain).unwrap();
    assert_eq!((folder.branch.as_deref(), folder.changed), (None, None), "a folder has only a time");
    assert_eq!(folder.git, None, "and no git segment");
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
    let moved_seg = now.git.clone().unwrap();
    assert_eq!((moved_seg.unstaged, moved_seg.untracked), (1, 1), "!1 ?1");
    assert!(moved_seg.since_ms > seg.since_ms, "a changed segment is dated anew");
    timeout(Duration::from_secs(60), b.sync_now()).await.unwrap().unwrap();
    assert_eq!(fact(&b, &repo), Some(now), "and beta has the new count");
    eprintln!(
        "[wsfacts] kept after {kept:?}; a write counted on alpha {moved:?} after it ({:?} with the round)",
        written.elapsed()
    );
}

/// **The prompt's segment, piece by piece** (UX-48): staged, edited,
/// untracked, a stash, a detached HEAD — read straight off a checkout.
#[test]
fn a_checkouts_segment_counts_what_a_prompt_counts() {
    let r = root("segment");
    let repo = r.0.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    fs::write(repo.join("b.txt"), "one\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "first line\n\nbody"]);
    let seg = khor_node::git::segment(&repo).unwrap();
    assert_eq!((seg.branch.as_deref(), seg.subject.as_deref()), (Some("main"), Some("first line")));
    assert_eq!((seg.ahead, seg.behind, seg.stashes, seg.staged, seg.unstaged, seg.untracked), (0, 0, 0, 0, 0, 0));

    fs::write(repo.join("a.txt"), "two\n").unwrap();
    git(&repo, &["stash", "-q"]);
    fs::write(repo.join("a.txt"), "three\n").unwrap();
    git(&repo, &["add", "a.txt"]);
    fs::write(repo.join("b.txt"), "two\n").unwrap();
    fs::write(repo.join("c.txt"), "new\n").unwrap();
    fs::write(repo.join("d.txt"), "new\n").unwrap();
    let seg = khor_node::git::segment(&repo).unwrap();
    assert_eq!(
        (seg.stashes, seg.staged, seg.unstaged, seg.untracked, seg.changed),
        (1, 1, 1, 2, 4),
        "*1 +1 !1 ?2"
    );

    let head = seg.head.clone().unwrap();
    git(&repo, &["checkout", "-q", "--detach"]);
    let seg = khor_node::git::segment(&repo).unwrap();
    assert_eq!((seg.branch, seg.head), (None, Some(head)), "detached: no branch, its short id");
    assert!(khor_node::git::last_moved(&repo) > 0, "git's own files date it");

    let plain = r.0.join("plain");
    fs::create_dir_all(&plain).unwrap();
    assert_eq!(khor_node::git::last_moved(&plain), 0);
}

/// Like [`git`], for the commands that stop on a conflict on purpose.
fn git_stops(dir: &std::path::Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "init.defaultBranch=main"])
        .args(args)
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap();
    assert!(!out.status.success(), "git {args:?} was to stop on a conflict");
}

/// **What a prompt says past the counts** (the workspace row lab draft,
/// 2026-10-08): a branch with no upstream counts the commits no remote
/// has — none at all in a checkout with no remote — and an operation
/// left half done is named, its conflicts counted as conflicts.
#[test]
fn a_segment_names_an_operation_and_counts_what_no_remote_has() {
    use khor_core::GitOperation as O;
    let r = root("operation");
    let origin = r.0.join("origin.git");
    let repo = r.0.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&r.0, &["init", "-q", "--bare", "origin.git"]);
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "first"]);
    let seg = |why: &str| {
        let s = khor_node::git::segment(&repo).unwrap();
        eprintln!("[segment] {why}: ahead {} behind {} ~{} {:?}", s.ahead, s.behind, s.conflicted, s.operation);
        s
    };
    assert_eq!((seg("no remote").ahead, seg("no remote").operation), (0, None), "no remote, no count");

    // Pushed without `-u`: on the remote, and no upstream configured.
    git(&repo, &["remote", "add", "origin", origin.to_str().unwrap()]);
    git(&repo, &["push", "-q", "origin", "main"]);
    assert_eq!(seg("main pushed, no upstream").ahead, 0);
    git(&repo, &["checkout", "-q", "-b", "topic"]);
    fs::write(repo.join("b.txt"), "b\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "topic b"]);
    fs::write(repo.join("a.txt"), "topic\n").unwrap();
    git(&repo, &["commit", "-q", "-am", "topic a"]);
    let s = seg("topic never pushed");
    assert_eq!((s.branch.as_deref(), s.ahead, s.behind), (Some("topic"), 2, 0), "⇡2 though no upstream");

    git(&repo, &["checkout", "-q", "main"]);
    fs::write(repo.join("a.txt"), "main\n").unwrap();
    git(&repo, &["commit", "-q", "-am", "main a"]);
    assert_eq!(seg("main one ahead of origin").ahead, 1);

    git_stops(&repo, &["merge", "-q", "topic"]);
    let s = seg("merging");
    assert_eq!((s.operation, s.conflicted), (Some(O::Merge), 1), "合并中 ~1");
    git(&repo, &["merge", "--abort"]);

    git_stops(&repo, &["cherry-pick", "topic"]);
    assert_eq!(seg("picking").operation, Some(O::CherryPick));
    git(&repo, &["cherry-pick", "--abort"]);

    let first = String::from_utf8(
        Command::new("git").arg("-C").arg(&repo).args(["rev-list", "--max-parents=0", "HEAD"]).output().unwrap().stdout,
    )
    .unwrap();
    git_stops(&repo, &["revert", "--no-edit", first.trim()]);
    assert_eq!(seg("reverting").operation, Some(O::Revert));
    git(&repo, &["revert", "--abort"]);

    git(&repo, &["checkout", "-q", "topic"]);
    git_stops(&repo, &["rebase", "main"]);
    assert_eq!(seg("rebasing").operation, Some(O::Rebase));
    git(&repo, &["rebase", "--abort"]);

    git(&repo, &["bisect", "start"]);
    assert_eq!(seg("bisecting").operation, Some(O::Bisect));
    git(&repo, &["bisect", "reset"]);
    assert_eq!(seg("done").operation, None);

    git(&repo, &["checkout", "-q", "--detach"]);
    assert_eq!(seg("detached").ahead, 0, "detached: no branch to count for");
}
