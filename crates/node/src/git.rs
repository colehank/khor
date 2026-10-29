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
use serde::{Deserialize, Serialize};

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
/// 「还有 U 等状态,使用对应的颜色与标识」— the user, 2026-09-15. These
/// two were folded into `M` until then, which is the shape this repo
/// keeps naming: a fact that cannot be said does not go blank, it
/// becomes its neighbour. A conflicted file and an edited one lead to
/// completely different next moves, and a rename is the one change
/// where the path on screen is not the path in HEAD.
pub const CONFLICT: &str = "C";
pub const RENAMED: &str = "R";
/// A path `.gitignore` covers. **git's own porcelain spelling** (`!!`,
/// written once here because khor's column is one character wide) —
/// 「文件树中,被.gitignore的文件/目录名要变灰」, the user, 2026-09-16.
///
/// **It is not a change and it never folds into one.** Every other
/// letter says「this differs from HEAD」; this one says「git is not
/// looking at this」, which is why [`fold`] gives way to any real letter
/// rather than ranking it — a directory holding an edited file and a
/// build output is 黄, not grey.
///
/// Emitted collapsed (`CollapseDirectory`), so a `target/` with fifty
/// thousand files in it arrives as one row and [`ROWS`] is not spent on
/// something nobody asked to see.
pub const IGNORED: &str = "!";

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
    /// Commits on this branch that the remote-tracking branch does not
    /// have, and the other way round. **Both zero when there is no
    /// upstream at all**, which is also what「in step」looks like — the
    /// two are told apart by [`Status::fetched_ms`] being 0, and the
    /// face draws nothing in either case.
    pub ahead: u32,
    pub behind: u32,
    /// When this repository last heard from its remote, in epoch
    /// milliseconds; **0 means never**.
    ///
    /// **This is why ahead/behind was cut in 批3 and is what brings it
    /// back.** The pair is not「how far this branch is from the
    /// remote」, it is「how far it was when somebody last fetched」, and
    /// a number that is quietly three weeks old is worse than no
    /// number: it is the same shape as a fresh one. khor does not fetch
    /// — it opens no connection a person did not ask for — so the only
    /// honest form of the number is the number *with its age*, and that
    /// is what travels.
    ///
    /// Read off `.git/FETCH_HEAD`'s mtime, which is what git writes on
    /// every fetch and touches for nothing else.
    pub fetched_ms: i64,
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

/// Whether `root` is a checkout: it has a `.git` of its own, directory
/// or file (a linked worktree's `.git` is a file naming its repository).
///
/// **Asked apart from [`status`], and never made to wait for it**
/// (docs/SHELL.md 第五轮 第 6 条). A workspace row wears a branch or a
/// folder by this answer alone, and a status walk on a big tree or a
/// network disk can take seconds or fail outright — while the corner
/// sat on the walk, 「这是一个仓库」 was painted only once 「这个仓库里
/// 哪些文件变了」 had been answered, and a slow repository wore a
/// folder. This is one `stat`.
///
/// **The same marker [`status`] opens**: `gix::open(root)` reads the
/// `.git` at the root and does not walk upward, so a directory this
/// answers `false` for is one `status` refuses, and the other way round.
pub fn is_checkout(root: &Path) -> bool {
    root.join(".git").exists()
}

/// The control directories and reference identity a workspace observer must
/// cover. Missing ref directories are kept: the watcher starts at their
/// nearest existing ancestor and replans after creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitWatchPlan {
    pub directories: Vec<PathBuf>,
    pub reference: Option<BString>,
    pub upstream: Option<BString>,
}

pub(crate) fn watch_plan(path: &Path) -> Result<Option<GitWatchPlan>, String> {
    let Some(root) = repo_root(path) else {
        return Ok(None);
    };
    let repo = gix::open(root).map_err(|e| e.to_string())?;
    let absolute = |path: &Path| {
        if path.is_absolute() {
            path.to_owned()
        } else {
            repo.current_dir().join(path)
        }
    };
    let git_dir = absolute(repo.git_dir());
    let common_dir = absolute(repo.common_dir());
    let mut directories = std::collections::BTreeSet::from([git_dir.clone(), common_dir.clone()]);
    if let Some(parent) = repo.index_path().parent() {
        directories.insert(absolute(parent));
    }
    if let Some(worktree) = repo.workdir() {
        directories.insert(absolute(worktree));
    }
    let head = repo.head().map_err(|e| e.to_string())?;
    let reference = head.referent_name().map(|name| name.as_bstr().to_owned());
    let upstream = head
        .referent_name()
        .and_then(|name| repo.branch_remote_tracking_ref_name(name, gix::remote::Direction::Fetch))
        .transpose()
        .map_err(|e| e.to_string())?
        .map(|name| name.as_bstr().to_owned());
    for name in reference.iter().chain(upstream.iter()) {
        let relative = gix::path::from_bstr(name.as_bstr());
        if relative.components().count() > 64 {
            return Err("Git reference exceeds watch path depth budget".to_owned());
        }
        if let Some(parent) = relative.parent() {
            directories.insert(git_dir.join(parent));
            directories.insert(common_dir.join(parent));
        }
    }
    if directories.len() > 16 {
        return Err("Git control directories exceed watch budget".to_owned());
    }
    Ok(Some(GitWatchPlan {
        directories: directories.into_iter().collect(),
        reference,
        upstream,
    }))
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
        // **Ignored paths come back too, folded to their directory**
        // (2026-09-16). The walk would otherwise pass over them in
        // silence, and a name the tree paints like every other name is
        // khor saying git is watching a file it is not.
        // `CollapseDirectory` is what keeps that cheap: one row for a
        // whole build directory rather than one per file in it.
        .dirwalk_options(|o| o.emit_ignored(Some(gix::dir::walk::EmissionMode::CollapseDirectory)))
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
        *slot = fold(slot, mark);
    }
    let rows = marks.into_iter().map(|(p, m)| (p, m.to_owned())).collect();
    let (ahead, behind) = divergence(&repo).unwrap_or((0, 0));
    Ok(Status {
        branch,
        rows,
        more,
        ahead,
        behind,
        fetched_ms: last_fetch(&repo),
    })
}

/// **A workspace's git state as a prompt's git segment says it** (UX-48:
/// the workspace row's 「main ⇡861 *1 !1 ?2」, powerlevel10k's counts),
/// with HEAD's subject for the row's second line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Segment {
    /// The branch HEAD is on; `None` detached or before the first commit.
    pub branch: Option<String>,
    /// HEAD's id, eight hex digits; `None` before the first commit.
    pub head: Option<String>,
    /// HEAD's subject line.
    pub subject: Option<String>,
    /// [`divergence`]; with no upstream, [`unpushed`] and 0; `(0, 0)`
    /// detached.
    pub ahead: u32,
    pub behind: u32,
    /// Entries in the stash.
    pub stashes: u32,
    /// Paths by what differs where: conflicted, in the index against
    /// HEAD, in the worktree against the index, unknown to git. One path
    /// staged and edited again counts in both of its middle two, as
    /// `git status` lists it twice.
    pub conflicted: u32,
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    /// Distinct paths that differ, untracked included.
    pub changed: u32,
    /// The walk stopped at [`ROWS`] paths: the counts are a floor.
    pub more: bool,
    /// [`operation`]: what the checkout is in the middle of.
    pub operation: Option<khor_core::GitOperation>,
}

/// [`Segment`] for one checkout. **Blocking**: a status walk.
pub fn segment(root: &Path) -> Result<Segment, String> {
    use gix::status::Item;
    use gix::status::index_worktree::Item as Work;
    use gix::status::plumbing::index_as_worktree::EntryStatus;
    use std::collections::HashSet;
    let repo = gix::open(root).map_err(|e| e.to_string())?;
    let branch = repo
        .head_name()
        .map_err(|e| e.to_string())?
        .map(|n| n.shorten().to_str_lossy().into_owned());
    let commit = repo.head_commit().ok();
    let head = commit.as_ref().map(|c| c.id.to_hex_with_len(8).to_string());
    let subject = commit
        .as_ref()
        .and_then(|c| c.message().ok())
        .map(|m| m.summary().to_str_lossy().into_owned())
        .filter(|s| !s.is_empty());
    let mut sets: [HashSet<String>; 4] = Default::default();
    let mut all: HashSet<String> = HashSet::new();
    let mut more = false;
    let items = repo
        .status(gix::progress::Discard)
        .map_err(|e| e.to_string())?
        .into_iter(Vec::<BString>::new())
        .map_err(|e| e.to_string())?;
    for item in items {
        let item = item.map_err(|e| e.to_string())?;
        let which = match &item {
            Item::TreeIndex(_) => 1,
            Item::IndexWorktree(Work::Modification { status, .. }) => match status {
                EntryStatus::Conflict { .. } => 0,
                EntryStatus::Change(_) => 2,
                EntryStatus::IntentToAdd | EntryStatus::NeedsUpdate(_) => continue,
            },
            Item::IndexWorktree(Work::Rewrite { .. }) => 2,
            Item::IndexWorktree(Work::DirectoryContents { entry, .. }) => match entry.status {
                gix::dir::entry::Status::Untracked => 3,
                _ => continue,
            },
        };
        let path = item.location().to_str_lossy().into_owned();
        if all.len() >= ROWS && !all.contains(&path) {
            more = true;
            break;
        }
        all.insert(path.clone());
        sets[which].insert(path);
    }
    // A conflicted path is that and nothing else.
    let conflicted = std::mem::take(&mut sets[0]);
    for s in &mut sets[1..] {
        s.retain(|p| !conflicted.contains(p));
    }
    sets[0] = conflicted;
    let n = |s: &HashSet<String>| s.len().min(u32::MAX as usize) as u32;
    // A branch with no upstream still says what it has not pushed.
    let (ahead, behind) = divergence(&repo)
        .or_else(|| unpushed(&repo).map(|a| (a, 0)))
        .unwrap_or((0, 0));
    Ok(Segment {
        branch,
        head,
        subject,
        ahead,
        behind,
        stashes: stashes(&repo),
        conflicted: n(&sets[0]),
        staged: n(&sets[1]),
        unstaged: n(&sets[2]),
        untracked: n(&sets[3]),
        changed: n(&all),
        more,
        operation: operation(&repo),
    })
}

