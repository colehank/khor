//! The PTY fallback: reading an agent's state off the screen it draws.
//!
//! **This is the bottom of the four-family order** (docs/handoff/001.md
//! 状态源四族: vendor files > hooks > OTEL > this). It is the family that
//! guesses, and everything about it is arranged so that the guess loses
//! to anything better and is cheap to be wrong about.
//!
//! # Why it exists at all
//!
//! A session khor hosts itself gets its word from `host.rs`, and today
//! that only happens for a shell — a `khor open --tui -- <agent>` whose
//! vendor has no hook installed **wears 忙碌 from the moment it opens
//! until it exits**. Measured 2026-08-17: the same `/bin/sh` reads 空闲
//! opened as a shell and 忙碌 opened as a tui. That is not a missing
//! word landing on its neighbour, it is a word that is permanently
//! wrong, and a signal that never changes is not a signal (docs/UX.md
//! 角标). This crate is what fills it.
//!
//! # Why it needs a screen and not the byte log
//!
//! The host already keeps every byte the child wrote, and matching
//! patterns against that would be **wrong in kind, not in degree**: the
//! rules describe what is on a screen *now*, and a byte log has no now.
//! "esc to interrupt" from a turn that ended an hour ago is still in the
//! log, so a session that ran once would read 忙碌 forever — trading
//! today's permanently-wrong word for a different permanently-wrong
//! word. Reading only the tail does not rescue it, because a
//! cursor-addressed redraw does not put the bottom of the screen at the
//! end of the stream. So bytes go through [`Screen`], which is a real
//! terminal, and the rules read the viewport it renders — the same
//! thing upstream reads, and upstream *still* needs a pass to drop
//! redraw fragments (see [`Screen::above_prompt`]).
//!
//! # The ceiling is structural
//!
//! [`Word`] has three variants and cannot be made to say 完成, 中断 or
//! 失败. That is not an omission to be filled in later: those three are
//! **endings**, and an ending is recorded by whoever was holding the
//! process when it ended (`khor_node::live::face`), not inferred from
//! pixels. A screen cannot tell "finished" from "idle", which is the
//! same limit claude's own status file has. Keeping the ceiling in the
//! type means a future table cannot quietly raise it — a rule saying
//! `then = "done"` fails the build, rather than putting a word on screen
//! that nothing behind it can support.
//!
//! # What is data and what is code
//!
//! `patterns.toml` holds the vendors, their rules and the glyph sets:
//! everything that changes when an agent reskins its UI, which upstream
//! does roughly monthly. This file holds the engine and the named
//! scopes, because a scope is screen geometry and expressing it in the
//! table would mean inventing a small language inside TOML. That split
//! is what a signed table update would later replace: the file, not the
//! crate.
//!
//! Knowledge transcribed from `kbwo/ccmanager` (MIT), read remotely
//! 2026-08-17. **Never run against a real session of any of these eight
//! vendors** — every rule here is upstream's word for it, and each
//! vendor block in `patterns.toml` names the direction it errs in.

use regex::Regex;

include!(concat!(env!("OUT_DIR"), "/patterns.rs"));

/// What a screen can say about an agent.
///
/// Three, and no way to add a fourth without a reason that survives the
/// module head. These map onto khor's six at the call site
/// (`khor_node`), which is also where the other three come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    /// A turn is running.
    Busy,
    /// An action is stuck behind the user's approval — khor's 待批.
    Waiting,
    /// Nothing is running. **Not "finished"**: a screen cannot tell the
    /// two apart, so this is the honest, weaker one of the pair.
    Idle,
}

/// Which slice of the screen a rule reads.
///
/// Kept in code rather than in the table: these are geometry, and there
/// are two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// The last N non-blank-padded lines of the viewport.
    Tail(usize),
    /// The most recent block above the input box. See
    /// [`Screen::above_prompt`].
    AbovePrompt(usize),
}

