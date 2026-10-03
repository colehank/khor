//! The workspace's git layer: what differs from HEAD, shaped for the
//! screen. Every judgment — which letter a path wears, where a hunk
//! sits, what counts as a checkout — is made in the node
//! (`khor_node::git`); this layer renames fields for TS and decides
//! nothing.
//!
//! # Why the diff is not computed here
//!
//! It would be easy to: the file's text already crosses to this
//! process, and HEAD's could ride beside it. **But docs/KHOR.md's third
//! foundation is that the CLI and the GUI are equivalent** — a diff
//! computed in the frontend is a diff `khor` cannot print. The hunk
//! shape on the wire is the same one both faces read.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// One path that differs from HEAD, and how.
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitRow {
    /// Relative to the workspace root, `/`-separated.
    pub path: String,
    /// One of `M`, `A`, `D`, `U` — **git's own letters, not khor's six
    /// state words.** Those belong to sessions (docs/UX.md), and a file
    /// is not a session; these are what every other tool a person uses
    /// already calls them.
    pub mark: String,
}

/// One checkout's state, as of the moment it was read.
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitStatus {
    /// Short (`main`). **Empty means there is no name to say** —
    /// detached, or no commit yet — and the head paints nothing rather
    /// than inventing a word for it.
    pub branch: String,
    pub rows: Vec<GitRow>,
    /// The list stopped early rather than being whole.
    pub more: bool,
    /// Commits this branch has that its remote does not, and the other
    /// way round. Both 0 when there is no upstream, which is also what
    /// being in step looks like — the face draws nothing either way.
    pub ahead: u32,
    pub behind: u32,
    /// When this repository last heard from its remote, epoch
    /// milliseconds, **0 meaning never**.
    ///
    /// **The pair above is unsayable without it.** khor never fetches,
    /// so ahead/behind is always「how far it was the last time somebody
    /// did」— and a three-week-old number has exactly the same shape as
    /// a fresh one. The face says the two together or says neither
    /// (工作区批3 cut the pair for this; 2026-09-13 brought it back
    /// with its date attached).
    #[ts(type = "number")]
    pub fetched_ms: i64,
}

/// One place a file differs from HEAD, in the coordinates of the file
/// **as it is now**. Which of the three kinds it is falls out of which
/// half is empty — inserted (`lines` > 0, `was` empty), deleted
/// (`lines` == 0, `was` full), changed (both). There is deliberately no
/// fourth field saying so, because a fourth field can disagree.
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitHunk {
    /// 1-based. When `lines` is 0 this is the line the deletion sits
    /// **after**, so a deletion at the top of a file is 0.
    pub at: u32,
    pub lines: u32,
    /// What HEAD had there, one string per line, no terminators.
    pub was: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct GitDiff {
    pub hunks: Vec<GitHunk>,
    pub more: bool,
}

/// One checkout's marks off any machine (`khor_node::Node::
/// git_status_on`; empty names this machine).
///
/// **A refusal arrives as an error, never as an empty list.** No rows
/// means「nothing differs from HEAD」— a clean tree, safe to edit — and
/// that is the one thing that must not be painted when khor could not
/// read the repository at all.
pub async fn status(root: &Path, machine: &str, dir: &str) -> Result<GitStatus, String> {
    let n = Node::open(root.to_path_buf())?;
    let s = n.git_status_on(machine, dir).await?;
    Ok(GitStatus {
        branch: s.branch,
        rows: s
            .rows
            .into_iter()
            .map(|(path, mark)| GitRow { path, mark })
            .collect(),
        more: s.more,
        ahead: s.ahead,
        behind: s.behind,
        fetched_ms: s.fetched_ms,
    })
}

/// One file's hunks off any machine (`khor_node::Node::git_diff_on`).
/// Refuses out loud for [`status`]'s reason: no hunks means the file
/// matches HEAD.
pub async fn diff(root: &Path, machine: &str, path: &str) -> Result<GitDiff, String> {
    let n = Node::open(root.to_path_buf())?;
    let d = n.git_diff_on(machine, path).await?;
    Ok(GitDiff {
        hunks: d
            .hunks
            .into_iter()
            .map(|h| GitHunk {
                at: h.at,
                lines: h.lines,
                was: h.was,
            })
            .collect(),
        more: d.more,
    })
}

// ── One file's history (round UX-4, the file page's git 历史) ──────────

/// One commit as a history row reads it (`khor_node::git::Commit`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitCommit {
    /// Full hex; the face shortens it.
    pub id: String,
    pub author: String,
    #[ts(type = "number")]
    pub when_ms: i64,
    pub subject: String,
}

