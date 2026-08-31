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
//!
//! **Scrollback lives here, in the emulator, and dies with the
//! attachment.** Closing the pane and opening it again builds a new
//! parser with an empty history — measured against the alternative and
//! chosen: what a person reaches back for is almost always the thing
//! that just scrolled past, and that is inside this attachment.
//!
//! **The alternative is written down because it is on the shelf, not
//! missing**: the host could keep the raw bytes and answer a request for
//! history, which would survive re-attaching and follow you to another
//! machine. It needs a new op, a wire format, and an answer to "how much
//! does the host keep" — a different size of job, and deliberately not
//! this one.

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

/// A run of adjacent cells that share every attribute **and every
/// cell's width** — coalesced so a row of one colour is one node to
/// paint, not eighty.
///
/// # Why a run carries its geometry
///
/// A terminal is a grid, and a face that paints runs as flowing text is
/// not painting a grid — it is asking the browser to guess one from
/// glyph advances. The guess is wrong the moment any glyph comes from a
/// font other than the measured one, which on this screen is constant:
/// CJK falls back to a face whose advance is not twice the latin one,
/// and every symbol an agent draws (`▰▱ ↻ ⏵⏵ ✻ ⏺`) comes from
/// somewhere else again.
///
/// Two things went wrong because of it, both reported from a real
/// screen (2026-09-01):
///
///   - **The cursor sat far past the last character.** It was placed by
///     arithmetic — `cursor_col * cell_width`, which is right — against
///     text that had drifted, which is not.
///   - **A spinner at the head of a line moved the rest of the line**
///     every time it changed glyph. Flowed text has no cells to hold
///     anything still.
///
/// So a run states where it starts and how many columns it covers, and
/// the face gives it a box exactly that wide. Drift is then bounded
/// inside one run and can never reach its neighbours.
///
/// **`wide` splits runs that `fg`/`bg` alone would have joined**, which
/// is the point: a box is only exact if every cell in it is the same
/// width, so a run may not straddle the boundary between narrow and
/// double-width cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct TermRun {
    pub text: String,
    /// The column this run starts at — its left edge in cells.
    pub col: u16,
    /// How many columns it covers. `text.chars().count()` for a narrow
    /// run, twice that for a wide one.
    pub cells: u16,
    /// Whether every cell in this run is double-width.
    pub wide: bool,
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

