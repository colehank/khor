//! The terminal modes a byte stream switched on, kept for the bytes a
//! replay no longer holds.
//!
//! # Why
//!
//! A replay is the tail of a stream: the host's ring ([`crate::host`])
//! and the app's feed (`khor_gui_core::term`) both trim their heads. A
//! mode is set **once** — a tmux client sends `ESC [ ? 1049 h` when it
//! attaches and never again, however long it then draws — so once the
//! head is trimmed past it, every later attacher replays a screen
//! drawn for the alternate buffer into the normal one. Measured
//! 2026-10-01 (the owner: 「初次进入一个 tmux 会话的标签，滚鼠标滚轮
//! 页面没有变化；把窗口缩放一下就能滚了」): after 1.5 MB through a tmux
//! viewer, a fresh attach in a real WKWebView left xterm in the
//! `normal` buffer with application cursor keys off, while tmux sat in
//! the alternate screen. The wheel then scrolled xterm's own, nearly
//! empty, scrollback — nothing moved, nothing reached tmux — and a
//! resize "fixed" it only because the program's repaint scrolled lines
//! into that wrong buffer's history for the wheel to find.
//!
//! So whoever trims a replay's head feeds the trimmed bytes through a
//! [`Modes`], and a replay starts with [`Modes::preamble`]: the modes
//! as they stood at the oldest byte still held. Preamble + tail is then
//! the same terminal state as the whole stream would have left.
//!
//! # What is tracked, and what is not
//!
//! The DEC private modes (`CSI ? Pm h` / `l`) that change how a
//! terminal *behaves* rather than what it shows — the buffer, cursor
//! keys, the mouse, paste, focus reports, wrap and cursor visibility —
//! and the keypad (`ESC =` / `ESC >`). A full reset (`ESC c`) forgets
//! them all. Everything else is skipped by a parser that only knows
//! enough grammar to not mistake a string (OSC, DCS, …) for a mode.
//! Not a screen model: what a screen *shows* comes back from the tail
//! and from the repaint an attach's resize asks for.

use std::collections::BTreeMap;

