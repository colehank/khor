//! The terminal registry: live attachments to a session's host, kept
//! as the raw byte stream the frontend's own emulator eats (完整终端批
//! 2026-09-03 — xterm.js renders; this side stopped painting).
//!
//! It mirrors [`crate::chat`] in shape — a webview cannot hold a socket
//! and the dev bridge serves one call at a time, so the socket lives
//! here: one reader thread per attachment. What the reader keeps
//! changed with that batch: not a painted cell grid but the bytes
//! themselves, in a bounded feed ([`FEED_CAP`]) a face replays once on
//! arrival ([`term_attach`]) and then tails ([`term_stream`]). The
//! screen model lives in the webview now, which is a full terminal;
//! keeping a second one here to re-paint what xterm already understands
//! was the drift the cell pipeline kept paying for — cursor arithmetic,
//! run boxes, the wheel judgment — and the whole surface went with it
//! (`TermScreen`/`TermRun`/`TermColor`/`TermBatch`/`Wheel`,
//! `term_poll`/`term_wheel`, `snapshot`).
//!
//! **The vt100 parser stays, demoted from painter to mode oracle.** Two
//! callers are left: bracketed paste ([`term_paste`] asks the screen
//! whether DECSET 2004 is on, and [`term_drop`] rides it) and the panic
//! self-heal in [`paint`] (replace the parser, send a same-size resize,
//! let SIGWINCH repaint). It keeps no scrollback any more — history is
//! the feed.
//!
//! **Where history comes from**: the feed starts collecting the moment
//! the attachment opens, and the host pours its ring to every fresh
//! client first (its own design for every attacher) — so an attach
//! replays the past the host kept, and the feed carries everything
//! since. The old head kept a shelf note ("the host could keep the raw
//! bytes and answer a request for history"); this is that note half
//! cashed — the near half. Cross-attachment history is still the
//! host's ring and dies with the host, exactly as before.

use std::collections::{HashMap, VecDeque};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine;
use khor_node::host::{ClientOp, connect, write_frame};
use khor_node::modes::Modes;
use khor_node::{Node, SessionId};
use serde::Serialize;
use ts_rs::TS;

/// The base64 the two answers speak — standard alphabet, the same
/// engine the node's wire uses.
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// How many raw bytes the feed keeps, per attachment.
///
/// Enough for a full-screen repaint many times over and for the ring
/// the host pours on attach; past it the head is trimmed, and a client
/// that fell that far behind is told to start over
/// ([`TermChunk::reset`]) rather than handed a stream with a hole —
/// these bytes are a grammar, and a splice mid-escape is garbage on
/// screen.
pub const FEED_CAP: usize = 512 * 1024;

/// The bytes the feed holds, and the modes the bytes it trimmed had set
/// (`khor_node::modes`): a replay of a trimmed feed opens with them, or
/// a face that attaches after the first [`FEED_CAP`] of a tmux client
/// draws the alternate screen into the normal buffer (2026-10-01 — the
/// host's ring has the same rule for the same reason).
#[derive(Default)]
struct Feed {
    bytes: VecDeque<u8>,
    head: Modes,
    /// The modes at the feed's end: what a face that starts over from
    /// the screen ([`take_since`]) is put into before the screen is drawn.
    tail: Modes,
    /// Where this attachment's life began on the cursor ([`restart_feed`]):
    /// a cursor before it is another life's.
    life: u64,
}

impl Feed {
    /// Everything a face that starts over is handed: the trimmed head's
    /// modes, then every byte still held.
    fn replay(&self) -> Vec<u8> {
        let mut out = self.head.preamble();
        out.extend(self.bytes.iter());
        out
    }
}

/// What an attach answers: everything the feed holds, and the cursor
/// to tail from ([`term_stream`]'s `since`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct TermStart {
    /// base64 of the feed — raw PTY bytes, not text: they carry
    /// escapes, split UTF-8, whatever the program wrote.
    pub bytes: String,
    #[ts(type = "number")]
    pub seq: u64,
}

