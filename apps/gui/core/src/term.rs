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

#[path = "term_upload.rs"]
mod upload;
pub use upload::{term_drop_folder, term_drop_stage_into, term_drop_stage_native};

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine;
use khor_node::host::{ClientOp, write_frame};
use khor_node::modes::Modes;
use khor_node::{Node, SessionId};
use serde::Serialize;
use ts_rs::TS;

use crate::miss::{Miss, MissKind};

/// The base64 the two answers speak — standard alphabet, the same
/// engine the node's wire uses.
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// How many raw bytes the feed keeps, per attachment: twice the host's
/// ring (`khor_node::host::RING`, item 17), as it was at 512 KiB to 256.
///
/// Enough for a full-screen repaint many times over and for the ring
/// the host pours on attach; past it the head is trimmed, and a client
/// that fell that far behind is told to start over
/// ([`TermChunk::reset`]) rather than handed a stream with a hole —
/// these bytes are a grammar, and a splice mid-escape is garbage on
/// screen.
pub const FEED_CAP: usize = 2 * khor_node::host::RING;

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
    /// **⌘K** ([`term_forget`]): the feed forgets everything before the
    /// line the cursor is on, as the host's ring does
    /// (`khor_node::host::ClientOp::Forget`), so a face that starts over
    /// from this feed is not handed what was cleared either.
    fn forget(&mut self) {
        let keep = self.bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        let gone: Vec<u8> = self.bytes.drain(..keep).collect();
        self.head.feed(&gone);
        while self.head.mid_sequence() {
            let Some(b) = self.bytes.pop_front() else { break };
            self.head.feed(&[b]);
        }
    }

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
    /// arrived, not a live screen. The face says so until bytes flow —
    /// on the session's pill, 「重新连接 Mac-mini」 — and this says why,
    /// in parts: the machine not answering (`unreached`, its words empty
    /// until the first try is answered) or its host for this session not
    /// answering (`host-lost`). ⚠️ 墓碑 (NOTICE-BRIEF): `rejoining`, a
    /// bare flag, and a bubble over the terminal's top (IMG_5412).
    pub stall: Option<Miss>,
}

struct Term {
    /// The write half. One lock per op so keystrokes never interleave.
    conn: crate::connection::Connection,
    commands: crate::command_queue::CommandQueue,
    /// The screen the reader thread overwrites in place.
    parser: Mutex<vt100::Parser>,
    /// **Lives as long as the screen, because `ESC ( 0` is a mode.**
    /// A program sets the graphics charset once and then draws through
    /// it across many reads; a filter rebuilt per read would translate
    /// the first chunk and hand the rest to the parser as `q`s
    /// (`crate::acs`).
    acs: Mutex<crate::acs::Acs>,
    gone: AtomicBool,
    /// Between a far pipe ending and the next one carrying bytes: why,
    /// as the last try said it ([`TermChunk::stall`]).
    stall: Mutex<Option<Miss>>,
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
    /// The host protocol version this attachment agreed
    /// (`khor_node::host::HOST_VERSION`): 1 for a host from before the
    /// handshake, which is sent nothing it does not know.
    version: AtomicU32,
    /// The most history any face holding this terminal has asked the
    /// host to keep (its 历史行数, item 56); 0 until one says. Said
    /// again to a host the attachment rejoins.
    history: AtomicU32,
}

/// **The host keeps the history the face shows** (item 56): the face's
/// 历史行数 goes to a host of version 4 or later as
/// [`ClientOp::Scrollback`], the largest any face holding the terminal
/// asked for. An older host keeps its own 2000 rows and is sent nothing.
fn keep_history(term: &Term, lines: u32) {
    let most = term.history.fetch_max(lines, Ordering::Relaxed).max(lines);
    say_history(term, &mut plock(&term.conn), most);
}

