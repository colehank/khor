//! What the host's screen model keeps beside vt100 (item 66): the OSC 8
//! links on its cells and the pictures drawn among its rows.
//!
//! # Why
//!
//! The host hands a re-joining face a snapshot of its model
//! ([`crate::host`], item 46), and vt100 keeps neither: it has no
//! hyperlinks and draws no pictures. An `ls --hyperlink`, a compiler's
//! `file://` links, an `imgcat` were dead text, or nothing, after an
//! attach, though a face that watched them arrive still had them.
//!
//! # How
//!
//! A small scanner reads the bytes the model is fed, ahead of vt100,
//! and cuts the feed wherever it must look at the screen: around each
//! character printed while a link is open (where it landed), after a
//! picture (where it was drawn), before a switch of screens, at an erase
//! of the display, after a reset. vt100 parses the same bytes either
//! way; a cut only decides when this table reads its cursor.
//!
//! **Rows are counted from the shell's birth**, so a row keeps its
//! number as it scrolls into the history. vt100 says nothing when it
//! scrolls, but a history view scrolled back by one row is moved one
//! further by every row that scrolls in (`Grid::scroll_up`): while the
//! model is fed, its view is held one row back and the count is how far
//! it moved ([`Beside::sync`]). Nothing reads the screen in between; the
//! view is back at the bottom before the feed returns.
//!
//! **A link is kept with the text it was printed over**, and handed out
//! only while its cells still hold that text: a redraw, an erase, a
//! program's own cursor moves cost a link, and never put one on other
//! text. A picture has no cells in vt100: it goes when its row leaves
//! the history, when the display under it is erased, and on a reset.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::pictures::PICTURES;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a;
const IIP: &[u8] = b"1337;File=";

/// The longest OSC read for a link: xterm.js keeps a link's target up to
/// its own limit; one longer than this is not taken.
const OSC_MOST: usize = 8 << 10;

/// Linked runs kept per shell before the oldest rows' go: an
/// `ls --hyperlink` of a large directory is one run a file.
const RUNS_MOST: usize = 32 << 10;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Scan {
    #[default]
    Ground,
    Esc,
    /// An escape with intermediates, waiting for its final byte.
    EscInter,
    Csi,
    Osc,
    OscEsc,
    /// `ESC P` and parameters: a sixel if `q` comes next.
    Dcs,
    /// A string nobody here reads (DCS, SOS, PM, APC), to its ST.
    Str,
    StrEsc,
    /// A picture, from its introducer on.
    Body {
        osc: bool,
    },
    BodyEsc {
        osc: bool,
    },
}

/// What a byte asks of the feed.
enum Step {
    Nothing,
    /// A character printed under a link starts here (cut before it)
    /// and ends here too when it is `Some` (cut after it).
    Char {
        starts: bool,
        ends: Option<char>,
    },
    /// A switch of screens: count before it, look after it.
    Switch {
        clears: bool,
    },
    /// An erase of the display (`CSI J`), by mode.
    Erase(u16),
    /// `ESC c`: everything goes.
    Reset,
    /// A picture's last byte.
    Picture,
}

/// Text printed under one link, on one row.
#[derive(Clone, Debug)]
struct Run {
    from: u16,
    to: u16,
    text: String,
    /// The OSC 8 payload after `8;`: parameters, `;`, target.
    link: Arc<str>,
}

#[derive(Clone, Debug)]
struct Picture {
    /// The row it was drawn on, and the column.
    at: u64,
    col: u16,
    bytes: Arc<[u8]>,
}

/// One screen's links and pictures. The alternate screen counts its rows
/// from its top: it has no history to scroll into.
#[derive(Default, Clone, Debug)]
struct Kept {
    runs: BTreeMap<u64, Vec<Run>>,
    /// How many runs `runs` holds.
    count: usize,
    pictures: Vec<Picture>,
}

