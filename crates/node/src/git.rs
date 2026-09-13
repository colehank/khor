//! What git already knows about a workspace, read in this process.
//!
//! **Read-only, and that is the whole of it for now.** Nothing here
//! writes a ref, an index or an object; the two questions it answers
//! are「which files here differ from HEAD」and「where in this one file」.
//! Committing is its own decision with its own batch.
//!
//! **In-process rather than `git` on the PATH.** khor's promise is that
//! the user side installs nothing (docs/KHOR.md), and a checkout is a
//! directory with a `.git` in it — which is not the same fact as a git
//! binary being installed, especially on the machines khor is for: a
//! rented box somebody ssh'd into once. It is also the same call this
//! tree already made for the GPU gauge (`vitals/gpu.rs`): a reading
//! taken while a person watches a screen does not justify spawning a
//! process per refresh.
//!
//! **Measured cost of that call, off the Cargo.lock diff rather than
//! estimated: 66 packages** (797 → 863), 51 of them `gix-*` — one
//! family, one author, one license (MIT OR Apache-2.0, checked before
//! use, not after). Four of the 51 are git's *network* stack
//! (`gix-protocol`, `gix-transport`, `gix-url`, `gix-packetline`),
//! which `gix` depends on unconditionally; khor makes no connection
//! through them, and `lto`+`strip` (workspace `[profile.release]`)
//! is what keeps them out of the shipped bytes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gix::bstr::{BString, ByteSlice};

use crate::proto::Hunk;

/// **git's own four letters, not khor's six words.** 「git 相关操作应该
/// 用符合使用习惯的英文」— the user, 2026-09-10. A person who reads
/// these already knows what they mean from every other tool, and a
/// translation would be a fifth vocabulary for a fact that is not
/// khor's to name. They are also not a state word (docs/UX.md 六个
///状态词): those belong to sessions, and a file is not a session.
pub const MODIFIED: &str = "M";
pub const ADDED: &str = "A";
pub const DELETED: &str = "D";
pub const UNTRACKED: &str = "U";

/// How many marked paths one answer carries. A repository whose
/// `.gitignore` misses a build directory can have tens of thousands of
/// untracked files, and the tree that paints them shows one directory
/// at a time — so the cut is a screen's problem, but the frame is
/// `proto::MAX_FRAME` and that is this file's.
pub(crate) const ROWS: usize = 5000;

/// How many bytes of「what HEAD had there」one diff carries, across
/// every hunk. Under `proto::SLICE`, which is the same number the file
/// view already lives inside.
pub(crate) const WAS: usize = 256 * 1024;

/// And a ceiling on the hunks themselves, for the shape the byte cap
/// cannot catch: a file that is all insertion carries no old lines at
/// all, so its hunks are free by that measure and still have to stop.
pub(crate) const HUNKS: usize = 2000;

/// One workspace's git state, as of the moment it was read.
#[derive(Debug, Clone)]
pub struct Status {
    /// The branch HEAD is on, short (`main`, not `refs/heads/main`).
    /// **Empty means there is no name to say** — a detached HEAD, or a
    /// repository with no commit yet. The consumer paints nothing
    /// rather than inventing a word for it:「没有就不显示」.
    pub branch: String,
    /// `(path relative to the workspace root, mark)`, sorted by path.
    /// Sorted here because the screen paints and never re-sorts
    /// (docs/UX.md 状态呈现), and because the walk that produced them
    /// runs three threads and is documented to have no order at all.
    pub rows: Vec<(String, String)>,
    /// The list stopped at [`ROWS`] rather than being whole. The
    /// no-silent-caps rule `Response::Dir::truncated` already carries.
    pub more: bool,
}

/// Every hunk in one file, plus whether the list is whole.
///
/// **The hunk itself is [`crate::proto::Hunk`]**, not a second struct
/// beside it. Its three-shape contract is a wire contract — the gutter
/// that reads it is on another machine — and a copy here would be a
/// second place for that table to be remembered, which is how the two
/// spellings of「a deletion sits after」would eventually disagree.
#[derive(Debug, Clone)]
pub struct Diff {
    pub hunks: Vec<Hunk>,
    pub more: bool,
}

/// The checkout `path` is inside, or `None`. One call into
/// [`crate::place::checkout_root`] rather than a second walk with its
/// own idea of where a repository starts — that function is where the
/// home-directory exception is remembered, and a copy of this walk that
/// forgot it would answer with somebody's dotfiles repo.
fn repo_root(path: &Path) -> Option<PathBuf> {
    let dir = if path.is_dir() { path } else { path.parent()? };
    crate::place::checkout_root(dir)
}

