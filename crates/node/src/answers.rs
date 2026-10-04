//! **The host's ledger of the questions a program asked its terminal**
//! (T-4/T-5 of the terminal audit, `audit/term.md`, spec rules 4 and 5).
//!
//! A program asks its terminal things in its output — what kind of
//! terminal it is (DA1, DA2), its status (DSR 5), its colours (OSC 10, 11,
//! 12, 4 with `?`), a setting (DECRQSS), a mode (DECRQM) — and waits for
//! the answer on its input. khor's terminals are faces (xterm.js) that each
//! answer on their own, so with two faces attached every question got two
//! answers, and a face hidden for seconds answered seconds late, into a
//! program long done waiting (`probe-two.jsonl`, `probe-hidden.jsonl`).
//!
//! The ledger sits where both directions pass, the way tmux keeps its own
//! (input.c `input_add_request` sends a forwarded query to one client and
//! matches its reply): the pty-read loop tells it every query it sees
//! ([`Ledger::saw_output`]); input from every face goes through
//! [`Ledger::take_input`], which lets an answer through only while a
//! matching question is pending and younger than [`ANSWER_MAX_AGE`] — the
//! first face to answer wins, a copy or a late answer is dropped. Keys are
//! never touched: only a byte run that has the exact shape of an answer is
//! weighed, and a cursor-position report (CPR) always passes, because
//! ⌃F3 has its shape and its question needs a face anyway.
//!
//! **No face answering** (every face hidden or suspended): after
//! [`FALLBACK_AFTER`] the host answers the questions that have a fixed
//! answer itself, as tmux does from its own state — DA1, DA2, DSR 5 — and
//! the colours from the last answer a face gave ([`Ledger::fallback`]).
//!
//! **Departure from the audit's spec, rule 3** (a new `ClientOp::Answer`):
//! answers stay in `Input` and are told apart by shape here. The frames
//! are MessagePack arrays and enum indices; a host already deployed on a
//! fleet machine fails to decode an op it does not know, and its input
//! loop ends — exactly T-16's failure, on every machine not yet upgraded.
//! The shape test the face already uses (`term-replies.ts`) is the same one
//! this ledger uses, and it costs no wire change.
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// How long a question waits for an answer before the host gives up on
/// the faces' answer (spec rule 4).
pub const ANSWER_MAX_AGE: Duration = Duration::from_secs(2);
/// How long the host waits for a face before answering itself (rule 5).
pub const FALLBACK_AFTER: Duration = Duration::from_millis(150);

/// What a question asks, and so which answer settles it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    Da1,
    Da2,
    Dsr5,
    /// OSC 10 / 11 / 12 / 4 with `?`.
    Colour(u16),
    Decrqss,
    Decrqm,
    /// The kitty keyboard protocol's flags, `CSI ? u` (roadmap 5.1; the
    /// faces keep them, `lib/term-kitty`).
    KittyFlags,
    /// A window report (xterm's `CSI Ps t`, the audit's spec rule 4), by
    /// the number its answer leads with: 14 → 4 (the text area in
    /// pixels), 16 → 6 (a cell), 18 → 8 (the text area in cells), 19 → 9,
    /// 13 → 3. tmux asks 14/16/18 of its outer terminal at every attach
    /// and resize; with two faces both answered, and tmux typed the
    /// second answer into the pane (the tmux walk, 2026-10-04).
    Window(u8),
}

/// The answer a window-report question gets, by its first parameter.
fn window_answer(question: u16) -> Option<u8> {
    match question {
        13 => Some(3),
        14 => Some(4),
        16 => Some(6),
        18 => Some(8),
        19 => Some(9),
        _ => None,
    }
}

/// A CSI's first numeric parameter.
fn first_param(params: &[u8]) -> Option<u16> {
    let end = params.iter().position(|&c| c == b';').unwrap_or(params.len());
    std::str::from_utf8(&params[..end]).ok()?.parse().ok()
}

/// One escape sequence at the start of `b`, and how long it is; `None`
/// when `b` does not start with a complete one.
fn sequence(b: &[u8]) -> Option<(Seq<'_>, usize)> {
    if b.len() < 2 || b[0] != 0x1b {
        return None;
    }
    match b[1] {
        b'[' => {
            let mut i = 2;
            while i < b.len() && (0x30..=0x3f).contains(&b[i]) {
                i += 1;
            }
            let params = &b[2..i];
            let inter_start = i;
            while i < b.len() && (0x20..=0x2f).contains(&b[i]) {
                i += 1;
            }
            let inter = &b[inter_start..i];
            let fin = *b.get(i)?;
            if !(0x40..=0x7e).contains(&fin) {
                return None;
            }
            Some((Seq::Csi { params, inter, fin }, i + 1))
        }
        b']' | b'P' => {
            // To BEL (OSC only) or ST.
            let mut i = 2;
            while i < b.len() {
                if b[1] == b']' && b[i] == 0x07 {
                    return Some((Seq::String { kind: b[1], body: &b[2..i] }, i + 1));
                }
                if b[i] == 0x1b {
                    return (b.get(i + 1) == Some(&b'\\')).then_some((Seq::String { kind: b[1], body: &b[2..i] }, i + 2));
                }
                i += 1;
            }
            None
        }
        _ => None,
    }
}

