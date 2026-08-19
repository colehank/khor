//! The terminal registry: live attachments to a session's host, painted
//! as a screen the frontend polls (docs/handoff 终端画屏).
//!
//! It mirrors [`crate::chat`] exactly — a webview cannot hold a socket
//! and the dev bridge serves one call at a time, so the socket lives
//! here: one reader thread per attachment feeds the host's raw PTY bytes
//! into a `vt100` screen, and [`term_poll`] answers instantly with a cell
//! grid. The difference from chat is what the reader keeps: not a growing
//! list of frames but one screen that the bytes overwrite in place — a
//! terminal is a *state*, not a stream, so a poll wants the latest screen
//! whole, and a sequence number is all it needs to skip an unchanged one.
//!
//! The emulation is `vt100`, the same engine the detector reads
//! (`crates/detect`): one terminal model for both "what does it say" and
//! "what does it look like", so they can never disagree.

use std::collections::HashMap;
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use khor_node::host::{connect, write_frame, ClientOp};
use khor_node::{Node, SessionId};
use serde::Serialize;
use ts_rs::TS;

/// A cell's colour, kept semantic rather than resolved: an indexed colour
/// means different pixels in light and dark, and that mapping is the
/// face's to make against its own palette (docs/UX.md 状态呈现). Default
/// is "whatever this terminal's foreground/background is".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", content = "n", rename_all = "snake_case")]
pub enum TermColor {
    Default,
    Idx(u8),
    Rgb([u8; 3]),
}

impl From<vt100::Color> for TermColor {
    fn from(c: vt100::Color) -> Self {
        match c {
            vt100::Color::Default => TermColor::Default,
            vt100::Color::Idx(i) => TermColor::Idx(i),
            vt100::Color::Rgb(r, g, b) => TermColor::Rgb([r, g, b]),
        }
    }
}

/// A run of adjacent cells that share every attribute — coalesced so a
/// row of one colour is one node to paint, not eighty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct TermRun {
    pub text: String,
    pub fg: TermColor,
    pub bg: TermColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

/// One painted screen: its size, cursor, and rows of runs. Rows are as
/// tall as the screen even when blank, so the grid never jumps.
#[derive(Debug, Clone, Serialize, TS)]
pub struct TermScreen {
    pub cols: u16,
    pub rows: u16,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_hidden: bool,
    pub lines: Vec<Vec<TermRun>>,
}

/// What a poll answers: the current screen if it changed since the
/// cursor (`None` if not — an unchanged terminal costs nothing to skip),
/// the next cursor, and whether the attachment ended (a goodbye and a
/// torn socket are one fact to a face, as in chat).
#[derive(Debug, Clone, Serialize, TS)]
pub struct TermBatch {
    pub screen: Option<TermScreen>,
    #[ts(type = "number")]
    pub seq: u64,
    pub gone: bool,
}

/// The attributes that make two cells one run. Not sent — it is the key
/// the coalescing compares on.
#[derive(PartialEq, Eq, Clone, Copy)]
struct Style {
    fg: TermColor,
    bg: TermColor,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

fn style_of(cell: &vt100::Cell) -> Style {
    Style {
        fg: cell.fgcolor().into(),
        bg: cell.bgcolor().into(),
        bold: cell.bold(),
        italic: cell.italic(),
        underline: cell.underline(),
        inverse: cell.inverse(),
    }
}

/// Reads a `vt100` screen into the shape a face paints, coalescing runs.
/// A wide character keeps its one glyph and its continuation cell is
/// skipped: a CJK glyph is two columns wide in a monospace grid on its
/// own, so painting the continuation as a space would double it.
fn snapshot(screen: &vt100::Screen) -> TermScreen {
    let (rows, cols) = screen.size();
    let mut lines = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut runs: Vec<TermRun> = Vec::new();
        let mut carry: Option<Style> = None;
        for col in 0..cols {
            let cell = screen.cell(row, col);
            if let Some(c) = cell {
                if c.is_wide_continuation() {
                    continue;
                }
            }
            let (text, style) = match cell {
                Some(c) if c.has_contents() => (c.contents().to_string(), style_of(c)),
                Some(c) => (" ".to_string(), style_of(c)),
                None => (" ".to_string(), Style {
                    fg: TermColor::Default,
                    bg: TermColor::Default,
                    bold: false,
                    italic: false,
                    underline: false,
                    inverse: false,
                }),
            };
            if carry == Some(style) {
                runs.last_mut().unwrap().text.push_str(&text);
            } else {
                runs.push(TermRun {
                    text,
                    fg: style.fg,
                    bg: style.bg,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    inverse: style.inverse,
                });
                carry = Some(style);
            }
        }
        lines.push(runs);
    }
    let (cursor_row, cursor_col) = screen.cursor_position();
    TermScreen {
        cols,
        rows,
        cursor_row,
        cursor_col,
        cursor_hidden: screen.hide_cursor(),
        lines,
    }
}