/// One pull on the tail. Empty `bytes` when nothing new — the face
/// polls on its own beat, the old poll's rhythm, and an idle pull
/// costs a lock and nothing else.
#[derive(Debug, Clone, Serialize, TS)]
pub struct TermChunk {
    pub bytes: String,
    #[ts(type = "number")]
    pub seq: u64,
    /// The attachment ended — a goodbye and a torn socket are one fact
    /// to a face.
    pub gone: bool,
    /// `since` fell off the feed's trimmed head, or names a life this
    /// `Term` never lived (a re-made attachment counts from zero
    /// again): reset the emulator and replay `bytes`, never splice. For
    /// a face that fell behind in this life, `bytes` is the screen as it
    /// stands; for another life's cursor, the whole feed.
    pub reset: bool,
    /// The pipe to a far machine's host ended and is being asked for
    /// again (`crate::rejoin`): what is on screen is the last thing that
    /// arrived, not a live screen. The face says so until bytes flow.
    pub rejoining: bool,
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
    gone: AtomicBool,
    /// Between a far pipe ending and the next one carrying bytes.
    rejoining: AtomicBool,
    /// Rung whenever the feed grows or a flag above changes: what a
    /// [`term_wait`] sleeps on instead of the face polling on a beat.
    arrived: tokio::sync::Notify,
    /// Open count, for React's double-mount, exactly as chat explains.
    holds: AtomicUsize,
    /// Nobody holds it any more: a pipe that ends now is not asked for
    /// again (`crate::rejoin`).
    left: AtomicBool,
    /// The bytes themselves, exactly as the parser ate them (post-ACS
    /// filter — the two consumers must see one stream), head-trimmed at
    /// [`FEED_CAP`], with the modes of what was trimmed.
    feed: Mutex<Feed>,
    /// Total bytes ever pushed into `feed` since this `Term` was made —
    /// the stream cursor. `fed - feed.len()` is the oldest byte still
    /// held. Written only inside [`push_feed`], under the `feed` lock,
    /// so a reader holding that lock sees the pair consistent.
    fed: AtomicU64,
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
pub(crate) fn reach_blocking(
    root: &Path,
    machine: &str,
    id: &SessionId,
) -> Result<(String, String), String> {
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
    .map_err(|_| khor_catalog::msg::reach_thread_died().to_string())?
}

/// Attaches to the session's host at `cols`×`rows`, idempotently (an
/// already-open terminal keeps its screen; a dead one is replaced). The
/// host resizes its PTY to this size, so the whole screen repaints and
/// the attacher sees a full frame rather than a fragment.
///
/// **The registry is not held while the attachment is made** (2026-10-02,
/// the phone that froze): a far terminal is reached over the network,
/// which takes up to the reach's deadline when that machine has dropped
/// off, and every other terminal's poll, key and resize takes this same
/// lock. Held across the reach, one unreachable machine stopped every
/// terminal on screen — and those doors answer the window. So the lock
/// is taken to look, let go, and taken again to put the new attachment
/// in; an open that raced this one and got there first is the one kept.
pub fn term_open(root: &Path, id: &str, cols: u16, rows: u16) -> Result<(), String> {
    if hold_if_open(id) {
        return Ok(());
    }
    let attached = attach(root, id, cols, rows);
    let mut registry = plock(terms());
    // A leave that came while this open was reaching takes its hold
    // (`crate::landing`): nobody is there to read what it attached.
    let left = plock(landing()).land(id);
    let (conn, far) = attached?;
    if left {
        let _ = conn.shutdown(std::net::Shutdown::Both);
        return Ok(());
    }
    if let Some(theirs) = registry.get(id)
        && !theirs.gone.load(Ordering::Relaxed)
    {
        // Another open made it while this one was reaching: theirs is
        // the attachment, and this connection goes unused.
        theirs.holds.fetch_add(1, Ordering::Relaxed);
        let _ = conn.shutdown(std::net::Shutdown::Both);
        return Ok(());
    }
    let sid = SessionId(id.to_owned());
    let mut reading = conn.try_clone().map_err(|e| e.to_string())?;
    let term = Arc::new(Term {
        conn: Mutex::new(conn),
        // No scrollback: the parser answers modes now, not history —
        // history is the feed, and xterm keeps the scroll.
        parser: Mutex::new(vt100::Parser::new(rows, cols, 0)),
        acs: Mutex::new(crate::acs::Acs::default()),
        gone: AtomicBool::new(false),
        rejoining: AtomicBool::new(false),
        arrived: tokio::sync::Notify::new(),
        holds: AtomicUsize::new(1),
        left: AtomicBool::new(false),
        feed: Mutex::new(Feed::default()),
        fed: AtomicU64::new(0),
    });
    let reader = term.clone();
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = [0u8; 8192];
        // The next bytes are a new attachment's: the host's ring again,
        // from its start (`start_over`).
        let mut fresh = false;
        loop {
            match reading.read(&mut buf) {
                Ok(0) | Err(_) => {
                    // **A far pipe that ends asks again where that is
                    // switched on** (a phone, `crate::rejoin`): the end
                    // of a pipe is not the end of the session over
                    // there. Only a far machine's answer that it is not
                    // there ends this — elsewhere the end of the pipe
                    // does, as it always has.
                    let Some(machine) = far.as_deref().filter(|_| crate::rejoin::is_on()) else {
                        break;
                    };
                    reader.rejoining.store(true, Ordering::Relaxed);
                    reader.arrived.notify_waiters();
                    let (rows, cols) = plock(&reader.parser).screen().size();
                    let Some(conn) = crate::rejoin::again(&root, machine, &sid, &reader.left, |addr, cookie| {
                        khor_node::host::connect_at(addr, cookie, cols, rows)
                    }) else {
                        break;
                    };
                    let Ok(again) = conn.try_clone() else { break };
                    *plock(&reader.conn) = conn;
                    reading = again;
                    fresh = true;
                }
                Ok(n) => {
                    if fresh {
                        start_over(&reader);
                        fresh = false;
                        reader.rejoining.store(false, Ordering::Relaxed);
                    }
                    if let Some((cols, rows)) = paint(
                        &reader.acs,
                        &reader.parser,
                        &reader.feed,
                        &reader.fed,
                        &buf[..n],
                    ) {
                        let mut conn = plock(&reader.conn);
                        let _ = write_frame(&mut *conn, &ClientOp::Resize { cols, rows });
                    }
                    reader.arrived.notify_waiters();
                }
            }
        }
        reader.gone.store(true, Ordering::Relaxed);
        reader.arrived.notify_waiters();
    });
    registry.insert(id.to_owned(), term);
    Ok(())
}

