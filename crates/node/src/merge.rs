//! Three texts in, one text out: the merge behind a refused save
//! (工作区批7).
//!
//! **Why this exists.** `files::write_whole` refuses a save whose `was`
//! is not the file's digest any more, and until now a person had two
//! answers to that refusal: throw their edit away, or throw the other
//! change away. Both are lossy, and the third answer — keep both, since
//! they usually touched different lines — needed an algorithm this tree
//! did not have.
//!
//! **Why gitoxide's and not another.** Two candidates, both measured off
//! the `Cargo.lock` diff on 2026-09-13: `gix`'s `merge` feature adds
//! exactly one package (`gix-merge`, everything under it already here
//! for `blob-diff`), and `diffy` adds exactly one package. A tie on
//! price, so the tie breaks on what a person can predict: this *is*
//! git's own text driver, so the markers, the minimization and the
//! auto-resolutions are the ones `git merge` would produce on the same
//! three texts. khor already speaks git through gix everywhere else
//! (`crate::git`); a second, differently-behaved merge in the same app
//! would be a second answer to one question.
//!
//! **This module writes nothing.** It is a function over three strings.
//! The merged text goes back in front of the person, and the file on
//! disk changes only when somebody presses save — the same rule the
//! undo step was built under (`file-undo` in the catalog).

use gix::merge::blob::builtin_driver::{self, text};
use khor_catalog::msg;

/// A merge that has been performed, and how much of it is still the
/// person's to decide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    /// The merged text, conflict markers included where there are any.
    pub text: String,
    /// How many places the two sides disagreed about and khor would not
    /// pick for them. `0` means the text is finished.
    pub clashes: u32,
}

/// The marker style. `Merge` is git's default: the base is not printed,
/// only the two sides. The other two styles print the base as well,
/// which is more information in a box the person is about to edit —
/// chosen against, because what they are looking for is which of *two*
/// lines to keep, and a third line between them is one more thing to
/// delete by hand.
const STYLE: text::ConflictStyle = text::ConflictStyle::Merge;

/// Merges `mine` and `theirs` over the `base` both started from.
///
/// - lines only one side touched: taken, from whichever side touched
///   them;
/// - lines both sides changed **the same way**: taken once (git calls
///   this an auto-resolution, and it is not a clash);
/// - lines both sides changed differently: both kept, between markers,
///   and counted.
///
/// The labels on the markers name the two sides in the words the
/// buttons beside them use, because the person reading the markers just
/// pressed one of those buttons.
pub fn three(base: &str, mine: &str, theirs: &str) -> Merged {
    let mut out = Vec::new();
    let mut input = gix::diff::blob::InternedInput::default();
    let labels = text::Labels {
        ancestor: None,
        current: Some(msg::MERGE_MINE.into()),
        other: Some(msg::MERGE_THEIRS.into()),
    };
    builtin_driver::text(
        &mut out,
        &mut input,
        labels,
        mine.as_bytes(),
        base.as_bytes(),
        theirs.as_bytes(),
        text::Options {
            conflict: text::Conflict::Keep {
                style: STYLE,
                marker_size: text::Conflict::DEFAULT_MARKER_SIZE.try_into().expect("7 is not zero"),
            },
            ..Default::default()
        },
    );
    // **The merge is over bytes and the box holds a string.** Both
    // sides came in as `&str`, so every byte in `out` came out of one
    // of them or out of a marker this module wrote; there is no third
    // source that could be invalid UTF-8.
    let text = String::from_utf8(out).unwrap_or_default();
    let clashes = clashes(&text);
    Merged { text, clashes }
}