impl Kept {
    /// The rows before `first` go.
    fn drop_before(&mut self, first: u64) {
        if self
            .runs
            .first_key_value()
            .is_some_and(|(&row, _)| row < first)
        {
            let kept = self.runs.split_off(&first);
            self.count = self
                .count
                .saturating_sub(self.runs.values().map(Vec::len).sum::<usize>());
            self.runs = kept;
        }
    }
}

/// What one row of the model carries, for the snapshot to write.
#[derive(Default)]
pub struct RowMarks {
    /// Each column's link, or nothing; empty for a row with none.
    pub links: Vec<Option<Arc<str>>>,
    /// The pictures drawn on the row, by column, oldest first.
    pub pictures: Vec<(u16, Arc<[u8]>)>,
}

/// Where the cursor stood: the absolute row, the column, which screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Spot {
    row: u64,
    col: u16,
    alt: bool,
}

#[derive(Default)]
pub struct Beside {
    scan: Scan,
    /// A CSI's parameters, an OSC's payload, a DCS's parameters.
    seq: Vec<u8>,
    /// The OSC is longer than [`OSC_MOST`]: read to its end, not kept.
    long: bool,
    /// The picture being read, from its ESC on, while it fits.
    body: Vec<u8>,
    fits: bool,
    /// The link printing now, if one is open.
    link: Option<Arc<str>>,
    /// The character under a link being read: its bytes, how many are
    /// still due, and where the cursor was before it.
    chr: Vec<u8>,
    due: u8,
    before: Option<Spot>,
    /// Rows scrolled into the normal screen's history since the model
    /// began: drawing row `r` is row `scrolled + r`, history row `h` of
    /// `L` is `scrolled - L + h`.
    scrolled: u64,
    /// The history's length and the view's offset when the count began;
    /// `None` while the alternate screen is up or nothing is fed.
    armed: Option<(usize, usize)>,
    normal: Kept,
    alt: Kept,
    pictures_size: usize,
}

impl Beside {
    /// Feeds `bytes` to `parser`, keeping what vt100 drops. `keeps` is
    /// the parser's history length.
    pub fn feed(&mut self, parser: &mut vt100::Parser, bytes: &[u8], keeps: usize) {
        self.arm(parser);
        let mut from = 0;
        let mut i = 0;
        while i < bytes.len() {
            match self.scan {
                // The common case: text with no link open, copied whole.
                Scan::Ground if self.link.is_none() => {
                    match bytes[i..].iter().position(|&b| b == ESC) {
                        Some(n) => i += n,
                        None => break,
                    }
                }
                Scan::Body { osc } => {
                    let run = bytes[i..]
                        .iter()
                        .position(|&b| b == ESC || b == CAN || b == SUB || (osc && b == BEL))
                        .unwrap_or(bytes.len() - i);
                    self.take(&bytes[i..i + run]);
                    i += run;
                    if i == bytes.len() {
                        break;
                    }
                }
                _ => {}
            }
            match self.step(bytes[i]) {
                Step::Nothing => {}
                Step::Char { starts, ends } => {
                    if starts {
                        parser.process(&bytes[from..i]);
                        from = i;
                        self.before = Some(self.spot(parser, keeps));
                    }
                    if let Some(c) = ends {
                        parser.process(&bytes[from..=i]);
                        from = i + 1;
                        self.printed(parser, keeps, c);
                    }
                }
                Step::Switch { clears } => {
                    parser.process(&bytes[from..i]);
                    self.sync(parser, keeps);
                    parser.process(&bytes[i..=i]);
                    from = i + 1;
                    if parser.screen().alternate_screen() {
                        if clears {
                            self.alt = Kept::default();
                        }
                    } else {
                        self.alt = Kept::default();
                    }
                    self.recount_pictures();
                    self.arm(parser);
                }
                Step::Erase(mode) => {
                    parser.process(&bytes[from..i]);
                    let at = self.spot(parser, keeps);
                    parser.process(&bytes[i..=i]);
                    from = i + 1;
                    self.erase(mode, at);
                }
                Step::Reset => {
                    parser.process(&bytes[from..=i]);
                    from = i + 1;
                    self.armed = None;
                    *self = Beside {
                        scan: self.scan,
                        ..Beside::default()
                    };
                    self.arm(parser);
                }
                Step::Picture => {
                    parser.process(&bytes[from..=i]);
                    from = i + 1;
                    let at = self.spot(parser, keeps);
                    self.keep_picture(at);
                }
            }
            i += 1;
        }
        parser.process(&bytes[from..]);
        self.disarm(parser, keeps);
    }

