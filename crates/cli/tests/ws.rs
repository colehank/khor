//! `khor ws` through the real binary (工作区批①): a workspace is a
//! directory somebody named, so the verbs that name it are the whole
//! feature and the list is what they add up to.
//!
//! **Through the binary and not the library**, because the thing being
//! held here is the third foundation (docs/KHOR.md: CLI 与 GUI 等价) —
//! every column the landing paints has to be reachable by typing, and
//! the only way to prove typing reaches it is to type.
//!
//! No serve, no network: every assertion below is about this machine's
//! own copy of the table, which is the design (`khor_sync::workspaces`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn khor(home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home).env("KHOR_NAME", "box").env_remove("KHOR_SESSION");
    c
}

/// Runs a verb and hands back `(ok, stdout)` — refusals are as much a
/// subject here as successes, so failure is not an assertion
/// (`agents.rs`'s helper).
fn run(home: &Path, args: &[&str]) -> (bool, String) {
    let out = khor(home).args(args).output().expect("khor runs");
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A fake home that takes itself away.
///
/// **Cleared on the way out and not only on the way in.** Clearing
/// first makes a rerun correct; clearing after is what keeps the
/// machine's temp directory from growing one home per run forever.
/// `Drop` rather than a line at the end of each test, because an
/// assertion panics past its cleanup and a `Drop` does not (`open.rs`'s
/// `Closer` learned that the same way).
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

fn home(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-ws-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

/// The whole loop on one machine: add, see it, float it, take it off.
///
/// **The name is checked and not merely the path**, because the name is
/// the one thing on a row that is derived rather than stored — a row
/// that printed the whole path where the landing prints `khor` would be
/// two faces disagreeing about what a workspace is called.
#[test]
fn a_directory_becomes_a_row_named_after_itself_and_comes_back_off() {
    let h = home("loop");
    let one = h.join("working/test");
    let two = h.join("working/other");
    std::fs::create_dir_all(&one).unwrap();
    std::fs::create_dir_all(&two).unwrap();

    assert_eq!(run(&h, &["ws", "ls"]).1, "", "nothing is a workspace until somebody says so");

    assert!(run(&h, &["ws", "add", one.to_str().unwrap()]).0);
    assert!(run(&h, &["ws", "add", two.to_str().unwrap()]).0);
    let listed = run(&h, &["ws", "ls"]).1;
    let rows: Vec<&str> = listed.lines().collect();
    assert_eq!(rows.len(), 2, "{listed}");
    assert!(
        rows.iter().any(|r| r.split('\t').nth(1) == Some("test")),
        "the name is the directory's last component: {listed}"
    );
    // Compared resolved: the row carries the physical spelling, and on
    // a Mac `temp_dir()` is itself a link (`/var` → `/private/var`).
    let two_real = std::fs::canonicalize(&two).unwrap().display().to_string();
    assert!(
        rows[0].ends_with(&format!("box:{two_real}")) || rows[1].ends_with(&format!("box:{two_real}")),
        "the row says which machine holds the path: {listed}"
    );

    // Pinned leads, whatever the paths sort like — `test` sorts after
    // `other`, so this cannot pass by accident.
    assert!(run(&h, &["ws", "pin", one.to_str().unwrap()]).0);
    let listed = run(&h, &["ws", "ls"]).1;
    assert_eq!(
        listed.lines().next().unwrap().split('\t').nth(1),
        Some("test"),
        "a pinned row leads: {listed}"
    );
    // The mark itself comes from the catalog, never spelled here: a
    // second copy of a user-facing word is one restyle away from an
    // assertion about a sentence nobody says any more (handoff 001).
    assert!(
        listed.lines().next().unwrap().starts_with(khor_catalog::cli::PINNED_MARK),
        "and says so: {listed}"
    );

    // …and unpinning puts it back in path order without taking it off
    // the list — the middle state a two-table design would have lost.
    assert!(run(&h, &["ws", "unpin", one.to_str().unwrap()]).0);
    let listed = run(&h, &["ws", "ls"]).1;
    assert_eq!(
        listed.lines().next().unwrap().split('\t').nth(1),
        Some("other"),
        "unpinned, path order returns: {listed}"
    );
    assert_eq!(listed.lines().count(), 2, "and it is still a workspace: {listed}");

    assert!(run(&h, &["ws", "rm", one.to_str().unwrap()]).0);
    let listed = run(&h, &["ws", "ls"]).1;
    assert_eq!(listed.lines().count(), 1, "{listed}");
    assert!(!listed.contains("test"), "{listed}");
    // **The directory is not touched** (user, 2026-09-14): removing a
    // workspace says「I am not working on this」, not「delete it」.
    assert!(one.is_dir(), "rm must take the row, never the directory");
}

/// A path that is not a directory on this machine is refused at the
/// moment somebody is there to be told, rather than becoming a row that
/// will never open.
#[test]
fn adding_something_that_is_not_a_directory_is_refused() {
    let h = home("refuse");
    let file = h.join("notes.md");
    std::fs::write(&file, "x").unwrap();

    let (ok, _) = run(&h, &["ws", "add", &h.join("nowhere").display().to_string()]);
    assert!(!ok, "a directory that is not there is not a workspace");
    let (ok, _) = run(&h, &["ws", "add", file.to_str().unwrap()]);
    assert!(!ok, "a file is not a workspace either");
    assert_eq!(run(&h, &["ws", "ls"]).1, "", "and neither left a row behind");
}

/// `khor ws add .` is what a person types, and a relative path names a
/// different directory on every machine that later reads the row — so
/// it is made absolute here, against this terminal's own cwd.
#[test]
fn a_dot_becomes_the_directory_it_was_typed_in() {
    let h = home("dot");
    let dir = h.join("working/here");
    std::fs::create_dir_all(&dir).unwrap();

    let out = khor(&h).args(["ws", "add", "."]).current_dir(&dir).output().expect("khor runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let listed = run(&h, &["ws", "ls"]).1;
    // `canonicalize` resolves the symlinks the temp directory is
    // reached through, so the row is compared the same way.
    let want = std::fs::canonicalize(&dir).unwrap();
    assert!(listed.trim_end().ends_with(&format!("box:{}", want.display())), "{listed}");
    assert_eq!(listed.lines().next().unwrap().split('\t').nth(1), Some("here"), "{listed}");
}

/// **Declared through a symlink, stored resolved.** A session records
/// `current_dir()`, which has no links in it; a row keyed by the linked
/// spelling would never match the sessions inside it (smoke 批⑥ found
/// exactly this on a `target/` that points at another disk).
#[test]
fn a_directory_added_through_a_link_is_stored_as_the_real_one() {
    let h = home("link");
    let real = h.join("disk/project");
    std::fs::create_dir_all(&real).unwrap();
    let link = h.join("working/project");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();

    assert!(run(&h, &["ws", "add", link.to_str().unwrap()]).0);
    let listed = run(&h, &["ws", "ls"]).1;
    let real_s = std::fs::canonicalize(&real).unwrap().display().to_string();
    assert!(
        listed.trim_end().ends_with(&format!("box:{real_s}")),
        "the real path, not the link: {listed}"
    );
    assert!(!listed.contains("working/project"), "{listed}");
}
