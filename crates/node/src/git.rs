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
    let (git, common) = (repo.git_dir().to_path_buf(), repo.common_dir().to_path_buf());
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
        .map(|b| b.split(|&c| c == b'\n').filter(|l| !l.is_empty()).count().min(u32::MAX as usize) as u32)
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

/// A question about one file's history. **One wire word for the four**
/// (`proto::Request::Git`), each its own variant here, so a later
/// question rides the same word and an older khor refuses the word once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitAsk {
    /// The commits that changed `path`, newest first, following the first
    /// parent (what `git log --first-parent -- path` lists).
    Log { path: String, limit: u32 },
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
    pub hunks: Vec<CommitHunk>,
}

/// One unified hunk, `git show`'s shape: [`CONTEXT`] unchanged lines on
/// either side of a run of changes, two runs closer than twice that
/// joined. **Each line carries its own first character** — `' '`, `-`,
/// `+` — the way a patch does, so there is no second list that could
/// disagree with it about which line is which.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitHunk {
    /// 1-based, where the hunk's first line sits on each side.
    pub old_at: u32,
    pub new_at: u32,
    pub lines: Vec<String>,
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
    Log { commits: Vec<Commit>, more: bool },
    Blame { commits: Vec<Commit>, runs: Vec<BlameRun> },
    /// `text` is the revision's file, decoded lossily and cut at
    /// [`SHOWN`] (`cut` says so); `hunks` are where the current file
    /// differs from it, in the current file's lines — the gutter's own
    /// shape, so 「与当前比较」 is the diff view the file already has.
    Show { text: String, cut: bool, hunks: Vec<Hunk>, more: bool },
    /// One commit and the files it changed, in path order. `total` is how
    /// many files it changed in all; `more` says the list stopped at
    /// [`COMMIT_FILES`] files or [`COMMIT_LINES`] changed lines — said,
    /// never cut silently.
    Commit { commit: Commit, files: Vec<CommitFile>, total: u32, more: bool },
}

/// Answers one history question on this machine.
pub fn answer(ask: &GitAsk) -> Result<GitAnswer, String> {
    match ask {
        GitAsk::Log { path, limit } => log(Path::new(path), (*limit).clamp(1, LOG)),
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
    let author = c.author().map(|a| a.name.to_str_lossy().into_owned()).unwrap_or_default();
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
        return Ok(GitAnswer::Log { commits: Vec::new(), more: false });
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
            let before = info.parent_ids.first().and_then(|p| blob_in(&repo, *p, &rel));
            here.is_some() && here != before
        };
        if changed {
            let c = repo.find_commit(info.id).map_err(|e| e.to_string())?;
            commits.push(commit_of(&c));
        }
    }
    Ok(GitAnswer::Log { commits, more })
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
        Some(blob) => repo.find_object(blob).map_err(|e| e.to_string())?.data.clone(),
        None => Vec::new(),
    };
    let new = std::fs::read(path).map_err(|e| e.to_string())?;
    let Diff { hunks, more } = hunks(&old, &new);
    let cut = old.len() > SHOWN;
    let text = String::from_utf8_lossy(&old[..old.len().min(SHOWN)]).into_owned();
    Ok(GitAnswer::Show { text, cut, hunks, more })
}

