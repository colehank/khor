//! 导入 (docs/SHELL.md 第八轮): what could be brought onto the session
//! list, and the bringing. Renames the node's answers for TS and decides
//! nothing — `Node::import_candidates` and `Node::import` do.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// One candidate, as the import panel draws it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct CandidateRow {
    /// What goes back to `import_sessions`.
    pub id: String,
    /// `shell`, `tui` (watched) or `gui` (an ACP conversation).
    pub form: String,
    /// The machine, by name — and by id, for its face.
    pub machine: String,
    pub device: String,
    pub title: String,
    pub place: Option<String>,
    #[ts(type = "number")]
    pub at_ms: i64,
    pub category: Option<String>,
    pub agent: Option<String>,
}

pub fn import_candidates(root: &Path) -> Result<Vec<CandidateRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    let names: std::collections::HashMap<String, String> = n
        .devices()?
        .into_iter()
        .map(|d| (d.id.clone(), d.shown().to_owned()))
        .collect();
    Ok(n.import_candidates()?
        .into_iter()
        .map(|c| CandidateRow {
            machine: names
                .get(&c.home)
                .cloned()
                .unwrap_or_else(|| c.home.clone()),
            device: c.home,
            id: c.id,
            form: c.form,
            title: c.title,
            place: c.place,
            at_ms: c.at_ms,
            category: c.category,
            agent: c.agent,
        })
        .collect())
}

/// Brings each onto the list, in order; answers the session ids. **Stops
/// at the first refusal** and says it: a panel that imported three of
/// five and reported success would leave two rows the person thinks are
/// on the list.
pub fn import_sessions(root: &Path, ids: &[String]) -> Result<Vec<String>, String> {
    let n = Node::open(root.to_path_buf())?;
    ids.iter().map(|id| n.import(id).map(|s| s.0)).collect()
}
