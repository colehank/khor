//! Two kinds of test, kept apart on purpose.
//!
//! **Vendor fixtures are transcribed from upstream's own test files**
//! (`kbwo/ccmanager`, `src/services/stateDetector/*.test.ts`, read
//! 2026-08-17) — screens upstream recorded from real agents, with the
//! answer upstream expects. They are not written from ccmanager's
//! parsing code: a fixture derived from the implementation shares the
//! implementation's misreadings and the two then agree forever.
//!
//! **Engine tests are ours and are allowed to be synthetic**, because
//! what they check is this crate's own machinery — the screen model, the
//! debounce clock, the conjunction form. Anything asserting what a
//! vendor's UI looks like belongs in the first group.
//!
//! One difference from upstream worth knowing: upstream hands its
//! detectors a hand-built buffer object, while these push bytes through
//! a real terminal first. That is strictly more of the path — and it is
//! the part this crate exists for, so it is the part that has to be
//! exercised.

use super::*;

/// A screen holding exactly these lines, the way upstream's mock does:
/// one line per row, nothing above, nothing wrapped.
///
/// Wide enough for the longest line on purpose. A narrower screen would
/// wrap it onto a second row, which pushes a fixture's top line off a
/// screen sized to its line count — the fixture would still be "there"
/// and the test would still pass or fail, just not for its own reasons.
fn screen_of(lines: &[&str]) -> Screen {
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let cols = u16::try_from(widest.max(78) + 2).unwrap_or(u16::MAX);
    let rows = u16::try_from(lines.len().max(1)).unwrap_or(u16::MAX);
    let mut screen = Screen::new(rows, cols);
    screen.feed(lines.join("\r\n").as_bytes());
    screen
}

fn detector(vendor: &str) -> Detector {
    Detector::for_vendor(vendor).expect("vendor is in patterns.toml")
}

fn word(vendor: &str, lines: &[&str], current: Word) -> Word {
    detector(vendor).word(&screen_of(lines), current, 0)
}

/// Upstream's `detectStateAfterDebounce`: read once, let the debounce
/// window pass with the screen unchanged, read again.
fn word_after_debounce(vendor: &str, lines: &[&str], current: Word) -> Word {
    let screen = screen_of(lines);
    let mut d = detector(vendor);
    d.word(&screen, current, 0);
    d.word(&screen, current, 1500)
}

// ── the harness has to be alive before any of it counts ─────

/// **The guard on every other test in this file.**
///
/// Seven of the eight vendors answer 空闲 when nothing matches, so a
/// [`screen_of`] that quietly produced an empty screen would leave every
/// idle assertion here passing and testing nothing at all. This asserts
/// the fixture actually reaches the screen before anything reads one.
#[test]
fn a_fixture_actually_lands_on_the_screen() {
    let screen = screen_of(&["first line", "second line", "third line"]);
    let rendered = screen.tail(30);
    assert!(rendered.contains("first line"), "top line missing: {rendered:?}");
    assert!(rendered.contains("third line"), "bottom line missing: {rendered:?}");
    assert_eq!(rendered.lines().count(), 3, "wrong row count: {rendered:?}");
}

/// Every vendor in the table is answered for by at least one fixture
/// below.
///
/// The reverse direction, and the one that rots silently: a vendor added
/// to `patterns.toml` with no fixture ships rules nobody ever ran. The
/// list is spelled out rather than derived so that adding a vendor
/// forces a decision here.
#[test]
fn every_vendor_in_the_table_has_fixtures() {
    let covered = [
        "claude",
        "codex",
        "gemini",
        "cursor",
        "github-copilot",
        "cline",
        "opencode",
        "kimi",
    ];
    let mut listed: Vec<&str> = vendors().collect();
    listed.sort_unstable();
    let mut expected = covered.to_vec();
    expected.sort_unstable();
    assert_eq!(listed, expected, "a vendor in patterns.toml has no fixtures here");
}

#[test]
fn a_vendor_nobody_has_written_a_table_for_has_no_detector() {
    assert!(Detector::for_vendor("no-such-agent").is_none());
}

// ── the screen model's own claims ───────────────────────────

