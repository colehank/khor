//! The persistent host: one detached process per 持久 session, owning
//! the PTY its command runs in (docs/SESSION.md 寿命). The opener's
//! terminal can go away; the host stays, keeps a replay ring for late
//! attachers, polls the six-word face, and records the ending. One
//! session, one host, one socket — close kills exactly one thing.
//!
//! Not tmux embedded and not tmux shelled out (the product expects no
//! user-installed dependency): the host is the ~10% of tmux this product
//! needs — PTY + lifetime + replay — on portable-pty, which is the same
//! API on every desktop OS.
//!
//! Local protocol, loopback TCP like `ipc.rs` (same reasoning, same
//! cookie discipline): the client sends one `Hello` frame, the host
//! answers one `Welcome` frame, then the host streams raw PTY bytes and
//! the client sends framed [`ClientOp`]s. Framing is 4-byte big-endian
//! length + msgpack. The replay ring is raw bytes, not a screen model —
//! good enough because attaching resizes the PTY, and the resize makes
//! any full-screen program repaint itself; a real terminal model waits
//! for the GUI batch. **A repaint redraws the screen, not the modes**
//! (the alternate buffer, cursor keys, the mouse, paste), so the bytes
//! the ring trims go through [`Modes`] and a replay opens with them
//! ([`Ring`]).

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use khor_catalog::msg;
use khor_core::{SessionId, State};

use crate::beside::{Beside, RowMarks};
use crate::modes::Modes;
use crate::pictures::Pictures;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};

use crate::live::LiveKind;
use crate::proto;

/// Every new PTY describes khor's terminal, regardless of which CLI, GUI,
/// daemon or automation environment happened to create its host. Application
/// identity never changes this contract. User shell startup files and an
/// explicit `env NO_COLOR=1 command` can still choose a different preference.
struct TerminalContext;

impl TerminalContext {
    fn apply(builder: &mut CommandBuilder) {
        builder.env("TERM", "xterm-256color");
        builder.env("TERM_PROGRAM", "khor");
        builder.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        builder.env("COLORTERM", "truecolor");
        // Color switches in the opener describe its output, not this new
        // interactive terminal. NO_COLOR from an automation launcher used
        // to suppress color inside every shell/TUI and tmux it created.
        for name in [
            "NO_COLOR", "FORCE_COLOR", "CLICOLOR", "CLICOLOR_FORCE",
            "CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_CHILD_SESSION",
            "TERM_SESSION_ID", "TERM_FEATURES", "TMUX", "TMUX_PANE",
        ] {
            builder.env_remove(name);
        }
        for (name, _) in std::env::vars_os() {
            if let Some(name) = name.to_str()
                && (name.starts_with("LC_TERMINAL") || name.starts_with("ITERM_"))
            {
                builder.env_remove(name);
            }
        }
        // TMUX_TMPDIR and TERMINFO paths belong to the user's installation,
        // unlike parent-client identity, and must remain available.
    }
}

/// Where attachers find the host, beside meta/state in the session dir.
/// Owner-only: the cookie is the capability, the port gates nothing.
#[derive(Serialize, Deserialize)]
pub struct HostFile {
    pub port: u16,
    pub cookie: String,
    pub host_pid: u32,
    pub child_pid: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Hello {
    pub cookie: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Serialize, Deserialize)]
pub struct Welcome {
    pub ok: bool,
    pub why: String,
}

