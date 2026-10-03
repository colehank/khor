//! Pictures a program drew, kept beside the replay ring instead of in it.
//!
//! # Why
//!
//! A terminal picture is one long escape sequence: sixel
//! (`ESC P … q <data> ESC \`) or iTerm2's inline image
//! (`ESC ] 1337 ; File= … : <base64> BEL`), which is what `imgcat`,
//! `chafa`, `timg` and matplotlib's terminal backends write and what the
//! app's xterm draws (`@xterm/addon-image`, roadmap 3.3). In the host's
//! raw ring ([`crate::host`], 1 MiB) one plot is a large share of the
//! whole: a few pictures push out the text history a late attacher wants,
//! and a trim that falls inside one replays the rest of its base64 as
//! text on the screen.
//!
//! So the ring takes the text and this store takes the pictures, each
//! with the place in the stream it was drawn at, and a replay puts them
//! back there. The store has its own budget ([`PICTURES`]) and lets the
//! oldest go first, as xterm.js's image addon does with its
//! `storageLimit` and tmux 3.4 does with the sixel images it keeps beside
//! its grid (`image.c`). A picture whose text has left the ring leaves
//! with it: it belonged to that block of output. The live stream is
//! untouched; this only decides what an attacher is handed.
//!
//! # What is read
//!
//! Just enough grammar to find those two sequences and their ends: a DCS
//! whose parameters are digits and `;` and whose final byte is `q`, and an
//! OSC that starts `1337;File=`. Every other string (a title, DECRQSS,
//! tmux's passthrough, a colour query) stays in the text, byte for byte.

use std::collections::VecDeque;

/// The replay's picture budget, per session: four rings' worth. A
/// picture bigger than this is drawn live and not replayed.
pub const PICTURES: usize = 4 << 20;

/// A DCS or OSC still undecided after this many bytes is not a picture.
const INTRO: usize = 64;

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a;
const IIP: &[u8] = b"\x1b]1337;File=";

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Text,
    /// An ESC, waiting for what it starts.
    Esc,
    /// `ESC P` and parameters: a sixel if `q` comes next.
    Dcs,
    /// `ESC ]` and some of `1337;File=`.
    Osc,
    /// Inside a picture.
    Body { osc: bool },
    /// An ESC inside a picture: `\` ends it, anything else breaks it.
    BodyEsc { osc: bool },
}

/// The pictures a session drew, and the one being drawn.
#[derive(Default)]
pub struct Pictures {
    state: State,
    /// The bytes since an ESC that may yet open a picture.
    held: Vec<u8>,
    /// The picture being read, from its ESC on.
    body: Vec<u8>,
    /// How long its introducer was: what is kept of it once it is
    /// too big to replay, so an attacher in the middle of it still
    /// reads its tail as a picture's, not as text.
    intro: usize,
    /// Whether it still fits the budget.
    fits: bool,
    /// Each finished picture, at the stream position it was drawn at.
    kept: VecDeque<(u64, Vec<u8>)>,
    size: usize,
}

impl Pictures {
    /// Sorts `bytes` into text, appended to `text`, and pictures, kept
    /// here. `end` is the stream position of the ring's last text byte
    /// plus one, before this chunk.
    pub fn sort(&mut self, bytes: &[u8], end: u64, text: &mut Vec<u8>) {
        let mut i = 0;
        while i < bytes.len() {
            match self.state {
                State::Text => {
                    // The common case: a run with no ESC in it, copied whole.
                    let run = bytes[i..]
                        .iter()
                        .position(|&b| b == ESC)
                        .unwrap_or(bytes.len() - i);
                    text.extend_from_slice(&bytes[i..i + run]);
                    i += run;
                    if i < bytes.len() {
                        self.held.push(ESC);
                        self.state = State::Esc;
                        i += 1;
                    }
                }
                State::Body { osc } => {
                    let run = bytes[i..]
                        .iter()
                        .position(|&b| b == ESC || b == CAN || b == SUB || (osc && b == BEL))
                        .unwrap_or(bytes.len() - i);
                    self.take(&bytes[i..i + run]);
                    i += run;
                    if i < bytes.len() {
                        let b = bytes[i];
                        i += 1;
                        match b {
                            ESC => self.state = State::BodyEsc { osc },
                            BEL => {
                                self.take(&[BEL]);
                                self.finish(end + text.len() as u64);
                            }
                            // CAN and SUB cancel the string: nothing drawn.
                            _ => self.drop_body(),
                        }
                    }
                }
                state => {
                    let b = bytes[i];
                    i += 1;
                    self.step(state, b, end, text);
                }
            }
        }
    }

