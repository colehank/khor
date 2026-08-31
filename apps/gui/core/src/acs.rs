//! DEC Special Graphics, translated into the characters a terminal
//! actually draws (docs/handoff 终端画屏).
//!
//! # Why khor needs this at all
//!
//! `vt100` 0.16.2 does not implement SCS: `ESC ( 0` reaches its
//! `esc_dispatch` with `(` as an intermediate and goes straight to
//! `unhandled_escape`. Nothing after it is translated, so a program
//! drawing a box gets a screen of literal `l q k x j` where the person
//! expects `┌ ─ ┐ │ ┘`.
//!
//! **This is not a corner case, it is ncurses' default.** `TERM` here
//! is `xterm-256color` (the host says so — `khor_node::host`), whose
//! terminfo `acsc` maps line drawing onto exactly this charset, so
//! every ncurses program with a border — htop, dialog, mc, a vim
//! window split — draws through it. It is a *different* bug from the
//! locale one that made a tmux client fall back here (`user_locale`):
//! that one was tmux choosing ACS because it could not encode UTF-8,
//! this one is every other program choosing ACS because that is what
//! its terminfo says. Fixing the locale did not touch it.
//!
//! # Why a byte filter rather than a patch to the parser
//!
//! The translation is defined on the byte stream, before any of it
//! becomes cells, and it is exactly what a terminal does. Doing it
//! here keeps `vt100` a dependency rather than a fork.
//!
//! **It cannot corrupt UTF-8**, and not by care — by arithmetic: the
//! only bytes it ever rewrites are `0x5F..=0x7E`, and every byte of a
//! multi-byte UTF-8 sequence is `>= 0x80`. The one thing it must get
//! right is not translating inside an escape sequence, which is what
//! the state machine below is for.
//!
//! # The table does not rot
//!
//! It is DEC's, fixed since 1978 and identical in every terminal that
//! implements it — the opposite of the "list of likely directories"
//! this repo refuses elsewhere. A wrong entry here would be visible
//! the moment anything drew a box.

/// `0x5F..=0x7E` in the DEC Special Graphics set, in order.
const GRAPHICS: [char; 32] = [
    ' ',      // _ blank
    '◆',      // `
    '▒',      // a
    '\u{2409}', // b HT
    '\u{240C}', // c FF
    '\u{240D}', // d CR
    '\u{240A}', // e LF
    '°',      // f
    '±',      // g
    '\u{2424}', // h NL
    '\u{240B}', // i VT
    '┘',      // j
    '┐',      // k
    '┌',      // l
    '└',      // m
    '┼',      // n
    '⎺',      // o
    '⎻',      // p
    '─',      // q
    '⎼',      // r
    '⎽',      // s
    '├',      // t
    '┤',      // u
    '┴',      // v
    '┬',      // w
    '│',      // x
    '≤',      // y
    '≥',      // z
    'π',      // {
    '≠',      // |
    '£',      // }
    '·',      // ~
];

#[derive(Clone, Copy, PartialEq)]
enum At {
    /// Ordinary text, where translation happens.
    Ground,
    /// Just saw `ESC`.
    Esc,
    /// Saw `ESC` and a charset designator (`(`, `)`, `*`, `+`); the
    /// next byte names the set.
    Designate(usize),
    /// Inside `ESC [ … final` — passed through untouched.
    Csi,
    /// Inside a string sequence (OSC, DCS, SOS, PM, APC) — passed
    /// through until its terminator.
    Str,
    /// Inside a string sequence, having just seen `ESC` (so the next
    /// byte decides whether that was the `ST` terminator).
    StrEsc,
}

/// Feeds bytes through, rewriting the graphics range while a graphics
/// set is mapped in.
///
/// **One per terminal, and it must live as long as the screen does**:
/// `ESC ( 0` is a mode, and a program may set it once and draw
/// through it across many reads. A filter rebuilt per read would
/// translate the first chunk and pass the rest through as `q`s.
pub struct Acs {
    at: At,
    /// Whether each of G0–G3 currently holds the graphics set.
    graphics: [bool; 4],
    /// Which of G0/G1 is invoked — `SI` selects G0, `SO` selects G1.
    invoked: usize,
}

impl Default for Acs {
    fn default() -> Self {
        Acs { at: At::Ground, graphics: [false; 4], invoked: 0 }
    }
}