    /// Reads one byte of the stream, outside the fast paths.
    fn step(&mut self, b: u8) -> Step {
        match self.scan {
            Scan::Ground => {
                if b == ESC {
                    self.cut_char();
                    self.scan = Scan::Esc;
                    return Step::Nothing;
                }
                if self.link.is_none() {
                    return Step::Nothing;
                }
                self.ground_char(b)
            }
            Scan::Esc => {
                self.scan = Scan::Ground;
                match b {
                    b'[' => self.open(Scan::Csi),
                    b']' => self.open(Scan::Osc),
                    b'P' => self.open(Scan::Dcs),
                    b'X' | b'^' | b'_' => self.scan = Scan::Str,
                    b'c' => return Step::Reset,
                    ESC => self.scan = Scan::Esc,
                    0x20..=0x2f => self.scan = Scan::EscInter,
                    _ => {}
                }
                Step::Nothing
            }
            Scan::EscInter => {
                match b {
                    0x20..=0x2f => {}
                    ESC => self.scan = Scan::Esc,
                    _ => self.scan = Scan::Ground,
                }
                Step::Nothing
            }
            Scan::Csi => match b {
                0x40..=0x7e => {
                    self.scan = Scan::Ground;
                    self.csi_final(b)
                }
                ESC => {
                    self.scan = Scan::Esc;
                    Step::Nothing
                }
                CAN | SUB => {
                    self.scan = Scan::Ground;
                    Step::Nothing
                }
                0x20..=0x3f => {
                    if self.seq.len() < 64 {
                        self.seq.push(b);
                    }
                    Step::Nothing
                }
                _ => Step::Nothing,
            },
            Scan::Osc => match b {
                BEL => {
                    self.osc_end();
                    Step::Nothing
                }
                ESC => {
                    self.scan = Scan::OscEsc;
                    Step::Nothing
                }
                CAN | SUB => {
                    self.scan = Scan::Ground;
                    Step::Nothing
                }
                _ => {
                    if self.seq.len() < OSC_MOST {
                        self.seq.push(b);
                    } else {
                        self.long = true;
                    }
                    if self.seq == IIP {
                        let mut body = b"\x1b]".to_vec();
                        body.extend_from_slice(IIP);
                        self.start_picture(body, true);
                    }
                    Step::Nothing
                }
            },
            Scan::OscEsc => {
                if b == b'\\' {
                    self.osc_end();
                    Step::Nothing
                } else {
                    // An ESC that does not end the string starts the next.
                    self.scan = Scan::Esc;
                    self.step(b)
                }
            }
            Scan::Dcs => {
                match b {
                    b'q' => {
                        let mut body = b"\x1bP".to_vec();
                        body.extend_from_slice(&self.seq);
                        body.push(b'q');
                        self.start_picture(body, false);
                    }
                    b'0'..=b'9' | b';' if self.seq.len() < 64 => self.seq.push(b),
                    ESC => self.scan = Scan::StrEsc,
                    CAN | SUB => self.scan = Scan::Ground,
                    _ => self.scan = Scan::Str,
                }
                Step::Nothing
            }
            Scan::Str => {
                match b {
                    ESC => self.scan = Scan::StrEsc,
                    CAN | SUB => self.scan = Scan::Ground,
                    _ => {}
                }
                Step::Nothing
            }
            Scan::StrEsc => {
                if b == b'\\' {
                    self.scan = Scan::Ground;
                    Step::Nothing
                } else {
                    self.scan = Scan::Esc;
                    self.step(b)
                }
            }
            Scan::Body { osc } => match b {
                BEL if osc => {
                    self.take(&[BEL]);
                    self.scan = Scan::Ground;
                    Step::Picture
                }
                ESC => {
                    self.scan = Scan::BodyEsc { osc };
                    Step::Nothing
                }
                // CAN and SUB cancel the string: nothing drawn.
                _ => {
                    self.drop_picture();
                    Step::Nothing
                }
            },
            Scan::BodyEsc { .. } => {
                if b == b'\\' {
                    self.take(&[ESC, b'\\']);
                    self.scan = Scan::Ground;
                    Step::Picture
                } else {
                    self.drop_picture();
                    self.scan = Scan::Esc;
                    self.step(b)
                }
            }
        }
    }