/// The connection [`term_open`] attaches with, and the far machine it
/// came through (`None` when the row is hosted here). No lock is held:
/// a far one waits on the network.
fn attach(
    root: &Path,
    id: &str,
    cols: u16,
    rows: u16,
) -> Result<(std::net::TcpStream, Option<String>), String> {
    let n = Node::open(root.to_path_buf())?;
    let sid = SessionId(id.to_owned());
    // **The fork is on the row, not on a flag** (`Node::far_machine`):
    // one list holds every machine's sessions, so one click has to mean
    // the same thing wherever the row lives. The CLI's `attach` forks on
    // this very question, and this door now matches it.
    let far = n.far_machine(&sid)?;
    let conn = match &far {
        // Reached, not hosted here: the resident serve binds a local
        // port onto the far host, and what arrives is still a TCP
        // address speaking the host protocol — so everything below this
        // line is unchanged, and the terminal never learns there is a
        // network under it (`host::connect_at`).
        Some(machine) => {
            let (addr, cookie) = reach_blocking(root, machine, &sid)?;
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
            // **The seat decides the address.** A row can have a host
            // of its own *and* sit in the person's tmux (a conversation
            // khor started, resumed in a terminal), and then the
            // terminal belongs to the viewer bridged onto tmux, never
            // to the ghost. `attach_tmux` is the one judgment of which
            // it is; a row in no tmux with a live host answers `Host`
            // from its file and one sweep, which is what the ordinary
            // open costs now.
            let seat = match n.attach_tmux(&sid) {
                Ok(seat) => seat,
                Err(_) if n.is_hosted(&sid) => khor_node::live::Seat::Host,
                Err(e) => return Err(e),
            };
            let dir = n
                .session_dir(&sid)
                .ok_or_else(|| khor_catalog::msg::no_such_session(id))?;
            match seat {
                khor_node::live::Seat::View => khor_node::host::connect_view(&dir, cols, rows)?,
                khor_node::live::Seat::Host => connect(&dir, cols, rows)?,
            }
        }
    };
    Ok((conn, far))
}

/// An open terminal under `id` takes one more hold and answers yes; a
/// dead one is taken out, the open is written down as on its way
/// (`crate::landing`), and the answer is no — the look `term_open`
/// makes before it reaches, with the lock let go again at once.
fn hold_if_open(id: &str) -> bool {
    let mut registry = plock(terms());
    if let Some(term) = registry.get(id) {
        if !term.gone.load(Ordering::Relaxed) {
            term.holds.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        registry.remove(id);
    }
    plock(landing()).begin(id);
    false
}

/// Terminal opens still reaching, and leaves that came before them.
/// Taken only while [`terms`] is held.
fn landing() -> &'static Mutex<crate::landing::Landing> {
    static LANDING: OnceLock<Mutex<crate::landing::Landing>> = OnceLock::new();
    LANDING.get_or_init(Default::default)
}

/// An attachment made again (`crate::rejoin`) starts a new life: the
/// host pours its whole ring to the new client, so the screen and the
/// mode oracle start over, and so does the feed — a face whose cursor
/// is in the old life is told to reset and replay ([`restart_feed`]).
/// Called on the new attachment's first bytes, not before: until they
/// arrive the face keeps showing the last screen it had.
fn start_over(term: &Term) {
    {
        let mut parser = plock(&term.parser);
        let (rows, cols) = parser.screen().size();
        *parser = vt100::Parser::new(rows, cols, 0);
    }
    *plock(&term.acs) = crate::acs::Acs::default();
    restart_feed(&term.feed, &term.fed);
}

