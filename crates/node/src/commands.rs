//! **The command ledger** (roadmap 1.2): what the shell in a hosted
//! terminal ran, where, and how it ended — read by the host off the same
//! bytes it already reads for the ring, from the marks the shell
//! integration writes (`crate::integration`).
//!
//! # The marks
//!
//! FinalTerm's, which iTerm2, Kitty, WezTerm, Ghostty, Windows Terminal and
//! VS Code all speak (iTerm2's "Shell Integration" escape codes; Kitty's
//! shell-integration docs; VS Code's `terminal/shellIntegration`):
//!
//! - `OSC 133 ; A` a prompt starts, `B` the input starts, `C` the command
//!   runs, `D [; status]` it finished. `C` may carry the command line
//!   itself as Kitty's `cmdline=` (escaped) or `cmdline_url=` (fish 4).
//! - `OSC 633 ; E ; line` VS Code's explicit command line (`\\` for a
//!   backslash, `\xNN` for `;` and control characters); also `633 ; P ;
//!   Cwd=…`. Without either, the text typed between `B` and `C` is kept,
//!   with its escape sequences dropped.
//! - `OSC 7 ; file://host/path` the directory; `OSC 0 / 2` the title.
//! - tmux's passthrough (`ESC P tmux; … ESC \`, inner `ESC` doubled) is
//!   opened, for a shell that wrapped its marks with no tmux in between.
//!
//! # Through a tmux client
//!
//! A host whose PTY runs khor's tmux client ([`Ledger::tmux`]) reads the
//! marks of every pane tmux shows, interleaved: tmux forwards a pane's
//! passthrough only while the pane is visible (`allow-passthrough on`,
//! tmux(1)), and every pane of a split window is. tmux takes the wrap off
//! and writes what was inside in one piece, so each mark from khor's
//! scripts arrives as `OSC 633 ; P ; KhorPane=%n` (the pane's
//! `$TMUX_PANE`) followed at once by the mark itself, which is read in
//! that pane's context; each pane keeps its own prompt, running command
//! and directory. Which pane
//! is in front comes from the title: khor's grouped session sets
//! `set-titles-string` to `#{pane_id} #{pane_title}`
//! (`Tmux::grouped_attach_argv`), and tmux rewrites the title whenever
//! the active pane or window changes. The row reads the pane in front.
//! Text typed at a prompt is never read here: the bytes between marks
//! are tmux's redraw of the whole screen.
//!
//! # A tmux typed in a shell
//!
//! A shell's own PTY ([`Ledger::default`]) carries the same wrapped marks
//! once the person types `tmux` in it and the panes' shells load khor's
//! integration (the `~/.zshrc` line Settings shows). From the first
//! `KhorPane` mark on, each pane's marks are read in that pane's context
//! as above, and every mark not led by one is the shell's own, read in
//! its own context: the shell's `tmux` runs until tmux exits. Which pane
//! is in front is not read here (the person's titles are their own); the
//! node asks tmux.
//!
//! Marks can arrive twice (two integrations, or a first prompt marked by
//! both the deferred install and its hook): a second `A` with nothing in
//! between, or a `C` while one is running, changes nothing.
//!
//! # What a finished command said
//!
//! The terminal row's second line once a command has finished (the owner
//! 2026-10-08: a finished command still showing its command line reads as
//! still running) is the last non-empty line of that command's own
//! output: the text between its `C` and its `D` — the same bytes the ring
//! holds between the command's `at` and `end_at` — read here as they pass,
//! so a ring trimmed past a long command's start still answers. Escape
//! sequences are dropped; a line rewritten with `\r` (a progress bar) is
//! its last rewrite; a backspace takes a character back; the line is cut
//! to [`LAST_LINE_MAX`] characters. Nothing for a command that held the
//! screen (a full-screen program draws, it does not print lines), nor
//! through a tmux client, whose bytes between marks are tmux's redraw.
//!
//! # What is kept
//!
//! The running command and the last [`RECENT`] finished ones, in this
//! process (so a face that re-attaches finds them unchanged) and in
//! `commands.json` beside the session (so the node can read them for the
//! row without asking the host), plus every finished command appended to
//! the machine's `history.jsonl` (roadmap 2.6), trimmed to its newest half
//! past [`HISTORY_MAX`]. A full-screen program (the alternate screen, the
//! mouse taken) is read off the same bytes (`crate::modes`).

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use khor_core::Doing;
use serde::{Deserialize, Serialize};

use crate::modes::Modes;

/// The longest last line a finished command keeps, in characters.
pub const LAST_LINE_MAX: usize = 200;
/// The most bytes of one output line the ledger holds while reading it.
const TAIL_LINE_MAX: usize = 4 * 1024;

/// How many finished commands a session keeps.
pub const RECENT: usize = 50;
/// How long a string mark may be before it is dropped unread.
const MARK_MAX: usize = 8 * 1024;

/// The least time between two bells the ledger records: a stream of BELs
/// (a held tab completion, `yes $'\a'`) is one bell a second, not a
/// rewrite of the row each.
const BELL_EVERY_MS: u64 = 1000;
/// How much typed text between `B` and `C` is kept for a command line
/// when the shell names none.
const TYPED_MAX: usize = 4 * 1024;
/// The machine's history file is cut to its newest half past this.
pub const HISTORY_MAX: u64 = 4 << 20;

/// One command, as the ledger knows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// Counts from 1 in this session.
    pub n: u64,
    pub text: Option<String>,
    pub cwd: Option<String>,
    pub started_ms: u64,
    pub ended_ms: Option<u64>,
    pub exit: Option<i32>,
    /// Where in the session's byte stream it started and ended (the
    /// stream the ring keeps the tail of).
    pub at: u64,
    pub end_at: Option<u64>,
    /// The tmux pane it ran in (`%3`), through a tmux client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<String>,
    /// The last non-empty line of its own output (see the module head);
    /// `None` while it runs, when it printed nothing, when it held the
    /// screen, and through a tmux client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_line: Option<String>,
}