/// **What the checkout is in the middle of**, as git's own prompt reads
/// it off the git directory (`gix::Repository::state`). A rebase is one
/// word however it was started; `git am` has none.
fn operation(repo: &gix::Repository) -> Option<khor_core::GitOperation> {
    use gix::state::InProgress as P;
    use khor_core::GitOperation as O;
    Some(match repo.state()? {
        P::Merge => O::Merge,
        P::Rebase | P::RebaseInteractive | P::ApplyMailboxRebase => O::Rebase,
        P::CherryPick | P::CherryPickSequence => O::CherryPick,
        P::Revert | P::RevertSequence => O::Revert,
        P::Bisect => O::Bisect,
        P::ApplyMailbox => return None,
    })
}

/// **Commits on HEAD's branch that no remote has**, for a branch with no
/// upstream to count against (UX-51, the owner 2026-10-08: a branch never
/// pushed is not silent). HEAD walked with every remote-tracking branch
/// hidden — `git rev-list --count HEAD --not --remotes`.
///
/// `None` detached, and in a checkout with no remote-tracking branch at
/// all: there every commit is 「not on a remote」, and a count of the
/// whole history says nothing a person can act on.
fn unpushed(repo: &gix::Repository) -> Option<u32> {
    let head = repo.head_ref().ok()??;
    let local = head.id().detach();
    let remotes: Vec<gix::ObjectId> = repo
        .references()
        .ok()?
        .remote_branches()
        .ok()?
        .filter_map(|r| Some(r.ok()?.into_fully_peeled_id().ok()?.detach()))
        .collect();
    if remotes.is_empty() {
        return None;
    }
    Some(
        repo.rev_walk([local])
            .with_hidden(remotes)
            .all()
            .ok()?
            .count()
            .min(u32::MAX as usize) as u32,
    )
}

/// **The latest moment git's own files say the segment could have
/// changed**, epoch ms, 0 when none can be read: HEAD moved (a checkout,
/// a commit, a reset), the index was written (a stage), the stash or a
/// remote-tracking branch moved (a push, a fetch). What a resident that
/// has not watched this checkout before dates its segment by — the
/// segment has held at least since then.
pub fn last_moved(root: &Path) -> i64 {
    let Ok(repo) = gix::open(root) else { return 0 };
    let (git, common) = (
        repo.git_dir().to_path_buf(),
        repo.common_dir().to_path_buf(),
    );
    let mut paths = vec![
        git.join("HEAD"),
        git.join("index"),
        git.join("logs/HEAD"),
        git.join("FETCH_HEAD"),
        common.join("logs/refs/stash"),
    ];
    // Remote-tracking branches' logs, `logs/refs/remotes/<remote>/<branch>`.
    let mut dirs = vec![common.join("logs/refs/remotes")];
    while let Some(dir) = dirs.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            match e.file_type() {
                Ok(t) if t.is_dir() && paths.len() < 256 => dirs.push(e.path()),
                Ok(_) => paths.push(e.path()),
                Err(_) => {}
            }
        }
    }
    paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
        .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .max()
        .unwrap_or(0)
}

/// Entries in the stash: the lines of its reflog, which is where git
/// keeps them (`stash@{n}` is the n-th line from the end).
fn stashes(repo: &gix::Repository) -> u32 {
    std::fs::read(repo.common_dir().join("logs/refs/stash"))
        .map(|b| {
            b.split(|&c| c == b'\n')
                .filter(|l| !l.is_empty())
                .count()
                .min(u32::MAX as usize) as u32
        })
        .unwrap_or(0)
}

/// How far HEAD is from its remote-tracking branch, `(ahead, behind)`.
///
/// **`None` for every shape that is not a question**: a detached HEAD,
/// a branch with no upstream configured, an upstream ref that is not
/// there yet. The caller reads all of them as `(0, 0)` and the face
/// draws nothing, which is the same thing it draws when a branch is in
/// step — and that is right, because 「no answer」 and 「zero」 lead to
/// the same next move here, unlike a mark on a file.
///
/// The two walks are commit-graph walks with the other side hidden,
/// which is `git rev-list --count --left-right A...B` by another name.
fn divergence(repo: &gix::Repository) -> Option<(u32, u32)> {
    let head = repo.head_ref().ok()??;
    let local = head.id().detach();
    let upstream = repo
        .branch_remote_tracking_ref_name(head.name(), gix::remote::Direction::Fetch)?
        .ok()?;
    let remote = repo
        .find_reference(upstream.as_ref())
        .ok()?
        .into_fully_peeled_id()
        .ok()?
        .detach();
    if local == remote {
        return Some((0, 0));
    }
    let count = |from, hide| -> Option<u32> {
        Some(
            repo.rev_walk([from])
                .with_hidden([hide])
                .all()
                .ok()?
                .count()
                .min(u32::MAX as usize) as u32,
        )
    };
    Some((count(local, remote)?, count(remote, local)?))
}

/// When `.git/FETCH_HEAD` was last written, in epoch milliseconds, or
/// 0 when it never was.
///
/// **git touches it on every fetch and for nothing else**, so it dates
/// the ahead/behind pair exactly. A repository that has never fetched
/// — a fresh clone answers with the file, a `git init` does not — has
/// no date, and no date is what makes the pair unsayable rather than
/// merely old.
fn last_fetch(repo: &gix::Repository) -> i64 {
    let at = repo.path().join("FETCH_HEAD");
    let Ok(meta) = std::fs::metadata(at) else {
        return 0;
    };
    let Ok(at) = meta.modified() else { return 0 };
    at.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Which of two marks for one path survives.
///
/// **The worktree's answer beats the index's**, because the file on
/// disk is the copy the screen is showing. A file staged as new and
/// then deleted reads `D`, not `A`: `A` would send somebody looking for
/// a file that is not there.
///
/// **[`CONFLICT`] is first and that is not the same kind of rule.** The
/// others rank by 「which copy is the one on screen」; this one ranks by
/// what is being said. Every other letter says 「differs from HEAD」,
/// and `C` says 「nothing here can be committed until somebody
/// chooses」 — a file that is both conflicted and edited is still a file
/// nobody can commit, so the letter that survives has to be that one.
pub(crate) fn fold(a: &'static str, b: &'static str) -> &'static str {
    // **[`IGNORED`] is not in the ranking, it is under it.** The list
    // below sorts changes by「which copy is the one on screen」, and an
    // ignored path is not a change at all — so it survives only when
    // there is nothing else to say. Written before the loop rather than
    // as a sixth entry because the loop's fall-through is `MODIFIED`:
    // two ignored halves folded through it would come out as an edit
    // nobody made, which is what the first spelling did.
    if a == IGNORED || b == IGNORED {
        return if a == b {
            IGNORED
        } else if a == IGNORED {
            b
        } else {
            a
        };
    }
    for want in [CONFLICT, DELETED, UNTRACKED, ADDED, RENAMED] {
        if a == want || b == want {
            return want;
        }
    }
    MODIFIED
}

/// One status item's letter, or `None` for the items that are not a
/// change at all — an entry whose stat needs refreshing, a path the
/// walk pruned for its own reasons. An ignored path *is* said, in
/// [`IGNORED`]: the tree greys those names, which it can only do if it
/// is told which ones they are.
fn mark_of(item: &gix::status::Item) -> Option<&'static str> {
    use gix::status::Item;
    use gix::status::index_worktree::Item as Work;
    use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
    match item {
        Item::TreeIndex(change) => Some(match change {
            gix::diff::index::Change::Addition { .. } => ADDED,
            gix::diff::index::Change::Deletion { .. } => DELETED,
            gix::diff::index::Change::Rewrite { .. } => RENAMED,
            _ => MODIFIED,
        }),
        Item::IndexWorktree(Work::Modification { status, .. }) => match status {
            EntryStatus::Change(Change::Removed) => Some(DELETED),
            EntryStatus::Change(_) => Some(MODIFIED),
            EntryStatus::Conflict { .. } => Some(CONFLICT),
            // `IntentToAdd` is a promise, not a change; `NeedsUpdate`
            // is the walk telling itself to refresh a stat. Neither is
            // a difference a person made.
            EntryStatus::IntentToAdd | EntryStatus::NeedsUpdate(_) => None,
        },
        Item::IndexWorktree(Work::DirectoryContents { entry, .. }) => {
            use gix::dir::entry::Status as Dir;
            match entry.status {
                Dir::Untracked => Some(UNTRACKED),
                Dir::Ignored(_) => Some(IGNORED),
                _ => None,
            }
        }
        Item::IndexWorktree(Work::Rewrite { .. }) => Some(RENAMED),
    }
}