    fn open(&mut self, scan: Scan) {
        self.scan = scan;
        self.seq.clear();
        self.long = false;
    }

    /// A byte of text under a link: where a character starts and ends.
    fn ground_char(&mut self, b: u8) -> Step {
        match b {
            0x20..=0x7e => {
                self.cut_char();
                Step::Char {
                    starts: true,
                    ends: Some(b as char),
                }
            }
            0xc2..=0xf4 => {
                self.cut_char();
                self.chr.push(b);
                self.due = if b >= 0xf0 {
                    3
                } else if b >= 0xe0 {
                    2
                } else {
                    1
                };
                Step::Char {
                    starts: true,
                    ends: None,
                }
            }
            0x80..=0xbf if self.due > 0 => {
                self.chr.push(b);
                self.due -= 1;
                if self.due > 0 {
                    return Step::Nothing;
                }
                let c = std::str::from_utf8(&self.chr)
                    .ok()
                    .and_then(|s| s.chars().next());
                self.chr.clear();
                match c {
                    Some(c) => Step::Char {
                        starts: false,
                        ends: Some(c),
                    },
                    None => {
                        self.before = None;
                        Step::Nothing
                    }
                }
            }
            _ => {
                self.cut_char();
                Step::Nothing
            }
        }
    }

    /// A character broken off: nothing is recorded for it.
    fn cut_char(&mut self) {
        self.chr.clear();
        self.due = 0;
        self.before = None;
    }

    fn csi_final(&mut self, b: u8) -> Step {
        let seq = std::mem::take(&mut self.seq);
        let private = seq.first() == Some(&b'?');
        let params = || {
            seq.split(|&c| c == b';')
                .map(|p| p.strip_prefix(b"?").unwrap_or(p))
        };
        match b {
            b'h' | b'l' if private => {
                let mut switch = None;
                for p in params() {
                    match p {
                        b"47" => switch = switch.or(Some(false)),
                        b"1047" | b"1049" => switch = Some(b == b'h'),
                        _ => {}
                    }
                }
                switch.map_or(Step::Nothing, |clears| Step::Switch { clears })
            }
            b'J' if seq.iter().all(|&c| c.is_ascii_digit() || c == b'?') => {
                let mode = params()
                    .next()
                    .and_then(|p| std::str::from_utf8(p).ok()?.parse().ok())
                    .unwrap_or(0);
                Step::Erase(mode)
            }
            _ => Step::Nothing,
        }
    }

    fn osc_end(&mut self) {
        self.scan = Scan::Ground;
        if self.long {
            return;
        }
        let Some(rest) = self.seq.strip_prefix(b"8;") else {
            return;
        };
        let Ok(rest) = std::str::from_utf8(rest) else {
            return;
        };
        let Some((_, target)) = rest.split_once(';') else {
            return;
        };
        self.link = (!target.is_empty()).then(|| Arc::from(rest));
        self.cut_char();
    }

