//! The workspace landing's data layer: one machine's checkouts, shaped
//! for the screen. The line between a checkout and any other directory
//! is drawn in the node (`khor_node::place`); this layer renames fields
//! for TS and decides nothing.
//!
//! # Why one machine at a time
//!
//! The landing shows every machine's workspaces in one list, but it
//! asks for them one machine at a time on purpose. A machine that is
//! slow does not hold up the rest, and — the reason that matters — a
//! machine that cannot answer is a fact about *that machine*. Folded
//! into one call, four different answers (not asked yet / cannot be
//! reached / has none / here they are) would arrive as one list and one
//! error, and the first three would be indistinguishable
//! (`scripts/check-acp.mjs` 第二批 holds the same line for the agent
//! registry).

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// One project on one machine.
///
/// **Not [`crate::PlaceRow`], though the four fields are the same
/// today.** That one answers 「where could a session start」 for the
/// wizard and takes every directory one ever ran in; this one answers
/// 「what is being worked on」 and takes only checkouts. They also grow
/// apart from here: a workspace row is where a branch and a count of
/// uncommitted files will land, and neither belongs in a wizard's
/// completion list.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WorkspaceRow {
    /// Absolute, in the far machine's own spelling.
    pub path: String,
    /// The same path as that machine's user reads it, home written `~`.
    /// Shortened over there, because whose home a path is under is a
    /// fact about the machine and not about this screen.
    pub shown: String,
    /// How many recorded sessions fold into it. A count of what the
    /// vendors' own files remember, never a total.
    pub sessions: u32,
    #[ts(type = "number")]
    pub last_ms: i64,
}

/// One machine's workspaces, newest first (`khor_node::Node::
/// workspaces_on`; empty names this machine).
///
/// **Uncapped, unlike the wizard's places.** That list is a completion
/// popup, where a cap costs a keystroke; this one is the landing
/// itself, and a project silently missing from it is a project a person
/// concludes they never worked on. The number of checkouts on a disk is
/// bounded by how many a person made.
///
/// A khor over there that predates the op refuses, and that refusal
/// arrives here as an error rather than as an empty list — the two mean
/// different next moves (`khor_catalog::msg::WORKSPACES_PREDATES`).
pub async fn workspaces(root: &Path, machine: &str) -> Result<Vec<WorkspaceRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    Ok(n.workspaces_on(machine)
        .await?
        .into_iter()
        .map(|(path, shown, sessions, last_ms)| WorkspaceRow { path, shown, sessions, last_ms })
        .collect())
}