/// Empties the feed and moves the cursor one past every place a face
/// can hold in the old life, so every face's next [`take_since`] falls
/// off the head and answers `reset` with only the new life's bytes —
/// the replay a face already does for a feed that trimmed past it.
fn restart_feed(feed: &Mutex<Feed>, fed: &AtomicU64) {
    let mut g = plock(feed);
    g.bytes.clear();
    g.head = Modes::default();
    g.tail = Modes::default();
    g.life = fed.fetch_add(1, Ordering::Relaxed) + 1;
}

/// **The one path bytes take from the socket onward** — through the
/// ACS filter once, then into both consumers of the same filtered
/// bytes: the feed a face replays, and the parser that answers modes.
/// A free function rather than lines inside the reader thread so a
/// test can run the reader's own composition — the failure this guards
/// against is not a broken filter or a broken feed but one that is
/// never called, and a test that assembled its own pipeline would pass
/// either way.
///
/// Answers the size to send back as a `Resize` when the parser had to
/// be replaced, and `None` otherwise.
fn paint(
    acs: &Mutex<crate::acs::Acs>,
    parser: &Mutex<vt100::Parser>,
    feed: &Mutex<Feed>,
    fed: &AtomicU64,
    bytes: &[u8],
) -> Option<(u16, u16)> {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    // DEC Special Graphics before either consumer sees it: `vt100` does
    // not implement SCS (`crate::acs`), and xterm.js gets the same
    // translation so the mode oracle and the picture can never disagree
    // about one stream. Taken and released before any other lock.
    let bytes = plock(acs).filter(bytes).into_owned();
    // **The parser's lock spans both consumers**, so the screen and the
    // feed's end always describe the same byte: a face that starts over
    // from the screen ([`take_since`]) then tails from exactly there.
    // Lock order: parser, then feed — everywhere.
    let mut g = plock(parser);
    // The feed first: a parser corner-case panic must not cost the face
    // its bytes — the heal below repaints the parser, and the stream
    // has to stay whole either way.
    push_feed(feed, fed, &bytes);
    // The catch is inside the lock scope on purpose: the guard then
    // drops normally and the mutex is never poisoned. On a panic
    // (vt100's corner cases — see `plock`) the parser is replaced at
    // the same size and a same-size resize is sent, which is a
    // SIGWINCH: the program repaints the whole frame into the fresh
    // screen (mandala's self-heal trick).
    if catch_unwind(AssertUnwindSafe(|| g.process(&bytes))).is_err() {
        let (rows, cols) = g.screen().size();
        *g = vt100::Parser::new(rows, cols, 0);
        return Some((cols, rows));
    }
    None
}

/// Appends to the feed and advances the cursor, trimming the head past
/// [`FEED_CAP`]. `fed` moves only here, under the feed lock.
fn push_feed(feed: &Mutex<Feed>, fed: &AtomicU64, bytes: &[u8]) {
    let mut g = plock(feed);
    g.bytes.extend(bytes.iter().copied());
    g.tail.feed(bytes);
    let over = g.bytes.len().saturating_sub(FEED_CAP);
    if over > 0 {
        let gone: Vec<u8> = g.bytes.drain(..over).collect();
        g.head.feed(&gone);
    }
    fed.fetch_add(bytes.len() as u64, Ordering::Relaxed);
}

/// The whole feed and its end cursor, consistent under one lock. The
/// cursor counts stream bytes only — the modes in front are not part of
/// the stream a later [`take_since`] continues.
fn take_all(feed: &Mutex<Feed>, fed: &AtomicU64) -> (Vec<u8>, u64) {
    let g = plock(feed);
    (g.replay(), fed.load(Ordering::Relaxed))
}

/// The bytes from `since` to the end — or, with `reset` raised:
///
/// - **`since` fell off the trimmed head in this life** (a face that
///   could not keep up with a burst): the screen as it stands — the
///   feed end's modes, then the screen drawn (`vt100`'s formatted
///   contents, cursor included), a few KB — and the face tails from
///   the end. Terminal audit T-12: it was handed the whole feed, 512 KiB
///   to parse while the burst ran on, so it fell behind again and was
///   reset again (7 times in a 4 MB `yes`), each replay carrying old
///   queries with it. Lost: scrollback past one screen, for the stretch
///   the face could not read anyway.
/// - **`since` is another life's, or beyond the end** (a re-made `Term`
///   counts from zero again, so a stale cursor can be *bigger* than
///   anything this feed ever counted): the whole feed, the new life's
///   history.
fn take_since(
    parser: &Mutex<vt100::Parser>,
    feed: &Mutex<Feed>,
    fed: &AtomicU64,
    since: u64,
) -> (Vec<u8>, u64, bool) {
    let screen = plock(parser);
    let g = plock(feed);
    let end = fed.load(Ordering::Relaxed);
    let start = end - g.bytes.len() as u64;
    if since < start && since >= g.life {
        let mut out = g.tail.preamble();
        out.extend(screen.screen().contents_formatted());
        return (out, end, true);
    }
    drop(screen);
    if since < start || since > end {
        return (g.replay(), end, true);
    }
    let from = (since - start) as usize;
    (g.bytes.iter().skip(from).copied().collect(), end, false)
}

