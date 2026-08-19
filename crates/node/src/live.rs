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
    /// The session's full id. Listing used to rebuild the id from the
    /// directory name by stripping `kind` off the front — which was
    /// only ever true by coincidence: every early kind happened to spell
    /// its ids `kind/leaf`. A GUI session is registered under the
    /// vendor-session agreement (`tui/<uuid>`, `gui_host` module head)
    /// with `kind: gui`, and the strip silently dropped the row from
    /// every list while `dir_of` kept finding it — a session that works
    /// and does not exist. Empty means an entry from before this field,
    /// and for all of those the old rebuild is right.
    #[serde(default)]
    pub id: String,
    /// The vendor's own session leaf (already `clean_leaf`ed), when a
    /// hook told us. A khor-opened agent session carries khor's minted
    /// leaf in its id, but the vendor names its transcript after its
    /// *own* session id — this is the bridge that lets `transcript_of`
    /// find that file. `None` until a hook speaks; write-once like
    /// `category`, and for the same reason.
    #[serde(default)]
    pub vendor_leaf: Option<String>,
}

/// How a word got onto a registry row.
///
/// Only exists to answer one question, and the question is asked in
/// exactly one place ([`LiveKind::rows`]): **when this row and the
/// vendor's own files disagree, which one is right?**
///
/// Until 2026-08-17 the answer was always "the registry", and the reason
/// written down for it was that a registered row is the *richer* of the
/// two — it carries a word history and it is the only side that can say
/// 完成. That reason is true of a hook and false of a screen reading, so
/// the rule could not simply be extended to the PTY fallback: it would
/// have let the family that guesses overrule the family that knows.
/// Hence a word carries where it came from, and the comparison reads
/// that instead of assuming the channel.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// A hook, or the host watching its own child's process group.
    /// Somebody in a position to know said so outright; nothing was
    /// inferred from appearances.
    Reported,
    /// Patterns matched against the screen the agent draws
    /// (`khor_detect`). The bottom of the four-family order — it is a
    /// guess, and it is meant to lose to anything else.
    Screen,
}

impl Source {
    /// Does a word from here stand against what the vendor's own files
    /// say?
    ///
    /// Note what is *not* modelled: the vendor's files are not a
    /// `Source`, because a sweep's sighting is never written to a
    /// registry entry — it is the thing an entry gets compared against.
    /// Giving the enum a `Disk` variant would add a state that can be
    /// spelled and never occur.
    fn outranks_disk(self) -> bool {
        match self {
            Source::Reported => true,
            Source::Screen => false,
        }
    }
}

/// How it is, as last written by its process (wrapper or hooks).
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LiveState {
    pub word: State,
    pub at_ms: i64,
    pub exit: Option<i32>,
    /// Absent on every entry written before provenance existed.
    ///
    /// **Absent counts as [`Source::Reported`]**, the strongest, and the
    /// reasoning is that there is nothing else it could honestly be:
    /// every word already on disk was put there by a hook or by the
    /// wrapper, since the screen reader did not exist when they were
    /// written. Defaulting the other way would be the real hazard — it
    /// would let a sweep start overruling existing rows the moment this
    /// field shipped, which is the one thing this change must not do.
    /// The lean lasts only until that row's next real write.
    #[serde(default)]
    pub source: Option<Source>,
}