#[derive(Serialize, Deserialize)]
pub enum ClientOp {
    Input(#[serde(with = "serde_bytes")] Vec<u8>),
    Resize { cols: u16, rows: u16 },
    /// **The client's protocol version** ([`HOST_VERSION`]), sent once,
    /// first, and only to a host whose `Welcome` advertised version 2 or
    /// later: the two then speak the lower of the two. Tail-appended, like
    /// every op after it — a host older than the handshake does not know
    /// these words and would drop the client's input on the first one, so
    /// none is ever sent to it.
    Version(u32),
    /// **⌘K, kept by the host** (version 2): the ring forgets everything
    /// before the line the cursor is on, so a face that attaches later —
    /// a reload, a re-attach, another face — is not handed the history
    /// the person cleared.
    Forget,
    /// **The face's 历史行数** (version 4, item 56): the rows of history
    /// it shows, said once after `Version`. The host's model keeps the
    /// most any face has asked for, up to [`MODEL_SCROLLBACK_MOST`].
    Scrollback(u32),
    /// The terminal has parsed its initial replay and can answer live
    /// queries (version 5). Releases this host's initial shell input once.
    Ready,
}

/// **The host protocol's version** (audit/term-attach.md). 1 is every
/// host and client before the handshake: `Hello`, `Welcome`, raw bytes,
/// `Input` and `Resize`. 2 adds [`ClientOp::Version`] and
/// [`ClientOp::Forget`]. 3 (item 46) changes what an attacher is handed
/// first: a client that says `Version(3)` gets the host's **snapshot**
/// ([`Model::snapshot`]: the scrollback as styled lines, the screen with
/// its attributes and cursor, the modes) instead of the ring's bytes; an
/// older client still gets the ring. No new frame: the host waits
/// [`FIRST_FRAME`] for the client's first word before choosing. 4 adds
/// [`ClientOp::Scrollback`] (item 56). 5 adds [`ClientOp::Ready`].
///
/// **How it is agreed without changing a frame's shape.** msgpack here
/// is positional, and a struct read from a longer array fails
/// (`LengthMismatch`, measured on rmp-serde 1.3): a `Hello` or `Welcome`
/// with one more field would lock out every host or client already
/// installed. So a host of version 2 or later says so in the `why` of a
/// successful `Welcome` — a field an older client only reads when the
/// answer is no — and a client that sees it answers with
/// [`ClientOp::Version`] before anything else. Neither side sends a word
/// the other has not said it knows.
pub const HOST_VERSION: u32 = 5;

/// How long a host waits after its `Welcome` for the client's first frame
/// before handing it the ring (item 46): a versioned client's `Version`
/// is its first write and arrives in a round trip — loopback, since far
/// faces attach through the node beside the host — and a client older
/// than the handshake, which never sends one, pays this once per attach.
const FIRST_FRAME: Duration = Duration::from_millis(300);

/// The `why` of a successful `Welcome` from a versioned host: this, then
/// the version.
const GREETING: &str = "khor-host/";

/// The version a `Welcome` advertises: 1 for a host older than the
/// handshake, which leaves `why` empty when it welcomes.
fn advertised(w: &Welcome) -> u32 {
    w.why.strip_prefix(GREETING).and_then(|v| v.parse().ok()).unwrap_or(1)
}

/// A connection with the version both ends agreed to speak.
pub struct Attached {
    pub stream: TcpStream,
    pub version: u32,
}

/// Above any sane keystroke burst, far below anything hostile-sized.
const MAX_OP: usize = 1 << 20;
/// **Replay budget: what a re-attach gets back** (item 17). 1 MiB holds the
/// last full repaint of anything and, measured, about 13 000–26 000 lines
/// of ordinary output (42–80 bytes a line: source 42, `git log` 56–57,
/// `ls -la` in colour 80), so a face keeping 10 000 lines of history
/// (`lib/term-history`) gets them back after a re-attach. It was 256 KiB
/// (3 300–6 500 lines).
///
/// What follows from it, kept in step: each client may fall
/// [`CLIENT_BEHIND`] (2 × RING) behind before it is let go; the app's feed
/// per attachment is `khor_gui_core::term::FEED_CAP` (2 × RING), which is
/// also where a face that falls behind is handed the screen instead (T-12);
/// a live session holds one ring, so 1 MiB a session in its host process.
/// The replay crosses a relay as the first thing a far face reads: at the
/// net lane's measured worst road (300 ms, 2 % loss, 0.33 Mbit/s) 1 MiB
/// takes about 25 s against 6 s for 256 KiB, on the measured ordinary
/// relay (1.4 MB/s) 0.7 s. Nothing waits for it to finish — the face draws
/// what has come and the rest streams on — but on a road that bad the
/// history fills in for that long; replaying the tail by lines (the last
/// N lines the face keeps) is the way to cut it if that matters.
pub const RING: usize = 1 << 20;

pub fn host_file_path(dir: &Path) -> PathBuf {
    dir.join("host.json")
}

/// Where a **viewer** host leaves its address — a different file from
/// the session's own host, on purpose. A viewer is a lens onto a
/// process that belongs to somebody else (an agent sitting in the
/// person's tmux), and that session may already have a host of its
/// own: a conversation khor started and the person then resumed in a
/// terminal (`claude -r`) is one row with a ghost *and* a tmux seat.
/// Two hosts writing one `host.json` took turns owning the row's
/// address, and whichever face read it second talked to the wrong one.
/// The terminal reads this file; the conversation reads the other.
pub fn view_file_path(dir: &Path) -> PathBuf {
    dir.join("view.json")
}

/// The viewer's address, when a viewer is up on this session.
pub fn read_view_file(dir: &Path) -> Result<HostFile, String> {
    let text =
        std::fs::read_to_string(view_file_path(dir)).map_err(|_| msg::no_host().to_string())?;
    serde_json::from_str(&text).map_err(msg::host_file_garbled)
}

/// [`connect`], to the viewer's address rather than the host's.
pub fn connect_view(dir: &Path, cols: u16, rows: u16) -> Result<TcpStream, String> {
    connect_view_versioned(dir, cols, rows).map(|a| a.stream)
}

/// [`connect_view`], keeping the agreed version.
pub fn connect_view_versioned(dir: &Path, cols: u16, rows: u16) -> Result<Attached, String> {
    let hf = read_view_file(dir)?;
    connect_versioned(&format!("127.0.0.1:{}", hf.port), &hf.cookie, cols, rows)
}

/// Where a host that died on the way up leaves the reason, for the
/// opener that is watching for [`host_file_path`] to appear.
///
/// # Why a file
///
/// The host is spawned detached, `setsid`, with stdin/stdout/stderr on
/// `/dev/null` — that is what lets the opener's terminal go away
/// without taking the session along, and it is also why nothing the
/// host says can be heard. So the opener had exactly one sentence for
/// every way a host can fail to come up: 宿主没在 5 秒内就位.
///
/// **That sentence is about speed, and most of these failures have no
/// speed.** 2026-08-31: a `khor serve` started by launchd searches
/// `/usr/bin:/bin:/usr/sbin:/sbin`, `tmux` is in `/opt/homebrew/bin`,
/// and so every tui row's terminal died at exec in about a
/// millisecond — while the person watched a five-second wait and was
/// then told the host had been slow. **Two people would draw the wrong
/// conclusion from that and go look at a machine that is not sick.**
///
/// Written by the host, read by the opener, and removed before each
/// spawn so a stale one can never answer for a live host.
pub fn host_error_path(dir: &Path) -> PathBuf {
    dir.join("host-error.txt")
}

/// **The last screen, on disk** (P5-SHELL item 1, the owner 2026-10-12):
/// the ring lives in the host's memory, so a host that died took its
/// session's screen with it and the dead row opened to nothing. The host
/// writes what a reconnecting face would be handed here, beside
/// `commands.json`, every [`RING_SAVE_EVERY`] while something was written
/// and on its way out (its child's end, or SIGTERM). Overwritten whole,
/// through a temporary name and a rename, so a reader never sees half a
/// file. A live session never reads it: its host serves the ring.
pub fn ring_file_path(dir: &Path) -> PathBuf {
    dir.join("ring.bin")
}

/// How often a host with fresh output writes [`ring_file_path`].
pub const RING_SAVE_EVERY: Duration = Duration::from_secs(30);

/// **`ring.bin`**: the first frame a reconnecting face would be handed,
/// and the terminal size it was laid out for. msgpack, positional; fields
/// only ever appended with a serde default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingFile {
    /// What `bytes` is: [`RING_FORM_SNAPSHOT`], the model's snapshot (a
    /// version-3 attacher's first frame: history as styled lines, the
    /// screen, the modes), or [`RING_FORM_REPLAY`], the ring's replay
    /// (the trimmed head's modes, then the bytes, pictures in place) when
    /// the snapshot would pass [`RING`].
    pub form: u32,
    pub cols: u16,
    pub rows: u16,
    /// When it was written, ms since the epoch.
    pub at_ms: u64,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

/// [`RingFile::form`]: the model's snapshot.
pub const RING_FORM_SNAPSHOT: u32 = 3;
/// [`RingFile::form`]: the ring's replay.
pub const RING_FORM_REPLAY: u32 = 1;

/// Writes `ring.bin` whole: a temporary name, then a rename.
pub fn write_ring_file(dir: &Path, ring: &RingFile) -> Result<(), String> {
    let bytes = rmp_serde::to_vec(ring).map_err(|e| e.to_string())?;
    let tmp = dir.join("ring.bin.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, ring_file_path(dir)).map_err(|e| e.to_string())
}

/// A dead session's last screen, when its host left one.
pub fn read_ring_file(dir: &Path) -> Option<RingFile> {
    let bytes = std::fs::read(ring_file_path(dir)).ok()?;
    rmp_serde::from_slice(&bytes).ok()
}

/// The terminal's last non-empty line, as the host keeps it beside its
/// state (`last.txt`) — the row preview's source for hosted sessions.
/// Absent file, absent preview; nothing here invents one.
///
/// A tmux status line is refused here too, not only at the writer
/// (`host_main` skips the preview for a PTY carrying a tmux client):
/// files written by hosts from before that gate are still on disk, and
/// a host from before it may still be running and rewriting one. The
/// preview's contract is 人话或空白 (退场批 sibling ruling), and a
/// refused line is the blank half of it.
pub fn read_last(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("last.txt")).ok()?;
    let line = text.trim();
    // A frame written before the filter existed is still on disk.
    (khor_core::says_something(line) && !looks_like_a_tmux_status_line(line)).then(|| line.to_owned())
}

/// Whether a line is tmux's status bar rather than anything a person
/// said or a program printed: `[<session>` … `:<window>*`, the `*`
/// being the current-window flag every tmux status line carries.
/// Deliberately narrow — a false positive costs one blank preview,
/// a broad pattern would eat real output that happens to use brackets.
fn looks_like_a_tmux_status_line(line: &str) -> bool {
    let Some(open) = line.find('[') else {
        return false;
    };
    let rest = &line[open + 1..];
    let Some(colon) = rest.find(':') else {
        return false;
    };
    let window = &rest[colon + 1..];
    let end = window.find(char::is_whitespace).unwrap_or(window.len());
    window[..end].contains('*')
}

/// Whether this host's PTY carries a tmux client rather than the
/// session's own process. True for both bridge shapes — the plain host
/// a tmux row gets and the viewer an agent-in-tmux row gets — because
/// what they share is the argv, and the argv is what decides what the
/// screen's bottom line is: tmux's status bar, not the session's
/// output.
fn pty_is_a_multiplexer_client(cmd: &[String]) -> bool {
    cmd.first()
        .map(Path::new)
        .and_then(Path::file_name)
        .is_some_and(|n| n == "tmux")
}

/// The last line with visible content out of raw PTY bytes, ANSI
/// stripped. An approximation by design: without a screen model a
/// cursor-addressed repaint cannot be replayed, so this reads the byte
/// tail the way `tail -1` would — good enough for a preview line, and
/// the TUI path (which has a screen) does not use it.
fn last_plain_line(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.peek() {
                // CSI: parameters then one final byte in @..~.
                Some('[') => {
                    chars.next();
                    for f in chars.by_ref() {
                        if ('@'..='~').contains(&f) {
                            break;
                        }
                    }
                }
                // OSC: terminated by BEL or ESC \ (the ESC re-enters here).
                Some(']') => {
                    chars.next();
                    for f in chars.by_ref() {
                        if f == '\u{7}' || f == '\u{1b}' {
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            },
            '\n' => out.push('\n'),
            '\r' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.lines()
        .rev()
        .find(|l| khor_core::says_something(l))
        .map(khor_core::preview)
}

pub fn read_host_file(dir: &Path) -> Result<HostFile, String> {
    let text =
        std::fs::read_to_string(host_file_path(dir)).map_err(|_| msg::no_host().to_string())?;
    serde_json::from_str(&text).map_err(msg::host_file_garbled)
}

pub fn write_frame<T: Serialize>(s: &mut (impl Write + ?Sized), t: &T) -> Result<(), String> {
    let bytes = proto::encode(t)?;
    // One write, length and body together: two small writes are the
    // pattern Nagle's algorithm holds the second of for an ack.
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    frame.extend_from_slice(&bytes);
    s.write_all(&frame).map_err(msg::handoff_failed)
}

/// A client's frame, or `None` for one over `MAX_OP` that was read past
/// and dropped: **an oversize frame costs that frame, not the client**
/// (T-16, the terminal audit: a 1.2 MB paste errored `read_frame`, the
/// per-client input loop exited, and every key after it was lost until a
/// reload, while output kept flowing). The length prefix says exactly how
/// much to skip, so the stream stays in step.
fn read_client_frame<T: for<'a> Deserialize<'a>>(s: &mut (impl Read + ?Sized)) -> Result<Option<T>, String> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len).map_err(msg::no_frame)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_OP {
        eprintln!("{}", msg::frame_too_big(len));
        let skipped = std::io::copy(&mut s.take(len as u64), &mut std::io::sink()).map_err(msg::half_a_frame)?;
        if skipped as usize != len {
            return Err(msg::half_a_frame(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
        }
        return Ok(None);
    }
    let mut bytes = vec![0u8; len];
    s.read_exact(&mut bytes).map_err(msg::half_a_frame)?;
    proto::decode(&bytes).map(Some)
}

pub fn read_frame<T: for<'a> Deserialize<'a>>(s: &mut (impl Read + ?Sized)) -> Result<T, String> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len).map_err(msg::no_frame)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_OP {
        return Err(msg::frame_too_big(len));
    }
    let mut bytes = vec![0u8; len];
    s.read_exact(&mut bytes).map_err(msg::half_a_frame)?;
    proto::decode(&bytes)
}

/// Answers the `_host`/`_ghost` calling convention **if this process was
/// started as one**, then never returns; a process started any other way
/// falls straight through. Every binary that can spawn hosts calls this
/// first thing in its `main` — [`spawn_host`] re-execs `current_exe()`,
/// so whichever consumer stood the host up (the CLI, the dev bridge, the
/// app) must itself answer the convention. The bridge not answering was
/// found live: a host spawned from it was another bridge, and the
/// terminal waited on a host.json nobody would ever write.
pub fn main_if_host(root: std::path::PathBuf) {
    crate::legacy_codex::main_if_shim();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match args.first().map(String::as_str) {
        Some("_host") => host_args(&root, &args[1..]),
        Some("_ghost") => ghost_args(&root, &args[1..]),
        // The claude shim (`cagent`): every binary that can spawn a GUI
        // host must also be able to *be* the agent it spawns, for the
        // same current_exe reason the two arms above exist.
        Some("_cagent") => crate::cagent::cagent_main(root.clone()),
        Some("_codexagent") => crate::codexagent::codexagent_main(root.clone()),
        // khor's verbs as an agent's tools (docs/AGENT.md). **Not an
        // internal `_word`, and here anyway**: the scheduler's MCP
        // config names `self_exe()`, which in a dev bridge is `bridge`
        // and in the app is `khor-gui` — so the binary that spawned the
        // agent has to be able to answer it. Measured the other way
        // round: the config pointed at `bridge mcp`, `bridge` did not
        // know the word, and the agent reported — correctly — that
        // khor's tools did not exist in its session.
        Some("mcp") => crate::mcp::serve_stdio(root.clone()),
        _ => return,
    };
    if let Err(e) = outcome {
        eprintln!("{e}");
        std::process::exit(1);
    }
    std::process::exit(0);
}

fn host_args(root: &Path, rest: &[String]) -> Result<(), String> {
    let (head, cmd) = match rest.iter().position(|a| a == "--") {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => return Err(msg::host_bad_convention().into()),
    };
    let [sid, cols, rows] = head else {
        return Err(msg::host_bad_convention().into());
    };
    if cmd.is_empty() {
        return Err(msg::host_bad_convention().into());
    }
    let size = (
        cols.parse::<u16>()
            .map_err(|_| msg::host_bad_convention().to_string())?,
        rows.parse::<u16>()
            .map_err(|_| msg::host_bad_convention().to_string())?,
    );
    host_main_with_startup(
        root.to_path_buf(),
        SessionId(sid.clone()),
        size,
        cmd.to_vec(),
        std::env::var(STARTUP_ENV).ok(),
    )
}

fn ghost_args(root: &Path, rest: &[String]) -> Result<(), String> {
    let (head, cmd) = match rest.iter().position(|a| a == "--") {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => return Err(msg::host_bad_convention().into()),
    };
    let [ready, title] = head else {
        return Err(msg::host_bad_convention().into());
    };
    crate::gui_host::gui_host_main(
        root.to_path_buf(),
        std::path::PathBuf::from(ready),
        title.clone(),
        cmd.to_vec(),
    )
}

// ── the opener's side ───────────────────────────────────────

/// Spawns the detached host for an already-registered session and waits
/// for it to be reachable. The host is its own session (setsid): the
/// terminal that runs `open` can die without taking it along.
pub fn spawn_host(
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    size: (u16, u16),
) -> Result<(), String> {
    spawn_host_with(dir, id, cmd, size, None, None, None)
}

/// `spawn_host`, born in a chosen working directory — the wizard names
/// where a session lives; every other opener is already standing there.
pub fn spawn_host_at(
    cwd: &Path,
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    size: (u16, u16),
) -> Result<(), String> {
    spawn_host_with(dir, id, cmd, size, None, Some(cwd), None)
}

/// A normal shell host with a single initial input. Attach never calls this.
pub fn spawn_shell_at(
    cwd: &Path,
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    size: (u16, u16),
    startup: &str,
) -> Result<(), String> {
    spawn_host_with(dir, id, cmd, size, None, Some(cwd), Some(startup))
}

/// A host that is a **lens, not an owner**: it bridges a terminal onto a
/// process that belongs to somebody else (an agent sitting in the user's
/// tmux), so it must not claim the row's pid, report words, or record an
/// ending — the vendor's own files keep answering all three. `made` says
/// whether the attach minted the registry dir purely to hang this host
/// on: then the host takes the dir with it when the bridge ends, and the
/// row goes back to being a discovered one.
pub fn spawn_viewer_host(
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    size: (u16, u16),
    made: bool,
) -> Result<(), String> {
    spawn_host_with(
        dir,
        id,
        cmd,
        size,
        Some(if made { "made" } else { "keep" }),
        None,
        None,
    )
}

/// How `_host` learns it is a viewer: env rather than argv, so the argv
/// contract (`host_args`) stays what every existing binary parses.
const VIEWER_ENV: &str = "KHOR_HOST_VIEWER";
const STARTUP_ENV: &str = "KHOR_HOST_STARTUP";

fn host_store_root(dir: &Path) -> Result<PathBuf, String> {
    let root = dir.ancestors().nth(3).ok_or_else(|| msg::no_such_dir(dir.display()))?;
    std::fs::canonicalize(root).map_err(|e| msg::cant_read(root.display(), e))
}

fn spawn_host_with(
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    (cols, rows): (u16, u16),
    viewer: Option<&str>,
    cwd: Option<&Path>,
    startup: Option<&str>,
) -> Result<(), String> {
    let exe = crate::self_exe()?;
    let mut c = std::process::Command::new(exe);
    // A host belongs to the store containing this session. A GUI request may
    // name a different root from its process environment; only the child gets
    // that root, before it changes directory to the session's working path.
    let root = host_store_root(dir)?;
    c.env("KHOR_HOME", &root);
    // Native TUI resume must read the very store the GUI shim/discovery used.
    // Set these on the child only; a private node cannot inherit the user's
    // unrelated native store through its parent process environment.
    c.env("CLAUDE_CONFIG_DIR", crate::adaptor::claude_home(&root));
    c.env("CODEX_HOME", crate::adaptor::codex_home(&root));
    c.env_remove(STARTUP_ENV);
    if let Some(text) = startup { c.env(STARTUP_ENV,text); }
    if let Some(cwd) = cwd {
        c.current_dir(cwd);
    }
    c.arg("_host")
        .arg(&id.0)
        .arg(cols.to_string())
        .arg(rows.to_string())
        .arg("--")
        .args(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    match viewer {
        Some(v) => {
            c.env(VIEWER_ENV, v);
        }
        None => {
            // A host spawned by a viewer host's own re-exec must not
            // inherit viewerhood.
            c.env_remove(VIEWER_ENV);
        }
    }
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    crate::mcp_worker::prepare_durable_child(&mut c);
    // The `PATH` this process worked out travels down rather than
    // being worked out again: `_host` is a fresh process and would
    // otherwise re-run the user's login shell on every single spawn
    // (measured 0.78s cold, and it runs their rc files). `KHOR_PATH` is
    // the override that already exists for exactly "khor, here is where
    // things are" — no second mechanism.
    c.env(crate::PATH_ENV, crate::Node::user_path());
    // The locale travels the same way and for the same reason: `_host`
    // is a fresh process, and a repaired locale set here is one it does
    // not have to go back to the login shell for.
    for (name, value) in crate::Node::user_locale() {
        c.env(name, value);
    }
    // **Wait for the file this host will actually write.** A viewer
    // leaves its address in `view.json` (`view_file_path` has why), and
    // an opener watching `host.json` for it waited the full five
    // seconds beside a viewer that had been up for four of them, then
    // told the person the host never came — measured live 2026-09-01
    // on a conversation the person had resumed in tmux: `view.json`
    // written, viewer alive, screen saying 宿主没在 5 秒内就位.
    let marker = if viewer.is_some() {
        view_file_path(dir)
    } else {
        host_file_path(dir)
    };
    let complaint = host_error_path(dir);
    // A leftover from a previous failed spawn must never answer for
    // this one; nothing else writes this file.
    let _ = std::fs::remove_file(&complaint);
    let mut child = c.spawn().map_err(msg::host_wont_start)?;
    for _ in 0..100 {
        if marker.exists() {
            return Ok(());
        }
        // The host's own words beat the timeout whenever it managed to
        // leave any (`host_error_path`).
        if let Ok(why) = std::fs::read_to_string(&complaint) {
            let why = why.trim();
            if !why.is_empty() {
                return Err(why.to_owned());
            }
        }
        // **A host that is already gone is not going to be ready.**
        // Without this the person waits the whole five seconds for a
        // process that died in a millisecond, and the wait itself is
        // the misleading part.
        if matches!(child.try_wait(), Ok(Some(_))) && !marker.exists() {
            return Err(match std::fs::read_to_string(&complaint) {
                Ok(why) if !why.trim().is_empty() => why.trim().to_owned(),
                _ => msg::host_died_mute().to_string(),
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(msg::host_never_ready().into())
}

/// Connects and completes the handshake; on return the stream carries
/// the replay and then live PTY bytes. Client ops go back framed.
pub fn connect(dir: &Path, cols: u16, rows: u16) -> Result<TcpStream, String> {
    connect_host_versioned(dir, cols, rows).map(|a| a.stream)
}

/// [`connect`], keeping the agreed version.
pub fn connect_host_versioned(dir: &Path, cols: u16, rows: u16) -> Result<Attached, String> {
    let hf = read_host_file(dir)?;
    connect_versioned(&format!("127.0.0.1:{}", hf.port), &hf.cookie, cols, rows)
}

/// The same handshake against an address the caller already has.
///
/// **This is the whole of what remote attach adds to this file.** A
/// session on another machine is reached through a pipe the resident
/// serve binds locally (`Node::reach`), so what arrives here is still a
/// TCP address and still the host protocol — the terminal never learns
/// there is a network under it, which is why nothing else in this file
/// changes.
///
/// **The handshake has a deadline** ([`HANDSHAKE`]). For a far session
/// the address is the resident's pipe, which accepts at once and only
/// then dials the far machine — so a machine that has dropped off left
/// this waiting on its `Welcome` for as long as the pipe took to give
/// up, with nothing bounding it (2026-10-02, the phone that froze). The
/// stream goes back to blocking without a timeout once welcomed: from
/// then on a quiet terminal is a terminal nobody is typing in.
pub fn connect_at(addr: &str, cookie: &str, cols: u16, rows: u16) -> Result<TcpStream, String> {
    connect_versioned(addr, cookie, cols, rows).map(|a| a.stream)
}

/// [`connect_at`], agreeing a version ([`HOST_VERSION`]): with a host
/// that advertises one, the client's own is sent before anything else.
pub fn connect_versioned(
    addr: &str,
    cookie: &str,
    cols: u16,
    rows: u16,
) -> Result<Attached, String> {
    let (mut s, w) = connect_welcomed(addr, cookie, cols, rows)?;
    let version = advertised(&w).min(HOST_VERSION);
    if version >= 2 {
        write_frame(&mut s, &ClientOp::Version(HOST_VERSION))?;
    }
    let _ = s.set_read_timeout(None);
    let _ = s.set_write_timeout(None);
    Ok(Attached { stream: s, version })
}

/// The common cookie handshake, before a terminal or GUI selects its protocol.
/// Successful `why` remains available for capability negotiation without
/// changing the positional Hello/Welcome frames.
pub fn connect_welcomed(
    addr: &str,
    cookie: &str,
    cols: u16,
    rows: u16,
) -> Result<(TcpStream, Welcome), String> {
    let mut s = TcpStream::connect(addr).map_err(|_| msg::host_gone().to_string())?;
    // A keystroke is one small frame: never held back for an ack.
    let _ = s.set_nodelay(true);
    let _ = s.set_read_timeout(Some(HANDSHAKE));
    let _ = s.set_write_timeout(Some(HANDSHAKE));
    write_frame(
        &mut s,
        &Hello {
            cookie: cookie.to_owned(),
            cols,
            rows,
        },
    )?;
    let w: Welcome = read_welcome(&mut s)?;
    if !w.ok {
        return Err(w.why);
    }
    Ok((s, w))
}

/// The host's `Welcome`, or **why it did not come** in words a person
/// reads (item 36): a host that took the connection and stayed silent
/// past [`HANDSHAKE`] ran the read into its deadline, which the socket
/// reports as `WouldBlock` — on macOS and iOS 「Resource temporarily
/// unavailable (os error 35)」, what the owner was shown.
fn read_welcome(s: &mut TcpStream) -> Result<Welcome, String> {
    let mut len = [0u8; 4];
    if let Err(e) = s.peek(&mut len) {
        if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) {
            return Err(msg::host_silent(HANDSHAKE.as_secs()));
        }
    }
    read_frame(s)
}

/// How long [`connect_at`] waits for a host to welcome it: a far pipe's
/// dial (≤ 10 s, `link::DIAL_TIMEOUT`, relay included) and the host's
/// answer, which is immediate. A host on this machine answers in
/// microseconds, so the deadline only ever bites a far one.
pub const HANDSHAKE: Duration = Duration::from_secs(15);

// ── the host process ────────────────────────────────────────

/// The replay: the last [`RING`] bytes of text, the modes the bytes
/// before them had set (`crate::modes` — a tmux client's `ESC [ ? 1049 h`
/// is its first line and never repeated, 2026-10-01), and the pictures
/// drawn among them, kept apart on their own budget (`crate::pictures`).
#[derive(Default)]
struct Ring {
    bytes: VecDeque<u8>,
    head: Modes,
    /// How many text bytes were ever trimmed: the stream position of
    /// `bytes[0]`.
    gone: u64,
    pictures: Pictures,
}

impl Ring {
    fn push(&mut self, bytes: &[u8]) {
        let mut text = Vec::with_capacity(bytes.len());
        self.pictures.sort(bytes, self.gone + self.bytes.len() as u64, &mut text);
        self.bytes.extend(text);
        let over = self.bytes.len().saturating_sub(RING);
        if over > 0 {
            let gone: Vec<u8> = self.bytes.drain(..over).collect();
            self.head.feed(&gone);
            self.gone += over as u64;
        }
        // Other long strings (a clipboard copy, a protocol the face does
        // not draw) stay in the text: a trim inside one runs on to its end.
        while self.head.mid_sequence() {
            let Some(b) = self.bytes.pop_front() else { break };
            self.head.feed(&[b]);
            self.gone += 1;
        }
        self.pictures.trim(self.gone);
    }

    /// **⌘K** ([`ClientOp::Forget`]): everything before the line the
    /// cursor is on goes, through the head's modes as a trim does, so the
    /// next attacher is handed that line and what follows it — at a
    /// prompt, the prompt, and the shell's own ⌃L redraw right after.
    fn forget(&mut self) {
        let keep = self.bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        let gone: Vec<u8> = self.bytes.drain(..keep).collect();
        self.head.feed(&gone);
        self.gone += keep as u64;
        while self.head.mid_sequence() {
            let Some(b) = self.bytes.pop_front() else { break };
            self.head.feed(&[b]);
            self.gone += 1;
        }
        self.pictures.trim(self.gone);
    }

    /// What an attacher is handed first: the trimmed head's modes, then
    /// the bytes still held, pictures in place.
    fn replay(&self) -> Vec<u8> {
        let mut out = self.head.preamble();
        self.pictures.splice(&self.bytes, self.gone, &mut out);
        out
    }

    /// **The modes in force now, and nothing else** (item 41): what a
    /// multiplexer client's attacher is handed instead of the ring. The
    /// head's modes, then every byte still held fed through them — a
    /// tmux client's `ESC [ ? 1049 h` may be in the ring or trimmed past
    /// it, and the attacher must start in it either way.
    fn modes_now(&self) -> Vec<u8> {
        let mut modes = self.head.clone();
        let (a, b) = self.bytes.as_slices();
        modes.feed(a);
        modes.feed(b);
        modes.preamble()
    }
}

/// The socket a tmux client's argv names (`-L name` or `-S path`, before
/// the command), to speak to the same server.
pub(crate) fn tmux_socket_args(cmd: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 1;
    while i + 1 < cmd.len() {
        match cmd[i].as_str() {
            "-L" | "-S" => {
                out.push(cmd[i].clone());
                out.push(cmd[i + 1].clone());
                i += 2;
            }
            // A global flag with no value, or the command: stop at the
            // first word that is neither.
            s if s.starts_with('-') && s.len() == 2 && !["-f", "-T"].contains(&s) => i += 1,
            "-f" | "-T" => i += 2,
            _ => break,
        }
    }
    out
}

/// **tmux draws the screen for a new attacher** (item 41): the client is
/// khor's own, so the attacher got the modes and no bytes; `refresh-client`
/// on this pty's tty makes tmux redraw it whole, through the reader and
/// so to every face, the new one included. A face opening at the same
/// size sends no SIGWINCH, which is why the ask is explicit.
fn redraw_tmux_client(cmd: &[String], tty: Option<std::path::PathBuf>) {
    let Some(tty) = tty else { return };
    let path = crate::spawn_path(&cmd[0]);
    let Some(tmux) = crate::find_program(&cmd[0], &path) else { return };
    let mut c = std::process::Command::new(tmux);
    c.args(tmux_socket_args(cmd)).arg("refresh-client").arg("-t").arg(tty).env("PATH", &path);
    for k in ["TMUX", "TMUX_PANE"] {
        c.env_remove(k);
    }
    c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    std::thread::spawn(move || {
        let _ = c.status();
    });
}

/// **A narrowing never splits a wide character** (item 57, measured):
/// vt100's resize cuts each row without clearing a wide character that
/// straddles the new last column, and the next erase or write on that
/// cell indexes past the row (`row.rs` `clear_wide`). A tmux client
/// redraws every row after a resize, so a CJK pane and a face one column
/// narrower panicked every vt100 that followed the pty: the host's model
/// and gui-core's mode oracle alike. The straddling characters of the
/// screen in view are erased first — they would be cut anyway — and the
/// cursor and attributes put back. The screen out of view (the shell's
/// under a full-screen program) cannot be reached; the callers' panic
/// heals ([`parser_like`]) catch what that leaves.
pub fn resize_screen(parser: &mut vt100::Parser, rows: u16, cols: u16) {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let screen = parser.screen();
    let (r, c) = screen.size();
    if (r, c) == (rows, cols) {
        return;
    }
    if cols < c && cols > 0 {
        let mut scrub = Vec::new();
        for row in 0..r {
            if screen.cell(row, cols - 1).is_some_and(vt100::Cell::is_wide) {
                scrub.extend_from_slice(format!("\x1b[{};{}H\x1b[X", row + 1, cols).as_bytes());
            }
        }
        if !scrub.is_empty() {
            let (cy, cx) = screen.cursor_position();
            let mut bytes = b"\x1b[0m".to_vec();
            bytes.extend_from_slice(&scrub);
            bytes.extend_from_slice(format!("\x1b[{};{}H", cy + 1, cx + 1).as_bytes());
            bytes.extend_from_slice(&screen.attributes_formatted());
            let _ = catch_unwind(AssertUnwindSafe(|| parser.process(&bytes)));
        }
    }
    parser.screen_mut().set_size(rows, cols);
}

/// **A fresh vt100 in place of one that panicked, on the same screen**
/// (item 57): at `old`'s size, on the alternate screen if it was, with
/// its input modes and its cursor's visibility. A program's repaint does
/// not switch screens again, so a parser left on the normal screen kept
/// the repaint where nobody looks and answered "no bracketed paste" to a
/// program that had asked for it.
pub fn parser_like(old: &vt100::Screen, scrollback: usize) -> vt100::Parser {
    let (rows, cols) = old.size();
    let mut kept = Vec::new();
    if old.alternate_screen() {
        kept.extend_from_slice(b"\x1b[?1049h");
    }
    kept.extend_from_slice(&old.input_mode_formatted());
    if old.hide_cursor() {
        kept.extend_from_slice(b"\x1b[?25l");
    }
    let mut fresh = vt100::Parser::new(rows, cols, scrollback);
    fresh.process(&kept);
    fresh
}

/// **The program on the terminal draws its screen again** (item 57):
/// SIGWINCH to the pty's foreground process group, after the host's
/// model lost its picture to a vt100 panic ([`Model::feed`]). Every
/// full-screen program repaints on it — a tmux client has its server
/// redraw it whole even at an unchanged size; vim, htop and claude
/// redraw; a shell its prompt. A same-size `TIOCSWINSZ` signals nothing
/// (the kernel compares the sizes first), so the signal is sent itself.
#[cfg(unix)]
fn repaint_foreground(leader: Option<i32>) {
    if let Some(group) = leader.filter(|&p| p > 0) {
        // SAFETY: a signal to a process group; no memory is touched.
        unsafe {
            libc::kill(-group, libc::SIGWINCH);
        }
    }
}

#[cfg(not(unix))]
fn repaint_foreground(_leader: Option<i32>) {}

/// The least time between two repaints asked for after a panic: a
/// program whose every repaint panics vt100 is not kept redrawing.
const REPAINT_EVERY: Duration = Duration::from_secs(1);

struct Shared {
    /// Lock order everywhere: ring, then clients — an attacher must
    /// snapshot and join atomically or it double-receives the bytes the
    /// reader is broadcasting.
    ring: Mutex<Ring>,
    clients: Mutex<Vec<Client>>,
    /// The screen the host keeps (item 46), fed under the ring's lock;
    /// `None` for a tmux client, whose screen is tmux's (item 41).
    model: Mutex<Option<Model>>,
    /// Counts every change to the ring (output, ⌘K), so the saver writes
    /// `ring.bin` only when something was written since its last write.
    changes: AtomicU64,
}

impl Shared {
    /// **What a reconnecting face would be handed now**, for `ring.bin`
    /// (P5-SHELL item 1): the model's snapshot under the ring's modes —
    /// the same first frame a version-3 attacher gets, at the size the
    /// model stands at — or the ring's replay when the snapshot would
    /// pass [`RING`]. `None` with no model (a tmux client: tmux holds
    /// its screen). Locks ring, then model, the attachers' order.
    fn ring_now(&self) -> Option<RingFile> {
        let ring = self.ring.lock().unwrap();
        let mut model = self.model.lock().unwrap();
        let m = model.as_mut()?;
        let (rows, cols) = m.parser.screen().size();
        let snapshot = m.snapshot(&ring.modes_now());
        let (form, bytes) = if snapshot.len() <= RING {
            (RING_FORM_SNAPSHOT, snapshot)
        } else {
            (RING_FORM_REPLAY, ring.replay())
        };
        Some(RingFile {
            form,
            cols,
            rows,
            at_ms: crate::live::now_ms().max(0) as u64,
            bytes,
        })
    }

    /// Writes `ring.bin` when the ring changed since `*saved` (the change
    /// count at the last write), and moves `*saved` on.
    fn save_ring(&self, dir: &Path, saved: &mut u64) {
        let now = self.changes.load(Ordering::SeqCst);
        if now == *saved {
            return;
        }
        if let Some(file) = self.ring_now()
            && write_ring_file(dir, &file).is_ok()
        {
            *saved = now;
        }
    }
}

/// Set by the host's SIGTERM handler: the saver writes `ring.bin`, then
/// the host dies of the signal as it always did.
static TERMED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn on_term(_: libc::c_int) {
    TERMED.store(true, Ordering::SeqCst);
}

/// Rows of history the model keeps above the screen until a face says
/// its own setting: about what the ring held of an 80-column shell, kept
/// as cells now (item 46). A shell opened from a face is told at once.
const MODEL_SCROLLBACK: usize = 2000;

/// **The most a face can ask the model to keep** (item 56): the face's
/// 历史行数 (item 17) goes up to 20 000, the host keeps 10 000 at most. A
/// vt100 row is 32 bytes a column whatever it holds — measured in
/// `model_memory`: 27 MB for 10 000 rows at 80 columns, 40 MB at 120.
pub const MODEL_SCROLLBACK_MOST: usize = 10_000;

/// **The host keeps a screen, not only bytes** (item 46; the owner chose
/// this over a private tmux). A vt100 terminal (MIT, the crate
/// `khor_detect` already reads agent screens with) over every shell khor
/// hosts, with scrollback; an attacher of version 3 is handed what it
/// shows — never the ring's bytes parsed visibly on the phone. The ring
/// stays for older clients and for ⌘K's `Forget`. What vt100 does not
/// keep — the OSC 8 links on its cells, the pictures among its rows — is
/// kept beside it (item 66, `crate::beside`) and goes into the snapshot.
struct Model {
    parser: vt100::Parser,
    beside: Beside,
    /// The rows of history `parser` keeps.
    keeps: usize,
    /// The most any face has asked for ([`ClientOp::Scrollback`]): taken
    /// on the normal screen, since the rebuild that grows the history
    /// cannot carry the normal screen under an alternate one.
    asked: usize,
}

impl Model {
    fn new(rows: u16, cols: u16) -> Self {
        Model {
            parser: vt100::Parser::new(rows, cols, MODEL_SCROLLBACK),
            beside: Beside::default(),
            keeps: MODEL_SCROLLBACK,
            asked: MODEL_SCROLLBACK,
        }
    }

    /// **The history follows the faces' setting** (item 56): a face says
    /// its 历史行数 on attach and the model keeps the most any face has
    /// asked for, up to [`MODEL_SCROLLBACK_MOST`] — never fewer, so the
    /// phone's 5000 does not cut the Mac's 10 000 under it. vt100 fixes
    /// the length at birth, so the model is rebuilt from its own
    /// snapshot at the new length (what an attacher is handed, proven
    /// cell for cell by `a_snapshot_rebuilds_the_screen_and_its_history`).
    fn keep(&mut self, lines: usize) {
        self.asked = self.asked.max(lines.min(MODEL_SCROLLBACK_MOST));
        self.settle();
    }

    fn settle(&mut self) {
        if self.asked <= self.keeps || self.parser.screen().alternate_screen() {
            return;
        }
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let (rows, cols) = self.parser.screen().size();
        let bytes = self.snapshot(&[]);
        let mut fresh = vt100::Parser::new(rows, cols, self.asked);
        if catch_unwind(AssertUnwindSafe(|| fresh.process(&bytes))).is_ok() {
            self.parser = fresh;
        }
        self.keeps = self.asked;
    }

    /// vt100 has corner-case panics on real streams. One bad frame must
    /// not kill the host: the parser is replaced at its size, and the
    /// history is lost. Answers whether it was, so the caller asks the
    /// program to draw again ([`repaint_foreground`]).
    ///
    /// The fresh parser stays on the screen the old one was on, with its
    /// input modes (item 57): a program's repaint does not switch screens
    /// again, and a model left on the normal screen was handed out under
    /// the modes' `ESC [ ? 1049 h` as a blank alternate screen.
    fn feed(&mut self, bytes: &[u8]) -> bool {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        let (parser, beside, keeps) = (&mut self.parser, &mut self.beside, self.keeps);
        let replaced = catch_unwind(AssertUnwindSafe(|| beside.feed(parser, bytes, keeps))).is_err();
        if replaced {
            self.parser = parser_like(self.parser.screen(), self.keeps);
            self.beside = Beside::default();
        }
        if self.asked > self.keeps {
            self.settle();
        }
        replaced
    }

    /// Follows the pty's size without splitting a wide character
    /// ([`resize_screen`], item 57).
    fn resize(&mut self, rows: u16, cols: u16) {
        resize_screen(&mut self.parser, rows, cols);
        self.beside.resize(rows);
    }

    /// **⌘K** ([`ClientOp::Forget`]), kept by the model as the ring keeps
    /// it: the history and the rows above the cursor's go, the cursor's
    /// row and what follows it stand at the top of a fresh screen, in the
    /// attributes and input modes in force. Normal screen only, as the
    /// face's ⌘K is.
    fn forget(&mut self) {
        let screen = self.parser.screen_mut();
        if screen.alternate_screen() {
            return;
        }
        let (rows, cols) = screen.size();
        screen.set_scrollback(0);
        let (cy, cx) = screen.cursor_position();
        self.beside.forget(self.beside.scrolled() + cy as u64);
        let mut bytes = Vec::new();
        let mut pen = Pen::default();
        for r in cy..rows {
            write_row(&mut bytes, screen, r, cols, &mut pen, &RowMarks::default());
        }
        if bytes.ends_with(b"\r\n") {
            bytes.truncate(bytes.len() - 2);
        }
        bytes.extend_from_slice(format!("\x1b[0m\x1b[1;{}H", cx + 1).as_bytes());
        if screen.hide_cursor() {
            bytes.extend_from_slice(b"\x1b[?25l");
        }
        bytes.extend_from_slice(&screen.attributes_formatted());
        bytes.extend_from_slice(&screen.input_mode_formatted());
        let mut fresh = vt100::Parser::new(rows, cols, self.keeps);
        fresh.process(&bytes);
        self.parser = fresh;
    }

    /// **The snapshot**: on the normal screen, the scrollback as styled
    /// lines (oldest first, wrapped rows joined so the face wraps them
    /// at its own width), then the screen with its attributes and the
    /// cursor where it is, then `modes` (the ring's, for what vt100 does
    /// not model: the kitty flags, modifyOtherKeys) and vt100's own input
    /// modes. On the alternate screen the modes switch the face to it
    /// and the program's screen follows; the normal screen underneath
    /// is not sent (vt100 shows one grid at a time).
    ///
    /// **The links and the pictures go where they were** (item 66): a
    /// link around its cells, in the history's lines and written again
    /// over the screen's; a picture at its place among the lines. A
    /// picture still on the screen was drawn among the screen's rows and
    /// pushed the rows after it down, so then the screen is written as
    /// lines too, the picture between them, and the cursor is moved back
    /// up to its row.
    fn snapshot(&mut self, modes: &[u8]) -> Vec<u8> {
        let beside = &self.beside;
        let screen = self.parser.screen_mut();
        let (rows, cols) = screen.size();
        let mut out = Vec::new();
        out.extend_from_slice(b"\x1b[0m");
        if !screen.alternate_screen() {
            let top = beside.scrolled();
            screen.set_scrollback(usize::MAX);
            let total = screen.scrollback();
            let mut left = total;
            let mut pen = Pen::default();
            while left > 0 {
                screen.set_scrollback(left);
                let take = left.min(rows as usize);
                for r in 0..take as u16 {
                    let at = (top + r as u64).saturating_sub(left as u64);
                    let marks = beside.marks(false, at, screen, r, cols);
                    write_row(&mut out, screen, r, cols, &mut pen, &marks);
                }
                left -= take;
            }
            screen.set_scrollback(0);
            pen.unlink(&mut out);
            out.extend_from_slice(b"\x1b[0m");
            if beside.pictures_from(false, top) {
                let mut pen = Pen::default();
                for r in 0..rows {
                    let marks = beside.marks(false, top + r as u64, screen, r, cols);
                    write_row(&mut out, screen, r, cols, &mut pen, &marks);
                }
                pen.unlink(&mut out);
                if out.ends_with(b"\r\n") {
                    out.truncate(out.len() - 2);
                }
                cursor_from_the_bottom(&mut out, screen);
            } else {
                // The last rows written are still on the face's screen:
                // the rest of a screen of line ends moves them into its
                // history, then the screen is drawn over the blank.
                if total > 0 {
                    out.extend_from_slice(&b"\r\n".repeat(rows.saturating_sub(1) as usize));
                }
                out.extend_from_slice(&screen.contents_formatted());
                overlay(&mut out, beside, false, top, screen);
            }
            out.extend_from_slice(modes);
        } else {
            out.extend_from_slice(modes);
            out.extend_from_slice(&screen.contents_formatted());
            overlay(&mut out, beside, true, 0, screen);
        }
        out.extend_from_slice(&screen.input_mode_formatted());
        out
    }
}

/// The links and pictures of the screen in view, drawn over what
/// `contents_formatted` drew (item 66): each linked run written again in
/// its looks inside its OSC 8, each picture at its place, then the
/// cursor and the attributes put back as they were.
fn overlay(out: &mut Vec<u8>, beside: &Beside, alt: bool, top: u64, screen: &vt100::Screen) {
    let (rows, cols) = screen.size();
    let mut over = Vec::new();
    for r in 0..rows {
        let marks = beside.marks(alt, top + r as u64, screen, r, cols);
        let mut c = 0;
        while (c as usize) < marks.links.len() {
            let Some(link) = &marks.links[c as usize] else {
                c += 1;
                continue;
            };
            over.extend_from_slice(format!("\x1b[{};{}H", r + 1, c + 1).as_bytes());
            let mut pen = Pen::default();
            while marks.links.get(c as usize).is_some_and(|l| l.as_deref() == Some(&**link)) {
                if let Some(cell) = screen.cell(r, c) {
                    pen.cell(&mut over, cell, Some(link));
                }
                c += 1;
            }
            pen.unlink(&mut over);
        }
        for (col, bytes) in &marks.pictures {
            over.extend_from_slice(format!("\x1b[{};{}H", r + 1, col + 1).as_bytes());
            over.extend_from_slice(bytes);
        }
    }
    if over.is_empty() {
        return;
    }
    out.extend_from_slice(&over);
    out.extend_from_slice(&screen.cursor_state_formatted());
    out.extend_from_slice(b"\x1b[0m");
    out.extend_from_slice(&screen.attributes_formatted());
}

/// After the screen was written as lines from the face's bottom row:
/// the cursor goes back up to its own row and column, a cursor waiting
/// past the last column waits there again (its cell written once more),
/// and the attributes and its visibility are put back.
fn cursor_from_the_bottom(out: &mut Vec<u8>, screen: &vt100::Screen) {
    let (rows, cols) = screen.size();
    let (cy, cx) = screen.cursor_position();
    out.extend_from_slice(b"\x1b[0m\r");
    let up = rows.saturating_sub(1).saturating_sub(cy);
    if up > 0 {
        out.extend_from_slice(format!("\x1b[{up}A").as_bytes());
    }
    let last = screen.cell(cy, cols.saturating_sub(1)).filter(|c| c.has_contents() && !c.is_wide_continuation());
    match last {
        Some(cell) if cx >= cols => {
            out.extend_from_slice(format!("\x1b[{cols}G").as_bytes());
            Pen::default().cell(out, cell, None);
        }
        _ => out.extend_from_slice(format!("\x1b[{}G", cx.min(cols.saturating_sub(1)) + 1).as_bytes()),
    }
    out.extend_from_slice(b"\x1b[0m");
    out.extend_from_slice(&screen.attributes_formatted());
    if screen.hide_cursor() {
        out.extend_from_slice(b"\x1b[?25l");
    }
}

/// What the cells written so far left in force: their looks and the
/// link they are inside.
#[derive(Default)]
struct Pen {
    look: CellLook,
    link: Option<Arc<str>>,
}

impl Pen {
    /// Inside `link` (an OSC 8 payload: parameters, `;`, target) or
    /// outside any.
    fn link_to(&mut self, out: &mut Vec<u8>, link: Option<&Arc<str>>) {
        if self.link.as_deref() == link.map(|l| &**l) {
            return;
        }
        match link {
            Some(l) => {
                out.extend_from_slice(b"\x1b]8;");
                out.extend_from_slice(l.as_bytes());
                out.extend_from_slice(b"\x1b\\");
            }
            None => out.extend_from_slice(b"\x1b]8;;\x1b\\"),
        }
        self.link = link.cloned();
    }

    fn unlink(&mut self, out: &mut Vec<u8>) {
        self.link_to(out, None);
    }

    /// One cell in its looks, inside `link`.
    fn cell(&mut self, out: &mut Vec<u8>, c: &vt100::Cell, link: Option<&Arc<str>>) {
        if c.is_wide_continuation() {
            return;
        }
        self.link_to(out, link);
        let want = CellLook::of(c);
        if want != self.look {
            out.extend_from_slice(&want.sgr());
            self.look = want;
        }
        if c.has_contents() {
            out.extend_from_slice(c.contents().as_bytes());
        } else {
            out.push(b' ');
        }
    }
}

/// The attributes the last cell written left in force.
#[derive(Default, PartialEq, Clone, Copy)]
struct CellLook {
    fg: Option<vt100::Color>,
    bg: Option<vt100::Color>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

impl CellLook {
    fn of(c: &vt100::Cell) -> Self {
        CellLook {
            fg: Some(c.fgcolor()),
            bg: Some(c.bgcolor()),
            bold: c.bold(),
            dim: c.dim(),
            italic: c.italic(),
            underline: c.underline(),
            inverse: c.inverse(),
        }
    }

    fn plain(&self) -> bool {
        *self == CellLook::default()
            || (matches!(self.fg, None | Some(vt100::Color::Default))
                && matches!(self.bg, None | Some(vt100::Color::Default))
                && !self.bold
                && !self.dim
                && !self.italic
                && !self.underline
                && !self.inverse)
    }

    /// The SGR that sets this look from scratch.
    fn sgr(&self) -> Vec<u8> {
        let mut p: Vec<String> = vec!["0".into()];
        if self.bold {
            p.push("1".into());
        }
        if self.dim {
            p.push("2".into());
        }
        if self.italic {
            p.push("3".into());
        }
        if self.underline {
            p.push("4".into());
        }
        if self.inverse {
            p.push("7".into());
        }
        let colour = |p: &mut Vec<String>, c: Option<vt100::Color>, base: u8| match c {
            Some(vt100::Color::Idx(n)) if n < 8 => p.push((base + n).to_string()),
            Some(vt100::Color::Idx(n)) if n < 16 => p.push((base + 60 + n - 8).to_string()),
            Some(vt100::Color::Idx(n)) => p.push(format!("{};5;{n}", base + 8)),
            Some(vt100::Color::Rgb(r, g, b)) => p.push(format!("{};2;{r};{g};{b}", base + 8)),
            _ => {}
        };
        colour(&mut p, self.fg, 30);
        colour(&mut p, self.bg, 40);
        format!("\x1b[{}m", p.join(";")).into_bytes()
    }
}

/// One scrollback row of the model, as the face's terminal takes it: the
/// cells with their looks and links, trailing plain blanks dropped, then
/// a line end — unless the row wraps into the next, which then continues
/// it (the full row makes the face wrap exactly there). A picture drawn
/// on the row goes in before the cell it was drawn at (item 66).
fn write_row(out: &mut Vec<u8>, screen: &vt100::Screen, row: u16, cols: u16, pen: &mut Pen, marks: &RowMarks) {
    let wrapped = screen.row_wrapped(row);
    let mut end = cols;
    if !wrapped {
        while end > 0 {
            match screen.cell(row, end - 1) {
                Some(c) if !c.has_contents() && CellLook::of(c).plain() => end -= 1,
                Some(c) if c.is_wide_continuation() => end -= 1,
                _ => break,
            }
        }
    }
    let mut pictures = marks.pictures.iter().peekable();
    let picture = |out: &mut Vec<u8>, pen: &mut Pen, bytes: &[u8]| {
        pen.unlink(out);
        out.extend_from_slice(bytes);
    };
    for col in 0..end {
        while let Some((_, bytes)) = pictures.next_if(|(at, _)| *at <= col) {
            picture(out, pen, bytes);
        }
        let Some(c) = screen.cell(row, col) else { break };
        pen.cell(out, c, marks.links.get(col as usize).and_then(Option::as_ref));
    }
    for (at, bytes) in pictures {
        if *at > end {
            out.extend_from_slice(format!("\x1b[{}G", at + 1).as_bytes());
        }
        picture(out, pen, bytes);
    }
    if !wrapped {
        pen.unlink(out);
        out.extend_from_slice(b"\r\n");
    }
}

/// How many bytes may wait for one client before it is let go. A face
/// that far behind is better served by the ring — it attaches again
/// (`rejoin`) and is poured the last [`RING`] bytes, reset and replay —
/// than by the program waiting for it.
const CLIENT_BEHIND: usize = 2 * RING;

/// **One attached face, fed by a thread of its own.**
///
/// ⚠️ 墓碑 (round NET-2): the PTY reader wrote every chunk to every client
/// itself, blocking, with the ring and the client list locked. A face
/// whose socket stopped draining — a far viewer behind a slow relay, a
/// phone frozen in the background — filled its socket buffer, the write
/// blocked, and the program's output stopped for everyone: the session
/// itself waited on its slowest reader. Now the reader only queues (an
/// unbounded channel, but counted), each client's thread does the
/// writing, and a client more than [`CLIENT_BEHIND`] behind is closed
/// rather than waited for — the same choice Eternal Terminal's catch-up
/// buffer makes (a bounded buffer per reader; past it, the reader starts
/// over from what is kept).
struct Client {
    /// For shutting the socket from outside its writer.
    socket: TcpStream,
    tx: std::sync::mpsc::Sender<Arc<[u8]>>,
    queued: Arc<std::sync::atomic::AtomicUsize>,
}

impl Client {
    fn new(socket: TcpStream) -> std::io::Result<Client> {
        let _ = socket.set_nodelay(true);
        let mut writing = socket.try_clone()?;
        let (tx, rx) = std::sync::mpsc::channel::<Arc<[u8]>>();
        let queued = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let left = queued.clone();
        std::thread::spawn(move || {
            while let Ok(bytes) = rx.recv() {
                if writing.write_all(&bytes).is_err() {
                    let _ = writing.shutdown(std::net::Shutdown::Both);
                    return;
                }
                left.fetch_sub(bytes.len(), Ordering::Relaxed);
            }
        });
        Ok(Client { socket, tx, queued })
    }

    /// Queues `bytes` without waiting. `false` when the client is gone or
    /// too far behind — it is shut, and its face starts over from the
    /// ring when it attaches again.
    fn feed(&self, bytes: &Arc<[u8]>) -> bool {
        let was = self.queued.fetch_add(bytes.len(), Ordering::Relaxed);
        if was + bytes.len() > CLIENT_BEHIND || self.tx.send(bytes.clone()).is_err() {
            let _ = self.socket.shutdown(std::net::Shutdown::Both);
            return false;
        }
        true
    }
}

/// How often the host looks at what its child is doing.
///
/// One cadence for both faces below. They read completely different
/// things — a process group versus a screen — but they answer the same
/// question at the same rate, and two numbers here would be two numbers
/// to keep in step for no reason.
const FACE_POLL: Duration = Duration::from_millis(700);

/// Which pattern table to try against this session's screen, if any.
///
/// **This reads khor's own argv, not the agent's output.** The user
/// wrote `khor open --tui -- claude`; naming the program is something
/// they did, not something khor inferred, which is what makes this
/// different from the ledger's standing refusal to read a vendor off a
/// command line. Two things keep it that way:
///
/// - **An exact basename match, and nothing else.** No alias
///   resolution, no unwrapping `npx`, no prefix matching. Those are the
///   cases where a command line lies, and the ledger's warning is that
///   it lies *convincingly*.
/// - **It never becomes the row's category.** It picks which patterns
///   to try; who the session belongs to is still only answered by a
///   source that read the vendor's own files or heard from its hooks.
///
/// A name that does not match runs no detector at all, and that is the
/// honest end of it: the session then behaves exactly as it did before
/// this existed. Guessing a table would be worse than having none —
/// the vendors' markers contradict each other outright ('esc to cancel'
/// is 忙碌 for gemini and 待批 for claude), so the wrong table is not a
/// blurrier answer, it is a confident opposite one.
fn table_for(command: &str) -> Option<&'static str> {
    let base = Path::new(command).file_name()?.to_str()?;
    khor_detect::vendors().find(|v| *v == base)
}

/// The three words a screen can reach, in khor's six.
///
/// Total and one-way: [`khor_detect::Word`] has no variant for 完成,
/// 中断 or 失败, so this cannot accidentally start claiming an ending.
/// Those are recorded by the host from the child's exit and by
/// `live::face` from what is still running — never from appearances.
fn state_of(word: khor_detect::Word) -> State {
    match word {
        khor_detect::Word::Busy => State::Busy,
        khor_detect::Word::Waiting => State::Blocked,
        khor_detect::Word::Idle => State::Idle,
    }
}

/// The body of `khor _host` — blocks until the child ends.
pub fn host_main(
    root: PathBuf,
    id: SessionId,
    size: (u16, u16),
    cmd: Vec<String>,
) -> Result<(), String> {
    host_main_with_startup(root,id,size,cmd,None)
}

/// Explicit startup input belongs to a newly created host, never a connection.
pub fn host_main_with_startup(root:PathBuf,id:SessionId,size:(u16,u16),cmd:Vec<String>,startup:Option<String>) -> Result<(),String> {
    let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
        .map_err(|e| e.to_string())?;
    let khor = root.join(".khor");
    let live = LiveKind::new(root, khor_core::DeviceId(*key.public().as_bytes()));
    let dir = live
        .dir_of(&id)
        .ok_or_else(|| msg::not_a_session_id(&id.0))?;
    // **The host's own words, left where the opener will look.** The
    // host is spawned detached with all three streams on /dev/null, so
    // until now every reason it had for dying went nowhere and the
    // opener had one sentence for all of them — a timeout
    // (`HOST_NEVER_READY`). See `host_error_path` for what that cost.
    let served = host_serving(&live, &dir, &khor, &id, size, &cmd, startup.as_deref());
    if let Err(why) = &served {
        let _ = std::fs::write(host_error_path(&dir), why);
    }
    served
}

fn host_serving(
    live: &LiveKind,
    dir: &Path,
    khor: &Path,
    id: &SessionId,
    size: (u16, u16),
    cmd: &[String],
    startup: Option<&str>,
) -> Result<(), String> {
    let dir = dir.to_path_buf();
    let id = id.clone();
    let live = live.clone();
    let cmd = cmd.to_vec();
    let kind = id.0.split('/').next().unwrap_or("").to_owned();
    // A PTY carrying a tmux client shows tmux's status bar as its
    // bottom line, so its "last non-empty line" is chrome, never the
    // session's output — the same judgment that keeps a viewer from
    // writing words (below). No preview is written, and one already on
    // disk is removed: it was written by a host from before this gate,
    // and it is exactly the `[sess:zsh*…` junk the row would keep
    // showing (副行要么人话要么空白).
    let mux_client = pty_is_a_multiplexer_client(&cmd);
    if mux_client {
        let _ = std::fs::remove_file(dir.join("last.txt"));
    }
    // A viewer never claims the row (`spawn_viewer_host`): the pid slot,
    // the words and the ending all stay the vendor's.
    let viewer = std::env::var(VIEWER_ENV).ok();
    if viewer.is_none() {
        live.set_pid(&id, std::process::id())?;
    }

    let pty = native_pty_system()
        .openpty(PtySize {
            rows: size.1,
            cols: size.0,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(msg::cant_open_pty)?;
    // **The PTY's command is looked up by khor, against a `PATH` that
    // can answer** — `crate::spawn_path`. A `khor serve` under launchd
    // has `/usr/bin:/bin:/usr/sbin:/sbin` and `tmux` lives in
    // `/opt/homebrew/bin`, so every tui row's terminal died here at
    // exec; and because this process is detached onto /dev/null, what
    // reached the person was 宿主没在 5 秒内就位 — a sentence about
    // *speed*, for something that was never going to work at all
    // (measured 2026-08-31, same binary, only the `PATH` differing).
    //
    // The child is handed that same `PATH`: khor is this terminal, and
    // a shell opened here with launchd's four directories is not the
    // shell the person would have got from their own terminal.
    let path = crate::spawn_path(&cmd[0]);
    let path = crate::legacy_codex::clean_path(khor, &path);
    let found = crate::find_program(&cmd[0], &path)
        .ok_or_else(|| msg::command_not_found(&cmd[0], &path))?;
    let mut builder = CommandBuilder::new(&found);
    // **The shell is taught to mark its commands** (roadmap 1.1,
    // `crate::integration`): a shell this host starts for a shell row,
    // never a viewer's tmux client or a program. A script that cannot be
    // written leaves the shell as it was.
    let integrated = (kind == khor_core::kind::SHELL && viewer.is_none() && !mux_client)
        .then(|| {
            let scripts = crate::integration::dir(khor);
            crate::integration::write_scripts(&scripts).ok()?;
            crate::integration::plan(&scripts, &cmd, |k| std::env::var(k).ok())
        })
        .flatten();
    match &integrated {
        Some(plan) => {
            builder.args(&plan.args);
            for (name, value) in &plan.env {
                builder.env(name, value);
            }
        }
        None => builder.args(&cmd[1..]),
    }
    builder.env("PATH", &path);
    builder.env_remove(crate::legacy_codex::SHIM_ENV);
    builder.env_remove(STARTUP_ENV);
    // **The person's own locale, when this process has none.**
    // launchd's environment has no `LANG`, and a tmux client without a
    // UTF-8 locale falls back to the DEC line-drawing set and writes
    // `_` for every character it cannot encode — so a screen of
    // markdown came through as `_` and `q` while the tmux server was
    // holding it perfectly (`Node::user_locale`, which has tmux's own
    // reading of the two arms). Empty for every khor started from a
    // terminal, which already has one.
    for (name, value) in crate::Node::user_locale() {
        builder.env(name, value);
    }
    builder.env("KHOR_SESSION", &id.0);
    TerminalContext::apply(&mut builder);
    // Where the person was standing when they started this process, not
    // where the process stands now — opening a node pins that to a
    // directory nothing can delete (第十四轮 第 4 条), and a shell born
    // in `/` is a shell nobody asked for.
    if let Some(cwd) = crate::cwd::standing() {
        builder.cwd(cwd);
    }
    let mut child = pty
        .slave
        .spawn_command(builder)
        .map_err(|e| msg::wont_start(&cmd[0], e))?;
    drop(pty.slave);
    let child_pid = child.process_id().ok_or(msg::child_has_no_pid())?;
    let master = Arc::new(Mutex::new(pty.master));
    let mut reader = master
        .lock()
        .unwrap()
        .try_clone_reader()
        .map_err(msg::cant_read_pty)?;
    let writer = Arc::new(Mutex::new(crate::shell_start::StartupInput::new(
        master
            .lock()
            .unwrap()
            .take_writer()
            .map_err(msg::cant_write_pty)?,
        startup,
    )));

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(msg::cant_listen_loopback)?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let cookie = crate::link::fresh_hex()?;
    crate::link::write_private(
        // A viewer's address goes in its own file (`view_file_path`).
        &if viewer.is_some() {
            view_file_path(&dir)
        } else {
            host_file_path(&dir)
        },
        &serde_json::to_vec(&HostFile {
            port,
            cookie: cookie.clone(),
            host_pid: std::process::id(),
            child_pid,
        })
        .map_err(|e| e.to_string())?,
    )?;

    let shared = Arc::new(Shared {
        ring: Mutex::new(Ring::default()),
        clients: Mutex::new(Vec::new()),
        // A tmux client's screen is tmux's (item 41): no model.
        model: Mutex::new((!mux_client).then(|| Model::new(size.1, size.0))),
        changes: AtomicU64::new(0),
    });
    let output_done = Arc::new(AtomicBool::new(false));
    // The questions the program asked its terminal, and which answers
    // may reach it (`answers`, T-4/T-5). Locked alone, never with Shared's.
    let ledger = Arc::new(Mutex::new(crate::answers::Ledger::default()));
    {
        let ledger = ledger.clone();
        let writer = writer.clone();
        let output_done = output_done.clone();
        std::thread::spawn(move || {
            while !output_done.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let own = ledger.lock().unwrap().fallback(std::time::Instant::now());
                if !own.is_empty() {
                    let mut w = writer.lock().unwrap();
                    // Replies belong to the shell's already-pending queries,
                    // not to the startup and keys waiting behind Ready.
                    let _ = w.reply(&own);
                }
            }
        });
    }

    // **The command ledger** (roadmap 1.2, `crate::commands`): fed the
    // same bytes as the ring, written beside the session when something a
    // reader sees changed, and every finished command added to the
    // machine's history. A tmux client's PTY carries the marks of every
    // pane tmux shows, each naming its pane, and the title names the one
    // in front (`commands::Ledger::tmux`, item 21). Never a viewer's:
    // its row is the agent's, and the shells beside it in the window are
    // not.
    let commands = (viewer.is_none() || !mux_client).then(|| {
        Arc::new(Mutex::new(if mux_client {
            crate::commands::Ledger::tmux()
        } else {
            crate::commands::Ledger::default()
        }))
    });
    let history = khor.join("history.jsonl");

    // A screen to match patterns against, for an agent TUI khor can
    // name. None for a shell (the process group answers better), and
    // none for a command whose basename is not a vendor in the table.
    let screen: Option<Arc<Mutex<khor_detect::Screen>>> = (kind == khor_core::kind::TUI)
        .then(|| table_for(&cmd[0]))
        .flatten()
        .map(|_| Arc::new(Mutex::new(khor_detect::Screen::new(size.1, size.0))));

    // PTY → ring + every attached client. Holding ring across the
    // broadcast keeps attachers exact (see Shared).
    {
        let shared = shared.clone();
        let output_done = output_done.clone();
        let screen = screen.clone();
        let last_dir = dir.clone();
        let ledger = ledger.clone();
        let commands = commands.clone();
        let history = history.clone();
        let session = id.0.clone();
        let master = master.clone();
        // A shell's own pty: the one whose program, once it is gone, takes
        // its reporting modes with it (below). Never a tmux client or a
        // viewer, whose foreground is a client of something else, nor an
        // agent row, whose child is the program itself.
        let shells_own = kind == khor_core::kind::SHELL && viewer.is_none() && !mux_client;
        std::thread::spawn(move || {
            // **One read is one chunk on every face, up to 64 KiB** (item
            // 18 B): a program's redraw that the PTY hands over in one
            // read reaches the face whole, not cut at 8 KiB into two
            // chunks a face could paint apart.
            let mut buf = vec![0u8; 64 * 1024];
            // The preview line (`last.txt`): derived after every read,
            // written only when it changed. Not throttled — a throttle
            // here has no trailing edge, and the one update that matters
            // most is the one after the *last* burst of output, exactly
            // the one a leading-edge throttle drops (caught live by the
            // smoke: the typed marker never previewed). The derive is a
            // 4 KiB strip or one screen read per 8 KiB of output —
            // cheap; the change gate keeps the disk out of the loop.
            let mut last_written: Option<String> = None;
            let mut repainted: Option<std::time::Instant> = None;
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                // Before anyone can see it: a question is pending before
                // a face can answer it.
                ledger.lock().unwrap().saw_output(&buf[..n], std::time::Instant::now());
                // **Mouse tracking off once the shell has the terminal
                // back** (item 25; the owner's zsh prompt filling with
                // `35;6;5M…`). A program that turned mouse tracking on and
                // exited without turning it off — killed, crashed, or
                // careless — leaves every face sending reports to a shell
                // that takes them as typing. When a read arrives while a
                // mouse mode is on and the pty's foreground group is the
                // shell itself, the host writes the modes off into the
                // stream, after the read: the faces, the ring, its replay
                // and the ledgers all see the same `l`. Checked only while
                // a mouse mode is on.
                let reset = shells_own
                    && ledger.lock().unwrap().mouse_on()
                    && master.lock().unwrap().process_group_leader().map(|p| p as u32) == Some(child_pid);
                let data: std::borrow::Cow<[u8]> = if reset {
                    ledger
                        .lock()
                        .unwrap()
                        .saw_output(crate::answers::MOUSE_OFF, std::time::Instant::now());
                    let mut v = buf[..n].to_vec();
                    v.extend_from_slice(crate::answers::MOUSE_OFF);
                    std::borrow::Cow::Owned(v)
                } else {
                    std::borrow::Cow::Borrowed(&buf[..n])
                };
                let n = data.len();
                let buf = &data[..];
                // Before the ring, and in its own scope: the screen lock
                // is never held while holding either of the other two,
                // so it cannot join their order (see Shared).
                if let Some(screen) = &screen {
                    screen.lock().unwrap().feed(&buf[..n]);
                }
                // Its own lock, never held with another.
                let marked = commands.as_ref().and_then(|c| {
                    let mut c = c.lock().unwrap();
                    c.feed(&buf[..n], crate::live::now_ms().max(0) as u64);
                    let done = c.take_finished();
                    c.take_changes().then(|| (c.saved(), done))
                });
                if let Some((saved, done)) = &marked {
                    crate::commands::write_saved(&last_dir, saved);
                    crate::commands::append_history(&history, &session, done);
                }
                let integrated = commands.as_ref().is_some_and(|c| c.lock().unwrap().marked);
                if integrated && last_written.is_some() {
                    let _ = std::fs::remove_file(last_dir.join("last.txt"));
                    last_written = None;
                }
                let mut ring = shared.ring.lock().unwrap();
                ring.push(&buf[..n]);
                shared.changes.fetch_add(1, Ordering::SeqCst);
                let lost = shared.model.lock().unwrap().as_mut().is_some_and(|m| m.feed(&buf[..n]));
                let chunk: Arc<[u8]> = Arc::from(&buf[..n]);
                let mut clients = shared.clients.lock().unwrap();
                // Queued, never written here: the program must not wait
                // for its slowest reader (`Client`).
                clients.retain(|c| c.feed(&chunk));
                drop(clients);
                drop(ring);
                // **The model lost its picture: the program draws it
                // again** (item 57), before a face joins and is handed
                // the blank. The faces attached now saw the bytes
                // themselves; to them the repaint is one more frame.
                if lost && repainted.is_none_or(|t| t.elapsed() >= REPAINT_EVERY) {
                    repainted = Some(std::time::Instant::now());
                    repaint_foreground(master.lock().unwrap().process_group_leader());
                }
                // Never for a tmux client (`mux_client`): its bottom
                // line is tmux's status bar, which is true of the
                // screen and a lie about the session. Nor for a shell
                // that marks its commands: its row says what it runs
                // (`Session::doing`), never the prompt's own glyphs.
                if !mux_client && !integrated {
                    // The screen's reading beats the byte tail — it is
                    // what is actually on the terminal; the strip is the
                    // screenless (plain shell) approximation.
                    let line = match &screen {
                        Some(s) => s
                            .lock()
                            .unwrap()
                            .last_line()
                            .map(|l| khor_core::preview(&l)),
                        None => {
                            let ring = shared.ring.lock().unwrap();
                            let tail: Vec<u8> =
                                ring.bytes.iter().rev().take(4096).rev().copied().collect();
                            drop(ring);
                            last_plain_line(&tail)
                        }
                    };
                    if line != last_written {
                        if let Some(l) = &line {
                            let _ = std::fs::write(last_dir.join("last.txt"), l);
                        }
                        last_written = line;
                    }
                }
            }
            output_done.store(true, Ordering::SeqCst);
            // Each client's queue drains first (its writer runs until the
            // channel is empty), then the socket closes with it.
            for c in shared.clients.lock().unwrap().drain(..) {
                drop(c.tx);
                let socket = c.socket;
                let queued = c.queued;
                std::thread::spawn(move || {
                    for _ in 0..200 {
                        if queued.load(Ordering::Relaxed) == 0 {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    let _ = socket.shutdown(std::net::Shutdown::Both);
                });
            }
        });
    }

    // The six-word face for a shell: 忙碌 = the terminal's foreground
    // group is not the shell itself. Never for a viewer — its PTY runs a
    // tmux client, whose foreground says nothing about the session the
    // row is about.
    if kind == khor_core::kind::SHELL && viewer.is_none() {
        let live = live.clone();
        let id = id.clone();
        let master = master.clone();
        let output_done = output_done.clone();
        std::thread::spawn(move || {
            let mut last = State::Busy;
            while !output_done.load(Ordering::SeqCst) {
                std::thread::sleep(FACE_POLL);
                let leader = master.lock().unwrap().process_group_leader();
                let word = match leader {
                    Some(pid) if pid as u32 != child_pid => State::Busy,
                    Some(_) => State::Idle,
                    None => continue,
                };
                // Reported, not read off a screen: this is the kernel's
                // answer to "who owns the terminal", not an appearance.
                if word != last
                    && live
                        .report(&id, word, crate::live::Source::Reported)
                        .is_ok()
                {
                    last = word;
                }
            }
        });
    }

    // The same face for an agent TUI, read off the screen it draws.
    //
    // **This is what a hookless agent had instead of a word.** Until
    // now the branch above was the only one, so a `khor open --tui --
    // <vendor with no hook>` kept the 忙碌 that `register` wrote and
    // wore it until it exited — measured 2026-08-17, and not a word
    // landing on its neighbour but one that is simply always wrong.
    //
    // It reports as `Source::Screen`, which is what keeps it in its
    // place: a sighting from the vendor's own files beats it at the
    // merge (`live::rows`), and a hook overwrites it outright. If this
    // family is ever wrong, it is wrong where nothing better exists.
    if let Some(screen) = screen.clone() {
        let Some(vendor) = table_for(&cmd[0]) else {
            unreachable!("screen only exists when a table was found");
        };
        if let Some(mut detector) = khor_detect::Detector::for_vendor(vendor) {
            let live = live.clone();
            let id = id.clone();
            let output_done = output_done.clone();
            std::thread::spawn(move || {
                // The word `register` wrote, so the first reading is
                // compared against what the row actually says rather
                // than against an assumption.
                let mut current = khor_detect::Word::Busy;
                let mut last = State::Busy;
                while !output_done.load(Ordering::SeqCst) {
                    std::thread::sleep(FACE_POLL);
                    let read = {
                        let s = screen.lock().unwrap();
                        detector.word(&s, current, crate::live::now_ms())
                    };
                    current = read;
                    let word = state_of(read);
                    if word != last && live.report(&id, word, crate::live::Source::Screen).is_ok() {
                        last = word;
                    }
                }
            });
        }
    }

    // **The last screen, on disk** (P5-SHELL item 1, `ring_file_path`):
    // every `RING_SAVE_EVERY` while something was written, and at once on
    // SIGTERM — then the host dies of the signal, as it always did. Never
    // a viewer's (the row is the vendor's) nor a tmux client's (tmux holds
    // that screen); its child's end is saved below, after the last read.
    let keeps_ring = viewer.is_none() && !mux_client;
    let saved = Arc::new(Mutex::new(0u64));
    if keeps_ring {
        #[cfg(unix)]
        // SAFETY: the handler only stores to an atomic. A caught signal
        // goes back to its default across exec, so the child is unchanged.
        unsafe {
            libc::signal(libc::SIGTERM, on_term as *const () as libc::sighandler_t);
        }
        let shared = shared.clone();
        let output_done = output_done.clone();
        let dir = dir.clone();
        let saved = saved.clone();
        std::thread::spawn(move || {
            let tick = Duration::from_millis(100);
            let mut since = Duration::ZERO;
            while !output_done.load(Ordering::SeqCst) {
                std::thread::sleep(tick);
                since += tick;
                if TERMED.load(Ordering::SeqCst) {
                    shared.save_ring(&dir, &mut saved.lock().unwrap());
                    #[cfg(unix)]
                    // SAFETY: back to the default action, then the signal
                    // again: the host ends exactly as an unhandled TERM
                    // ended it before.
                    unsafe {
                        libc::signal(libc::SIGTERM, libc::SIG_DFL);
                        libc::kill(libc::getpid(), libc::SIGTERM);
                    }
                    return;
                }
                if since >= RING_SAVE_EVERY {
                    since = Duration::ZERO;
                    shared.save_ring(&dir, &mut saved.lock().unwrap());
                }
            }
        });
    }

    // Attachers.
    {
        let shared = shared.clone();
        let ledger = ledger.clone();
        let master = master.clone();
        let writer = writer.clone();
        let cmd = cmd.clone();
        std::thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                let _ = conn.set_write_timeout(Some(Duration::from_secs(2)));
                let mut conn = conn;
                let Ok(hello) = read_frame::<Hello>(&mut conn) else {
                    continue;
                };
                if hello.cookie != cookie {
                    let _ = write_frame(
                        &mut conn,
                        &Welcome {
                            ok: false,
                            why: msg::wrong_cookie().into(),
                        },
                    );
                    continue;
                }
                let _ = master.lock().unwrap().resize(PtySize {
                    rows: hello.rows,
                    cols: hello.cols,
                    pixel_width: 0,
                    pixel_height: 0,
                });
                if let Some(m) = shared.model.lock().unwrap().as_mut() {
                    m.resize(hello.rows, hello.cols);
                }
                // The reader has to follow, or it is matching against a
                // screen of a different shape than the one the agent is
                // drawing to — and the first thing to break is the box
                // border that scopes claude's busy rules, since a
                // wrapped border line stops being a line of all `─`.
                if let Some(screen) = &screen {
                    screen.lock().unwrap().resize(hello.rows, hello.cols);
                }
                if write_frame(
                    &mut conn,
                    &Welcome {
                        ok: true,
                        why: format!("{GREETING}{HOST_VERSION}"),
                    },
                )
                .is_err()
                {
                    continue;
                }
                {
                    let Ok(mut readable) = conn.try_clone() else {
                        continue;
                    };
                    // **The client's first word, before anything is
                    // queued** (item 46): a versioned client's `Version`
                    // decides between the snapshot and the ring. Waited for
                    // `FIRST_FRAME` at most; a frame that has begun to
                    // arrive is read whole.
                    let mut first_op: Option<ClientOp> = None;
                    let _ = readable.set_read_timeout(Some(FIRST_FRAME));
                    if matches!(readable.peek(&mut [0u8; 1]), Ok(n) if n > 0) {
                        let _ = readable.set_read_timeout(None);
                        match read_client_frame::<ClientOp>(&mut readable) {
                            Ok(op) => first_op = op,
                            Err(_) => continue,
                        }
                    }
                    let _ = readable.set_read_timeout(None);
                    let agreed_first = match first_op {
                        Some(ClientOp::Version(v)) => v.min(HOST_VERSION),
                        _ => 1,
                    };
                    // Anything else said first (an old client's keystroke
                    // in the window) is the loop's first op below.
                    let mut pending = first_op.filter(|op| !matches!(op, ClientOp::Version(_)));
                    // The first chunk is queued under the ring's lock so
                    // nothing the reader broadcasts is missed or doubled —
                    // and queued, not written, so a slow far viewer
                    // attaching does not stall the program while it
                    // crosses a relay.
                    let ring = shared.ring.lock().unwrap();
                    let Ok(client) = Client::new(conn) else {
                        continue;
                    };
                    // **A tmux row is never replayed the ring** (item 41):
                    // the ring of a tmux client is its redraws over and
                    // over, up to a MiB, parsed visibly on the phone, and
                    // tmux can draw the screen afresh instead; the history
                    // is read from tmux (item 31). The modes still go first.
                    // **A version-3 client gets the snapshot** (item 46);
                    // an older one the ring, as before.
                    let mut model = shared.model.lock().unwrap();
                    let first = match model.as_mut() {
                        _ if mux_client => ring.modes_now(),
                        Some(m) if agreed_first >= 3 => m.snapshot(&ring.modes_now()),
                        _ => ring.replay(),
                    };
                    drop(model);
                    if !client.feed(&Arc::from(first)) {
                        continue;
                    }
                    shared.clients.lock().unwrap().push(client);
                    drop(ring);
                    if mux_client {
                        redraw_tmux_client(&cmd, master.lock().unwrap().tty_name());
                    }
                    // Older clients cannot acknowledge renderer readiness.
                    // They retain attach-time startup; no new op is required.
                    if agreed_first < 5 && writer.lock().unwrap().ready().is_err() {
                        continue;
                    }
                    let master = master.clone();
                    let writer = writer.clone();
                    let ledger = ledger.clone();
                    let shared = shared.clone();
                    std::thread::spawn(move || {
                        let mut readable = readable;
                        // What this client and this host speak: 1 until
                        // the client says otherwise.
                        let mut agreed = agreed_first;
                        loop {
                            let op = match pending.take() {
                                Some(op) => op,
                                None => match read_client_frame::<ClientOp>(&mut readable) {
                                    Ok(Some(op)) => op,
                                    Ok(None) => continue,
                                    Err(_) => break,
                                },
                            };
                            match op {
                                ClientOp::Input(bytes) => {
                                    // An answer only while its question waits
                                    // (`answers`); keys whole.
                                    let bytes = ledger.lock().unwrap().take_input(&bytes, std::time::Instant::now());
                                    if bytes.is_empty() {
                                        continue;
                                    }
                                    let mut w = writer.lock().unwrap();
                                    if w.write_all(&bytes).and_then(|()| w.flush()).is_err() {
                                        break;
                                    }
                                }
                                ClientOp::Resize { cols, rows } => {
                                    let _ = master.lock().unwrap().resize(PtySize {
                                        rows,
                                        cols,
                                        pixel_width: 0,
                                        pixel_height: 0,
                                    });
                                    if let Some(m) = shared.model.lock().unwrap().as_mut() {
                                        m.resize(rows, cols);
                                    }
                                }
                                ClientOp::Version(v) => agreed = v.min(HOST_VERSION),
                                ClientOp::Forget if agreed >= 2 => {
                                    let mut ring = shared.ring.lock().unwrap();
                                    ring.forget();
                                    if let Some(m) = shared.model.lock().unwrap().as_mut() {
                                        m.forget();
                                    }
                                    shared.changes.fetch_add(1, Ordering::SeqCst);
                                    drop(ring);
                                }
                                ClientOp::Forget => {}
                                ClientOp::Scrollback(lines) if agreed >= 4 => {
                                    if let Some(m) = shared.model.lock().unwrap().as_mut() {
                                        m.keep(lines as usize);
                                    }
                                }
                                ClientOp::Scrollback(_) => {}
                                ClientOp::Ready if agreed >= 5 => {
                                    if writer.lock().unwrap().ready().is_err() {
                                        break;
                                    }
                                }
                                ClientOp::Ready => {}
                            }
                        }
                    });
                }
            }
        });
    }

    // The ending — recorded even if nobody is watching; leniency on the
    // writes because `close` may have removed the dir already. A viewer
    // records nothing (its child ending is a detach, not the session
    // ending) and instead cleans up after itself: the host marker always,
    // and the whole dir when the attach minted it — the row then goes
    // back to being a discovered one, exactly as before the bridge.
    let status = child.wait().map_err(msg::host_child_wait)?;
    match &viewer {
        None => {
            let _ = live.record_exit(&id, status.exit_code() as i32);
        }
        Some(v) => {
            if v == "made" {
                let _ = std::fs::remove_dir_all(&dir);
            } else {
                let _ = std::fs::remove_file(view_file_path(&dir));
            }
        }
    }
    for _ in 0..20 {
        if output_done.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // The child ended: its last screen, after the last read. `close`
    // may have removed the dir already, and then this writes nothing.
    if keeps_ring && dir.exists() {
        shared.save_ring(&dir, &mut saved.lock().unwrap());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-16: a frame over `MAX_OP` is skipped and the next one still
    /// arrives — the client's input loop does not end on it.
    #[test]
    fn an_oversize_frame_costs_that_frame_not_the_client() {
        let mut wire = Vec::new();
        write_frame(&mut wire, &ClientOp::Input(vec![b'x'; MAX_OP + 1])).unwrap();
        write_frame(&mut wire, &ClientOp::Input(b"after".to_vec())).unwrap();
        let mut r = std::io::Cursor::new(wire);
        assert!(read_client_frame::<ClientOp>(&mut r).unwrap().is_none());
        match read_client_frame::<ClientOp>(&mut r).unwrap() {
            Some(ClientOp::Input(b)) => assert_eq!(b, b"after"),
            _ => panic!("the frame after the oversize one was lost"),
        }
    }

    /// **Pictures do not push the text out of the ring** (roadmap 3.3).
    /// Before, three 400 KiB `imgcat` plots between a command's output
    /// lines trimmed most of that output, and a trim inside a plot
    /// replayed the rest of its base64 as text. Now the text is all
    /// there, the replay starts on text, and the newest plots are in
    /// place.
    #[test]
    fn pictures_do_not_push_the_text_out_of_the_ring() {
        let plot = |n: u8| {
            [b"\x1b]1337;File=inline=1:".as_slice(), &vec![b'A' + n; 400 << 10], b"\x07"].concat()
        };
        let mut ring = Ring::default();
        let mut text = Vec::new();
        for n in 0..3u8 {
            let line = format!("line {n} of the output\r\n").repeat(1000);
            ring.push(line.as_bytes());
            text.extend_from_slice(line.as_bytes());
            ring.push(&plot(n));
        }
        assert_eq!(ring.bytes.iter().copied().collect::<Vec<_>>(), text);
        let replay = ring.replay();
        assert!(replay.starts_with(b"line 0"), "the replay opens on text, not base64");
        for n in 0..3u8 {
            assert!(replay.windows(16).any(|w| w == [b'A' + n; 16]), "plot {n} is in the replay");
        }
    }

    /// A path is fine; the basename is what is compared.
    #[test]
    fn a_vendors_own_name_picks_its_table() {
        assert_eq!(table_for("claude"), Some("claude"));
        assert_eq!(
            table_for("/Users/someone/.local/bin/claude"),
            Some("claude")
        );
        assert_eq!(table_for("codex"), Some("codex"));
        assert_eq!(table_for("github-copilot"), Some("github-copilot"));
    }

    /// **The half that matters.** Everything here is a way a command
    /// line lies about what is behind it, and the ledger's warning is
    /// that it lies convincingly. Matching any of them would apply one
    /// vendor's markers to another's screen — and the markers contradict
    /// each other outright, so that is not a vaguer answer, it is a
    /// confident opposite one. No match runs no detector, which leaves
    /// the session exactly where it was before this existed.
    #[test]
    fn anything_that_merely_resembles_a_vendor_picks_no_table() {
        for lie in [
            "npx",       // the vendor is an argument, not the command
            "my-claude", // someone's wrapper
            "claude-wrapper",
            "claude.sh", // a script around it
            "Claude",    // the table is lower case and so are the names
            "sh",
            "/bin/sh",
            "",
        ] {
            assert_eq!(table_for(lie), None, "{lie:?} must not select a table");
        }
    }

    /// Three in, three out. The other three of khor's six are endings
    /// and are unreachable from here — not by this function's choice but
    /// because `khor_detect::Word` has no variant for them.
    #[test]
    fn the_three_words_a_screen_reaches_map_to_khors_own() {
        assert_eq!(state_of(khor_detect::Word::Busy), State::Busy);
        assert_eq!(state_of(khor_detect::Word::Waiting), State::Blocked);
        assert_eq!(state_of(khor_detect::Word::Idle), State::Idle);
    }
}

#[cfg(test)]
mod last_line_tests {
    use super::{
        CellLook, MODEL_SCROLLBACK, MODEL_SCROLLBACK_MOST, Model, last_plain_line, looks_like_a_tmux_status_line, pty_is_a_multiplexer_client, read_last, tmux_socket_args,
    };
    use crate::modes::Modes;
    use std::path::Path;

    /// The preview strip reads bytes the way `tail -1` would: ANSI
    /// dropped, CR dropped, the last line with visible content wins.
    #[test]
    fn ansi_is_stripped_and_the_last_visible_line_wins() {
        let bytes = b"\x1b[32mhello\x1b[0m world\r\n\x1b]0;title\x07$ make test\r\n\r\n";
        assert_eq!(last_plain_line(bytes).as_deref(), Some("$ make test"));
    }

    /// **A prompt's frame is never a preview** (the audit fix plan F2.3):
    /// a line of box drawing gives way to the line above it.
    #[test]
    fn a_frame_only_line_gives_way_to_the_line_above() {
        let bytes = "cargo test -p khor-node\r\n\u{256d}\u{2500} \u{2500}\u{256e}\r\n\u{2570}\u{2500} \u{2500}\u{256f}\r\n".as_bytes();
        assert_eq!(last_plain_line(bytes).as_deref(), Some("cargo test -p khor-node"));
        assert!(last_plain_line("\u{2570}\u{2500}\u{256f}\r\n".as_bytes()).is_none());
    }

    /// **A tmux status line is never a preview** (副行要么人话要么空白).
    /// The two positives are real samples off this machine's own rows —
    /// the second with the stray character the ANSI strip left in front.
    /// The negatives are the control: ordinary lines wearing brackets
    /// and colons must keep their preview, or the filter is a broad net
    /// eating real output.
    #[test]
    fn a_tmux_status_line_is_refused_and_ordinary_lines_are_not() {
        for junk in [
            r#"[khor0-1870:zsh*"" 15:04 21-Aug-25"#,
            r#"m[keyprobe-0:zsh*"Mac-mini" 09:12"#,
        ] {
            assert!(looks_like_a_tmux_status_line(junk), "must refuse: {junk}");
        }
        for fine in [
            "$ make test",
            "[INFO] 12:30:45 build finished",
            "error[E0063]: missing field `dead`",
            "[2026-08-23] done",
        ] {
            assert!(!looks_like_a_tmux_status_line(fine), "must keep: {fine}");
        }
    }

    /// The whole read path: a `last.txt` holding a status line answers
    /// nothing, and one holding real output answers it — the same file,
    /// so the blank is the filter and not a broken read.
    #[test]
    fn a_last_file_full_of_tmux_chrome_reads_as_no_preview() {
        let dir = std::env::temp_dir().join(format!("khor-last-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("last.txt"),
            r#"[khor0-1870:zsh*"" 15:04 21-Aug-25"#,
        )
        .unwrap();
        assert_eq!(read_last(&dir), None, "tmux chrome must read as blank");
        std::fs::write(dir.join("last.txt"), "$ cargo build").unwrap();
        assert_eq!(
            read_last(&dir).as_deref(),
            Some("$ cargo build"),
            "control: real output survives"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Which hosts the writer gate applies to: any argv whose command is
    /// tmux, by basename — both bridge shapes get there through one.
    /// The three streams the snapshot is checked against, here and in the
    /// face's gate (`apps/gui/scripts/check-snapshot.mjs`, through xterm):
    /// a shell transcript with colours and a wrapped line, a full-screen
    /// program on the alternate screen holding the mouse, and wide
    /// characters with every attribute.
    fn snapshot_streams() -> Vec<(&'static str, Vec<u8>)> {
        let mut shell = Vec::new();
        for i in 0..120 {
            shell.extend_from_slice(format!("\x1b[32mdesk\x1b[0m:\x1b[34m~/work\x1b[0m$ echo {i}\r\n").as_bytes());
            if i % 7 == 0 {
                shell.extend_from_slice(format!("\x1b[1;33mwarning\x1b[0m line {i} {}\r\n", "0123456789".repeat(12)).as_bytes());
            } else {
                shell.extend_from_slice(format!("{i}\r\n").as_bytes());
            }
        }
        shell.extend_from_slice(b"\x1b[32mdesk\x1b[0m:\x1b[34m~/work\x1b[0m$ ");
        let mut tui = Vec::new();
        tui.extend_from_slice(b"\x1b[?1049h\x1b[?1000h\x1b[?1006h\x1b[H\x1b[2J");
        tui.extend_from_slice("\x1b[2;3H\x1b[36m\u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}\x1b[0m".as_bytes());
        tui.extend_from_slice("\x1b[3;3H\x1b[36m\u{2502}\x1b[0m > hi \x1b[36m\u{2502}\x1b[0m".as_bytes());
        tui.extend_from_slice("\x1b[4;3H\x1b[36m\u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}\x1b[0m".as_bytes());
        tui.extend_from_slice(b"\x1b[10;1H\x1b[7m status \x1b[0m\x1b[3;9H");
        let mut wide = Vec::new();
        wide.extend_from_slice("\x1b[1m\u{4f60}\u{597d}\x1b[0m \x1b[4m\u{4e16}\u{754c}\x1b[0m \x1b[38;5;208mo\x1b[48;2;10;20;30mb\x1b[0m\r\n".as_bytes());
        wide.extend_from_slice(format!("{}\r\n", "\u{6f22}".repeat(50)).as_bytes());
        wide.extend_from_slice(format!("{}", "x".repeat(80)).as_bytes());
        wide.extend_from_slice(b"\x1b[3;2mdim italic\x1b[0m end\r\n$ ");
        vec![("shell", shell), ("tui", tui), ("wide", wide), ("links", links_stream()), ("pictures", pictures_stream())]
    }

    /// `ls --hyperlink` and friends (item 66): OSC 8 links scrolled into
    /// the history and on the screen, one with an `id=` and a colour, one
    /// the grid wraps, one in Chinese ended by BEL, and one a line
    /// redrawn over (`gone`), whose text is no longer the link's.
    fn links_stream() -> Vec<u8> {
        let mut s = b"$ ls --hyperlink\r\n".to_vec();
        for i in 0..40 {
            s.extend_from_slice(format!("\x1b]8;;file://desk/home/u/src/main{i}.rs\x1b\\main{i}.rs\x1b]8;;\x1b\\  plain {i}\r\n").as_bytes());
        }
        s.extend_from_slice(b"\x1b[34m\x1b]8;id=x1;https://example.com/a\x1b\\docs\x1b]8;;\x1b\\\x1b[0m and more\r\n");
        let long = format!("https://example.com/{}", "very-long-path/".repeat(6));
        s.extend_from_slice(format!("see \x1b]8;;{long}\x1b\\{long}\x1b]8;;\x1b\\ end\r\n").as_bytes());
        s.extend_from_slice("\x1b]8;;file:///tmp/%E6%8A%A5%E5%91%8A.txt\x07\u{62a5}\u{544a}.txt\x1b]8;;\x07 ok\r\n".as_bytes());
        s.extend_from_slice(b"\x1b]8;;https://gone.example/\x1b\\gone\x1b]8;;\x1b\\\r\x1b[Kplain over it\r\n");
        s.extend_from_slice(b"$ ");
        s
    }

    const IIP_A: &[u8] = b"\x1b]1337;File=name=YS5wbmc=;inline=1:QUFBQQ==\x07";
    const IIP_C: &[u8] = b"\x1b]1337;File=name=Yy5wbmc=;inline=1:Q0NDQw==\x1b\\";
    const SIXEL_B: &[u8] = b"\x1bP0;1;0q\"1;1;4;4#0;2;100;0;0#0~~~~-\x1b\\";

    /// `imgcat` and `chafa` (item 66): a picture scrolled into the
    /// history, one cleared off the screen with `clear` (gone from a face
    /// that watched), and one on the screen at the end.
    fn pictures_stream() -> Vec<u8> {
        let mut s = b"$ imgcat a.png\r\n".to_vec();
        s.extend_from_slice(IIP_A);
        s.extend_from_slice(b"\r\n");
        for i in 0..30 {
            s.extend_from_slice(format!("$ echo {i}\r\n{i}\r\n").as_bytes());
        }
        s.extend_from_slice(b"$ imgcat c.png\r\n");
        s.extend_from_slice(IIP_C);
        s.extend_from_slice(b"\r\n$ clear\r\n\x1b[H\x1b[2J$ ls\r\na b\r\n$ chafa b\r\n");
        s.extend_from_slice(SIXEL_B);
        s.extend_from_slice(b"\r\n$ ");
        s
    }

    /// The OSC 8 runs a stream prints, in order: each link's target and
    /// the text printed under it, consecutive runs of one target as one
    /// (a run the grid wrapped may be written row by row). Read without
    /// the host's model, as the oracle for it.
    fn linked_text(bytes: &[u8]) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut on: Option<String> = None;
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == 0x1b && bytes.get(i + 1) == Some(&b']') {
                let start = i + 2;
                let mut j = start;
                let mut end = None;
                while j < bytes.len() {
                    if bytes[j] == 0x07 {
                        end = Some((j, j + 1));
                        break;
                    }
                    if bytes[j] == 0x1b && bytes.get(j + 1) == Some(&b'\\') {
                        end = Some((j, j + 2));
                        break;
                    }
                    j += 1;
                }
                let Some((stop, next)) = end else { break };
                let body = String::from_utf8_lossy(&bytes[start..stop]).into_owned();
                if let Some(rest) = body.strip_prefix("8;") {
                    let uri = rest.split_once(';').map(|(_, u)| u).unwrap_or("");
                    on = (!uri.is_empty()).then(|| uri.to_owned());
                }
                i = next;
                continue;
            }
            if b == 0x1b {
                // CSI to its final byte; any other escape is two bytes,
                // and a DCS runs to its ST.
                match bytes.get(i + 1) {
                    Some(b'[') => {
                        i += 2;
                        while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                            i += 1;
                        }
                        i += 1;
                    }
                    Some(b'P') => {
                        i += 2;
                        while i + 1 < bytes.len() && !(bytes[i] == 0x1b && bytes[i + 1] == b'\\') {
                            i += 1;
                        }
                        i += 2;
                    }
                    _ => i += 2,
                }
                continue;
            }
            if b < 0x20 || b == 0x7f {
                i += 1;
                continue;
            }
            let len = match b {
                0xf0.. => 4,
                0xe0.. => 3,
                0xc0.. => 2,
                _ => 1,
            };
            let ch = String::from_utf8_lossy(&bytes[i..(i + len).min(bytes.len())]).into_owned();
            i += len;
            if let Some(uri) = &on {
                match out.last_mut() {
                    Some((u, t)) if u == uri => t.push_str(&ch),
                    _ => out.push((uri.clone(), ch)),
                }
            }
        }
        out
    }

    /// Where each of `pictures` is drawn in `bytes`, in the order drawn:
    /// which one, the text of the row above the cursor, and the cursor's
    /// column — read by a terminal that has read everything before it.
    fn picture_places(bytes: &[u8], pictures: &[&[u8]]) -> Vec<(usize, String, u16)> {
        let mut found: Vec<(usize, usize)> = Vec::new();
        for (n, p) in pictures.iter().enumerate() {
            if let Some(at) = bytes.windows(p.len()).position(|w| w == *p) {
                found.push((at, n));
            }
        }
        found.sort();
        found
            .into_iter()
            .map(|(at, n)| {
                let mut t = vt100::Parser::new(24, 80, 5000);
                t.process(&bytes[..at]);
                let s = t.screen_mut();
                let (row, col) = s.cursor_position();
                let above = if row > 0 {
                    s.rows(0, 80).nth(row as usize - 1).unwrap_or_default()
                } else {
                    s.set_scrollback(1);
                    s.rows(0, 80).next().unwrap_or_default()
                };
                (n, above.trim_end().to_owned(), col)
            })
            .collect()
    }

    /// **A link or picture printed before an attach is alive after it**
    /// (item 66): the snapshot carries the OSC 8 links still on their
    /// text, history and screen alike, and the pictures still kept, each
    /// where a face that watched them arrive has it. A link whose text
    /// was redrawn and a picture `clear` took are not carried.
    #[test]
    fn a_snapshot_carries_the_links_and_the_pictures() {
        let raw = links_stream();
        let mut model = Model::new(24, 80);
        model.feed(&raw);
        let snap = model.snapshot(b"");
        let want: Vec<_> = linked_text(&raw).into_iter().filter(|(u, _)| !u.contains("gone")).collect();
        assert_eq!(want.len(), 43, "the oracle reads the stream's links");
        assert_eq!(linked_text(&snap), want, "the links on their text, in order");
        assert!(!String::from_utf8_lossy(&snap).contains("gone.example"), "a link redrawn over is not carried");

        let raw = pictures_stream();
        let mut model = Model::new(24, 80);
        model.feed(&raw);
        let snap = model.snapshot(b"");
        let all = [IIP_A, IIP_C, SIXEL_B];
        let watched = picture_places(&raw, &all);
        assert_eq!(watched.iter().map(|p| p.0).collect::<Vec<_>>(), [0, 1, 2], "the oracle finds the stream's pictures");
        let want: Vec<_> = watched.into_iter().filter(|p| p.0 != 1).collect();
        assert_eq!(picture_places(&snap, &all), want, "the pictures kept, each where it was drawn; the cleared one not");
    }

    /// Each row's links, history first, as the model hands them out.
    fn links_of(model: &mut Model) -> Vec<Vec<Option<String>>> {
        let top = model.beside.scrolled();
        let s = model.parser.screen_mut();
        let (rows, cols) = s.size();
        s.set_scrollback(usize::MAX);
        let total = s.scrollback();
        let mut out = Vec::new();
        let row = |s: &vt100::Screen, r: u16, at: u64| {
            let links = model.beside.marks(s.alternate_screen(), at, s, r, cols).links;
            (0..cols as usize).map(|c| links.get(c).cloned().flatten().map(|l| l.to_string())).collect::<Vec<_>>()
        };
        for h in 0..total {
            s.set_scrollback(total - h);
            out.push(row(s, 0, top - (total - h) as u64));
        }
        s.set_scrollback(0);
        let top = if s.alternate_screen() { 0 } else { top };
        for r in 0..rows {
            out.push(row(s, r, top + r as u64));
        }
        out
    }

    /// **The links come back on the same cells** (item 66): a model fed
    /// the snapshot hands out the links a model fed the stream does, cell
    /// for cell over the history and the screen — fed whole or a byte at
    /// a time, with the history full past its cap, and on a program's
    /// alternate screen.
    #[test]
    fn the_links_come_back_on_the_same_cells() {
        let mut full = Vec::new();
        for i in 0..MODEL_SCROLLBACK + 100 {
            full.extend_from_slice(format!("\x1b]8;;https://e.example/{i}\x1b\\item {i}\x1b]8;;\x1b\\ text\r\n").as_bytes());
        }
        let mut alt = b"$ vim\r\n\x1b[?1049h\x1b[H\x1b[2J".to_vec();
        alt.extend_from_slice(b"\x1b[3;5H\x1b[4m\x1b]8;;https://alt.example/\x1b\\in the program\x1b]8;;\x1b\\\x1b[0m\x1b[10;1H");
        alt.extend_from_slice(b"\x1b[5;2H\x1b]1337;File=inline=1:QQ==\x07\x1b[20;1H");
        for (name, raw) in [("links", links_stream()), ("full", full), ("alt", alt)] {
            let mut a = Model::new(24, 80);
            a.feed(&raw);
            let want = links_of(&mut a);
            assert!(want.iter().flatten().any(Option::is_some), "{name}: the stream has links in the model");
            let snap = a.snapshot(b"");
            let mut b = Model::new(24, 80);
            b.feed(&snap);
            assert_eq!(links_of(&mut b), want, "{name}: the snapshot's links on the stream's cells");
            if name != "full" {
                let mut c = Model::new(24, 80);
                for byte in raw.chunks(1) {
                    c.feed(byte);
                }
                assert_eq!(c.snapshot(b""), snap, "{name}: fed a byte at a time, the same snapshot");
            }
        }
    }

    /// What the side table costs the reader (item 66): one model fed the
    /// same 20 000 lines plain and with every name linked, as
    /// `ls --hyperlink` prints them, in 4 KiB reads. Run alone:
    /// `cargo test --release -p khor-node --lib model_feed_cost -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn model_feed_cost() {
        let lines = |linked: bool| {
            let mut s = Vec::new();
            for i in 0..20_000 {
                let name = format!("file-{i}.rs");
                if linked {
                    s.extend_from_slice(format!("\x1b]8;;file://desk/home/u/{name}\x1b\\{name}\x1b]8;;\x1b\\  4.0K  Oct  7\r\n").as_bytes());
                } else {
                    s.extend_from_slice(format!("{name}  4.0K  Oct  7\r\n").as_bytes());
                }
            }
            s
        };
        for linked in [false, true] {
            let raw = lines(linked);
            let mut model = Model::new(40, 120);
            let t = std::time::Instant::now();
            for chunk in raw.chunks(4096) {
                model.feed(chunk);
            }
            let fed = t.elapsed();
            let t = std::time::Instant::now();
            let snap = model.snapshot(b"");
            eprintln!("{}: fed {} KB in {:.1} ms, snapshot {} KB in {:.1} ms", if linked { "linked" } else { "plain" }, raw.len() >> 10, fed.as_secs_f64() * 1e3, snap.len() >> 10, t.elapsed().as_secs_f64() * 1e3);
        }
    }

    /// The snapshot of a model with no links and no pictures is what it
    /// was before item 66, byte for byte: the face's fixtures stand.
    #[test]
    fn a_plain_snapshot_is_unchanged() {
        for (name, raw) in snapshot_streams().into_iter().filter(|(n, _)| ["shell", "tui", "wide"].contains(n)) {
            let mut model = Model::new(24, 80);
            model.feed(&raw);
            let mut modes = Modes::default();
            modes.feed(&raw);
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui/scripts/fixtures/snapshot");
            let kept = std::fs::read(dir.join(format!("{name}.snap"))).unwrap();
            assert_eq!(model.snapshot(&modes.preamble()), kept, "{name}");
        }
    }

    /// A snapshot fed to a fresh terminal shows what the model shows:
    /// the screen cell for cell, the cursor, the history's text, the
    /// alternate screen and the modes (item 46).
    #[test]
    fn a_snapshot_rebuilds_the_screen_and_its_history() {
        for (name, raw) in snapshot_streams() {
            let mut model = Model::new(24, 80);
            model.feed(&raw);
            let mut modes = Modes::default();
            modes.feed(&raw);
            let snap = model.snapshot(&modes.preamble());
            let mut again = vt100::Parser::new(24, 80, MODEL_SCROLLBACK);
            again.process(&snap);
            let a = model.parser.screen();
            let b = again.screen();
            assert_eq!(a.alternate_screen(), b.alternate_screen(), "{name}: the buffer");
            assert_eq!(a.contents(), b.contents(), "{name}: the screen's text");
            assert_eq!(a.cursor_position(), b.cursor_position(), "{name}: the cursor");
            for r in 0..24 {
                for c in 0..80 {
                    let (x, y) = (a.cell(r, c).unwrap(), b.cell(r, c).unwrap());
                    assert_eq!((x.contents(), CellLook::of(x) == CellLook::of(y)), (y.contents(), true), "{name}: cell {r},{c}");
                }
            }
            assert_eq!(a.mouse_protocol_mode(), b.mouse_protocol_mode(), "{name}: the mouse");
            if !a.alternate_screen() {
                let mut a = Model::new(24, 80);
                a.feed(&raw);
                let a = a.parser.screen_mut();
                a.set_scrollback(usize::MAX);
                let total = a.scrollback();
                b_history(&mut again, total, a, name);
            }
            assert!(snap.len() < raw.len() + 4096, "{name}: a snapshot is not longer than the stream it stands for by more than its framing ({} vs {})", snap.len(), raw.len());
        }
    }

    /// The history rows of the rebuilt terminal read as the model's, row
    /// for row from the top.
    fn b_history(again: &mut vt100::Parser, total: usize, a: &mut vt100::Screen, name: &str) {
        let b = again.screen_mut();
        b.set_scrollback(usize::MAX);
        assert_eq!(b.scrollback(), total, "{name}: the history's length");
        let text = |s: &mut vt100::Screen, off: usize| {
            s.set_scrollback(off);
            s.rows(0, 80).next().unwrap_or_default()
        };
        let mut off = total;
        while off > 0 {
            assert_eq!(text(a, off), text(b, off), "{name}: history row {} from the top", total - off);
            off -= 1;
        }
    }

    /// The history's length in rows, and its oldest row's text.
    fn history_of(model: &mut Model) -> (usize, String) {
        let s = model.parser.screen_mut();
        s.set_scrollback(usize::MAX);
        let total = s.scrollback();
        let oldest = s.rows(0, 80).next().unwrap_or_default();
        s.set_scrollback(0);
        (total, oldest.trim_end().to_owned())
    }

    /// **The faces' 历史行数** (item 56): a face that asks for more grows
    /// the model's history without losing what it holds or the screen; a
    /// smaller ask never cuts it; the ask is capped at the most.
    #[test]
    fn the_model_keeps_the_history_the_faces_ask_for() {
        let mut model = Model::new(24, 80);
        model.feed(b"\x1b[?2004h");
        for i in 0..3000 {
            model.feed(format!("\x1b[3{}mline {i}\x1b[0m\r\n", i % 7 + 1).as_bytes());
        }
        model.feed(b"$ typed");
        assert_eq!(history_of(&mut model).0, MODEL_SCROLLBACK, "the length before any face asks");
        let before = model.parser.screen().contents_formatted();
        model.keep(5000);
        assert_eq!(model.parser.screen().contents_formatted(), before, "the screen, its colours and the cursor survive the rebuild");
        assert!(model.parser.screen().bracketed_paste(), "the modes survive it");
        let (total, oldest) = history_of(&mut model);
        assert_eq!((total, oldest.as_str()), (MODEL_SCROLLBACK, "line 977"), "the rows held are kept, none invented");
        for i in 3000..9000 {
            model.feed(format!("line {i}\r\n").as_bytes());
        }
        assert_eq!(history_of(&mut model).0, 5000, "the history grows to the ask");
        model.keep(1000);
        assert_eq!(history_of(&mut model).0, 5000, "a smaller ask (the phone's) does not cut a larger one (the Mac's)");
        model.keep(50_000);
        for i in 9000..21000 {
            model.feed(format!("line {i}\r\n").as_bytes());
        }
        assert_eq!(history_of(&mut model).0, MODEL_SCROLLBACK_MOST, "capped at the most");
    }

    /// An ask that comes while a program holds the alternate screen waits
    /// for it to end: the rebuild cannot carry the normal screen under it.
    #[test]
    fn the_model_grows_after_the_alternate_screen() {
        let mut model = Model::new(24, 80);
        for i in 0..2500 {
            model.feed(format!("line {i}\r\n").as_bytes());
        }
        model.feed(b"\x1b[?1049h\x1b[Hfull screen");
        model.keep(5000);
        assert_eq!(model.keeps, MODEL_SCROLLBACK, "not rebuilt on the alternate screen");
        model.feed(b"\x1b[?1049l");
        assert_eq!(model.keeps, 5000, "rebuilt once it ends");
        assert_eq!(history_of(&mut model), (MODEL_SCROLLBACK, "line 477".to_owned()), "the normal screen's history came through");
    }

    /// A shell's history, then a full-screen program with CJK to its
    /// right edge (40 wide characters fill 80 columns) and the cursor
    /// parked in it, the way a tmux client leaves its screen.
    fn tmux_with_cjk_to_the_edge() -> Vec<u8> {
        let mut raw = Vec::new();
        for i in 0..100 {
            raw.extend_from_slice(format!("line {i}\r\n").as_bytes());
        }
        raw.extend_from_slice(b"\x1b[?1049h\x1b[?1h\x1b=\x1b[H\x1b[2J");
        for r in 1..=6 {
            raw.extend_from_slice(format!("\x1b[{r};1H{}", "\u{4e2d}".repeat(40)).as_bytes());
        }
        raw.extend_from_slice(b"\x1b[7;1HMARK\x1b[24;1H\x1b[42m[one] 0:sh*\x1b[0m\x1b[7;5H");
        raw
    }

    /// A tmux client's redraw after a resize: every row from the top,
    /// each erased to its end.
    fn redraw(cols: usize) -> Vec<u8> {
        let mut raw = b"\x1b[H".to_vec();
        for r in 1..=6 {
            raw.extend_from_slice(format!("\x1b[{r};1H{}\x1b[K", "\u{6587}".repeat(cols / 2)).as_bytes());
        }
        raw.extend_from_slice(b"\x1b[7;1HREDRAWN\x1b[K\x1b[7;8H");
        raw
    }

    /// The model's snapshot as a face shows it.
    fn face_of(model: &mut Model, modes: &[u8], rows: u16, cols: u16) -> vt100::Parser {
        let snap = model.snapshot(modes);
        let mut face = vt100::Parser::new(rows, cols, MODEL_SCROLLBACK);
        face.process(&snap);
        face
    }

    /// **A narrowing does not panic the model** (item 57, measured with
    /// a real tmux client's bytes): vt100's resize leaves a wide character
    /// cut at the new last column, and tmux's redraw erasing that cell
    /// panicked the parser, so the model came back blank and lost the
    /// shell's history. The resize erases what it would cut first.
    #[test]
    fn a_narrowing_over_wide_characters_keeps_the_screen_and_the_history() {
        let raw = tmux_with_cjk_to_the_edge();
        let mut modes = Modes::default();
        modes.feed(&raw);
        let mut model = Model::new(24, 80);
        let _ = model.feed(&raw);
        model.resize(24, 79);
        let _ = model.feed(&redraw(79));
        let face = face_of(&mut model, &modes.preamble(), 24, 79);
        let s = face.screen();
        assert!(s.alternate_screen(), "the face is on the program's screen");
        assert_eq!(s.rows(0, 79).nth(6).unwrap().trim_end(), "REDRAWN", "the redraw is what the face is handed");
        assert_eq!(s.rows(0, 79).next().unwrap().trim_end(), "\u{6587}".repeat(39), "the wide rows at the new width");
        let _ = model.feed(b"\x1b[?1049l");
        assert_eq!(history_of(&mut model).1, "line 0", "the shell's history under the program is kept");
    }

    /// **A model that panics keeps its screen for the repaint** (item
    /// 57): whatever vt100 panics on, the fresh parser stays on the
    /// alternate screen with the modes the program set, and says so, so
    /// the host asks the program to draw again and the redraw lands
    /// where the face will look. Left on the normal screen, a full
    /// redraw went to the normal grid and the face, switched to the
    /// alternate screen by the modes, was handed a blank.
    #[test]
    fn a_model_that_panics_keeps_its_screen_for_the_repaint() {
        let raw = tmux_with_cjk_to_the_edge();
        let mut modes = Modes::default();
        modes.feed(&raw);
        let mut model = Model::new(24, 80);
        let _ = model.feed(&raw);
        // vt100's own resize, without the model's care: the cut the
        // panic comes from.
        model.parser.screen_mut().set_size(24, 79);
        let lost = model.feed(&redraw(79));
        assert!(lost, "the model says it lost its picture");
        assert!(model.parser.screen().alternate_screen(), "still on the program's screen");
        assert!(model.parser.screen().application_cursor(), "the program's modes are kept");
        let _ = model.feed(&redraw(79));
        let face = face_of(&mut model, &modes.preamble(), 24, 79);
        assert_eq!(face.screen().rows(0, 79).nth(6).unwrap().trim_end(), "REDRAWN", "the repaint is what the face is handed, not a blank");
    }

    /// What one shell's model costs (item 56): the process's resident
    /// size before and after filling one model's history, `MODEL_COLS`
    /// wide and `MODEL_LINES` long — one measurement a process, since
    /// freed pages stay resident. Run alone:
    /// `MODEL_COLS=80 MODEL_LINES=10000 cargo test -p khor-node --lib model_memory -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn model_memory() {
        let rss = || {
            let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().unwrap();
            String::from_utf8_lossy(&out.stdout).trim().parse::<u64>().unwrap()
        };
        let env = |k: &str, or: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(or);
        let (cols, lines) = (env("MODEL_COLS", 80) as u16, env("MODEL_LINES", MODEL_SCROLLBACK));
        let start = rss();
        let mut model = Model::new(40, cols);
        model.keep(lines);
        let line = format!("\x1b[32m$\x1b[0m {}\r\n", "x".repeat(40.min(cols as usize - 3)));
        for _ in 0..lines + 40 {
            model.feed(line.as_bytes());
        }
        assert_eq!(history_of(&mut model).0, lines.min(MODEL_SCROLLBACK_MOST).max(MODEL_SCROLLBACK));
        eprintln!("model {cols} columns, {lines} rows: {:.1} MB", rss().saturating_sub(start) as f64 / 1024.0);
    }

    /// ⌘K in the model: the history and the rows above the cursor's are
    /// gone, the cursor's row stands first, the attributes and modes stay.
    #[test]
    fn the_model_forgets_what_was_cleared() {
        let mut model = Model::new(24, 80);
        model.feed(b"\x1b[?2004h");
        for i in 0..40 {
            model.feed(format!("line {i}\r\n").as_bytes());
        }
        model.feed(b"\x1b[33m$ \x1b[0mtyped");
        model.forget();
        let s = model.parser.screen_mut();
        assert_eq!(s.cursor_position(), (0, 7));
        assert_eq!(s.contents().trim_end(), "$ typed");
        assert_eq!(s.cell(0, 0).unwrap().fgcolor(), vt100::Color::Idx(3), "the prompt keeps its colour");
        assert!(s.bracketed_paste(), "the modes stay");
        s.set_scrollback(usize::MAX);
        assert_eq!(s.scrollback(), 0, "the history is gone");
        let snap = model.snapshot(b"");
        assert!(!String::from_utf8_lossy(&snap).contains("line 39"), "a later attacher does not see it");
    }

    /// Writes the streams and their snapshots for the face's gate
    /// (`npm run snapshot` compares them through xterm). Run by hand:
    /// `cargo test -p khor-node --lib writes_snapshot_fixtures -- --ignored`.
    #[test]
    #[ignore]
    fn writes_snapshot_fixtures() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui/scripts/fixtures/snapshot");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, raw) in snapshot_streams() {
            let mut model = Model::new(24, 80);
            model.feed(&raw);
            let mut modes = Modes::default();
            modes.feed(&raw);
            std::fs::write(dir.join(format!("{name}.raw")), &raw).unwrap();
            std::fs::write(dir.join(format!("{name}.snap")), model.snapshot(&modes.preamble())).unwrap();
        }
    }

    /// The socket of the server a client argv speaks to (item 41), with
    /// the other global flags, valued or not, stepped over.
    #[test]
    fn a_tmux_argv_names_its_socket() {
        let argv = |s: &[&str]| s.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        assert_eq!(tmux_socket_args(&argv(&["tmux", "-L", "khor", "-T", "extkeys", "new-session", "-t", "$0"])), argv(&["-L", "khor"]));
        assert_eq!(tmux_socket_args(&argv(&["tmux", "-u", "-f", "/dev/null", "-S", "/tmp/s", "attach"])), argv(&["-S", "/tmp/s"]));
        assert!(tmux_socket_args(&argv(&["tmux", "attach", "-t", "-L"])).is_empty());
    }

    #[test]
    fn a_tmux_argv_marks_the_pty_as_a_multiplexer_client() {
        let argv = |s: &[&str]| s.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        assert!(pty_is_a_multiplexer_client(&argv(&["tmux", "attach"])));
        assert!(pty_is_a_multiplexer_client(&argv(&[
            "/opt/homebrew/bin/tmux",
            "-L",
            "sock"
        ])));
        assert!(!pty_is_a_multiplexer_client(&argv(&["zsh"])));
        assert!(!pty_is_a_multiplexer_client(&argv(&[
            "/bin/bash",
            "-lc",
            "tmux ls"
        ])));
        assert!(!pty_is_a_multiplexer_client(&[]));
    }

    /// All-control input previews as nothing, not as an empty string —
    /// `read_last` treats blank as absent for the same reason.
    #[test]
    fn a_tail_of_pure_control_bytes_is_no_preview() {
        assert_eq!(last_plain_line(b"\x1b[2J\x1b[H\r\r\n"), None);
    }
}

#[cfg(test)]
mod handshake_tests {
    use super::{ClientOp, HANDSHAKE, Hello, Welcome, connect_at, connect_versioned, read_frame, write_frame};

    /// **A versioned client against a host from before the handshake**:
    /// the host welcomes with an empty `why`, so the client speaks version
    /// 1 and its first frame is the person's input, never a `Version` the
    /// old host would choke on. The old host is played here by its own
    /// words: `Hello` in, `Welcome { ok, "" }` out, then only the two ops
    /// it knew.
    #[test]
    fn a_versioned_client_speaks_version_one_to_a_host_from_before() {
        #[derive(serde::Deserialize, Debug)]
        enum OldOp {
            Input(#[serde(with = "serde_bytes")] Vec<u8>),
            #[allow(dead_code)]
            Resize { cols: u16, rows: u16 },
        }
        let old = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = old.local_addr().unwrap().to_string();
        let host = std::thread::spawn(move || {
            let (mut c, _) = old.accept().unwrap();
            let hello: Hello = read_frame(&mut c).unwrap();
            write_frame(&mut c, &Welcome { ok: true, why: String::new() }).unwrap();
            let first: OldOp = read_frame(&mut c).expect("the old host can read what came first");
            (hello.cols, first)
        });
        let a = connect_versioned(&addr, "cookie", 80, 24).unwrap();
        assert_eq!(a.version, 1);
        let mut s = a.stream;
        write_frame(&mut s, &ClientOp::Input(b"hi".to_vec())).unwrap();
        let (cols, first) = host.join().unwrap();
        assert_eq!(cols, 80);
        match first {
            OldOp::Input(b) => assert_eq!(b, b"hi"),
            other => panic!("{other:?}"),
        }
    }

    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    /// **A host that never says welcome is given up on** (2026-10-02,
    /// the phone that froze). A far terminal's address is the resident's
    /// pipe, which accepts at once and only then dials the far machine;
    /// when that machine has dropped off, nothing came back and the
    /// attach waited with no end. Here the pipe is a listener that
    /// accepts and stays silent. The bound below is the test's, written
    /// as a number on purpose: on the code before the deadline this
    /// never returned, and the test says so instead of hanging.
    #[test]
    fn a_host_that_never_welcomes_is_given_up_on() {
        let silent = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = silent.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let held = silent.accept();
            std::thread::sleep(Duration::from_secs(120));
            drop(held);
        });
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let begun = Instant::now();
            let answer = connect_at(&addr, "cookie", 80, 24);
            let _ = tx.send((answer.map(|_| ()), begun.elapsed()));
        });
        let (answer, took) = rx
            .recv_timeout(Duration::from_secs(40))
            .expect("connect_at must come back from a host that never answers");
        // **And it is said in words** (item 36): the read ran into its
        // deadline, which macOS and iOS report as 「Resource temporarily
        // unavailable (os error 35)」 — what the owner was shown.
        assert_eq!(
            answer,
            Err(khor_catalog::msg::host_silent(HANDSHAKE.as_secs())),
            "a silent host is not a welcome, and the face is told why"
        );
        assert!(
            took >= Duration::from_secs(5),
            "it waited for the far dial, not just a moment ({took:?})"
        );
    }
}