enum Seq<'a> {
    Csi { params: &'a [u8], inter: &'a [u8], fin: u8 },
    /// OSC (`]`) or DCS (`P`) with its body.
    String { kind: u8, body: &'a [u8] },
}

/// The question a sequence in a program's output asks, if it asks one a
/// face answers.
fn question(s: &Seq<'_>) -> Option<Kind> {
    match s {
        Seq::Csi { params, inter, fin } => match (*fin, *inter) {
            (b'c', b"") if params.is_empty() || *params == b"0" => Some(Kind::Da1),
            (b'c', b"") if params.first() == Some(&b'>') => Some(Kind::Da2),
            (b'n', b"") if *params == b"5" => Some(Kind::Dsr5),
            (b'p', b"$") => Some(Kind::Decrqm),
            (b'u', b"") if *params == b"?" => Some(Kind::KittyFlags),
            (b't', b"") => window_answer(first_param(params)?).map(Kind::Window),
            _ => None,
        },
        Seq::String { kind: b']', body } => {
            let (ps, rest) = split_ps(body)?;
            match ps {
                10..=12 if rest == b"?" => Some(Kind::Colour(ps)),
                4 if rest.ends_with(b";?") => Some(Kind::Colour(4)),
                _ => None,
            }
        }
        Seq::String { kind: b'P', body } if body.starts_with(b"$q") => Some(Kind::Decrqss),
        _ => None,
    }
}

/// The answer a sequence in a face's input is, if it is one the ledger
/// weighs. CPR is not weighed (see the head).
fn answer(s: &Seq<'_>) -> Option<Kind> {
    match s {
        Seq::Csi { params, inter, fin } => match (*fin, *inter) {
            (b'c', b"") if params.first() == Some(&b'?') => Some(Kind::Da1),
            (b'c', b"") if params.first() == Some(&b'>') => Some(Kind::Da2),
            (b'n', b"") if *params == b"0" || *params == b"3" => Some(Kind::Dsr5),
            (b'y', b"$") => Some(Kind::Decrqm),
            // `CSI ? flags u`; a key in that protocol never has the `?`.
            (b'u', b"") if params.len() > 1 && params[0] == b'?' && params[1..].iter().all(u8::is_ascii_digit) => {
                Some(Kind::KittyFlags)
            }
            // `CSI 4 ; h ; w t` and its kin: three parameters, the first the
            // report's own number. A key never ends in `t`.
            (b't', b"") if params.iter().filter(|&&c| c == b';').count() == 2 => match first_param(params)? {
                n @ (3 | 4 | 6 | 8 | 9) => Some(Kind::Window(n as u8)),
                _ => None,
            },
            _ => None,
        },
        Seq::String { kind: b']', body } => {
            let (ps, rest) = split_ps(body)?;
            match ps {
                10..=12 | 4 if rest != b"?" => Some(Kind::Colour(ps)),
                _ => None,
            }
        }
        Seq::String { kind: b'P', body } if body.starts_with(b"1$r") || body.starts_with(b"0$r") => Some(Kind::Decrqss),
        _ => None,
    }
}

fn split_ps(body: &[u8]) -> Option<(u16, &[u8])> {
    let semi = body.iter().position(|&c| c == b';')?;
    let ps = std::str::from_utf8(&body[..semi]).ok()?.parse().ok()?;
    Some((ps, &body[semi + 1..]))
}

#[derive(Default)]
pub struct Ledger {
    pending: VecDeque<(Kind, Instant)>,
    /// The unfinished end of the last output read, so a question split
    /// across two reads is still seen.
    tail: Vec<u8>,
    /// The last colour answers a face gave, the host's fallback for them.
    colours: HashMap<u16, Vec<u8>>,
}

/// The most a question can be split across reads and still be seen.
const TAIL_MAX: usize = 256;

