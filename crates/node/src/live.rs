//! The live kind: sessions backed by a real local process — an agent TUI
//! reporting through hooks, or a command wrapped by `khor run` (临时: it
//! lives and dies with the terminal that started it; the persistent host
//! is the next batch). State is derived fresh on every ask and never
//! syncs (docs/NET.md: a stored copy is stale by definition); only this
//! device can derive these rows, so they travel as peer reports.
//!
//! Registry: one dir per session under `.khor/sessions/<kind>-<leaf>`,
//! holding `meta.json` (what it is) and `state.json` (how it is). Hooks
//! and the wrapper are separate processes writing the same files, so
//! every write is a whole file via tmp+rename: a race costs a moment of
//! staleness, never a torn read.
//!
//! Word discipline (docs/SESSION.md 六词映射): processes report only the
//! live words — busy, blocked, done, errored, idle. `failed` is never
//! reported; it is derived from the exit code (or from an ending nobody
//! recorded), so no hook can paint a live process as failed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use khor_catalog::msg;
use khor_core::{DeviceId, Kind, Millis, Session, SessionId, State, StateStamp};

use crate::adaptor::Discovery;

/// What a live session is. `pid` is the process whose death means the
/// session is over — None for observed sessions registered by hooks,
/// where no reliable pid exists (the hook's parent is a transient shell,
/// not the agent).
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Meta {
    pub kind: String,
    pub title: String,
    pub pid: Option<u32>,
    pub started_ms: i64,
    /// Whose session this is (`khor_core::Session::category`), when
    /// anyone can say. A registry entry written before this field existed
    /// reads as `None`, which is the same thing it means for a fresh one:
    /// nobody placed it.
    #[serde(default)]
    pub category: Option<String>,
}

/// How it is, as last written by its process (wrapper or hooks).
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LiveState {
    pub word: State,
    pub at_ms: i64,
    pub exit: Option<i32>,
}

pub fn sessions_dir(root: &Path) -> PathBuf {
    root.join(".khor").join("sessions")
}

/// A leaf survives only as lowercase `[a-z0-9-]`, capped — it lands in a
/// directory name, and hook-supplied ids are untrusted input.
pub fn clean_leaf(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => Some(c),
            'A'..='Z' => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .take(24)
        .collect();
    let cleaned = cleaned.trim_matches('-').to_string();
    if cleaned.is_empty() { "x".into() } else { cleaned }
}

#[derive(Clone)]
pub struct LiveKind {
    root: PathBuf,
    device: DeviceId,
    /// The vendors this device reads (crates/node/src/adaptor). Empty by
    /// default and opted into once, at [`crate::Node::open_as`]: a
    /// registry test that swept the machine's real `~/.claude` would be
    /// answering with whatever the user happens to be running.
    discovery: Arc<Discovery>,
    /// Live sessions the last sweep could not read. Kept from the sweep
    /// that produced the rows so the two never disagree.
    unmapped: Arc<AtomicUsize>,
}