fn term_of(id: &str) -> Result<Arc<Term>, String> {
    terms()
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| khor_catalog::msg::no_such_session(id))
}

/// The feed so far, for a face that just attached: reset the emulator,
/// write these bytes, then tail from `seq` with [`term_stream`]. The
/// host pours its ring to every fresh client, so this carries the
/// history the host kept plus everything since this attachment opened.
pub fn term_attach(id: &str) -> Result<TermStart, String> {
    let term = term_of(id)?;
    let (bytes, seq) = take_all(&term.feed, &term.fed);
    Ok(TermStart {
        bytes: B64.encode(bytes),
        seq,
    })
}

/// The bytes since `since` — see [`TermChunk`] for the two flags.
pub fn term_stream(id: &str, since: u64) -> Result<TermChunk, String> {
    let term = term_of(id)?;
    let (bytes, seq, reset) = take_since(&term.parser, &term.feed, &term.fed, since);
    Ok(TermChunk {
        bytes: B64.encode(bytes),
        seq,
        gone: term.gone.load(Ordering::Relaxed),
        reset,
        rejoining: term.rejoining.load(Ordering::Relaxed),
    })
}

/// [`term_stream`], **pushed rather than polled**: answers the moment the
/// feed grows past `since` (or the attachment ends, or starts or stops
/// rejoining), and at the latest after `longest` with nothing new.
///
/// ⚠️ 墓碑 (round NET-2): the face asked [`term_stream`] every 50 ms, so
/// an echo that had already crossed the network waited on average 25 ms
/// more, and up to 50, for the next beat. A face in the app now holds one
/// of these open at a time (the bridge's HTTP loop serves one request at
/// a time, so the browser face keeps the beat).
pub async fn term_wait(id: &str, since: u64, longest: std::time::Duration) -> Result<TermChunk, String> {
    let term = term_of(id)?;
    let woke = term.arrived.notified();
    tokio::pin!(woke);
    woke.as_mut().enable();
    if term.fed.load(Ordering::Relaxed) == since && !term.gone.load(Ordering::Relaxed) {
        let _ = tokio::time::timeout(longest, woke).await;
    }
    term_stream(id, since)
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
///
/// **Private since the xterm.js move** (完整终端批, and the user waived
/// old-version compatibility): xterm wraps the person's own clipboard
/// gesture itself, so no face calls this over the wire any more. The
/// one caller left is [`term_drop`] — a dropped file is a paste, and
/// the wrapping decision still lives where the DECSET 2004 fact is.
fn term_paste(id: &str, text: &str) -> Result<(), String> {
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
                return Err(khor_catalog::msg::landed_unsaid().to_owned());
            }
            landed.extend(where_it_is);
        }
        Ok(landed)
    })
    .join()
    .map_err(|_| khor_catalog::msg::deliver_thread_died().to_owned())?
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
    let text = here
        .iter()
        .map(|p| shell_quote(p))
        .collect::<Vec<_>>()
        .join(" ");
    term_paste(id, &text)
}

/// A resize: the PTY is told (so programs repaint at the new size) and
/// the local screen is set to match, so the bytes that repaint arrive
/// into a screen already the right shape.
pub fn term_resize(id: &str, cols: u16, rows: u16) -> Result<(), String> {
    let term = term_of(id)?;
    plock(&term.parser).screen_mut().set_size(rows, cols);
    let mut conn = plock(&term.conn);
    write_frame(&mut *conn, &ClientOp::Resize { cols, rows })
}