impl Ledger {
    /// Records the questions in a read of the program's output.
    pub fn saw_output(&mut self, bytes: &[u8], now: Instant) {
        let mut buf = std::mem::take(&mut self.tail);
        buf.extend_from_slice(bytes);
        let mut i = 0;
        while i < buf.len() {
            if buf[i] != 0x1b {
                i += 1;
                continue;
            }
            match sequence(&buf[i..]) {
                Some((s, n)) => {
                    if let Some(k) = question(&s) {
                        self.pending.push_back((k, now));
                    }
                    i += n;
                }
                None if buf.len() - i <= TAIL_MAX && unfinished(&buf[i..]) => {
                    self.tail = buf[i..].to_vec();
                    break;
                }
                None => i += 1,
            }
        }
        self.expire(now);
    }

    /// What of a face's input reaches the pty: keys whole, an answer only
    /// while its question is pending.
    pub fn take_input(&mut self, bytes: &[u8], now: Instant) -> Vec<u8> {
        self.expire(now);
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b
                && let Some((s, n)) = sequence(&bytes[i..])
                && let Some(k) = answer(&s)
            {
                if let Some(at) = self.pending.iter().position(|(q, _)| *q == k) {
                    self.pending.remove(at);
                    if let Kind::Colour(ps) = k {
                        self.colours.insert(ps, bytes[i..i + n].to_vec());
                    }
                    out.extend_from_slice(&bytes[i..i + n]);
                }
                i += n;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        }
        out
    }

    /// The host's own answers for questions no face answered within
    /// [`FALLBACK_AFTER`]; the questions it cannot answer wait out their
    /// age and are dropped.
    pub fn fallback(&mut self, now: Instant) -> Vec<u8> {
        self.expire(now);
        let mut out = Vec::new();
        let mut keep = VecDeque::new();
        while let Some((k, at)) = self.pending.pop_front() {
            if now.duration_since(at) < FALLBACK_AFTER {
                keep.push_back((k, at));
                continue;
            }
            let answer: Option<Vec<u8>> = match k {
                // xterm.js's own: VT100 with advanced video (xterm 6,
                // InputHandler.ts `sendDeviceAttributesPrimary`).
                Kind::Da1 => Some(b"\x1b[?1;2c".to_vec()),
                // …and its DA2 (`sendDeviceAttributesSecondary`).
                Kind::Da2 => Some(b"\x1b[>0;276;0c".to_vec()),
                Kind::Dsr5 => Some(b"\x1b[0n".to_vec()),
                Kind::Colour(ps) => self.colours.get(&ps).cloned(),
                // The flags are the faces'; a program that hears only
                // its DA1 sentinel concludes there is no protocol, which
                // is true of a terminal nobody is looking at.
                Kind::Decrqss | Kind::Decrqm | Kind::KittyFlags | Kind::Window(_) => None,
            };
            match answer {
                Some(a) => out.extend_from_slice(&a),
                None => keep.push_back((k, at)),
            }
        }
        self.pending = keep;
        out
    }

    fn expire(&mut self, now: Instant) {
        self.pending.retain(|(_, at)| now.duration_since(*at) <= ANSWER_MAX_AGE);
    }
}