/// Where one file differs from HEAD.
///
/// **A file git has never seen is one insertion, not an error.** That
/// falls out of diffing against an empty side, and it is the right
/// answer: the gutter for a new file is a bar down the whole of it,
/// which is what every editor draws.
pub fn diff(path: &Path) -> Result<Diff, String> {
    let root = repo_root(path).ok_or_else(|| khor_catalog::msg::no_checkout().to_owned())?;
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
    let Ok(tree) = repo.head_tree() else {
        return Ok(Vec::new());
    };
    let found = tree.lookup_entry_by_path(rel).map_err(|e| e.to_string())?;
    let Some(entry) = found else {
        return Ok(Vec::new());
    };
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
        out.push(Hunk {
            at: h.after.start + 1,
            lines: h.after.end - h.after.start,
            was,
        });
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

// ── One file's history (round UX-4, the file page's git 历史) ──────────

/// How many commits one log answer carries at most, and how many a walk
/// looks at to find them. A file touched once in a long history would
/// otherwise cost the whole history to answer; past [`WALK`] the list
/// says `more` rather than walking on.
pub(crate) const LOG: u32 = 200;
pub(crate) const WALK: usize = 20_000;

/// How many bytes of a revision one [`GitAnswer::Show`] carries — the
/// file view's own read cap (`proto::SLICE`), so a revision opens where
/// the current file would.
pub(crate) const SHOWN: usize = 256 * 1024;

/// A repository or file-history question carried by `proto::Request::Git`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitAsk {
    /// The commits that changed `path`, newest first, following the first
    /// parent (what `git log --first-parent -- path` lists).
    Log { path: String, limit: u32 },
    /// All commits reachable from a fixed HEAD, with an opaque continuation.
    RepositoryLog {
        dir: String,
        cursor: Option<String>,
        limit: u32,
    },
    /// Who last changed each line of `path` as HEAD has it.
    Blame { path: String },
    /// `path` as revision `rev` had it, and where the file as it is now
    /// differs from that.
    Show { path: String, rev: String },
    /// **One commit of the repository `root` is in, and what it changed**
    /// against its first parent (the commit page, COMMITS-BRIEF
    /// 2026-10-12: 「点开后显示这个 commit 的更改」). A root commit has no
    /// parent and every file it holds arrives added.
    Commit { root: String, rev: String },
}

/// One file a commit changed, as the commit page draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitFile {
    /// Relative to the repository root, `/`-separated.
    pub path: String,
    /// Where a rename came from; empty for every other change.
    pub from: String,
    /// git's own letter: [`ADDED`], [`MODIFIED`], [`DELETED`], [`RENAMED`].
    pub status: String,
    pub added: u32,
    pub removed: u32,
    /// Either side is not text (a NUL in its head, git's own test): no
    /// lines are said, and the counts are 0.
    pub binary: bool,
    /// Either side is over [`SHOWN`] bytes (cm-d, 2026-10-12: the open
    /// question of commits §「需要负责人定的」1, decided at the file
    /// view's own read cap). The counts are said; neither text is.
    pub big: bool,
    /// **The commit's answer had no room left for this file's text**
    /// (cm-f, 2026-10-12, the owner: 「一次提交传的总量超过 2 MB，剩下的
    /// 文件只列路径和 +a −b，不画 diff」): the files before it spent
    /// [`COMMIT_BYTES`] between their two sides. The counts are said;
    /// neither text is — `big`'s shape, for a reason that is the commit's
    /// and not the file's. Never with `binary` or `big`.
    pub capped: bool,
    /// **Both sides whole** — the parent's and the commit's, decoded
    /// lossily — and the page diffs them itself (`@codemirror/merge`'s
    /// unified view, syntax ink and folded stretches; cm-d item 1).
    /// Empty for a binary or a big file, and for the side an addition or
    /// a deletion does not have. The hunks `git show` would draw are not
    /// sent beside them: every changed line would cross twice.
    pub before: String,
    pub after: String,
}

/// One commit, as a history row reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    /// The full hex id; the face shortens it.
    pub id: String,
    pub author: String,
    /// Committer time, epoch milliseconds.
    pub when_ms: i64,
    /// The message's first paragraph, on one line.
    pub subject: String,
    /// The message after the first blank line, or empty. The list row
    /// shows its first non-empty line; the commit page shows all of it.
    pub body: String,
}

/// Bounded metadata for a repository-history row; full messages belong to detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitSummary {
    pub id: String,
    pub author: String,
    pub when_ms: i64,
    pub subject: String,
    /// Author or subject exceeded its UTF-8 byte budget.
    pub cut: bool,
}

const HISTORY_DEFAULT: u32 = 40;
const HISTORY_LIMIT: u32 = 100;
const HISTORY_SUBJECT_BYTES: usize = 1024;
const HISTORY_AUTHOR_BYTES: usize = 256;
const HISTORY_PAGE_BYTES: usize = 256 * 1024;

/// A run of lines one commit wrote, in HEAD's lines: `start` 0-based,
/// `len` lines, `commit` an index into the answer's commits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlameRun {
    pub start: u32,
    pub len: u32,
    pub commit: u32,
}

/// The answer to a [`GitAsk`], variant for variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitAnswer {
    RepositoryLog {
        head: Option<String>,
        commits: Vec<CommitSummary>,
        next: Option<String>,
    },
    Log {
        commits: Vec<Commit>,
        more: bool,
    },
    Blame {
        commits: Vec<Commit>,
        runs: Vec<BlameRun>,
    },
    /// `text` is the revision's file, decoded lossily and cut at
    /// [`SHOWN`] (`cut` says so); `hunks` are where the current file
    /// differs from it, in the current file's lines — the gutter's own
    /// shape, so 「与当前比较」 is the diff view the file already has.
    Show {
        text: String,
        cut: bool,
        hunks: Vec<Hunk>,
        more: bool,
    },
    /// One commit and the files it changed, in path order. `total` is how
    /// many files it changed in all; `more` says the list stopped at
    /// [`COMMIT_FILES`] files or once [`COMMIT_LINES`] changed lines were
    /// spent — said, never cut silently.
    Commit {
        commit: Commit,
        files: Vec<CommitFile>,
        total: u32,
        more: bool,
    },
}

/// Answers one history question on this machine.
pub fn answer(ask: &GitAsk) -> Result<GitAnswer, String> {
    match ask {
        GitAsk::Log { path, limit } => log(Path::new(path), (*limit).clamp(1, LOG)),
        GitAsk::RepositoryLog { dir, cursor, limit } => {
            repository_log(Path::new(dir), cursor.as_deref(), *limit)
        }
        GitAsk::Blame { path } => blame(Path::new(path)),
        GitAsk::Show { path, rev } => show(Path::new(path), rev),
        GitAsk::Commit { root, rev } => commit_show(Path::new(root), rev),
    }
}

/// The repository `path` is in, and `path` relative to its root.
fn open_rel(path: &Path) -> Result<(gix::Repository, String), String> {
    let root = repo_root(path).ok_or_else(|| khor_catalog::msg::no_checkout().to_owned())?;
    let repo = gix::open(&root).map_err(|e| e.to_string())?;
    let rel = path.strip_prefix(&root).map_err(|e| e.to_string())?;
    Ok((repo, rel.to_string_lossy().replace('\\', "/")))
}

fn commit_of(c: &gix::Commit<'_>) -> Commit {
    let author = c
        .author()
        .map(|a| a.name.to_str_lossy().into_owned())
        .unwrap_or_default();
    let when_ms = c.time().map(|t| t.seconds * 1000).unwrap_or(0);
    let msg = c.message().ok();
    let subject = msg
        .as_ref()
        .map(|m| m.summary().to_str_lossy().into_owned())
        .unwrap_or_default();
    // Everything after the first blank line, trailers included: the
    // face reads `Co-Authored-By:` off it for the author line.
    let body = msg
        .as_ref()
        .and_then(|m| m.body())
        .map(|b| b.to_str_lossy().trim().to_owned())
        .unwrap_or_default();
    Commit {
        id: c.id.to_string(),
        author,
        when_ms,
        subject,
        body,
    }
}

/// The blob id `rel` has in a commit's tree, `None` where it has none.
fn blob_in(repo: &gix::Repository, commit: gix::ObjectId, rel: &str) -> Option<gix::ObjectId> {
    let tree = repo.find_commit(commit).ok()?.tree().ok()?;
    tree.lookup_entry_by_path(rel).ok()?.map(|e| e.object_id())
}