#[cfg(test)]
mod ring_file_tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("khor-ringfile-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn shared(rows: u16, cols: u16) -> Shared {
        Shared {
            ring: Mutex::new(Ring::default()),
            clients: Mutex::new(Vec::new()),
            model: Mutex::new(Some(Model::new(rows, cols))),
            changes: AtomicU64::new(0),
        }
    }

    /// What the PTY reader does with a read: the ring, the model, the count.
    fn feed(s: &Shared, bytes: &[u8]) {
        let mut ring = s.ring.lock().unwrap();
        ring.push(bytes);
        s.model.lock().unwrap().as_mut().unwrap().feed(bytes);
        s.changes.fetch_add(1, Ordering::SeqCst);
    }

    /// The file reads back as it was written, a second write replaces the
    /// first whole, and no temporary is left beside it.
    #[test]
    fn a_ring_file_reads_back_whole_and_is_overwritten_whole() {
        let d = dir("whole");
        assert_eq!(read_ring_file(&d), None, "no file, no screen");
        let first = RingFile { form: RING_FORM_SNAPSHOT, cols: 80, rows: 24, at_ms: 1, bytes: b"one".repeat(1000) };
        write_ring_file(&d, &first).unwrap();
        assert_eq!(read_ring_file(&d).as_ref(), Some(&first));
        let second = RingFile { form: RING_FORM_REPLAY, cols: 100, rows: 30, at_ms: 2, bytes: b"two".to_vec() };
        write_ring_file(&d, &second).unwrap();
        assert_eq!(read_ring_file(&d), Some(second));
        assert!(!d.join("ring.bin.tmp").exists(), "the temporary is renamed, never left");
        std::fs::write(ring_file_path(&d), b"\x93garbled").unwrap();
        assert_eq!(read_ring_file(&d), None, "a garbled file is no screen, not a panic");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **The dead screen opens the way a reconnect would**: the saved
    /// bytes, fed to a fresh terminal of the saved size, give the model's
    /// screen cell for cell and its modes — for a shell transcript, a
    /// full-screen program on the alternate screen, and wide characters.
    #[test]
    fn the_saved_screen_is_the_screen_a_reconnect_gets() {
        for (name, stream) in [
            ("shell", (0..300).flat_map(|i| format!("\x1b[32m$\x1b[0m echo {i}\r\n{i}\r\n").into_bytes()).collect::<Vec<u8>>()),
            ("tui", b"\x1b[?1049h\x1b[?1000h\x1b[H\x1b[2J\x1b[3;5H\x1b[7m menu \x1b[0m\x1b[10;1Hstatus".to_vec()),
            ("wide", "\u{4f60}\u{597d} \x1b[1mbold\x1b[0m\r\n$ ".as_bytes().to_vec()),
        ] {
            let s = shared(24, 80);
            feed(&s, &stream);
            let file = s.ring_now().expect("a model, a file");
            assert_eq!((file.rows, file.cols), (24, 80), "{name}: the size it was laid out for");
            assert_eq!(file.form, RING_FORM_SNAPSHOT, "{name}");
            let mut again = vt100::Parser::new(file.rows, file.cols, 0);
            again.process(&file.bytes);
            let m = s.model.lock().unwrap();
            let a = m.as_ref().unwrap().parser.screen();
            let b = again.screen();
            assert_eq!(a.contents(), b.contents(), "{name}: the screen's text");
            assert_eq!(a.alternate_screen(), b.alternate_screen(), "{name}: the buffer");
            assert_eq!(a.cursor_position(), b.cursor_position(), "{name}: the cursor");
            assert_eq!(a.mouse_protocol_mode(), b.mouse_protocol_mode(), "{name}: the mouse");
        }
    }

    /// Written only when something was written since the last write: the
    /// 30-second beat costs a quiet shell nothing.
    #[test]
    fn the_ring_is_saved_only_when_it_changed() {
        let d = dir("changed");
        let s = shared(24, 80);
        let mut saved = 0u64;
        s.save_ring(&d, &mut saved);
        assert!(!ring_file_path(&d).exists(), "nothing written yet, nothing saved");
        feed(&s, b"hello\r\n");
        s.save_ring(&d, &mut saved);
        let first = read_ring_file(&d).expect("written once");
        assert!(String::from_utf8_lossy(&first.bytes).contains("hello"));
        std::fs::remove_file(ring_file_path(&d)).unwrap();
        s.save_ring(&d, &mut saved);
        assert!(!ring_file_path(&d).exists(), "no change since, no write");
        feed(&s, b"again\r\n");
        s.save_ring(&d, &mut saved);
        assert!(String::from_utf8_lossy(&read_ring_file(&d).unwrap().bytes).contains("again"));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A tmux client keeps no model, so no file: tmux holds that screen.
    #[test]
    fn a_tmux_client_saves_no_screen() {
        let s = Shared {
            ring: Mutex::new(Ring::default()),
            clients: Mutex::new(Vec::new()),
            model: Mutex::new(None),
            changes: AtomicU64::new(0),
        };
        s.ring.lock().unwrap().push(b"x");
        assert!(s.ring_now().is_none());
    }
}

#[cfg(test)]
mod command_root_tests {
    use super::*;
    #[test]
    fn host_store_comes_from_the_session_directory() {
        let root = std::env::temp_dir().join(format!("khor-host-root-{}", crate::link::fresh_hex().unwrap()));
        let dir = crate::live::sessions_dir(&root).join("example-session");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(host_store_root(&dir).unwrap(), std::fs::canonicalize(&root).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}
