//! Opens still on their way, and the leaves that arrived before them.
//!
//! A far terminal or conversation is attached over the network, so its
//! open runs off the window's thread (2026-10-02: on the main thread,
//! one machine that had dropped off froze the phone for the length of
//! the reach). That costs the order the window used to get for free:
//! a face that mounts and unmounts at once (React's double mount, a
//! person tapping back) sends its open and then its leave, and the
//! leave can now land first — on a registry with nothing in it, where
//! it did nothing, after which the open put in an attachment nobody
//! holds and its reader kept reaching for ever.
//!
//! So a leave for an id with an open in flight is written down here,
//! and the open that lands next takes it instead of a hold. The ledger
//! is consulted only under the registry's own lock (the terminal's or
//! the conversation's), so the two never disagree about an id.

use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Landing {
    opening: HashMap<String, usize>,
    early: HashMap<String, usize>,
}

impl Landing {
    /// An open has started reaching for `id`.
    pub(crate) fn begin(&mut self, id: &str) {
        *self.opening.entry(id.to_owned()).or_default() += 1;
    }

    /// A leave found nothing registered under `id`. Kept for an open
    /// still on its way, if there is one; answers whether there was.
    pub(crate) fn leave_early(&mut self, id: &str) -> bool {
        if self.opening.get(id).copied().unwrap_or(0) == 0 {
            return false;
        }
        *self.early.entry(id.to_owned()).or_default() += 1;
        true
    }

    /// An open for `id` has landed, or failed. Answers whether a leave
    /// already came for it — then it takes no hold.
    pub(crate) fn land(&mut self, id: &str) -> bool {
        let left = match self.early.get_mut(id) {
            Some(n) if *n > 0 => {
                *n -= 1;
                true
            }
            _ => false,
        };
        if let Some(n) = self.opening.get_mut(id) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.opening.remove(id);
                self.early.remove(id);
            }
        }
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The double mount: open, leave before it lands, open again. The
    /// first to land takes the early leave; the second holds.
    #[test]
    fn an_early_leave_is_taken_by_the_next_open_to_land() {
        let mut l = Landing::default();
        l.begin("t");
        assert!(l.leave_early("t"), "an open is on its way");
        l.begin("t");
        assert!(l.land("t"), "the first to land takes the leave");
        assert!(!l.land("t"), "the second holds");
        assert!(!l.leave_early("t"), "nothing in flight any more");
    }

    /// A leave with no open in flight is the ordinary no-op it was.
    #[test]
    fn a_leave_with_nothing_on_its_way_is_not_kept() {
        let mut l = Landing::default();
        assert!(!l.leave_early("t"));
        l.begin("t");
        assert!(!l.land("t"), "no leave was kept for it");
    }
}