impl LiveKind {
    pub fn new(root: PathBuf, device: DeviceId) -> LiveKind {
        LiveKind {
            root,
            device,
            discovery: Arc::new(Discovery::empty()),
            unmapped: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Also surface the agent sessions this machine's vendors are
    /// running, with no configuration from the user (docs/HOOKS.md).
    pub fn discovering(mut self, discovery: Arc<Discovery>) -> LiveKind {
        self.discovery = discovery;
        self
    }

    /// How many live sessions the last sweep saw but could not read —
    /// the "适配器过时" signal. Never inferred from an empty list: a
    /// vendor that changed its file layout produces zero rows and a
    /// non-zero count, and only the count tells them apart.
    pub fn unreadable_sessions(&self) -> usize {
        self.unmapped.load(Ordering::Relaxed)
    }

    /// `<kind>/<leaf>` → its registry dir. The one place the mapping
    /// lives; the slash swap keeps the registry flat.
    fn dir(&self, id: &SessionId) -> Option<PathBuf> {
        let (kind, leaf) = id.0.split_once('/')?;
        if leaf.is_empty() || kind.is_empty() {
            return None;
        }
        if clean_leaf(leaf) != leaf || clean_leaf(kind) != kind {
            return None;
        }
        Some(sessions_dir(&self.root).join(format!("{kind}-{leaf}")))
    }

    /// Whether this id names a session in the local registry.
    pub fn claims(&self, id: &SessionId) -> bool {
        self.dir(id).is_some_and(|d| d.join("meta.json").exists())
    }

    /// The registry dir a valid id would use (whether or not it exists).
    pub fn dir_of(&self, id: &SessionId) -> Option<PathBuf> {
        self.dir(id)
    }

    /// Registers a session. Refuses an id already taken — a hook
    /// re-registering must go through `ensure` instead.
    ///
    /// `category` is whose session it is, when the caller knows: a hook
    /// knows (it is that vendor's hook), `khor run` knows only that the
    /// user started a command. **Nobody guesses it from the command
    /// name** — an alias, a wrapper or `npx` would each make that answer
    /// wrong while looking right (see [`crate::adaptor::Found`]).
    pub fn register(
        &self,
        id: &SessionId,
        kind: &str,
        title: &str,
        pid: Option<u32>,
        category: Option<&str>,
    ) -> Result<(), String> {
        let dir = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        if dir.join("meta.json").exists() {
            return Err(msg::session_already_exists(&id.0));
        }
        fs::create_dir_all(&dir).map_err(msg::cant_make_session_dir)?;
        let meta = Meta {
            kind: kind.to_owned(),
            title: title.to_owned(),
            pid,
            started_ms: now_ms(),
            category: category.map(str::to_owned),
        };
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)?;
        self.write_state(&dir, State::Busy, None)
    }

    /// Registers if absent; either way the session exists afterwards.
    ///
    /// An existing session **learns its category here if it had none**:
    /// `khor run --tui -- <an agent>` registers a row nobody can place,
    /// and the first hook from that agent is the moment somebody can.
    /// A category already on record is never overwritten — the vendor
    /// that registered a row is the one that knows.
    pub fn ensure(
        &self,
        id: &SessionId,
        kind: &str,
        title: &str,
        pid: Option<u32>,
        category: Option<&str>,
    ) -> Result<(), String> {
        if self.claims(id) {
            return self.learn_category(id, category);
        }
        self.register(id, kind, title, pid, category)
    }

    /// Fills in a category the registry did not have. Never replaces one.
    pub fn learn_category(&self, id: &SessionId, category: Option<&str>) -> Result<(), String> {
        let Some(category) = category else {
            return Ok(());
        };
        let dir = self.existing_dir(id)?;
        let mut meta = read_meta(&dir)?;
        if meta.category.is_some() {
            return Ok(());
        }
        meta.category = Some(category.to_owned());
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)
    }