/// `commands.json`: what the node and the faces read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub doing: Doing,
    pub recent: Vec<Command>,
    /// Through a tmux client, the pane `doing` is the reading of (`%3`):
    /// the node falls back to tmux's own reading of it while it has sent
    /// no marks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<String>,
    /// The tmux panes that have sent a mark, oldest first. `None` from a
    /// host older than this field, which kept no such list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marked_panes: Option<Vec<String>>,
    /// Each pane's own reading, by pane, from a shell's own ledger (a
    /// `tmux` typed in it): its running command, else its last one, and
    /// its directory. The node says it for the pane in front of the
    /// shell's tmux client. Empty from a host older than this field.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub panes: BTreeMap<String, Doing>,
}

/// One line of the machine's `history.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub session: String,
    #[serde(flatten)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Scan {
    #[default]
    Ground,
    Esc,
    /// Inside an OSC.
    Osc,
    OscEsc,
    /// Inside a DCS (only a tmux passthrough is read).
    Dcs,
    DcsEsc,
    /// Inside a CSI: skipped, the modes have their own reader.
    Csi,
    /// After `ESC (`, `ESC )`, `ESC *` or `ESC +`: the one byte naming a
    /// character set (`tput sgr0` writes `ESC ( B`), not text.
    Charset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Unknown,
    Prompt,
    Input,
    Running,
}

/// The most panes a tmux client's ledger keeps a context for.
const PANES_MAX: usize = 64;

/// One pane's place in its marks, while another pane's is being read.
#[derive(Debug, Default)]
struct Parked {
    phase: Phase,
    named: Option<String>,
    running: Option<Command>,
    cwd: Option<String>,
}

/// **The running command's output, as lines** (see the module head): the
/// line being written, and the last non-empty one finished.
#[derive(Debug, Default)]
struct Tail {
    line: Vec<u8>,
    last: Vec<u8>,
    /// A `\r` came: the next text rewrites the line from its start.
    back: bool,
    /// The command held the screen at some point: it said no lines.
    held: bool,
}

impl Tail {
    fn byte(&mut self, b: u8) {
        match b {
            b'\n' => {
                if !blank(&self.line) {
                    self.last = std::mem::take(&mut self.line);
                } else {
                    self.line.clear();
                }
                self.back = false;
            }
            b'\r' => self.back = true,
            0x08 => {
                while let Some(last) = self.line.pop() {
                    if last & 0xc0 != 0x80 {
                        break;
                    }
                }
            }
            b'\t' | 0x20.. if b != 0x7f => {
                if std::mem::take(&mut self.back) {
                    self.line.clear();
                }
                if self.line.len() < TAIL_LINE_MAX {
                    self.line.push(if b == b'\t' { b' ' } else { b });
                }
            }
            _ => {}
        }
    }

    /// The last non-empty line: the one being written, else the last one
    /// finished; trimmed and cut to [`LAST_LINE_MAX`] characters.
    fn said(&self) -> Option<String> {
        if self.held {
            return None;
        }
        let line = if blank(&self.line) { &self.last } else { &self.line };
        let text = String::from_utf8_lossy(line);
        let text = text.trim();
        (!text.is_empty()).then(|| text.chars().take(LAST_LINE_MAX).collect())
    }
}

/// Nothing but spaces.
fn blank(line: &[u8]) -> bool {
    line.iter().all(|b| b.is_ascii_whitespace())
}

/// See the module head.
#[derive(Debug, Default)]
pub struct Ledger {
    scan: Scan,
    buf: Vec<u8>,
    modes: Modes,
    phase: Phase,
    /// Text typed after `B`, for a shell that names no command line.
    typed: Vec<u8>,
    /// A command line that came before its `C` (VS Code's order).
    named: Option<String>,
    running: Option<Command>,
    recent: VecDeque<Command>,
    cwd: Option<String>,
    title: Option<String>,
    progress: Option<u8>,
    progress_failed: bool,
    notice: Option<(String, u64)>,
    /// When the bell last rang.
    bell: u64,
    next: u64,
    offset: u64,
    /// Any mark seen at all: the shell has an integration.
    pub marked: bool,
    /// Something a reader would see changed since [`Ledger::take_changes`].
    changed: bool,
    /// Commands finished since [`Ledger::take_finished`].
    finished: Vec<Command>,
    /// Read off a tmux client (see the module head).
    tmux: bool,
    /// The pane whose context is in the fields above; `None` for the
    /// shell's own.
    pane: Option<String>,
    /// The pane tmux has in front, from its title.
    front: Option<String>,
    /// The other contexts, oldest first (`None` is a shell's own).
    parked: Vec<(Option<String>, Parked)>,
    /// A `KhorPane` named a pane for the string that follows it alone.
    lent: bool,
    /// Every pane that has sent a mark, oldest first.
    marked_panes: Vec<String>,
    /// The running command's output, as lines.
    tail: Tail,
}

impl Ledger {
    /// A ledger for a host whose PTY runs khor's tmux client.
    pub fn tmux() -> Ledger {
        Ledger {
            tmux: true,
            ..Ledger::default()
        }
    }

    /// Eats the session's output in stream order; sequences may be split
    /// anywhere across calls. `now_ms` stamps what starts or ends here.
    pub fn feed(&mut self, bytes: &[u8], now_ms: u64) {
        let full = self.modes.program_holds_screen();
        self.modes.feed(bytes);
        if self.modes.program_holds_screen() != full {
            self.changed = true;
        }
        if self.running.is_some() && (full || self.modes.program_holds_screen()) {
            self.tail.held = true;
        }
        for &b in bytes {
            self.byte(b, now_ms);
            self.offset += 1;
        }
    }