fn log(path: &Path, limit: u32) -> Result<GitAnswer, String> {
    let (repo, rel) = open_rel(path)?;
    let Ok(head) = repo.head_id() else {
        // No commit yet: no history, which is not an error.
        return Ok(GitAnswer::Log {
            commits: Vec::new(),
            more: false,
        });
    };
    let walk = repo
        .rev_walk([head.detach()])
        .first_parent_only()
        .all()
        .map_err(|e| e.to_string())?;
    let mut commits = Vec::new();
    let mut more = false;
    for (seen, info) in walk.enumerate() {
        let info = info.map_err(|e| e.to_string())?;
        if seen >= WALK || commits.len() >= limit as usize {
            more = true;
            break;
        }
        // The repository itself (`rel` empty, the workspace's commit
        // line above the tree — ws-lab 2026-10-12): every commit on the
        // first-parent walk is its history, nothing to compare. A path:
        // only the commits where its object changed. `blob_in` answers
        // for a directory too (a tree's id changes when anything under
        // it does), so a subtree's log is the same question.
        let changed = rel.is_empty() || {
            let here = blob_in(&repo, info.id, &rel);
            let before = info
                .parent_ids
                .first()
                .and_then(|p| blob_in(&repo, *p, &rel));
            here.is_some() && here != before
        };
        if changed {
            let c = repo.find_commit(info.id).map_err(|e| e.to_string())?;
            commits.push(commit_of(&c));
        }
    }
    Ok(GitAnswer::Log { commits, more })
}

fn clip_history_field(mut value: String, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value, false);
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    (value, true)
}

fn summary_of(commit: &gix::Commit<'_>) -> Result<CommitSummary, String> {
    let author = commit.author().map_err(|e| e.to_string())?;
    let (author, author_cut) = clip_history_field(
        author.name.to_str_lossy().into_owned(),
        HISTORY_AUTHOR_BYTES,
    );
    let message = commit.message().map_err(|e| e.to_string())?;
    let (subject, subject_cut) = clip_history_field(
        message.summary().to_str_lossy().into_owned(),
        HISTORY_SUBJECT_BYTES,
    );
    Ok(CommitSummary {
        id: commit.id.to_string(),
        author,
        when_ms: commit
            .time()
            .map_err(|e| e.to_string())?
            .seconds
            .saturating_mul(1000),
        subject,
        cut: author_cut || subject_cut,
    })
}

struct HistoryCursor {
    head: gix::ObjectId,
    after: gix::ObjectId,
}

impl HistoryCursor {
    fn parse(value: &str, hash: gix::hash::Kind) -> Result<Self, String> {
        let invalid = || "Invalid repository history cursor".to_owned();
        let (head, after) = value.split_once(':').ok_or_else(invalid)?;
        let parse = |text: &str| {
            let id = gix::ObjectId::from_hex(text.as_bytes()).map_err(|_| invalid())?;
            if id.kind() != hash {
                return Err(invalid());
            }
            Ok(id)
        };
        Ok(Self {
            head: parse(head)?,
            after: parse(after)?,
        })
    }

    fn encode(head: gix::ObjectId, after: gix::ObjectId) -> String {
        format!("{head}:{after}")
    }
}

fn repository_log(path: &Path, cursor: Option<&str>, limit: u32) -> Result<GitAnswer, String> {
    let (repo, _) = open_rel(path)?;
    if repo.shallow_commits().map_err(|e| e.to_string())?.is_some() {
        return Err("Repository history is incomplete in a shallow checkout".to_owned());
    }
    let cursor = cursor
        .map(|c| HistoryCursor::parse(c, repo.object_hash()))
        .transpose()?;
    let head = match &cursor {
        Some(cursor) => cursor.head,
        None => {
            let head = repo.head().map_err(|e| e.to_string())?;
            let Some(id) = head.id() else {
                return Ok(GitAnswer::RepositoryLog {
                    head: None,
                    commits: Vec::new(),
                    next: None,
                });
            };
            id.detach()
        }
    };
    // Read objects even when a commit-graph exists: missing objects must fail,
    // and a graph written between requests must not change continuation order.
    repo.find_commit(head).map_err(|e| e.to_string())?;
    if let Some(cursor) = &cursor {
        repo.find_commit(cursor.after).map_err(|e| e.to_string())?;
    }
    let walk = repo
        .rev_walk([head])
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(false)
        .all()
        .map_err(|e| e.to_string())?;
    let limit = if limit == 0 {
        HISTORY_DEFAULT
    } else {
        limit.min(HISTORY_LIMIT)
    } as usize;
    let envelope = GitAnswer::RepositoryLog {
        head: Some(head.to_string()),
        commits: Vec::new(),
        next: Some(HistoryCursor::encode(head, head)),
    };
    let mut bytes = serde_json::to_vec(&envelope)
        .map_err(|e| e.to_string())?
        .len();
    let mut commits = Vec::new();
    let mut found_after = cursor.is_none();
    let mut more = false;
    for info in walk {
        let info = info.map_err(|e| e.to_string())?;
        if !found_after {
            found_after = cursor.as_ref().is_some_and(|c| c.after == info.id);
            continue;
        }
        if commits.len() == limit {
            more = true;
            break;
        }
        let summary = summary_of(&repo.find_commit(info.id).map_err(|e| e.to_string())?)?;
        let row_bytes = serde_json::to_vec(&summary)
            .map_err(|e| e.to_string())?
            .len()
            + usize::from(!commits.is_empty());
        if bytes + row_bytes > HISTORY_PAGE_BYTES {
            more = true;
            break;
        }
        bytes += row_bytes;
        commits.push(summary);
    }
    if !found_after {
        return Err("Repository history cursor is not reachable from its snapshot HEAD".to_owned());
    }
    let next = if more {
        let last = commits
            .last()
            .ok_or_else(|| "Repository history row exceeds page budget".to_owned())?;
        Some(format!("{head}:{}", last.id))
    } else {
        None
    };
    Ok(GitAnswer::RepositoryLog {
        head: Some(head.to_string()),
        commits,
        next,
    })
}

fn blame(path: &Path) -> Result<GitAnswer, String> {
    let (repo, rel) = open_rel(path)?;
    let head = repo.head_id().map_err(|e| e.to_string())?.detach();
    let out = repo
        .blame_file(rel.as_bytes().as_bstr(), head, Default::default())
        .map_err(|e| e.to_string())?;
    let mut index: BTreeMap<gix::ObjectId, u32> = BTreeMap::new();
    let mut commits = Vec::new();
    let mut runs = Vec::new();
    for e in &out.entries {
        let at = match index.get(&e.commit_id) {
            Some(i) => *i,
            None => {
                let c = repo.find_commit(e.commit_id).map_err(|e| e.to_string())?;
                commits.push(commit_of(&c));
                let i = (commits.len() - 1) as u32;
                index.insert(e.commit_id, i);
                i
            }
        };
        runs.push(BlameRun {
            start: e.start_in_blamed_file,
            len: e.len.get(),
            commit: at,
        });
    }
    Ok(GitAnswer::Blame { commits, runs })
}

fn show(path: &Path, rev: &str) -> Result<GitAnswer, String> {
    let (repo, rel) = open_rel(path)?;
    let id = gix::ObjectId::from_hex(rev.as_bytes()).map_err(|e| e.to_string())?;
    let old = match blob_in(&repo, id, &rel) {
        Some(blob) => repo
            .find_object(blob)
            .map_err(|e| e.to_string())?
            .data
            .clone(),
        None => Vec::new(),
    };
    let new = std::fs::read(path).map_err(|e| e.to_string())?;
    let Diff { hunks, more } = hunks(&old, &new);
    let cut = old.len() > SHOWN;
    let text = String::from_utf8_lossy(&old[..old.len().min(SHOWN)]).into_owned();
    Ok(GitAnswer::Show {
        text,
        cut,
        hunks,
        more,
    })
}

/// How many files one [`GitAnswer::Commit`] lists, and how many changed
/// lines it spends before it stops listing. A vendored-dependency bump
/// touches thousands; past either the answer says `more` and the page
/// says so. A listed file is whole or not there: the budget decides
/// whether the next file is listed, never where a file's text stops
/// (each side is capped by [`SHOWN`] instead, `big`).
pub(crate) const COMMIT_FILES: usize = 50;
pub(crate) const COMMIT_LINES: usize = 2000;

/// How many bytes of text one [`GitAnswer::Commit`] carries in all, both
/// sides of every file (cm-f, 2026-10-12: cm-d's merge view made the
/// answer 17× larger, and it crosses a relay). Spent in the order the
/// files are listed, so the first keep their diffs; the file that would
/// go past it and every one after it are `capped`.
pub(crate) const COMMIT_BYTES: usize = 2 * 1024 * 1024;