/// What differs from HEAD across one workspace.
///
/// `root` must be the checkout itself — the workspace list only ever
/// hands over directories that folded as one (`Place::checkout`).
pub fn status(root: &Path) -> Result<Status, String> {
    let repo = gix::open(root).map_err(|e| e.to_string())?;
    let branch = repo
        .head_name()
        .map_err(|e| e.to_string())?
        .map(|n| n.shorten().to_str_lossy().into_owned())
        .unwrap_or_default();

    // A map rather than a list, because the same path can arrive twice:
    // once from the tree↔index comparison and once from the
    // index↔worktree one, and a file that is both staged and edited
    // again is one row on a screen. The order the three producers run
    // in is documented as undefined, so **which** of the two arrives
    // first cannot be leaned on — `fold` below picks the same answer
    // either way.
    let mut marks: BTreeMap<String, &'static str> = BTreeMap::new();
    let mut more = false;
    let items = repo
        .status(gix::progress::Discard)
        .map_err(|e| e.to_string())?
        .into_iter(Vec::<BString>::new())
        .map_err(|e| e.to_string())?;
    for item in items {
        let item = item.map_err(|e| e.to_string())?;
        let Some(mark) = mark_of(&item) else { continue };
        if marks.len() >= ROWS && !marks.contains_key(item.location().to_str_lossy().as_ref()) {
            more = true;
            break;
        }
        let path = item.location().to_str_lossy().into_owned();
        let slot = marks.entry(path).or_insert(mark);
        *slot = fold(*slot, mark);
    }
    let rows = marks.into_iter().map(|(p, m)| (p, m.to_owned())).collect();
    Ok(Status { branch, rows, more })
}

/// Which of two marks for one path survives.
///
/// **The worktree's answer beats the index's**, because the file on
/// disk is the copy the screen is showing. A file staged as new and
/// then deleted reads `D`, not `A`: `A` would send somebody looking for
/// a file that is not there.
pub(crate) fn fold(a: &'static str, b: &'static str) -> &'static str {
    for want in [DELETED, UNTRACKED, ADDED] {
        if a == want || b == want {
            return want;
        }
    }
    MODIFIED
}

/// One status item's letter, or `None` for the items that are not a
/// change at all — an entry whose stat needs refreshing, an ignored
/// path the walk still emitted.
fn mark_of(item: &gix::status::Item) -> Option<&'static str> {
    use gix::status::Item;
    use gix::status::index_worktree::Item as Work;
    use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
    match item {
        Item::TreeIndex(change) => Some(match change {
            gix::diff::index::Change::Addition { .. } => ADDED,
            gix::diff::index::Change::Deletion { .. } => DELETED,
            _ => MODIFIED,
        }),
        Item::IndexWorktree(Work::Modification { status, .. }) => match status {
            EntryStatus::Change(Change::Removed) => Some(DELETED),
            EntryStatus::Change(_) => Some(MODIFIED),
            EntryStatus::Conflict { .. } => Some(MODIFIED),
            // `IntentToAdd` is a promise, not a change; `NeedsUpdate`
            // is the walk telling itself to refresh a stat. Neither is
            // a difference a person made.
            EntryStatus::IntentToAdd | EntryStatus::NeedsUpdate(_) => None,
        },
        Item::IndexWorktree(Work::DirectoryContents { entry, .. }) => {
            use gix::dir::entry::Status as Dir;
            match entry.status {
                Dir::Untracked => Some(UNTRACKED),
                _ => None,
            }
        }
        Item::IndexWorktree(Work::Rewrite { .. }) => Some(MODIFIED),
    }
}

/// Where one file differs from HEAD.
///
/// **A file git has never seen is one insertion, not an error.** That
/// falls out of diffing against an empty side, and it is the right
/// answer: the gutter for a new file is a bar down the whole of it,
/// which is what every editor draws.
pub fn diff(path: &Path) -> Result<Diff, String> {
    let root = repo_root(path).ok_or_else(|| khor_catalog::msg::NO_CHECKOUT.to_owned())?;
    let repo = gix::open(&root).map_err(|e| e.to_string())?;
    let rel = path.strip_prefix(&root).map_err(|e| e.to_string())?;
    let rel = rel.to_string_lossy().replace('\\', "/");

    let old = head_blob(&repo, &rel)?;
    let new = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(hunks(&old, &new))
}

/// What HEAD has at `rel`, or empty bytes when it has nothing there —
/// a file that is new, or a repository with no commit yet.
fn head_blob(repo: &gix::Repository, rel: &str) -> Result<Vec<u8>, String> {
    let Ok(tree) = repo.head_tree() else { return Ok(Vec::new()) };
    let found = tree.lookup_entry_by_path(rel).map_err(|e| e.to_string())?;
    let Some(entry) = found else { return Ok(Vec::new()) };
    let obj = entry.object().map_err(|e| e.to_string())?;
    Ok(obj.data.clone())
}