/// **The reason this crate carries a terminal at all.**
///
/// An agent redraws its footer by addressing the cursor, so the words it
/// wrote a moment ago are gone from the screen while remaining in the
/// byte stream forever. Here the bytes say "esc to interrupt" and the
/// screen does not — which is exactly the difference between reading a
/// screen and grepping the host's replay ring, and the whole argument
/// for the dependency.
#[test]
fn text_that_was_overwritten_is_not_on_the_screen_any_more() {
    let mut screen = Screen::new(4, 40);
    let bytes: &[u8] = b"working\r\nesc to interrupt\r\n";
    screen.feed(bytes);
    assert!(screen.tail(30).contains("esc to interrupt"), "precondition: it was there");

    // Up one row, clear it, write over it — what an agent does every
    // frame it repaints its status line.
    screen.feed(b"\x1b[A\r\x1b[2Kall done");

    let rendered = screen.tail(30);
    assert!(!rendered.contains("esc to interrupt"), "stale frame survived: {rendered:?}");
    assert!(rendered.contains("all done"), "the repaint is missing: {rendered:?}");
    assert!(
        String::from_utf8_lossy(bytes).contains("esc to interrupt"),
        "the byte log still holds it — which is why the byte log is not what we match",
    );
}

#[test]
fn blank_rows_below_the_last_line_are_not_part_of_the_screen() {
    let mut screen = Screen::new(10, 40);
    screen.feed(b"one\r\ntwo");
    assert_eq!(screen.tail(30), "one\ntwo");
}

#[test]
fn a_tail_takes_the_last_lines_not_the_first() {
    let screen = screen_of(&["a", "b", "c", "d"]);
    assert_eq!(screen.tail(2), "c\nd");
}

/// The input box is found by its two rules and everything from it down
/// is dropped — including what the user typed into it.
#[test]
fn the_prompt_box_and_everything_in_it_is_out_of_scope() {
    let screen = screen_of(&[
        "real output",
        "──────────────────────────────",
        "esc to interrupt",
        "──────────────────────────────",
    ]);
    let above = screen.above_prompt(30);
    assert!(above.contains("real output"), "lost the output: {above:?}");
    assert!(!above.contains("esc to interrupt"), "read inside the box: {above:?}");
}

/// Only the newest block survives; a blank line ends it.
#[test]
fn only_the_most_recent_block_above_the_box_is_this_turn() {
    let screen = screen_of(&[
        "esc to interrupt",
        "",
        "all finished",
        "──────────────────────────────",
        "❯",
        "──────────────────────────────",
    ]);
    let above = screen.above_prompt(30);
    assert_eq!(above, "all finished");
}

// ── claude ──────────────────────────────────────────────────

#[test]
fn claude_is_busy_while_it_says_how_to_interrupt_it() {
    for marker in ["Press ESC to interrupt", "Googling. (ctrl+c to interrupt"] {
        let lines = [
            "Processing...",
            marker,
            "──────────────────────────────",
            "❯",
            "──────────────────────────────",
        ];
        assert_eq!(word("claude", &lines, Word::Idle), Word::Busy, "{marker}");
    }
}

/// No box on screen, so the whole screen is the block. Upstream keeps
/// this fallback because the box is absent while claude is starting up.
#[test]
fn claude_is_busy_on_an_interrupt_line_with_no_prompt_box_at_all() {
    let lines = ["Running command...", "press esc to interrupt the process"];
    assert_eq!(word("claude", &lines, Word::Idle), Word::Busy);
}

#[test]
fn claude_is_busy_on_a_spinner_with_an_activity_label() {
    for glyph in ["✱", "✳", "✻", "✽", "❀", "❇", "✦", "·", "⏺", "○"] {
        let line = format!("{glyph} Kneading…");
        let lines = [line.as_str(), "❯"];
        assert_eq!(word("claude", &lines, Word::Idle), Word::Busy, "{glyph}");
    }
}

/// The label is half the rule. A spinner glyph on an ordinary sentence
/// is not a turn in flight, and upstream tests for exactly that.
///
/// The positive half runs first and is not decoration: this assertion
/// is satisfied by a screen with nothing on it, so without showing that
/// the very same glyph *does* fire when it carries a label, "not busy"
/// would also be the answer for a detector that had stopped working.
#[test]
fn claude_is_not_busy_on_a_spinner_glyph_without_an_activity_label() {
    let with_label = ["✽ Tempering…", "❯"];
    assert_eq!(word("claude", &with_label, Word::Idle), Word::Busy, "control");

    let without = ["✽ Some random text", "❯"];
    assert_eq!(word_after_debounce("claude", &without, Word::Idle), Word::Idle);
}