fn say_history(term: &Term, conn: &mut std::net::TcpStream, lines: u32) {
    if lines > 0 && term.version.load(Ordering::Relaxed) >= 4 {
        let _ = write_frame(conn, &ClientOp::Scrollback(lines));
    }
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
///
/// `history` is the face's 历史行数 (item 56, [`keep_history`]); 0 says
/// nothing.
pub fn term_open(root: &Path, id: &str, cols: u16, rows: u16, history: u32) -> Result<(), String> {
    if hold_if_open(id) {
        // **The face joining still says its size** (item 33): another face
        // may have sized the pty since. Unchanged, the kernel signals
        // nothing; a socket that just ended is the stream's to report.
        let _ = term_resize(id, cols, rows);
        if let Ok(term) = term_of(id) {
            let target = term.clone();
            term.commands.submit(move || { keep_history(&target, history); Ok(()) })?.wait()?;
        }
        return Ok(());
    }
    let attached = attach(root, id, cols, rows);
    let mut registry = plock(terms());
    // A leave that came while this open was reaching takes its hold
    // (`crate::landing`): nobody is there to read what it attached.
    let left = plock(landing()).land(id);
    let (conn, far, version) = attached?;
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
        conn: crate::connection::Connection::new(conn)?,
        commands: Default::default(),
        // No scrollback: the parser answers modes now, not history —
        // history is the feed, and xterm keeps the scroll.
        parser: Mutex::new(vt100::Parser::new(rows, cols, 0)),
        acs: Mutex::new(crate::acs::Acs::default()),
        gone: AtomicBool::new(false),
        stall: Mutex::new(None),
        arrived: tokio::sync::Notify::new(),
        holds: AtomicUsize::new(1),
        left: AtomicBool::new(false),
        feed: Mutex::new(Feed::default()),
        fed: AtomicU64::new(0),
        version: AtomicU32::new(1),
        history: AtomicU32::new(0),
    });
    term.version.store(version, Ordering::Relaxed);
    let target = term.clone();
    let initial = term.commands.submit(move || { keep_history(&target, history); Ok(()) })?;
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
                    *plock(&reader.stall) = Some(Miss::new(MissKind::Unreached, Some(machine), ""));
                    reader.arrived.notify_waiters();
                    let (rows, cols) = plock(&reader.parser).screen().size();
                    let told = |miss: Miss| {
                        *plock(&reader.stall) = Some(miss);
                        reader.arrived.notify_waiters();
                    };
                    let Some(conn) = crate::rejoin::again(
                        &root,
                        machine,
                        &sid,
                        &reader.left,
                        |addr, cookie| {
                            // A far host may have been replaced meanwhile:
                            // the version is agreed again.
                            khor_node::host::connect_versioned(addr, cookie, cols, rows).map(|a| {
                                reader.version.store(a.version, Ordering::Relaxed);
                                a.stream
                            })
                        },
                        told,
                    ) else {
                        break;
                    };
                    let Ok(again) = conn.try_clone() else { break };
                    if reader.conn.replace(conn).is_err() { break; }
                    keep_history(&reader, reader.history.load(Ordering::Relaxed));
                    reading = again;
                    fresh = true;
                }
                Ok(n) => {
                    if fresh {
                        start_over(&reader);
                        fresh = false;
                        *plock(&reader.stall) = None;
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
        *plock(&reader.stall) = None;
        reader.gone.store(true, Ordering::Relaxed);
        reader.commands.close();
        reader.conn.close();
        reader.arrived.notify_waiters();
    });
    registry.insert(id.to_owned(), term.clone());
    drop(registry);
    initial.wait()?;
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
) -> Result<(std::net::TcpStream, Option<String>, u32), String> {
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
            khor_node::host::connect_versioned(&addr, &cookie, cols, rows)?
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
                khor_node::live::Seat::View => khor_node::host::connect_view_versioned(&dir, cols, rows)?,
                khor_node::live::Seat::Host => khor_node::host::connect_host_versioned(&dir, cols, rows)?,
            }
        }
    };
    Ok((conn.stream, far, conn.version))
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
    // the same size, on the same screen with the same modes
    // (`khor_node::host::parser_like`, item 57b): a repaint lands where
    // the face looks, and `?2004h` is still known. The same-size resize
    // sent back is **not** a SIGWINCH — the kernel signals only a change
    // of size — so it repaints nothing; the repaint comes from the
    // host, whose own model meets the same bytes and heals by signalling
    // the program (`host::repaint_foreground`). The resize stays only
    // because it costs nothing; a repaint asked of the host would need
    // an op of its own (audit/term-57.md).
    if catch_unwind(AssertUnwindSafe(|| g.process(&bytes))).is_err() {
        let (rows, cols) = g.screen().size();
        *g = khor_node::host::parser_like(g.screen(), 0);
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
    // A trim that fell inside a sequence runs on to its end, even into
    // later pushes: a replay never opens with a picture's base64
    // (roadmap 3.3; the host's ring keeps pictures apart instead).
    while g.head.mid_sequence() {
        let Some(b) = g.bytes.pop_front() else { break };
        g.head.feed(&[b]);
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

/// A socket and its feed may exist before xterm does. Only the renderer
/// can say that replay is over and live queries now have a responder.
pub fn term_ready(id: &str) -> Result<(), String> {
    submit_ready(id)?.wait()
}
pub fn submit_ready(id: &str) -> Result<crate::command_queue::Pending<()>, String> {
    submit(id, |term| {
        if term.version.load(Ordering::Relaxed) >= 5 {
            write_frame(&mut *plock(&term.conn), &ClientOp::Ready)?;
        }
        Ok(())
    })
}

fn submit<T: Send + 'static>(id: &str, work: impl FnOnce(Arc<Term>) -> Result<T, String> + Send + 'static) -> Result<crate::command_queue::Pending<T>, String> {
    let term = term_of(id)?;
    let target = term.clone();
    term.commands.submit(move || work(target))
}

/// **A dead terminal row's page** (P5-SHELL items 1 and 2): how it ended,
/// its last screen when its host left one (`ring.bin`), else the last
/// commands its shell marked. Read only for a dead row of this machine's;
/// a live one is attached as ever, and never reads the file.
#[derive(Debug, Clone, Serialize, TS)]
pub struct EndedPage {
    pub ended: crate::RowEnding,
    pub screen: Option<EndedScreen>,
    /// Newest first.
    pub recent: Vec<EndedCommand>,
}

/// The last screen: what a reconnecting face would have been handed, and
/// the size it was laid out for.
#[derive(Debug, Clone, Serialize, TS)]
pub struct EndedScreen {
    pub cols: u16,
    pub rows: u16,
    /// base64 of the bytes, as [`TermStart::bytes`].
    pub bytes: String,
    #[ts(type = "number")]
    pub at_ms: u64,
}

/// One command from the dead session's ledger (`commands.json`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct EndedCommand {
    pub text: String,
    pub cwd: Option<String>,
    pub exit: Option<i32>,
    #[ts(type = "number")]
    pub started_ms: u64,
    #[ts(type = "number | null")]
    pub ended_ms: Option<u64>,
}

/// [`EndedPage`] for `id`; `None` while the session is not dead here.
pub fn ended(root: &Path, id: &str) -> Result<Option<EndedPage>, String> {
    let n = Node::open(root.to_path_buf())?;
    Ok(n.ended(&SessionId(id.to_owned())).map(|e| EndedPage {
        ended: e.ending.into(),
        screen: e.screen.map(|r| EndedScreen {
            cols: r.cols,
            rows: r.rows,
            bytes: B64.encode(r.bytes),
            at_ms: r.at_ms,
        }),
        recent: e
            .recent
            .into_iter()
            .rev()
            .filter_map(|c| {
                Some(EndedCommand {
                    text: c.text?,
                    cwd: c.cwd,
                    exit: c.exit,
                    started_ms: c.started_ms,
                    ended_ms: c.ended_ms,
                })
            })
            .collect(),
    }))
}

/// One command line from a machine's history (roadmap 2.6): its newest
/// run, and how many times it ran.
#[derive(Debug, Clone, Serialize, TS)]
pub struct HistoryRow {
    pub session: String,
    pub text: String,
    pub cwd: Option<String>,
    #[ts(type = "number")]
    pub started_ms: u64,
    #[ts(type = "number | null")]
    pub ended_ms: Option<u64>,
    pub exit: Option<i32>,
    pub runs: u32,
}

/// **A machine's commands, searched** (roadmap 2.6): words in the line,
/// the directory it ran in or under, only the failed ones — asked of the
/// machine that keeps them (`khor_node::Node::history`), newest first.
pub async fn history(root: &Path, machine: &str, text: &str, dir: &str, failed: bool) -> Result<Vec<HistoryRow>, String> {
    let n = Node::open(root.to_path_buf())?;
    let ask = khor_node::commands::HistoryAsk {
        text: text.to_owned(),
        dir: dir.to_owned(),
        failed,
    };
    Ok(n
        .history(machine, &ask)
        .await?
        .into_iter()
        .map(|h| HistoryRow {
            session: h.session,
            text: h.text,
            cwd: h.cwd,
            started_ms: h.started_ms,
            ended_ms: h.ended_ms,
            exit: h.exit,
            runs: h.runs,
        })
        .collect())
}

/// **The prompt before or after the one in view, for a tmux row** (item
/// 21): tmux holds that screen and its history, so tmux moves there
/// (`khor_node::Node::jump_prompt_anywhere`), on the row's own machine.
pub async fn prompt(root: &Path, id: &str, up: bool) -> Result<(), String> {
    let n = Node::open(root.to_path_buf())?;
    n.jump_prompt_anywhere(&SessionId(id.to_owned()), up).await
}

