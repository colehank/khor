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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use khor_catalog::msg;
use khor_core::{SessionId, State};

use crate::modes::Modes;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};

use crate::live::LiveKind;
use crate::proto;

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
}

/// Above any sane keystroke burst, far below anything hostile-sized.
const MAX_OP: usize = 1 << 20;
/// Replay budget: enough to hold the last full repaint of anything.
const RING: usize = 256 * 1024;

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
    let hf = read_view_file(dir)?;
    connect_at(&format!("127.0.0.1:{}", hf.port), &hf.cookie, cols, rows)
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
    s.write_all(&(bytes.len() as u32).to_be_bytes())
        .and_then(|()| s.write_all(&bytes))
        .map_err(msg::handoff_failed)
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
    host_main(
        root.to_path_buf(),
        SessionId(sid.clone()),
        size,
        cmd.to_vec(),
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
    spawn_host_with(dir, id, cmd, size, None, None)
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
    spawn_host_with(dir, id, cmd, size, None, Some(cwd))
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
    )
}

/// How `_host` learns it is a viewer: env rather than argv, so the argv
/// contract (`host_args`) stays what every existing binary parses.
const VIEWER_ENV: &str = "KHOR_HOST_VIEWER";

fn spawn_host_with(
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    (cols, rows): (u16, u16),
    viewer: Option<&str>,
    cwd: Option<&Path>,
) -> Result<(), String> {
    let exe = crate::self_exe()?;
    let mut c = std::process::Command::new(exe);
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
    let hf = read_host_file(dir)?;
    connect_at(&format!("127.0.0.1:{}", hf.port), &hf.cookie, cols, rows)
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
    let mut s = TcpStream::connect(addr).map_err(|_| msg::host_gone().to_string())?;
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
    let w: Welcome = read_frame(&mut s)?;
    if !w.ok {
        return Err(w.why);
    }
    let _ = s.set_read_timeout(None);
    let _ = s.set_write_timeout(None);
    Ok(s)
}

/// How long [`connect_at`] waits for a host to welcome it: a far pipe's
/// dial (≤ 10 s, `link::DIAL_TIMEOUT`, relay included) and the host's
/// answer, which is immediate. A host on this machine answers in
/// microseconds, so the deadline only ever bites a far one.
pub const HANDSHAKE: Duration = Duration::from_secs(15);

// ── the host process ────────────────────────────────────────

/// The replay: the last [`RING`] bytes, and the modes the bytes before
/// them had set (`crate::modes` — a tmux client's `ESC [ ? 1049 h` is
/// its first line and never repeated, 2026-10-01).
#[derive(Default)]
struct Ring {
    bytes: VecDeque<u8>,
    head: Modes,
}

impl Ring {
    fn push(&mut self, bytes: &[u8]) {
        self.bytes.extend(bytes);
        let over = self.bytes.len().saturating_sub(RING);
        if over > 0 {
            let gone: Vec<u8> = self.bytes.drain(..over).collect();
            self.head.feed(&gone);
        }
    }

    /// What an attacher is handed first: the trimmed head's modes, then
    /// the bytes still held.
    fn replay(&self) -> Vec<u8> {
        let mut out = self.head.preamble();
        out.extend(self.bytes.iter());
        out
    }
}

struct Shared {
    /// Lock order everywhere: ring, then clients — an attacher must
    /// snapshot and join atomically or it double-receives the bytes the
    /// reader is broadcasting.
    ring: Mutex<Ring>,
    clients: Mutex<Vec<TcpStream>>,
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
    let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
        .map_err(|e| e.to_string())?;
    let live = LiveKind::new(root, khor_core::DeviceId(*key.public().as_bytes()));
    let dir = live
        .dir_of(&id)
        .ok_or_else(|| msg::not_a_session_id(&id.0))?;
    // **The host's own words, left where the opener will look.** The
    // host is spawned detached with all three streams on /dev/null, so
    // until now every reason it had for dying went nowhere and the
    // opener had one sentence for all of them — a timeout
    // (`HOST_NEVER_READY`). See `host_error_path` for what that cost.
    let served = host_serving(&live, &dir, &id, size, &cmd);
    if let Err(why) = &served {
        let _ = std::fs::write(host_error_path(&dir), why);
    }
    served
}