    fn step(&mut self, state: State, b: u8, end: u64, text: &mut Vec<u8>) {
        match state {
            State::Esc => {
                self.held.push(b);
                match b {
                    b'P' => self.state = State::Dcs,
                    b']' => self.state = State::Osc,
                    _ => self.give_up(text),
                }
            }
            State::Dcs => {
                self.held.push(b);
                if b == b'q' {
                    self.open();
                } else if !(b.is_ascii_digit() || b == b';') || self.held.len() > INTRO {
                    self.give_up(text);
                }
            }
            State::Osc => {
                self.held.push(b);
                if !IIP.starts_with(&self.held) {
                    self.give_up(text);
                } else if self.held.len() == IIP.len() {
                    self.open();
                }
            }
            State::BodyEsc { .. } => {
                if b == b'\\' {
                    self.take(&[ESC, b'\\']);
                    self.finish(end + text.len() as u64);
                } else {
                    // An ESC that does not end the string starts the next
                    // sequence, and the picture is abandoned.
                    self.drop_body();
                    self.held.push(ESC);
                    self.state = State::Esc;
                    self.step(State::Esc, b, end, text);
                }
            }
            State::Text | State::Body { .. } => unreachable!("handled in `sort`"),
        }
    }

    /// Not a picture after all: what was held is text. An ESC that ended
    /// it may start one, so it is held again.
    fn give_up(&mut self, text: &mut Vec<u8>) {
        let last = self.held.pop();
        text.extend_from_slice(&self.held);
        self.held.clear();
        self.state = State::Text;
        match last {
            Some(ESC) => {
                self.held.push(ESC);
                self.state = State::Esc;
            }
            Some(b) => text.push(b),
            None => {}
        }
    }

    fn open(&mut self) {
        self.state = State::Body {
            osc: self.held[1] == b']',
        };
        self.body = std::mem::take(&mut self.held);
        self.intro = self.body.len();
        self.fits = true;
    }

    fn take(&mut self, bytes: &[u8]) {
        if !self.fits {
            return;
        }
        if self.body.len() + bytes.len() > PICTURES {
            self.fits = false;
            self.body.truncate(self.intro);
            return;
        }
        self.body.extend_from_slice(bytes);
    }

    fn finish(&mut self, at: u64) {
        let body = std::mem::take(&mut self.body);
        self.state = State::Text;
        if !self.fits {
            return;
        }
        self.size += body.len();
        self.kept.push_back((at, body));
        while self.size > PICTURES {
            let Some((_, gone)) = self.kept.pop_front() else {
                break;
            };
            self.size -= gone.len();
        }
    }

    fn drop_body(&mut self) {
        self.body.clear();
        self.state = State::Text;
    }

    /// The ring trimmed its text up to stream position `gone`: the
    /// pictures drawn before it go too.
    pub fn trim(&mut self, gone: u64) {
        while self.kept.front().is_some_and(|(at, _)| *at < gone) {
            let (_, p) = self.kept.pop_front().unwrap();
            self.size -= p.len();
        }
    }

    /// The ring's text (which starts at stream position `gone`) with the
    /// pictures back in place, then whatever is part-way through: an
    /// attacher's parser must be in the same string the live bytes
    /// continue.
    pub fn splice(&self, text: &VecDeque<u8>, gone: u64, out: &mut Vec<u8>) {
        let mut from = 0;
        for (at, p) in &self.kept {
            let to = ((at - gone) as usize).min(text.len());
            out.extend(text.range(from..to));
            out.extend_from_slice(p);
            from = to;
        }
        out.extend(text.range(from..));
        match self.state {
            State::Text => {}
            State::Esc | State::Dcs | State::Osc => out.extend_from_slice(&self.held),
            State::Body { .. } => out.extend_from_slice(&self.body),
            State::BodyEsc { .. } => {
                out.extend_from_slice(&self.body);
                out.push(ESC);
            }
        }
    }