    fn byte(&mut self, b: u8, now: u64) {
        match self.scan {
            Scan::Ground => {
                if b == 0x1b {
                    self.scan = Scan::Esc;
                } else if b == 0x07 {
                    self.ring(now);
                } else if self.phase == Phase::Input && !self.through_tmux() {
                    self.typed_byte(b);
                } else if self.phase == Phase::Running && !self.through_tmux() {
                    self.tail.byte(b);
                }
            }
            Scan::Esc => {
                self.buf.clear();
                self.scan = match b {
                    b']' => Scan::Osc,
                    b'P' => Scan::Dcs,
                    b'[' => Scan::Csi,
                    b'(' | b')' | b'*' | b'+' => Scan::Charset,
                    _ => Scan::Ground,
                };
            }
            Scan::Charset => self.scan = Scan::Ground,
            Scan::Csi => {
                if (0x40..=0x7e).contains(&b) {
                    self.scan = Scan::Ground;
                }
            }
            Scan::Osc => match b {
                0x07 => {
                    self.scan = Scan::Ground;
                    self.osc(now);
                }
                0x1b => self.scan = Scan::OscEsc,
                0x18 | 0x1a => self.scan = Scan::Ground,
                _ => self.push(b),
            },
            Scan::OscEsc => {
                if b == b'\\' {
                    self.scan = Scan::Ground;
                    self.osc(now);
                } else {
                    // Not a terminator: the string ended unterminated.
                    self.scan = Scan::Ground;
                }
            }
            Scan::Dcs => match b {
                0x1b => self.scan = Scan::DcsEsc,
                0x18 | 0x1a => self.scan = Scan::Ground,
                _ => self.push(b),
            },
            Scan::DcsEsc => {
                if b == b'\\' {
                    self.scan = Scan::Ground;
                    self.dcs(now);
                } else {
                    // tmux doubles every ESC inside its passthrough: two
                    // are one.
                    self.push(0x1b);
                    if b != 0x1b {
                        self.push(b);
                    }
                    self.scan = Scan::Dcs;
                }
            }
        }
    }

    fn push(&mut self, b: u8) {
        if self.buf.len() < MARK_MAX {
            self.buf.push(b);
        }
    }

    /// What is typed at a prompt, roughly: printable text, with a
    /// backspace taking the last character back. Escape sequences never
    /// reach here (the scan takes them).
    fn typed_byte(&mut self, b: u8) {
        match b {
            0x08 | 0x7f => {
                while let Some(last) = self.typed.pop() {
                    // Back to the start of the UTF-8 character.
                    if last & 0xc0 != 0x80 {
                        break;
                    }
                }
            }
            b'\r' | b'\n' => {}
            _ if b >= 0x20 && self.typed.len() < TYPED_MAX => self.typed.push(b),
            _ => {}
        }
    }

    fn dcs(&mut self, now: u64) {
        let Some(inner) = self.buf.strip_prefix(b"tmux;").map(<[u8]>::to_vec) else {
            return;
        };
        // The inner bytes are a whole sequence of their own.
        let saved = std::mem::take(&mut self.buf);
        let scan = std::mem::take(&mut self.scan);
        for b in inner {
            self.byte(b, now);
        }
        self.scan = scan;
        self.buf = saved;
        self.buf.clear();
    }

    /// Whether the context in the fields reads its marks through tmux.
    fn through_tmux(&self) -> bool {
        self.tmux || self.pane.is_some()
    }

    /// Back to the pane in front once a lent pane's mark is read, or to a
    /// shell's own context.
    fn give_back(&mut self) {
        if !std::mem::take(&mut self.lent) {
            return;
        }
        if !self.tmux {
            self.switch(None);
        } else if let Some(front) = self.front.clone() {
            self.switch(Some(&front));
        }
    }

    /// Puts `pane`'s context in the fields, parking the one there.
    fn switch(&mut self, pane: Option<&str>) {
        if self.pane.as_deref() == pane {
            return;
        }
        let was = self.pane.take();
        // A tmux client's context before any pane is nobody's; a shell's
        // is its own.
        if was.is_some() || !self.tmux {
            let parked = Parked {
                phase: std::mem::take(&mut self.phase),
                named: self.named.take(),
                running: self.running.take(),
                cwd: self.cwd.take(),
            };
            if self.parked.len() >= PANES_MAX {
                self.parked.remove(0);
            }
            self.parked.push((was, parked));
        }
        let back = self
            .parked
            .iter()
            .position(|(p, _)| p.as_deref() == pane)
            .map(|i| self.parked.remove(i).1)
            .unwrap_or_default();
        self.phase = back.phase;
        self.named = back.named;
        self.running = back.running;
        self.cwd = back.cwd;
        self.typed.clear();
        self.pane = pane.map(str::to_owned);
        self.changed = true;
    }

    fn osc(&mut self, now: u64) {
        let text = String::from_utf8_lossy(&self.buf).into_owned();
        let (code, rest) = text.split_once(';').unwrap_or((text.as_str(), ""));
        if code == "633"
            && let Some(pane) = rest.strip_prefix("P;KhorPane=").filter(|p| !p.is_empty())
        {
            self.give_back();
            if !self.marked_panes.iter().any(|p| p == pane) {
                if self.marked_panes.len() >= PANES_MAX {
                    self.marked_panes.remove(0);
                }
                self.marked_panes.push(pane.to_owned());
                self.changed = true;
            }
            self.switch(Some(pane));
            self.lent = true;
            return;
        }
        self.osc_read(code, rest, now);
        self.give_back();
    }

