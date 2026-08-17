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
//! for the GUI batch.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use khor_catalog::msg;
use khor_core::{SessionId, State};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
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

pub fn read_host_file(dir: &Path) -> Result<HostFile, String> {
    let text = std::fs::read_to_string(host_file_path(dir))
        .map_err(|_| msg::NO_HOST.to_string())?;
    serde_json::from_str(&text).map_err(msg::host_file_garbled)
}

pub fn write_frame<T: Serialize>(s: &mut (impl Write + ?Sized), t: &T) -> Result<(), String> {
    let bytes = proto::encode(t)?;
    s.write_all(&(bytes.len() as u32).to_be_bytes())
        .and_then(|()| s.write_all(&bytes))
        .map_err(msg::handoff_failed)
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

// ── the opener's side ───────────────────────────────────────

/// Spawns the detached host for an already-registered session and waits
/// for it to be reachable. The host is its own session (setsid): the
/// terminal that runs `open` can die without taking it along.
pub fn spawn_host(
    dir: &Path,
    id: &SessionId,
    cmd: &[String],
    (cols, rows): (u16, u16),
) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(msg::cant_find_self)?;
    let mut c = std::process::Command::new(exe);
    c.arg("_host")
        .arg(&id.0)
        .arg(cols.to_string())
        .arg(rows.to_string())
        .arg("--")
        .args(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    c.spawn().map_err(msg::host_wont_start)?;
    let marker = host_file_path(dir);
    for _ in 0..100 {
        if marker.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(msg::HOST_NEVER_READY.into())
}

/// Connects and completes the handshake; on return the stream carries
/// the replay and then live PTY bytes. Client ops go back framed.
pub fn connect(dir: &Path, cols: u16, rows: u16) -> Result<TcpStream, String> {
    let hf = read_host_file(dir)?;
    let mut s = TcpStream::connect(("127.0.0.1", hf.port))
        .map_err(|_| msg::HOST_GONE.to_string())?;
    write_frame(&mut s, &Hello { cookie: hf.cookie, cols, rows })?;
    let w: Welcome = read_frame(&mut s)?;
    if !w.ok {
        return Err(w.why);
    }
    Ok(s)
}

// ── the host process ────────────────────────────────────────

struct Shared {
    /// Lock order everywhere: ring, then clients — an attacher must
    /// snapshot and join atomically or it double-receives the bytes the
    /// reader is broadcasting.
    ring: Mutex<VecDeque<u8>>,
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
pub fn host_main(root: PathBuf, id: SessionId, size: (u16, u16), cmd: Vec<String>) -> Result<(), String> {
    let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
        .map_err(|e| e.to_string())?;
    let live = LiveKind::new(root, khor_core::DeviceId(*key.public().as_bytes()));
    let dir = live.dir_of(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
    let kind = id.0.split('/').next().unwrap_or("").to_owned();
    live.set_pid(&id, std::process::id())?;

    let pty = native_pty_system()
        .openpty(PtySize { rows: size.1, cols: size.0, pixel_width: 0, pixel_height: 0 })
        .map_err(msg::cant_open_pty)?;
    let mut builder = CommandBuilder::new(&cmd[0]);
    builder.args(&cmd[1..]);
    builder.env("KHOR_SESSION", &id.0);
    if let Ok(cwd) = std::env::current_dir() {
        builder.cwd(cwd);
    }
    let mut child = pty
        .slave
        .spawn_command(builder)
        .map_err(|e| msg::wont_start(&cmd[0], e))?;
    drop(pty.slave);
    let child_pid = child.process_id().ok_or(msg::CHILD_HAS_NO_PID)?;
    let master = Arc::new(Mutex::new(pty.master));
    let mut reader = master
        .lock()
        .unwrap()
        .try_clone_reader()
        .map_err(msg::cant_read_pty)?;
    let writer = Arc::new(Mutex::new(
        master.lock().unwrap().take_writer().map_err(msg::cant_write_pty)?,
    ));

    let listener =
        TcpListener::bind(("127.0.0.1", 0)).map_err(msg::cant_listen_loopback)?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let cookie = crate::link::fresh_hex()?;
    crate::link::write_private(
        &host_file_path(&dir),
        &serde_json::to_vec(&HostFile {
            port,
            cookie: cookie.clone(),
            host_pid: std::process::id(),
            child_pid,
        })
        .map_err(|e| e.to_string())?,
    )?;

    let shared = Arc::new(Shared {
        ring: Mutex::new(VecDeque::new()),
        clients: Mutex::new(Vec::new()),
    });
    let output_done = Arc::new(AtomicBool::new(false));

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
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
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
                let mut ring = shared.ring.lock().unwrap();
                ring.extend(&buf[..n]);
                while ring.len() > RING {
                    ring.pop_front();
                }
                let mut clients = shared.clients.lock().unwrap();
                clients.retain_mut(|c| c.write_all(&buf[..n]).is_ok());
            }
            output_done.store(true, Ordering::SeqCst);
            for c in shared.clients.lock().unwrap().drain(..) {
                let _ = c.shutdown(std::net::Shutdown::Both);
            }
        });
    }

    // The six-word face for a shell: 忙碌 = the terminal's foreground
    // group is not the shell itself.
    if kind == khor_core::kind::SHELL {
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
                if word != last && live.report(&id, word, crate::live::Source::Reported).is_ok() {
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
                    if word != last
                        && live.report(&id, word, crate::live::Source::Screen).is_ok()
                    {
                        last = word;
                    }
                }
            });
        }
    }

    // Attachers.
    {
        let shared = shared.clone();
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
                    let _ = write_frame(&mut conn, &Welcome { ok: false, why: msg::WRONG_COOKIE.into() });
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
                if write_frame(&mut conn, &Welcome { ok: true, why: String::new() }).is_err() {
                    continue;
                }
                {
                    let ring = shared.ring.lock().unwrap();
                    let (a, b) = ring.as_slices();
                    if conn.write_all(a).and_then(|()| conn.write_all(b)).is_err() {
                        continue;
                    }
                    let Ok(readable) = conn.try_clone() else { continue };
                    shared.clients.lock().unwrap().push(conn);
                    drop(ring);
                    let master = master.clone();
                    let writer = writer.clone();
                    std::thread::spawn(move || {
                        let mut readable = readable;
                        while let Ok(op) = read_frame::<ClientOp>(&mut readable) {
                            match op {
                                ClientOp::Input(bytes) => {
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
    // writes because `close` may have removed the dir already.
    let status = child.wait().map_err(msg::host_child_wait)?;
    let _ = live.record_exit(&id, status.exit_code() as i32);
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

    /// A path is fine; the basename is what is compared.
    #[test]
    fn a_vendors_own_name_picks_its_table() {
        assert_eq!(table_for("claude"), Some("claude"));
        assert_eq!(table_for("/Users/someone/.local/bin/claude"), Some("claude"));
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
            "npx",             // the vendor is an argument, not the command
            "my-claude",       // someone's wrapper
            "claude-wrapper",
            "claude.sh",       // a script around it
            "Claude",          // the table is lower case and so are the names
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