/// **The wheel over a shell's own tmux client** (item 58,
/// `Node::tmux_wheel_anywhere`): `rows` up when positive. False when no
/// tmux client is in front of the row's terminal: the face keeps the
/// wheel. 0 rows only asks.
pub async fn scroll(root: &Path, id: &str, rows: i32) -> Result<bool, String> {
    let n = Node::open(root.to_path_buf())?;
    n.tmux_wheel_anywhere(&SessionId(id.to_owned()), rows).await
}

/// Where the face's copy of a tmux pane's history ends (item 31,
/// `khor_node::adaptor::tmux::HistoryAnchor`): handed back as it came.
#[derive(Debug, Clone, Serialize, serde::Deserialize, TS)]
pub struct PaneAnchor {
    pub pane: String,
    pub size: u32,
    pub tail: Vec<String>,
}

/// One read of a tmux row's front pane history (item 31,
/// `khor_node::adaptor::tmux::PaneHistory`, whose fields these are).
#[derive(Debug, Clone, Serialize, TS)]
pub struct PaneHistory {
    pub pane: String,
    pub alternate: bool,
    pub refill: bool,
    pub lead: String,
    pub lines: Vec<String>,
    pub screen: Vec<String>,
    pub anchor: PaneAnchor,
    /// Rows above the pane on khor's client (a status line at the top, a
    /// split), and columns left of it.
    pub top: u16,
    pub left: u16,
    /// The pane's program holds the mouse (item 36): it scrolls itself.
    pub mouse: bool,
    /// tmux's window size at the read, `WxH` (item 45).
    pub window: String,
}

