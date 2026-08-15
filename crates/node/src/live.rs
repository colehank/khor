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

use khor_catalog::msg;
use khor_core::{DeviceId, Kind, Millis, Session, SessionId, State, StateStamp};

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
}

impl LiveKind {
    pub fn new(root: PathBuf, device: DeviceId) -> LiveKind {
        LiveKind { root, device }
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
    pub fn register(
        &self,
        id: &SessionId,
        kind: &str,
        title: &str,
        pid: Option<u32>,
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
        };
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)?;
        self.write_state(&dir, State::Busy, None)
    }

    /// Registers if absent; either way the session exists afterwards.
    pub fn ensure(&self, id: &SessionId, kind: &str, title: &str, pid: Option<u32>) -> Result<(), String> {
        if self.claims(id) {
            return Ok(());
        }
        self.register(id, kind, title, pid)
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

    /// Every registered session as a row. Unreadable entries are skipped:
    /// a half-written registry dir is a moment, not an error.
    pub fn rows(&self, watermark: impl Fn(&str) -> i64) -> Vec<Session> {
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
            out.push(Session {
                id,
                kind: Kind(meta.kind.clone()),
                title: meta.title.clone(),
                home: self.device,
                state: StateStamp { state: word, at: Millis(at.max(0) as u64) },
                unread,
            });
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
        let unread = u64::from(state.at_ms > watermark);
        let word = if unread > 0 { State::Done } else { State::Idle };
        return (word, state.at_ms, unread);
    }
    if !pid_says_alive(meta) {
        return (State::Failed, state.at_ms, 0);
    }
    if state.word == State::Done {
        let unread = u64::from(state.at_ms > watermark);
        let word = if unread > 0 { State::Done } else { State::Idle };
        return (word, state.at_ms, unread);
    }
    (state.word, state.at_ms, 0)
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
    let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
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

// ── Claude Code hook glue ───────────────────────────────────

/// What `khor state --hook` did, for the caller to print (or not).
pub enum Hooked {
    Updated(SessionId, State),
    Ended(SessionId),
    /// The event carries no state change (a passing notification, an
    /// event this glue does not map).
    Ignored,
}

/// Reads one Claude Code hook payload and moves the session it belongs
/// to. Claude-version-specific glue, quarantined here: the mapping from
/// hook events to the six words is this function and nothing else.
///
/// The session: `KHOR_SESSION` when the agent runs under `khor run`
/// (one session, reliable pid, real exit code); otherwise an observed
/// `tui/<claude session id>` is registered on first sight, with no pid.
///
/// Word mapping — the one deliberate narrowing is Notification: only a
/// permission ask becomes 待批. Claude also notifies on long idle, and
/// "等你说下一句" is exactly what 待批 must never mean (docs/SESSION.md).
pub fn claude_hook(live: &LiveKind, payload: &str) -> Result<Hooked, String> {
    let v: serde_json::Value =
        serde_json::from_str(payload).map_err(msg::hook_payload_garbled)?;
    let event = v["hook_event_name"].as_str().unwrap_or("");

    let id = match std::env::var("KHOR_SESSION") {
        Ok(sid) if live.claims(&SessionId(sid.clone())) => SessionId(sid),
        _ => {
            let raw = v["session_id"].as_str().unwrap_or("");
            if raw.is_empty() {
                return Err(msg::HOOK_PAYLOAD_NO_SESSION.into());
            }
            let title = v["cwd"]
                .as_str()
                .and_then(|c| Path::new(c).file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "claude".into());
            let id = SessionId(format!("{}/{}", khor_core::kind::TUI, clean_leaf(raw)));
            live.ensure(&id, khor_core::kind::TUI, &title, None)?;
            id
        }
    };

    let word = match event {
        "SessionStart" => Some(State::Idle),
        "UserPromptSubmit" => Some(State::Busy),
        "Stop" => Some(State::Done),
        "Notification" => {
            let msg = v["message"].as_str().unwrap_or("");
            if msg.contains("permission") { Some(State::Blocked) } else { None }
        }
        "SessionEnd" => {
            live.record_exit(&id, 0)?;
            return Ok(Hooked::Ended(id));
        }
        _ => None,
    };
    match word {
        Some(w) => {
            live.report(&id, w)?;
            Ok(Hooked::Updated(id, w))
        }
        None => Ok(Hooked::Ignored),
    }
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
        k.register(&id, "tui", "khor", Some(std::process::id())).unwrap();

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
        k.register(&id2, "tui", "khor", Some(std::process::id())).unwrap();
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
        k.register(&id, "shell", "build", Some(std::process::id())).unwrap();
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
        k.register(&id, "shell", "x", Some(dead_pid())).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.state.state, State::Failed);
        assert!(
            k.report(&id, State::Busy).unwrap_err() == msg::session_over(&id.0),
            "a dead session refuses reports by name"
        );

        let id2 = sid("tui/nopid1");
        k.register(&id2, "tui", "x", None).unwrap();
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
                k.register(&sid(bad), "tui", "x", None).is_err(),
                "{bad:?} should not register"
            );
        }
        assert_eq!(clean_leaf("../../etc/passwd"), "etcpasswd");
        assert_eq!(clean_leaf("A-B_C.9"), "a-bc9");

        let id = sid("tui/ok1");
        k.register(&id, "tui", "x", None).unwrap();
        assert_eq!(k.report(&id, State::Failed).unwrap_err(), msg::FAILED_IS_NOT_REPORTABLE);
        k.record_exit(&id, 0).unwrap();
        assert_eq!(k.report(&id, State::Busy).unwrap_err(), msg::session_over(&id.0));
        k.record_exit(&id, 7).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.state.state, State::Idle, "the first recorded ending wins");
        let _ = fs::remove_dir_all(&root);
    }

    /// The Claude glue end to end on payloads: an observed session
    /// registers itself on first sight; the idle-notification does NOT
    /// become 待批 (a badge that cannot reach zero is no badge); a
    /// permission ask does; SessionEnd settles it.
    #[test]
    fn claude_payloads_move_an_observed_session() {
        let root = tmp("claude");
        let k = kind_at(&root);
        let payload = |event: &str, extra: &str| {
            format!(
                r#"{{"session_id":"55E-fake.uuid","cwd":"/home/u/proj","hook_event_name":"{event}"{extra}}}"#
            )
        };

        claude_hook(&k, &payload("SessionStart", "")).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.id.0, "tui/55e-fakeuuid");
        assert_eq!(row.title, "proj");
        assert_eq!(row.state.state, State::Idle, "started, waiting for the first prompt");

        claude_hook(&k, &payload("UserPromptSubmit", "")).unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Busy);

        let idle_note = payload("Notification", r#","message":"Claude is waiting for your input""#);
        assert!(matches!(claude_hook(&k, &idle_note).unwrap(), Hooked::Ignored));
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Busy, "long-idle must not become 待批");

        let ask = payload("Notification", r#","message":"Claude needs your permission to use Bash""#);
        claude_hook(&k, &ask).unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Blocked);

        claude_hook(&k, &payload("Stop", "")).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Done, 1));

        claude_hook(&k, &payload("SessionEnd", "")).unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Idle, "a clean end is idle");
        let _ = fs::remove_dir_all(&root);
    }
}