    fn start_picture(&mut self, body: Vec<u8>, osc: bool) {
        self.body = body;
        self.fits = true;
        self.scan = Scan::Body { osc };
    }

    fn take(&mut self, bytes: &[u8]) {
        if !self.fits {
            return;
        }
        if self.body.len() + bytes.len() > PICTURES {
            self.fits = false;
            self.body = Vec::new();
            return;
        }
        self.body.extend_from_slice(bytes);
    }

    fn drop_picture(&mut self) {
        self.body = Vec::new();
        self.scan = Scan::Ground;
    }

    fn keep_picture(&mut self, at: Spot) {
        let body = std::mem::take(&mut self.body);
        if !self.fits || body.is_empty() {
            return;
        }
        self.pictures_size += body.len();
        let kept = if at.alt {
            &mut self.alt
        } else {
            &mut self.normal
        };
        kept.pictures.push(Picture {
            at: at.row,
            col: at.col,
            bytes: Arc::from(body),
        });
        // The oldest go first, the normal screen's before the program's.
        while self.pictures_size > PICTURES {
            let kept = if self.normal.pictures.is_empty() {
                &mut self.alt
            } else {
                &mut self.normal
            };
            if kept.pictures.is_empty() {
                break;
            }
            let gone = kept.pictures.remove(0);
            self.pictures_size -= gone.bytes.len();
        }
    }

    fn recount_pictures(&mut self) {
        self.pictures_size = self
            .normal
            .pictures
            .iter()
            .chain(&self.alt.pictures)
            .map(|p| p.bytes.len())
            .sum();
    }

    /// `CSI J` at `at`: the pictures on the erased part of the display go.
    fn erase(&mut self, mode: u16, at: Spot) {
        let top = if at.alt { 0 } else { self.scrolled };
        let kept = if at.alt {
            &mut self.alt
        } else {
            &mut self.normal
        };
        let before = |p: &Picture| p.at < at.row || (p.at == at.row && p.col < at.col);
        kept.pictures.retain(|p| match mode {
            0 => before(p),
            1 => p.at < top || !(before(p) || (p.at == at.row && p.col == at.col)),
            2 => p.at < top,
            _ => true,
        });
        self.recount_pictures();
    }

    /// Starts counting the rows the normal screen scrolls (the module's
    /// note): the view held one row back, or none while the history is
    /// empty and its length is the count.
    fn arm(&mut self, parser: &mut vt100::Parser) {
        let s = parser.screen_mut();
        if s.alternate_screen() {
            self.armed = None;
            return;
        }
        s.set_scrollback(usize::MAX);
        let len = s.scrollback();
        s.set_scrollback(1);
        self.armed = Some((len, s.scrollback()));
    }

    /// Adds the rows scrolled since [`arm`](Self::arm) and puts the view
    /// back at the bottom. A count that may have run past what the view
    /// can show is not trusted: the normal screen's marks go.
    fn count(&mut self, parser: &mut vt100::Parser, keeps: usize) {
        let Some((len0, off0)) = self.armed.take() else {
            return;
        };
        let s = parser.screen_mut();
        if s.alternate_screen() {
            return;
        }
        let off = s.scrollback();
        s.set_scrollback(usize::MAX);
        let len = s.scrollback();
        s.set_scrollback(0);
        let (n, sure) = if off0 == 0 {
            (len.saturating_sub(len0), len < keeps)
        } else {
            (off.saturating_sub(off0), off < keeps)
        };
        self.scrolled += n as u64;
        if !sure {
            self.normal = Kept::default();
            self.recount_pictures();
        }
    }

    fn sync(&mut self, parser: &mut vt100::Parser, keeps: usize) {
        self.count(parser, keeps);
        self.arm(parser);
    }