impl Acs {
    /// The bytes to hand the parser. Returns borrowed input untouched
    /// in the common case — **no graphics set is mapped in and the
    /// chunk holds no escape** — so a terminal that never draws a box
    /// pays a scan and no allocation.
    pub fn filter<'a>(&mut self, bytes: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        if self.at == At::Ground
            && !self.graphics.iter().any(|g| *g)
            && !bytes.iter().any(|b| matches!(b, 0x1b | 0x0e | 0x0f))
        {
            return std::borrow::Cow::Borrowed(bytes);
        }
        let mut out = Vec::with_capacity(bytes.len());
        for &b in bytes {
            self.step(b, &mut out);
        }
        std::borrow::Cow::Owned(out)
    }

    fn step(&mut self, b: u8, out: &mut Vec<u8>) {
        match self.at {
            At::Ground => match b {
                0x1b => {
                    self.at = At::Esc;
                    out.push(b);
                }
                // Shift Out / Shift In: which of G0/G1 is invoked.
                // Consumed rather than forwarded — `vt100` does not act
                // on them, so passing them on would leave a stray
                // control byte in the stream for nothing.
                0x0e => self.invoked = 1,
                0x0f => self.invoked = 0,
                0x5f..=0x7e if self.graphics[self.invoked] => {
                    let mut buf = [0u8; 4];
                    let c = GRAPHICS[(b - 0x5f) as usize];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                _ => out.push(b),
            },
            At::Esc => {
                // The four charset designators, in G0–G3 order.
                let slot = match b {
                    b'(' => Some(0),
                    b')' => Some(1),
                    b'*' => Some(2),
                    b'+' => Some(3),
                    _ => None,
                };
                match slot {
                    // **The whole sequence is swallowed**, this one
                    // included: `vt100` cannot act on it, and what it
                    // does with an escape it cannot act on is call
                    // `unhandled_escape`. Having acted on it here,
                    // forwarding it would be asking a second party to
                    // ignore it.
                    Some(g) => {
                        out.pop();
                        self.at = At::Designate(g);
                    }
                    None => {
                        out.push(b);
                        self.at = match b {
                            b'[' => At::Csi,
                            b']' | b'P' | b'X' | b'^' | b'_' => At::Str,
                            _ => At::Ground,
                        };
                    }
                }
            }
            At::Designate(g) => {
                // `0` is DEC Special Graphics; `B` (ASCII) and every
                // other designation take it back off.
                self.graphics[g] = b == b'0';
                self.at = At::Ground;
            }
            At::Csi => {
                out.push(b);
                if (0x40..=0x7e).contains(&b) {
                    self.at = At::Ground;
                }
            }
            At::Str => {
                out.push(b);
                self.at = match b {
                    0x07 => At::Ground,
                    0x1b => At::StrEsc,
                    _ => At::Str,
                };
            }
            At::StrEsc => {
                out.push(b);
                // `ESC \` is `ST`; anything else was a stray escape
                // inside the string and the string carries on.
                self.at = if b == b'\\' { At::Ground } else { At::Str };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn through(chunks: &[&[u8]]) -> String {
        let mut acs = Acs::default();
        let mut out = Vec::new();
        for c in chunks {
            out.extend_from_slice(&acs.filter(c));
        }
        String::from_utf8(out).expect("the filter must never produce invalid UTF-8")
    }

    /// The reason this file exists: an ncurses box, drawn the way
    /// `xterm-256color`'s terminfo says to draw one.
    ///
    /// **The control is the same bytes without the designator.** A
    /// filter that simply mapped every `q` to `─` would pass the first
    /// assertion and be catastrophically wrong — `q` is a letter.
    #[test]
    fn a_box_drawn_through_the_graphics_charset_becomes_a_box() {
        assert_eq!(through(&[b"\x1b(0lqqk\x1b(B"]), "┌──┐", "the sequence draws a box");
        assert_eq!(
            through(&[b"lqqk"]),
            "lqqk",
            "and the very same letters, with no charset invoked, are letters"
        );
    }

    /// **`ESC ( 0` is a mode, so the filter has to be one too.** A
    /// program sets the charset in one write and draws in the next;
    /// per-read state would translate the first chunk only, which is
    /// exactly how a screen ends up half-drawn.
    #[test]
    fn the_charset_survives_the_read_it_was_set_in() {
        assert_eq!(through(&[b"\x1b(0", b"qqq", b"\x1b(B", b"qqq"]), "───qqq");
    }

    /// Nothing inside an escape sequence may be touched: `q` is a
    /// perfectly ordinary byte in a CSI parameter, an OSC title, and a
    /// DCS payload, and rewriting one would corrupt the sequence rather
    /// than draw anything.
    #[test]
    fn escape_sequences_pass_through_even_while_graphics_is_mapped_in() {
        assert_eq!(
            through(&[b"\x1b(0\x1b[1;31mq\x1b[0m"]),
            "\x1b[1;31m─\x1b[0m",
            "the CSI is untouched and only the text between is drawn"
        );
        assert_eq!(
            through(&[b"\x1b(0\x1b]0;a q title\x07q"]),
            "\x1b]0;a q title\x07─",
            "an OSC string keeps its own letters"
        );
        assert_eq!(
            through(&[b"\x1b(0\x1bP q payload \x1b\\q"]),
            "\x1bP q payload \x1b\\─",
            "and so does a DCS, whose terminator is ST rather than BEL"
        );
    }

    /// **It cannot corrupt UTF-8**, by arithmetic rather than by care:
    /// every byte of a multi-byte sequence is `>= 0x80` and the only
    /// bytes rewritten are `0x5F..=0x7E`.
    #[test]
    fn multibyte_text_is_never_touched() {
        assert_eq!(through(&[b"\x1b(0", "中文 ✻ 🙂".as_bytes()]), "中文 ✻ 🙂");
    }

    /// `SO`/`SI` pick which of G0/G1 is invoked — the other half of the
    /// mechanism, and the half a filter that only watched `ESC ( 0`
    /// would miss.
    #[test]
    fn shift_out_selects_the_other_charset() {
        // G1 gets the graphics set; G0 stays ASCII. `SO` invokes G1.
        assert_eq!(through(&[b"\x1b)0q\x0eq\x0fq"]), "q─q");
    }

    /// The fast path must be a fast path *and* a correct one: a chunk
    /// with no escape and no charset in play is handed back as it came.
    #[test]
    fn ordinary_text_is_passed_through_without_copying() {
        let mut acs = Acs::default();
        assert!(matches!(acs.filter(b"plain lqk text"), std::borrow::Cow::Borrowed(_)));
        assert_eq!(&*acs.filter(b"plain lqk text"), b"plain lqk text");
    }
}