pub(crate) enum TestSpec {
    Contains { pattern: &'static str, fold: bool },
    /// Every one of them, anywhere. One rule in the whole table needs
    /// this; upstream requires both halves because either alone shows up
    /// in ordinary output.
    ContainsAll { patterns: &'static [&'static str], fold: bool },
    Regex { pattern: &'static str, fold: bool },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Outcome {
    Busy,
    Waiting,
    Idle,
    /// Answer with whatever the caller already had. Upstream uses this
    /// where an overlay hides the session's real state — the screen
    /// stops being evidence, so the rule declines to testify rather than
    /// reporting the overlay.
    Keep,
}

pub(crate) struct RuleSpec {
    pub scope: Scope,
    pub test: TestSpec,
    pub then: Outcome,
}

pub(crate) struct VendorTable {
    pub name: &'static str,
    /// What this vendor is when no rule fires. Idle for seven of the
    /// eight; cline is busy, because its idle line is unmistakable and
    /// its working output is not.
    pub default: Word,
    /// Only claude has one. See [`Detector::settle_idle`].
    pub idle_debounce_ms: Option<i64>,
    pub rules: &'static [RuleSpec],
}

/// How much of the screen has to hold still for the debounce.
///
/// Upstream watches the same 30 lines its rules read, not the whole
/// viewport: a clock or a progress counter further up would otherwise
/// keep the screen "changing" forever and the debounce would never
/// settle.
const DEBOUNCE_LINES: usize = 30;

// ── the screen ──────────────────────────────────────────────

/// A terminal, in memory, fed the bytes the agent writes.
///
/// The scrollback is zero on purpose. Every rule in the table describes
/// the visible screen, and keeping history would put text back within
/// reach of a match long after it left the screen — which is the exact
/// failure that ruled out matching the host's byte ring.
pub struct Screen {
    parser: vt100::Parser,
}

impl Screen {
    pub fn new(rows: u16, cols: u16) -> Screen {
        Screen { parser: vt100::Parser::new(rows, cols, 0) }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        // vt100 has corner-case panics on real streams (seen live, 2026-08-18:
        // a wide character at the last column, vt100 screen.rs:870 unwrap on
        // the neighbour cell — hit within seconds of attaching a real tmux
        // with a CJK prompt; the exact byte recipe did not reproduce in
        // isolation). One bad frame must not kill the process reading it:
        // catch the panic, replace the parser at the same size, and the
        // program's next full repaint restores the picture.
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let (rows, cols) = self.parser.screen().size();
        if catch_unwind(AssertUnwindSafe(|| self.parser.process(bytes))).is_err() {
            self.parser = vt100::Parser::new(rows, cols, 0);
        }
    }

    /// Follow the PTY's size.
    ///
    /// Not optional bookkeeping: an agent draws its box to the width it
    /// was told, so a screen of the wrong size wraps that box and the
    /// border lines [`Screen::above_prompt`] looks for stop being whole
    /// lines.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(rows, cols);
    }

    /// The last line with anything on it — a hosted row's preview
    /// (khor_core::Session::last). `visible` already drops trailing
    /// blanks, so its last element is that line.
    pub fn last_line(&self) -> Option<String> {
        self.visible().pop()
    }

    /// The visible rows, right-trimmed, with trailing blank rows
    /// dropped.
    fn visible(&self) -> Vec<String> {
        let (_, cols) = self.parser.screen().size();
        let mut lines: Vec<String> = self.parser.screen().rows(0, cols).collect();
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        lines
    }

    /// The last `max_lines` lines of the screen, joined.
    fn tail(&self, max_lines: usize) -> String {
        let mut lines = self.visible();
        if lines.len() > max_lines {
            lines.drain(..lines.len() - max_lines);
        }
        lines.join("\n")
    }

    /// The most recent block of output above the agent's input box.
    ///
    /// **The narrowest scope in the table, and the reason it exists is
    /// that the wider one is unsafe.** The input box echoes what the
    /// user typed: read it, and a person who types the words "esc to
    /// interrupt" into their own prompt puts their session into 忙碌.
    /// So the box is found (its two `─` borders, from the bottom) and
    /// everything from it down is discarded.
    ///
    /// Then the *recent* part: claude redraws its lower pane by moving
    /// the cursor, and a terminal keeps fragments of earlier frames
    /// above the current one. Only the last contiguous block — back to
    /// the nearest blank or rule line — is this turn's. Upstream needs
    /// this pass against a real terminal's buffer, which is the sharpest
    /// evidence available that matching the raw byte stream was never
    /// going to work.
    fn above_prompt(&self, max_lines: usize) -> String {
        let all: Vec<String> = self.tail(max_lines).split('\n').map(str::to_owned).collect();

        let mut lines = all.clone();
        let mut borders = 0;
        for i in (0..all.len()).rev() {
            let t = all[i].trim();
            if !t.is_empty() && t.chars().all(|c| c == '─') {
                borders += 1;
                if borders == 2 {
                    lines = all[..i].to_vec();
                    break;
                }
            }
        }

        while lines.last().is_some_and(|l| {
            let t = l.trim();
            t.is_empty() || t == "❯" || is_rule(t)
        }) {
            lines.pop();
        }
        if lines.is_empty() {
            return String::new();
        }

        let mut start = lines.len() as isize - 1;
        while start >= 0 {
            let t = lines[start as usize].trim();
            if t.is_empty() || is_rule(t) {
                start += 1;
                break;
            }
            start -= 1;
        }
        lines[start.max(0) as usize..].join("\n")
    }
}

/// A separator line: dashes, box-drawing horizontals, blanks.
fn is_rule(trimmed: &str) -> bool {
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '-' || c == '─' || c.is_whitespace())
}

// ── the engine ──────────────────────────────────────────────