/// Whether `b`, starting at an ESC, could still become a sequence.
fn unfinished(b: &[u8]) -> bool {
    match b.get(1) {
        None => true,
        Some(b'[') => b[2..].iter().all(|&c| (0x20..=0x3f).contains(&c)),
        Some(b']') | Some(b'P') => !b[2..].contains(&0x07) && !b[2..].windows(2).any(|w| w == b"\x1b\\"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t0() -> Instant {
        Instant::now()
    }

    /// T-4: two faces answer one question; the pty gets one answer.
    /// The kitty keyboard protocol's question (roadmap 5.1): the first
    /// face's flags reach the program, the second face's copy does not,
    /// and the protocol's keys - `CSI 13;2u`, `CSI 27u` - are never
    /// weighed as answers.
    /// **tmux's window reports, answered once** (the tmux walk): tmux asks
    /// `CSI 14 t`, `16 t`, `18 t` at an attach; two faces answered each,
    /// and tmux typed the copies into the pane as `;46;115t…`. Now the
    /// first face's answers pass and the second's are dropped; a report
    /// nobody asked for is dropped too; a window manipulation (`CSI 8 ; r
    /// ; c t` sent *by* a program) is output, never weighed.
    #[test]
    fn window_reports_are_answered_once() {
        let t = Instant::now();
        let mut l = Ledger::default();
        l.saw_output(b"\x1b[14t\x1b[16t\x1b[18t", t);
        let first = b"\x1b[4;736;920t\x1b[6;16;8t\x1b[8;46;115t";
        assert_eq!(l.take_input(first, t), first);
        assert_eq!(l.take_input(b"\x1b[4;752;384t\x1b[6;16;8t\x1b[8;47;48t", t), b"", "the second face's");
        assert_eq!(l.take_input(b"\x1b[8;24;80t", t), b"", "nothing pending");
        assert_eq!(l.take_input(b"abc\x1b[A", t), b"abc\x1b[A", "keys pass");
    }

    #[test]
    fn kitty_flags_are_answered_once_and_its_keys_pass() {
        let mut l = Ledger::default();
        let t = Instant::now();
        l.saw_output(b"\x1b[>1u\x1b[?u\x1b[c", t);
        assert_eq!(l.take_input(b"\x1b[?1u", t), b"\x1b[?1u");
        assert_eq!(l.take_input(b"\x1b[?1u", t), b"", "the second face's copy");
        assert_eq!(l.take_input(b"\x1b[13;2u\x1b[27u\x1b[105;5u", t), b"\x1b[13;2u\x1b[27u\x1b[105;5u");
        // No face: the DA1 sentinel is answered, the flags are not.
        let mut l = Ledger::default();
        l.saw_output(b"\x1b[?u\x1b[c", t);
        assert_eq!(l.fallback(t + FALLBACK_AFTER), b"\x1b[?1;2c");
    }

    #[test]
    fn the_first_answer_wins_and_the_copy_is_dropped() {
        let now = t0();
        let mut l = Ledger::default();
        l.saw_output(b"probe\x1b]11;?\x1b\\\x1b[c", now);
        let a = l.take_input(b"\x1b]11;rgb:0000/0000/0000\x1b\\\x1b[?1;2c", now);
        let b = l.take_input(b"\x1b]11;rgb:0000/0000/0000\x1b\\\x1b[?1;2c", now);
        assert_eq!(a, b"\x1b]11;rgb:0000/0000/0000\x1b\\\x1b[?1;2c");
        assert!(b.is_empty(), "the second face's copy reached the pty: {b:?}");
    }

    /// T-5: an answer older than its question's patience is dropped.
    #[test]
    fn a_late_answer_is_dropped() {
        let now = t0();
        let mut l = Ledger::default();
        l.saw_output(b"\x1b[5n", now);
        let late = now + ANSWER_MAX_AGE + Duration::from_millis(10);
        assert!(l.take_input(b"\x1b[0n", late).is_empty());
    }

    /// Keys are never weighed, around an answer or alone.
    #[test]
    fn keys_pass_whole() {
        let now = t0();
        let mut l = Ledger::default();
        for key in [&b"ls -la\r"[..], b"\x1b[A", b"\x1b[1;5D", b"\x1bOP", b"\x1b[15~", b"\x1b", b"\x1bx", b"\x1b[200~text\x1b[201~", b"\x1b[<0;10;5M"] {
            assert_eq!(l.take_input(key, now), key);
        }
        l.saw_output(b"\x1b[c", now);
        assert_eq!(l.take_input(b"a\x1b[?1;2cb", now), b"a\x1b[?1;2cb");
        // An answer with nothing asked is dropped, the keys round it kept.
        assert_eq!(l.take_input(b"a\x1b[?1;2cb", now), b"ab");
        // CPR passes, asked or not (⌃F3 has its shape).
        assert_eq!(l.take_input(b"\x1b[1;5R", now), b"\x1b[1;5R");
    }

    /// Rule 5: nobody answers; the host does, for what has a fixed answer.
    #[test]
    fn the_host_answers_when_no_face_does() {
        let now = t0();
        let mut l = Ledger::default();
        l.saw_output(b"\x1b[c\x1b[>c\x1b[?2004$p", now);
        assert!(l.fallback(now).is_empty(), "the host answered before the faces had their chance");
        let later = now + FALLBACK_AFTER;
        assert_eq!(l.fallback(later), b"\x1b[?1;2c\x1b[>0;276;0c");
        // The face's answer, after the host's, is a copy.
        assert!(l.take_input(b"\x1b[?1;2c", later).is_empty());
    }

    /// Colours fall back to the last answer a face gave.
    #[test]
    fn colours_fall_back_to_the_last_face_answer() {
        let now = t0();
        let mut l = Ledger::default();
        l.saw_output(b"\x1b]11;?\x07", now);
        l.take_input(b"\x1b]11;rgb:f4f4/f4f4/efef\x1b\\", now);
        l.saw_output(b"\x1b]11;?\x07", now);
        assert_eq!(l.fallback(now + FALLBACK_AFTER), b"\x1b]11;rgb:f4f4/f4f4/efef\x1b\\");
    }

    /// A question split across two reads of the pty is still seen.
    #[test]
    fn a_question_split_across_reads_is_seen() {
        let now = t0();
        let mut l = Ledger::default();
        l.saw_output(b"text\x1b]1", now);
        l.saw_output(b"1;?\x1b\\more", now);
        assert_eq!(l.take_input(b"\x1b]11;rgb:0/0/0\x1b\\", now), b"\x1b]11;rgb:0/0/0\x1b\\");
    }
}