/// The modes worth carrying across a trimmed head, each with the value
/// a freshly reset terminal has. The three alternate-screen spellings
/// come first so a preamble switches buffers before it sets anything a
/// buffer switch could reset.
const TRACKED: &[(u16, bool)] = &[
    (47, false),
    (1047, false),
    (1049, false),
    // DECCKM: cursor keys send `ESC O A` rather than `ESC [ A`.
    (1, false),
    // DECAWM and DECTCEM — on by default.
    (7, true),
    (25, true),
    // Mouse reporting, its encodings, focus reports.
    (1000, false),
    (1002, false),
    (1003, false),
    (1004, false),
    (1005, false),
    (1006, false),
    (1015, false),
    // Bracketed paste.
    (2004, false),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Parse {
    #[default]
    Ground,
    Esc,
    /// Inside `CSI`; `private` once a leading `?` was seen, `foreign`
    /// once a byte made it some other sequence than a DEC mode set.
    Csi {
        private: bool,
        foreign: bool,
        started: bool,
    },
    /// Inside a string (OSC, DCS, SOS, PM, APC) until BEL or `ESC \`.
    Str,
    /// `ESC` seen inside a string — `\` ends it, anything else is data.
    StrEsc,
}

/// See the module head.
#[derive(Debug, Clone, Default)]
pub struct Modes {
    parse: Parse,
    params: Vec<u16>,
    current: Option<u16>,
    /// Only modes that were ever set or reset; the rest are default.
    set: BTreeMap<u16, bool>,
    keypad: bool,
}

impl Modes {
    /// Eats bytes in stream order. Sequences may be split across calls
    /// anywhere — the parse state carries over.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.byte(b);
        }
    }

    fn byte(&mut self, b: u8) {
        // CAN and SUB abort any sequence, in every state.
        if b == 0x18 || b == 0x1a {
            self.parse = Parse::Ground;
            return;
        }
        match self.parse {
            Parse::Ground => {
                if b == 0x1b {
                    self.parse = Parse::Esc;
                }
            }
            Parse::Esc => {
                self.parse = Parse::Ground;
                match b {
                    b'[' => {
                        self.params.clear();
                        self.current = None;
                        self.parse = Parse::Csi {
                            private: false,
                            foreign: false,
                            started: false,
                        };
                    }
                    b']' | b'P' | b'X' | b'^' | b'_' => self.parse = Parse::Str,
                    b'=' => self.keypad = true,
                    b'>' => self.keypad = false,
                    b'c' => {
                        self.set.clear();
                        self.keypad = false;
                    }
                    0x1b => self.parse = Parse::Esc,
                    _ => {}
                }
            }
            Parse::Csi {
                private,
                foreign,
                started,
            } => match b {
                0x1b => self.parse = Parse::Esc,
                b'?' if !started => {
                    self.parse = Parse::Csi {
                        private: true,
                        foreign,
                        started: true,
                    }
                }
                b'0'..=b'9' => {
                    let d = u16::from(b - b'0');
                    self.current = Some(
                        self.current
                            .unwrap_or(0)
                            .saturating_mul(10)
                            .saturating_add(d),
                    );
                    self.parse = Parse::Csi {
                        private,
                        foreign,
                        started: true,
                    };
                }
                b';' => {
                    self.params.push(self.current.take().unwrap_or(0));
                    self.parse = Parse::Csi {
                        private,
                        foreign,
                        started: true,
                    };
                }
                0x40..=0x7e => {
                    self.parse = Parse::Ground;
                    if let Some(n) = self.current.take() {
                        self.params.push(n);
                    }
                    if private && !foreign && (b == b'h' || b == b'l') {
                        for &p in &self.params {
                            if TRACKED.iter().any(|&(m, _)| m == p) {
                                self.set.insert(p, b == b'h');
                            }
                        }
                    }
                }
                // Intermediates, other parameter bytes (`>`, `!`, `:`…):
                // some other sequence than a DEC mode set.
                0x20..=0x3f => {
                    self.parse = Parse::Csi {
                        private,
                        foreign: true,
                        started: true,
                    }
                }
                // C0 controls execute inside a CSI without ending it.
                _ => {}
            },
            Parse::Str => match b {
                0x07 => self.parse = Parse::Ground,
                0x1b => self.parse = Parse::StrEsc,
                _ => {}
            },
            Parse::StrEsc => {
                self.parse = if b == b'\\' {
                    Parse::Ground
                } else {
                    Parse::Str
                };
            }
        }
    }

    /// The bytes that put a freshly reset terminal into the modes the
    /// stream had reached — empty when every tracked mode is at its
    /// default, so an untrimmed replay is unchanged.
    pub fn preamble(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for &(mode, default) in TRACKED {
            match self.set.get(&mode) {
                Some(&on) if on != default => {
                    out.extend_from_slice(
                        format!("\x1b[?{mode}{}", if on { 'h' } else { 'l' }).as_bytes(),
                    );
                }
                _ => {}
            }
        }
        if self.keypad {
            out.extend_from_slice(b"\x1b=");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(bytes: &[u8]) -> Vec<u8> {
        let mut m = Modes::default();
        m.feed(bytes);
        m.preamble()
    }

    /// What a tmux client sends on attach (captured from tmux 3.7b, the
    /// head of a fresh viewer's ring) comes back as the modes it set.
    #[test]
    fn a_tmux_attach_head_comes_back_as_its_modes() {
        let head = b"\x1b[?1049h\x1b[?1h\x1b=\x1b[H\x1b[2J\x1b[?12l\x1b[?25h\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1005l\x1b[?2004h\x1b[?2031h\x1b[?996n\x1b[m";
        assert_eq!(after(head), b"\x1b[?1049h\x1b[?1h\x1b[?2004h\x1b=".to_vec());
    }

    #[test]
    fn nothing_set_is_an_empty_preamble() {
        assert!(after(b"plain text\r\n\x1b[31mred\x1b[m\x1b[2J\x1b[10;1H").is_empty());
    }

    /// Split anywhere — a read boundary falls where it falls.
    #[test]
    fn a_sequence_split_across_reads_still_counts() {
        let whole = b"\x1b[?1049h\x1b[?1000;1006h";
        for cut in 0..whole.len() {
            let mut m = Modes::default();
            m.feed(&whole[..cut]);
            m.feed(&whole[cut..]);
            assert_eq!(
                m.preamble(),
                b"\x1b[?1049h\x1b[?1000h\x1b[?1006h".to_vec(),
                "cut at {cut}"
            );
        }
    }

    #[test]
    fn a_later_reset_wins_and_a_full_reset_forgets() {
        assert!(after(b"\x1b[?1049h\x1b[?1049l").is_empty());
        assert_eq!(after(b"\x1b[?25l"), b"\x1b[?25l".to_vec());
        assert!(after(b"\x1b[?1049h\x1b=\x1bc").is_empty());
    }

    /// A string is not a mode, even when it spells one; a non-private
    /// `h` (SM, ANSI modes) is not a DEC one.
    #[test]
    fn strings_and_other_sequences_are_not_modes() {
        assert!(after(b"\x1b]0;\x1b[?1049h\x07").is_empty());
        assert!(after(b"\x1bP\x1b[?1049h\x1b\\").is_empty());
        assert!(after(b"\x1b[4h\x1b[>1h\x1b[?1049$p").is_empty());
    }
}