    /// The bytes held for pictures (for the tests).
    #[cfg(test)]
    fn held_bytes(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIXEL: &[u8] = b"\x1bP0;1;0q\"1;1;4;4#0;2;100;0;0#0~~~~-\x1b\\";
    const IIP_BEL: &[u8] = b"\x1b]1337;File=inline=1;size=4:AAAA\x07";
    const IIP_ST: &[u8] = b"\x1b]1337;File=name=eC5wbmc=;inline=1:QUJD\x1b\\";

    /// Feeds `chunks` and returns (text, replay).
    fn run(chunks: &[&[u8]]) -> (Vec<u8>, Vec<u8>) {
        let mut p = Pictures::default();
        let mut text = VecDeque::new();
        for c in chunks {
            let mut t = Vec::new();
            p.sort(c, text.len() as u64, &mut t);
            text.extend(t);
        }
        let mut out = Vec::new();
        p.splice(&text, 0, &mut out);
        (text.into_iter().collect(), out)
    }

    fn stream() -> Vec<u8> {
        [
            b"$ imgcat a.png\r\n".as_slice(),
            IIP_BEL,
            b"\r\n$ chafa b\r\n",
            SIXEL,
            b"\r\n",
            IIP_ST,
            b"$ ",
        ]
        .concat()
    }

    /// The text keeps none of a picture; the replay is the stream itself.
    #[test]
    fn pictures_leave_the_text_and_come_back_in_place() {
        let whole = stream();
        let (text, replay) = run(&[&whole]);
        assert_eq!(text, b"$ imgcat a.png\r\n\r\n$ chafa b\r\n\r\n$ ");
        assert_eq!(replay, whole);
    }

    /// The PTY hands over whatever a read returned: a cut anywhere,
    /// even between an ESC and its `\`, reads the same.
    #[test]
    fn a_chunk_boundary_anywhere_changes_nothing() {
        let whole = stream();
        let (text, _) = run(&[&whole]);
        for cut in 0..=whole.len() {
            let (t, r) = run(&[&whole[..cut], &whole[cut..]]);
            assert_eq!(t, text, "cut at {cut}");
            assert_eq!(r, whole, "cut at {cut}");
        }
        let bytes: Vec<&[u8]> = whole.chunks(1).collect();
        assert_eq!(run(&bytes).1, whole);
    }

    /// The control: strings that are not pictures stay text, byte for
    /// byte — a title, DECRQSS, a colour query, tmux's passthrough, a
    /// sixel-looking DCS with a letter in its parameters, and an OSC
    /// 1337 that is not a file.
    #[test]
    fn other_strings_stay_in_the_text() {
        let s: &[u8] = b"\x1b]0;title\x07\x1bP$qm\x1b\\\x1b]11;?\x1b\\\x1bPtmux;\x1b\x1b[1m\x1b\\\x1bP1$r0m\x1b\\\x1b]1337;SetMark\x07\x1b[31mred\x1b\x1b[0m";
        let (text, replay) = run(&[s]);
        assert_eq!(text, s);
        assert_eq!(replay, s);
    }

    /// CAN cancels a picture and an ESC that is not ST breaks it: nothing
    /// kept, and the sequence the ESC starts is read as usual.
    #[test]
    fn a_broken_picture_is_dropped_and_what_follows_is_read() {
        let (text, replay) = run(&[b"a\x1bPq#0~~\x18b\x1b]1337;File=:QQ\x1b[1mc"]);
        assert_eq!(text, b"ab\x1b[1mc");
        assert_eq!(replay, b"ab\x1b[1mc");
    }

    /// The budget: the oldest picture goes first, the newest stays.
    #[test]
    fn the_oldest_picture_goes_past_the_budget() {
        let big = |n: u8| {
            [
                b"\x1b]1337;File=inline=1:".as_slice(),
                &vec![b'A' + n; PICTURES / 3],
                b"\x07",
            ]
            .concat()
        };
        let mut p = Pictures::default();
        let mut t = Vec::new();
        for n in 0..4 {
            p.sort(&big(n), t.len() as u64, &mut t);
            t.push(b'x');
        }
        assert!(p.held_bytes() <= PICTURES);
        let mut out = Vec::new();
        p.splice(&t.iter().copied().collect(), 0, &mut out);
        assert!(
            !out.windows(8).any(|w| w == [b'A'; 8]),
            "the oldest is gone"
        );
        assert!(out.windows(8).any(|w| w == [b'D'; 8]), "the newest stays");
    }

    /// One picture over the whole budget is not kept, and while it is
    /// still arriving an attacher gets its introducer, so the live tail
    /// lands inside a string and not on the screen.
    #[test]
    fn a_picture_over_the_budget_is_not_kept_but_its_start_is() {
        let mut p = Pictures::default();
        let mut t = Vec::new();
        p.sort(b"\x1bPq", 0, &mut t);
        p.sort(&vec![b'~'; PICTURES + 1], 0, &mut t);
        let mut out = Vec::new();
        p.splice(&VecDeque::new(), 0, &mut out);
        assert_eq!(out, b"\x1bPq");
        p.sort(b"\x1b\\after", 0, &mut t);
        assert_eq!(t, b"after");
        assert_eq!(p.held_bytes(), 0);
    }

    /// A picture whose text the ring let go of goes with it.
    #[test]
    fn a_picture_leaves_with_its_text() {
        let mut p = Pictures::default();
        let mut t = Vec::new();
        p.sort(&[b"one", SIXEL, b"two", IIP_BEL].concat(), 0, &mut t);
        assert_eq!(t, b"onetwo");
        p.trim(4);
        let held: VecDeque<u8> = t[4..].iter().copied().collect();
        let mut out = Vec::new();
        p.splice(&held, 4, &mut out);
        assert_eq!(out, [b"wo".as_slice(), IIP_BEL].concat());
    }
}