/// The diff itself, over two byte buffers and nothing else — a pure
/// function, so its four shapes are testable without a repository on
/// disk.
fn hunks(old: &[u8], new: &[u8]) -> Diff {
    use gix::diff::blob::{Algorithm, InternedInput, sources};
    let input = InternedInput::new(sources::byte_lines(old), sources::byte_lines(new));
    let mut d = gix::diff::blob::Diff::compute(Algorithm::Histogram, &input);
    // Slides a hunk to the boundary a person would have put it at —
    // the difference between「this block was added」and「the last line
    // of the old block and all but the last of the new one」.
    d.postprocess_lines(&input);

    let mut out = Vec::new();
    let mut spent = 0usize;
    let mut more = false;
    for h in d.hunks() {
        if out.len() >= HUNKS || spent >= WAS {
            more = true;
            break;
        }
        // `was` is decoded lossily on purpose: what is on the screen is
        // already a decoded file (`gui_core::files::shape` decided it
        // was text), and a byte that does not decode is one unreadable
        // character in an expanded hunk rather than a reason to refuse
        // the whole gutter.
        //
        // **Each line without its terminator.** A `Vec` of lines already
        // says where each one ends, so a trailing `\n` on every element
        // would be a second spelling of the same fact — and one the
        // consumer would have to strip before painting, in a place that
        // has no business knowing how the file ended. `\r\n` goes too:
        // a repository checked out on Windows would otherwise show a
        // stray glyph at the end of every removed line.
        let was: Vec<String> = h
            .before
            .clone()
            .map(|i| {
                let raw = String::from_utf8_lossy(input.interner[input.before[i as usize]]);
                raw.strip_suffix('\n')
                    .map(|s| s.strip_suffix('\r').unwrap_or(s))
                    .unwrap_or(&raw)
                    .to_owned()
            })
            .collect();
        spent += was.iter().map(|s| s.len() + 1).sum::<usize>();
        out.push(Hunk { at: h.after.start + 1, lines: h.after.end - h.after.start, was });
    }
    // A pure deletion has no current line to sit on, so `at` names the
    // line it follows — one below what the loop above wrote, which
    // assumed there was something to point at.
    for h in out.iter_mut() {
        if h.lines == 0 {
            h.at -= 1;
        }
    }
    Diff { hunks: out, more }
}

#[cfg(test)]
mod hunk_tests {
    use super::*;

    fn shape(old: &str, new: &str) -> Vec<(u32, u32, Vec<String>)> {
        hunks(old.as_bytes(), new.as_bytes())
            .hunks
            .into_iter()
            .map(|h| (h.at, h.lines, h.was))
            .collect()
    }

    #[test]
    fn a_file_git_has_never_seen_is_one_insertion() {
        // Not an error and not a refusal: the whole file is new, which
        // is the bar every editor draws down the whole gutter.
        assert_eq!(shape("", "a\nb\nc\n"), vec![(1, 3, vec![])]);
    }

    #[test]
    fn a_changed_line_carries_what_head_had() {
        // The one shape the expansion exists for: both halves present.
        assert_eq!(shape("a\nb\nc\n", "a\nB\nc\n"), vec![(2, 1, vec!["b".to_owned()])]);
    }

    #[test]
    fn a_deletion_names_the_line_it_follows() {
        // There is no current line to point at, so `at` is the line
        // above and `lines` is 0 — which is also how the consumer tells
        // a deletion from the other two.
        assert_eq!(shape("a\nb\nc\n", "a\nc\n"), vec![(1, 0, vec!["b".to_owned()])]);
    }

    #[test]
    fn a_deletion_at_the_top_has_no_line_above_it() {
        assert_eq!(shape("a\nb\n", "b\n"), vec![(0, 0, vec!["a".to_owned()])]);
    }

    #[test]
    fn an_unchanged_file_has_no_hunks() {
        assert!(shape("a\nb\n", "a\nb\n").is_empty());
    }

    #[test]
    fn two_edits_far_apart_stay_two_hunks() {
        // What the「导航数得对」half of this batch rests on: the count
        // is hunks, not changed lines.
        let out = shape("a\nb\nc\nd\ne\nf\n", "A\nb\nc\nd\ne\nF\n");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, 1);
        assert_eq!(out[1].0, 6);
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;

    /// The checkout this test is compiled inside. Not a fixture built
    /// in a temp directory: creating a repository needs gix's
    /// worktree-mutation half, which khor does not compile in, and the
    /// thing worth checking here is that the real walk answers at all.
    fn here() -> PathBuf {
        repo_root(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("khor is a checkout")
    }

    #[test]
    fn a_real_checkout_answers_with_a_branch() {
        let s = status(&here()).expect("status");
        assert!(!s.branch.is_empty(), "HEAD has a name in this repo");
    }

    #[test]
    fn every_marked_row_carries_one_of_the_four_letters() {
        let s = status(&here()).expect("status");
        for (path, mark) in &s.rows {
            assert!(
                [MODIFIED, ADDED, DELETED, UNTRACKED].contains(&mark.as_str()),
                "{path} got {mark}"
            );
        }
    }

    #[test]
    fn a_file_outside_any_checkout_says_so_rather_than_failing_to_open() {
        let err = diff(Path::new("/etc/hosts")).expect_err("no checkout above /etc");
        assert_eq!(err, khor_catalog::msg::NO_CHECKOUT);
    }
}