#[test]
fn claude_is_busy_on_a_token_stats_line() {
    for stats in ["(9m 21s · ↓ 13.7k tokens)", "  ( 1m · 500 TOKENS )  "] {
        let lines = [stats, "──────────────────────────────", "❯", "──────────────────────────────"];
        assert_eq!(word("claude", &lines, Word::Idle), Word::Busy, "{stats}");
    }
}

/// A digit is required, so prose about tokens is not a stats line.
///
/// The first two lines are a minimal pair — one digit apart — which is
/// what makes the negative mean anything. Writing this control is also
/// what turned up how narrow the rule really is: the word `tokens` has
/// to be the last thing inside the parentheses, so `(see 9 tokens in
/// docs)` is **not** a stats line either. That is upstream's shape and
/// the third case keeps their wording for it.
#[test]
fn claude_is_not_busy_on_parenthetical_prose_that_merely_says_tokens() {
    let boxed = |first: &'static str| {
        [first, "──────────────────────────────", "❯", "──────────────────────────────"]
    };
    assert_eq!(word("claude", &boxed("(see 9 tokens)"), Word::Idle), Word::Busy, "control");
    assert_eq!(
        word_after_debounce("claude", &boxed("(see tokens)"), Word::Idle),
        Word::Idle,
        "the digit is the whole difference",
    );
    assert_eq!(
        word_after_debounce("claude", &boxed("(see tokens in docs)"), Word::Idle),
        Word::Idle,
    );
}

#[test]
fn claude_is_waiting_on_a_question_with_options() {
    let lines = ["Do you want to continue?", "❯ 1. Yes", "  2. No"];
    assert_eq!(word("claude", &lines, Word::Idle), Word::Waiting);
}

/// A permission menu with no question in it. The numbered deny option is
/// the marker that holds across the wordings.
#[test]
fn claude_is_waiting_on_a_permission_menu_with_a_numbered_deny() {
    let lines = [
        "Claude in Chrome wants to navigate on example.com",
        "❯ 1. Allow",
        "  2. Deny (esc)",
    ];
    assert_eq!(word("claude", &lines, Word::Idle), Word::Waiting);
}

/// 待批 is read off the whole screen including the box, unlike 忙碌.
/// The asymmetry is upstream's and it is deliberate: a question drawn
/// inside the box is still a question.
#[test]
fn claude_reads_a_question_inside_the_box_but_not_an_interrupt_inside_it() {
    let box_lines = |middle: &'static str| {
        ["Some idle output", "──────────────────────────────", middle, "──────────────────────────────"]
    };
    assert_eq!(word("claude", &box_lines("esc to cancel"), Word::Idle), Word::Waiting);
    assert_eq!(
        word_after_debounce("claude", &box_lines("esc to interrupt"), Word::Idle),
        Word::Idle,
    );
    assert_eq!(
        word_after_debounce("claude", &box_lines("✽ Tempering…"), Word::Idle),
        Word::Idle,
    );
}

/// Both of these are screens where a turn has finished but its evidence
/// is still further up the scrollback. Reading the whole screen would
/// call them busy forever.
///
/// **What makes this a real test is the control**: the same marker, in
/// the same table, on the same kind of screen, but as the newest block
/// — it says 忙碌. So the difference the two halves measure is position
/// and nothing else. Without it "idle" is also what a detector that had
/// stopped recognising the marker at all would answer, and this is the
/// test most likely to be read as proving the geometry works.
#[test]
fn claude_ignores_a_finished_turns_leftovers_further_up_the_screen() {
    let fresh = [
        "Command completed successfully",
        "",
        "Press esc to interrupt",
        "──────────────────────────────",
        "❯",
        "──────────────────────────────",
    ];
    assert_eq!(word("claude", &fresh, Word::Idle), Word::Busy, "control: newest block counts");

    let stale_spinner = [
        "✻ Seasoning… (44s · ↓ 247 tokens)",
        "  ⎿ Tip: Use /btw to ask a quick side question",
        "",
        "⏺ 全て通過。",
        "",
        "  - lint: pass (0 errors)",
        "  - typecheck: pass",
        "  - tests: 56 files, 775 passed, 5 skipped",
        "──────────────────────────────",
        "❯",
        "──────────────────────────────",
    ];
    assert_eq!(word_after_debounce("claude", &stale_spinner, Word::Busy), Word::Idle);

    let stale_interrupt = [
        "Press esc to interrupt",
        "Working...",
        "",
        "Command completed successfully",
        "Ready for next command",
        "──────────────────────────────",
        "❯",
        "──────────────────────────────",
    ];
    assert_eq!(word_after_debounce("claude", &stale_interrupt, Word::Busy), Word::Idle);
}