    fn osc_read(&mut self, code: &str, rest: &str, now: u64) {
        match code {
            "133" => {
                self.marked = true;
                let mut parts = rest.split(';');
                let kind = parts.next().unwrap_or("");
                let params: Vec<&str> = parts.collect();
                self.mark(kind, &params, now);
            }
            "633" => {
                self.marked = true;
                let mut parts = rest.splitn(2, ';');
                match parts.next() {
                    Some("E") => {
                        let line = parts.next().unwrap_or("");
                        // A trailing `;nonce` (VS Code's) is not the line.
                        let line = line.split(';').next().unwrap_or("");
                        self.name(unescape(line));
                    }
                    Some("P") => {
                        if let Some(cwd) = parts.next().and_then(|p| p.strip_prefix("Cwd=")) {
                            self.set_cwd(unescape(cwd));
                        }
                    }
                    Some(kind @ ("A" | "B" | "C" | "D")) => {
                        let params: Vec<&str> = parts
                            .next()
                            .map(|p| p.split(';').collect())
                            .unwrap_or_default();
                        self.mark(kind, &params, now);
                    }
                    _ => {}
                }
            }
            "7" => {
                if let Some(path) = file_url_path(rest) {
                    self.set_cwd(path);
                }
            }
            // ConEmu's progress (`9;4;state;percent`, which Windows
            // Terminal, Ghostty and WezTerm read too): 0 clears, 1 a
            // figure, 2 failed, 3 busy with no figure, 4 paused.
            "9" if rest.starts_with("4;") => {
                let mut f = rest[2..].split(';');
                let state = f.next().and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
                let pct = f.next().and_then(|v| v.parse::<u8>().ok()).unwrap_or(0).min(100);
                let (progress, failed) = match state {
                    0 => (None, false),
                    1 | 4 => (Some(pct), false),
                    2 => (Some(pct), true),
                    _ => (Some(255), false),
                };
                if (progress, failed) != (self.progress, self.progress_failed) {
                    self.progress = progress;
                    self.progress_failed = failed;
                    self.changed = true;
                }
            }
            // ConEmu's other `9;n;…` verbs are not notifications.
            "9" if rest.split(';').next().is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())) => {}
            // iTerm2's notification: `9;text`.
            "9" => self.notify(rest, now),
            // urxvt's and Ghostty's: `777;notify;title;body`.
            "777" => {
                let mut f = rest.splitn(3, ';');
                if f.next() == Some("notify") {
                    let title = f.next().unwrap_or("");
                    let body = f.next().unwrap_or("");
                    let text = match (title.is_empty(), body.is_empty()) {
                        (false, false) => format!("{title}: {body}"),
                        (false, true) => title.to_owned(),
                        _ => body.to_owned(),
                    };
                    self.notify(&text, now);
                }
            }
            "0" | "2" => {
                let mut t = rest.trim();
                // khor's grouped session titles its client `%n title`.
                if self.tmux
                    && let Some((pane, title)) = front_pane(t)
                {
                    if self.front.as_deref() != Some(pane) {
                        self.front = Some(pane.to_owned());
                        self.switch(Some(pane));
                    }
                    t = title;
                }
                let t = (!t.is_empty()).then(|| t.to_owned());
                if t != self.title {
                    self.title = t;
                    self.changed = true;
                }
            }
            _ => {}
        }
    }

    fn mark(&mut self, kind: &str, params: &[&str], now: u64) {
        match kind {
            "A" => {
                // A prompt over a command never marked as ended: the
                // shell was interrupted, or its integration has no D.
                if self.running.is_some() {
                    self.end(None, now);
                }
                self.phase = Phase::Prompt;
            }
            "B" => {
                self.phase = Phase::Input;
                self.typed.clear();
            }
            "C" => {
                if self.running.is_some() {
                    // Through tmux, a pane out of sight loses its marks
                    // (`allow-passthrough on`): a new command in it says
                    // the last one ended, when, unknown how.
                    if !self.through_tmux() {
                        return;
                    }
                    self.end(None, now);
                }
                let mut text = self.named.take();
                for p in params {
                    if let Some(v) = p.strip_prefix("cmdline_url=") {
                        text = Some(percent_decode(v));
                    } else if let Some(v) = p.strip_prefix("cmdline=") {
                        text = Some(unescape(v));
                    }
                }
                if text.is_none() && !self.typed.is_empty() {
                    text = Some(String::from_utf8_lossy(&self.typed).trim().to_owned());
                }
                self.typed.clear();
                self.next += 1;
                self.running = Some(Command {
                    n: self.next,
                    text: text.filter(|t| !t.is_empty()),
                    cwd: self.cwd.clone(),
                    started_ms: now,
                    ended_ms: None,
                    exit: None,
                    at: self.offset,
                    end_at: None,
                    pane: self.pane.clone(),
                    last_line: None,
                });
                self.tail = Tail::default();
                self.phase = Phase::Running;
                self.changed = true;
            }
            "D" => {
                let exit = params.first().and_then(|p| p.trim().parse::<i32>().ok());
                self.end(exit, now);
            }
            _ => {}
        }
    }

    fn name(&mut self, line: String) {
        let line = line.trim().to_owned();
        if line.is_empty() {
            return;
        }
        let through_tmux = self.through_tmux();
        match &mut self.running {
            // fish 4 says C first, then this; khor's scripts never do
            // inside tmux, where a command still running is one whose
            // end was lost out of sight.
            Some(c) if !through_tmux => {
                c.text = Some(line);
                self.changed = true;
            }
            _ => self.named = Some(line),
        }
    }

    fn end(&mut self, exit: Option<i32>, now: u64) {
        let Some(mut c) = self.running.take() else {
            return;
        };
        c.ended_ms = Some(now);
        c.exit = exit;
        c.end_at = Some(self.offset);
        c.last_line = std::mem::take(&mut self.tail).said().filter(|_| !self.through_tmux());
        // A finished command's progress is over, whatever it last said.
        self.progress = None;
        self.progress_failed = false;
        self.recent.push_back(c.clone());
        while self.recent.len() > RECENT {
            self.recent.pop_front();
        }
        self.finished.push(c);
        self.phase = Phase::Unknown;
        self.changed = true;
    }

    fn notify(&mut self, text: &str, now: u64) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        self.notice = Some((text.chars().take(200).collect(), now));
        self.changed = true;
    }

    /// The bell (BEL outside any sequence; a BEL ending an OSC is that
    /// string's end, not a bell).
    fn ring(&mut self, now: u64) {
        if self.bell == 0 || now.saturating_sub(self.bell) >= BELL_EVERY_MS {
            self.bell = now;
            self.changed = true;
        }
    }

    fn set_cwd(&mut self, cwd: String) {
        if self.cwd.as_deref() != Some(cwd.as_str()) {
            self.cwd = Some(cwd);
            self.changed = true;
        }
    }

    /// One context's reading: its running command, else the last one it
    /// finished, and its directory.
    fn reading(&self, pane: Option<&str>, running: Option<&Command>, cwd: Option<&String>) -> Doing {
        let c = running.or_else(|| self.recent.iter().rev().find(|c| c.pane.as_deref() == pane));
        Doing {
            command: c.and_then(|c| c.text.as_ref()).map(|t| one_line(t)),
            running: running.is_some(),
            started_ms: c.map_or(0, |c| c.started_ms),
            ended_ms: c.and_then(|c| c.ended_ms),
            exit: c.and_then(|c| c.exit),
            cwd: cwd.cloned(),
            last_line: c.filter(|_| running.is_none()).and_then(|c| c.last_line.clone()),
            ..Doing::default()
        }
    }

    /// Every pane's reading in a shell's own ledger, the one in the
    /// fields and the parked ones. A tmux client's ledger reads its front
    /// pane in [`Ledger::doing`] already.
    fn panes(&self) -> BTreeMap<String, Doing> {
        if self.tmux {
            return BTreeMap::new();
        }
        let here = self
            .pane
            .as_deref()
            .map(|p| (p, self.reading(Some(p), self.running.as_ref(), self.cwd.as_ref())));
        let parked = self.parked.iter().filter_map(|(p, k)| {
            let p = p.as_deref()?;
            Some((p, self.reading(Some(p), k.running.as_ref(), k.cwd.as_ref())))
        });
        here.into_iter()
            .chain(parked)
            .filter(|(p, _)| self.marked_panes.iter().any(|m| m == p))
            .map(|(p, d)| (p.to_owned(), d))
            .collect()
    }

    /// The row's reading: the running command, else the last one.
    pub fn doing(&self) -> Doing {
        Doing {
            title: self.title.clone(),
            // tmux itself holds a client's screen, whatever the pane runs.
            full_screen: !self.tmux && self.modes.program_holds_screen(),
            progress: self.progress,
            progress_failed: self.progress_failed,
            notice: self.notice.as_ref().map(|(t, _)| t.clone()),
            notice_ms: self.notice.as_ref().map_or(0, |(_, at)| *at),
            bell_ms: self.bell,
            ..self.reading(self.pane.as_deref(), self.running.as_ref(), self.cwd.as_ref())
        }
    }

    pub fn saved(&self) -> Saved {
        Saved {
            doing: self.doing(),
            recent: self.recent.iter().cloned().collect(),
            pane: self.front.clone().or_else(|| self.pane.clone()),
            marked_panes: Some(self.marked_panes.clone()),
            panes: self.panes(),
        }
    }

    /// Whether anything a reader sees changed since the last ask.
    pub fn take_changes(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// The commands that finished since the last ask.
    pub fn take_finished(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.finished)
    }
}

