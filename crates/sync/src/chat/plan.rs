//! How two sides align over files. Pure planning — no IO, so any
//! transport (live stream, ssh) can drive it.
//!
//! Blocks are immutable and (author, seq) is globally unique, so "does
//! the far side have this block" is a filename comparison: the far side
//! only needs `ls` and `cp`, which is exactly what lets a node that runs
//! nothing participate.
//!
//! But "do I have it" must be judged by the ledger ("ever merged"), not
//! the directory ("still present"): compaction deletes files whose
//! content lives on in the snapshot, and judging by directory would
//! re-pull them on every sync — compaction would never stay done.

use std::collections::BTreeSet;

/// One side's books.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Side {
    /// Blocks currently in the directory.
    pub present: BTreeSet<String>,
    /// Blocks ever merged (including ones compaction deleted). Empty on
    /// a dumb node — it merges nothing — which degrades the algorithm to
    /// a plain directory diff, no special case needed.
    pub merged: BTreeSet<String>,
}

impl Side {
    pub fn new(present: impl IntoIterator<Item = String>, merged: impl IntoIterator<Item = String>) -> Self {
        Self {
            present: present.into_iter().collect(),
            merged: merged.into_iter().collect(),
        }
    }
}

/// What this round moves. Both directions from one computation: two
/// separate passes would read the sides at different moments, and
/// "should the block I just pulled be pushed back" stops having an
/// answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub pull: Vec<String>,
    pub push: Vec<String>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.pull.is_empty() && self.push.is_empty()
    }
}

/// Pull: present there, never merged here. Push: present here, neither
/// present nor merged there — merged means they have it, even if their
/// compaction deleted the file. `BTreeSet` keeps the output ordered and
/// reproducible.
pub fn plan(mine: &Side, theirs: &Side) -> Plan {
    Plan {
        pull: theirs.present.difference(&mine.merged).cloned().collect(),
        push: mine
            .present
            .iter()
            .filter(|n| !theirs.present.contains(*n) && !theirs.merged.contains(*n))
            .cloned()
            .collect(),
    }
}

/// The merged-blocks ledger. Plain text, one name per line — not JSON:
/// single-column data, and line-appends under PIPE_BUF are atomic, so two
/// processes appending never corrupt each other's line.
#[derive(Debug, Clone, Default)]
pub struct Ledger(BTreeSet<String>);

impl Ledger {
    pub fn parse(text: &str) -> Self {
        Self(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect(),
        )
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        for n in &self.0 {
            s.push_str(n);
            s.push('\n');
        }
        s
    }

    pub fn insert(&mut self, name: impl Into<String>) -> bool {
        self.0.insert(name.into())
    }

    pub fn has(&self, name: &str) -> bool {
        self.0.contains(name)
    }

    pub fn names(&self) -> &BTreeSet<String> {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