/// One changed file before it is read: (path, from, letter, the parent's
/// bytes, the commit's).
type Row = (String, String, &'static str, Vec<u8>, Vec<u8>);

fn commit_show(root: &Path, rev: &str) -> Result<GitAnswer, String> {
    use gix::object::tree::diff::ChangeDetached as C;
    let root = repo_root(root).ok_or_else(|| khor_catalog::msg::no_checkout().to_owned())?;
    let repo = gix::open(&root).map_err(|e| e.to_string())?;
    let id = gix::ObjectId::from_hex(rev.as_bytes()).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(id).map_err(|e| e.to_string())?;
    let new = commit.tree().map_err(|e| e.to_string())?;
    let old = match commit.parent_ids().next() {
        Some(p) => Some(
            repo.find_commit(p.detach())
                .map_err(|e| e.to_string())?
                .tree()
                .map_err(|e| e.to_string())?,
        ),
        None => None,
    };
    // Renames tracked with git's own defaults (50 % similar), not the
    // repository's `diff.renames`: the page reads the same everywhere.
    let opts = gix::diff::Options::default().with_rewrites(Some(Default::default()));
    let changes = repo
        .diff_tree_to_tree(old.as_ref(), Some(&new), opts)
        .map_err(|e| e.to_string())?;
    // (path, from, letter, old blob, new blob), files only: a directory
    // is said by the files under it, a submodule has no lines here.
    let mut rows: Vec<(
        String,
        String,
        &'static str,
        Option<gix::ObjectId>,
        Option<gix::ObjectId>,
    )> = Vec::new();
    for c in changes {
        if !c.entry_mode().is_blob_or_symlink() {
            continue;
        }
        let path = c.location().to_str_lossy().into_owned();
        rows.push(match c {
            C::Addition { id, .. } => (path, String::new(), ADDED, None, Some(id)),
            C::Deletion { id, .. } => (path, String::new(), DELETED, Some(id), None),
            C::Modification {
                previous_id, id, ..
            } => (path, String::new(), MODIFIED, Some(previous_id), Some(id)),
            C::Rewrite { copy: true, id, .. } => (path, String::new(), ADDED, None, Some(id)),
            C::Rewrite {
                source_location,
                source_id,
                id,
                ..
            } => (
                path,
                source_location.to_str_lossy().into_owned(),
                RENAMED,
                Some(source_id),
                Some(id),
            ),
        });
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let total = rows.len().min(u32::MAX as usize) as u32;
    let read = |id: Option<gix::ObjectId>| -> Result<Vec<u8>, String> {
        match id {
            Some(id) => Ok(repo
                .find_object(id)
                .map_err(|e| e.to_string())?
                .data
                .clone()),
            None => Ok(Vec::new()),
        }
    };
    let (files, more) =
        files_of(rows.into_iter().map(|(path, from, status, was, now)| {
            Ok((path, from, status, read(was)?, read(now)?))
        }))?;
    Ok(GitAnswer::Commit {
        commit: commit_of(&commit),
        files,
        total,
        more,
    })
}

/// The listed files of one commit, in `rows`' order, and whether the
/// list stopped early (`more`). A row is read only once it is known to
/// be listed.
fn files_of(
    rows: impl Iterator<Item = Result<Row, String>>,
) -> Result<(Vec<CommitFile>, bool), String> {
    let mut files = Vec::new();
    let mut budget = COMMIT_LINES;
    let mut room = COMMIT_BYTES;
    for row in rows {
        if files.len() >= COMMIT_FILES || budget == 0 {
            return Ok((files, true));
        }
        let (path, from, status, was, now) = row?;
        files.push(file_of(
            path,
            from,
            status,
            was,
            now,
            &mut budget,
            &mut room,
        ));
    }
    Ok((files, false))
}

/// One changed file's answer from its two sides' bytes (`a` the
/// parent's, `b` the commit's; empty for the side it does not have),
/// spending its changed lines from `budget` and its bytes from `room`.
/// A big or capped file spends nothing: none of its lines are sent. The
/// first file that does not fit leaves no room for any after it.
fn file_of(
    path: String,
    from: String,
    status: &str,
    a: Vec<u8>,
    b: Vec<u8>,
    budget: &mut usize,
    room: &mut usize,
) -> CommitFile {
    let binary = [&a, &b].iter().any(|x| x[..x.len().min(8000)].contains(&0));
    let big = !binary && (a.len() > SHOWN || b.len() > SHOWN);
    let (added, removed) = if binary { (0, 0) } else { counts(&a, &b) };
    let capped = !binary && !big && a.len() + b.len() > *room;
    let said = !binary && !big && !capped;
    if said {
        *budget = budget.saturating_sub((added + removed) as usize);
        *room -= a.len() + b.len();
    }
    if capped {
        *room = 0;
    }
    let text = |x: Vec<u8>| {
        if !said {
            return String::new();
        }
        String::from_utf8(x).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
    };
    CommitFile {
        path,
        from,
        status: status.to_owned(),
        added,
        removed,
        binary,
        big,
        capped,
        before: text(a),
        after: text(b),
    }
}

/// How many lines `new` adds to `old` and removes from it, git's
/// histogram diff — the numbers `git show --stat` says.
fn counts(old: &[u8], new: &[u8]) -> (u32, u32) {
    use gix::diff::blob::{Algorithm, InternedInput, sources};
    let input = InternedInput::new(sources::byte_lines(old), sources::byte_lines(new));
    let mut d = gix::diff::blob::Diff::compute(Algorithm::Histogram, &input);
    d.postprocess_lines(&input);
    d.hunks().fold((0, 0), |(a, r), h| {
        (
            a + h.after.end - h.after.start,
            r + h.before.end - h.before.start,
        )
    })
}

#[cfg(test)]
mod history_tests {
    use super::*;

    /// This crate's own manifest, in the checkout the test is compiled
    /// inside (the reason `status_tests` gives).
    fn manifest() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
    }

    #[test]
    fn a_tracked_file_has_a_history_newest_first() {
        let GitAnswer::Log { commits, .. } = answer(&GitAsk::Log {
            path: manifest().to_string_lossy().into_owned(),
            limit: 5,
        })
        .unwrap() else {
            panic!("not a log")
        };
        assert!(
            !commits.is_empty(),
            "the manifest was committed at least once"
        );
        assert!(commits.len() <= 5);
        assert!(
            commits
                .windows(2)
                .all(|w| w[0].when_ms >= w[1].when_ms || w[0].when_ms == 0)
        );
        assert_eq!(commits[0].id.len(), 40);
    }

    #[test]
    fn blame_covers_every_line_of_head() {
        let GitAnswer::Blame { commits, runs } = answer(&GitAsk::Blame {
            path: manifest().to_string_lossy().into_owned(),
        })
        .unwrap() else {
            panic!("not a blame")
        };
        let (repo, rel) = open_rel(&manifest()).unwrap();
        let head = head_blob(&repo, &rel).unwrap();
        let lines = head.lines().count() as u32;
        assert_eq!(runs.iter().map(|r| r.len).sum::<u32>(), lines);
        assert!(runs.iter().all(|r| (r.commit as usize) < commits.len()));
    }

    #[test]
    fn a_revision_shows_what_it_had() {
        let (repo, rel) = open_rel(&manifest()).unwrap();
        let head = repo.head_id().unwrap().to_string();
        let GitAnswer::Show { text, cut, .. } = answer(&GitAsk::Show {
            path: manifest().to_string_lossy().into_owned(),
            rev: head,
        })
        .unwrap() else {
            panic!("not a show")
        };
        assert!(!cut);
        assert_eq!(text.as_bytes(), head_blob(&repo, &rel).unwrap().as_slice());
    }
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
        assert_eq!(
            shape("a\nb\nc\n", "a\nB\nc\n"),
            vec![(2, 1, vec!["b".to_owned()])]
        );
    }

    #[test]
    fn a_deletion_names_the_line_it_follows() {
        // There is no current line to point at, so `at` is the line
        // above and `lines` is 0 — which is also how the consumer tells
        // a deletion from the other two.
        assert_eq!(
            shape("a\nb\nc\n", "a\nc\n"),
            vec![(1, 0, vec!["b".to_owned()])]
        );
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

    /// **The repository's own log, for the commit line above the tree**
    /// (ws-lab, 2026-10-12). Asked with the checkout's root — `rel`
    /// empty — the walk used to compare the root's blob in each commit
    /// against its parent's, and a root has no blob, so every workspace
    /// answered with no history at all. The first entry is HEAD.
    #[test]
    fn the_checkout_itself_has_a_log_whose_first_entry_is_head() {
        let root = here();
        let GitAnswer::Log { commits, .. } = log(&root, 5).expect("log") else {
            panic!("not a log")
        };
        assert!(!commits.is_empty(), "a checkout has at least one commit");
        let repo = gix::open(&root).expect("open");
        let head = repo.head_id().expect("head").to_string();
        assert_eq!(commits[0].id, head, "the first entry is HEAD");
    }

    /// **The commit page's question** (COMMITS-BRIEF, 2026-10-12): HEAD
    /// of this checkout changed at least one file, and its text files
    /// arrive with both sides whole (cm-d item 1).
    #[test]
    fn head_answers_its_files_with_both_sides() {
        let root = here();
        let head = gix::open(&root)
            .expect("open")
            .head_id()
            .expect("head")
            .to_string();
        let GitAnswer::Commit {
            commit,
            files,
            total,
            ..
        } = answer(&GitAsk::Commit {
            root: root.to_string_lossy().into_owned(),
            rev: head.clone(),
        })
        .expect("commit")
        else {
            panic!("not a commit")
        };
        assert_eq!(commit.id, head);
        assert!(!files.is_empty() && total as usize >= files.len());
        let f = files
            .iter()
            .find(|f| !f.binary && !f.big)
            .expect("a text file");
        assert!(f.added + f.removed > 0);
        assert_ne!(
            f.before, f.after,
            "{} changed and both sides read the same",
            f.path
        );
    }

    /// A commit with no parent is diffed against nothing: every file it
    /// holds is added, and none was removed.
    #[test]
    fn the_root_commit_adds_every_file() {
        let root = here();
        let repo = gix::open(&root).expect("open");
        let first = repo
            .rev_walk([repo.head_id().expect("head").detach()])
            .first_parent_only()
            .all()
            .expect("walk")
            .filter_map(Result::ok)
            .last()
            .expect("a root");
        assert!(first.parent_ids.is_empty());
        let GitAnswer::Commit {
            files, total, more, ..
        } = answer(&GitAsk::Commit {
            root: root.to_string_lossy().into_owned(),
            rev: first.id.to_string(),
        })
        .expect("commit")
        else {
            panic!("not a commit")
        };
        assert!(!files.is_empty());
        assert!(
            files.iter().all(|f| f.status == ADDED && f.removed == 0),
            "{files:?}"
        );
        if files.len() < total as usize {
            assert!(
                more,
                "{} of {total} files listed and the answer did not say so",
                files.len()
            );
        }
    }

    /// **A text file carries both sides, a binary neither** (cm-d item
    /// 1), and the counts are git's.
    #[test]
    fn a_text_file_carries_both_sides_and_a_binary_neither() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n";
        let new = "1\n2\n3\nfour\n5\n6\n7\neight\n9\n10\n11\n12\n";
        let mut budget = COMMIT_LINES;
        let mut room = COMMIT_BYTES;
        let f = file_of(
            "a.txt".into(),
            String::new(),
            MODIFIED,
            old.into(),
            new.into(),
            &mut budget,
            &mut room,
        );
        assert_eq!((f.added, f.removed, f.binary, f.big), (2, 2, false, false));
        assert_eq!((f.before.as_str(), f.after.as_str()), (old, new));
        assert_eq!(budget, COMMIT_LINES - 4, "four changed lines spent");

        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        let f = file_of(
            "a.png".into(),
            String::new(),
            ADDED,
            Vec::new(),
            png,
            &mut budget,
            &mut room,
        );
        assert!(f.binary && !f.big);
        assert_eq!((f.added, f.removed), (0, 0));
        assert!(
            f.before.is_empty() && f.after.is_empty(),
            "a binary sends no text"
        );
        assert_eq!(budget, COMMIT_LINES - 4, "a binary spends nothing");
    }

    /// **Past [`SHOWN`] on either side a file is `big`**: its counts are
    /// said, neither text is, and its lines spend nothing.
    #[test]
    fn a_side_over_the_byte_cap_says_so_and_sends_no_text() {
        let line = "x".repeat(99) + "\n";
        let old = line.repeat(SHOWN / 100 + 1).into_bytes();
        assert!(old.len() > SHOWN);
        let mut new = old.clone();
        new.extend_from_slice(b"one more\n");
        let mut budget = COMMIT_LINES;
        let mut room = COMMIT_BYTES;
        let f = file_of(
            "big.txt".into(),
            String::new(),
            MODIFIED,
            old,
            new,
            &mut budget,
            &mut room,
        );
        assert!(f.big && !f.binary);
        assert_eq!((f.added, f.removed), (1, 0));
        assert!(f.before.is_empty() && f.after.is_empty());
        assert_eq!(budget, COMMIT_LINES);
        // One byte under on both sides is text.
        let small = "y\n".repeat(SHOWN / 2).into_bytes();
        assert_eq!(small.len(), SHOWN);
        let f = file_of(
            "edge.txt".into(),
            String::new(),
            ADDED,
            Vec::new(),
            small,
            &mut budget,
            &mut room,
        );
        assert!(!f.big && f.after.len() == SHOWN);
    }

    /// The budget stops the list at the next file, never inside one: a
    /// file is whole or not listed.
    #[test]
    fn the_line_budget_spends_whole_files() {
        let new: String = (0..10).map(|i| format!("{i}\n")).collect();
        let mut budget = 4;
        let mut room = COMMIT_BYTES;
        let f = file_of(
            "ten.txt".into(),
            String::new(),
            ADDED,
            Vec::new(),
            new.clone().into_bytes(),
            &mut budget,
            &mut room,
        );
        assert_eq!(f.added, 10);
        assert_eq!(
            f.after, new,
            "the file that spent the budget is still whole"
        );
        assert_eq!(budget, 0, "and the next one is not listed");
    }

    /// **A commit's answer carries at most [`COMMIT_BYTES`] of text**
    /// (cm-f): the files that fit keep both sides; the one that would go
    /// past it and every one after — a small one too — say their counts
    /// and no text, `capped`. A binary after the cap stays a binary.
    #[test]
    fn past_the_commit_byte_cap_the_rest_say_counts_and_no_text() {
        // Four modifications 250 KiB a side (under SHOWN; ≈ 500 KiB each
        // in the answer) make 1.95 MiB; a fifth would pass 2 MiB.
        let side = |c: char, n: usize| -> Vec<u8> {
            (0..n)
                .map(|i| format!("{c}{i:0>98}\n"))
                .collect::<String>()
                .into_bytes()
        };
        let lines = 250 * 1024 / 100;
        let mut rows: Vec<Result<Row, String>> = (0..6)
            .map(|i| {
                let old = side('a', lines);
                let mut new = old.clone();
                new.extend_from_slice(format!("{i} more\n").as_bytes());
                Ok((format!("f{i}.txt"), String::new(), MODIFIED, old, new))
            })
            .collect();
        rows.push(Ok((
            "g.png".into(),
            String::new(),
            ADDED,
            Vec::new(),
            b"\x89PNG\0".to_vec(),
        )));
        rows.push(Ok((
            "h.txt".into(),
            String::new(),
            ADDED,
            Vec::new(),
            b"one\ntwo\n".to_vec(),
        )));
        let (files, more) = files_of(rows.into_iter()).expect("files");
        assert!(
            !more,
            "every file is listed: the byte cap drops texts, not files"
        );
        assert_eq!(files.len(), 8);
        let sent: usize = files.iter().map(|f| f.before.len() + f.after.len()).sum();
        assert!(sent <= COMMIT_BYTES, "{sent} bytes of text sent");
        for f in &files[..4] {
            assert!(
                !f.capped && !f.before.is_empty() && !f.after.is_empty(),
                "{} fits and keeps its text",
                f.path
            );
            assert_eq!((f.added, f.removed), (1, 0), "{}", f.path);
        }
        for f in &files[4..6] {
            assert!(
                f.capped && !f.big && !f.binary,
                "{} is past the cap: {f:?}",
                f.path
            );
            assert!(
                f.before.is_empty() && f.after.is_empty(),
                "{} sends no text",
                f.path
            );
            assert_eq!(
                (f.added, f.removed),
                (1, 0),
                "{}'s counts are still git's",
                f.path
            );
        }
        assert!(
            files[6].binary && !files[6].capped,
            "a binary is a binary, cap or not"
        );
        let h = &files[7];
        assert!(
            h.capped && h.after.is_empty(),
            "a small file after the cap has no room either: {h:?}"
        );
        assert_eq!((h.added, h.removed), (2, 0));
    }

    /// Under the cap nothing is capped: HEAD's own commit of this
    /// checkout is far smaller than 2 MiB.
    #[test]
    fn a_small_commit_caps_nothing() {
        let root = here();
        let head = gix::open(&root)
            .expect("open")
            .head_id()
            .expect("head")
            .to_string();
        let GitAnswer::Commit { files, .. } = answer(&GitAsk::Commit {
            root: root.to_string_lossy().into_owned(),
            rev: head,
        })
        .expect("commit") else {
            panic!("not a commit")
        };
        let sent: usize = files.iter().map(|f| f.before.len() + f.after.len()).sum();
        if sent < COMMIT_BYTES / 2 {
            assert!(files.iter().all(|f| !f.capped), "{files:?}");
        }
    }

    #[test]
    fn a_real_checkout_answers_with_a_branch() {
        let s = status(&here()).expect("status");
        assert!(!s.branch.is_empty(), "HEAD has a name in this repo");
    }

    #[test]
    fn every_marked_row_carries_one_of_the_seven_letters() {
        let s = status(&here()).expect("status");
        for (path, mark) in &s.rows {
            assert!(
                [
                    MODIFIED, ADDED, DELETED, UNTRACKED, CONFLICT, RENAMED, IGNORED
                ]
                .contains(&mark.as_str()),
                "{path} got {mark}"
            );
        }
    }

    /// **`target/` is one row, not forty thousand** (2026-09-16). The
    /// walk emits ignored paths now, and the only reason that is
    /// affordable is `CollapseDirectory`: an ignored directory arrives
    /// whole. Asserted against this checkout, which has a `target/` in
    /// it that a debug build fills — so the shape being tested is the
    /// expensive one.
    #[test]
    fn an_ignored_directory_arrives_as_one_row() {
        let s = status(&here()).expect("status");
        let ignored: Vec<&str> = s
            .rows
            .iter()
            .filter(|(_, m)| m == IGNORED)
            .map(|(p, _)| p.as_str())
            .collect();
        for path in &ignored {
            assert!(
                !path.contains("target/"),
                "{path} came back as a file under an ignored directory — the walk is not collapsing"
            );
        }
        // …and nothing under an ignored directory is marked as a
        // change either, which is what the fold protects.
        for (path, mark) in &s.rows {
            assert!(
                !path.starts_with("target/"),
                "{path} ({mark}) is inside an ignored directory"
            );
        }
    }

    /// Which letter survives when one path arrives twice, for the two
    /// that joined the alphabet in 工作区批⑦.
    ///
    /// **A unit test rather than a fixture**: making a conflicted index
    /// needs gix's worktree-mutation half, which khor does not compile
    /// in ([`here`] says the same about creating a repository at all) —
    /// and the ranking is the part that can be got wrong silently.
    #[test]
    fn a_conflict_outranks_every_other_letter_for_one_path() {
        assert_eq!(fold(CONFLICT, MODIFIED), CONFLICT);
        assert_eq!(fold(DELETED, CONFLICT), CONFLICT);
        assert_eq!(fold(CONFLICT, UNTRACKED), CONFLICT);
        // …and a rename beats a plain edit, because the path on screen
        // is not the path HEAD has and that is the bigger fact about
        // the row.
        assert_eq!(fold(RENAMED, MODIFIED), RENAMED);
        // The old ranking is untouched by the two new letters.
        assert_eq!(fold(ADDED, DELETED), DELETED);
        assert_eq!(fold(MODIFIED, MODIFIED), MODIFIED);
    }

    #[test]
    fn a_file_outside_any_checkout_says_so_rather_than_failing_to_open() {
        let err = diff(Path::new("/etc/hosts")).expect_err("no checkout above /etc");
        assert_eq!(err, khor_catalog::msg::no_checkout());
    }
}