struct Term {
    /// The write half. One lock per op so keystrokes never interleave.
    conn: Mutex<TcpStream>,
    /// The screen the reader thread overwrites in place.
    parser: Mutex<vt100::Parser>,
    /// Bumped on every byte batch — the poll skips an unchanged screen.
    seq: AtomicU64,
    gone: AtomicBool,
    /// Open count, for React's double-mount, exactly as chat explains.
    holds: AtomicUsize,
}

/// A lock that outlives a panic on the other side. A reader thread that
/// panicked mid-`process` (vt100 has corner-case panics on real streams)
/// poisons whatever it held; every later `.lock().unwrap()` then panics
/// too, and in the bridge that cascade killed the whole server (seen
/// live, 2026-08-18: one bad tmux frame → poisoned parser → a resize's
/// unwrap → exit 101). The data under these locks is a screen or a
/// socket — nothing an interrupted writer leaves half-true in a way a
/// repaint does not fix — so the poison flag carries no information
/// worth dying for.
fn plock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn terms() -> &'static Mutex<HashMap<String, Arc<Term>>> {
    static TERMS: OnceLock<Mutex<HashMap<String, Arc<Term>>>> = OnceLock::new();
    TERMS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// [`Node::reach`] from a synchronous caller, on a thread of its own.
///
/// The reach is async and this door is not. Building a runtime on
/// whatever thread happens to call it works until that caller is itself
/// async — and a tauri command is — at which point it panics rather than
/// returning an error anybody can read. A thread costs one click's worth
/// of nothing and cannot be wrong about where it is.
fn reach_blocking(root: &Path, machine: &str, id: &SessionId) -> Result<(String, String), String> {
    let (root, machine, id) = (root.to_path_buf(), machine.to_owned(), id.clone());
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(khor_catalog::msg::runtime_wont_start)?;
        let n = Node::open(root)?;
        rt.block_on(n.reach(&machine, &id))
    })
    .join()
    .map_err(|_| khor_catalog::msg::REACH_THREAD_DIED.to_string())?
}

/// Attaches to the session's host at `cols`×`rows`, idempotently (an
/// already-open terminal keeps its screen; a dead one is replaced). The
/// host resizes its PTY to this size, so the whole screen repaints and
/// the attacher sees a full frame rather than a fragment.
pub fn term_open(root: &Path, id: &str, cols: u16, rows: u16) -> Result<(), String> {
    let mut registry = plock(terms());
    if let Some(term) = registry.get(id) {
        if !term.gone.load(Ordering::Relaxed) {
            term.holds.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        registry.remove(id);
    }
    let n = Node::open(root.to_path_buf())?;
    let sid = SessionId(id.to_owned());
    // **The fork is on the row, not on a flag** (`Node::far_machine`):
    // one list holds every machine's sessions, so one click has to mean
    // the same thing wherever the row lives. The CLI's `attach` forks on
    // this very question, and this door now matches it.
    let conn = match n.far_machine(&sid)? {
        // Reached, not hosted here: the resident serve binds a local
        // port onto the far host, and what arrives is still a TCP
        // address speaking the host protocol — so everything below this
        // line is unchanged, and the terminal never learns there is a
        // network under it (`host::connect_at`).
        Some(machine) => {
            let (addr, cookie) = reach_blocking(root, &machine, &sid)?;
            khor_node::host::connect_at(&addr, &cookie, cols, rows)?
        }
        None => {
            // A row this machine does not host yet may still have a
            // terminal to be had — a discovered tmux session, or an
            // agent sitting inside one (its row's `term` said so). The
            // bridge stands a host up under this very id (grouped
            // client — `LiveKind::attach_multiplexed` has the judgment,
            // including refusing rows with no route), and from here on
            // it is any hosted session.
            if !n.is_hosted(&sid) {
                n.attach_tmux(&sid)?;
            }
            let dir = n
                .session_dir(&sid)
                .ok_or_else(|| khor_catalog::msg::no_such_session(id))?;
            connect(&dir, cols, rows)?
        }
    };
    let mut reading = conn.try_clone().map_err(|e| e.to_string())?;
    let term = Arc::new(Term {
        conn: Mutex::new(conn),
        parser: Mutex::new(vt100::Parser::new(rows, cols, 0)),
        seq: AtomicU64::new(0),
        gone: AtomicBool::new(false),
        holds: AtomicUsize::new(1),
    });
    let reader = term.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut buf = [0u8; 8192];
        loop {
            match reading.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // The catch is inside the lock scope on purpose: the
                    // guard then drops normally and the mutex is never
                    // poisoned. On a panic (vt100's corner cases — see
                    // `plock`) the screen is replaced at the same size
                    // and a same-size resize is sent, which is a
                    // SIGWINCH: the program repaints the whole frame
                    // into the fresh screen (mandala's self-heal trick).
                    let mut g = plock(&reader.parser);
                    if catch_unwind(AssertUnwindSafe(|| g.process(&buf[..n]))).is_err() {
                        let (rows, cols) = g.screen().size();
                        *g = vt100::Parser::new(rows, cols, 0);
                        drop(g);
                        let mut conn = plock(&reader.conn);
                        let _ = write_frame(&mut *conn, &ClientOp::Resize { cols, rows });
                    }
                    reader.seq.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        reader.gone.store(true, Ordering::Relaxed);
    });
    registry.insert(id.to_owned(), term);
    Ok(())
}

