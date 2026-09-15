//! **常去的地方: the directories this machine's own records say work
//! happened in, folded to the project each belongs to.**
//!
//! # This is a suggestion list, not the workspace list
//!
//! It used to be both — the workspace landing was this fold with a
//! checkout filter over it. The user ended that on 2026-09-14:
//! 「用户有agent记录的目录不一定是工作区.可能只是随手打开的」. A
//! directory a session once ran in is a fine thing to *offer* when
//! somebody is about to start another one, and a poor thing to assert
//! somebody works in. Workspaces are declared and live in a replicated
//! table now (`crate::workspace`, `khor_sync::workspaces`); this stays
//! exactly what it always was underneath — the wizard's completions.
//!
//! Nothing here is stale, because nothing here is kept: the vendors
//! record where each session ran, and this reads that.
//!
//! # Why the project root and not the working directory
//!
//! The raw directory is too fine to group by. Measured on this machine
//! the same tree was open at two depths at once:
//!
//! ```text
//! /Users/zgh/Desktop/working/tokens/apps/desktop/src-tauri
//! /Users/zgh/Desktop/working/tokens
//! ```
//!
//! Two groups by cwd, **one** by project root — and the second is what
//! a person means by 工作区. It also survives a `cd` inside the tree,
//! which the cwd does not: the key is steadier than the fact it is
//! derived from.

use std::path::{Path, PathBuf};

/// One directory people work in, and how much has happened there.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Place {
    /// The project root, absolute.
    pub path: String,
    /// Recorded sessions that folded into it. A count, not an
    /// inventory: it counts what the vendors' own files remember, and
    /// says so wherever it is shown.
    pub sessions: u32,
    /// The newest of them, ms since the epoch.
    pub last_ms: i64,
}

/// The project a directory belongs to — `git rev-parse --show-toplevel`
/// without the subprocess, falling back to the directory itself.
///
/// **The nearest ancestor wins**, which is git's own answer: inside a
/// submodule or a nested checkout the innermost repo is the project you
/// are in, not the one containing it.
///
/// `.git` is tested with `exists`, not `is_dir`, on purpose — in a
/// linked worktree (`git worktree add`, which this project's own
/// discipline reaches for) it is a *file* pointing at the real one, and
/// an `is_dir` test would walk straight past a checkout and answer with
/// somebody's home directory.
///
/// No marker anywhere means the directory is its own project. That is
/// the honest answer for `/tmp/scratch`, and it is why this never
/// returns something the caller did not pass in the first place.
///
/// # The home directory is never somebody's project
///
/// A checkout **at** `$HOME` is a dotfiles repo, and `git rev-parse
/// --show-toplevel` will happily answer with it. Following git there
/// makes every directory under home that is not itself a checkout fold
/// into one bucket — measured on this machine the first time this ran,
/// `~` came back as **the largest place, 73 sessions**, ahead of every
/// real project. That is the failure this codebase already has a name
/// for: a branch that cannot answer does not go blank, it becomes its
/// neighbour, and here the neighbour was the one group that means
/// nothing.
///
/// So home's own marker is passed over. Work that genuinely happened in
/// `~` still places there — that comes out of the fallback, not out of
/// the marker — and a directory under home with no checkout of its own
/// stays itself, which is exactly the place the person was standing in.
pub fn project_root(dir: &Path) -> PathBuf {
    checkout_root(dir).unwrap_or_else(|| dir.to_path_buf())
}

/// The nearest enclosing checkout, or `None` when the walk found no
/// marker at all.
///
/// **The same walk as [`project_root`], with the fallback withheld.**
/// It exists because two callers want different halves of one answer.
/// Grouping wants an answer for every session, so it takes the
/// fallback: a shell opened in `/tmp/scratch` belongs *somewhere*, and
/// that somewhere is `/tmp/scratch`. A git question wants the other
/// half: a file outside every checkout has no repository to be read
/// against, and `None` is the honest answer (`crate::git::repo_root`).
///
/// Written as one walk with one wrapper on purpose. Two walks would be
/// two places for the home-directory exception above to be remembered,
/// and the one that forgot it would answer with a dotfiles repo.
pub fn checkout_root(dir: &Path) -> Option<PathBuf> {
    let home = std::env::home_dir();
    for step in dir.ancestors() {
        if home.as_deref() == Some(step) {
            continue;
        }
        if step.join(".git").exists() {
            return Some(step.to_path_buf());
        }
    }
    None
}

/// A path as a person reads it: their own home written `~`.
///
/// Only a whole leading component is replaced — `/Users/zghost` does not
/// begin at `/Users/zgh`'s home, and a prefix test on the string alone
/// would say it does.
pub fn shorten(path: &Path) -> String {
    let shown = path.display().to_string();
    let Some(home) = std::env::home_dir() else { return shown };
    shorten_under(&shown, &home.display().to_string())
}