/// One vendor's table, ready to answer.
///
/// Holds state between calls because one rule in the table needs it:
/// claude's idle is debounced, and a debounce is a memory of what the
/// screen looked like last time.
pub struct Detector {
    table: &'static VendorTable,
    /// Compiled once, aligned with `table.rules`; `None` where the rule
    /// is not a regex.
    patterns: Vec<Option<Regex>>,
    last_content: String,
    stable_since_ms: i64,
}

/// Every vendor this table can speak for.
pub fn vendors() -> impl Iterator<Item = &'static str> {
    VENDORS.iter().map(|v| v.name)
}

impl Detector {
    /// The detector for a vendor, if the table has one that works.
    ///
    /// `None` covers two cases that must stay indistinguishable to the
    /// caller: khor does not know this vendor, and the table for it is
    /// broken. **A broken table takes the whole vendor down rather than
    /// running the rules that still compile** — a claude table with its
    /// busy rules missing does not degrade, it reports 空闲 through an
    /// entire turn. No detector means no word, which leaves the session
    /// exactly where it would have been without this crate; a partial
    /// one means a confident wrong word, and this family is last in the
    /// order precisely because its wrong words must be cheap.
    ///
    /// In practice `build.rs` has already refused to ship a pattern that
    /// does not compile, so the broken half is unreachable from here.
    pub fn for_vendor(name: &str) -> Option<Detector> {
        let table = VENDORS.iter().find(|v| v.name == name)?;
        let mut patterns = Vec::with_capacity(table.rules.len());
        for rule in table.rules {
            match &rule.test {
                TestSpec::Regex { pattern, .. } => patterns.push(Some(Regex::new(pattern).ok()?)),
                _ => patterns.push(None),
            }
        }
        Some(Detector { table, patterns, last_content: String::new(), stable_since_ms: 0 })
    }

    pub fn vendor(&self) -> &'static str {
        self.table.name
    }

    /// What this screen says the agent is doing.
    ///
    /// `current` is what the caller has on record. It is an input rather
    /// than a detail because two of the outcomes need it: a rule that
    /// declines to answer, and an idle that has not held still long
    /// enough yet. Both give it straight back.
    pub fn word(&mut self, screen: &Screen, current: Word, now_ms: i64) -> Word {
        let mut cache: Vec<(Scope, String)> = Vec::new();
        let mut hit: Option<Outcome> = None;

        for (i, rule) in self.table.rules.iter().enumerate() {
            let text = match cache.iter().position(|(s, _)| *s == rule.scope) {
                Some(at) => &cache[at].1,
                None => {
                    let rendered = match rule.scope {
                        Scope::Tail(n) => screen.tail(n),
                        Scope::AbovePrompt(n) => screen.above_prompt(n),
                    };
                    cache.push((rule.scope, rendered));
                    &cache[cache.len() - 1].1
                }
            };
            if fires(&rule.test, self.patterns[i].as_ref(), text) {
                hit = Some(rule.then);
                break;
            }
        }

        match hit {
            Some(Outcome::Busy) => Word::Busy,
            Some(Outcome::Waiting) => Word::Waiting,
            Some(Outcome::Keep) => current,
            Some(Outcome::Idle) => self.settle_idle(screen, current, now_ms),
            None => match self.table.default {
                Word::Idle => self.settle_idle(screen, current, now_ms),
                other => other,
            },
        }
    }

    /// Idle, but only once the screen has stopped moving.
    ///
    /// Upstream's note is worth keeping: claude can *look* finished
    /// while a turn is still running, mid-redraw. Without this, a
    /// session would flicker to 空闲 and back several times a turn — and
    /// a word that flickers is worse than one that lags, because the
    /// list sorts on it.
    ///
    /// A vendor with no window in the table answers immediately; the
    /// debounce is one vendor's workaround, not a house style.
    fn settle_idle(&mut self, screen: &Screen, current: Word, now_ms: i64) -> Word {
        let Some(window) = self.table.idle_debounce_ms else {
            return Word::Idle;
        };
        let content = screen.tail(DEBOUNCE_LINES);
        if content != self.last_content {
            self.last_content = content;
            self.stable_since_ms = now_ms;
        }
        if now_ms.saturating_sub(self.stable_since_ms) >= window {
            Word::Idle
        } else {
            current
        }
    }
}

fn fires(test: &TestSpec, compiled: Option<&Regex>, text: &str) -> bool {
    match test {
        TestSpec::Contains { pattern, fold } => folded(text, *fold).contains(*pattern),
        TestSpec::ContainsAll { patterns, fold } => {
            let hay = folded(text, *fold);
            patterns.iter().all(|p| hay.contains(*p))
        }
        TestSpec::Regex { fold, .. } => {
            compiled.is_some_and(|re| re.is_match(&folded(text, *fold)))
        }
    }
}

fn folded(text: &str, fold: bool) -> std::borrow::Cow<'_, str> {
    if fold {
        std::borrow::Cow::Owned(text.to_lowercase())
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests;