/// A tmux client's title as khor's grouped session sets it: the pane in
/// front (`%3`), then the pane's own title.
fn front_pane(title: &str) -> Option<(&str, &str)> {
    let (pane, rest) = title.split_once(' ').unwrap_or((title, ""));
    let digits = pane.strip_prefix('%')?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then_some((pane, rest.trim()))
}

/// `commands.json` beside a session.
pub fn saved_path(dir: &Path) -> PathBuf {
    dir.join("commands.json")
}

/// The ledger's file, written whole (a reader never sees half of it).
pub fn write_saved(dir: &Path, saved: &Saved) {
    let Ok(bytes) = serde_json::to_vec(saved) else {
        return;
    };
    let tmp = dir.join("commands.json.tmp");
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(&tmp, saved_path(dir));
    }
}

/// What the session's shell is doing, from its ledger's file — `None`
/// for a session whose shell never marked anything.
pub fn read_doing(dir: &Path) -> Option<Doing> {
    let bytes = std::fs::read(saved_path(dir)).ok()?;
    serde_json::from_slice::<Saved>(&bytes)
        .ok()
        .map(|s| s.doing)
}

/// A session's ledger file, whole.
pub fn read_saved(dir: &Path) -> Option<Saved> {
    let bytes = std::fs::read(saved_path(dir)).ok()?;
    serde_json::from_slice::<Saved>(&bytes).ok()
}

