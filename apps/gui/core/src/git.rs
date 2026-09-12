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
        rows: s.rows.into_iter().map(|(path, mark)| GitRow { path, mark }).collect(),
        more: s.more,
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
            .map(|h| GitHunk { at: h.at, lines: h.lines, was: h.was })
            .collect(),
        more: d.more,
    })
}