    /// Replaces the recorded pid — the wrapper registers before it knows
    /// the child's pid so hooks firing at startup find the session.
    pub fn set_pid(&self, id: &SessionId, pid: u32) -> Result<(), String> {
        let dir = self.existing_dir(id)?;
        let mut meta = read_meta(&dir)?;
        meta.pid = Some(pid);
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)
    }

    /// A process reporting how it is. Only live words pass: failed is
    /// exit-derived, and a session that already ended stays ended.
    pub fn report(&self, id: &SessionId, word: State) -> Result<(), String> {
        if word == State::Failed {
            return Err(msg::FAILED_IS_NOT_REPORTABLE.into());
        }
        let dir = self.existing_dir(id)?;
        let (meta, state) = read_pair(&dir)?;
        if state.exit.is_some() || !pid_says_alive(&meta) {
            return Err(msg::session_over(&id.0));
        }
        self.write_state(&dir, word, None)
    }

    /// The ending, recorded by whoever waited (the wrapper) or by an end
    /// hook. First recording wins; a session ends once.
    pub fn record_exit(&self, id: &SessionId, code: i32) -> Result<(), String> {
        let dir = self.existing_dir(id)?;
        let (_, state) = read_pair(&dir)?;
        if state.exit.is_some() {
            return Ok(());
        }
        self.write_state(&dir, state.word, Some(code))
    }

    /// The row's current stamp — what "looked at now" covers.
    pub fn stamp(&self, id: &SessionId) -> Result<i64, String> {
        let dir = self.existing_dir(id)?;
        Ok(read_pair(&dir)?.1.at_ms)
    }

    /// Kills the process if it still runs, then forgets the session.
    /// 失败沉底 is a list fact, not a registry fact — closing is how a
    /// settled row leaves the list.
    pub fn close_session(&self, id: &SessionId) -> Result<(), String> {
        let dir = self.existing_dir(id)?;
        if let Ok(hf) = crate::host::read_host_file(&dir) {
            // A hosted session dies by its child's process group; the
            // host sees the exit and leaves on its own. Anything on the
            // tty that dodged the group gets SIGHUP when the host drops
            // the PTY master. The host pid is the belt on top.
            #[cfg(unix)]
            unsafe {
                libc::kill(-(hf.child_pid as i32), libc::SIGTERM);
            }
            #[cfg(not(unix))]
            terminate(hf.child_pid);
            for _ in 0..20 {
                if !crate::link::pid_alive(hf.host_pid) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            if crate::link::pid_alive(hf.host_pid) {
                terminate(hf.host_pid);
            }
        } else {
            let (meta, state) = read_pair(&dir)?;
            if state.exit.is_none() && pid_says_alive(&meta) {
                if let Some(pid) = meta.pid {
                    terminate(pid);
                }
            }
        }
        fs::remove_dir_all(&dir).map_err(msg::cant_delete)
    }

    /// Every live session on this device as a row: the ones registered
    /// here, then the ones discovered by reading the vendors' own files.
    ///
    /// **The registry wins a collision, and that is the whole of "同源
    /// 去重".** A claude session that both reports through hooks and
    /// shows up on disk is one session; the registered row is the richer
    /// of the two (it has a word history, and can say 完成, which the
    /// status file cannot), so the discovered sighting steps aside. Both
    /// sides mint the id through `adaptor::id_for`, which is what makes
    /// the collision detectable at all.
    ///
    /// **The one exception is an ending.** A hook-registered observed
    /// session carries no pid — the hook's parent is a transient shell,
    /// not the agent — so its word stands only because nobody is in a
    /// position to pronounce it dead. The vendor's own file names its
    /// pid, so the sweep is exactly that position. Without this, a user
    /// who installs the hook would get *worse* crash detection than one
    /// who installs nothing: a hard-killed claude would park on its last
    /// word forever while the disk sat there knowing better.
    ///
    /// Unreadable entries are skipped: a half-written registry dir is a
    /// moment, not an error.
    ///
    /// **A multiplexer session is a row only if nothing inside it is one
    /// already** (`crate::adaptor::Multiplexed`). This is the same
    /// judgement as the merge above wearing a different key: there, two
    /// sources named one session with one id; here, one tmux session
    /// holds a claude that claude itself named, and the two ids have
    /// nothing in common — the only thing they share is a running
    /// process. So the test is on pids, and it is asked here rather than
    /// inside the sweep because **the registry's own pids count too**: a
    /// `khor run -- claude` started in a tmux pane is that pane's
    /// session, and khor already lists it.
    pub fn rows(&self, watermark: impl Fn(&str) -> i64) -> Vec<Session> {
        let registered = self.registry_rows(&watermark);
        let mut out: Vec<Session> = registered.iter().map(|(row, _)| row.clone()).collect();
        let unpinned: Vec<&SessionId> = registered
            .iter()
            .filter(|(_, pid)| pid.is_none())
            .map(|(row, _)| &row.id)
            .collect();
        // Everything on this list that is a running process. A pid on
        // record for a process that has already died costs nothing here:
        // what it is compared against is built from the live process
        // table, so a dead number can never match.
        let mut claimed: Vec<u32> = registered.iter().filter_map(|(_, pid)| *pid).collect();
        let sweep = self.discovery.sweep();
        self.unmapped.store(sweep.unmapped, Ordering::Relaxed);
        claimed.extend(sweep.rows.iter().flat_map(|f| f.sighting.pids.iter().copied()));
        for found in sweep.rows {
            let id = found.id();
            let kind = found.kind;
            let sighting = found.sighting;
            if let Some(seat) = out.iter().position(|r| r.id == id) {
                // The registered row wins, but it may have been registered
                // by something that could not name the vendor (a bare
                // `khor run --tui`). The sweep read that vendor's own
                // files, so it can — and an empty category is a gap to
                // fill, not an answer to respect.
                if out[seat].category.is_none() {
                    out[seat].category = Some(found.category.to_owned());
                }
                if sighting.word == State::Failed && unpinned.contains(&&id) {
                    out[seat].state =
                        StateStamp { state: State::Failed, at: Millis(sighting.at_ms.max(0) as u64) };
                    out[seat].unread = 0;
                }
                continue;
            }
            let (state, unread) =
                settle_done(sighting.word, sighting.at_ms, watermark(&id.0));
            out.push(Session {
                id,
                kind: Kind(kind.to_owned()),
                title: sighting.title,
                home: self.device,
                state: StateStamp {
                    state,
                    at: Millis(sighting.at_ms.max(0) as u64),
                },
                unread,
                category: Some(found.category.to_owned()),
            });
        }
        for held in sweep.multiplexed {
            if held.holds.iter().any(|p| claimed.contains(p)) {
                continue;
            }
            let id = held.found.id();
            if out.iter().any(|r| r.id == id) {
                continue;
            }
            let sighting = held.found.sighting;
            out.push(Session {
                id,
                kind: Kind(held.found.kind.to_owned()),
                title: sighting.title,
                home: self.device,
                state: StateStamp {
                    state: sighting.word,
                    at: Millis(sighting.at_ms.max(0) as u64),
                },
                // A multiplexer cannot say 完成, so there is nothing here
                // that could be unread (docs/UX.md 角标可归零).
                unread: 0,
                category: Some(held.found.category.to_owned()),
            });
        }
        out
    }

    /// Registry rows, each with the pid on record for it. Whether there
    /// is one decides whether anyone can pronounce the session dead;
    /// which one it is decides whether a multiplexer session is already
    /// on this list (see [`LiveKind::rows`]).
    fn registry_rows(&self, watermark: &impl Fn(&str) -> i64) -> Vec<(Session, Option<u32>)> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(sessions_dir(&self.root)) else {
            return out;
        };
        for e in rd.flatten() {
            let Ok((meta, state)) = read_pair(&e.path()) else {
                continue;
            };
            let Some(name) = e.file_name().to_str().map(String::from) else {
                continue;
            };
            let Some(leaf) = name.strip_prefix(&format!("{}-", meta.kind)) else {
                continue;
            };
            let id = SessionId(format!("{}/{leaf}", meta.kind));
            let (word, at, unread) = face(&meta, &state, watermark(&id.0));
            out.push((
                Session {
                    id,
                    kind: Kind(meta.kind.clone()),
                    title: meta.title.clone(),
                    home: self.device,
                    state: StateStamp { state: word, at: Millis(at.max(0) as u64) },
                    unread,
                    category: meta.category.clone(),
                },
                meta.pid,
            ));
        }
        out
    }

    fn existing_dir(&self, id: &SessionId) -> Result<PathBuf, String> {
        let dir = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        if !dir.join("meta.json").exists() {
            return Err(msg::no_such_session(&id.0));
        }
        Ok(dir)
    }

    fn write_state(&self, dir: &Path, word: State, exit: Option<i32>) -> Result<(), String> {
        let state = LiveState { word, at_ms: now_ms(), exit };
        write_whole(&dir.join("state.json"), &serde_json::to_vec(&state).map_err(|e| e.to_string())?)
    }
}