/// Appends finished commands to the machine's history, cutting the file
/// to its newest half when it passes [`HISTORY_MAX`].
pub fn append_history(file: &Path, session: &str, done: &[Command]) {
    use std::io::Write;
    if done.is_empty() {
        return;
    }
    let mut out = Vec::new();
    for c in done.iter().filter(|c| c.text.is_some()) {
        if let Ok(line) = serde_json::to_vec(&Line {
            session: session.to_owned(),
            command: c.clone(),
        }) {
            out.extend_from_slice(&line);
            out.push(b'\n');
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
    {
        let _ = f.write_all(&out);
    }
    if std::fs::metadata(file).map(|m| m.len()).unwrap_or(0) > HISTORY_MAX
        && let Ok(all) = std::fs::read(file)
    {
        let half = all.len() / 2;
        let from = all[half..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(all.len(), |i| half + i + 1);
        let tmp = file.with_extension("jsonl.tmp");
        if std::fs::write(&tmp, &all[from..]).is_ok() {
            let _ = std::fs::rename(&tmp, file);
        }
    }
}

/// Where a khor root keeps its machine's history.
pub fn history_file(root: &Path) -> PathBuf {
    root.join(".khor").join("history.jsonl")
}

/// What a history search asks (roadmap 2.6). Every condition given must
/// hold. Positional on the wire (`proto::Request::History`): a field is
/// only ever added at the end, with `#[serde(default)]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryAsk {
    /// Words the command line holds, in any case and any order.
    pub text: String,
    /// Ran in this directory or under it; empty is anywhere.
    pub dir: String,
    /// Only the commands that failed (an exit other than 0).
    pub failed: bool,
}

/// One command line found, as the wire and the faces carry it: the
/// newest run of it, and how many times it ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryHit {
    pub session: String,
    pub text: String,
    pub cwd: Option<String>,
    pub started_ms: u64,
    pub ended_ms: Option<u64>,
    pub exit: Option<i32>,
    pub runs: u32,
}

/// The most lines a search answers: a list to pick from, not an export.
pub const HISTORY_HITS: usize = 200;

/// **The machine's commands, searched** (roadmap 2.6): newest first, one
/// row per command line with its run count — iTerm2's command history and
/// atuin's search show the same. Fed by every session's ledger
/// ([`append_history`]), so it spans sessions.
pub fn search_history(file: &Path, ask: &HistoryAsk) -> Vec<HistoryHit> {
    let words: Vec<String> = ask
        .text
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    let dir = ask.dir.trim_end_matches('/');
    let mut out: Vec<HistoryHit> = Vec::new();
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for line in read_history(file).into_iter().rev() {
        let c = line.command;
        let Some(text) = c.text.clone() else { continue };
        if ask.failed && !matches!(c.exit, Some(e) if e != 0) {
            continue;
        }
        if !dir.is_empty() {
            let Some(cwd) = c.cwd.as_deref() else { continue };
            if cwd != dir && !cwd.starts_with(&format!("{dir}/")) {
                continue;
            }
        }
        let lower = text.to_lowercase();
        if !words.iter().all(|w| lower.contains(w.as_str())) {
            continue;
        }
        if let Some(&i) = seen.get(&text) {
            out[i].runs += 1;
            continue;
        }
        if out.len() >= HISTORY_HITS {
            continue;
        }
        seen.insert(text.clone(), out.len());
        out.push(HistoryHit {
            session: line.session,
            text,
            cwd: c.cwd,
            started_ms: c.started_ms,
            ended_ms: c.ended_ms,
            exit: c.exit,
            runs: 1,
        });
    }
    out
}

/// The machine's history, newest last.
pub fn read_history(file: &Path) -> Vec<Line> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// A multi-line command for one line of a row.
fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ⏎ ")
}