/// The search overlay hides the session, so the screen stops being
/// evidence and the rule declines to answer rather than reporting the
/// overlay. Note it beats a spinner that is still on screen — which is
/// what the control establishes: the same spinner without the overlay
/// really does say 忙碌, so the overlay is doing the work here.
#[test]
fn claudes_search_overlay_makes_it_stop_reading_the_screen() {
    assert_eq!(word("claude", &["✽ Tempering…"], Word::Idle), Word::Busy, "control");

    let overlaid = ["⌕ Search…", "✽ Tempering…"];
    assert_eq!(word_after_debounce("claude", &overlaid, Word::Idle), Word::Idle);
}

#[test]
fn claude_keeps_whatever_word_it_had_while_the_history_hint_is_up() {
    for hint in [
        "Press Ctrl+R to toggle history search",
        "CTRL+R TO TOGGLE",
        "Press ctrl+r to toggle the search",
    ] {
        let lines = ["Some output", hint];
        for held in [Word::Idle, Word::Busy, Word::Waiting] {
            assert_eq!(word("claude", &lines, held), held, "{hint}");
        }
    }
}

// ── the debounce ────────────────────────────────────────────

#[test]
fn claude_does_not_go_idle_the_instant_a_screen_stops_moving() {
    let screen = screen_of(&["Command completed successfully", "> "]);
    let mut d = detector("claude");
    assert_eq!(d.word(&screen, Word::Busy, 0), Word::Busy);
    assert_eq!(d.word(&screen, Word::Busy, 1499), Word::Busy, "one tick early");
    assert_eq!(d.word(&screen, Word::Busy, 1500), Word::Idle, "the window is 1500");
}

/// The clock restarts whenever the screen changes, so a session that
/// keeps printing never settles.
#[test]
fn a_screen_that_keeps_changing_restarts_claudes_idle_clock() {
    let mut d = detector("claude");
    let first = screen_of(&["Output v1", "> "]);
    d.word(&first, Word::Busy, 0);

    let second = screen_of(&["Output v2", "> "]);
    assert_eq!(d.word(&second, Word::Busy, 1400), Word::Busy, "changed at 1400");
    assert_eq!(d.word(&second, Word::Busy, 1600), Word::Busy, "old deadline must not count");
    assert_eq!(d.word(&second, Word::Busy, 2900), Word::Idle, "1400 + 1500");
}

/// Only idle waits. A turn starting or a question appearing is reported
/// the moment it shows up — the delay exists to stop 空闲 flickering,
/// and applying it to 待批 would sit on the one word the user is
/// waiting for.
#[test]
fn the_debounce_holds_up_idle_only() {
    let busy = ["Processing...", "Press ESC to interrupt", "──────────────────────────────", "❯", "──────────────────────────────"];
    assert_eq!(word("claude", &busy, Word::Idle), Word::Busy);
    let asking = ["Do you want to continue?", "❯ 1. Yes", "  2. No"];
    assert_eq!(word("claude", &asking, Word::Idle), Word::Waiting);
}

/// A vendor with no window in the table answers at once.
#[test]
fn a_vendor_without_a_debounce_says_idle_on_the_first_look() {
    let lines = ["Welcome to Gemini CLI", "Type your message below"];
    assert_eq!(word("gemini", &lines, Word::Busy), Word::Idle);
}

// ── codex ───────────────────────────────────────────────────