    /// The count is in and the view at the bottom; the marks whose rows
    /// left the history go.
    fn disarm(&mut self, parser: &mut vt100::Parser, keeps: usize) {
        self.count(parser, keeps);
        let s = parser.screen_mut();
        if s.alternate_screen() {
            return;
        }
        s.set_scrollback(usize::MAX);
        let len = s.scrollback() as u64;
        s.set_scrollback(0);
        let first = self.scrolled.saturating_sub(len);
        self.normal.drop_before(first);
        let before = self.normal.pictures.len();
        self.normal.pictures.retain(|p| p.at >= first);
        if self.normal.pictures.len() != before {
            self.recount_pictures();
        }
    }

    /// Where the cursor is, the count brought up to now.
    fn spot(&mut self, parser: &mut vt100::Parser, keeps: usize) -> Spot {
        self.sync(parser, keeps);
        let s = parser.screen();
        let (row, col) = s.cursor_position();
        let alt = s.alternate_screen();
        Spot {
            row: if alt { 0 } else { self.scrolled } + row as u64,
            col,
            alt,
        }
    }

    /// A character under the link landed: the cell it took is the one
    /// the cursor passed over. One that took none joined the cell before
    /// it (a combining mark).
    fn printed(&mut self, parser: &mut vt100::Parser, keeps: usize, c: char) {
        let Some(a) = self.before.take() else { return };
        let b = self.spot(parser, keeps);
        let Some(link) = self.link.clone() else {
            return;
        };
        if a.alt != b.alt {
            return;
        }
        let kept = if b.alt {
            &mut self.alt
        } else {
            &mut self.normal
        };
        let (row, from, to) = if b.row == a.row && b.col > a.col {
            (a.row, a.col, b.col)
        } else if b.row == a.row + 1 && b.col > 0 && b.col <= 2 {
            // Wrapped: drawn from the next row's first column.
            (b.row, 0, b.col)
        } else {
            if b == a
                && let Some(run) = kept
                    .runs
                    .get_mut(&a.row)
                    .and_then(|r| r.iter_mut().rev().find(|r| r.to == a.col))
            {
                run.text.push(c);
            }
            return;
        };
        let runs = kept.runs.entry(row).or_default();
        match runs.last_mut() {
            Some(run) if run.to == from && Arc::ptr_eq(&run.link, &link) => {
                run.to = to;
                run.text.push(c);
            }
            _ => {
                runs.push(Run {
                    from,
                    to,
                    text: c.to_string(),
                    link,
                });
                kept.count += 1;
            }
        }
        if self.normal.count + self.alt.count > RUNS_MOST {
            let kept = if self.normal.runs.is_empty() {
                &mut self.alt
            } else {
                &mut self.normal
            };
            if let Some((_, gone)) = kept.runs.pop_first() {
                kept.count = kept.count.saturating_sub(gone.len());
            }
        }
    }

    /// The count of rows scrolled into the normal screen's history: its
    /// drawing row `r` is row `scrolled() + r`.
    pub fn scrolled(&self) -> u64 {
        self.scrolled
    }

    /// Whether any picture is drawn on the screen in view (`rows` rows
    /// from `top`), on the screen `alt` names.
    pub fn pictures_from(&self, alt: bool, top: u64) -> bool {
        let kept = if alt { &self.alt } else { &self.normal };
        kept.pictures.iter().any(|p| p.at >= top)
    }

    /// What absolute row `at` carries, read against `screen`'s visible
    /// row `row`: each link whose text is still on its cells, a later one
    /// over an earlier, and the pictures drawn there.
    pub fn marks(
        &self,
        alt: bool,
        at: u64,
        screen: &vt100::Screen,
        row: u16,
        cols: u16,
    ) -> RowMarks {
        let kept = if alt { &self.alt } else { &self.normal };
        let mut out = RowMarks::default();
        if let Some(runs) = kept.runs.get(&at) {
            for run in runs {
                if run.to > cols || !holds(screen, row, run) {
                    continue;
                }
                if out.links.is_empty() {
                    out.links = vec![None; cols as usize];
                }
                for c in run.from..run.to {
                    out.links[c as usize] = Some(run.link.clone());
                }
            }
        }
        out.pictures = kept
            .pictures
            .iter()
            .filter(|p| p.at == at)
            .map(|p| (p.col, p.bytes.clone()))
            .collect();
        out
    }