/// [`shorten`] with the home handed in.
///
/// **Split because whose home a path is under is a fact about the
/// machine holding it** — a row read off another machine has to be
/// shortened with *that* machine's home, or `~` lands where the
/// sponsor's home happens to match and the far machine's own home is
/// spelled out in full, wrong in both directions at once.
///
/// ⚠️ **The caller that needed that is gone** (2026-09-15): the
/// satellite road read the far home over the same ssh that read the
/// rows, and it went with the derived workspace list it existed for.
/// What is left is [`shorten`]'s own body and a seam its test uses, and
/// the argument above is kept because the next caller reading a far
/// machine's paths will need it before it needs anything else here.
///
/// An empty home shortens nothing, which is the honest answer when the
/// far side did not say.
pub fn shorten_under(shown: &str, home: &str) -> String {
    if home.is_empty() {
        return shown.to_owned();
    }
    if shown == home {
        return "~".to_owned();
    }
    match shown.strip_prefix(&format!("{home}/")) {
        Some(rest) => format!("~/{rest}"),
        None => shown.to_owned(),
    }
}

/// Folds recorded working directories into places: `(cwd, how many
/// sessions were there, when the newest of them was)`.
///
/// **A place that is gone is not a place to start**, so a root that no
/// longer exists is dropped. Note that a *deleted subdirectory* does
/// not drop its project: the walk up in [`project_root`] lands on the
/// checkout, which is still there — the fold repairs those rather than
/// losing them.
///
/// Newest first, and ties broken by path so two machines listing the
/// same history print it in the same order.
pub fn fold(seen: impl IntoIterator<Item = (PathBuf, u32, i64)>) -> Vec<Place> {
    merge(seen.into_iter().filter_map(|(dir, count, at)| {
        let root = project_root(&dir);
        if !root.is_dir() {
            return None;
        }
        Some((root.display().to_string(), count, at))
    }))
}