#[cfg(test)]
pub(crate) mod repository_history_tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    pub(crate) struct Checkout(pub(crate) PathBuf);

    impl Checkout {
        pub(crate) fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "khor-repository-history-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            let checkout = Self(std::fs::canonicalize(path).unwrap());
            checkout.git(&["init", "--quiet", "--initial-branch=main"]);
            checkout
        }

        fn command(&self) -> Command {
            let mut cmd = Command::new("git");
            cmd.env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .env("HOME", &self.0)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("LC_ALL", "C")
                .current_dir(&self.0)
                .args(["-c", "core.fsync=none"]);
            cmd
        }

        pub(crate) fn git(&self, args: &[&str]) -> Vec<u8> {
            let output = self.command().args(args).output().unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        }

        fn input(&self, args: &[&str], data: &[u8]) -> Vec<u8> {
            let mut child = self
                .command()
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let written = child.stdin.take().unwrap().write_all(data);
            if let Err(error) = written {
                let _ = child.kill();
                let _ = child.wait();
                panic!("fixture stdin: {error}");
            }
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        }

        fn import(&self, stream: &str) -> BTreeMap<usize, String> {
            let marks = self.0.join("marks");
            self.input(
                &[
                    "fast-import",
                    "--quiet",
                    &format!("--export-marks={}", marks.display()),
                ],
                stream.as_bytes(),
            );
            std::fs::read_to_string(marks)
                .unwrap()
                .lines()
                .map(|line| {
                    let (mark, oid) = line.split_once(' ').unwrap();
                    (
                        mark.trim_start_matches(':').parse().unwrap(),
                        oid.to_owned(),
                    )
                })
                .collect()
        }
    }

    impl Drop for Checkout {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn record(
        stream: &mut String,
        mark: usize,
        branch: &str,
        parents: &[usize],
        time: usize,
        author: &str,
        message: &str,
    ) {
        use std::fmt::Write;
        writeln!(stream, "commit refs/heads/{branch}\nmark :{mark}\nauthor {author} <a@example.test> {time} +0000\ncommitter A <a@example.test> {time} +0000\ndata {}\n{message}", message.len()).unwrap();
        if let Some((first, rest)) = parents.split_first() {
            writeln!(stream, "from :{first}").unwrap();
            for parent in rest {
                writeln!(stream, "merge :{parent}").unwrap();
            }
        }
        stream.push_str("deleteall\n\n");
    }

    pub(crate) fn linear(count: usize) -> (Checkout, BTreeMap<usize, String>) {
        let checkout = Checkout::new();
        let mut stream = String::new();
        for i in 1..=count {
            let parents = if i == 1 { vec![] } else { vec![i - 1] };
            record(
                &mut stream,
                i,
                "main",
                &parents,
                1_700_000_000 + i,
                "A",
                &format!("commit {i}"),
            );
        }
        let marks = checkout.import(&stream);
        (checkout, marks)
    }

    fn page(
        checkout: &Checkout,
        cursor: Option<&str>,
        limit: u32,
    ) -> (Option<String>, Vec<CommitSummary>, Option<String>) {
        let answer = answer(&GitAsk::RepositoryLog {
            dir: checkout.0.display().to_string(),
            cursor: cursor.map(str::to_owned),
            limit,
        })
        .unwrap();
        assert!(serde_json::to_vec(&answer).unwrap().len() <= HISTORY_PAGE_BYTES);
        let GitAnswer::RepositoryLog {
            head,
            commits,
            next,
        } = answer
        else {
            panic!("repository log");
        };
        (head, commits, next)
    }

    fn collect(checkout: &Checkout, limit: u32) -> Vec<CommitSummary> {
        let mut cursor = None;
        let mut rows = Vec::new();
        loop {
            let (_, commits, next) = page(checkout, cursor.as_deref(), limit);
            assert!(!commits.is_empty() || next.is_none());
            rows.extend(commits);
            let Some(next) = next else {
                return rows;
            };
            assert_ne!(cursor.as_ref(), Some(&next));
            cursor = Some(next);
        }
    }

    #[test]
    fn repository_history_reaches_merge_parents() {
        let checkout = Checkout::new();
        let mut stream = String::new();
        for (mark, branch, parents) in [
            (1, "main", vec![]),
            (2, "main", vec![1]),
            (3, "topic", vec![1]),
            (4, "topic", vec![3]),
            (5, "main", vec![2]),
            (6, "main", vec![5, 4]),
        ] {
            record(
                &mut stream,
                mark,
                branch,
                &parents,
                1_700_000_000,
                "A",
                &format!("commit {mark}"),
            );
        }
        let marks = checkout.import(&stream);
        let GitAnswer::Log {
            commits: path_log,
            more,
        } = log(&checkout.0, LOG).unwrap()
        else {
            panic!("path log");
        };
        assert_eq!(path_log.len(), 4);
        assert!(!more);

        let all = collect(&checkout, 2);
        let actual: std::collections::BTreeSet<_> = all.iter().map(|c| c.id.clone()).collect();
        assert_eq!(actual.len(), all.len());
        assert_eq!(actual, marks.into_values().collect());
        let first = page(&checkout, None, 2);
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(page(&checkout, None, 2)).unwrap()
        );
        let next = first.2.unwrap();
        assert_eq!(
            page(&checkout, Some(&next), 2),
            page(&checkout, Some(&next), 2)
        );
    }

    #[test]
    fn repository_history_continues_after_two_hundred() {
        let (checkout, _) = linear(260);
        let all = collect(&checkout, 40);
        assert_eq!(all.len(), 260);
        let expected: Vec<_> = (1..=260).rev().map(|i| format!("commit {i}")).collect();
        assert_eq!(
            all.into_iter().map(|c| c.subject).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(page(&checkout, None, 0).1.len(), 40);
        assert_eq!(page(&checkout, None, u32::MAX).1.len(), 100);
    }

    #[test]
    fn repository_history_continues_beyond_twenty_thousand() {
        let (checkout, marks) = linear(20_005);
        let first = page(&checkout, None, 100);
        assert_eq!(first.1.len(), 100);
        assert_eq!(first.1[0].id, marks[&20_005]);
        let cursor = HistoryCursor::encode(
            gix::ObjectId::from_hex(marks[&20_005].as_bytes()).unwrap(),
            gix::ObjectId::from_hex(marks[&5].as_bytes()).unwrap(),
        );
        let (_, rows, next) = page(&checkout, Some(&cursor), 40);
        assert_eq!(
            rows.into_iter().map(|c| c.id).collect::<Vec<_>>(),
            (1..=4).rev().map(|i| marks[&i].clone()).collect::<Vec<_>>()
        );
        assert_eq!(next, None);
    }

    #[test]
    fn repository_history_keeps_snapshot_when_head_moves() {
        let (checkout, marks) = linear(6);
        let (head, first, next) = page(&checkout, None, 2);
        checkout.git(&["update-ref", "refs/heads/main", &marks[&3]]);
        let (same_head, second, _) = page(&checkout, next.as_deref(), 2);
        assert_eq!(head, same_head);
        assert_eq!(
            first.iter().map(|c| &c.id).collect::<Vec<_>>(),
            vec![&marks[&6], &marks[&5]]
        );
        assert_eq!(
            second.iter().map(|c| &c.id).collect::<Vec<_>>(),
            vec![&marks[&4], &marks[&3]]
        );
        assert_eq!(page(&checkout, None, 2).0, Some(marks[&3].clone()));
        checkout.git(&["repack", "-ad"]);
        checkout.git(&["commit-graph", "write", "--reachable"]);
        assert_eq!(page(&checkout, next.as_deref(), 2).1, second);
    }

    #[test]
    fn repository_history_empty_is_distinct_from_missing_or_invalid_objects() {
        let checkout = Checkout::new();
        assert_eq!(page(&checkout, None, 40), (None, vec![], None));
        for cursor in [
            "",
            "HEAD:HEAD",
            "a:b",
            &format!("{}:{}", "f".repeat(40), "e".repeat(40)),
        ] {
            assert!(
                repository_log(&checkout.0, Some(cursor), 40).is_err(),
                "{cursor}"
            );
        }
        std::fs::write(
            checkout.0.join(".git/refs/heads/main"),
            format!("{}\n", "f".repeat(40)),
        )
        .unwrap();
        assert!(repository_log(&checkout.0, None, 40).is_err());
    }

    #[test]
    fn repository_history_reports_a_missing_ancestor_instead_of_completion() {
        let (checkout, _) = linear(1);
        let tree = String::from_utf8(checkout.git(&["rev-parse", "HEAD^{tree}"])).unwrap();
        let raw = format!(
            "tree {}\nparent {}\nauthor A <a@example.test> 1700000001 +0000\ncommitter A <a@example.test> 1700000001 +0000\n\nmissing parent\n",
            tree.trim(),
            "f".repeat(40)
        );
        let oid = String::from_utf8(checkout.input(
            &["hash-object", "-t", "commit", "-w", "--stdin"],
            raw.as_bytes(),
        ))
        .unwrap();
        checkout.git(&["update-ref", "refs/heads/main", oid.trim()]);
        assert!(repository_log(&checkout.0, None, 40).is_err());
    }

    #[test]
    fn repository_history_rejects_unreachable_cursors_and_shallow_completion() {
        let checkout = Checkout::new();
        let mut stream = String::new();
        record(&mut stream, 1, "main", &[], 1_700_000_000, "A", "main");
        record(&mut stream, 2, "other", &[], 1_700_000_000, "A", "other");
        let marks = checkout.import(&stream);
        let cursor = format!("{}:{}", marks[&1], marks[&2]);
        assert!(
            repository_log(&checkout.0, Some(&cursor), 40)
                .unwrap_err()
                .contains("not reachable")
        );
        std::fs::write(checkout.0.join(".git/shallow"), format!("{}\n", marks[&1])).unwrap();
        assert!(
            repository_log(&checkout.0, None, 40)
                .unwrap_err()
                .contains("incomplete")
        );
    }

    #[test]
    fn repository_history_clips_utf8_metadata_without_sending_body() {
        let checkout = Checkout::new();
        let mut stream = String::new();
        record(
            &mut stream,
            1,
            "main",
            &[],
            1_700_000_000,
            &"é".repeat(160),
            &format!("{}\n\nPRIVATE BODY", "🦀".repeat(300)),
        );
        checkout.import(&stream);
        let (_, rows, _) = page(&checkout, None, 40);
        let row = &rows[0];
        assert_eq!(row.author, "é".repeat(128));
        assert_eq!(row.subject, "🦀".repeat(256));
        assert!(row.cut);
        assert!(!serde_json::to_string(row).unwrap().contains("PRIVATE BODY"));
        assert_eq!(
            clip_history_field("é".repeat(3), 5),
            ("éé".to_owned(), true)
        );
        assert_eq!(
            clip_history_field("plain".to_owned(), 5),
            ("plain".to_owned(), false)
        );
    }

    #[test]
    fn repository_history_counts_json_escaping_and_preserves_rejected_next_row() {
        let checkout = Checkout::new();
        let mut stream = String::new();
        for i in 1..=100 {
            let parents = if i == 1 { vec![] } else { vec![i - 1] };
            record(
                &mut stream,
                i,
                "main",
                &parents,
                1_700_000_000 + i,
                "A",
                &"\u{1}".repeat(1200),
            );
        }
        let marks = checkout.import(&stream);
        let first = page(&checkout, None, 100);
        assert!(first.1.len() < 100);
        assert!(first.2.is_some());
        let all = collect(&checkout, 100);
        assert!(all.iter().all(|row| row.cut && row.subject.len() == 1024));
        assert_eq!(
            all.into_iter().map(|c| c.id).collect::<Vec<_>>(),
            (1..=100)
                .rev()
                .map(|i| marks[&i].clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn git_watch_plan_keeps_unborn_nested_refs_and_tracks_identity_changes() {
        let checkout = Checkout::new();
        checkout.git(&["symbolic-ref", "HEAD", "refs/heads/topic/deep/main"]);
        let unborn = watch_plan(&checkout.0).unwrap().unwrap();
        assert_eq!(
            unborn.reference.as_ref().unwrap().as_bstr(),
            b"refs/heads/topic/deep/main".as_bstr()
        );
        assert!(
            unborn
                .directories
                .contains(&checkout.0.join(".git/refs/heads/topic/deep"))
        );
        assert!(!checkout.0.join(".git/refs/heads/topic/deep").exists());
        checkout.git(&["symbolic-ref", "HEAD", "refs/heads/other/deep/main"]);
        let switched = watch_plan(&checkout.0).unwrap().unwrap();
        assert_ne!(unborn, switched);
        assert!(switched.directories.contains(&checkout.0.join(".git")));
    }

    #[test]
    fn git_watch_plan_uses_real_linked_worktree_gitdir_and_common_refs() {
        let (checkout, marks) = linear(1);
        let linked = checkout.0.join("linked");
        checkout.git(&[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "topic/deep",
            linked.to_str().unwrap(),
            &marks[&1],
        ]);
        let repo = gix::open(&linked).unwrap();
        assert!(linked.join(".git").is_file());
        assert_ne!(repo.git_dir(), repo.common_dir());
        let plan = watch_plan(&linked).unwrap().unwrap();
        assert!(plan.directories.contains(&repo.git_dir().to_path_buf()));
        assert!(plan.directories.contains(&repo.common_dir().to_path_buf()));
        assert!(
            plan.directories
                .contains(&repo.common_dir().join("refs/heads/topic"))
        );
        assert!(
            !plan.directories.contains(&linked.join(".git")),
            "a linked marker file is not a watch directory"
        );
        assert_eq!(
            plan.reference.as_ref().unwrap().as_bstr(),
            b"refs/heads/topic/deep".as_bstr()
        );
    }

    #[test]
    fn git_watch_plan_covers_upstream_and_external_index_without_a_status_walk() {
        let (checkout, _) = linear(1);
        checkout.git(&[
            "config",
            "remote.origin.url",
            "https://fixture.invalid/repo",
        ]);
        checkout.git(&[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ]);
        checkout.git(&["config", "branch.main.remote", "origin"]);
        checkout.git(&["config", "branch.main.merge", "refs/heads/topic/deep"]);
        let external = checkout.0.join("external-index/state/index");
        checkout.git(&[
            "config",
            "gitoxide.core.indexFile",
            external.to_str().unwrap(),
        ]);
        let plan = watch_plan(&checkout.0).unwrap().unwrap();
        assert_eq!(
            plan.upstream.as_ref().unwrap().as_bstr(),
            b"refs/remotes/origin/topic/deep".as_bstr()
        );
        assert!(
            plan.directories
                .contains(&checkout.0.join(".git/refs/remotes/origin/topic"))
        );
        assert!(
            plan.directories
                .contains(&external.parent().unwrap().to_path_buf())
        );
    }

    #[test]
    fn git_watch_plan_handles_plain_folders_and_detached_head() {
        let checkout = Checkout::new();
        let plain = checkout.0.join("plain");
        std::fs::create_dir(&plain).unwrap();
        std::fs::rename(checkout.0.join(".git"), checkout.0.join("git-not-a-marker")).unwrap();
        assert_eq!(watch_plan(&plain).unwrap(), None);
        let (checkout, marks) = linear(1);
        checkout.git(&["update-ref", "--no-deref", "HEAD", &marks[&1]]);
        let detached = watch_plan(&checkout.0).unwrap().unwrap();
        assert_eq!(detached.reference, None);
        assert_eq!(detached.upstream, None);
        assert!(detached.directories.contains(&checkout.0.join(".git")));
    }
}