/// The commits that changed one file, newest first. `more`: the list or
/// the walk stopped early.
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitLog {
    pub commits: Vec<GitCommit>,
    pub more: bool,
}

/// One run of HEAD's lines a commit wrote: `start` 0-based, `len` lines,
/// `commit` an index into [`GitBlame::commits`].
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitBlameRun {
    pub start: u32,
    pub len: u32,
    pub commit: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct GitBlame {
    pub commits: Vec<GitCommit>,
    pub runs: Vec<GitBlameRun>,
}

/// A revision of one file, and where the file as it is now differs from
/// it — the gutter's own shape, so comparing is the diff view.
#[derive(Debug, Clone, Serialize, TS)]
pub struct GitShow {
    pub text: String,
    /// The text stopped at the file view's read cap.
    pub cut: bool,
    pub diff: GitDiff,
}

fn commit(c: khor_node::git::Commit) -> GitCommit {
    GitCommit {
        id: c.id,
        author: c.author,
        when_ms: c.when_ms,
        subject: c.subject,
    }
}

fn hunk(h: khor_node::proto::Hunk) -> GitHunk {
    GitHunk {
        at: h.at,
        lines: h.lines,
        was: h.was,
    }
}

async fn ask(root: &Path, machine: &str, q: khor_node::git::GitAsk) -> Result<khor_node::git::GitAnswer, String> {
    Node::open(root.to_path_buf())?.git_on(machine, &q).await
}

/// One file's log off any machine (`khor_node::Node::git_on`).
pub async fn log(root: &Path, machine: &str, path: &str) -> Result<GitLog, String> {
    use khor_node::git::{GitAnswer, GitAsk};
    match ask(root, machine, GitAsk::Log { path: path.to_owned(), limit: 200 }).await? {
        GitAnswer::Log { commits, more } => Ok(GitLog {
            commits: commits.into_iter().map(commit).collect(),
            more,
        }),
        other => Err(format!("{other:?}")),
    }
}

/// Who wrote each of one file's lines, as HEAD has it.
pub async fn blame(root: &Path, machine: &str, path: &str) -> Result<GitBlame, String> {
    use khor_node::git::{GitAnswer, GitAsk};
    match ask(root, machine, GitAsk::Blame { path: path.to_owned() }).await? {
        GitAnswer::Blame { commits, runs } => Ok(GitBlame {
            commits: commits.into_iter().map(commit).collect(),
            runs: runs
                .into_iter()
                .map(|r| GitBlameRun {
                    start: r.start,
                    len: r.len,
                    commit: r.commit,
                })
                .collect(),
        }),
        other => Err(format!("{other:?}")),
    }
}

/// One file as revision `rev` had it.
pub async fn show(root: &Path, machine: &str, path: &str, rev: &str) -> Result<GitShow, String> {
    use khor_node::git::{GitAnswer, GitAsk};
    let q = GitAsk::Show {
        path: path.to_owned(),
        rev: rev.to_owned(),
    };
    match ask(root, machine, q).await? {
        GitAnswer::Show { text, cut, hunks, more } => Ok(GitShow {
            text,
            cut,
            diff: GitDiff {
                hunks: hunks.into_iter().map(hunk).collect(),
                more,
            },
        }),
        other => Err(format!("{other:?}")),
    }
}

/// Whether that directory is a checkout (`khor_node::Node::checkout_on`;
/// empty names this machine). **Asked beside [`status`], not after it**:
/// the workspace row's branch-or-folder corner is this answer, and the
/// status walk is the one that can be slow or fail
/// (docs/SHELL.md 第五轮 第 6 条).
pub async fn checkout(root: &Path, machine: &str, dir: &str) -> Result<bool, String> {
    Node::open(root.to_path_buf())?
        .checkout_on(machine, dir)
        .await
}