/// The half of [`fold`] that touches no disk: rooted rows in, one row
/// per root out, newest first.
///
/// **Split because rows can be rooted somewhere else.** Rooting a
/// directory means asking whether `.git` is above it and whether the
/// root still exists, and for a machine only ssh can reach both
/// questions are answered over there. What must not be answered twice
/// is how duplicates combine and how the list is ordered — two lists
/// sorted by two rules would be one landing painting two orders, and
/// which one you read would depend on which machine a row came from.
///
/// ⚠️ **Only [`fold`] calls this today** (2026-09-15): the satellite
/// road that rooted its own rows went with the derived workspace list.
/// The seam is kept for the reason above, which is about the next such
/// caller rather than about the one that left.
pub fn merge(rooted: impl IntoIterator<Item = (String, u32, i64)>) -> Vec<Place> {
    let mut by: std::collections::HashMap<String, (u32, i64)> = std::collections::HashMap::new();
    for (root, count, at) in rooted {
        let entry = by.entry(root).or_insert((0, i64::MIN));
        entry.0 += count;
        entry.1 = entry.1.max(at);
    }
    let mut out: Vec<Place> = by
        .into_iter()
        .map(|(path, (sessions, last_ms))| Place { path, sessions, last_ms })
        .collect();
    out.sort_by(|a, b| b.last_ms.cmp(&a.last_ms).then_with(|| a.path.cmp(&b.path)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repo's own temp-dir convention (`crates/node/src/tests.rs`),
    /// which is also why this file needs no dev-dependency.
    fn root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("khor-place-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// A checkout is the project; everything under it folds into it.
    #[test]
    fn the_project_is_the_nearest_checkout_above_a_directory() {
        let tmp = root("nearest");
        let repo = tmp.join("repo");
        let deep = repo.join("apps/gui/src");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();

        assert_eq!(project_root(&deep), repo);
        assert_eq!(project_root(&repo), repo);

        // **The control: no marker, no invention.** A directory that
        // belongs to no checkout is its own project — this is what
        // stops the walk from answering with somebody's home.
        let loose = tmp.join("loose");
        std::fs::create_dir_all(&loose).unwrap();
        assert_eq!(project_root(&loose), loose);
    }

    /// **A dotfiles repo at `$HOME` is not a workspace.** Following
    /// git here folded every non-checkout directory under home into one
    /// bucket, and on the machine this was written on that bucket was
    /// the *largest* place in the list — 73 sessions, ahead of every
    /// real project.
    ///
    /// The control arm is the second half: work that really did happen
    /// in `~` must still place there. A guard that answered "never
    /// home" would pass the first assertion and lose that.
    #[test]
    fn a_checkout_at_home_does_not_swallow_everything_under_it() {
        let home = std::env::home_dir().expect("a home directory");
        if !home.join(".git").exists() {
            // The machine this runs on decides whether this case
            // exists. Saying so beats a silent pass.
            eprintln!("skipped: {} is not a checkout", home.display());
            return;
        }
        let under = home.join("Downloads");
        if under.is_dir() {
            assert_eq!(project_root(&under), under, "a plain directory under home is itself");
        }
        assert_eq!(project_root(&home), home, "and work done in ~ still places in ~");
    }

    /// **No line is drawn here any more.** Both of these are places a
    /// session ran and both fold — 常去的地方 offers every directory
    /// somebody stood in. The checkout filter that used to sit on top
    /// of this list was the workspace landing's, and the user ended it
    /// on 2026-09-14 (module head); what stays is the walk, whose
    /// withheld fallback is still what `git` asks for.
    #[test]
    fn a_directory_with_no_marker_folds_like_any_other_place() {
        let tmp = root("bare");
        let repo = tmp.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        let scratch = tmp.join("scratch");
        std::fs::create_dir_all(&scratch).unwrap();

        let places = fold([(repo.join("src"), 1, 200), (scratch.clone(), 1, 100)]);
        assert_eq!(places.len(), 2, "{places:?}");
        let has = |p: &std::path::Path| places.iter().any(|q| q.path == p.display().to_string());
        assert!(has(&repo), "the checkout folded to its root");
        assert!(has(&scratch), "and the directory with no marker is here too");
        assert_eq!(checkout_root(&scratch), None, "and the walk withholds its fallback");
    }

    /// A linked worktree's `.git` is a *file*. An `is_dir` test here
    /// walks past the checkout entirely, so this is what holds that
    /// line.
    #[test]
    fn a_worktree_is_a_project_even_though_its_git_is_a_file() {
        let tmp = root("worktree");
        let tree = tmp.join("tree");
        std::fs::create_dir_all(tree.join("src")).unwrap();
        std::fs::write(tree.join(".git"), "gitdir: /elsewhere/.git/worktrees/tree\n").unwrap();

        assert_eq!(project_root(&tree.join("src")), tree);
    }

    /// The nested case, which is the one that decides between "nearest"
    /// and "outermost": git answers with the inner checkout, so khor
    /// must too, or a submodule's sessions file under its parent.
    #[test]
    fn a_checkout_inside_a_checkout_is_its_own_project() {
        let tmp = root("nested");
        let outer = tmp.join("outer");
        let inner = outer.join("vendor/inner");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::create_dir_all(outer.join(".git")).unwrap();
        std::fs::create_dir_all(inner.join(".git")).unwrap();

        assert_eq!(project_root(&inner), inner);
        assert_eq!(project_root(&outer.join("apps")), outer);
    }

    /// **Two depths of one tree are one place.** This is the whole
    /// reason the key is the project and not the directory — both rows
    /// were observed open at once on the machine this was written on.
    #[test]
    fn two_depths_of_one_checkout_fold_into_one_place() {
        let tmp = root("fold");
        let repo = tmp.join("repo");
        let deep = repo.join("apps/desktop/src-tauri");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        let other = tmp.join("elsewhere");
        std::fs::create_dir_all(&other).unwrap();

        let places = fold([(deep, 3, 100), (repo.clone(), 2, 500), (other.clone(), 1, 900)]);

        // The control arm: an unrelated directory must stay its own
        // place. A fold that answered "one place" for everything would
        // pass the first assertion alone.
        assert_eq!(places.len(), 2, "{places:?}");
        assert_eq!(places[0].path, other.display().to_string(), "newest first");
        assert_eq!(places[1].path, repo.display().to_string());
        assert_eq!(places[1].sessions, 5, "both depths counted");
        assert_eq!(places[1].last_ms, 500, "the newest of the two");
    }

    /// A place nobody can stand in is not offered. Its *subdirectory*
    /// going missing is a different case and must not lose the project.
    #[test]
    fn a_place_that_is_gone_is_dropped_but_a_missing_subdirectory_is_not() {
        let tmp = root("gone");
        let repo = tmp.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        let gone = tmp.join("deleted-entirely");

        let places = fold([(repo.join("was-here"), 1, 100), (gone, 1, 900)]);

        assert_eq!(places.len(), 1, "{places:?}");
        assert_eq!(places[0].path, repo.display().to_string());
    }

    #[test]
    fn home_is_written_the_way_a_person_writes_it() {
        let home = std::env::home_dir().expect("a home directory");
        assert_eq!(shorten(&home), "~");
        assert_eq!(shorten(&home.join("Desktop/x")), "~/Desktop/x");
        // **Not a string prefix.** A sibling whose name merely starts
        // with the home directory's is not inside it.
        let sibling = PathBuf::from(format!("{}ost/x", home.display()));
        assert_eq!(shorten(&sibling), sibling.display().to_string());
    }
}