fn host_serving(
    live: &LiveKind,
    dir: &Path,
    id: &SessionId,
    size: (u16, u16),
    cmd: &[String],
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
    let found = crate::find_program(&cmd[0], &path)
        .ok_or_else(|| msg::command_not_found(&cmd[0], &path))?;
    let mut builder = CommandBuilder::new(&found);
    builder.args(&cmd[1..]);
    builder.env("PATH", &path);
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
    // **The host is the terminal, so the host says what it is.**
    // Inheriting `TERM` names whatever shell happened to start khor,
    // and for a 持久 session that is not even a coherent question: the
    // host outlives its opener and several people with different
    // terminals attach to the same screen. tmux settles this the same
    // way — a pane gets a fixed `TERM`, never the client's.
    //
    // Measured on turing, where it is not cosmetic: `khor serve` was
    // started by the installer with no tty, so `TERM` was `unknown`,
    // and every tmux bridge stood up there died at birth with
    // `open terminal failed: missing or unsuitable terminal: unknown`.
    // What reached the person clicking the row was 读不到帧:
    // Connection reset — a sentence about a socket, for a terminfo
    // entry. **Any machine that installed itself and runs serve as a
    // daemon had this**, which is every machine the install story is
    // written for.
    //
    // `xterm-256color` because that is what khor's own screen model
    // (`vt100`, the same engine the detector reads) actually emulates,
    // and it is the entry ncurses has shipped everywhere for twenty
    // years.
    builder.env("TERM", "xterm-256color");
    // A hosted session is nobody's nested session, whatever process
    // tree the host was spawned from. The markers are claude's, but
    // clearing them costs a shell nothing — and inheriting them costs a
    // claude its transcript (`CLAUDE_CODE_CHILD_SESSION` turns saving
    // off; found live through a dev bridge started inside a claude).
    for marker in [
        "CLAUDECODE",
        "CLAUDE_CODE_ENTRYPOINT",
        "CLAUDE_CODE_CHILD_SESSION",
    ] {
        builder.env_remove(marker);
    }
    // **And the terminal's own name is khor's** (T-26, the terminal
    // audit: a hosted shell got `TERM_PROGRAM=tmux`, `TERM_SESSION_ID`,
    // `LC_TERMINAL=iTerm2` and `TERM_FEATURES` from whatever launched
    // khor, and no `COLORTERM` under launchd). Programs branch on these —
    // Claude Code and neovim on `TERM_PROGRAM` — and the terminal they
    // talk to is khor's xterm, not the opener's. `COLORTERM=truecolor`
    // is what xterm.js draws (its 24-bit SGR), as iTerm2, kitty and
    // WezTerm set it.
    builder.env("TERM_PROGRAM", "khor");
    builder.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    builder.env("COLORTERM", "truecolor");
    for (name, _) in std::env::vars_os() {
        let Some(name) = name.to_str() else { continue };
        // `TMUX` and `TMUX_PANE` say "you are inside that tmux"; the
        // person's own `TMUX_TMPDIR` says where their tmux lives, and
        // stays.
        if matches!(name, "TERM_SESSION_ID" | "TERM_FEATURES" | "TMUX" | "TMUX_PANE")
            || name.starts_with("LC_TERMINAL")
            || name.starts_with("ITERM_")
        {
            builder.env_remove(name);
        }
    }
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
    let writer = Arc::new(Mutex::new(
        master
            .lock()
            .unwrap()
            .take_writer()
            .map_err(msg::cant_write_pty)?,
    ));

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
                    let _ = w.write_all(&own).and_then(|()| w.flush());
                }
            }
        });
    }

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
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            // The preview line (`last.txt`): derived after every read,
            // written only when it changed. Not throttled — a throttle
            // here has no trailing edge, and the one update that matters
            // most is the one after the *last* burst of output, exactly
            // the one a leading-edge throttle drops (caught live by the
            // smoke: the typed marker never previewed). The derive is a
            // 4 KiB strip or one screen read per 8 KiB of output —
            // cheap; the change gate keeps the disk out of the loop.
            let mut last_written: Option<String> = None;
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                // Before the ring, and in its own scope: the screen lock
                // is never held while holding either of the other two,
                // so it cannot join their order (see Shared).
                if let Some(screen) = &screen {
                    screen.lock().unwrap().feed(&buf[..n]);
                }
                // Before anyone can see it: a question is pending before
                // a face can answer it.
                ledger.lock().unwrap().saw_output(&buf[..n], std::time::Instant::now());
                let mut ring = shared.ring.lock().unwrap();
                ring.push(&buf[..n]);
                let mut clients = shared.clients.lock().unwrap();
                clients.retain_mut(|c| c.write_all(&buf[..n]).is_ok());
                drop(clients);
                drop(ring);
                // Never for a tmux client (`mux_client`): its bottom
                // line is tmux's status bar, which is true of the
                // screen and a lie about the session.
                if !mux_client {
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
            for c in shared.clients.lock().unwrap().drain(..) {
                let _ = c.shutdown(std::net::Shutdown::Both);
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

    // Attachers.
    {
        let shared = shared.clone();
        let ledger = ledger.clone();
        let master = master.clone();
        let writer = writer.clone();
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
                        why: String::new(),
                    },
                )
                .is_err()
                {
                    continue;
                }
                {
                    let ring = shared.ring.lock().unwrap();
                    if conn.write_all(&ring.replay()).is_err() {
                        continue;
                    }
                    let Ok(readable) = conn.try_clone() else {
                        continue;
                    };
                    shared.clients.lock().unwrap().push(conn);
                    drop(ring);
                    let master = master.clone();
                    let writer = writer.clone();
                    let ledger = ledger.clone();
                    std::thread::spawn(move || {
                        let mut readable = readable;
                        while let Ok(op) = read_client_frame::<ClientOp>(&mut readable) {
                            let Some(op) = op else { continue };
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
                                }
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
        last_plain_line, looks_like_a_tmux_status_line, pty_is_a_multiplexer_client, read_last,
    };

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
    use super::connect_at;
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
        assert!(
            answer.is_err(),
            "a silent host is not a welcome: {answer:?}"
        );
        assert!(
            took >= Duration::from_secs(5),
            "it waited for the far dial, not just a moment ({took:?})"
        );
    }
}