fn term_of(id: &str) -> Result<Arc<Term>, String> {
    terms()
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| khor_catalog::msg::no_such_session(id))
}

/// The screen since `since` — the whole current screen if the sequence
/// moved, nothing if it did not. Instant either way.
pub fn term_poll(id: &str, since: u64) -> Result<TermBatch, String> {
    let term = term_of(id)?;
    let seq = term.seq.load(Ordering::Relaxed);
    let screen = (seq != since).then(|| snapshot(plock(&term.parser).screen()));
    Ok(TermBatch { screen, seq, gone: term.gone.load(Ordering::Relaxed) })
}

/// Keystrokes, as the bytes a terminal would send — the face translates
/// keys to bytes, this layer stays a pipe.
pub fn term_key(id: &str, bytes: Vec<u8>) -> Result<(), String> {
    let term = term_of(id)?;
    let mut conn = plock(&term.conn);
    write_frame(&mut *conn, &ClientOp::Input(bytes))
}

/// A resize: the PTY is told (so programs repaint at the new size) and
/// the local screen is set to match, so the bytes that repaint arrive
/// into a screen already the right shape.
pub fn term_resize(id: &str, cols: u16, rows: u16) -> Result<(), String> {
    let term = term_of(id)?;
    plock(&term.parser).screen_mut().set_size(rows, cols);
    term.seq.fetch_add(1, Ordering::Relaxed);
    let mut conn = plock(&term.conn);
    write_frame(&mut *conn, &ClientOp::Resize { cols, rows })
}

/// Drops one hold; the last one out closes the socket (chat's rule, and
/// for the same double-mount reason).
pub fn term_leave(id: &str) -> Result<(), String> {
    let mut registry = plock(terms());
    if let Some(term) = registry.get(id) {
        if term.holds.fetch_sub(1, Ordering::Relaxed) <= 1 {
            registry.remove(id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mechanism the live crash cascaded through: a thread panicking
    /// while holding a lock poisons it, and an `unwrap` on the next lock
    /// re-panics. `plock` must answer with the data instead — the flag
    /// carries nothing a repaint does not fix.
    #[test]
    fn a_poisoned_lock_answers_instead_of_cascading() {
        let m = Arc::new(Mutex::new(7u8));
        let poisoner = m.clone();
        let _ = std::thread::spawn(move || {
            let _g = poisoner.lock().unwrap();
            panic!("poison it");
        })
        .join();
        assert!(m.lock().is_err(), "the control: the mutex really is poisoned");
        assert_eq!(*plock(&m), 7, "plock answers with the value regardless");
    }

    fn feed(bytes: &[u8]) -> TermScreen {
        let mut parser = vt100::Parser::new(3, 10, 0);
        parser.process(bytes);
        snapshot(parser.screen())
    }

    #[test]
    fn plain_text_lands_in_one_run_on_the_first_row() {
        let s = feed(b"hello");
        assert_eq!(s.rows, 3);
        assert_eq!(s.cols, 10);
        // "hello" then five trailing spaces, all default style → one run.
        assert_eq!(s.lines[0].len(), 1);
        assert_eq!(s.lines[0][0].text, "hello     ");
        assert_eq!(s.cursor_col, 5, "the cursor sits after the text");
    }

    #[test]
    fn a_colour_change_splits_the_row_into_runs() {
        // "ab" default, then red "cd".
        let s = feed(b"ab\x1b[31mcd");
        let runs = &s.lines[0];
        assert!(runs.len() >= 2, "the colour change is a new run: {runs:?}");
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[0].fg, TermColor::Default);
        assert_eq!(runs[1].text, "cd");
        assert_eq!(runs[1].fg, TermColor::Idx(1), "SGR 31 is indexed red");
    }

    #[test]
    fn a_cursor_move_paints_where_it_was_addressed() {
        // Move to row 2 col 3 (1-based in the escape), write "X".
        let s = feed(b"\x1b[2;3HX");
        assert_eq!(s.lines[1][0].text.chars().nth(2), Some('X'));
        assert_eq!((s.cursor_row, s.cursor_col), (1, 3));
    }
}