impl LiveState {
    /// See [`LiveState::source`] for why absent is the strong answer.
    fn word_outranks_disk(&self) -> bool {
        self.source.is_none_or(Source::outranks_disk)
    }
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
            id: id.0.clone(),
            vendor_leaf: None,
        };
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)?;
        // Whoever registers a session is starting it, so this opening
        // 忙碌 is a fact about a spawn and not a reading of anything.
        self.write_state(&dir, State::Busy, None, Source::Reported)
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

    /// Re-seats a row on a new presentation channel — the takeover
    /// (批C): the same conversation, a different process showing it.
    /// Creates the entry when the row was only ever discovered; clears
    /// any recorded ending (the old process was killed *by* the
    /// takeover, and that ending is the switch, not the session's);
    /// the fresh word is 空闲 — a prompt nobody has typed into yet.
    pub fn reopen(
        &self,
        id: &SessionId,
        kind: &str,
        title: &str,
        pid: u32,
    ) -> Result<(), String> {
        self.reseat(id, kind, title, Some(pid))
    }

    /// [`Self::reopen`] without claiming the pid — for the re-seating
    /// where the **new process registers itself** (a terminal host calls
    /// `set_pid` on the way up). Passing a pid here anyway would either
    /// be a lie (the caller's own) or a race (the host's, read before it
    /// exists).
    pub fn reseat(
        &self,
        id: &SessionId,
        kind: &str,
        title: &str,
        pid: Option<u32>,
    ) -> Result<(), String> {
        let dir = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        fs::create_dir_all(&dir).map_err(msg::cant_make_session_dir)?;
        let mut meta = read_meta(&dir).unwrap_or(Meta {
            kind: kind.to_owned(),
            title: title.to_owned(),
            pid: None,
            started_ms: now_ms(),
            category: None,
            id: id.0.clone(),
            vendor_leaf: None,
        });
        meta.kind = kind.to_owned();
        meta.title = title.to_owned();
        if let Some(pid) = pid {
            meta.pid = Some(pid);
        }
        meta.id = id.0.clone();
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)?;
        self.write_state(&dir, State::Idle, None, Source::Reported)
    }

    /// Fills in the vendor's session leaf. Write-once, like
    /// `learn_category`: the first hook knows, and a later different
    /// answer would mean the vendor restarted a new session inside the
    /// same terminal — at which point the *newest* transcript under the
    /// first leaf is still the conversation this row hosted.
    pub fn learn_vendor_leaf(&self, id: &SessionId, leaf: &str) -> Result<(), String> {
        if leaf.is_empty() {
            return Ok(());
        }
        let dir = self.existing_dir(id)?;
        let mut meta = read_meta(&dir)?;
        if meta.vendor_leaf.is_some() {
            return Ok(());
        }
        meta.vendor_leaf = Some(leaf.to_owned());
        write_whole(&dir.join("meta.json"), &serde_json::to_vec(&meta).map_err(|e| e.to_string())?)
    }

    /// The vendor leaf on record for a session, if a hook ever spoke.
    pub fn vendor_leaf_of(&self, id: &SessionId) -> Option<String> {
        let dir = self.dir(id)?;
        read_meta(&dir).ok()?.vendor_leaf
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
    ///
    /// `source` is a parameter rather than a default because the whole
    /// point of it is that the weakest caller must not be able to arrive
    /// here looking like the strongest. A default would put that in a
    /// comment asking new callers to be honest; a parameter makes the
    /// compiler ask.
    pub fn report(&self, id: &SessionId, word: State, source: Source) -> Result<(), String> {
        if word == State::Failed {
            return Err(msg::FAILED_IS_NOT_REPORTABLE.into());
        }
        let dir = self.existing_dir(id)?;
        let (meta, state) = read_pair(&dir)?;
        if state.exit.is_some() || !pid_says_alive(&meta) {
            return Err(msg::session_over(&id.0));
        }
        self.write_state(&dir, word, None, source)
    }

    /// The ending, recorded by whoever waited (the wrapper) or by an end
    /// hook. First recording wins; a session ends once.
    ///
    /// Keeps the word's provenance along with the word: this rewrites
    /// the row to add an exit code, and re-stamping it as though someone
    /// had freshly reported it would promote a screen reading on the way
    /// past.
    pub fn record_exit(&self, id: &SessionId, code: i32) -> Result<(), String> {
        let dir = self.existing_dir(id)?;
        let (_, state) = read_pair(&dir)?;
        if state.exit.is_some() {
            return Ok(());
        }
        let source = state.source.unwrap_or(Source::Reported);
        self.write_state(&dir, state.word, Some(code), source)
    }

    /// The row's current stamp — what "looked at now" covers.
    pub fn stamp(&self, id: &SessionId) -> Result<i64, String> {
        let dir = self.existing_dir(id)?;
        Ok(read_pair(&dir)?.1.at_ms)
    }

    /// The live sighting behind an id, for the takeover: the title the
    /// vendor gave it and the processes to end. `None` when nothing is
    /// running behind it — then there is nothing to kill, only a record
    /// to resume.
    pub fn sighting_of(&self, id: &SessionId) -> Option<(String, Vec<u32>)> {
        self.discovery
            .sweep()
            .rows
            .into_iter()
            .find(|f| f.id() == *id)
            .map(|f| (f.sighting.title, f.sighting.pids))
    }

    /// The kind and title on record, when a registry entry exists.
    pub fn on_record(&self, id: &SessionId) -> Option<(String, String)> {
        let dir = self.dir(id)?;
        read_meta(&dir).ok().map(|m| (m.kind, m.title))
    }

    /// Stands a host up for a discovered multiplexer session, so the
    /// terminal can attach to it like any hosted row (docs/handoff tmux
    /// 接桥). The host runs a **grouped** tmux client
    /// (`Tmux::grouped_attach_argv` has the judgment), registered under
    /// the discovered row's own id — the registry row then wins the
    /// merge, `hosted` turns true, and the whole hosted path (terminal,
    /// preview, close) applies unchanged. Closing kills khor's client,
    /// which is a detach; the user's own session is never touched, and
    /// the discovered row resurfaces on the next sweep.
    ///
    /// Idempotent: a second open finds the host and does nothing — two
    /// clicks must not stand up two clients.
    pub fn attach_multiplexed(&self, id: &SessionId) -> Result<(), String> {
        if let Some(dir) = self.dir_of(id) {
            if let Ok(hf) = crate::host::read_host_file(&dir) {
                if crate::link::pid_alive(hf.host_pid) {
                    return Ok(());
                }
                // A host that died without cleaning up (killed hard):
                // the marker points at nothing, so clear it and stand a
                // fresh one up rather than answering "attached" about a
                // corpse.
                let _ = fs::remove_file(crate::host::host_file_path(&dir));
            }
        }
        let tmux = self.discovery.tmux().ok_or_else(|| msg::no_terminal_here(&id.0))?;
        // Confirmed against a live sweep, not just parsed: a tmux leaf
        // also carries the session's creation time, so a same-numbered
        // session on a restarted server does not impersonate the clicked
        // one — and an agent row's route exists only while the sweep can
        // still see its process inside a session.
        let sweep = self.discovery.sweep();
        let shell_prefix = format!("{}/", khor_core::kind::SHELL);
        if let Some(target) = id
            .0
            .strip_prefix(&shell_prefix)
            .filter(|leaf| crate::adaptor::tmux::is_tmux_leaf(leaf))
            .and_then(crate::adaptor::tmux::tmux_target_of)
        {
            // A tmux row names its session outright.
            let held = sweep
                .multiplexed
                .iter()
                .find(|h| h.found.id() == *id)
                .ok_or_else(|| msg::tmux_session_gone(&id.0))?;
            let title = held.found.sighting.title.clone();
            self.ensure(id, khor_core::kind::SHELL, &title, None, Some(khor_core::category::SHELL))?;
            let dir = self.dir_of(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
            crate::host::spawn_host(&dir, id, &tmux.grouped_attach_argv(&target, None), (80, 24))?;
            return Ok(());
        }
        // Any other row reaches its terminal by redoing the fold's
        // intersection (`rows`): which multiplexer session holds this
        // row's process. The process is named by the vendor's sighting
        // for a discovered agent, or by the registry's own pid for a
        // row khor registered (`khor run` inside a pane). The host stood
        // up is a **viewer**: a lens on a process that is not its child,
        // so it must not write words, record endings, or claim the
        // row's pid (`host::spawn_viewer_host`).
        let fresh = !self.claims(id);
        let (pids, title, category, word) =
            if let Some(found) = sweep.rows.iter().find(|f| f.id() == *id) {
                (
                    found.sighting.pids.clone(),
                    found.sighting.title.clone(),
                    Some(found.category),
                    Some(found.sighting.word),
                )
            } else if let Ok(dir) = self.existing_dir(id) {
                let meta = read_meta(&dir)?;
                let pid = meta.pid.ok_or_else(|| msg::no_terminal_here(&id.0))?;
                (vec![pid], meta.title, None, None)
            } else {
                return Err(msg::no_terminal_here(&id.0));
            };
        // **Whose terminal, when one session has two.** `claude -r`
        // resumes a conversation into a new process while the old one is
        // still running, and both keep writing under the same
        // `sessionId` — one row standing for both, by the adaptor's
        // ruling (`adaptor::claude`'s sweep has the note). That row
        // already picks the freshest of them for its word and hands its
        // pids over freshest-first; asking the multiplexer sessions in
        // *their* order instead handed out whichever route came up
        // first, so the row could show one process's word over another
        // process's screen. Measured on this machine: a conversation
        // five days idle owned the terminal of the one being typed into,
        // and the session the person had in front of them was reachable
        // from no row at all.
        let held = pids
            .iter()
            .find_map(|p| sweep.multiplexed.iter().find(|h| h.holds.contains(p)))
            .ok_or_else(|| msg::no_terminal_here(&id.0))?;
        let target = held
            .found
            .id()
            .0
            .strip_prefix(&shell_prefix)
            .and_then(crate::adaptor::tmux::tmux_target_of)
            .ok_or_else(|| msg::no_terminal_here(&id.0))?;
        let window = tmux.window_holding(&target, &pids, &crate::adaptor::Procs::snapshot());
        if fresh {
            self.ensure(id, khor_core::kind::TUI, &title, None, category)?;
            // `register` opens with a reported 忙碌 — right for a spawn,
            // wrong here: nothing was spawned, and a reported word would
            // outrank the vendor's own files at the merge
            // (`a_word_read_off_the_screen_does_not_cover_what_the_vendor_says`).
            // Demote the opening word to a screen-grade one so the
            // sighting keeps deciding.
            if let Some(w) = word {
                let _ = self.report(id, w, Source::Screen);
            }
        }
        let dir = self.dir_of(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        crate::host::spawn_viewer_host(
            &dir,
            id,
            &tmux.grouped_attach_argv(&target, window.as_deref()),
            (80, 24),
            fresh,
        )?;
        Ok(())
    }

    /// Closes bridge hosts whose tmux session holds a row that lists on
    /// its own — the cleanup half of the fold in [`LiveKind::rows`]. The
    /// fold hides such a bridge row the moment it sees one (one running
    /// process, one row — the agent's); what it cannot do from a listing
    /// is stop the host process the bridge left running, so that happens
    /// here, on the serve tick like `Node::reap_borrows`. Killing the
    /// host is a tmux detach: the user's own session is never touched.
    pub fn reap_folded_bridges(&self) {
        let registered = self.registry_rows(&|_| 0);
        let bridges: Vec<SessionId> = registered
            .iter()
            .filter(|r| {
                r.row.id.0
                    .strip_prefix(&format!("{}/", khor_core::kind::SHELL))
                    .is_some_and(crate::adaptor::tmux::is_tmux_leaf)
            })
            .map(|r| r.row.id.clone())
            .collect();
        if bridges.is_empty() {
            return;
        }
        let sweep = self.discovery.sweep();
        let mut claimed: Vec<u32> = registered.iter().filter_map(|r| r.pid).collect();
        claimed.extend(sweep.rows.iter().flat_map(|f| f.sighting.pids.iter().copied()));
        for id in bridges {
            let Some(held) = sweep.multiplexed.iter().find(|h| h.found.id() == id) else {
                continue;
            };
            if held.holds.iter().any(|p| claimed.contains(p)) {
                let _ = self.close_session(&id);
            }
        }
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
    /// **A collision is won by the better source, and that is the whole
    /// of "同源去重".** A claude session that both reports through hooks
    /// and shows up on disk is one session; the hook-registered row is
    /// the richer of the two (it has a word history, and can say 完成,
    /// which the status file cannot), so the discovered sighting steps
    /// aside. Both sides mint the id through `adaptor::id_for`, which is
    /// what makes the collision detectable at all.
    ///
    /// **This used to be phrased as "the registry wins", and that was
    /// only ever right by accident** — everything that could write a
    /// registry word outranked a sighting, so "who wrote it" and "which
    /// channel" gave the same answer. The PTY fallback splits them: it
    /// writes registry words that a sighting beats. So the row carries
    /// [`LiveState::source`] and the test is on that. Nothing about the
    /// hook case changed; what changed is that its reason is now the
    /// thing being read, instead of a coincidence being read in its
    /// place.
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
        let mut out: Vec<Session> = registered.iter().map(|r| r.row.clone()).collect();
        let unpinned: Vec<&SessionId> =
            registered.iter().filter(|r| r.pid.is_none()).map(|r| &r.row.id).collect();
        // Rows whose word came off a screen, and which a sighting from
        // the vendor's own files therefore outranks.
        let guessed: Vec<&SessionId> = registered
            .iter()
            .filter(|r| !r.word_outranks_disk)
            .map(|r| &r.row.id)
            .collect();
        // Everything on this list that is a running process — and which
        // row it landed on. A pid on record for a process that has
        // already died costs nothing here: what it is compared against
        // is built from the live process table, so a dead number can
        // never match. The seat matters now, not just membership: a
        // multiplexer session holding a listed agent used to be merely
        // dropped, and the link — "that agent's terminal lives in this
        // tmux session" — was computed and then thrown away. The seat
        // map is what lets the multiplexed loop below keep it, as the
        // agent row's `term`.
        let mut seat_of_pid: std::collections::BTreeMap<u32, usize> =
            std::collections::BTreeMap::new();
        for (i, r) in registered.iter().enumerate() {
            if let Some(pid) = r.pid {
                seat_of_pid.insert(pid, i);
            }
        }
        let sweep = self.discovery.sweep();
        self.unmapped.store(sweep.unmapped, Ordering::Relaxed);
        for found in sweep.rows {
            let id = found.id();
            let kind = found.kind;
            let sighting = found.sighting;
            let pids = sighting.pids.clone();
            if let Some(seat) = out.iter().position(|r| r.id == id) {
                for &p in &pids {
                    seat_of_pid.insert(p, seat);
                }
                // The registered row wins, but it may have been registered
                // by something that could not name the vendor (a bare
                // `khor run --tui`). The sweep read that vendor's own
                // files, so it can — and an empty category is a gap to
                // fill, not an answer to respect.
                if out[seat].category.is_none() {
                    out[seat].category = Some(found.category.to_owned());
                }
                if guessed.contains(&&id) {
                    // **The registry does not always win — the stronger
                    // source does.** This row's word was read off the
                    // screen, and the vendor just told us in its own
                    // files. Letting the guess stand here would put the
                    // bottom of the four-family order on top of the
                    // family it is a fallback *for*.
                    let (state, unread) =
                        settle_done(sighting.word, sighting.at_ms, watermark(&id.0));
                    out[seat].state =
                        StateStamp { state, at: Millis(sighting.at_ms.max(0) as u64) };
                    out[seat].unread = unread;
                } else if sighting.word == State::Failed && unpinned.contains(&&id) {
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
                last: None,
                via: None,
                term: false,
            });
            let seat = out.len() - 1;
            for &p in &pids {
                seat_of_pid.insert(p, seat);
            }
        }
        let mut folded: Vec<SessionId> = Vec::new();
        // Oldest first, so the original session outranks any grouped
        // twin of it; the leaf leads with the creation epoch, so the
        // string order is the age order.
        let mut multiplexed = sweep.multiplexed;
        multiplexed
            .sort_by(|a, b| a.found.sighting.vendor_session_id.cmp(&b.found.sighting.vendor_session_id));
        let mut seen_holds: Vec<u32> = Vec::new();
        for held in multiplexed {
            let id = held.found.id();
            let bridge_seat = out.iter().position(|r| r.id == id);
            // A grouped session lists the very same panes under a second
            // session id — measured (`tmux new-session -t base` → two
            // lines, one pane pid). The bridge's own client is exactly
            // such a twin, so without this every open terminal face grew
            // a phantom row beside the session it was showing.
            if bridge_seat.is_none() && held.holds.iter().any(|p| seen_holds.contains(p)) {
                continue;
            }
            seen_holds.extend(held.holds.iter().copied());
            let seats: Vec<usize> = held
                .holds
                .iter()
                .filter_map(|p| seat_of_pid.get(p).copied())
                .filter(|s| Some(*s) != bridge_seat)
                .collect();
            if !seats.is_empty() {
                // The old rule ended at "drop the duplicate". The same
                // judgment now also keeps what it computed: these rows'
                // terminals live inside this tmux session, so a host can
                // be bridged in for them (`attach_multiplexed` resolves
                // the route by redoing this very intersection).
                for s in seats {
                    out[s].term = true;
                }
                // A bridge row somebody stood up for this same session
                // (before the fold rule, by clicking the tmux row) is
                // the duplicate wearing a registry entry: one running
                // process, one row — the agent's. The bridge host is
                // reaped separately (`reap_folded_bridges`).
                if bridge_seat.is_some() {
                    folded.push(id);
                }
                continue;
            }
            if let Some(seat) = bridge_seat {
                // This row *is* the multiplexer session, wearing a
                // registry entry from an earlier open. The session is
                // still on this machine, so a terminal is still to be
                // had — whether or not the host stood up by that open
                // outlived it. Without this line the honest `term`
                // above (which answers about a process) takes the face
                // away from a session sitting right there.
                out[seat].term = true;
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
                last: None,
                via: None,
                // A discovered multiplexer session is exactly what the
                // bridge can attach to.
                term: true,
            });
        }
        out.retain(|r| !folded.contains(&r.id));
        // The preview post-pass, one place for every claude-backed row —
        // discovered, hooked, hosted or GUI alike: whatever the vendor's
        // transcript last said. **The transcript outranks the hosted
        // screen line**, not the other way around: an agent TUI's last
        // non-empty screen line is its chrome ("⏵⏵ bypass permissions
        // on…"), which is true of the screen and useless as a preview —
        // the screen line stays only as the fallback for a row whose
        // transcript nobody can find yet (no hook has spoken, no leaf
        // matches). The leaf is the vendor's own where a hook recorded
        // one (`Meta::vendor_leaf` — a khor-minted id names no vendor
        // file), else the id's leaf, which for discovered rows is the
        // vendor's id already. Cached by (path, mtime) inside
        // `last_said`, so a list poll costs stats, not reads.
        let vendor_leaves: std::collections::BTreeMap<String, String> = registered
            .iter()
            .filter_map(|r| r.vendor_leaf.clone().map(|v| (r.row.id.0.clone(), v)))
            .collect();
        let claude =
            crate::adaptor::claude::Claude::at(crate::adaptor::vendor_home(&self.root).join(".claude"));
        for r in out.iter_mut() {
            if r.category.as_deref() != Some("claude") {
                continue;
            }
            let leaf = vendor_leaves.get(&r.id.0).cloned().or_else(|| {
                r.id.0
                    .strip_prefix(&format!("{}/", khor_core::kind::TUI))
                    .map(str::to_owned)
            });
            if let Some(said) = leaf.and_then(|l| claude.last_said(&l)) {
                r.last = Some(said);
            }
        }
        out
    }

    /// Registry rows, each with the pid on record for it. Whether there
    /// is one decides whether anyone can pronounce the session dead;
    /// which one it is decides whether a multiplexer session is already
    /// on this list (see [`LiveKind::rows`]).
    fn registry_rows(&self, watermark: &impl Fn(&str) -> i64) -> Vec<RegistryRow> {
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
            // The id is the meta's own where it says (Meta::id); the
            // directory-name rebuild is only for entries older than
            // that field, whose ids all spell `kind/leaf`.
            let id = if meta.id.is_empty() {
                let Some(leaf) = name.strip_prefix(&format!("{}-", meta.kind)) else {
                    continue;
                };
                SessionId(format!("{}/{leaf}", meta.kind))
            } else {
                SessionId(meta.id.clone())
            };
            let (word, at, unread) = face(&e.path(), &meta, &state, watermark(&id.0));
            out.push(RegistryRow {
                word_outranks_disk: state.word_outranks_disk(),
                vendor_leaf: meta.vendor_leaf.clone(),
                row: Session {
                    id,
                    kind: Kind(meta.kind.clone()),
                    title: meta.title.clone(),
                    home: self.device,
                    state: StateStamp { state: word, at: Millis(at.max(0) as u64) },
                    unread,
                    category: meta.category.clone(),
                    // The host keeps the terminal's last non-empty line
                    // beside its state; a row with no host keeps `None`
                    // here and may still earn a preview from a
                    // transcript (`rows`'s post-pass — which also
                    // outranks this line for agent rows).
                    last: crate::host::read_last(&e.path()),
                    via: None,
                    // A host here is a terminal to be had here — a
                    // *running* host (`Node::is_hosted` has why the
                    // file alone cannot answer this). A row whose host
                    // died but whose process is still sitting in a
                    // multiplexer earns `term` back in the loop below,
                    // which is where that route is resolved anyway.
                    term: crate::host::read_host_file(&e.path())
                        .is_ok_and(|hf| crate::link::pid_running(hf.host_pid)),
                },
                pid: meta.pid,
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

    fn write_state(
        &self,
        dir: &Path,
        word: State,
        exit: Option<i32>,
        source: Source,
    ) -> Result<(), String> {
        let state = LiveState { word, at_ms: now_ms(), exit, source: Some(source) };
        write_whole(&dir.join("state.json"), &serde_json::to_vec(&state).map_err(|e| e.to_string())?)
    }
}

/// A registry row plus the two things [`LiveKind::rows`] needs that a
/// [`Session`] does not carry: who is running it, and how good its word
/// is.
struct RegistryRow {
    row: Session,
    pid: Option<u32>,
    /// See [`LiveState::source`]. False only for a word read off the
    /// screen, which is the one case where the vendor's own files know
    /// better than the row already on the list.
    word_outranks_disk: bool,
    /// The vendor's own leaf where a hook recorded one (`Meta`), carried
    /// so the preview post-pass can find a khor-minted row's transcript.
    vendor_leaf: Option<String>,
}

/// Who is still there behind a registry row.
///
/// The registry records **one** pid, and for a 持久 session that pid is
/// the host — khor's own plumbing — not the thing the user started. Most
/// of the time that distinction does not matter, because the host
/// outlives its child and records the ending. [`Backing::HostLost`] is
/// the case where it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backing {
    /// No pid on record: nobody is in a position to pronounce it dead
    /// (a hook-observed session; a row registered before its process
    /// started).
    Unpronounceable,
    Running,
    /// **The host is gone and the session's process is not.** khor
    /// cannot reach this session any more — the port in `host.json`
    /// belongs to nothing — while the work inside it carries on.
    HostLost,
    Gone,
}

/// What is still running behind one registry entry.
///
/// Both questions go through `link::pid_alive`, so both inherit its
/// rule: **only ESRCH is dead.** A pid that exists but refuses a signal
/// (EPERM) is alive, which matters here twice over — reading it the
/// other way would turn a host khor cannot signal into a lost host, and
/// a child khor cannot signal into a dead session.
fn backing(dir: &Path, meta: &Meta) -> Backing {
    let Some(pid) = meta.pid else {
        return Backing::Unpronounceable;
    };
    if crate::link::pid_alive(pid) {
        return Backing::Running;
    }
    // The recorded pid is gone. For a hosted session that was the host,
    // and the host dying is not the session dying: ask the child.
    match crate::host::read_host_file(dir) {
        Ok(hf) if crate::link::pid_alive(hf.child_pid) => Backing::HostLost,
        _ => Backing::Gone,
    }
}

/// The six-word mapping (docs/SESSION.md), one place:
///
/// - an exit code decides first — shell lands on 完成/失败, a tui that
///   exited cleanly was looked at by definition (its turn ended on
///   screen) and lands on 空闲;
/// - a dead process with no recorded ending is 失败: the normal path
///   always records, so a missing ending is itself an abnormal ending;
/// - **a host that died while its child lives is 中断**, derived below;
/// - otherwise the reported word stands, with 完成 downgrading to 空闲
///   once the seen watermark covers it (看过了).
///
/// Only 完成/未看 counts as unread — 失败 sinks in the list but never
/// holds the badge (docs/UX.md 角标可归零).
///
/// # Why a lost host is 中断 and not 失败
///
/// Derived from the pair the two words form, not chosen. 中断 and 失败
/// both say something went wrong; **what divides them is whether the
/// process is still there** — 中断 is an error you can carry on from,
/// 失败 is one you have to restart from (`khor_core::State::Errored`
/// says so in its own words: "the name must not suggest stopped").
///
/// Here the session's process is running. The user's work is not lost,
/// and restarting would throw it away. So 失败 is the one word this must
/// not be, and 中断 is what is left once that is settled.
///
/// **This corrects docs/SESSION.md's table, where the shell row has "—"
/// under 中断.** That row describes a shell's own vocabulary, where an
/// error simply exits and there is nothing to be alive afterwards. It
/// was written before the persistent host existed, and the host is a
/// third party the shell knows nothing about: it is khor that broke
/// here, not the shell.
///
/// The stamp stays the last word's, because nothing recorded when the
/// host died — the row therefore ages from the last thing that was
/// known, which is the honest reading of "and then khor lost sight of
/// it".
fn face(dir: &Path, meta: &Meta, state: &LiveState, watermark: i64) -> (State, i64, u64) {
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
    match backing(dir, meta) {
        Backing::Gone => (State::Failed, state.at_ms, 0),
        Backing::HostLost => (State::Errored, state.at_ms, 0),
        Backing::Running | Backing::Unpronounceable => {
            let (word, unread) = settle_done(state.word, state.at_ms, watermark);
            (word, state.at_ms, unread)
        }
    }
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

    /// A row whose kind is not its id's first segment must still list.
    /// A GUI session registers under the vendor-session agreement
    /// (`tui/<uuid>`) with `kind: gui`, and the old directory-name
    /// rebuild — strip the kind off the front — dropped exactly those
    /// rows: the session answered `dir_of` and every op while appearing
    /// in no list anywhere. `Meta::id` is what carries it now.
    #[test]
    fn a_row_lists_even_when_its_kind_is_not_its_ids_first_segment() {
        let root = tmp("gui-id");
        let k = kind_at(&root);
        let id = sid("tui/stub-session-1");
        k.register(&id, "gui", "stub", Some(std::process::id()), None).unwrap();
        let rows = k.rows(|_| 0);
        let row = rows.iter().find(|r| r.id == id).expect("the row lists");
        assert_eq!(row.kind.0, "gui");
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

        k.report(&id, State::Blocked, Source::Reported).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Blocked, 0), "waiting for approval");

        k.report(&id, State::Done, Source::Reported).unwrap();
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
            k.report(&id, State::Busy, Source::Reported).unwrap_err() == msg::session_over(&id.0),
            "a dead session refuses reports by name"
        );

        let id2 = sid("tui/nopid1");
        k.register(&id2, "tui", "x", None, None).unwrap();
        k.report(&id2, State::Busy, Source::Reported).unwrap();
        let row = k.rows(|_| 0).into_iter().find(|r| r.id == id2).unwrap();
        assert_eq!(row.state.state, State::Busy, "no pid on record = the word stands");
        let _ = fs::remove_dir_all(&root);
    }

    /// The lost-host rule at the level it is decided, with no host to
    /// start: a dead recorded pid, a `host.json` naming a child that is
    /// certainly alive (this test process), and the row must not say
    /// 失败.
    ///
    /// The integration test in `crates/cli/tests/open.rs` does the same
    /// thing for real — a real host, killed for real. This one exists
    /// because the rule is a branch in `face`, and a branch deserves a
    /// test that runs on a machine where spawning a pty is not an option.
    #[test]
    fn a_dead_host_with_a_live_child_is_errored_not_failed() {
        let root = tmp("hostlost");
        let k = kind_at(&root);
        let id = sid("shell/hosted1");
        let dead = dead_pid();
        k.register(&id, "shell", "work", Some(dead), None).unwrap();
        let dir = k.dir_of(&id).unwrap();

        // Control first: with no host file, a dead recorded pid is the
        // ordinary missing ending.
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Failed);

        fs::write(
            crate::host::host_file_path(&dir),
            serde_json::to_vec(&crate::host::HostFile {
                port: 1,
                cookie: "x".into(),
                host_pid: dead,
                // Alive by construction: this is the test itself.
                child_pid: std::process::id(),
            })
            .unwrap(),
        )
        .unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(
            row.state.state,
            State::Errored,
            "the host is khor's, the child is the user's work, and it is still running"
        );
        assert_eq!(row.unread, 0, "中断 carries no badge; only 完成 does");

        // And the other control: a host file naming a child that is also
        // gone is back to 失败, so the branch above turns on the child
        // being alive and not merely on a host file existing.
        fs::write(
            crate::host::host_file_path(&dir),
            serde_json::to_vec(&crate::host::HostFile {
                port: 1,
                cookie: "x".into(),
                host_pid: dead,
                child_pid: dead_pid(),
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Failed);
        let _ = fs::remove_dir_all(&root);
    }

    /// **A marker file outlives the process it names.** `term` is the
    /// row's claim that a terminal can be had here, and a host dies with
    /// the bridge that spawned it while its `host.json` stays on disk.
    /// Reading only the file's existence made that claim about a corpse,
    /// and every click on such a row then skipped the re-bridge that
    /// would have fixed it and connected to a closed port instead
    /// (`Node::is_hosted` carries the whole story).
    ///
    /// The live half is the control, and it is the half that matters: a
    /// `term` that simply answered false always would pass the negative
    /// assertion below while taking the terminal away from every hosted
    /// row on the machine.
    ///
    /// No pid on record on purpose — a row with one can earn `term` back
    /// from the multiplexer fold, which is a different rule and would
    /// answer this test's question for it.
    #[test]
    fn a_host_file_left_behind_by_a_dead_host_is_not_a_terminal() {
        let root = tmp("stalehost");
        let k = kind_at(&root);
        let id = sid("shell/stale1");
        k.register(&id, "shell", "work", None, None).unwrap();
        let dir = k.dir_of(&id).unwrap();
        let write_host = |host_pid: u32| {
            fs::write(
                crate::host::host_file_path(&dir),
                serde_json::to_vec(&crate::host::HostFile {
                    port: 1,
                    cookie: "x".into(),
                    host_pid,
                    child_pid: std::process::id(),
                })
                .unwrap(),
            )
            .unwrap();
        };
        let term_now = || k.rows(|_| 0).into_iter().find(|r| r.id == id).unwrap().term;

        // Alive by construction: this is the test itself.
        write_host(std::process::id());
        assert!(term_now(), "a running host is a terminal to be had");

        write_host(dead_pid());
        assert!(
            !term_now(),
            "the host is gone; the file it left behind must not promise a terminal"
        );
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
        assert_eq!(k.report(&id, State::Failed, Source::Reported).unwrap_err(), msg::FAILED_IS_NOT_REPORTABLE);
        k.record_exit(&id, 0).unwrap();
        assert_eq!(k.report(&id, State::Busy, Source::Reported).unwrap_err(), msg::session_over(&id.0));
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
        k.report(&id, State::Done, Source::Reported).unwrap();

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

    /// The same collision, decided the other way, because the registry
    /// word came off a screen.
    ///
    /// **Both halves are the test.** The screen and the vendor's files
    /// are made to disagree, and then the *only* thing that changes
    /// between the two halves is which [`Source`] the word was written
    /// with — same fixture, same ids, same two words. Without the
    /// control the first half proves nothing: "the row says 待批" is
    /// also what you get from a merge that ignored the registry
    /// entirely, and from one that never wrote the word at all.
    ///
    /// The direction being defended is the one that would hurt: a
    /// pattern match saying "nothing is happening" must not be able to
    /// paint over a permission prompt that claude has written down. Get
    /// this backwards and khor stops asking the user for something an
    /// agent is stuck waiting on — the one word 待批 exists for.
    #[test]
    fn a_word_read_off_the_screen_does_not_cover_what_the_vendor_says() {
        use crate::adaptor::{Discovery, Proc, Procs};

        let root = tmp("provenance");
        let vendors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc { name: "claude".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: None },
        )]);
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(procs));
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(discovery);

        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        k.register(&id, "tui", "one", Some(std::process::id()), None).unwrap();

        // The screen reader says the session is idle. The vendor's own
        // file says it is sitting on a permission prompt.
        k.report(&id, State::Idle, Source::Screen).unwrap();
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "still one session");
        assert_eq!(
            rows[0].state.state,
            State::Blocked,
            "the vendor's own file outranks a guess about its screen",
        );

        // Control: the identical disagreement, reported instead of read.
        k.report(&id, State::Idle, Source::Reported).unwrap();
        assert_eq!(
            k.rows(|_| 0)[0].state.state,
            State::Idle,
            "control: a reported word still wins, so the source is the only thing deciding",
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// Recording an ending must not launder a word's provenance.
    ///
    /// `record_exit` rewrites the whole entry to add an exit code, so it
    /// is a place where a screen reading could be re-stamped as though
    /// somebody had reported it — a promotion nobody asked for, arriving
    /// through a function whose job is unrelated. Cheap to get right,
    /// invisible when wrong, which is why it is worth its own test
    /// rather than a note.
    #[test]
    fn recording_an_ending_keeps_the_word_as_weak_as_it_was() {
        let root = tmp("exitprov");
        let k = kind_at(&root);
        let id = sid("tui/screenword");
        k.register(&id, "tui", "agent", Some(std::process::id()), None).unwrap();
        k.report(&id, State::Idle, Source::Screen).unwrap();
        assert_eq!(
            read_pair(&k.dir(&id).unwrap()).unwrap().1.source,
            Some(Source::Screen),
            "precondition: it went in weak",
        );

        k.record_exit(&id, 0).unwrap();
        assert_eq!(
            read_pair(&k.dir(&id).unwrap()).unwrap().1.source,
            Some(Source::Screen),
            "an ending is not a fresh report",
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// An entry written before provenance existed reads as the strong
    /// source, not the weak one.
    ///
    /// This is the "what does it turn into while it is not connected
    /// yet" blank filled in. Absent leans strong because every word
    /// already on a disk somewhere was put there by a hook or the
    /// wrapper — the screen reader did not exist yet. Leaning the other
    /// way would have every existing row on every existing machine
    /// quietly handed over to the sweep the moment this field shipped.
    #[test]
    fn a_row_written_before_provenance_existed_is_treated_as_reported() {
        use crate::adaptor::{Discovery, Proc, Procs};

        let root = tmp("legacyprov");
        let vendors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc { name: "claude".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: None },
        )]);
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(procs));
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(discovery);

        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        k.register(&id, "tui", "one", Some(std::process::id()), None).unwrap();
        k.report(&id, State::Idle, Source::Reported).unwrap();

        // Rewrite the entry the way an older khor would have left it.
        // Built by *deleting* the key from what this khor writes rather
        // than by spelling the old shape out here: that keeps the other
        // fields encoded exactly as the real writer encodes them, so the
        // test cannot pass because it happened to hand-write a word the
        // way the parser likes it.
        let dir = k.dir(&id).unwrap();
        let path = dir.join("state.json");
        let mut doc: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        doc.as_object_mut().unwrap().remove("source").expect("the new writer put one there");
        let bytes = serde_json::to_vec(&doc).unwrap();
        assert!(
            !String::from_utf8_lossy(&bytes).contains("source"),
            "the bytes must really lack the key, or this test proves nothing",
        );
        fs::write(&path, &bytes).unwrap();
        assert!(
            read_pair(&dir).unwrap().1.source.is_none(),
            "precondition: it reads back absent",
        );

        assert_eq!(
            k.rows(|_| 0)[0].state.state,
            State::Idle,
            "an entry from before this field stands against the sweep",
        );
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
        assert!(
            rows[0].term,
            "the fold keeps what it computed: this agent's terminal lives in that tmux session"
        );

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
        assert!(rows[0].term, "a discovered tmux session is exactly what the bridge attaches to");
        assert!(
            !rows[1].term,
            "control: an agent held by no multiplexer has no route to a terminal here"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The fold against a **registry entry**: a bridge row somebody
    /// stood up for a tmux session (by clicking it before the agent
    /// inside was listed, or before the fold existed) is the duplicate
    /// wearing a registry dir — one running process, one row, and the
    /// row is the agent's, wearing the route.
    #[test]
    fn an_old_bridge_row_folds_into_the_agent_row_it_duplicates() {
        use crate::adaptor::{tmux::Tmux, Discovery, Proc, Procs};

        let root = tmp("bridge-fold");
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([
            (
                3001,
                Proc { name: "-zsh".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: Some(1) },
            ),
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
            "bridge-fold",
            "$7|1786245338|1786245400|3001|0|zsh|with-claude\n",
        );
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&holding)),
        ));
        // The bridge row, registered the way `attach_multiplexed` would
        // have registered it for this very session.
        let bridge = sid("shell/1786245338-7");
        k.register(&bridge, "shell", "with-claude", None, Some(khor_core::category::SHELL))
            .unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "the bridge row stops listing: one process, one row");
        assert_eq!(rows[0].id.0, "tui/11111111-1111-4111-8111", "and the row is the agent's");
        assert!(rows[0].term, "wearing the route");
        let _ = fs::remove_dir_all(&root);
    }

    /// A grouped twin session is furniture, not a second row: tmux lists
    /// the same panes again under the twin's own id (measured — see the
    /// comment in `rows`), and the twin a bridge's client creates would
    /// otherwise appear as a phantom row for exactly as long as someone
    /// is looking at the terminal face.
    #[test]
    fn a_grouped_twin_of_a_listed_session_is_not_a_second_row() {
        use crate::adaptor::{tmux::Tmux, Discovery, Proc, Procs};

        let root = tmp("tmux-twin");
        let procs = Procs::of([(
            3001,
            Proc { name: "-zsh".into(), started_ms: 1_700_000_000_000, cwd: None, ppid: Some(1) },
        )]);
        // The same pane, listed twice: once by the original session,
        // once by a grouped twin created later.
        let fake = crate::adaptor::tmux::fake_tmux(
            "twin",
            "$7|1786245338|1786245400|3001|0|zsh|base\n\
             $9|1786245400|1786245400|3001|0|zsh|base-1\n",
        );
        let k = LiveKind::new(root.clone(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::empty().with_procs(procs).with_tmux(Tmux::at_binary(&fake)),
        ));
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "one set of panes is one row");
        assert_eq!(rows[0].id.0, "shell/1786245338-7", "and the original session is the row");
        let _ = fs::remove_dir_all(&root);
    }

    /// The transcript outranks the hosted screen line for an agent row
    /// (会话身份批): a TUI's last non-empty screen line is its chrome —
    /// "⏵⏵ bypass permissions on…" — true of the screen, useless as a
    /// preview. Three rows, one edge each:
    /// - a row whose id leaf finds a transcript: the transcript wins;
    /// - a row with no transcript anywhere: the screen line is the
    ///   honest fallback and stays;
    /// - a khor-minted row whose transcript is reachable only through
    ///   the recorded vendor leaf: the vendor leaf is the path.
    #[test]
    fn the_transcript_outranks_the_hosted_screen_line() {
        let root = tmp("preview-precedence");
        let proj = root.join(".claude/projects/-w");
        fs::create_dir_all(&proj).unwrap();
        let uuid = "afafafaf-bbbb-4ccc-8ddd-eeeeeeeeeeee";
        fs::write(
            proj.join(format!("{uuid}.jsonl")),
            r#"{"type":"user","message":{"role":"user","content":"from the transcript"}}"#,
        )
        .unwrap();
        let k = kind_at(&root);

        let hooked = crate::adaptor::id_for(uuid);
        k.register(&hooked, "tui", "one", None, Some("claude")).unwrap();
        fs::write(k.dir_of(&hooked).unwrap().join("last.txt"), "⏵⏵ bypass permissions on")
            .unwrap();

        let bare = sid("tui/no-transcript-here");
        k.register(&bare, "tui", "two", None, Some("claude")).unwrap();
        fs::write(k.dir_of(&bare).unwrap().join("last.txt"), "$ make test").unwrap();

        let minted = sid("tui/khormint00");
        k.register(&minted, "tui", "three", None, Some("claude")).unwrap();
        k.learn_vendor_leaf(&minted, &clean_leaf(uuid)).unwrap();
        fs::write(k.dir_of(&minted).unwrap().join("last.txt"), "⏵⏵ auto mode on").unwrap();

        let rows = k.rows(|_| 0);
        let last = |id: &SessionId| rows.iter().find(|r| r.id == *id).unwrap().last.clone();
        assert_eq!(last(&hooked).as_deref(), Some("from the transcript"));
        assert_eq!(
            last(&bare).as_deref(),
            Some("$ make test"),
            "no transcript: the screen line is the honest fallback"
        );
        assert_eq!(
            last(&minted).as_deref(),
            Some("from the transcript"),
            "a khor-minted row reaches its transcript through the vendor leaf"
        );
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
        k.report(&id, State::Busy, Source::Reported).unwrap();

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