/// The six-word mapping (docs/SESSION.md), one place:
///
/// - an exit code decides first — shell lands on 完成/失败, a tui that
///   exited cleanly was looked at by definition (its turn ended on
///   screen) and lands on 空闲;
/// - a dead process with no recorded ending is 失败: the normal path
///   always records, so a missing ending is itself an abnormal ending;
/// - otherwise the reported word stands, with 完成 downgrading to 空闲
///   once the seen watermark covers it (看过了).
///
/// Only 完成/未看 counts as unread — 失败 sinks in the list but never
/// holds the badge (docs/UX.md 角标可归零).
fn face(meta: &Meta, state: &LiveState, watermark: i64) -> (State, i64, u64) {
    if let Some(code) = state.exit {
        if code != 0 {
            return (State::Failed, state.at_ms, 0);
        }
        if meta.kind == khor_core::kind::TUI {
            return (State::Idle, state.at_ms, 0);
        }
        let (word, unread) = settle_done(State::Done, state.at_ms, watermark);
        return (word, state.at_ms, unread);
    }
    if !pid_says_alive(meta) {
        return (State::Failed, state.at_ms, 0);
    }
    let (word, unread) = settle_done(state.word, state.at_ms, watermark);
    (word, state.at_ms, unread)
}

/// 完成 clears to 空闲 once the seen watermark covers it, and it is the
/// only word that carries unread (docs/UX.md 角标可归零). One place,
/// because both the registry and the discovery sweep need it and two
/// copies of a rule about badges is how a badge stops clearing.
fn settle_done(word: State, at_ms: i64, watermark: i64) -> (State, u64) {
    if word != State::Done {
        return (word, 0);
    }
    let unread = u64::from(at_ms > watermark);
    (if unread > 0 { State::Done } else { State::Idle }, unread)
}

/// No pid on record means nobody can pronounce it dead — the word stands
/// and ages visibly (its stamp is on the row). Observed sessions whose
/// agent is hard-killed park on their last word until closed; the pid
/// discovery that would fix this is on the ledger.
fn pid_says_alive(meta: &Meta) -> bool {
    match meta.pid {
        Some(pid) => crate::link::pid_alive(pid),
        None => true,
    }
}

