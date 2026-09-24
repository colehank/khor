//! **A node pins its own working directory** (docs/SHELL.md 第十四轮
//! 第 4 条).
//!
//! The user opened a git checkout in the workspace and read
//! 「"…" does not appear to be a git repository」 over a directory that
//! is plainly a repository — the avatar beside it, one `stat` away, had
//! already said so. It was never git's judgment: the install script
//! starts the app with `nohup` from inside the export directory and
//! then deletes that directory, so the process stands nowhere, and
//! `gix::open` goes through the current directory even when the path it
//! is handed is absolute.
//!
//! So this is the shape of the bug, reproduced: stand in a directory,
//! have it deleted underneath, and then ask. **A node opened from
//! nowhere must still read a repository by its absolute path.**
//!
//! Its own test binary on purpose: the current directory is process
//! state, and a test that moves it cannot share a process with tests
//! that read it.

use std::path::{Path, PathBuf};

/// A fresh directory under `$TMPDIR`, removed if a previous run left one.
fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-cwdpin-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    // The macOS `$TMPDIR` is a symlink (`/var` → `/private/var`), and
    // git's own idea of a path has the links resolved — a row's path is
    // compared against this, so resolve it once here.
    std::fs::canonicalize(&p).unwrap_or(p)
}

fn wipe(p: &Path) {
    let _ = std::fs::remove_dir_all(p);
}

#[test]
fn a_node_opened_from_a_deleted_directory_still_reads_a_repository() {
    let home = temp("home");
    let repo = temp("repo");
    gix::init(&repo).expect("a repository to read");
    std::fs::write(repo.join("a.txt"), "hello\n").unwrap();

    // Stand where the install script stood, and then lose the ground.
    let gone = temp("gone");
    std::env::set_current_dir(&gone).unwrap();
    wipe(&gone);

    // The bug itself, asserted rather than assumed: from nowhere, git
    // refuses — this is the sentence the user read on screen.
    let refused = khor_node::git::status(&repo);
    assert!(
        refused.is_err(),
        "the premise is gone: git read the repository from a deleted directory without the pin, \
         so this test no longer measures anything",
    );

    // Opening the node is what puts the process somewhere again.
    let node = khor_node::Node::open(home.clone()).expect("the node opens");

    let status = khor_node::git::status(&repo)
        .expect("git::status reads an absolute path after the node has pinned the directory");
    assert!(
        status.rows.iter().any(|(path, _)| path == "a.txt"),
        "the walk found {:?}, not the file that is plainly there",
        status.rows,
    );

    drop(node);
    wipe(&home);
    wipe(&repo);
}
