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

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull is judged by "ever merged", not "still on disk".
    #[test]
    fn the_plan_pulls_what_i_have_not_merged_not_what_i_lack_on_disk() {
        // I compacted: only the snapshot remains, but the ledger
        // remembers.
        let mine = Side::new(
            vec!["snap-00000002.loro".to_string()],
            vec![
                "u-0000000000000001-00000000.loro".to_string(),
                "u-0000000000000001-00000001.loro".to_string(),
                "snap-00000002.loro".to_string(),
            ],
        );
        let theirs = Side::new(
            vec![
                "u-0000000000000001-00000000.loro".to_string(),
                "u-0000000000000001-00000001.loro".to_string(),
            ],
            Vec::<String>::new(),
        );
        let p = plan(&mine, &theirs);
        assert!(
            p.pull.is_empty(),
            "压实掉的块不许被拉回来,否则压实成了一个永远做不完的动作:{:?}",
            p.pull
        );
        assert_eq!(p.push, vec!["snap-00000002.loro"], "我的快照该推过去");
    }

    /// Push must honour the far side's ledger: merged means they have
    /// it, even if their compaction deleted the file.
    #[test]
    fn the_plan_does_not_push_what_they_already_merged() {
        let mine = Side::new(vec!["u-a.loro".to_string()], vec!["u-a.loro".to_string()]);
        let theirs = Side::new(Vec::<String>::new(), vec!["u-a.loro".to_string()]);
        assert!(
            plan(&mine, &theirs).push.is_empty(),
            "对面合过的不该再推"
        );
        // Control: never-met must be pushed to, or a push-nothing
        // implementation stays green.
        let never = Side::default();
        assert_eq!(plan(&mine, &never).push, vec!["u-a.loro"]);
    }

    /// The ledger's text format: one name per line, round-trips.
    #[test]
    fn the_ledger_round_trips() {
        let mut l = Ledger::default();
        l.insert("u-b.loro");
        l.insert("u-a.loro");
        let text = l.render();
        assert_eq!(text, "u-a.loro\nu-b.loro\n", "有序,一行一个");
        let back = Ledger::parse(&text);
        assert_eq!(back.len(), 2);
        assert!(back.has("u-a.loro") && back.has("u-b.loro"));
        // Blank lines and whitespace must not become entries.
        assert_eq!(Ledger::parse("\n  \n u-a.loro \n\n").len(), 1);
    }
}