fn read_meta(dir: &Path) -> Result<Meta, String> {
    let text = fs::read_to_string(dir.join("meta.json")).map_err(msg::cant_read_session_meta)?;
    serde_json::from_str(&text).map_err(msg::session_meta_garbled)
}

fn read_pair(dir: &Path) -> Result<(Meta, LiveState), String> {
    let meta = read_meta(dir)?;
    let text = fs::read_to_string(dir.join("state.json")).map_err(msg::cant_read_session_state)?;
    let state = serde_json::from_str(&text).map_err(msg::session_state_garbled)?;
    Ok((meta, state))
}

/// Whole-file write via tmp+rename: concurrent readers (serve deriving
/// rows) and writers (hooks) never meet a half-written file.
fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| msg::cant_write(path.display(), e))?;
    fs::rename(&tmp, path).map_err(|e| msg::cant_place(path.display(), e))
}

fn terminate(pid: u32) {
    // SIGTERM, not SIGKILL: the process gets to clean up. If it lingers,
    // the registry dir is already gone and the pid is printed nowhere —
    // acceptable for 临时 processes the user started themselves.
    //
    // `libc::kill` rather than the `kill` program, for the reason
    // `link::pid_alive` states: everything khor needs is compiled in, and
    // Windows has no `kill`. This is the single-pid door; the group
    // signal a few lines up in `close_session` was already libc.
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }
    #[cfg(not(unix))]
    {
        // Windows bring-up owns this (on the ledger).
        let _ = pid;
    }
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── the wrapper: khor run (临时) ────────────────────────────

/// Spawns `cmd` as a watched child and blocks until it ends. The session
/// is registered before the spawn so hooks firing at startup find it;
/// the child gets `KHOR_SESSION` so its hooks report into this session
/// instead of minting their own. Returns the child's exit code.
///
/// With a tty, the child takes the foreground (its own process group +
/// tcsetpgrp): Ctrl-C reaches it and not the wrapper, so the ending is
/// always recorded. Without a tty the child shares our group — a group
/// signal kills both, and the missing-ending rule reads that honestly
/// as 失败.
pub fn run_wrapped(live: &LiveKind, id: &SessionId, cmd: &[String]) -> Result<i32, String> {
    use std::process::Command;

    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]).env("KHOR_SESSION", &id.0);

    #[cfg(unix)]
    let on_tty = unsafe { libc::isatty(0) == 1 };
    #[cfg(not(unix))]
    let on_tty = false;

    #[cfg(unix)]
    if on_tty {
        use std::os::unix::process::CommandExt;
        c.process_group(0);
    }

    let mut child = c.spawn().map_err(|e| msg::wont_start(&cmd[0], e))?;
    live.set_pid(id, child.id())?;

    #[cfg(unix)]
    if on_tty {
        // Hand the terminal to the child; ignore SIGTTOU first or taking
        // it back from the background would stop us instead.
        unsafe {
            libc::signal(libc::SIGTTOU, libc::SIG_IGN);
            libc::signal(libc::SIGINT, libc::SIG_IGN);
            libc::signal(libc::SIGQUIT, libc::SIG_IGN);
            libc::tcsetpgrp(0, child.id() as i32);
        }
    }

    let status = child.wait().map_err(msg::cant_await_child)?;

    #[cfg(unix)]
    if on_tty {
        unsafe {
            libc::tcsetpgrp(0, libc::getpgrp());
        }
    }

    let code = exit_code_of(status);
    live.record_exit(id, code)?;
    Ok(code)
}

#[cfg(unix)]
fn exit_code_of(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    // Death by signal is a non-zero ending, spelled the shell way.
    status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
}