/// How many lines above the screen this app keeps to look back at.
///
/// Two thousand at eighty columns is a couple of megabytes per open
/// terminal, which a desktop does not notice; the half saved by keeping
/// a thousand only ever shows up as "why is it gone" the one time
/// somebody reaches for it (领队裁定 2026-08-21). A machine holding
/// twenty terminals open at once would be the reason to revisit it.
pub const SCROLLBACK: usize = 2000;

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
    /// Where the answer was actually taken from, in lines above the
    /// live screen. **Not simply what was asked for**: a request to go
    /// back further than there is history is clamped, and a face that
    /// painted its own number instead of this one would show somebody
    /// standing five hundred lines up in a terminal that has twenty.
    #[ts(type = "number")]
    pub back: usize,
    /// How many lines of history exist right now. It grows as output
    /// scrolls off, which is what lets a face hold its place: the view
    /// is an offset from the *bottom*, so a line arriving moves the
    /// content under a fixed offset by one.
    #[ts(type = "number")]
    pub depth: usize,
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
        let mut carry: Option<(Style, bool)> = None;
        for col in 0..cols {
            let cell = screen.cell(row, col);
            if let Some(c) = cell {
                // The second half of a double-width character is not a
                // cell of its own — its column is already spoken for by
                // the run the first half opened.
                if c.is_wide_continuation() {
                    continue;
                }
            }
            let wide = cell.is_some_and(vt100::Cell::is_wide);
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
            let step = if wide { 2 } else { 1 };
            // **The width joins the style in deciding what a run is.** A
            // box is only exact if every cell in it is the same width.
            if carry == Some((style, wide)) {
                let last = runs.last_mut().unwrap();
                last.text.push_str(&text);
                last.cells += step;
            } else {
                runs.push(TermRun {
                    text,
                    col,
                    cells: step,
                    wide,
                    fg: style.fg,
                    bg: style.bg,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    inverse: style.inverse,
                });
                carry = Some((style, wide));
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
    /// **Lives as long as the screen, because `ESC ( 0` is a mode.**
    /// A program sets the graphics charset once and then draws through
    /// it across many reads; a filter rebuilt per read would translate
    /// the first chunk and hand the rest to the parser as `q`s
    /// (`crate::acs`).
    acs: Mutex<crate::acs::Acs>,
    /// Bumped on every byte batch — the poll skips an unchanged screen.
    seq: AtomicU64,
    gone: AtomicBool,
    /// Open count, for React's double-mount, exactly as chat explains.
    holds: AtomicUsize,
    /// Where the last answer was taken from. **A screen is worth
    /// sending when the view moved, and the sequence number cannot say
    /// that** — coming back down to the live screen changes no bytes,
    /// so a poll judged on `seq` alone answers "nothing new" and the
    /// face goes on painting the history it was looking at. Kept per
    /// attachment rather than per reader: with two panes at different
    /// offsets it only means an extra screen now and then, which is
    /// cheap and never wrong.
    last_back: AtomicUsize,
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
        parser: Mutex::new(vt100::Parser::new(rows, cols, SCROLLBACK)),
        acs: Mutex::new(crate::acs::Acs::default()),
        seq: AtomicU64::new(0),
        gone: AtomicBool::new(false),
        holds: AtomicUsize::new(1),
        last_back: AtomicUsize::new(0),
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
                    if let Some((cols, rows)) = paint(&reader.acs, &reader.parser, &buf[..n]) {
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

/// **The one path bytes take from the socket to the screen.** A free
/// function rather than three lines inside the reader thread so a test
/// can run the same composition the reader runs — the failure this
/// guards against is not a broken filter but a filter that is never
/// called, and a test that assembled its own pipeline would pass
/// either way.
///
/// Answers the size to send back as a `Resize` when the screen had to
/// be replaced, and `None` otherwise.
fn paint(
    acs: &Mutex<crate::acs::Acs>,
    parser: &Mutex<vt100::Parser>,
    bytes: &[u8],
) -> Option<(u16, u16)> {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    // DEC Special Graphics before the parser sees it: `vt100` does not
    // implement SCS, so an ncurses border would otherwise land as
    // literal `l q k` (`crate::acs`). Taken and released before the
    // parser's lock — the two are never held together, so this thread
    // joins no lock order.
    let bytes = plock(acs).filter(bytes).into_owned();
    // The catch is inside the lock scope on purpose: the guard then
    // drops normally and the mutex is never poisoned. On a panic
    // (vt100's corner cases — see `plock`) the screen is replaced at
    // the same size and a same-size resize is sent, which is a
    // SIGWINCH: the program repaints the whole frame into the fresh
    // screen (mandala's self-heal trick).
    let mut g = plock(parser);
    if catch_unwind(AssertUnwindSafe(|| g.process(&bytes))).is_err() {
        let (rows, cols) = g.screen().size();
        *g = vt100::Parser::new(rows, cols, SCROLLBACK);
        return Some((cols, rows));
    }
    None
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
pub fn term_poll(id: &str, since: u64, back: usize) -> Result<TermBatch, String> {
    let term = term_of(id)?;
    let seq = term.seq.load(Ordering::Relaxed);
    // **One lock for the whole look, and the parser is left where it was
    // found.** Reading history is done by moving the emulator's own view
    // and reading the cells through it, so the move, the read and the
    // move back happen without letting go — two panes on one session
    // share this parser (the registry is keyed by session id), and a
    // window left shifted is the other pane's screen scrolled away by
    // somebody it never heard of.
    let mut g = plock(&term.parser);
    let screen = g.screen_mut();
    // The clamp is the emulator's, so the depth is read by asking for
    // more than can exist and seeing where it lands.
    screen.set_scrollback(usize::MAX);
    let depth = screen.scrollback();
    screen.set_scrollback(back);
    let at = screen.scrollback();
    // A screen is worth sending when the terminal moved, whenever the
    // reader is looking at history at all (the view then depends on
    // where they stand, not only on what arrived), and on the one move
    // that changes neither: coming back down to the live screen.
    let moved = at != term.last_back.swap(at, Ordering::Relaxed);
    let shot = (seq != since || at > 0 || moved).then(|| snapshot(g.screen()));
    g.screen_mut().set_scrollback(0);
    Ok(TermBatch { screen: shot, seq, gone: term.gone.load(Ordering::Relaxed), back: at, depth })
}

/// Keystrokes, as the bytes a terminal would send — the face translates
/// keys to bytes, this layer stays a pipe.
pub fn term_key(id: &str, bytes: Vec<u8>) -> Result<(), String> {
    let term = term_of(id)?;
    let mut conn = plock(&term.conn);
    write_frame(&mut *conn, &ClientOp::Input(bytes))
}

/// A paste, wrapped the way the program in there asked for it to be.
///
/// **The screen already knows, so nothing has to be guessed.** A program
/// that wants pasted text marked as pasted turns on DECSET 2004, and
/// `vt100` tracks that on the very screen this module keeps in order to
/// paint (`Screen::bracketed_paste`). So the decision is made where the
/// fact is, and the face says "this is a paste" instead of assembling
/// bytes it has no way to be right about — the same split that keeps it
/// from re-deriving a state.
///
/// **Why a paste is not just typing.** Sent plain, a path containing a
/// newline is not text arriving in a prompt, it is a command the shell
/// runs; a path containing a tab may be completed into something else
/// again. Bracketed paste is what lets the program treat the whole run
/// as one lump of text — which is also what makes dropping a file into
/// a terminal safe enough to offer at all (批④).
///
/// A program that has *not* asked keeps the old behaviour exactly: the
/// bytes go through bare, because wrapping them for a reader that does
/// not know the escape would paste the escape.
pub fn term_paste(id: &str, text: &str) -> Result<(), String> {
    let term = term_of(id)?;
    let bracketed = plock(&term.parser).screen().bracketed_paste();
    let mut bytes = Vec::new();
    if bracketed {
        bytes.extend_from_slice(b"\x1b[200~");
    }
    bytes.extend_from_slice(text.as_bytes());
    if bracketed {
        bytes.extend_from_slice(b"\x1b[201~");
    }
    let mut conn = plock(&term.conn);
    write_frame(&mut *conn, &ClientOp::Input(bytes))
}

/// One path, written so a POSIX shell reads back exactly that path.
///
/// **Single quotes and nothing else.** Inside them every byte is
/// literal — spaces, `$`, backticks, `*`, newlines, semicolons — so
/// there is one rule instead of a list of characters somebody has to
/// remember to keep in step with the shell. The single quote is the one
/// character a single-quoted string cannot hold, and it is closed,
/// escaped, and reopened (`'\''`).
///
/// This is the whole reason dropping a file into a terminal is safe to
/// offer: an unquoted path with a space in it is two arguments, and one
/// with a newline is a second command.
pub fn shell_quote(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 2);
    out.push('\'');
    for c in path.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Sends files to a machine and has it take them in, answering with
/// where they landed **over there**.
///
/// Its own thread with its own runtime, for the reason
/// [`reach_blocking`] spells out: this is called from a tauri command,
/// which is already async, and a nested runtime panics instead of
/// returning something a person can read.
///
/// **The sync in the middle is not optional and not a cadence tweak.**
/// `send` writes the offer into this machine's copy of the chat
/// document; the machine being asked to accept looks the transfer up in
/// *its* copy, and without a push it has never heard of it and refuses
/// by name. The CLI's `send` leaves that to the serve's own rhythm
/// because an offer is content to wait for a person — this one cannot,
/// because the same gesture that sends is the gesture that accepts.
///
/// **Since `2f68ac2` the push is an optimisation, not a requirement**,
/// and the distinction is worth keeping straight. `Node::accept` now
/// drives a sync round of its own when the offer is not here yet, so
/// dropping this line would still work — it would just make the far
/// machine pull from everyone instead of letting this one push what it
/// has just written, which is strictly the more expensive half of the
/// same round. So it stays, on cost rather than on correctness.
///
/// It was, until that commit, one of **two** hand-placed workarounds for
/// one bug: this one in product code (so the feature worked and nobody
/// looked), and a `b.sync_now()` in `crates/node/tests/transfer.rs` (so
/// the test was green and nobody looked). Between them they explain how
/// a race with a written-down repro sat in the ledger for days —
/// **every road had a brick under it, and the bricks had no names.**
fn deliver_blocking(root: &Path, machine: &str, paths: &[String]) -> Result<Vec<String>, String> {
    let (root, machine, paths) = (root.to_path_buf(), machine.to_owned(), paths.to_vec());
    std::thread::spawn(move || -> Result<Vec<String>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(khor_catalog::msg::runtime_wont_start)?;
        let n = Node::open(root)?;
        let mut sent = Vec::with_capacity(paths.len());
        for p in &paths {
            sent.push(n.send(&machine, Path::new(p))?);
        }
        rt.block_on(n.sync_now())?;
        let mut landed = Vec::with_capacity(sent.len());
        for id in &sent {
            let (_, where_it_is) = rt.block_on(n.accept(id))?;
            if where_it_is.is_empty() {
                // The files did arrive — this is the far machine
                // declining to say where, which is what a khor from
                // before that field on the wire does
                // (`khor_node::proto::Response::Acted`). Said out loud
                // rather than pasting nothing: a drop that types
                // nothing into the terminal is the 做了但没变化 failure,
                // and a guessed path would be worse — it would be a
                // path that does not exist over there.
                return Err(khor_catalog::msg::LANDED_UNSAID.to_owned());
            }
            landed.extend(where_it_is);
        }
        Ok(landed)
    })
    .join()
    .map_err(|_| khor_catalog::msg::DELIVER_THREAD_DIED.to_owned())?
}

/// Files dropped onto a terminal: their paths, quoted, as one paste.
///
/// A paste and not typing, which is what [`term_paste`] buys — a path
/// with a newline in it typed straight in is a command the shell runs,
/// and that is exactly the shape a dropped file can have.
///
/// **The fork is on the row, not on a flag** (`Node::far_machine`), the
/// same question [`term_open`] asks. A terminal running on another
/// machine cannot be handed a path from this one: that string names a
/// file the far shell has no way to open, and pasting it would produce
/// a command that looks right and fails. So the files travel first, and
/// what gets pasted is where they came to rest over there.
pub fn term_drop(root: &Path, id: &str, paths: &[String]) -> Result<(), String> {
    let n = Node::open(root.to_path_buf())?;
    let sid = SessionId(id.to_owned());
    let here = match n.far_machine(&sid)? {
        Some(machine) => deliver_blocking(root, &machine, paths)?,
        None => paths.to_vec(),
    };
    let text = here.iter().map(|p| shell_quote(p)).collect::<Vec<_>>().join(" ");
    term_paste(id, &text)
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

    /// **The filter is wired into the path the socket's bytes take**,
    /// which is a different claim from "the filter works" (`acs`'s own
    /// tests make that one). This runs `paint`, the function the reader
    /// thread runs, so a filter that stopped being called would show up
    /// here as a row of `l q k`.
    ///
    /// Two chunks, because the charset is a mode: the reader gets
    /// whatever the socket hands it, and the designator and the drawing
    /// routinely arrive in different reads.
    #[test]
    fn the_reader_path_draws_an_ncurses_border_as_a_border() {
        let acs = Mutex::new(crate::acs::Acs::default());
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        assert_eq!(paint(&acs, &parser, b"\x1b(0"), None);
        assert_eq!(paint(&acs, &parser, b"lqqqk"), None);
        let s = snapshot(plock(&parser).screen());
        assert!(
            s.lines[0][0].text.starts_with("┌───┐"),
            "the border must reach the screen drawn: {:?}",
            s.lines[0][0].text
        );

        // The control: the same five letters with no charset invoked
        // are five letters, so this is not a blanket rewrite of `q`.
        let acs = Mutex::new(crate::acs::Acs::default());
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        paint(&acs, &parser, b"lqqqk");
        let s = snapshot(plock(&parser).screen());
        assert!(s.lines[0][0].text.starts_with("lqqqk"), "got {:?}", s.lines[0][0].text);
    }

    /// **Every run states where it starts and how wide it is, and the
    /// two always agree with the grid.** This is the claim the painted
    /// terminal rests on: the face gives each run a box exactly
    /// `cells` wide, so a glyph from a fallback font can be loose or
    /// tight inside its own run and still not move anything after it.
    ///
    /// Reported from a real screen 2026-09-01: a spinner at the head of
    /// a line moved the rest of the line every time it changed glyph,
    /// and the cursor sat far past the last character — both of them
    /// flowed text pretending to be a grid.
    #[test]
    fn every_run_says_where_it_starts_and_the_columns_add_up() {
        // Three styles, so three runs: default, red, default again.
        let s = feed(b"ab\x1b[31mcd\x1b[0mef");
        let runs = &s.lines[0];
        assert!(runs.len() >= 3, "a colour change is a run boundary: {runs:?}");
        assert_eq!((runs[0].col, runs[0].cells), (0, 2));
        assert_eq!((runs[1].col, runs[1].cells), (2, 2));
        assert_eq!((runs[2].col, runs[2].cells), (4, 6), "the tail runs to the edge");
        // The invariant, on every row of every screen: runs tile the
        // line with no gap and no overlap, and end exactly at `cols`.
        for (i, line) in s.lines.iter().enumerate() {
            let mut at = 0;
            for r in line {
                assert_eq!(r.col, at, "row {i} run {r:?} does not start where the last one ended");
                at += r.cells;
            }
            assert_eq!(at, s.cols, "row {i} must be covered exactly once");
        }
    }

    /// **A double-width character owns two columns and says so** — and
    /// it may not share a run with narrow cells, because a box is only
    /// exact when every cell in it is the same width.
    ///
    /// The control is in the same assertion: the latin half of the same
    /// line, in the same style, is a *separate* run. Without the width
    /// in the run-boundary test they would coalesce, `cells` would be
    /// right and the box would still be a lie.
    #[test]
    fn a_wide_character_takes_two_columns_and_a_run_of_its_own() {
        let s = feed("ab中".as_bytes());
        let runs = &s.lines[0];
        assert_eq!(runs[0].text, "ab");
        assert!(!runs[0].wide);
        assert_eq!((runs[0].col, runs[0].cells), (0, 2));
        assert_eq!(runs[1].text, "中", "same style, different width — a run of its own");
        assert!(runs[1].wide);
        assert_eq!((runs[1].col, runs[1].cells), (2, 2), "one character, two columns");
        assert_eq!(runs[2].col, 4, "and the next run starts past both of them");
        let mut at = 0;
        for r in runs {
            assert_eq!(r.col, at);
            at += r.cells;
        }
        assert_eq!(at, s.cols);
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

#[cfg(test)]
mod quoting_tests {
    use super::shell_quote;

    /// **The shell is the oracle, not my idea of the shell.** Every case
    /// is fed back through a real `sh` and the bytes it prints must be
    /// the path that went in — so this checks the rule that actually
    /// governs, rather than checking my quoting against my own reading
    /// of it. A hand-written table of "these characters are dangerous"
    /// is exactly the thing that goes stale without anybody noticing.
    fn survives_a_real_shell(path: &str) {
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf %s {}", shell_quote(path)))
            .output()
            .expect("sh must run");
        assert!(out.status.success(), "sh refused {:?}", shell_quote(path));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            path,
            "the shell read {:?} back as something else",
            path,
        );
    }

    #[test]
    fn a_quoted_path_reaches_the_shell_as_itself() {
        for path in [
            "/tmp/plain.txt",
            "/tmp/with a space.txt",
            "/tmp/it's got a quote.txt",
            "/tmp/two''quotes.txt",
            "/tmp/a\nnewline.txt",
            "/tmp/$HOME-is-not-expanded",
            "/tmp/`backtick`.txt",
            "/tmp/semi;colon && echo pwned",
            "/tmp/star*glob?.txt",
            "/tmp/中文 路径.txt",
            "/tmp/tab\there.txt",
            "",
        ] {
            survives_a_real_shell(path);
        }
    }

    /// The dangerous one on its own, said out loud: a path that would be
    /// a second command if it went in bare must come back as one string.
    #[test]
    fn a_path_that_looks_like_a_command_stays_a_path() {
        let nasty = "/tmp/x; rm -rf /tmp/should-not-happen";
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("printf %s {}", shell_quote(nasty)))
            .output()
            .expect("sh must run");
        assert_eq!(String::from_utf8_lossy(&out.stdout), nasty);
    }

    /// Several files are several arguments, not one run-on word.
    #[test]
    fn each_dropped_path_is_its_own_argument() {
        let quoted: Vec<String> =
            ["/tmp/a b.txt", "/tmp/c.txt"].iter().map(|p| shell_quote(p)).collect();
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("for a in {}; do printf '[%s]' \"$a\"; done", quoted.join(" ")))
            .output()
            .expect("sh must run");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "[/tmp/a b.txt][/tmp/c.txt]");
    }
}