/// Drops one hold; the last one out closes the socket (chat's rule, and
/// for the same double-mount reason).
pub fn term_leave(id: &str) -> Result<(), String> {
    let mut registry = plock(terms());
    match registry.get(id) {
        Some(term) => {
            if term.holds.fetch_sub(1, Ordering::Relaxed) <= 1 {
                term.left.store(true, Ordering::Relaxed);
                registry.remove(id);
            }
        }
        // Its open may still be reaching: the hold it would take is
        // this leave's (`crate::landing`).
        None => {
            plock(landing()).leave_early(id);
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
        assert!(
            m.lock().is_err(),
            "the control: the mutex really is poisoned"
        );
        assert_eq!(*plock(&m), 7, "plock answers with the value regardless");
    }

    /// **The filter and the feed are wired into the path the socket's
    /// bytes take** — `paint`, the composition the reader thread runs.
    /// Three claims in one run: the feed holds the *filtered* bytes
    /// (xterm must see exactly what the mode oracle saw, or the two
    /// disagree about one stream), the cursor counts exactly what went
    /// in, and the parser still ate the same thing (its grid drew the
    /// border). The control arm proves the filter is a mode, not a
    /// blanket rewrite of `q` — and, for the feed, that an unfiltered
    /// stream arrives verbatim.
    ///
    /// Two chunks, because the charset is a mode: the designator and
    /// the drawing routinely arrive in different reads.
    #[test]
    fn the_reader_path_feeds_the_same_filtered_bytes_it_parses() {
        let acs = Mutex::new(crate::acs::Acs::default());
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        assert_eq!(paint(&acs, &parser, &feed, &fed, b"\x1b(0"), None);
        assert_eq!(paint(&acs, &parser, &feed, &fed, b"lqqqk"), None);
        let (bytes, seq) = take_all(&feed, &fed);
        assert!(
            !bytes.is_empty(),
            "an empty feed is the un-called feed this test exists for"
        );
        assert_eq!(
            seq,
            bytes.len() as u64,
            "the cursor counts what the feed holds"
        );
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("┌───┐"),
            "the feed carries the filtered border: {text:?}"
        );
        assert!(
            plock(&parser).screen().contents().contains('┌'),
            "…and the parser ate the same bytes"
        );

        // The control: no designator, five letters stay five letters —
        // in the feed and on the oracle's grid alike.
        let acs = Mutex::new(crate::acs::Acs::default());
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        paint(&acs, &parser, &feed, &fed, b"lqqqk");
        let (bytes, _) = take_all(&feed, &fed);
        assert_eq!(bytes, b"lqqqk", "no mode, no rewrite");
    }

    /// attach → stream is seamless: what attach ends at, stream picks
    /// up from, and the two halves concatenate to exactly what was
    /// fed. Then an idle pull answers empty at the same cursor — the
    /// face's 50ms beat costs a lock and nothing else.
    #[test]
    fn attach_then_stream_is_seamless_and_idle_is_empty() {
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        push_feed(&feed, &fed, b"hello ");
        let (first, at) = take_all(&feed, &fed);
        push_feed(&feed, &fed, b"world");
        let (rest, end, reset) = take_since(&parser, &feed, &fed, at);
        assert!(!reset, "a cursor still on the feed never resets");
        assert_eq!([first, rest].concat(), b"hello world");
        assert_eq!(end, 11);
        let (idle, still, reset) = take_since(&parser, &feed, &fed, end);
        assert!(
            idle.is_empty() && still == end && !reset,
            "idle is empty at the same cursor"
        );
    }

    /// Past the cap the head is trimmed, and a cursor that fell off it
    /// answers reset-with-the-screen (T-12: a few KB to draw, not the
    /// 512 KiB a face that is already behind cannot read in time), never
    /// a splice with a hole mid-escape; a cursor from a previous life,
    /// bigger than anything this feed ever counted, is a restart too.
    #[test]
    fn a_cursor_off_the_trimmed_head_gets_the_screen_not_a_splice() {
        let acs = Mutex::new(crate::acs::Acs::default());
        let parser = Mutex::new(vt100::Parser::new(24, 80, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        let chunk = b"line of output\r\n".repeat(200 * 1024 / 16);
        paint(&acs, &parser, &feed, &fed, &chunk);
        paint(&acs, &parser, &feed, &fed, &chunk);
        paint(&acs, &parser, &feed, &fed, b"\x1b[?1hthe last line");
        paint(&acs, &parser, &feed, &fed, &chunk);
        paint(&acs, &parser, &feed, &fed, b"the very last");
        assert_eq!(plock(&feed).bytes.len(), FEED_CAP, "the feed is capped");
        let (bytes, end, reset) = take_since(&parser, &feed, &fed, 0);
        assert!(reset, "cursor 0 fell off the trimmed head");
        assert!(bytes.len() < 8 * 1024, "the screen, not the feed: {} bytes", bytes.len());
        assert!(bytes.starts_with(b"\x1b[?1h"), "in the modes the stream had reached");
        let mut face = vt100::Parser::new(24, 80, 0);
        face.process(&bytes);
        assert_eq!(
            face.screen().contents(),
            plock(&parser).screen().contents(),
            "…and drawn, it is the screen the program left"
        );
        assert_eq!(face.screen().cursor_position(), plock(&parser).screen().cursor_position());
        assert_eq!(end, fed.load(Ordering::Relaxed), "the face tails from the end");
        let (tail, _, reset) = take_since(&parser, &feed, &fed, end - 13);
        assert!(!reset, "a cursor still on the feed streams on");
        assert_eq!(tail, b"the very last");
        let (_, _, reset) = take_since(&parser, &feed, &fed, end + 1);
        assert!(
            reset,
            "another life's cursor is a restart, not a backwards stream"
        );
    }

    /// **An attachment made again starts a new life** (`crate::rejoin`):
    /// every place a face held in the old one answers `reset` with the
    /// new life's bytes alone — the host's ring, poured again, never
    /// spliced after the screen the face already has (that would be the
    /// whole ring twice) — and the new life then streams on as usual.
    #[test]
    fn a_rejoined_feed_resets_every_face_to_the_new_life() {
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        push_feed(&feed, &fed, b"\x1b[?1049hold screen");
        let (_, caught_up) = take_all(&feed, &fed);
        restart_feed(&feed, &fed);
        let (bytes, end, reset) = take_since(&parser, &feed, &fed, caught_up);
        assert!(reset, "a face that had read everything is told to start over");
        assert!(bytes.is_empty(), "with nothing of the old life: {bytes:?}");
        push_feed(&feed, &fed, b"ring again");
        let (bytes, _, reset) = take_since(&parser, &feed, &fed, caught_up);
        assert!(reset && bytes == b"ring again", "a face still in the old life gets the new one whole");
        let (bytes, _, reset) = take_since(&parser, &feed, &fed, end);
        assert!(!reset && bytes == b"ring again", "a face already in the new life streams on");
        let (all, _) = take_all(&feed, &fed);
        assert_eq!(all, b"ring again", "and the old modes are not the new life's");
    }

    /// **A replay of a trimmed feed opens in the modes its head set** —
    /// the same rule as the host's ring, for a face that attaches (or
    /// starts over) after the first [`FEED_CAP`] of a tmux client: attach
    /// hands out the whole feed behind them, and a reset the screen,
    /// in the same modes.
    #[test]
    fn a_trimmed_feed_replays_into_the_modes_its_head_set() {
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        push_feed(&feed, &fed, b"\x1b[?1049h\x1b[?1h\x1b=");
        let line = [b'x'; 1023]
            .iter()
            .chain(b"\n")
            .copied()
            .collect::<Vec<u8>>();
        for _ in 0..600 {
            push_feed(&feed, &fed, &line);
        }
        let (all, end) = take_all(&feed, &fed);
        assert!(
            all.starts_with(b"\x1b[?1049h\x1b[?1h\x1b="),
            "attach opens in the alternate screen with cursor keys and keypad set, began {:?}",
            String::from_utf8_lossy(&all[..all.len().min(24)])
        );
        assert_eq!(all.len(), FEED_CAP + 15, "the modes, then every byte held");
        let (again, _, reset) = take_since(&parser, &feed, &fed, 0);
        assert!(
            reset && again.starts_with(b"\x1b[?1049h\x1b[?1h\x1b="),
            "a reset opens in the same modes"
        );
        let (tail, _, reset) = take_since(&parser, &feed, &fed, end - 4);
        assert!(
            !reset && tail.len() == 4,
            "a tail is the stream alone, no modes spliced in"
        );

        // The control: the same modes still held are not repeated.
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        push_feed(&feed, &fed, b"\x1b[?1049hhi");
        assert_eq!(take_all(&feed, &fed).0, b"\x1b[?1049hhi");
    }

    /// The public doors over a real registry entry: attach hands back
    /// base64 of the feed with its cursor, stream tails it, and a
    /// `gone` flip reaches the answer. Built on a real local socket
    /// pair — no host, because what is under test ends at the
    /// registry.
    #[test]
    fn the_doors_answer_base64_and_carry_gone() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let term = Arc::new(Term {
            conn: Mutex::new(conn),
            parser: Mutex::new(vt100::Parser::new(3, 10, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            rejoining: AtomicBool::new(false),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
        });
        push_feed(&term.feed, &term.fed, b"\x1b[31mhi");
        let id = format!("tui/feed-doors-{}", std::process::id());
        plock(terms()).insert(id.clone(), term.clone());

        let start = term_attach(&id).unwrap();
        assert_eq!(
            B64.decode(start.bytes.as_bytes()).unwrap(),
            b"\x1b[31mhi",
            "attach replays the raw bytes, escapes intact"
        );
        assert_eq!(start.seq, 7);

        push_feed(&term.feed, &term.fed, b"!");
        term.gone.store(true, Ordering::Relaxed);
        let chunk = term_stream(&id, start.seq).unwrap();
        assert_eq!(B64.decode(chunk.bytes.as_bytes()).unwrap(), b"!");
        assert!(chunk.gone, "a torn socket reaches the face");
        assert!(!chunk.reset);

        plock(terms()).remove(&id);
    }

    /// **A pushed wait answers when the bytes arrive, not on a beat**
    /// (round NET-2): held open with nothing new, it returns the moment
    /// the feed grows — measured against the 50 ms beat it replaces.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_wait_answers_when_the_bytes_arrive() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let term = Arc::new(Term {
            conn: Mutex::new(conn),
            parser: Mutex::new(vt100::Parser::new(3, 10, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            rejoining: AtomicBool::new(false),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
        });
        let id = format!("tui/feed-wait-{}", std::process::id());
        plock(terms()).insert(id.clone(), term.clone());

        let mut lags = Vec::new();
        for _ in 0..20 {
            let since = term.fed.load(Ordering::Relaxed);
            let feeder = term.clone();
            let arrives = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                let at = std::time::Instant::now();
                push_feed(&feeder.feed, &feeder.fed, b"x");
                feeder.arrived.notify_waiters();
                at
            });
            let chunk = term_wait(&id, since, std::time::Duration::from_secs(2)).await.unwrap();
            let answered = std::time::Instant::now();
            let at = arrives.join().unwrap();
            assert_eq!(B64.decode(chunk.bytes.as_bytes()).unwrap(), b"x");
            lags.push(answered.duration_since(at));
        }
        lags.sort();
        let median = lags[lags.len() / 2];
        eprintln!("[wait] arrival -> answer median {median:?}, worst {:?} (the beat it replaces: 0-50 ms, 25 on average)", lags[lags.len() - 1]);
        assert!(median < std::time::Duration::from_millis(5), "{lags:?}");

        // Nothing new: it waits, then answers with nothing.
        let since = term.fed.load(Ordering::Relaxed);
        let began = std::time::Instant::now();
        let chunk = term_wait(&id, since, std::time::Duration::from_millis(200)).await.unwrap();
        assert!(began.elapsed() >= std::time::Duration::from_millis(190));
        assert!(chunk.bytes.is_empty());

        plock(terms()).remove(&id);
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
        let quoted: Vec<String> = ["/tmp/a b.txt", "/tmp/c.txt"]
            .iter()
            .map(|p| shell_quote(p))
            .collect();
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "for a in {}; do printf '[%s]' \"$a\"; done",
                quoted.join(" ")
            ))
            .output()
            .expect("sh must run");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "[/tmp/a b.txt][/tmp/c.txt]"
        );
    }
}