#[test]
fn codex_is_waiting_on_each_of_its_confirmation_wordings() {
    for lines in [
        vec!["Some output", "Allow command?", "│ > "],
        vec!["Some output", "Continue? [y/n]", "> "],
        vec!["Some output", "Apply changes? yes (y) / no (n)"],
        vec!["Some output", "Press enter to confirm or esc to cancel"],
    ] {
        assert_eq!(word("codex", &lines, Word::Idle), Word::Waiting, "{lines:?}");
    }
}

#[test]
fn codex_is_waiting_on_a_numbered_command_approval() {
    let lines = [
        "Would you like to run the following command?",
        "",
        "Reason: Need to write to .git/worktrees metadata to stage changes for the requested commi",
        "",
        "$ git add test.ts",
        "",
        "› 1. Yes, proceed (y)",
        "  2. Yes, and don't ask again for this command (a)",
        "  3. No, and tell Codex what to do differently (esc)",
        "",
        "Press enter to confirm or esc to cancel",
    ];
    assert_eq!(word("codex", &lines, Word::Idle), Word::Waiting);
}

/// A screen can hold both markers at once, and the question wins. The
/// order in the table is the whole of that rule.
#[test]
fn codex_reads_a_pending_question_over_a_running_turn() {
    let lines = ["esc to interrupt", "Press enter to confirm or esc to cancel"];
    assert_eq!(word("codex", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn codex_is_busy_while_it_offers_a_way_to_interrupt() {
    let lines = ["Thinking...", "esc to interrupt"];
    assert_eq!(word("codex", &lines, Word::Idle), Word::Busy);
}

// ── gemini ──────────────────────────────────────────────────

#[test]
fn gemini_is_waiting_on_its_boxed_prompts_with_or_without_the_question_mark() {
    for prompt in [
        "│ Apply this change?",
        "│ Apply this change",
        "│ Allow execution?",
        "│ Allow execution",
        "│ Do you want to proceed?",
        "│ Do you want to proceed",
    ] {
        let lines = ["Some output from Gemini", prompt, "│ > "];
        assert_eq!(word("gemini", &lines, Word::Idle), Word::Waiting, "{prompt}");
    }
}

#[test]
fn gemini_is_waiting_when_it_says_so_outright() {
    let lines = ["Processing...", "Waiting for user confirmation..."];
    assert_eq!(word("gemini", &lines, Word::Idle), Word::Waiting);
}

/// **The same seven words mean the opposite thing one vendor over.**
/// 'esc to cancel' is 待批 for claude and 忙碌 here, which is why this
/// file is per-vendor tables and not one shared list of clever strings.
#[test]
fn esc_to_cancel_is_busy_for_gemini_and_waiting_for_claude() {
    let lines = ["Processing your request...", "Press ESC to cancel"];
    assert_eq!(word("gemini", &lines, Word::Idle), Word::Busy);
    assert_eq!(word("claude", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn gemini_reads_a_confirmation_over_a_running_turn() {
    let lines = ["Press ESC to cancel", "│ Apply this change?", "│ > "];
    assert_eq!(word("gemini", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn gemini_is_idle_on_its_own_welcome_screen() {
    let lines = ["Welcome to Gemini CLI", "Type your message below"];
    assert_eq!(word("gemini", &lines, Word::Idle), Word::Idle);
}

// ── cursor ──────────────────────────────────────────────────

#[test]
fn cursor_is_waiting_on_its_option_labels_whatever_their_case() {
    for lines in [
        vec!["Some output", "Apply changes? (y) (enter)", "> "],
        vec!["Some output", "Continue? (Y) (ENTER)", "> "],
        vec!["Changes detected", "Keep (n) or replace?", "> "],
        vec!["Some output", "KEEP (N) current version?", "> "],
        vec!["Some output", "Auto apply changes (shift+tab)", "> "],
        vec!["Some output", "AUTO COMPLETE (SHIFT+TAB)", "> "],
        vec!["Some prompt", "   Skip (esc or n)"],
    ] {
        assert_eq!(word("cursor", &lines, Word::Idle), Word::Waiting, "{lines:?}");
    }
}

#[test]
fn cursor_is_waiting_on_a_command_approval_menu() {
    let lines = [
        "Run this command?",
        "Not in allowlist: cd /some/path, npm run test",
        " → Run (once) (y)",
        "   Run Everything (shift+tab)",
        "   Skip (esc or n)",
    ];
    assert_eq!(word("cursor", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn cursor_is_busy_on_a_spinner_with_either_kind_of_ellipsis() {
    for line in ["  ⬡ Grepping..", "  ⬢ Reading...", "⬡ Searching…"] {
        assert_eq!(word("cursor", &[line], Word::Idle), Word::Busy, "{line}");
    }
}

#[test]
fn cursor_is_busy_while_it_offers_a_way_to_stop() {
    for line in ["Press ctrl+c to stop", "PRESS CTRL+C TO STOP"] {
        let lines = ["Processing...", line, "Working..."];
        assert_eq!(word("cursor", &lines, Word::Idle), Word::Busy, "{line}");
    }
}

#[test]
fn cursor_reads_a_pending_option_over_a_running_turn() {
    let lines = ["ctrl+c to stop", "(y) (enter)"];
    assert_eq!(word("cursor", &lines, Word::Idle), Word::Waiting);
}

// ── github-copilot ──────────────────────────────────────────

#[test]
fn copilot_is_waiting_on_a_confirm_line_however_long_the_key_name() {
    for line in ["Confirm with Y Enter", "Confirm with Shift + Y Enter"] {
        let lines = ["Some output", line];
        assert_eq!(word("github-copilot", &lines, Word::Idle), Word::Waiting, "{line}");
    }
}

#[test]
fn copilot_is_waiting_on_a_boxed_question_in_any_case() {
    let lines = ["Running GitHub Copilot CLI...", "│ DO YOU WANT to run this command?", "│ > "];
    assert_eq!(word("github-copilot", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn copilot_is_busy_while_it_offers_a_way_to_cancel() {
    let lines = ["Executing request...", "Press Esc to cancel"];
    assert_eq!(word("github-copilot", &lines, Word::Idle), Word::Busy);
}

#[test]
fn copilot_reads_a_confirmation_over_a_running_turn() {
    let lines = ["Press Esc to cancel", "Confirm with Y Enter"];
    assert_eq!(word("github-copilot", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn copilot_is_idle_at_its_own_ready_prompt() {
    let lines = ["GitHub Copilot CLI ready.", "Type a command to begin."];
    assert_eq!(word("github-copilot", &lines, Word::Idle), Word::Idle);
}

// ── cline ───────────────────────────────────────────────────

#[test]
fn cline_is_waiting_when_it_asks_to_use_a_tool() {
    for lines in [
        vec![
            "┃ [act mode] Let Cline use this tool?",
            "┃ >  Yes",
            "┃   Yes, and don't ask again for this task",
            "┃   No, with feedback",
        ],
        vec!["Some output", "LET CLINE USE THIS TOOL?", ">  Yes"],
    ] {
        assert_eq!(word("cline", &lines, Word::Idle), Word::Waiting, "{lines:?}");
    }
}

#[test]
fn cline_is_idle_only_on_its_ready_line() {
    for lines in [
        vec!["┃ [act mode] Cline is ready for your message...", "┃ /plan or /act to switch modes"],
        vec!["┃ [plan mode] Cline is ready for your message...", "┃ ctrl+e to open editor"],
        vec!["Some output", "CLINE IS READY FOR YOUR MESSAGE", "Ready to go"],
    ] {
        assert_eq!(word("cline", &lines, Word::Idle), Word::Idle, "{lines:?}");
    }
}

/// **The one vendor whose default is busy.** Anything that is not the
/// ready line reads as a turn in flight, including an empty screen —
/// upstream's choice, transcribed rather than improved.
#[test]
fn cline_treats_anything_it_does_not_recognise_as_a_running_turn() {
    let working = ["Processing your request...", "Running analysis...", "Working on it..."];
    assert_eq!(word("cline", &working, Word::Idle), Word::Busy);
    assert_eq!(word("cline", &[], Word::Idle), Word::Busy, "an empty screen too");
}

#[test]
fn cline_reads_a_pending_question_over_its_own_ready_line() {
    let lines = [
        "┃ [act mode] Cline is ready for your message...",
        "┃ Let Cline use this tool?",
        "┃ >  Yes",
    ];
    assert_eq!(word("cline", &lines, Word::Idle), Word::Waiting);
}

// ── opencode ────────────────────────────────────────────────

#[test]
fn opencode_is_waiting_on_its_permission_banner() {
    let lines = [
        "opencode v0.1.0",
        "",
        "△ Permission required",
        "The AI wants to execute a shell command",
        "",
        "Press Enter to allow, Esc to deny",
    ];
    assert_eq!(word("opencode", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn opencode_is_busy_on_an_interrupt_line_in_any_case() {
    for line in ["Press esc to interrupt", "PRESS ESC TO INTERRUPT", "Esc to interrupt"] {
        let lines = ["Processing...", line];
        assert_eq!(word("opencode", &lines, Word::Idle), Word::Busy, "{line}");
    }
}

#[test]
fn opencode_reads_a_permission_banner_over_a_running_turn() {
    let lines = ["esc to interrupt", "△ Permission required"];
    assert_eq!(word("opencode", &lines, Word::Idle), Word::Waiting);
}

#[test]
fn opencode_is_idle_when_nothing_matches() {
    let lines = ["Normal output", "Some message", "Ready"];
    assert_eq!(word("opencode", &lines, Word::Idle), Word::Idle);
}

// ── kimi ────────────────────────────────────────────────────

#[test]
fn kimi_is_waiting_on_its_approval_words() {
    for line in ["Allow?", "Confirm?", "Approve?", "Proceed?", "Continue [y/n]", "Continue (y/n)"] {
        let lines = ["Some output", line];
        assert_eq!(word("kimi", &lines, Word::Idle), Word::Waiting, "{line}");
    }
}

#[test]
fn kimi_is_busy_on_its_progress_words() {
    for line in ["Thinking", "Processing", "Generating", "Waiting for response", "Press ctrl+c to cancel"] {
        let lines = ["Some output", line];
        assert_eq!(word("kimi", &lines, Word::Idle), Word::Busy, "{line}");
    }
}

/// **The failure mode kimi's block in `patterns.toml` names, pinned
/// here as a fact rather than a worry.** Its busy rules are bare English
/// words matched anywhere on screen, so an agent that writes a sentence
/// containing "thinking" reports itself busy by saying so. Transcribed
/// as upstream wrote it; this test is what makes the cost visible to
/// whoever gets a real kimi session and comes to fix it.
#[test]
fn kimi_calls_itself_busy_for_merely_writing_the_word_thinking() {
    let lines = ["I've finished. My thinking here was that the cache was cold.", "> "];
    assert_eq!(
        word("kimi", &lines, Word::Idle),
        Word::Busy,
        "if this ever stops being true, kimi's table got better and its note should say so",
    );
}

// ── engine, not vendors ─────────────────────────────────────

/// Both halves are required. Upstream has exactly one rule of this shape
/// and no fixture that reaches it — every fixture containing the second
/// half also trips an earlier rule — so the mechanism is checked here
/// with a screen of our own rather than left unexercised.
#[test]
fn a_conjunction_rule_needs_every_one_of_its_parts() {
    let both = ["   Add Write(/tmp/x.ts) to allowlist? (tab)"];
    assert_eq!(word("cursor", &both, Word::Idle), Word::Waiting);

    // The same rule's other half alone, on a screen carrying no other
    // cursor marker at all.
    let half = ["   the allowlist is configured elsewhere"];
    assert_eq!(word("cursor", &half, Word::Idle), Word::Idle);
}

/// Case folding is per rule, not per vendor: gemini reads its boxed
/// prompts exactly and its interrupt line folded, in one table.
#[test]
fn a_case_sensitive_rule_does_not_fire_on_the_wrong_case() {
    let shouting = ["Some output from Gemini", "│ APPLY THIS CHANGE?", "│ > "];
    assert_eq!(
        word("gemini", &shouting, Word::Idle),
        Word::Idle,
        "the boxed prompts are matched exactly, so upper case is not that prompt",
    );
    let folded = ["Running command...", "press ESC TO CANCEL now"];
    assert_eq!(word("gemini", &folded, Word::Idle), Word::Busy, "and this one folds");
}