/// **A tmux row's front pane history from an anchor on** (item 31,
/// `khor_node::Node::pane_history_anywhere`): what the phone scrolls.
pub async fn pane_history(root: &Path, id: &str, since: Option<PaneAnchor>) -> Result<PaneHistory, String> {
    use khor_node::adaptor::tmux::HistoryAnchor;
    let n = Node::open(root.to_path_buf())?;
    let since = since.map(|a| HistoryAnchor { pane: a.pane, size: a.size as usize, tail: a.tail });
    let h = n.pane_history_anywhere(&SessionId(id.to_owned()), since).await?;
    Ok(PaneHistory {
        pane: h.pane,
        alternate: h.alternate,
        refill: h.refill,
        lead: h.lead,
        lines: h.lines,
        screen: h.screen,
        anchor: PaneAnchor {
            pane: h.anchor.pane,
            size: h.anchor.size.min(u32::MAX as usize) as u32,
            tail: h.anchor.tail,
        },
        top: h.top,
        left: h.left,
        mouse: h.mouse,
        window: h.window,
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
        stall: plock(&term.stall).clone(),
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
    submit_key(id, bytes)?.wait()
}
pub fn submit_key(id: &str, bytes: Vec<u8>) -> Result<crate::command_queue::Pending<()>, String> {
    submit(id, move |term| write_input(&mut *plock(&term.conn), &bytes))
}

/// The most input bytes one frame to the host carries (T-16, the terminal
/// audit). The host takes at most 1 MiB a frame (`host::MAX_OP`) and skips
/// a bigger one whole, so a paste of 1.2 MB once arrived as nothing.
const INPUT_FRAME: usize = 64 * 1024;

/// Input in frames of at most [`INPUT_FRAME`], in order, under the one
/// connection lock the caller holds — so no other keystroke lands between
/// two pieces of a paste. A cut may fall inside a UTF-8 character: the pty
/// reads one byte stream, and the pieces join again there.
fn write_input(conn: &mut (impl std::io::Write + ?Sized), bytes: &[u8]) -> Result<(), String> {
    for piece in bytes.chunks(INPUT_FRAME) {
        write_frame(conn, &ClientOp::Input(piece.to_vec()))?;
    }
    Ok(())
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
fn term_paste(term: Arc<Term>, text: String, stopped: Arc<AtomicBool>) -> Result<crate::command_queue::Pending<()>, String> {
    let target = term.clone();
    term.commands.submit(move || {
        if stopped.load(Ordering::Relaxed) { return Err(khor_node::drops::cancelled()); }
        if target.left.load(Ordering::Relaxed) { return Err("attachment is closed".into()); }
        paste(&target, &text, &stopped)
    })
}

fn paste(term: &Term, text: &str, stopped: &AtomicBool) -> Result<(), String> {
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
    // Cancellation may arrive while the parser or writer is locked. The
    // paste becomes committed only when this check permits its first bytes.
    if stopped.load(Ordering::Relaxed) {
        return Err(khor_node::drops::cancelled());
    }
    if term.left.load(Ordering::Relaxed) {
        return Err("attachment is closed".into());
    }
    write_input(&mut *conn, &bytes)
}

/// One path, written so a POSIX shell reads back exactly that path.
///
/// **Single quotes when it needs any** (a plain path goes in bare, the
/// list below being only what is *allowed* bare). Inside them every
/// byte is literal — spaces, `$`, backticks, `*`, newlines, semicolons — so
/// there is one rule instead of a list of characters somebody has to
/// remember to keep in step with the shell. The single quote is the one
/// character a single-quoted string cannot hold, and it is closed,
/// escaped, and reopened (`'\''`).
///
/// This is the whole reason dropping a file into a terminal is safe to
/// offer: an unquoted path with a space in it is two arguments, and one
/// with a newline is a second command.
pub fn shell_quote(path: &str) -> String {
    // **Bare when every character is one no shell reads** (item 60,
    // iTerm's manner): letters and digits in any script, and a short
    // list that sh, bash, zsh and fish all take as themselves — `=`
    // only past the first character, where zsh's `=cmd` cannot start.
    let plain = !path.is_empty()
        && !path.starts_with('=')
        && path
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+' | ',' | '@' | '%' | ':' | '='));
    if plain {
        return path.to_owned();
    }
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

/// Puts files on another machine for an agent conversation that runs
/// there, answering where they landed **over there**: the drop road
/// (`Node::drop_file`) — that machine pulls each one itself, into its
/// receive directory or the absolute directory `into` names. ⚠️ 墓碑
/// (STORE-BRIEF phase 5): this sent a file message into the machine's
/// `chat/<机器>` conversation and took it; that conversation is gone, and
/// a file handed to an agent is not a message for the broadcast.
///
/// Its own thread with its own runtime, for the reason
/// [`reach_blocking`] spells out: this is called from a tauri command,
/// which is already async, and a nested runtime panics instead of
/// returning something a person can read.
pub(crate) fn deliver_blocking(root: &Path, machine: &str, paths: &[String], into: &str) -> Result<Vec<String>, String> {
    let (root, machine, paths, into) = (root.to_path_buf(), machine.to_owned(), paths.to_vec(), into.to_owned());
    std::thread::spawn(move || -> Result<Vec<String>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(khor_catalog::msg::runtime_wont_start)?;
        let n = Node::open(root)?;
        let into = (!into.is_empty()).then_some(into.as_str());
        let mut landed = Vec::with_capacity(paths.len());
        for p in &paths {
            landed.push(rt.block_on(n.drop_file("", p, &machine, into, |_| {}))?);
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
///
/// `from` names the machine the files are on, `None` being this one: a
/// drag from a listing of another machine's disk (item 60). The road is
/// `Node::drop_file` — nothing moves when the files are already on the
/// session's machine; otherwise that machine pulls them into its
/// `~/khor-drop` itself, and [`term_drop_progress`] reads how far it got.
///
/// **A file a face staged** ([`term_drop_stage`]) is read by nobody once
/// the drop has landed or been cancelled, and goes then. **A drop that
/// failed keeps it** (NOTICE-BRIEF): the pill offers 重试, which drops the
/// same staged paths again, and 取消, which is [`term_drop_cancel`] with
/// them. Staged for a session
/// on this machine, it is not pasted where it waits but moved into this
/// machine's `~/khor-drop`, where a picked file lands on any machine.
///
/// **`into`, the shell's current directory** (item 60 part 5, 「放到当前
/// 目录」): the files land there on the session's machine in place of
/// its `~/khor-drop`, and a file already on that machine is copied
/// there.
///
/// **Cancelled** ([`term_drop_cancel`], item 71, the pill's ×): the
/// drop stops before its next file and its pull before the next slice,
/// and **nothing is pasted** — not even the files that had already
/// landed, which stay where they landed. It answers the cancel, which
/// the face does not show.
///
/// **Answers where the files landed** (absolute, on the session's
/// machine), so the face can tell that machine's file trees which levels
/// changed (UX-43 §1) — **or what went wrong, in parts** ([`Miss`]):
/// the kind, the machine, and the raw words for 详情 alone. ⚠️ 墓碑
/// (NOTICE-BRIEF): the failure was one sentence with the dial's English
/// inside it, shown whole in red over the terminal (IMG_5413).
pub fn term_drop(
    root: &Path,
    id: &str,
    paths: &[String],
    from: Option<&str>,
    into: Option<&str>,
) -> DropEnd {
    begin_drop(id).run_paths(root, paths, from, into)
}

/// Register cancellation and capture the attachment on the caller's thread.
/// Transfers run later, but neither a late cancel nor a reopened id can change
/// which job/terminal this request belongs to.
pub struct DropJob {
    held: Held,
    attachment: Result<Arc<Term>, String>,
}

pub fn begin_drop(id: &str) -> DropJob {
    DropJob { held: Held::begin(id), attachment: term_of(id) }
}

impl DropJob {
    pub fn run_paths(self, root: &Path, paths: &[String], from: Option<&str>, into: Option<&str>) -> DropEnd {
        let items = paths.iter().map(|path| DropItem {
            path: path.clone(), from: from.map(str::to_owned), keep: false,
        }).collect::<Vec<_>>();
        self.run(root, &items, into)
    }

    pub fn run(self, root: &Path, items: &[DropItem], into: Option<&str>) -> DropEnd {
        let id = &self.held.id;
        let ended = drop_items_text(root, id, items, into, &self.held).and_then(|(landed, text)| {
            if !text.is_empty() {
                term_paste(self.attachment.clone()?, text, self.held.stopped.clone())?.wait()?;
            }
            Ok(landed)
        });
        match ended {
            Ok(landed) => DropEnd { landed, miss: None },
            Err(why) => {
                let far = Node::open(root.to_path_buf()).ok()
                    .and_then(|n| n.far_machine(&SessionId(id.to_owned())).ok().flatten());
                let mut miss = crate::miss::read(&why, far.as_deref());
                if miss.kind == MissKind::Cancelled { miss.raw.clear(); }
                DropEnd { landed: Vec::new(), miss: Some(miss) }
            }
        }
    }
}

/// One source per item: a selection may span this device and several peers.
#[derive(Debug, Clone, serde::Deserialize, Serialize, TS)]
pub struct DropItem {
    pub path: String,
    pub from: Option<String>,
    /// App-owned native picks stay available if the operator cancels or retries.
    #[serde(default)]
    pub keep: bool,
}

pub fn term_drop_items(root: &Path, id: &str, items: &[DropItem], into: Option<&str>) -> DropEnd {
    begin_drop(id).run(root, items, into)
}

/// What [`term_drop`] answers: where the files landed, or the failure.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DropEnd {
    pub landed: Vec<String>,
    pub miss: Option<Miss>,
}

/// [`term_drop`] up to its paste: the files put where the session reads
/// them, and their paths quoted as one paste — or the cancel, if one came.
#[cfg(test)]
fn drop_text(
    root: &Path,
    id: &str,
    paths: &[String],
    from: Option<&str>,
    into: Option<&str>,
    held: &Held,
) -> Result<(Vec<String>, String), String> {
    let items = paths
        .iter()
        .map(|path| DropItem {
            path: path.clone(),
            from: from.map(str::to_owned),
            keep: false,
        })
        .collect::<Vec<_>>();
    drop_items_text(root, id, &items, into, held)
}

fn drop_items_text(
    root: &Path,
    id: &str,
    items: &[DropItem],
    into: Option<&str>,
    held: &Held,
) -> Result<(Vec<String>, String), String> {
    let n = Node::open(root.to_path_buf())?;
    let to = n
        .far_machine(&SessionId(id.to_owned()))?
        .unwrap_or_default();
    let here = if to.is_empty()
        && into.is_none()
        && items
            .iter()
            .all(|i| i.from.as_deref().unwrap_or("").is_empty())
    {
        items
            .iter()
            .map(|item| match staged_dir(root, &item.path) {
                _ if held.stopped() => Err(khor_node::drops::cancelled()),
                Some(_) => {
                    let mut job = khor_node::drops::Job::default();
                    job.stopped = held.stopped.clone();
                    khor_node::drops::copy_into(Path::new(&item.path), &n.receive_dir()?, &job)
                        .map(|p| p.display().to_string())
                }
                None => Ok(item.path.clone()),
            })
            .collect::<Result<Vec<_>, _>>()
    } else {
        drop_items_blocking(root, id, &to, into, items, &held.stopped)
    };
    if let Err(why) = &here
        && !held.stopped()
    {
        return Err(why.clone());
    }
    term_drop_forget(
        root,
        &items
            .iter()
            .filter(|i| !i.keep && i.from.as_deref().unwrap_or("").is_empty())
            .map(|i| i.path.clone())
            .collect::<Vec<_>>(),
    );
    let here = here?;
    if held.stopped() {
        return Err(khor_node::drops::cancelled());
    }
    let text = here
        .iter()
        .map(|p| shell_quote(p))
        .collect::<Vec<_>>()
        .join(" ");
    Ok((here, text))
}

/// Where a face's picked files wait to be dropped ([`term_drop_stage`]):
/// under this machine's store, which its node serves by path like any
/// other file (`FetchPath`) — the session's machine pulls from here.
fn stage_root(root: &Path) -> std::path::PathBuf {
    root.join(".khor").join("uploads").join("drop")
}

/// How long a staged file is kept for a drop that never came (a face
/// that staged one file of three and then failed on the next). A drop
/// cancelled while it staged does not wait for this: its staged files
/// go with the cancel ([`term_drop_cancel`]).
const STAGED_FOR: std::time::Duration = std::time::Duration::from_secs(3600);

/// The staging directory `path` sits in, when it is a file
/// [`term_drop_stage`] made: exactly `<stage root>/<dir>/<name>`, with
/// no `..` to climb out by, since that directory is deleted.
fn staged_dir(root: &Path, path: &str) -> Option<std::path::PathBuf> {
    let p = Path::new(path);
    use std::path::Component;
    if p.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return None;
    }
    let dir = p.parent()?;
    (dir.parent()? == stage_root(root)).then(|| dir.to_path_buf())
}

/// **A file picked in a face, put where this machine's node serves it**
/// (item 60 part 3): the phone's photo or file arrives as bytes (a web
/// view has no path to name), and a drop needs a path. Written under
/// [`stage_root`] in a directory of its own, under its own name — the
/// name the session's machine lands it under. Capped at the drop's cap
/// as it is written, and a refused or broken write leaves nothing.
/// Answers the path, for [`term_drop`].
pub fn term_drop_stage(root: &Path, name: &str, body: &mut dyn std::io::Read) -> Result<String, String> {
    use std::io::Write;
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let base = stage_root(root);
    // Whatever a drop never came for, from an hour ago and more.
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.flatten() {
            let old = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > STAGED_FOR);
            if old {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dir = base.join(format!("{stamp}-{}-{}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = khor_node::transfer::safe_name(name);
    let path = dir.join(&name);
    let written = (|| -> Result<(), String> {
        let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        let mut taken: u64 = 0;
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = body.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                return Ok(());
            }
            taken += n as u64;
            if taken > khor_node::drops::CAP {
                return Err(khor_catalog::msg::drop_too_big(khor_node::drops::CAP >> 20, &name));
            }
            f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
    })();
    if let Err(why) = written {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(why);
    }
    Ok(path.display().to_string())
}

/// A staged file for a session on this machine: moved into its receive
/// directory (`Node::receive_dir`) under the drop's naming (a taken name
/// suffixed).
#[cfg(test)]
fn land_staged(root: &Path, staged: &str) -> Result<String, String> {
    khor_node::drops::copy_into(
        Path::new(staged),
        &Node::open(root.to_path_buf())?.receive_dir()?,
        &khor_node::drops::Job::default(),
    )
    .map(|p| p.display().to_string())
}

/// One terminal's drop under way: the share of it landed (0 to 1), and
/// whether it was cancelled.
struct Dropping {
    share: f32,
    stopped: Arc<AtomicBool>,
}

/// Each terminal's drop under way.
fn dropping() -> &'static Mutex<HashMap<String, Dropping>> {
    static DROPPING: OnceLock<Mutex<HashMap<String, Dropping>>> = OnceLock::new();
    DROPPING.get_or_init(Default::default)
}

/// A drop's place in [`dropping`], for as long as [`term_drop`] runs:
/// there from its first line, gone when it returns, however it returns.
struct Held {
    id: String,
    stopped: Arc<AtomicBool>,
}

impl Held {
    fn begin(id: &str) -> Held {
        let stopped = Arc::new(AtomicBool::new(false));
        plock(dropping()).insert(
            id.to_owned(),
            Dropping {
                share: 0.0,
                stopped: stopped.clone(),
            },
        );
        Held {
            id: id.to_owned(),
            stopped,
        }
    }

    fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut all = plock(dropping());
        // Its own entry only: a face never starts a second drop on one
        // terminal before the first is over, and this keeps it so.
        if all.get(&self.id).is_some_and(|d| Arc::ptr_eq(&d.stopped, &self.stopped)) {
            all.remove(&self.id);
        }
    }
}

/// **How far this terminal's drop has got** (item 60), for its pill's
/// line: the share landed, whole files and the bytes of the one under
/// way, or `None` when nothing is being dropped.
pub fn term_drop_progress(id: &str) -> Option<f32> {
    plock(dropping()).get(id).map(|d| d.share)
}

/// **The drop under way on terminal `id`, cancelled** (item 71, the
/// owner 2026-10-07: the pill's × while files go in). It stops before
/// its next file, the pull under way stops at its next slice on
/// whichever machine runs it, and [`term_drop`] pastes nothing. With no
/// drop on `id` it does nothing.
///
/// `staged` are files a face staged for a drop it will now never ask
/// for — the phone's picks, cancelled while they were being staged —
/// which go now rather than waiting out [`STAGED_FOR`]. Only what
/// [`term_drop_stage`] made is taken for one.
pub fn term_drop_cancel(root: &Path, id: &str, staged: &[String]) {
    cancel_drop(id);
    term_drop_forget(root, staged);
}

/// Set the current job's flag before an IPC handler returns; cleanup can wait.
pub fn cancel_drop(id: &str) {
    if let Some(d) = plock(dropping()).get(id) {
        d.stopped.store(true, Ordering::Relaxed);
    }
}

pub fn term_drop_forget(root: &Path, staged: &[String]) {
    for path in staged {
        if let Some(dir) = staged_dir(root, path) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// The files of one drop put on the session's machine, one after
/// another, answering where each landed over there; `stopped` ends it
/// before the next file and the pull under way at its next slice. Its
/// own thread and runtime, for [`reach_blocking`]'s reason.
fn drop_items_blocking(
    root: &Path,
    id: &str,
    to: &str,
    into: Option<&str>,
    items: &[DropItem],
    stopped: &Arc<AtomicBool>,
) -> Result<Vec<String>, String> {
    let (root, id, to, items) = (
        root.to_path_buf(),
        id.to_owned(),
        to.to_owned(),
        items.to_vec(),
    );
    let into = into.map(str::to_owned);
    let stopped = stopped.clone();
    std::thread::spawn(move || -> Result<Vec<String>, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(khor_catalog::msg::runtime_wont_start)?;
        let n = Node::open(root)?;
        let count = items.len().max(1) as f32;
        let mut landed = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            if stopped.load(Ordering::Relaxed) {
                return Err(khor_node::drops::cancelled());
            }
            let at = rt.block_on(n.drop_file_until(
                item.from.as_deref().unwrap_or(""),
                &item.path,
                &to,
                into.as_deref(),
                |state| {
                    let part = if state.total > 0 {
                        state.moved as f32 / state.total as f32
                    } else if state.done {
                        1.0
                    } else {
                        0.0
                    };
                    if let Some(d) = plock(dropping()).get_mut(&id)
                        && Arc::ptr_eq(&d.stopped, &stopped)
                    {
                        d.share = (i as f32 + part.min(1.0)) / count;
                    }
                },
                || stopped.load(Ordering::Relaxed),
            ))?;
            landed.push(at);
        }
        Ok(landed)
    })
    .join()
    .map_err(|_| khor_catalog::msg::deliver_thread_died().to_owned())?
}

/// **⌘K, kept where the bytes are** (audit/term-attach.md): this
/// attachment's feed forgets everything before the cursor's line, and so
/// does the host's ring when the host speaks version 2 — a reload or a
/// re-attach then starts from the clear, as iTerm2's Clear Buffer does.
/// `redraw` sends the shell its ⌃L after the forget, on the same
/// connection so the host takes them in that order. Answers whether the
/// host kept it (false: a host from before the handshake, where the
/// clear is this face's alone, as it always was).
pub fn term_forget(id: &str, redraw: bool) -> Result<bool, String> {
    submit_forget(id, redraw)?.wait()
}
pub fn submit_forget(id: &str, redraw: bool) -> Result<crate::command_queue::Pending<bool>, String> {
    submit(id, move |term| forget(&term, redraw))
}
fn forget(term: &Term, redraw: bool) -> Result<bool, String> {
    let kept = term.version.load(Ordering::Relaxed) >= 2;
    {
        let mut conn = plock(&term.conn);
        if kept {
            write_frame(&mut *conn, &ClientOp::Forget)?;
        }
        if redraw {
            write_frame(&mut *conn, &ClientOp::Input(vec![0x0c]))?;
        }
    }
    plock(&term.feed).forget();
    Ok(kept)
}

/// A resize: the PTY is told (so programs repaint at the new size) and
/// the local screen is set to match, so the bytes that repaint arrive
/// into a screen already the right shape.
pub fn term_resize(id: &str, cols: u16, rows: u16) -> Result<(), String> {
    submit_resize(id, cols, rows)?.wait()
}
pub fn submit_resize(id: &str, cols: u16, rows: u16) -> Result<crate::command_queue::Pending<()>, String> {
    submit(id, move |term| resize(&term, cols, rows))
}
fn resize(term: &Term, cols: u16, rows: u16) -> Result<(), String> {
    // Never a wide character split at the new edge (item 57b): the next
    // redraw over it panicked this oracle as it did the host's model.
    khor_node::host::resize_screen(&mut plock(&term.parser), rows, cols);
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
                let term = registry.remove(id).unwrap();
                drop(registry);
                term.commands.close();
                term.conn.close();
                return Ok(());
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
#[path = "term_io_tests.rs"]
mod io_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// T-16's second half: a paste of 3 MiB goes as frames the host
    /// takes (at most 64 KiB each, under its 1 MiB `MAX_OP`), in order,
    /// and together they are the paste.
    #[test]
    fn a_big_input_goes_as_frames_the_host_takes() {
        let paste: Vec<u8> = (0..3 * 1024 * 1024 + 7).map(|i| (i % 251) as u8).collect();
        let mut wire = Vec::new();
        write_input(&mut wire, &paste).unwrap();
        let mut read = std::io::Cursor::new(wire);
        let mut back = Vec::new();
        let mut frames = 0;
        while (read.position() as usize) < read.get_ref().len() {
            match khor_node::host::read_frame::<ClientOp>(&mut read).unwrap() {
                ClientOp::Input(piece) => {
                    assert!(piece.len() <= INPUT_FRAME, "a frame of {} bytes", piece.len());
                    back.extend_from_slice(&piece);
                }
                _ => panic!("only input was written"),
            }
            frames += 1;
        }
        assert_eq!(frames, 49);
        assert!(back == paste, "the frames join back into the paste");
    }

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

    /// **A trim never leaves the head inside a picture** (roadmap 3.3):
    /// an `imgcat` plot that straddles the cap, or is still arriving
    /// when the trim reaches it, is cut whole, so a face attaching then
    /// draws text and not base64. The cursor still counts every byte.
    #[test]
    fn a_trim_inside_a_picture_cuts_the_whole_picture() {
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        let lines = b"line of output\r\n".repeat(FEED_CAP / 16);
        push_feed(&feed, &fed, b"\x1b]1337;File=inline=1:");
        push_feed(&feed, &fed, &vec![b'Q'; 4096]);
        push_feed(&feed, &fed, &lines);
        push_feed(&feed, &fed, &vec![b'Q'; 4096]);
        push_feed(&feed, &fed, b"\x07after the picture\r\n");
        let (bytes, end) = take_all(&feed, &fed);
        assert!(bytes.starts_with(b"after the picture"), "{:?}", &bytes[..24]);
        assert_eq!(end, fed.load(Ordering::Relaxed));
        assert_eq!(
            end - plock(&feed).bytes.len() as u64,
            21 + 4096 + lines.len() as u64 + 4096 + 1,
            "the head cursor moved past the picture's end"
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
        // Three of these are 1.2 caps: the feed trims past its head.
        let chunk = b"line of output\r\n".repeat(FEED_CAP * 2 / 5 / 16);
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

    /// **A narrowing over CJK keeps the oracle whole** (item 57b): a
    /// program on the alternate screen with bracketed paste on and wide
    /// characters to the right edge, the face one column narrower
    /// (`term_resize`'s own call), then the program's redraw. vt100's
    /// resize had left a wide character split at the new edge, the redraw
    /// panicked the parser, and the fresh one forgot `?2004h` (a paste
    /// went unbracketed, item 55's question came back) and answered a
    /// face that fell behind with a blank screen.
    #[test]
    fn a_narrowing_over_wide_characters_keeps_the_modes_and_the_screen() {
        let acs = Mutex::new(crate::acs::Acs::default());
        let parser = Mutex::new(vt100::Parser::new(24, 80, 0));
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        // Past the cap first, so cursor 0 falls off the head.
        let chunk = b"line of output\r\n".repeat(FEED_CAP * 2 / 5 / 16);
        for _ in 0..3 {
            paint(&acs, &parser, &feed, &fed, &chunk);
        }
        let mut program = b"\x1b[?2004h\x1b[?1049h\x1b[H\x1b[2J".to_vec();
        for r in 1..=6 {
            program.extend_from_slice(format!("\x1b[{r};1H{}", "\u{4e2d}".repeat(40)).as_bytes());
        }
        assert_eq!(paint(&acs, &parser, &feed, &fed, &program), None);
        khor_node::host::resize_screen(&mut plock(&parser), 24, 79);
        let mut redraw = Vec::new();
        for r in 1..=6 {
            redraw.extend_from_slice(format!("\x1b[{r};1H{}\x1b[K", "\u{6587}".repeat(39)).as_bytes());
        }
        redraw.extend_from_slice(b"\x1b[7;1HREDRAWN\x1b[K");
        paint(&acs, &parser, &feed, &fed, &redraw);
        assert!(plock(&parser).screen().bracketed_paste(), "the oracle still knows the program asked for bracketed paste");
        let (bytes, _, reset) = take_since(&parser, &feed, &fed, 0);
        assert!(reset, "cursor 0 fell off the trimmed head");
        let mut face = vt100::Parser::new(24, 79, 0);
        face.process(&bytes);
        assert!(face.screen().alternate_screen(), "the face is on the program's screen");
        assert_eq!(
            face.screen().rows(0, 79).nth(6).unwrap().trim_end(),
            "REDRAWN",
            "a face that fell behind is handed the screen, not a blank"
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
    /// **⌘K forgets up to the cursor's line** (audit/term-attach.md): a
    /// face that starts over is handed the prompt and what follows, in
    /// the modes the forgotten bytes had set; a face already past the
    /// clear goes on where it was.
    #[test]
    fn a_forgotten_feed_starts_at_the_cursors_line_in_its_modes() {
        let feed = Mutex::new(Feed::default());
        let fed = AtomicU64::new(0);
        push_feed(&feed, &fed, b"\x1b[?2004hsecret one\r\nsecret two\r\n$ ls");
        let end = fed.load(Ordering::Relaxed);
        plock(&feed).forget();
        let (all, _) = take_all(&feed, &fed);
        assert_eq!(all, b"\x1b[?2004h$ ls", "the modes, then the line the cursor is on");
        push_feed(&feed, &fed, b"\r\nafter");
        let parser = Mutex::new(vt100::Parser::new(3, 10, 0));
        let (tail, _, reset) = take_since(&parser, &feed, &fed, end);
        assert!(!reset && tail == b"\r\nafter", "{tail:?}");
    }

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
        // A little past the cap, in lines of 1 KiB.
        for _ in 0..FEED_CAP / 1024 + 88 {
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
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            parser: Mutex::new(vt100::Parser::new(3, 10, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
            version: AtomicU32::new(1),
            history: AtomicU32::new(0),
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

    /// **A face that joins an attachment already open still says its
    /// size** (item 33): another face may have sized the pty since, and
    /// tmux then drew its client at that size under this face — the
    /// status line in mid-screen, the lower rows out of tmux's reach —
    /// until the first key or the keyboard resized it.
    #[test]
    fn an_open_that_joins_a_held_attachment_sends_its_size() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut host, _) = listener.accept().unwrap();
        host.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
        let term = Arc::new(Term {
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            parser: Mutex::new(vt100::Parser::new(3, 10, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
            version: AtomicU32::new(1),
            history: AtomicU32::new(0),
        });
        let id = format!("shell/held-size-{}", std::process::id());
        plock(terms()).insert(id.clone(), term.clone());

        term_open(Path::new("/nonexistent"), &id, 46, 30, 0).unwrap();
        assert_eq!(term.holds.load(Ordering::Relaxed), 2, "held, not attached again");
        match khor_node::host::read_frame::<ClientOp>(&mut host) {
            Ok(ClientOp::Resize { cols, rows }) => assert_eq!((cols, rows), (46, 30)),
            Ok(_) => panic!("something other than a resize reached the host"),
            Err(e) => panic!("no resize reached the host: {e}"),
        }
        assert_eq!(plock(&term.parser).screen().size(), (30, 46), "the mode oracle follows");

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
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            parser: Mutex::new(vt100::Parser::new(3, 10, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
            version: AtomicU32::new(1),
            history: AtomicU32::new(0),
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

    /// **Bare when nothing in it needs quoting** (item 60, iTerm's
    /// manner): a plain path pastes as itself, quotes only where a space
    /// or a character the shell reads would change it.
    #[test]
    fn a_plain_path_goes_in_bare() {
        for path in ["/tmp/plain.txt", "/Users/a/khor-drop/b-2.tar.gz", "/tmp/中文.txt", "a@b:c,d+e=f%"] {
            assert_eq!(shell_quote(path), path);
        }
        for path in ["/tmp/a b", "~/x", "/tmp/a#b", "/tmp/a!b", "/tmp/[x]", "/tmp/a{b}", "=ls", ""] {
            assert!(shell_quote(path).starts_with('\''), "{path:?} is quoted");
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
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            parser: Mutex::new(vt100::Parser::new(24, 80, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
            version: AtomicU32::new(1),
            history: AtomicU32::new(0),
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

#[cfg(test)]
mod stage_tests {
    use super::*;
    use std::io::Read;

    fn fresh(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("khor-stage-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// **A picked file waits under its own name in a directory of its
    /// own** (item 60 part 3): the name the session's machine lands it
    /// under, and the directory a drop deletes when it is over.
    #[test]
    fn a_staged_file_keeps_its_name_in_a_directory_of_its_own() {
        let root = fresh("name");
        let a = term_drop_stage(&root, "IMG 0001.HEIC", &mut &b"photo"[..]).unwrap();
        let b = term_drop_stage(&root, "IMG 0001.HEIC", &mut &b"other"[..]).unwrap();
        assert_ne!(a, b, "the same name twice is two files");
        for (p, bytes) in [(&a, "photo"), (&b, "other")] {
            let p = Path::new(p);
            assert_eq!(p.file_name().unwrap(), "IMG 0001.HEIC");
            assert_eq!(p.parent().unwrap().parent().unwrap(), stage_root(&root));
            assert_eq!(std::fs::read_to_string(p).unwrap(), bytes);
        }
        // A name is not a path.
        let c = term_drop_stage(&root, "../../.ssh/x", &mut &b"z"[..]).unwrap();
        assert_eq!(Path::new(&c).file_name().unwrap(), "sshx");
        assert!(staged_dir(&root, &c).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Only a file the stage made is taken for one: never a path that
    /// climbs out of it, nor one beside it, whose directory a drop
    /// would otherwise delete.
    #[test]
    fn only_a_staged_file_is_taken_for_one() {
        let root = fresh("which");
        let a = term_drop_stage(&root, "a.txt", &mut &b"a"[..]).unwrap();
        assert_eq!(staged_dir(&root, &a).as_deref(), Path::new(&a).parent());
        let base = stage_root(&root);
        let climbs = format!("{}/x/../../../a.txt", base.display());
        assert_eq!(staged_dir(&root, &climbs), None);
        assert_eq!(staged_dir(&root, &base.join("a.txt").display().to_string()), None);
        assert_eq!(staged_dir(&root, &root.join("x/a.txt").display().to_string()), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A cancelled drop pastes nothing** (item 71): cancelled while it
    /// ran, it answers the cancel instead of the paste's text, lands no
    /// file it had not landed yet, and its staged files go.
    #[test]
    fn a_cancelled_drop_pastes_nothing() {
        let root = fresh("cancel");
        let a = term_drop_stage(&root, "a.png", &mut &b"one"[..]).unwrap();
        let b = term_drop_stage(&root, "b.png", &mut &b"two"[..]).unwrap();
        let held = Held::begin("stage-test/cancel");
        assert_eq!(term_drop_progress("stage-test/cancel"), Some(0.0), "held from the start");
        term_drop_cancel(&root, "stage-test/cancel", &[]);
        let out = drop_text(&root, "stage-test/cancel", &[a.clone(), b.clone()], None, None, &held);
        assert_eq!(out, Err(khor_node::drops::cancelled()), "the cancel, not a paste");
        assert!(!Path::new(&a).exists() && !Path::new(&b).exists(), "the staged files went");
        assert!(!khor_node::drops::dir_for(&root).join("a.png").exists(), "nothing landed");
        drop(held);
        assert_eq!(term_drop_progress("stage-test/cancel"), None, "gone when it returns");
        // Uncancelled, the same road gives the paste.
        let c = term_drop_stage(&root, "c.png", &mut &b"three"[..]).unwrap();
        let held = Held::begin("stage-test/cancel");
        let (landed, text) = drop_text(&root, "stage-test/cancel", &[c], None, None, &held).unwrap();
        let at = khor_node::drops::dir_for(&root).join("c.png").display().to_string();
        assert_eq!(text, at);
        // Where it landed, for the face's trees (UX-43 §1).
        assert_eq!(landed, vec![at]);
    }

    #[test]
    fn native_folder_selection_survives_cancel_and_retry() {
        let root = fresh("native-folder-retry");
        let folder = term_drop_folder(&root, "data", &["nested/empty".into()]).unwrap();
        term_drop_stage_into(&root, &folder, "nested/a.txt", &mut &b"abc"[..]).unwrap();
        let items = [DropItem {
            path: folder.clone(),
            from: None,
            keep: true,
        }];
        let held = Held::begin("stage-test/native-retry");
        term_drop_cancel(&root, "stage-test/native-retry", &[]);
        assert!(drop_items_text(&root, "stage-test/native-retry", &items, None, &held).is_err());
        assert!(Path::new(&folder).join("nested/a.txt").is_file());
        drop(held);
        let held = Held::begin("stage-test/native-retry");
        let (landed, text) =
            drop_items_text(&root, "stage-test/native-retry", &items, None, &held).unwrap();
        assert_eq!(text, shell_quote(&landed[0]));
        assert!(Path::new(&landed[0]).join("nested/empty").is_dir());
        assert_eq!(
            std::fs::read(Path::new(&landed[0]).join("nested/a.txt")).unwrap(),
            b"abc"
        );
        assert!(
            Path::new(&folder).is_dir(),
            "selection owns native token until completion is acknowledged"
        );
        term_drop_forget(&root, &[folder]);
        assert!(
            Path::new(&landed[0]).is_dir(),
            "releasing selection cannot delete its landed directory"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn failed_batch_keeps_earlier_staged_sources_for_retry() {
        let root = fresh("batch-retry");
        let a = term_drop_stage(&root, "a.txt", &mut &b"abc"[..]).unwrap();
        let b = term_drop_stage(&root, "missing.txt", &mut &b"def"[..]).unwrap();
        std::fs::remove_file(&b).unwrap();
        let held = Held::begin("stage-test/batch-retry");
        assert!(
            drop_text(
                &root,
                "stage-test/batch-retry",
                &[a.clone(), b],
                None,
                None,
                &held
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(a).unwrap(),
            b"abc",
            "an earlier landed file does not consume retry input"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// **A cancel with no drop does nothing** — it does not wait for
    /// the next drop on that terminal — and a cancel while the face was
    /// still staging takes the files it had staged.
    #[test]
    fn a_cancel_with_no_drop_does_nothing_but_take_what_was_staged() {
        let root = fresh("cancel-none");
        term_drop_cancel(&root, "stage-test/none", &[]);
        let held = Held::begin("stage-test/none");
        assert!(!held.stopped(), "the next drop is not born cancelled");
        drop(held);
        let a = term_drop_stage(&root, "a.png", &mut &b"one"[..]).unwrap();
        let beside = root.join("keep.txt");
        std::fs::write(&beside, b"x").unwrap();
        term_drop_cancel(&root, "stage-test/none", &[a.clone(), beside.display().to_string()]);
        assert!(!Path::new(&a).parent().unwrap().exists(), "the staged file's directory went");
        assert!(beside.exists(), "a path the face did not stage is not taken for one");
    }

    /// Over the drop's cap: refused with the cap in the sentence, and
    /// nothing of it left behind.
    #[test]
    fn over_the_cap_is_refused_and_leaves_nothing() {
        let root = fresh("cap");
        let why = term_drop_stage(&root, "big.mov", &mut std::io::repeat(7).take(khor_node::drops::CAP + 1)).unwrap_err();
        assert!(why.contains(&(khor_node::drops::CAP >> 20).to_string()), "{why}");
        assert_eq!(std::fs::read_dir(stage_root(&root)).unwrap().count(), 0, "no staged file, no directory");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// For a session on this machine a staged file is moved into its
    /// drop directory, a taken name suffixed as any drop's is.
    #[test]
    fn a_staged_file_for_a_session_here_lands_in_the_drop_directory() {
        let root = fresh("here");
        let a = term_drop_stage(&root, "a b.png", &mut &b"one"[..]).unwrap();
        let b = term_drop_stage(&root, "a b.png", &mut &b"two"[..]).unwrap();
        let drop_dir = khor_node::drops::dir_for(&root);
        assert_eq!(land_staged(&root, &a).unwrap(), drop_dir.join("a b.png").display().to_string());
        assert_eq!(land_staged(&root, &b).unwrap(), drop_dir.join("a b-2.png").display().to_string());
        let mut names: Vec<_> = std::fs::read_dir(&drop_dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["a b-2.png", "a b.png"], "no part file left");
        assert_eq!(std::fs::read(drop_dir.join("a b-2.png")).unwrap(), b"two");
        let _ = std::fs::remove_dir_all(&root);
    }
}