#[cfg(test)]
mod burst_tests {
    use super::*;

    /// **A burst a face cannot keep up with** (terminal audit T-12): 4 MB
    /// of `yes` through the reader's own path while a face reads the
    /// stream and spends time on what it got, at the rate xterm parses
    /// (`BURST_FACE_MBPS`, default 20). Every time the
    /// face falls past `FEED_CAP` it is handed a reset; counted here are
    /// the resets, the bytes the face had to parse, and how long after
    /// the burst ended the face had the last screen.
    #[test]
    fn a_face_that_falls_behind_a_burst_starts_over_from_the_screen() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let term = Arc::new(Term {
            conn: Mutex::new(conn),
            parser: Mutex::new(vt100::Parser::new(24, 80, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            rejoining: AtomicBool::new(false),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
        });
        let id = format!("tui/burst-{}", std::process::id());
        plock(terms()).insert(id.clone(), term.clone());

        let producer = term.clone();
        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        let burst = std::thread::spawn(move || {
            let chunk = b"y\r\n".repeat(8192 / 3);
            let mut sent = 0;
            while sent < 4 << 20 {
                paint(&producer.acs, &producer.parser, &producer.feed, &producer.fed, &chunk);
                sent += chunk.len();
            }
            paint(&producer.acs, &producer.parser, &producer.feed, &producer.fed, b"\x1b[1mDONE\x1b[m\r\n$ ");
            finished.store(true, Ordering::SeqCst);
            std::time::Instant::now()
        });

        let face_rate = std::env::var("BURST_FACE_MBPS")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(20.0)
            * 1e6;
        let (mut since, mut resets, mut parsed) = (0u64, 0u32, 0usize);
        let mut screen = vt100::Parser::new(24, 80, 0);
        loop {
            let chunk = term_stream(&id, since).unwrap();
            let bytes = B64.decode(chunk.bytes.as_bytes()).unwrap();
            if chunk.reset {
                resets += 1;
                screen = vt100::Parser::new(24, 80, 0);
            }
            screen.process(&bytes);
            parsed += bytes.len();
            since = chunk.seq;
            // The face's own time on what it got: xterm at `BURST_FACE_MBPS`
            // (default 20, about the audit's Mac; a phone is slower).
            std::thread::sleep(std::time::Duration::from_secs_f64(bytes.len() as f64 / face_rate));
            if done.load(Ordering::SeqCst) && since == term.fed.load(Ordering::SeqCst) {
                break;
            }
            if bytes.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        let caught_up = std::time::Instant::now();
        let ended = burst.join().unwrap();
        plock(terms()).remove(&id);
        eprintln!(
            "[burst] 4 MiB of yes: {resets} resets, the face parsed {:.1} MiB, last screen {:?} after the burst ended",
            parsed as f64 / (1 << 20) as f64,
            caught_up.duration_since(ended)
        );
        assert_eq!(
            screen.screen().contents(),
            plock(&term.parser).screen().contents(),
            "the face ends on the screen the program drew"
        );
        assert!(screen.screen().contents().contains("DONE"));
    }
}