/// Counts the clash regions in merged text.
///
/// **By counting the opening marker line**, which is what every tool
/// downstream of a merge does, git included. A file whose own content
/// holds a line spelled exactly like khor's opening marker — seven `<`,
/// a space, and khor's word for「我改的」— would be counted as one more
/// than there are; that line cannot occur by accident in source and the
/// alternative is a second parser for our own output.
fn clashes(text: &str) -> u32 {
    let head = format!("{} {}", "<".repeat(text::Conflict::DEFAULT_MARKER_SIZE as usize), msg::MERGE_MINE);
    text.lines().filter(|l| *l == head).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case the whole batch is for: two people, one file, and no
    /// line in common. Neither edit is lost and nobody is asked
    /// anything.
    #[test]
    fn two_edits_that_never_meet_both_survive() {
        let base = "one\ntwo\nthree\nfour\nfive\n";
        let mine = "ONE\ntwo\nthree\nfour\nfive\n";
        let theirs = "one\ntwo\nthree\nfour\nFIVE\n";
        let m = three(base, mine, theirs);
        assert_eq!(m.text, "ONE\ntwo\nthree\nfour\nFIVE\n");
        assert_eq!(m.clashes, 0);
    }

    /// One line, two different rewrites. khor does not pick, and says
    /// how many times it did not.
    #[test]
    fn one_line_two_answers_is_one_clash_and_both_are_kept() {
        let base = "a\nb\nc\n";
        let mine = "a\nMINE\nc\n";
        let theirs = "a\nTHEIRS\nc\n";
        let m = three(base, mine, theirs);
        assert_eq!(m.clashes, 1);
        assert!(m.text.contains("MINE"), "my line is in there: {}", m.text);
        assert!(m.text.contains("THEIRS"), "their line is in there: {}", m.text);
        assert!(m.text.contains("<<<<<<<"), "and it is marked: {}", m.text);
    }

    /// Two clashes are two, not one — the count is regions, not a
    /// boolean wearing a number.
    #[test]
    fn two_disagreements_are_counted_twice() {
        let base = "a\nb\nc\nd\ne\nf\ng\n";
        let mine = "A\nb\nc\nd\ne\nf\nG\n";
        let theirs = "1\nb\nc\nd\ne\nf\n7\n";
        assert_eq!(three(base, mine, theirs).clashes, 2);
    }

    /// Both sides typed the same thing. git calls it resolved and so
    /// does khor: one copy, no markers, nothing to decide.
    #[test]
    fn the_same_edit_on_both_sides_is_not_a_clash() {
        let base = "x\ny\n";
        let both = "x\nY\n";
        let m = three(base, both, both);
        assert_eq!(m.text, both);
        assert_eq!(m.clashes, 0);
    }

    /// I changed nothing; the answer is their file, byte for byte. The
    /// degenerate case matters because it is what a merge pressed by
    /// habit does.
    #[test]
    fn an_untouched_side_gives_back_the_other_one() {
        let base = "k\nl\nm\n";
        let theirs = "k\nL\nm\nn\n";
        let m = three(base, base, theirs);
        assert_eq!(m.text, theirs);
        assert_eq!(m.clashes, 0);
    }

    /// A file without a trailing newline is a real file, and a merge
    /// that quietly added one would be a diff nobody asked for.
    #[test]
    fn a_file_with_no_last_newline_does_not_grow_one() {
        let base = "p\nq";
        let mine = "P\nq";
        let m = three(base, mine, base);
        assert_eq!(m.text, mine);
    }

    /// Empty means empty, not a panic — an empty box is reachable by
    /// selecting everything and pressing delete.
    #[test]
    fn an_emptied_side_is_a_merge_like_any_other() {
        let m = three("a\nb\n", "", "a\nb\n");
        assert_eq!(m.text, "");
        assert_eq!(m.clashes, 0);
    }

    /// The markers say which side is which, in the words the person
    /// just read on the buttons.
    #[test]
    fn the_markers_carry_both_sides_names() {
        let m = three("a\n", "mine\n", "theirs\n");
        assert!(m.text.contains(msg::MERGE_MINE), "mine is named: {}", m.text);
        assert!(m.text.contains(msg::MERGE_THEIRS), "theirs is named: {}", m.text);
    }
}