#[cfg(not(unix))]
fn exit_code_of(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("khor-live-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn kind_at(root: &Path) -> LiveKind {
        LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
    }

    fn sid(s: &str) -> SessionId {
        SessionId(s.to_owned())
    }

    /// A pid that is certainly dead: spawn `true`, reap it.
    fn dead_pid() -> u32 {
        let mut c = std::process::Command::new("true").spawn().unwrap();
        let pid = c.id();
        c.wait().unwrap();
        pid
    }

    /// The tui walk: reported words stand while the process lives, 完成
    /// clears to 空闲 through the watermark, a clean exit is 空闲, a
    /// non-zero exit is 失败 — and 失败 never holds the badge.
    #[test]
    fn a_tui_row_walks_the_reported_words_and_exit_decides_the_end() {
        let root = tmp("tui");
        let k = kind_at(&root);
        let id = sid("tui/abc123");
        k.register(&id, "tui", "khor", Some(std::process::id()), None).unwrap();

        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Busy, 0));

        k.report(&id, State::Blocked).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Blocked, 0), "waiting for approval");

        k.report(&id, State::Done).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Done, 1), "turn done, not looked at");
        let row = &k.rows(|_| i64::MAX)[0];
        assert_eq!((row.state.state, row.unread), (State::Idle, 0), "looked at");

        k.record_exit(&id, 0).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Idle, 0), "a clean tui exit is idle");

        let id2 = sid("tui/def456");
        k.register(&id2, "tui", "khor", Some(std::process::id()), None).unwrap();
        k.record_exit(&id2, 3).unwrap();
        let row = k.rows(|_| 0).into_iter().find(|r| r.id == id2).unwrap();
        assert_eq!((row.state.state, row.unread), (State::Failed, 0), "failed sinks, no badge");
        let _ = fs::remove_dir_all(&root);
    }

    /// Shell exit mapping differs from tui: a clean exit is 完成 until
    /// looked at (the tui showed its ending on screen; a wrapped command
    /// may have ended long after you left).
    #[test]
    fn a_clean_shell_exit_waits_to_be_looked_at() {
        let root = tmp("shell");
        let k = kind_at(&root);
        let id = sid("shell/run1");
        k.register(&id, "shell", "build", Some(std::process::id()), None).unwrap();
        k.record_exit(&id, 0).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Done, 1));
        let row = &k.rows(|_| i64::MAX)[0];
        assert_eq!((row.state.state, row.unread), (State::Idle, 0));
        let _ = fs::remove_dir_all(&root);
    }

    /// The missing-ending rule: a dead pid with no recorded exit is 失败
    /// — the normal path always records, so a missing ending is itself
    /// an abnormal one. A session with no pid on record is exempt: nobody
    /// can pronounce it dead.
    #[test]
    fn a_dead_process_with_no_recorded_ending_is_failed() {
        let root = tmp("dead");
        let k = kind_at(&root);
        let id = sid("shell/gone1");
        k.register(&id, "shell", "x", Some(dead_pid()), None).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.state.state, State::Failed);
        assert!(
            k.report(&id, State::Busy).unwrap_err() == msg::session_over(&id.0),
            "a dead session refuses reports by name"
        );

        let id2 = sid("tui/nopid1");
        k.register(&id2, "tui", "x", None, None).unwrap();
        k.report(&id2, State::Busy).unwrap();
        let row = k.rows(|_| 0).into_iter().find(|r| r.id == id2).unwrap();
        assert_eq!(row.state.state, State::Busy, "no pid on record = the word stands");
        let _ = fs::remove_dir_all(&root);
    }

    /// Nothing a hook sends may leave the registry, name a failed state,
    /// or resurrect an ended session.
    #[test]
    fn hostile_and_out_of_order_input_is_refused_by_name() {
        let root = tmp("hostile");
        let k = kind_at(&root);

        for bad in ["tui/../../etc", "tui/", "/leaf", "noslash", "tui/UPPER CASE!"] {
            assert!(
                k.register(&sid(bad), "tui", "x", None, None).is_err(),
                "{bad:?} should not register"
            );
        }
        assert_eq!(clean_leaf("../../etc/passwd"), "etcpasswd");
        assert_eq!(clean_leaf("A-B_C.9"), "a-bc9");

        let id = sid("tui/ok1");
        k.register(&id, "tui", "x", None, None).unwrap();
        assert_eq!(k.report(&id, State::Failed).unwrap_err(), msg::FAILED_IS_NOT_REPORTABLE);
        k.record_exit(&id, 0).unwrap();
        assert_eq!(k.report(&id, State::Busy).unwrap_err(), msg::session_over(&id.0));
        k.record_exit(&id, 7).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.state.state, State::Idle, "the first recorded ending wins");
        let _ = fs::remove_dir_all(&root);
    }

    /// 同源去重: one claude session that both reports through a hook and
    /// shows up in the vendor's own files is one row, and the registered
    /// side is the one that survives.
    ///
    /// The two sources are made to disagree on the word on purpose — the
    /// fixture session is `waiting` on a permission prompt (待批) while
    /// the hook has just been told the turn ended (完成). A merge that
    /// kept both, or kept the wrong one, cannot pass this.
    #[test]
    fn a_session_that_is_both_hooked_and_discovered_is_one_row() {
        use crate::adaptor::{Discovery, Proc, Procs};

        let root = tmp("dedup");
        // The fixture's first session, its pid vouched for by a process
        // table that exists only here.
        let vendors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc { name: "claude".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: None },
        )]);
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(procs));
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(discovery);

        // Discovery alone: one row, wearing the word from disk.
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.0, "tui/11111111-1111-4111-8111");
        assert_eq!(rows[0].state.state, State::Blocked);
        assert_eq!(rows[0].title, "one-aa");

        // Now the hook registers the same claude session and reports a
        // different word.
        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        k.register(&id, "tui", "one", Some(std::process::id()), None).unwrap();
        k.report(&id, State::Done).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "one session, one row — not two");
        assert_eq!(rows[0].id, id);
        assert_eq!(
            (rows[0].state.state, rows[0].unread),
            (State::Done, 1),
            "the registered row wins: it is the only one that can say 完成"
        );
        assert_eq!(rows[0].title, "one");
        let _ = fs::remove_dir_all(&root);
    }

    /// A row registered before anyone could place it learns its category
    /// from whoever can — and **never has one overwritten**.
    ///
    /// Both halves matter. Without the first, `khor run --tui -- claude`
    /// stays uncategorised forever even though claude is right there
    /// writing its own files. Without the second, the last writer wins
    /// and a session changes hands depending on sweep order.
    #[test]
    fn an_unplaced_row_learns_its_category_once_and_keeps_it() {
        let root = tmp("learn");
        let k = kind_at(&root);
        let id = sid("tui/unplaced1");
        k.register(&id, "tui", "some agent", None, None).unwrap();
        assert_eq!(k.rows(|_| 0)[0].category, None, "control: nobody placed it");

        k.learn_category(&id, Some("claude")).unwrap();
        assert_eq!(k.rows(|_| 0)[0].category.as_deref(), Some("claude"));

        k.learn_category(&id, Some("codex")).unwrap();
        assert_eq!(
            k.rows(|_| 0)[0].category.as_deref(),
            Some("claude"),
            "a category already on record is not up for grabs"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The disk sweep places a registered row that arrived unplaced.
    ///
    /// This is the same shape as the ending rule below: the registry wins
    /// the row, but the vendor's own files know something the registry
    /// cannot — there, that the process died; here, whose session it is.
    #[test]
    fn a_registered_row_gets_its_category_from_the_vendor_that_knows_it() {
        use crate::adaptor::{Discovery, Proc, Procs};

        let root = tmp("place-from-disk");
        let vendors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc { name: "claude".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: None },
        )]);
        let k = LiveKind::new(root.clone(), DeviceId([9; 32]))
            .discovering(Arc::new(Discovery::at(&vendors).with_procs(procs)));

        // The registry claims the id first, with nothing to say about
        // whose it is — what `khor run --tui` leaves behind.
        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        k.register(&id, "tui", "one", Some(std::process::id()), None).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "still one row");
        assert_eq!(
            rows[0].category.as_deref(),
            Some("claude"),
            "the sweep read claude's own files, so it can place what the registry could not"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// **One running claude in a tmux pane is one row, and it is
    /// claude's.**
    ///
    /// The two sources have nothing in common to merge on: claude's id
    /// comes from its own session uuid, tmux's from a server-local
    /// number, and neither has ever heard of the other. All they share
    /// is a process — so that is what is compared, through parentage,
    /// because the agent is a *grandchild* of the pane and any test on
    /// the pane pid alone would miss it.
    ///
    /// The control group is the same tmux session with nothing listed
    /// inside it: it becomes a row. Without that half, an implementation
    /// that simply never emitted tmux rows would pass.
    #[test]
    fn a_tmux_session_holding_a_listed_agent_is_not_a_second_row() {
        use crate::adaptor::{tmux::Tmux, Discovery, Proc, Procs};

        let root = tmp("tmux-dedup");
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let shell = |pid: u32, parent: u32| {
            (pid, Proc { name: "-zsh".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: Some(parent) })
        };
        // Pane 3001 has the fixture's claude (pid 4001) under it; pane
        // 3002 has nothing but its own shell.
        let procs = Procs::of([
            shell(3001, 1),
            shell(3002, 1),
            (
                4001,
                Proc {
                    name: "claude".into(),
                    started_ms: 1_700_000_000_000,
                    cwd: None,
                    ppid: Some(3001),
                },
            ),
        ]);
        let holding = crate::adaptor::tmux::fake_tmux(
            "holding",
            "$7|1786245338|1786245400|3001|0|zsh|with-claude\n",
        );
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs.clone())
                .with_tmux(Tmux::at_binary(&holding)),
        ));
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "one running process is one row");
        assert_eq!(rows[0].id.0, "tui/11111111-1111-4111-8111");
        assert_eq!(rows[0].category.as_deref(), Some("claude"), "and it is claude's, not a shell");

        // The control: the same session, holding nothing anyone listed.
        let empty = crate::adaptor::tmux::fake_tmux(
            "empty-pane",
            "$7|1786245338|1786245400|3002|0|zsh|just-a-shell\n",
        );
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&empty)),
        ));
        let mut rows = k.rows(|_| 0);
        rows.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        assert_eq!(rows.len(), 2, "a bare tmux session is a row of its own");
        assert_eq!(rows[0].id.0, "shell/1786245338-7");
        assert_eq!(rows[0].title, "just-a-shell");
        assert_eq!(rows[0].category.as_deref(), Some(khor_core::category::SHELL));
        assert_eq!(rows[0].state.state, State::Idle);
        let _ = fs::remove_dir_all(&root);
    }

    /// The same rule against the **registry**: `khor run` started inside
    /// a tmux pane is already a row, so the pane is not a second one.
    ///
    /// Separate from the sweep case above because the two claims arrive
    /// from different places — one from a vendor's files, one from
    /// khor's own directory — and an implementation that only consulted
    /// the sweep would pass the other test and fail here.
    #[test]
    fn a_tmux_session_holding_a_khor_session_is_not_a_second_row() {
        use crate::adaptor::{tmux::Tmux, Discovery, Proc, Procs};

        let root = tmp("tmux-registry");
        let k_root = root.clone();
        let me = std::process::id();
        let procs = Procs::of([
            (
                3001,
                Proc {
                    name: "-zsh".into(),
                    started_ms: 1_700_000_000_000,
                    cwd: None,
                    ppid: Some(1),
                },
            ),
            (
                me,
                Proc {
                    name: "khor".into(),
                    started_ms: 1_700_000_000_000,
                    cwd: None,
                    ppid: Some(3001),
                },
            ),
        ]);
        let fake = crate::adaptor::tmux::fake_tmux(
            "registry",
            "$7|1786245338|1786245400|3001|0|zsh|running-khor\n",
        );
        let k = LiveKind::new(k_root, DeviceId([9; 32])).discovering(Arc::new(
            Discovery::empty().with_procs(procs).with_tmux(Tmux::at_binary(&fake)),
        ));
        // The registry's row, its pid a child of the pane.
        let id = sid("shell/run1");
        k.register(&id, "shell", "build", Some(me), None).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "the command khor started is the row; the pane is not a second");
        assert_eq!(rows[0].id, id);
        let _ = fs::remove_dir_all(&root);
    }

    /// Installing the hook must not make crash detection worse.
    ///
    /// A hook-observed session has no pid, so its last word stands
    /// forever on its own. The vendor's file has the pid, so when the
    /// sweep says the process is gone it is the only party that knows —
    /// and 失败 has to reach the row even though the registry owns it.
    #[test]
    fn a_hooked_session_still_learns_from_disk_that_its_process_died() {
        use crate::adaptor::{Discovery, Procs};

        let root = tmp("crashed-hooked");
        let vendors = root.join("vendors");
        let sessions = vendors.join(".claude/sessions");
        fs::create_dir_all(&sessions).unwrap();
        let vendor_sid = "88888888-8888-4888-8888-888888888888";
        let at = now_ms() - 60_000;
        fs::write(
            sessions.join("8001.json"),
            serde_json::to_vec(&serde_json::json!({
                "pid": 8001, "sessionId": vendor_sid, "cwd": "/w/hooked",
                "startedAt": at - 1000, "name": "hooked-ee",
                "status": "busy", "statusUpdatedAt": at,
            }))
            .unwrap(),
        )
        .unwrap();

        // The hook's side: registered with no pid, last word 忙碌.
        let id = crate::adaptor::id_for(vendor_sid);
        // An empty process table is the whole point — pid 8001 is gone.
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(Procs::default()));
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(discovery);
        k.register(&id, "tui", "hooked", None, None).unwrap();
        k.report(&id, State::Busy).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "still one row");
        assert_eq!(
            rows[0].state.state,
            State::Failed,
            "the registry had no pid; the vendor file did, and it says the process is gone"
        );
        assert_eq!(rows[0].unread, 0, "失败 sinks and never holds the badge");

        // The control: with the process alive, the registry's own word
        // is what shows — so the 失败 above came from the ending rule and
        // not from the merge always preferring disk.
        let alive = Arc::new(
            Discovery::at(&vendors).with_procs(Procs::of([(
                8001,
                crate::adaptor::Proc { name: "claude".into(), started_ms: at - 1000, cwd: None, ppid: None },
            )])),
        );
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(alive);
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Busy);
        let _ = fs::remove_dir_all(&root);
    }
}