/// How many files one [`GitAnswer::Commit`] lists, and how many changed
/// lines it carries across them. A vendored-dependency bump touches
/// thousands; past either the answer says `more` and the page says so.
pub(crate) const COMMIT_FILES: usize = 50;
pub(crate) const COMMIT_LINES: usize = 2000;
/// Unchanged lines around a change, `git diff`'s default.
pub(crate) const CONTEXT: u32 = 3;

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
    let mut rows: Vec<(String, String, &'static str, Option<gix::ObjectId>, Option<gix::ObjectId>)> = Vec::new();
    for c in changes {
        if !c.entry_mode().is_blob_or_symlink() {
            continue;
        }
        let path = c.location().to_str_lossy().into_owned();
        rows.push(match c {
            C::Addition { id, .. } => (path, String::new(), ADDED, None, Some(id)),
            C::Deletion { id, .. } => (path, String::new(), DELETED, Some(id), None),
            C::Modification { previous_id, id, .. } => (path, String::new(), MODIFIED, Some(previous_id), Some(id)),
            C::Rewrite { copy: true, id, .. } => (path, String::new(), ADDED, None, Some(id)),
            C::Rewrite { source_location, source_id, id, .. } => {
                (path, source_location.to_str_lossy().into_owned(), RENAMED, Some(source_id), Some(id))
            }
        });
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let total = rows.len().min(u32::MAX as usize) as u32;
    let read = |id: Option<gix::ObjectId>| -> Result<Vec<u8>, String> {
        match id {
            Some(id) => Ok(repo.find_object(id).map_err(|e| e.to_string())?.data.clone()),
            None => Ok(Vec::new()),
        }
    };
    let mut files = Vec::new();
    let mut budget = COMMIT_LINES;
    let mut more = false;
    for (path, from, status, was, now) in rows {
        if files.len() >= COMMIT_FILES || budget == 0 {
            more = true;
            break;
        }
        let (a, b) = (read(was)?, read(now)?);
        let binary = [&a, &b].iter().any(|x| x[..x.len().min(8000)].contains(&0));
        let (hunks, added, removed, cut) = if binary {
            (Vec::new(), 0, 0, false)
        } else {
            unified(&a, &b, &mut budget)
        };
        more |= cut;
        files.push(CommitFile {
            path,
            from,
            status: status.to_owned(),
            added,
            removed,
            binary,
            hunks,
        });
        if cut {
            break;
        }
    }
    Ok(GitAnswer::Commit {
        commit: commit_of(&commit),
        files,
        total,
        more,
    })
}

/// Two texts as unified hunks, spending `budget` changed lines; the
/// counts are the whole file's whatever the budget, `cut` says the hunks
/// stopped. Lines lose their terminators, as [`hunks`]'s `was` does.
fn unified(old: &[u8], new: &[u8], budget: &mut usize) -> (Vec<CommitHunk>, u32, u32, bool) {
    use gix::diff::blob::{Algorithm, InternedInput, sources};
    let input = InternedInput::new(sources::byte_lines(old), sources::byte_lines(new));
    let mut d = gix::diff::blob::Diff::compute(Algorithm::Histogram, &input);
    d.postprocess_lines(&input);
    let line = |tok: gix::diff::blob::Token| -> String {
        let raw = String::from_utf8_lossy(input.interner[tok]);
        raw.strip_suffix('\n')
            .map(|s| s.strip_suffix('\r').unwrap_or(s))
            .unwrap_or(&raw)
            .to_owned()
    };
    let raw: Vec<_> = d.hunks().collect();
    let added = raw.iter().map(|h| h.after.end - h.after.start).sum();
    let removed = raw.iter().map(|h| h.before.end - h.before.start).sum();
    let n_old = input.before.len() as u32;
    let mut out: Vec<CommitHunk> = Vec::new();
    let mut cut = false;
    let mut i = 0;
    while i < raw.len() {
        // One unified hunk: from CONTEXT above the first run to CONTEXT
        // below the last run that sits within 2×CONTEXT of the one before.
        let first = &raw[i];
        let lead = first.before.start.min(CONTEXT);
        let mut h = CommitHunk {
            old_at: first.before.start - lead + 1,
            new_at: first.after.start - lead + 1,
            lines: Vec::new(),
        };
        let mut at = first.before.start - lead;
        loop {
            let r = &raw[i];
            for k in at..r.before.start {
                h.lines.push(format!(" {}", line(input.before[k as usize])));
            }
            let size = (r.before.end - r.before.start + r.after.end - r.after.start) as usize;
            if size > *budget {
                cut = true;
                *budget = 0;
                break;
            }
            *budget -= size;
            for k in r.before.clone() {
                h.lines.push(format!("-{}", line(input.before[k as usize])));
            }
            for k in r.after.clone() {
                h.lines.push(format!("+{}", line(input.after[k as usize])));
            }
            at = r.before.end;
            i += 1;
            if i >= raw.len() || raw[i].before.start - at > 2 * CONTEXT {
                break;
            }
        }
        if !cut {
            for k in at..(at + CONTEXT).min(n_old) {
                h.lines.push(format!(" {}", line(input.before[k as usize])));
            }
        }
        if h.lines.iter().any(|l| !l.starts_with(' ')) {
            out.push(h);
        }
        if cut {
            break;
        }
    }
    (out, added, removed, cut)
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
        assert!(!commits.is_empty(), "the manifest was committed at least once");
        assert!(commits.len() <= 5);
        assert!(commits.windows(2).all(|w| w[0].when_ms >= w[1].when_ms || w[0].when_ms == 0));
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
    /// of this checkout changed at least one file, and says where.
    #[test]
    fn head_answers_its_files_with_hunks() {
        let root = here();
        let head = gix::open(&root).expect("open").head_id().expect("head").to_string();
        let GitAnswer::Commit { commit, files, total, .. } = answer(&GitAsk::Commit {
            root: root.to_string_lossy().into_owned(),
            rev: head.clone(),
        })
        .expect("commit") else {
            panic!("not a commit")
        };
        assert_eq!(commit.id, head);
        assert!(!files.is_empty() && total as usize >= files.len());
        let f = files.iter().find(|f| !f.binary).expect("a text file");
        assert!(!f.hunks.is_empty(), "{} changed and says nowhere", f.path);
        assert!(f.added + f.removed > 0);
        for h in &f.hunks {
            assert!(h.lines.iter().all(|l| matches!(l.chars().next(), Some(' ' | '+' | '-'))));
        }
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
        let GitAnswer::Commit { files, total, more, .. } = answer(&GitAsk::Commit {
            root: root.to_string_lossy().into_owned(),
            rev: first.id.to_string(),
        })
        .expect("commit") else {
            panic!("not a commit")
        };
        assert!(!files.is_empty());
        assert!(files.iter().all(|f| f.status == ADDED && f.removed == 0), "{files:?}");
        if files.len() < total as usize {
            assert!(more, "{} of {total} files listed and the answer did not say so", files.len());
        }
    }

    #[test]
    fn two_runs_close_together_are_one_hunk_with_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n";
        let new = "1\n2\n3\nfour\n5\n6\n7\neight\n9\n10\n11\n12\n";
        let mut budget = COMMIT_LINES;
        let (h, added, removed, cut) = unified(old.as_bytes(), new.as_bytes(), &mut budget);
        assert_eq!((added, removed, cut), (2, 2, false));
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].old_at, h[0].new_at), (1, 1));
        assert_eq!(h[0].lines.first().map(String::as_str), Some(" 1"));
        assert_eq!(h[0].lines.last().map(String::as_str), Some(" 11"));
        assert!(h[0].lines.contains(&"-4".to_owned()) && h[0].lines.contains(&"+eight".to_owned()));
    }

    #[test]
    fn the_line_budget_says_it_stopped() {
        let new: String = (0..10).map(|i| format!("{i}\n")).collect();
        let mut budget = 4;
        let (h, added, _, cut) = unified(b"", new.as_bytes(), &mut budget);
        assert!(cut, "ten added lines do not fit in four");
        assert_eq!(added, 10, "the count is the whole file's");
        assert!(h.is_empty() || h.iter().all(|h| h.lines.len() <= 4));
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