/// VS Code's escaping, undone: `\\` and `\xNN`.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            if b[i + 1] == b'\\' {
                out.push(b'\\');
                i += 2;
                continue;
            }
            if b[i + 1] == b'x'
                && i + 3 < b.len()
                && let Ok(v) = u8::from_str_radix(&s[i + 2..i + 4], 16)
            {
                out.push(v);
                i += 4;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The path of a `file://host/path` URL (the host is not checked: OSC 7
/// from this session's shell is about this session's machine).
fn file_url_path(url: &str) -> Option<String> {
    let rest = url.strip_prefix("file://")?;
    let slash = rest.find('/')?;
    Some(percent_decode(&rest[slash..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fed(bytes: &[u8]) -> Ledger {
        let mut l = Ledger::default();
        // Fed in uneven pieces: a mark split anywhere must still read.
        for (i, chunk) in bytes.chunks(7).enumerate() {
            l.feed(chunk, 1000 + i as u64);
        }
        l
    }

    /// **The machine's history, searched** (roadmap 2.6): across
    /// sessions, newest first, one row per line with its count, by words,
    /// by directory (and under it, not a sibling sharing a prefix), by
    /// failure.
    #[test]
    fn history_is_searched_by_words_directory_and_failure() {
        let dir = std::env::temp_dir().join(format!("khor-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("history.jsonl");
        let c = |n: u64, text: &str, cwd: &str, exit: i32| Command {
            n,
            text: Some(text.to_owned()),
            cwd: Some(cwd.to_owned()),
            started_ms: n * 1000,
            ended_ms: Some(n * 1000 + 5),
            exit: Some(exit),
            at: 0,
            end_at: None,
            pane: None,
            last_line: None,
        };
        append_history(&file, "shell/a", &[c(1, "cargo test -p khor-node", "/w/khor", 0), c(2, "git status", "/w/khor", 0)]);
        append_history(&file, "shell/b", &[c(3, "cargo build", "/w/khor-net", 101), c(4, "Cargo Test -p khor-node", "/w/khor/crates", 1)]);
        append_history(&file, "shell/a", &[c(5, "cargo test -p khor-node", "/w/khor", 0)]);
        let ask = |text: &str, dir: &str, failed: bool| {
            search_history(&file, &HistoryAsk { text: text.into(), dir: dir.into(), failed })
                .into_iter()
                .map(|h| (h.text, h.runs))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ask("", "", false),
            [("cargo test -p khor-node".into(), 2), ("Cargo Test -p khor-node".into(), 1), ("cargo build".into(), 1), ("git status".into(), 1)]
        );
        assert_eq!(ask("TEST node", "", false).len(), 2, "words in any case and order");
        assert_eq!(
            ask("cargo", "/w/khor", false),
            [("cargo test -p khor-node".into(), 2), ("Cargo Test -p khor-node".into(), 1)],
            "under the directory, not khor-net beside it"
        );
        assert_eq!(ask("", "", true), [("Cargo Test -p khor-node".into(), 1), ("cargo build".into(), 1)]);
        assert!(search_history(&dir.join("none.jsonl"), &HistoryAsk::default()).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn texts(l: &Ledger) -> Vec<(Option<String>, Option<i32>)> {
        l.recent.iter().map(|c| (c.text.clone(), c.exit)).collect()
    }

    #[test]
    fn the_marks_make_commands_with_their_ends() {
        let s = b"\x1b]7;file://h/home/me\x07\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\
            \x1b]633;E;ls -la\x07\x1b]133;C\x07a b\r\n\x1b]133;D;0\x07\
            \x1b]133;A\x07$ \x1b]133;B\x07\x1b]633;E;false\x07\x1b]133;C\x07\x1b]133;D;1\x07\
            \x1b]133;A\x07";
        let l = fed(s);
        assert_eq!(
            texts(&l),
            vec![
                (Some("ls -la".into()), Some(0)),
                (Some("false".into()), Some(1))
            ]
        );
        assert_eq!(l.recent[0].cwd.as_deref(), Some("/home/me"));
        let d = l.doing();
        assert!(!d.running);
        assert_eq!(d.command.as_deref(), Some("false"));
        assert_eq!(d.exit, Some(1));
    }

    /// **What a finished command last said** (the terminal row, the owner
    /// 2026-10-08): the last non-empty line of its own output, colours
    /// and a charset switch dropped, a `\r` progress line as its last
    /// rewrite, a backspace taking a character back, cut to
    /// [`LAST_LINE_MAX`]; nothing while it runs, when it printed nothing,
    /// or when it held the screen.
    #[test]
    fn a_finished_command_keeps_the_last_line_it_printed() {
        let run = |cmd: &str, out: &[u8], exit: i32| {
            let mut s = format!("\x1b]133;A\x07$ \x1b]133;B\x07\x1b]633;E;{cmd}\x07\x1b]133;C\x07").into_bytes();
            s.extend_from_slice(out);
            s.extend_from_slice(format!("\x1b]133;D;{exit}\x07\x1b]133;A\x07$ ").as_bytes());
            s
        };
        let last = |s: &[u8]| fed(s).doing().last_line;
        assert_eq!(
            last(&run("cargo build", b"   Compiling khor\r\n\x1b[1m\x1b[32m    Finished\x1b(B\x1b[m `release` in 41s\r\n\r\n", 0)).as_deref(),
            Some("Finished `release` in 41s"),
            "the last non-empty line, escapes dropped"
        );
        assert_eq!(
            last(&run("pip install x", b"  10%\r  55%\r 100% done\r\n", 0)).as_deref(),
            Some("100% done"),
            "a rewritten line is its last rewrite"
        );
        assert_eq!(last(&run("printf", b"no newline", 0)).as_deref(), Some("no newline"), "a line not ended yet");
        assert_eq!(last(&run("typo", b"abX\x08c\r\n", 1)).as_deref(), Some("abc"), "a backspace takes one back");
        assert_eq!(last(&run("true", b"", 0)), None, "printed nothing");
        assert_eq!(last(&run("vim", b"\x1b[?1049hdrawn\r\n\x1b[?1049l", 0)), None, "a full-screen program says no line");
        let long = "x".repeat(LAST_LINE_MAX + 50);
        assert_eq!(last(&run("yes", long.as_bytes(), 0)).map(|l| l.chars().count()), Some(LAST_LINE_MAX));
        // Running: no last line yet, and a new command starts empty.
        let mut s = run("echo a", b"a\r\n", 0);
        s.extend_from_slice(b"\x1b]133;B\x07\x1b]633;E;sleep 9\x07\x1b]133;C\x07");
        let l = fed(&s);
        assert!(l.doing().running && l.doing().last_line.is_none());
        assert_eq!(l.recent.back().and_then(|c| c.last_line.as_deref()), Some("a"));
    }

    #[test]
    fn a_command_named_after_its_start_is_named_and_a_tmux_wrap_is_opened() {
        // fish 4: C with `cmdline_url`, then VS Code's E; and a mark
        // wrapped for tmux with nothing to unwrap it.
        let s = b"\x1b]133;A;click_events=1\x1b\\\x1b]133;B\x1b\\\
            \x1b]133;C;cmdline_url=echo%20hi\x1b\\\x1b]633;E;echo hi\x07\
            \x1bPtmux;\x1b\x1b]133;D;0\x07\x1b\\";
        let l = fed(s);
        assert_eq!(texts(&l), vec![(Some("echo hi".into()), Some(0))]);
    }

    /// **Through a tmux client** (item 21): two visible panes' marks
    /// interleave, each pane keeps its own command and directory, the row
    /// reads the pane the title puts in front, and a switch of front pane
    /// shows that pane's own last command, not the other's.
    #[test]
    fn a_tmux_client_reads_each_pane_and_shows_the_one_in_front() {
        // As tmux writes a mark it unwrapped: the pane, then the mark.
        fn wrap(pane: &str, osc: &str) -> String {
            format!("\x1b]633;P;KhorPane={pane}\x07\x1b]{osc}\x07")
        }
        let mut s = String::from("\x1b]2;%1 left\x07");
        for (pane, osc) in [
            ("%1", "7;file://h/a"),
            ("%1", "133;A"),
            ("%2", "7;file://h/b"),
            ("%2", "133;A"),
            ("%1", "633;E;make"),
            ("%1", "133;C"),
            ("%2", "633;E;sleep 9"),
            ("%2", "133;C"),
            ("%1", "133;D;2"),
            ("%1", "133;A"),
        ] {
            s.push_str(&wrap(pane, osc));
            // tmux's redraw between marks is never typed text.
            s.push_str("\x1b[3;1Hxyz");
        }
        let mut l = Ledger::tmux();
        for (i, chunk) in s.as_bytes().chunks(7).enumerate() {
            l.feed(chunk, 1000 + i as u64);
        }
        let d = l.doing();
        assert_eq!(d.title.as_deref(), Some("left"));
        assert_eq!((d.command.as_deref(), d.running, d.exit), (Some("make"), false, Some(2)));
        assert_eq!(d.cwd.as_deref(), Some("/a"));
        // A mark from the pane beside it is that pane's, and the row
        // stays on the one in front.
        l.feed(wrap("%2", "7;file://h/b2").as_bytes(), 4000);
        assert_eq!(l.doing().cwd.as_deref(), Some("/a"));
        let done = l.take_finished();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].pane.as_deref(), Some("%1"));
        // The person moves to the right pane: its command is still running.
        l.feed(b"\x1b]2;%2 right\x07", 5000);
        let d = l.doing();
        assert_eq!((d.command.as_deref(), d.running), (Some("sleep 9"), true));
        assert_eq!(d.cwd.as_deref(), Some("/b2"));
        // Still wrapped, as a tmux nested in this one passes it on.
        l.feed(b"\x1bPtmux;\x1b\x1b]633;P;KhorPane=%1\x07\x1b\x1b]633;E;ls\x07\x1b\\\
            \x1bPtmux;\x1b\x1b]633;P;KhorPane=%1\x07\x1b\x1b]133;C\x07\x1b\\", 5500);
        assert_eq!(l.doing().command.as_deref(), Some("sleep 9"));
        l.feed(wrap("%2", "133;D;0").as_bytes(), 6000);
        let d = l.doing();
        assert_eq!((d.command.as_deref(), d.running, d.exit), (Some("sleep 9"), false, Some(0)));
        // Out of sight, a pane's end and next prompt are dropped: its next
        // command closes the last one, end unknown, and is its own.
        l.feed(format!("{}{}", wrap("%2", "633;E;sleep 1"), wrap("%2", "133;C")).as_bytes(), 6500);
        l.feed(format!("{}{}", wrap("%2", "633;E;date"), wrap("%2", "133;C")).as_bytes(), 6600);
        let d = l.doing();
        assert_eq!((d.command.as_deref(), d.running), (Some("date"), true));
        let lost = l.take_finished();
        assert_eq!(lost.last().map(|c| (c.text.as_deref(), c.exit)), Some((Some("sleep 1"), None)));
        // A pane that never marked shows nothing of the others'.
        l.feed(b"\x1b]2;%7 vim\x07", 7000);
        assert_eq!(l.doing().command, None);
    }

    #[test]
    fn doubled_marks_count_once_and_typed_text_stands_in_for_a_name() {
        let s = b"\x1b]133;A\x07\x1b]133;A\x07% \x1b]133;B\x07git sx\x08tatus\r\n\
            \x1b]133;C\x07\x1b]133;C\x07out\r\n\x1b]133;D;0\x07\x1b]133;D;0\x07";
        let l = fed(s);
        assert_eq!(texts(&l), vec![(Some("git status".into()), Some(0))]);
    }

    #[test]
    fn a_full_screen_program_and_the_title_are_read() {
        let mut l = fed(b"\x1b]0;build\x07\x1b]133;C\x07\x1b[?1049h");
        assert!(l.doing().full_screen && l.doing().running);
        assert_eq!(l.doing().title.as_deref(), Some("build"));
        l.feed(b"\x1b[?1049l\x1b]133;D;0\x07", 2000);
        assert!(!l.doing().full_screen && !l.doing().running);
    }

    #[test]
    fn progress_and_notifications_are_read() {
        let mut l = fed(b"\x1b]133;C\x07\x1b]9;4;1;42\x07");
        assert_eq!(l.doing().progress, Some(42));
        l.feed(b"\x1b]9;4;3;0\x07", 2);
        assert_eq!(l.doing().progress, Some(255));
        l.feed(b"\x1b]9;4;2;80\x1b\\", 3);
        assert!(l.doing().progress_failed);
        l.feed(b"\x1b]9;build done\x07", 4);
        assert_eq!(l.doing().notice.as_deref(), Some("build done"));
        assert_eq!(l.doing().notice_ms, 4);
        l.feed(b"\x1b]9;1;500\x07\x1b]777;notify;make;all tests passed\x07", 5);
        assert_eq!(l.doing().notice.as_deref(), Some("make: all tests passed"));
        l.feed(b"\x1b]133;D;0\x07", 6);
        assert_eq!(l.doing().progress, None, "a finished command's progress is over");
    }

    /// **The bell is recorded with its time** (item 67): a BEL in the
    /// text rings, one ending an OSC does not, and a stream of them is
    /// one a second.
    #[test]
    fn the_bell_is_recorded_once_a_second() {
        let mut l = fed(b"\x1b]0;title\x07$ ");
        assert_eq!(l.doing().bell_ms, 0, "an OSC's BEL is not a bell");
        l.feed(b"\x07", 5000);
        assert_eq!(l.doing().bell_ms, 5000);
        assert!(l.take_changes(), "a bell changes the row");
        l.feed(b"\x07\x07", 5400);
        assert_eq!(l.doing().bell_ms, 5000, "within the second, the same bell");
        assert!(!l.take_changes());
        l.feed(b"\x07", 6100);
        assert_eq!(l.doing().bell_ms, 6100);
    }

    #[test]
    fn the_escapes_come_undone() {
        assert_eq!(
            unescape(r"for i in 1 2\x3b do\x0aecho $i\x3b done"),
            "for i in 1 2; do\necho $i; done"
        );
        assert_eq!(unescape(r"a\\b"), r"a\b");
        assert_eq!(
            file_url_path("file://host/a%20b/c").as_deref(),
            Some("/a b/c")
        );
        assert_eq!(one_line("a\n  b"), "a ⏎ b");
    }
}