    /// ⌘K in the model: rows before `cut` are gone, and the cursor's row
    /// is the new first one.
    pub fn forget(&mut self, cut: u64) {
        self.normal.drop_before(cut);
        self.normal.pictures.retain(|p| p.at >= cut);
        self.scrolled = cut;
        self.recount_pictures();
    }

    /// The screen is `rows` high now: vt100 dropped the rows below, and
    /// the pictures drawn on them go with them.
    pub fn resize(&mut self, rows: u16) {
        let end = self.scrolled + rows as u64;
        self.normal.pictures.retain(|p| p.at < end);
        self.alt.pictures.retain(|p| p.at < rows as u64);
        self.recount_pictures();
    }
}

/// Whether the cells under `run` still read its text.
fn holds(screen: &vt100::Screen, row: u16, run: &Run) -> bool {
    let mut text = String::new();
    for c in run.from..run.to {
        match screen.cell(row, c) {
            Some(cell) if cell.is_wide_continuation() => {}
            Some(cell) => text.push_str(cell.contents()),
            None => return false,
        }
    }
    text == run.text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `chunks`, each through the table, to a fresh 24×80 model
    /// keeping `keeps` rows of history.
    fn fed(chunks: &[&[u8]], keeps: usize) -> (vt100::Parser, Beside) {
        let mut p = vt100::Parser::new(24, 80, keeps);
        let mut b = Beside::default();
        for c in chunks {
            b.feed(&mut p, c, keeps);
        }
        (p, b)
    }

    /// Every linked cell, as (absolute row, column, target), over the
    /// history and the screen.
    fn linked(p: &mut vt100::Parser, b: &Beside) -> Vec<(u64, u16, String)> {
        let s = p.screen_mut();
        s.set_scrollback(usize::MAX);
        let len = s.scrollback();
        let mut out = Vec::new();
        let mut read = |s: &vt100::Screen, row: u16, at: u64| {
            for (c, l) in b.marks(false, at, s, row, 80).links.iter().enumerate() {
                if let Some(l) = l {
                    out.push((at, c as u16, l.to_string()));
                }
            }
        };
        for h in 0..len {
            s.set_scrollback(len - h);
            read(s, 0, b.scrolled() - (len - h) as u64);
        }
        s.set_scrollback(0);
        for r in 0..24 {
            read(s, r, b.scrolled() + r as u64);
        }
        out
    }

    /// The count of rows scrolled is right below the history's cap, at
    /// it, from an empty history, and with the view held across chunks,
    /// and the view is at the bottom after every feed.
    #[test]
    fn rows_are_counted_through_the_history() {
        for keeps in [10usize, 1000] {
            let mut chunks: Vec<Vec<u8>> = Vec::new();
            for i in 0..300 {
                chunks.push(format!("line {i}\r\n").into_bytes());
            }
            let refs: Vec<&[u8]> = chunks.iter().map(Vec::as_slice).collect();
            let (mut p, b) = fed(&refs, keeps);
            assert_eq!(
                b.scrolled(),
                300 - 23,
                "keeps {keeps}: one row a line past the screen"
            );
            assert_eq!(p.screen().scrollback(), 0, "the view is at the bottom");
            let s = p.screen_mut();
            assert_eq!(s.cell(22, 0).unwrap().contents(), "l");
        }
    }

    /// A link is recorded on the cells its text took, whatever the chunk
    /// boundaries, and only while its text is still there.
    #[test]
    fn a_link_lands_on_its_cells_from_any_cut() {
        let whole = "$ \x1b]8;;file:///a\x1b\\a.rs\x1b]8;;\x1b\\ \x1b]8;id=1;https://x\x07\u{62a5}\u{544a}\x1b]8;;\x07\r\n$ ".as_bytes();
        let (mut p, b) = fed(&[whole], 100);
        let want = linked(&mut p, &b);
        assert_eq!(
            want,
            [
                (0, 2, ";file:///a".into()),
                (0, 3, ";file:///a".into()),
                (0, 4, ";file:///a".into()),
                (0, 5, ";file:///a".into()),
                (0, 7, "id=1;https://x".into()),
                (0, 8, "id=1;https://x".into()),
                (0, 9, "id=1;https://x".into()),
                (0, 10, "id=1;https://x".into())
            ]
        );
        for cut in 0..=whole.len() {
            let (mut p, b) = fed(&[&whole[..cut], &whole[cut..]], 100);
            assert_eq!(linked(&mut p, &b), want, "cut at {cut}");
        }
        // Overwritten: the cells read other text, the link is not handed out.
        let (mut p, b) = fed(&[whole, b"\x1b[1;1H\x1b[2Kother text here"], 100);
        assert!(linked(&mut p, &b).is_empty());
    }

    /// A link printed, then scrolled through a full history with a switch
    /// of screens inside the same chunk, keeps its row.
    #[test]
    fn a_switch_of_screens_inside_a_chunk_keeps_the_count() {
        let mut chunk = b"\x1b]8;;https://kept\x1b\\kept\x1b]8;;\x1b\\\r\n".to_vec();
        for i in 0..40 {
            chunk.extend_from_slice(format!("line {i}\r\n").as_bytes());
        }
        chunk.extend_from_slice(b"\x1b[?1049h\x1b[Hfull\x1b[?1049l");
        for i in 0..5 {
            chunk.extend_from_slice(format!("after {i}\r\n").as_bytes());
        }
        let (mut p, b) = fed(&[&chunk], 100);
        assert_eq!(b.scrolled(), 41 + 5 - 23);
        let got = linked(&mut p, &b);
        assert_eq!(got.len(), 4);
        assert!(
            got.iter()
                .all(|(row, _, l)| *row == 0 && l == ";https://kept"),
            "{got:?}"
        );
    }

    /// A picture keeps its row; `clear` takes the ones on the screen, the
    /// history's stay; a reset takes everything.
    #[test]
    fn pictures_go_with_the_display_erased_under_them() {
        let pic: &[u8] = b"\x1b]1337;File=inline=1:QQ==\x07";
        let mut s = b"one\r\n".to_vec();
        s.extend_from_slice(pic);
        for i in 0..30 {
            s.extend_from_slice(format!("\r\nline {i}").as_bytes());
        }
        s.extend_from_slice(b"\r\nshown\r\n");
        s.extend_from_slice(pic);
        let (_, b) = fed(&[&s], 100);
        assert_eq!(
            b.normal
                .pictures
                .iter()
                .map(|p| (p.at, p.col))
                .collect::<Vec<_>>(),
            [(1, 0), (33, 0)]
        );
        let (_, b) = fed(&[&s, b"\x1b[H\x1b[2J"], 100);
        assert_eq!(
            b.normal.pictures.iter().map(|p| p.at).collect::<Vec<_>>(),
            [1],
            "the screen's picture is cleared"
        );
        let (_, b) = fed(&[&s, b"\x1bc"], 100);
        assert!(
            b.normal.pictures.is_empty() && b.scrolled() == 0,
            "a reset takes everything"
        );
        // A picture cut across chunks anywhere is still one picture.
        for cut in 0..=s.len() {
            let (_, b) = fed(&[&s[..cut], &s[cut..]], 100);
            assert_eq!(b.normal.pictures.len(), 2, "cut at {cut}");
            assert!(b.normal.pictures.iter().all(|p| &*p.bytes == pic));
        }
    }
}
