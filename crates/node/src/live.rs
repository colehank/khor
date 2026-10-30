//! The live kind: sessions backed by a real local process — an agent TUI
//! reporting through hooks, or a command wrapped by `khor run` (临时: it
//! lives and dies with the terminal that started it; the persistent host
//! is the next batch). State is derived fresh on every ask and never
//! syncs (docs/NET.md: a stored copy is stale by definition); only this
//! device can derive these rows, so they travel as peer reports.
//!
//! Registry: one dir per session under `.khor/sessions/<kind>-<leaf>`,
//! holding `meta.json` and the legacy `state.json`. A generation-owned
//! GUI keeps its sole live state inside its Meta snapshot; old files stay
//! readable for rows without that owner. Short row locks serialize writers,
//! while GUI readers need only one atomic whole-file snapshot, never a lock.
//!
//! Word discipline (docs/SESSION.md 六词映射): processes report only the
//! live words — busy, blocked, done, errored, idle. `failed` is never
//! reported; it is derived from the exit code (or from an ending nobody
//! recorded), so no hook can paint a live process as failed.

use std::fs;
use std::io::Read;
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use khor_catalog::msg;
use khor_core::{DeviceId, Kind, Millis, Session, SessionId, State, StateStamp};

use crate::adaptor::Discovery;

mod recovery_attempt;
#[path = "session_recovery.rs"]
mod session_recovery;
mod proc_identity;

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
    /// Exact source and execution identity for a Khor-owned GUI host. Legacy
    /// rows remain readable; a later GUI opening establishes these facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) gui_native: Option<crate::session_content::journal::Identity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) gui_run: Option<GuiRun>,
    /// Fresh shell ancestry is metadata, never an argv replay instruction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) shell_run: Option<ShellRun>,
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
    /// The working directory khor started this session in, raw — and
    /// `khor_core::Session::place` now carries the same directory
    /// unchanged (`place_of`). Written at birth by whichever opener
    /// chose the directory, because that is
    /// the only moment anybody knows: a shell that has `cd`ed away
    /// cannot be asked where it began. Runtime location is `doing.cwd`;
    /// readers use `Session::current_cwd` for the current directory.
    ///
    /// `None` for an entry from before this field, and for a session
    /// nobody could place — the same thing it means for a fresh one.
    #[serde(default)]
    pub place: Option<String>,
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ShellRun {
    pub(crate) previous: SessionId,
    pub(crate) root: SessionId,
}

/// A GUI execution, distinct from both the retained row and provider conversation.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuiRun {
    id: String,
    started_ms: i64,
    /// The single live state owner for this GUI generation. Legacy state.json
    /// remains untouched; no GUI reader consults that older projection.
    state: LiveState,
}

/// Only the host that received this key may report for that GUI execution.
pub(crate) struct RunKey {
    session: SessionId,
    directory: PathBuf,
    run_id: String,
    native: crate::session_content::journal::Identity,
}
impl std::fmt::Debug for RunKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunKey")
            .field("session", &self.session)
            .field("run", &self.run_id)
            .finish_non_exhaustive()
    }
}

/// A recorded working directory as the row carries it: **the directory
/// itself**, and `None` when there was nothing recorded.
///
/// # Why this does not fold, since 工作区批③
///
/// It used to answer the enclosing checkout (`crate::place::
/// project_root`), and the reason written down was that folding in
/// three adaptors is three judgments that can disagree. That is still
/// true, and the fold is still made exactly once — in
/// [`crate::Node::places`], for the one list that wants it.
///
/// What changed is who reads this field. A workspace is now any
/// directory a person named (docs/WORKSPACE.md), and the workspace pane
/// asks 「which sessions are running **under** this directory」. A
/// folded answer is wrong in both directions for that question: a
/// session started in `~/w/paper/data` reports `~/w/paper` and misses a
/// workspace at `~/w/paper/data`, while a workspace at the checkout
/// root collects every session under it — which looks right, and is a
/// coincidence, because the fold and the subtree agree only when the
/// fold happened to stop at that directory.
///
/// So this says where the session actually is, and whoever needs
/// something coarser derives it. A raw cwd can always be folded; a
/// folded one cannot be unfolded.
fn place_of(cwd: Option<&str>) -> Option<String> {
    let cwd = cwd?;
    if cwd.is_empty() {
        return None;
    }
    Some(cwd.to_owned())
}

/// **Where a hosted shell stands, as a row's line** (item 26): the
/// ledger's `OSC 7` directory, else the title a program set, else the
/// shell's working directory as the OS reports it; a directory under home
/// written with `~`. `None` when none of the three is known.
fn shell_stands(doing: Option<&khor_core::Doing>, shell_pid: u32) -> Option<String> {
    if let Some(cwd) = doing.and_then(|d| d.cwd.as_deref()) {
        return Some(tilde(cwd));
    }
    if let Some(title) = doing
        .and_then(|d| d.title.as_deref())
        .filter(|t| !t.trim().is_empty())
    {
        return Some(title.to_owned());
    }
    cwd_of(shell_pid).map(|p| tilde(&p.to_string_lossy()))
}

/// A path under the home directory, written from `~`.
fn tilde(path: &str) -> String {
    match std::env::home_dir().map(|h| h.to_string_lossy().into_owned()) {
        Some(h) if !h.is_empty() && (path == h || path.starts_with(&format!("{h}/"))) => {
            format!("~{}", &path[h.len()..])
        }
        _ => path.to_owned(),
    }
}

/// A process's working directory, as the OS reports it.
fn cwd_of(pid: u32) -> Option<PathBuf> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut sys = System::new();
    let pid = Pid::from_u32(pid);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cwd(UpdateKind::Always),
    );
    sys.process(pid)?.cwd().map(Path::to_path_buf)
}

/// Which address a terminal should connect to after
/// [`LiveKind::attach_multiplexed`] — the session's own host, or a
/// viewer bridged onto a tmux seat (`crate::host::view_file_path` has
/// why those are two files).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    Host,
    View,
}

enum AddressPreference {
    Host,
    Seat,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) gui_run: Option<String>,
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
    if cleaned.is_empty() {
        "x".into()
    } else {
        cleaned
    }
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

    /// The file an unregistered agent row's [`SeatNote`] lives in — the
    /// registry's own `<kind>-<leaf>` spelling, under `.khor/seats`.
    fn seat_note_path(&self, id: &SessionId) -> Option<PathBuf> {
        let dir = self.dir(id)?;
        Some(seats_dir(&self.root).join(dir.file_name()?))
    }

    fn seat_note(&self, id: &SessionId) -> Option<SeatNote> {
        let text = fs::read(self.seat_note_path(id)?).ok()?;
        serde_json::from_slice::<SeatNote>(&text)
            .ok()
            .filter(|n| n.id == id.0)
    }

    /// Every note on disk; an unreadable one is skipped, like a
    /// half-written registry dir.
    fn seat_notes(&self) -> Vec<SeatNote> {
        let Ok(rd) = fs::read_dir(seats_dir(&self.root)) else {
            return Vec::new();
        };
        rd.flatten()
            .filter_map(|e| fs::read(e.path()).ok())
            .filter_map(|b| serde_json::from_slice::<SeatNote>(&b).ok())
            .filter(|n| self.seat_note_path(&SessionId(n.id.clone())).is_some())
            .collect()
    }

    /// Best effort, `note_seat`'s terms: a note that did not land is
    /// written again by the next listing that sees the row.
    fn write_seat_note(&self, note: &SeatNote) {
        let Some(path) = self.seat_note_path(&SessionId(note.id.clone())) else {
            return;
        };
        let _ = fs::create_dir_all(seats_dir(&self.root));
        if let Ok(bytes) = serde_json::to_vec(note) {
            let _ = write_whole(&path, &bytes);
        }
    }

    fn clear_seat_note(&self, id: &SessionId) {
        if let Some(path) = self.seat_note_path(id) {
            let _ = fs::remove_file(path);
        }
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
        fs::create_dir_all(&dir).map_err(msg::cant_make_session_dir)?;
        let gate = RowGate::try_acquire(&dir)?;
        if dir.join("meta.json").exists() {
            return Err(msg::session_already_exists(&id.0));
        }
        let meta = Meta {
            shell_run: None,
            kind: kind.to_owned(),
            title: title.to_owned(),
            pid,
            started_ms: now_ms(),
            gui_native: None,
            gui_run: None,
            category: category.map(str::to_owned),
            id: id.0.clone(),
            vendor_leaf: None,
            place: None,
        };
        gate.write_meta(&meta)?;
        // Whoever registers a session is starting it, so this opening
        // 忙碌 is a fact about a spawn and not a reading of anything.
        gate.write_state(State::Busy, None, Source::Reported, None)
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
    pub fn reopen(&self, id: &SessionId, kind: &str, title: &str, pid: u32) -> Result<(), String> {
        self.reseat(id, kind, title, Some(pid))
    }

    /// A source check, not an execution reservation. Known-native callers use
    /// it before spawning their ghost; begin_gui_run checks again at publish.
    pub(crate) fn check_gui_source(
        &self,
        native: &crate::session_content::journal::Identity,
    ) -> Result<(), String> {
        let id = crate::adaptor::id_for(&native.native_id);
        if native.home != self.device
            || native.agent.is_empty()
            || native.native_id.is_empty()
            || serde_json::to_vec(native).map_err(|e| e.to_string())?.len() > 4096
        {
            return Err("GUI source needs its exact home, agent and native identity".into());
        }
        let dir = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        if !dir.exists() {
            return Ok(());
        }
        recovery_attempt::check_unquarantined(&dir)?;
        let legacy = self.legacy_gui_source(&id)?;
        let gate = RowGate::try_acquire(&dir)?;
        let previous = gate.read_meta_or_new()?;
        gate.check_native(&id, previous.as_ref(), native, legacy.as_ref())?;
        if previous.as_ref().is_some_and(|meta| {
            !gui_slot_ended(meta) && meta.pid.is_some_and(crate::link::pid_running)
        }) {
            return Err("GUI row still has a live execution".into());
        }
        Ok(())
    }

    /// Publish one GUI execution without changing its logical SID or full
    /// provider identity. The later reservation slice moves upstream admission
    /// before provider spawn; this row transaction only owns publication.
    pub(crate) fn begin_gui_run(
        &self,
        id: &SessionId,
        title: &str,
        pid: u32,
        native: &crate::session_content::journal::Identity,
        category: Option<&str>,
        resume: bool,
    ) -> Result<RunKey, String> {
        if native.home != self.device
            || crate::adaptor::id_for(&native.native_id) != *id
            || native.agent.is_empty()
            || native.native_id.is_empty()
            || serde_json::to_vec(native).map_err(|e| e.to_string())?.len() > 4096
        {
            return Err("GUI execution needs its exact home, agent and native identity".into());
        }
        let dir = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        // Old external TUI rows can have no journal yet. Resolve their actual
        // unique native transcript outside the short row gate, never by merely
        // accepting another full ID with the same clean_leaf.
        let legacy = if resume {
            self.legacy_gui_source(id)?
        } else {
            None
        };
        let run_id = crate::link::fresh_hex()?;
        fs::create_dir_all(&dir).map_err(msg::cant_make_session_dir)?;
        let gate = RowGate::acquire(&dir)?;
        self.begin_gui_run_held(
            &gate, id, title, pid, native, category, resume, legacy.as_ref(), &run_id, State::Idle, None,
        )
    }

    // The Attempt publisher already owns this exact short row gate. It supplies
    // its durable run ID and opening state; this helper must not reacquire it.
    fn begin_gui_run_held(
        &self,
        gate: &RowGate,
        id: &SessionId,
        title: &str,
        pid: u32,
        native: &crate::session_content::journal::Identity,
        category: Option<&str>,
        resume: bool,
        legacy: Option<&(Option<String>, crate::session_content::journal::Identity)>,
        run_id: &str,
        word: State,
        place: Option<&Path>,
    ) -> Result<RunKey, String> {
        let dir = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        if gate.dir != dir || native.home != self.device
            || crate::adaptor::id_for(&native.native_id) != *id
        {
            return Err("GUI publication has another row gate or source".into());
        }
        recovery_attempt::check_unquarantined(&dir)?;
        let run = GuiRun {
            id: run_id.to_owned(),
            started_ms: now_ms(),
            state: LiveState {
                gui_run: Some(run_id.to_owned()),
                word,
                at_ms: now_ms(),
                exit: None,
                source: Some(Source::Reported),
            },
        };
        let previous = gate.read_meta_or_new()?;
        if previous.is_some() && !resume {
            return Err(msg::session_already_exists(&id.0));
        }
        gate.check_native(id, previous.as_ref(), native, legacy)?;
        if let Some(meta) = &previous {
            if meta
                .pid
                .is_some_and(|old| old != pid && crate::link::pid_running(old))
                && !gui_slot_ended(meta)
            {
                return Err("GUI row still has a live execution".into());
            }
        }
        let fresh = previous.is_none();
        let mut meta = previous.unwrap_or(Meta {
            shell_run: None,
            kind: khor_core::kind::GUI.into(),
            title: title.into(),
            pid: None,
            started_ms: run.started_ms,
            category: category.map(str::to_owned),
            id: id.0.clone(),
            vendor_leaf: None,
            place: None,
            gui_native: None,
            gui_run: None,
        });
        meta.kind = khor_core::kind::GUI.into();
        meta.title = title.into();
        meta.pid = Some(pid);
        meta.gui_native = Some(native.clone());
        meta.gui_run = Some(run.clone());
        if meta.place.is_none() {
            meta.place = place.map(|cwd| cwd.display().to_string());
        }
        // A fresh row keeps the legacy file shape for stored data, but this
        // generation's only live state is published inside its atomic Meta.
        // Reopening never rewrites the old state.json.
        if fresh {
            gate.write_state(word, None, Source::Reported, None)?;
        }
        gate.write_meta(&meta)?;
        gate.sync_meta()?;
        Ok(RunKey {
            session: id.clone(),
            directory: dir,
            run_id: run.id,
            native: native.clone(),
        })
    }

    fn legacy_gui_source(
        &self,
        id: &SessionId,
    ) -> Result<Option<(Option<String>, crate::session_content::journal::Identity)>, String> {
        let Some(dir) = self.dir(id) else {
            return Ok(None);
        };
        let meta = match fs::read(dir.join("meta.json")) {
            Ok(bytes) => {
                serde_json::from_slice::<Meta>(&bytes).map_err(msg::session_meta_garbled)?
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(msg::cant_read_session_meta(why)),
        };
        if meta.gui_native.is_some() || dir.join("journal.json").exists() {
            return Ok(None);
        }
        let Some(provider) = crate::session_content::Provider::of(meta.category.as_deref()) else {
            return Ok(None);
        };
        let leaf = id.0.split_once('/').map(|(_, leaf)| leaf).unwrap_or("");
        let source =
            crate::session_content::NativeTranscript::resolve(&self.root, provider, leaf, None)
                .map_err(|_| "legacy GUI source is unreadable or ambiguous".to_owned())?;
        let Some(source) = source else {
            return Ok(None);
        };
        let source = crate::session_content::ContentSource::Native(source);
        let (_, agent, native_id) = source.observation_identity();
        Ok(Some((
            meta.category,
            crate::session_content::journal::Identity {
                home: self.device,
                agent: agent.into(),
                native_id: native_id.into(),
            },
        )))
    }

    pub(crate) fn report_run(
        &self,
        key: &RunKey,
        word: State,
        source: Source,
    ) -> Result<(), String> {
        if word == State::Failed {
            return Err(msg::failed_is_not_reportable().into());
        }
        self.with_run(key, |gate, meta, state| {
            if state.exit.is_some() || !pid_says_alive(meta) {
                return Err(msg::session_over(&key.session.0));
            }
            gate.write_state(word, None, source, Some(&key.run_id))
        })
    }

    pub(crate) fn record_exit_run(&self, key: &RunKey, code: i32) -> Result<(), String> {
        self.with_run(key, |gate, _, state| {
            if state.exit.is_some() {
                return Ok(());
            }
            gate.write_state(
                state.word,
                Some(code),
                state.source.unwrap_or(Source::Reported),
                Some(&key.run_id),
            )
        })
    }

    fn with_run<T>(
        &self,
        key: &RunKey,
        write: impl FnOnce(&RowGate, &Meta, &LiveState) -> Result<T, String>,
    ) -> Result<T, String> {
        let gate = self.run_gate(key)?;
        let (meta, state) = gate.read_pair()?;
        check_run(&meta, key)?;
        write(&gate, &meta, &state)
    }

    fn run_gate(&self, key: &RunKey) -> Result<RowGate, String> {
        let dir = self.existing_dir(&key.session)?;
        if dir != key.directory {
            return Err("GUI run key belongs to another row".into());
        }
        RowGate::acquire(&dir)
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
        let gate = RowGate::acquire(&dir)?;
        let mut meta = gate.read_meta_or_new()?.unwrap_or(Meta {
            shell_run: None,
            kind: kind.to_owned(),
            title: title.to_owned(),
            pid: None,
            started_ms: now_ms(),
            gui_native: None,
            gui_run: None,
            category: None,
            id: id.0.clone(),
            vendor_leaf: None,
            place: None,
        });
        if meta.gui_run.is_some() {
            if kind == khor_core::kind::GUI || meta.pid.is_none_or(crate::link::pid_running) {
                return Err("GUI execution requires its current run key".into());
            }
            meta.gui_run = None;
        }
        meta.kind = kind.to_owned();
        meta.title = title.to_owned();
        if let Some(pid) = pid {
            meta.pid = Some(pid);
        }
        meta.id = id.0.clone();
        // The old GUI Meta remains a complete snapshot until the new legacy
        // state is ready; only then is its run key invalidated by publication.
        gate.write_state(State::Idle, None, Source::Reported, None)?;
        gate.write_meta(&meta)
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
        let gate = RowGate::try_acquire(&dir)?;
        let mut meta = gate.read_meta()?;
        if meta.vendor_leaf.is_some() {
            return Ok(());
        }
        meta.vendor_leaf = Some(leaf.to_owned());
        gate.write_meta(&meta)
    }

    /// The vendor's session running in this row's host **now**, as the
    /// sweep's pid join reads it: unlike [`LiveKind::learn_vendor_leaf`]
    /// it replaces a different leaf on record. A `/clear`, or any new
    /// session in the same host, is the conversation on screen, and the
    /// first leaf names a transcript nobody writes any more.
    pub fn follow_vendor_leaf(&self, id: &SessionId, leaf: &str) -> Result<(), String> {
        if leaf.is_empty() {
            return Ok(());
        }
        let dir = self.existing_dir(id)?;
        let gate = RowGate::try_acquire(&dir)?;
        let mut meta = gate.read_meta()?;
        if meta.vendor_leaf.as_deref() == Some(leaf) {
            return Ok(());
        }
        meta.vendor_leaf = Some(leaf.to_owned());
        gate.write_meta(&meta)
    }

    /// Writes down where khor started this session. Write-once, like
    /// `learn_vendor_leaf`: it is a fact about the session's birth, and
    /// nothing later is a correction of it.
    ///
    /// **Said by the opener rather than taken from `register`**, which
    /// is the same shape `learn_category` has: most registrations know
    /// no directory (a hook's parent shell is not where anything runs),
    /// and a required argument would have every one of them passing a
    /// guess.
    pub fn learn_place(&self, id: &SessionId, cwd: &std::path::Path) -> Result<(), String> {
        let dir = self.existing_dir(id)?;
        let gate = RowGate::try_acquire(&dir)?;
        let mut meta = gate.read_meta()?;
        if meta.place.is_some() {
            return Ok(());
        }
        meta.place = Some(cwd.display().to_string());
        gate.write_meta(&meta)
    }

    /// **Every place khor itself started a session in**, for the
    /// wizard's 常去的地方 (`crate::place`): one entry per registry
    /// row that has a birthplace on record, `(cwd, 1, when it started)`.
    /// The vendors answer for conversations; this is the half only khor
    /// knows — a directory somebody only ever opened a terminal in.
    pub fn places(&self) -> Vec<(PathBuf, u32, i64)> {
        let Ok(rd) = fs::read_dir(sessions_dir(&self.root)) else {
            return Vec::new();
        };
        rd.flatten()
            .filter_map(|e| read_meta(&e.path()).ok())
            .filter_map(|m| Some((PathBuf::from(m.place?), 1, m.started_ms)))
            .collect()
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
        let gate = RowGate::try_acquire(&dir)?;
        let mut meta = gate.read_meta()?;
        if meta.category.is_some() {
            return Ok(());
        }
        meta.category = Some(category.to_owned());
        gate.write_meta(&meta)
    }

    /// Replaces the recorded pid — the wrapper registers before it knows
    /// the child's pid so hooks firing at startup find the session.
    pub fn set_pid(&self, id: &SessionId, pid: u32) -> Result<(), String> {
        let dir = self.existing_dir(id)?;
        let gate = RowGate::acquire(&dir)?;
        let mut meta = gate.read_meta()?;
        if meta.kind == khor_core::kind::GUI || meta.gui_run.is_some() {
            // PID publication is not an implicit GUI-to-terminal transition.
            // The explicit takeover must first retire that GUI run via reseat.
            return Err("GUI execution requires its current run key".into());
        }
        meta.pid = Some(pid);
        gate.write_meta(&meta)
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
            return Err(msg::failed_is_not_reportable().into());
        }
        let dir = self.existing_dir(id)?;
        let gate = RowGate::try_acquire(&dir)?;
        let (meta, state) = gate.read_pair()?;
        if meta.gui_run.is_some() {
            return Err("GUI execution requires its current run key".into());
        }
        if state.exit.is_some() || !pid_says_alive(&meta) {
            return Err(msg::session_over(&id.0));
        }
        gate.write_state(word, None, source, None)
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
        let gate = RowGate::acquire(&dir)?;
        let (meta, state) = gate.read_pair()?;
        if meta.gui_run.is_some() {
            return Err("GUI execution requires its current run key".into());
        }
        if state.exit.is_some() {
            return Ok(());
        }
        let source = state.source.unwrap_or(Source::Reported);
        gate.write_state(state.word, Some(code), source, None)
    }

    /// The row's current stamp — what "looked at now" covers.
    pub fn stamp(&self, id: &SessionId) -> Result<i64, String> {
        let dir = self.existing_dir(id)?;
        Ok(read_pair(&dir)?.1.at_ms)
    }

    fn discover_registered(&self, registered: &[RegistryRow]) -> crate::adaptor::Findings {
        let mut found = self.discovery.sweep();
        let terminals: Vec<_> = registered
            .iter()
            .filter_map(|r| {
                Some((
                    r.host_child?,
                    r.row.doing.as_ref().and_then(|d| d.title.clone()),
                ))
            })
            .collect();
        crate::codex_terminal::select(&mut found, &terminals);
        found
    }

    fn discover(&self) -> crate::adaptor::Findings {
        self.discover_registered(&self.registry_rows(&|_| 0))
    }

    /// The live sighting behind an id, for the takeover: the title the
    /// vendor gave it and the processes to end. `None` when nothing is
    /// running behind it — then there is nothing to kill, only a record
    /// to resume.
    pub fn sighting_of(&self, id: &SessionId) -> Option<(String, Vec<u32>)> {
        self.discover()
            .rows
            .into_iter()
            .find(|f| f.id() == *id)
            .map(|f| (f.sighting.title, f.sighting.pids))
    }

    /// Whether this row is **a terminal khor hosts** — a shell a person
    /// types into, and so a place an agent can be started *inside*. A
    /// hook arriving with this row's `KHOR_SESSION` is that agent's, not
    /// the terminal's (docs/SESSION.md 形态随进程变). A `khor run -- …`
    /// is not one: it has no host, its process *is* the command, and a
    /// hook from it is the command speaking for itself.
    pub fn is_a_terminal(&self, id: &SessionId) -> bool {
        self.dir(id).is_some_and(|dir| {
            read_meta(&dir).is_ok_and(|m| m.kind == khor_core::kind::SHELL)
                && crate::host::read_host_file(&dir).is_ok()
        })
    }

    fn owns_agent_endpoint(&self, id: &SessionId, procs: &crate::adaptor::Procs) -> bool {
        let Some(dir) = self.dir(id) else {
            return false;
        };
        let Ok((meta, state)) = read_pair(&dir) else {
            return false;
        };
        let Ok(host) = crate::host::read_host_file(&dir) else {
            return false;
        };
        let Some(process) = procs.get(host.host_pid) else {
            return false;
        };
        let written_ms = crate::host::host_file_path(&dir)
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|time| time.as_millis());
        if state.exit.is_some()
            || meta.pid != Some(host.host_pid)
            || written_ms.is_none_or(|written| {
                process.started_ms < 0 || process.started_ms as u128 > written
            })
        {
            return false;
        }
        if meta.kind == khor_core::kind::GUI && host.child_pid == host.host_pid {
            return true;
        }
        let Some(child) = procs.get(host.child_pid) else {
            return false;
        };
        child.ppid == Some(host.host_pid)
    }

    /// The kind and title on record, when a registry entry exists.
    pub fn on_record(&self, id: &SessionId) -> Option<(String, String)> {
        let dir = self.dir(id)?;
        read_meta(&dir).ok().map(|m| (m.kind, m.title))
    }

    /// Whether this row's process sits in a tmux session right now —
    /// the fact [`khor_core::Session::mux`] carries, asked afresh at
    /// the moment a terminal is being opened (one sweep, ~13 ms).
    fn held_in_mux(&self, id: &SessionId) -> bool {
        let sweep = self.discover();
        let pids: Vec<u32> = if let Some(found) = sweep.rows.iter().find(|f| f.id() == *id) {
            found.sighting.pids.clone()
        } else if let Ok(dir) = self.existing_dir(id) {
            read_meta(&dir)
                .ok()
                .and_then(|m| m.pid)
                .filter(|pid| !crate::codex_terminal::scoped(&sweep.procs, *pid))
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        pids.iter()
            .any(|p| sweep.multiplexed.iter().any(|h| h.holds.contains(p)))
    }

    /// A clicked imported agent ID may now hold its original tmux terminal
    /// in shell form. This permits terminal access, never an agent-thread join.
    fn holds_terminal_form(&self, id: &SessionId) -> bool {
        self.seated(&|_| 0, &|sid| sid == id.0)
            .rows
            .iter()
            .any(|row| row.id == *id && row.kind.0 == khor_core::kind::SHELL && row.term && row.mux)
    }

    pub(crate) fn require_input_owner(&self, id: &SessionId) -> Result<(), String> {
        crate::session_content::refuse_control(id)?;
        let current = self.seated(&|_| 0, &|_| false);
        if current.input_owners.contains(id) {
            return Ok(());
        }
        if current.rows.iter().any(|row| row.id == *id) {
            return Err(
                "This agent runs outside Khor; import it for observation or resume explicitly"
                    .into(),
            );
        }
        Err(msg::no_such_session(&id.0))
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
    pub fn attach_multiplexed(&self, id: &SessionId) -> Result<Seat, String> {
        self.require_input_owner(id)?;
        self.attach_owned(id)
    }

    fn attach_owned(&self, id: &SessionId) -> Result<Seat, String> {
        let resource = id.0.starts_with("shell/")
            || self
                .on_record(id)
                .is_some_and(|(kind, _)| kind == khor_core::kind::SHELL);
        if !resource {
            let dir = self
                .dir_of(id)
                .ok_or_else(|| msg::not_a_session_id(&id.0))?;
            let meta = read_meta(&dir)?;
            if meta.kind == khor_core::kind::TUI
                && crate::host::read_host_file(&dir)
                    .is_ok_and(|host| crate::link::pid_alive(host.host_pid))
            {
                return Ok(Seat::Host);
            }
            return Err(
                "This agent has no owned terminal; an external terminal view is read-only".into(),
            );
        }
        if let Some(dir) = self.dir_of(id) {
            // A viewer already up is the answer for an agent in tmux; a
            // host already up is the answer for a tmux session khor
            // bridged as a shell. Checked in that order, because a row
            // can have *both* a host and a tmux seat (a conversation
            // khor started and the person resumed in a terminal), and
            // then the terminal is the viewer's, not the ghost's.
            if let Ok(vf) = crate::host::read_view_file(&dir) {
                if crate::link::pid_alive(vf.host_pid) {
                    return Ok(Seat::View);
                }
                let _ = fs::remove_file(crate::host::view_file_path(&dir));
            }
            if let Ok(hf) = crate::host::read_host_file(&dir) {
                if crate::link::pid_alive(hf.host_pid) {
                    // A live host is the row's own terminal (T-34, the
                    // terminal audit), and its marker is never removed:
                    // it is the only way back to that process. Asking the
                    // panes first once seated a khor-started shell inside
                    // the tmux pane its bridge descended from, deleted
                    // the live host's marker, and sent the shell's keys
                    // into that pane. Only a conversation ghost whose
                    // agent was resumed in a pane goes on to the viewer
                    // below, its host left standing for the chat.
                    let shell = self
                        .on_record(id)
                        .is_some_and(|(kind, _)| kind == khor_core::kind::SHELL);
                    if shell || !self.held_in_mux(id) {
                        return Ok(Seat::Host);
                    }
                } else {
                    // A host that died without cleaning up (killed
                    // hard): the marker points at nothing, so clear it
                    // and stand a fresh one up rather than answering
                    // "attached" about a corpse.
                    let _ = fs::remove_file(crate::host::host_file_path(&dir));
                }
            }
        }
        let tmux = self
            .discovery
            .tmux()
            .ok_or_else(|| msg::no_terminal_here(&id.0))?;
        // Confirmed against a live sweep, not just parsed: a tmux leaf
        // also carries the session's creation time, so a same-numbered
        // session on a restarted server does not impersonate the clicked
        // one — and an agent row's route exists only while the sweep can
        // still see its process inside a session.
        let sweep = self.discover();
        // Every attach below turns the mouse on for khor's own client
        // (`Tmux::grouped_attach_argv`, so the wheel scrolls tmux's
        // history). A tmux with no `mouse` option attaches all the same
        // — the setting is `-q` — and this line is what is left to say
        // why its wheel is still ↑ ↓.
        if !tmux.knows_mouse() {
            eprintln!("{}", msg::tmux_no_mouse_option(&id.0));
        }
        let shell_prefix = format!("{}/", khor_core::kind::SHELL);
        if let Some(target) =
            id.0.strip_prefix(&shell_prefix)
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
            self.ensure(
                id,
                khor_core::kind::SHELL,
                &title,
                None,
                Some(khor_core::category::SHELL),
            )?;
            let dir = self
                .dir_of(id)
                .ok_or_else(|| msg::not_a_session_id(&id.0))?;
            // Panes opened from khor's client start their shell marking
            // its commands (`Tmux::shell_shim`); the scripts are written
            // first, as a host starting a shell writes them.
            let scripts = crate::integration::dir(&self.root.join(".khor"));
            let shim = crate::integration::write_scripts(&scripts)
                .ok()
                .and_then(|()| tmux.shell_shim(&scripts));
            crate::host::spawn_host(
                &dir,
                id,
                &tmux.grouped_attach_argv(&id.0, &target, None, shim.as_deref()),
                (80, 24),
            )?;
            return Ok(Seat::Host);
        }
        // **A row whose agent has left the tmux session it sat in** —
        // it holds that session's seat now, in the terminal's form
        // (docs/SESSION.md 形态随进程变), and its terminal is the session
        // itself. Nothing narrows the view to a window: the process that
        // named one is gone. The row keeps its registry dir, so the lens
        // cleans up only its own file when it ends.
        //
        // **An agent nobody registered** remembers its seat in a note
        // (`SeatNote`) instead. The lens needs a registry dir to hang on,
        // so one is minted here — the discovered path's rule below — and
        // taken away with the lens, after which the note seats the row
        // again exactly as before.
        let noted = (!self.claims(id)).then(|| self.seat_note(id)).flatten();
        if (!sweep.rows.iter().any(|f| f.id() == *id) || self.holds_terminal_form(id))
            && let Some(dir) = self.dir_of(id)
            && let Some(seat) = read_seat(&dir).or_else(|| noted.as_ref().map(|n| n.seat.clone()))
            && sweep.multiplexed.iter().any(|h| h.found.id().0 == seat)
            && let Some(target) = seat
                .strip_prefix(&shell_prefix)
                .and_then(crate::adaptor::tmux::tmux_target_of)
        {
            let minted = match &noted {
                Some(n) if !self.claims(id) => {
                    self.ensure(
                        id,
                        khor_core::kind::TUI,
                        &n.title,
                        None,
                        n.category.as_deref(),
                    )?;
                    // Nothing was spawned: the opening 忙碌 `register`
                    // writes is demoted, as for a discovered row below.
                    let _ = self.report(id, State::Idle, Source::Screen);
                    note_seat(&dir, Some(&seat));
                    true
                }
                _ => false,
            };
            crate::host::spawn_viewer_host(
                &dir,
                id,
                &tmux.grouped_attach_argv(&id.0, &target, None, None),
                (80, 24),
                minted,
            )?;
            return Ok(Seat::View);
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
                let pid = meta
                    .pid
                    .filter(|pid| !crate::codex_terminal::scoped(&sweep.procs, *pid))
                    .ok_or_else(|| msg::no_terminal_here(&id.0))?;
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
        let dir = self
            .dir_of(id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        crate::host::spawn_viewer_host(
            &dir,
            id,
            &tmux.grouped_attach_argv(&id.0, &target, window.as_deref(), None),
            (80, 24),
            fresh,
        )?;
        Ok(Seat::View)
    }

    /// **Moves to the shell prompt before or after the one in view** in
    /// the pane a tmux row's client has in front (item 21,
    /// `Tmux::jump_prompt`): there tmux holds the screen and its history,
    /// so the face asks tmux.
    pub fn jump_prompt(&self, id: &SessionId, up: bool) -> Result<(), String> {
        let row = self
            .discovery
            .tmux()
            .ok_or_else(|| msg::no_terminal_here(&id.0))
            .and_then(|t| t.jump_prompt(&id.0, up));
        // A shell with the person's own tmux client in front (item 58).
        match row {
            Err(e) => match self.tmux_in_shell(id) {
                Some(c) => c.server().jump_prompt_in(&c.session_id, &id.0, up),
                None => Err(e),
            },
            ok => ok,
        }
    }

    /// **A person's own tmux client in front of this row's terminal**
    /// (item 58, `adaptor::tmux::client_in_front`): a khor shell in which
    /// they typed `tmux`, and the session that client is attached to.
    /// `None` when the row has no running host here, or its terminal's
    /// foreground is anything else — the moment the client leaves
    /// (`exit`, a detach), the row is a plain shell again.
    pub fn tmux_in_shell(&self, id: &SessionId) -> Option<crate::adaptor::tmux::ClientInFront> {
        let dir = self.dir_of(id)?;
        let hf = crate::host::read_host_file(&dir).ok()?;
        if !crate::link::pid_running(hf.host_pid) {
            return None;
        }
        crate::adaptor::tmux::client_in_front(hf.child_pid)
    }

    /// **The wheel over a shell's own tmux client** (item 58,
    /// `Tmux::wheel_in`): `rows` up when positive, in the session the
    /// client in front is attached to. `Ok(false)` when no tmux client is
    /// in front: the face keeps the wheel for itself. `rows` 0 only asks.
    pub fn tmux_wheel(&self, id: &SessionId, rows: i32) -> Result<bool, String> {
        let Some(c) = self.tmux_in_shell(id) else {
            return Ok(false);
        };
        c.server().wheel_in(&c.session_id, &id.0, rows)?;
        Ok(true)
    }

    /// **The history of the pane a tmux row's client has in front**
    /// (item 31, `Tmux::history`), from `since` on: what the phone scrolls
    /// locally instead of tmux's copy mode.
    pub fn tmux_history(
        &self,
        id: &SessionId,
        since: Option<&crate::adaptor::tmux::HistoryAnchor>,
    ) -> Result<crate::adaptor::tmux::PaneHistory, String> {
        let row = self
            .discovery
            .tmux()
            .ok_or_else(|| msg::no_terminal_here(&id.0))
            .and_then(|t| t.history(&id.0, since));
        // A shell with the person's own tmux client in front (item 58):
        // the history of the session that client is attached to.
        match row {
            Err(e) => match self.tmux_in_shell(id) {
                Some(c) => c.server().history_in(&c.session_id, &id.0, since),
                None => Err(e),
            },
            ok => ok,
        }
    }

    /// **The address a face attaches to for this row, on the machine
    /// that hosts it** — what a far machine's `Reach` is answered with,
    /// and this machine's own reach of itself.
    ///
    /// The row's own host when one is running (a conversation's ghost, a
    /// khor terminal, a tmux session bridged as a shell) — the answer it
    /// always was. Otherwise the bridge stands one up
    /// ([`Self::attach_multiplexed`]), and **the seat it answers names
    /// the file**: an agent sitting in the person's tmux is reached
    /// through a viewer, and a viewer's address is `view.json`, never
    /// `host.json` (`crate::host::view_file_path` has why they are two).
    ///
    /// Reading `host.json` whatever the seat was is how a phone opening
    /// a Claude Code session that lived in tmux on a Mac was told the
    /// session had no host (2026-10-02, `no-host`): the Mac stood its
    /// viewer up, wrote `view.json`, and then looked for the other file.
    /// The Mac's own window never hit it — its terminal door already
    /// read the seat (`khor_gui_core::term::term_open`).
    ///
    /// Blocking: the bridge sweeps and may start a host.
    pub fn terminal_address(&self, id: &SessionId) -> Result<crate::host::HostFile, String> {
        self.input_address(id, AddressPreference::Host)
    }

    /// A terminal client uses the current seat, including a view beside a GUI host.
    pub(crate) fn terminal_view_address(
        &self,
        id: &SessionId,
    ) -> Result<crate::host::HostFile, String> {
        self.input_address(id, AddressPreference::Seat)
    }

    fn input_address(
        &self,
        id: &SessionId,
        preference: AddressPreference,
    ) -> Result<crate::host::HostFile, String> {
        self.require_input_owner(id)?;
        if matches!(preference, AddressPreference::Host)
            && let Some(dir) = self.dir_of(id)
            && let Ok(host) = crate::host::read_host_file(&dir)
            && crate::link::pid_running(host.host_pid)
        {
            return Ok(host);
        }
        let seat = self.attach_owned(id)?;
        let dir = self
            .dir_of(id)
            .ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let hf = match seat {
            Seat::View => crate::host::read_view_file(&dir)?,
            Seat::Host => crate::host::read_host_file(&dir)?,
        };
        // **A host file outlives its host.** An address whose far end is
        // a closed port drops the pipe, and the attach then reports a
        // socket error for a session that simply ended.
        if !crate::link::pid_alive(hf.host_pid) {
            return Err(msg::host_gone().into());
        }
        Ok(hf)
    }

    /// Closes bridge hosts the seating in [`LiveKind::rows`] folded while
    /// they still run — the cleanup half of that judgment, on the serve
    /// tick like `Node::reap_borrows`: a listing can hide a row, but only
    /// this stops the process behind it. Killing the host is a tmux
    /// detach; the user's own session is never touched.
    ///
    /// **Since 2026-09-30 this finds nothing, and that is the rule.** It
    /// used to close every bridge whose tmux session held a listed agent
    /// (one running process, one row — the agent's), which closed the
    /// terminal a person had open the moment they started claude in it.
    /// Now a row with a terminal open holds its host's seat before
    /// anything else (docs/SESSION.md 形态随进程变), so a running bridge
    /// is never the one folded. The reaper asks the listing's own
    /// question rather than a second copy of it, so the two cannot drift
    /// apart if that order ever changes.
    pub fn reap_folded_bridges(&self) {
        // This runs on every serve tick, and only a bridge with a running
        // host can be reaped — so the whole sweep is asked only when one
        // exists, which is what this cost before the seating too.
        let shell_prefix = format!("{}/", khor_core::kind::SHELL);
        let any_open_bridge = self.registry_rows(&|_| 0).iter().any(|r| {
            r.open
                && r.row
                    .id
                    .0
                    .strip_prefix(&shell_prefix)
                    .is_some_and(crate::adaptor::tmux::is_tmux_leaf)
        });
        if !any_open_bridge {
            return;
        }
        for id in self.seated(&|_| 0, &|_| false).folded_bridges {
            let _ = self.close_session(&id);
        }
    }

    /// Kills the process if it still runs, then forgets the session.
    /// 失败沉底 is a list fact, not a registry fact — closing is how a
    /// settled row leaves the list.
    /// Ends khor's own **lens** on a session, if it has one — the viewer
    /// host a terminal face stood up to bridge into somebody else's
    /// multiplexer or agent (`host::spawn_viewer_host`) — and nothing
    /// else: the session, its process, its registry entry and its
    /// vendor's files stay. What 删除 on an imported row does beside
    /// taking it off the list (docs/SHELL.md 第八轮 第 3 条).
    pub fn drop_lens(&self, id: &SessionId) {
        let Some(dir) = self.dir_of(id) else { return };
        if let Ok(vf) = crate::host::read_view_file(&dir)
            && crate::link::pid_alive(vf.host_pid)
        {
            terminate(vf.host_pid);
        }
    }

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
            if state.exit.is_none()
                && pid_says_alive(&meta)
                && let Some(pid) = meta.pid
            {
                terminate(pid);
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
    ///
    /// **And a host is one row, wearing what runs in it now**
    /// (docs/SESSION.md 形态随进程变, 2026-09-30) — the seating below,
    /// which replaced the rule that a multiplexer session holding a
    /// listed agent simply stopped being a row.
    pub fn rows(&self, watermark: impl Fn(&str) -> i64) -> Vec<Session> {
        self.rows_listed(watermark, |_| false)
    }

    /// [`LiveKind::rows`], knowing which rows are **on the list** — the
    /// second tier of who holds a host's seat. The list is the node's
    /// (`khor_sync::roster`), so it is asked for rather than read here:
    /// this kind never opens that document, and a row the person put on
    /// their list must not lose its seat to one they never did (an agent
    /// found inside it is not on the list until somebody imports it, so
    /// losing the seat to it was losing the row altogether).
    pub fn rows_listed(
        &self,
        watermark: impl Fn(&str) -> i64,
        listed: impl Fn(&str) -> bool,
    ) -> Vec<Session> {
        self.seated(&watermark, &listed).rows
    }

    /// The agent row whose form `id` is wearing, if it wears one — the
    /// conversation a press on it acts on (接管 on a terminal that has a
    /// claude in it takes over **that claude**).
    pub fn worn_by(&self, id: &SessionId, listed: impl Fn(&str) -> bool) -> Option<SessionId> {
        self.seated(&|_| 0, &listed)
            .worn
            .into_iter()
            .find(|(holder, _)| holder == id)
            .map(|(_, agent)| agent)
    }

    /// What 「looked at now」 covers for a row as it is **shown**: its own
    /// stamp, or the stamp of the form it wears if that is newer. Without
    /// the second half a 完成 worn by a terminal could never be cleared —
    /// looking at the terminal would mark the terminal's older stamp.
    pub fn shown_stamp(
        &self,
        id: &SessionId,
        listed: impl Fn(&str) -> bool,
    ) -> Result<i64, String> {
        let own = self.stamp(id)?;
        let shown = self
            .seated(&|_| 0, &listed)
            .rows
            .into_iter()
            .find(|r| r.id == *id)
            .and_then(|r| r.state.map(|stamp| stamp.at.0 as i64));
        Ok(shown.map_or(own, |s| s.max(own)))
    }

    fn seated(&self, watermark: &impl Fn(&str) -> i64, listed: &impl Fn(&str) -> bool) -> Seated {
        let registered = self.registry_rows(watermark);
        let mut out: Vec<Session> = registered.iter().map(|r| r.row.clone()).collect();
        // Each row's word before 完成 was settled, beside `out` — a row
        // that wears another's form settles it against its own watermark
        // (`RegistryRow::raw`).
        let mut raw: Vec<State> = registered.iter().map(|r| r.raw).collect();
        let unpinned: Vec<&SessionId> = registered
            .iter()
            .filter(|r| r.pid.is_none())
            .map(|r| &r.row.id)
            .collect();
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
        let sweep = self.discover_registered(&registered);
        // Imported rows may have recorded this client's PID before an in-TUI
        // thread switch. Native selection, not that old pin, owns it now.
        seat_of_pid.retain(|pid, _| !crate::codex_terminal::scoped(&sweep.procs, *pid));
        let selection_hosts: std::collections::BTreeSet<_> = registered
            .iter()
            .filter(|r| {
                r.host_child.is_some_and(|pid| {
                    sweep
                        .procs
                        .held_by(&[pid])
                        .iter()
                        .any(|p| crate::codex_terminal::scoped(&sweep.procs, *p))
                })
            })
            .map(|r| r.row.id.0.clone())
            .collect();
        let bound: std::collections::BTreeSet<_> = sweep
            .rows
            .iter()
            .filter(|r| r.category == "codex")
            .flat_map(|r| r.sighting.pids.iter().copied())
            .collect();
        let unbound: std::collections::BTreeSet<_> = sweep
            .procs
            .named("codex")
            .into_iter()
            // Selection already excludes suspended jobs. They must not veto a
            // runnable client's bound host again in the final shell fallback.
            .filter(|(pid, _)| {
                crate::codex_terminal::scoped(&sweep.procs, *pid)
                    && !sweep.procs.suspended(*pid)
                    && !bound.contains(pid)
            })
            .map(|(pid, _)| pid)
            .collect();
        self.unmapped.store(sweep.unmapped, Ordering::Relaxed);
        // Vendor leaves learned this poll, for the preview post-pass:
        // `registered` was read before they were written.
        let mut learned: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut joined: Vec<bool> = vec![false; registered.len()];
        // What each live hosted agent row's host holds, taken once: one
        // subtree walk per such row per poll, on the table the sweep took.
        let hosted_holds: Vec<(usize, Vec<u32>)> = registered
            .iter()
            .enumerate()
            .filter(|(_, r)| r.row.kind.0 == khor_core::kind::TUI && !r.row.dead)
            .filter_map(|(i, r)| Some((i, sweep.procs.subtree(&[r.host_child?]))))
            .collect();
        for found in sweep.rows {
            let found_id = found.id();
            let kind = found.kind;
            let category = found.category;
            let sighting = found.sighting;
            let pids = sighting.pids.clone();
            // **A hosted agent row is the sighting whose pid runs under
            // its host** (SESS-BRIEF 2b). `khor run --tui claude` and
            // `--tui codex` mint their own id, and nothing names the
            // vendor's session to them: codex cannot be told one at
            // start, and no hook of khor's is in the owner's Claude
            // settings. The sweep joined the process to its transcript;
            // the process descends from the row's host, so the row is
            // that session — it records the leaf once (the preview reads
            // the transcript through it) and wears the sighting instead
            // of the sighting standing as a second row. A new session in
            // the same host (`/clear`) is the one on screen: the leaf on
            // record follows it.
            let by_id = out.iter().position(|r| r.id == found_id);
            let hosted = by_id.is_none().then(|| {
                hosted_holds
                    .iter()
                    .find(|(_, holds)| holds.iter().any(|p| pids.contains(p)))
                    .map(|&(i, _)| i)
            });
            let leaf = clean_leaf(&sighting.vendor_session_id);
            if let Some(Some(seat)) = hosted
                // One session per host per poll, the first the sweep
                // gives: a second claude under the same host must not
                // have the leaf flip back and forth.
                && !std::mem::replace(&mut joined[seat], true)
                && registered[seat].vendor_leaf.as_deref() != Some(leaf.as_str())
            {
                let row = &registered[seat].row;
                let _ = self.follow_vendor_leaf(&row.id, &leaf);
                if row.category.is_none() {
                    let _ = self.learn_category(&row.id, Some(category));
                }
                learned.insert(row.id.0.clone(), leaf);
            }
            let id = match hosted.flatten() {
                Some(seat) => out[seat].id.clone(),
                None => found_id,
            };
            if let Some(seat) = by_id.or(hosted.flatten()) {
                for &p in &pids {
                    seat_of_pid.insert(p, seat);
                }
                // A sighting is a voucher: the vendor's own file is
                // being read for this row, so its word is no orphan —
                // whatever `face` said from the pid-less registry entry
                // (词的保鲜期批: only truly signal-less rows may fade).
                out[seat].unbacked = false;
                // The registered row wins, but it may have been registered
                // by something that could not name the vendor (a bare
                // `khor run --tui`). The sweep read that vendor's own
                // files, so it can — and an empty category is a gap to
                // fill, not an answer to respect.
                if out[seat].category.is_none() {
                    out[seat].category = Some(found.category.to_owned());
                }
                // Same shape, same reason: khor notes where it started
                // a session, but a session it did not start (or one
                // registered by a hook, whose parent shell is nowhere in
                // particular) has no note — and the vendor's own file
                // does. A place already on record is never overwritten;
                // it is the birth fact, and this is a reading of now.
                if out[seat].place.is_none() {
                    out[seat].place = place_of(sighting.cwd.as_deref());
                }
                // **A report from a process that is not the one the
                // vendor's file describes does not outrank that file.**
                // `Reported` means "first-hand", and it is — for the
                // process that wrote it. When the registry's pid is a
                // different process from every live one the sighting
                // accounts for, the report is about somebody else: a
                // ghost that *resumed* this session while the person's
                // own claude kept running (found live, 2026-09-01: the
                // reopen wrote 空闲 for a session that was busy in its
                // terminal the whole time, and 空闲 stood for as long as
                // the ghost lived — a reporter that has since died is
                // the same case). A pid-less report (a hook's) keeps
                // its rank: the hook and the file are one process, and
                // the hook is the fresher of the two by design.
                let other_reporter =
                    !pids.is_empty() && registered[seat].pid.is_some_and(|p| !pids.contains(&p));
                if guessed.contains(&&id) || other_reporter {
                    // **The registry does not always win — the stronger
                    // source does.** This row's word was read off the
                    // screen (or reported by a process that is not the
                    // one running it), and the vendor just told us in
                    // its own files. Letting the guess stand here would
                    // put the bottom of the four-family order on top of
                    // the family it is a fallback *for*.
                    let (state, unread) =
                        settle_done(sighting.word, sighting.at_ms, watermark(&id.0));
                    out[seat].state = Some(StateStamp {
                        state,
                        at: Millis(sighting.at_ms.max(0) as u64),
                    });
                    out[seat].unread = unread;
                    raw[seat] = sighting.word;
                } else if sighting.word == State::Failed && unpinned.contains(&&id) {
                    out[seat].state = Some(StateStamp {
                        state: State::Failed,
                        at: Millis(sighting.at_ms.max(0) as u64),
                    });
                    out[seat].unread = 0;
                    raw[seat] = State::Failed;
                }
                continue;
            }
            let (state, unread) = settle_done(sighting.word, sighting.at_ms, watermark(&id.0));
            raw.push(sighting.word);
            out.push(Session {
                observation: None,
                can_control: false,
                id,
                kind: Kind(kind.to_owned()),
                title: sighting.title,
                home: self.device,
                state: Some(StateStamp {
                    state,
                    at: Millis(sighting.at_ms.max(0) as u64),
                }),
                unread,
                // A discovered 失败 is the crash rule: the status file
                // outlived its process, so nothing is behind this row
                // any more. Every other sighting is a live process.
                dead: sighting.word == State::Failed,
                // A sighting IS the voucher: the vendor's file was read
                // this very sweep.
                unbacked: false,
                category: Some(found.category.to_owned()),
                last: None,
                via: None,
                term: false,
                place: place_of(sighting.cwd.as_deref()),
                mux: false,
                doing: None,
            });
            let seat = out.len() - 1;
            for &p in &pids {
                seat_of_pid.insert(p, seat);
            }
        }
        // Where each row was last seen sitting: a registered row's own
        // `seat` file, and for an agent nobody registered, its note
        // (`SeatNote`). A note for a row that has since been registered
        // is the registry's business now, and is not read.
        let notes: Vec<SeatNote> = self
            .seat_notes()
            .into_iter()
            .filter(|n| !registered.iter().any(|r| r.row.id.0 == n.id))
            .collect();
        let mut last_seat: Vec<Option<String>> = out
            .iter()
            .enumerate()
            .map(|(i, r)| match registered.get(i) {
                Some(reg) => reg.seat.clone(),
                None => notes
                    .iter()
                    .find(|n| n.id == r.id.0)
                    .map(|n| n.seat.clone()),
            })
            .collect();
        // ── the seating (docs/SESSION.md 形态随进程变) ─────────────
        //
        // **One host, one row, wearing what runs in the host now.** A
        // host is a tmux session or khor's own pty, and what is inside it
        // is read off the process tree the sweep was answered from
        // (`Findings::procs`). Every row a host touches is one of three:
        // the host's own row (a bridge, a hosted terminal, the tmux
        // session as found), a row whose live process sits inside it, or
        // an agent row last seen in it whose agent is gone. One of them
        // holds the seat and wears the host's current form; the rest
        // fold. Until 2026-09-30 the answer was fixed one way round — an
        // agent inside always won, a bridge row folded into it and its
        // host was reaped (the terminal a person was typing into closed
        // under them) — and an agent that left took its row with it: a
        // dead registry row went on saying 空闲 over a tmux session that
        // was still there, and the session came back as a second row.
        let procs = &sweep.procs;
        let mut live_pids: Vec<Vec<u32>> = vec![Vec::new(); out.len()];
        for (&p, &i) in &seat_of_pid {
            if procs.get(p).is_some() {
                live_pids[i].push(p);
            }
        }
        let shell_prefix = format!("{}/", khor_core::kind::SHELL);
        let mut hosts: Vec<Host> = Vec::new();
        // khor's own terminals, each holding its child's subtree. A
        // bridge is not one: its pty runs a tmux client, and what it
        // shows lives under the tmux server — it is seated below, as its
        // session's own row.
        for (i, r) in registered.iter().enumerate() {
            let bridge = r
                .row
                .id
                .0
                .strip_prefix(&shell_prefix)
                .is_some_and(crate::adaptor::tmux::is_tmux_leaf);
            if let Some(child) = r.host_child
                && r.row.kind.0 == khor_core::kind::SHELL
                && !r.row.dead
                && !bridge
            {
                hosts.push(Host {
                    id: r.row.id.clone(),
                    holds: procs.subtree(&[child]),
                    own: Some(i),
                    tmux: None,
                    running: None,
                    panes: Vec::new(),
                    client: None,
                });
            }
        }
        // Oldest first, so the original session outranks any grouped
        // twin of it; the leaf leads with the creation epoch, so the
        // string order is the age order.
        let mut multiplexed = sweep.multiplexed;
        multiplexed.sort_by(|a, b| {
            a.found
                .sighting
                .vendor_session_id
                .cmp(&b.found.sighting.vendor_session_id)
        });
        let mut seen_holds: Vec<u32> = Vec::new();
        for held in multiplexed {
            let id = held.found.id();
            let own = out.iter().position(|r| r.id == id);
            // A grouped session lists the very same panes under a second
            // session id — measured (`tmux new-session -t base` → two
            // lines, one pane pid). The bridge's own client is exactly
            // such a twin, so without this every open terminal face grew
            // a phantom row beside the session it was showing.
            if own.is_none() && held.holds.iter().any(|p| seen_holds.contains(p)) {
                continue;
            }
            seen_holds.extend(held.holds.iter().copied());
            hosts.push(Host {
                id,
                holds: held.holds,
                own,
                tmux: Some(held.found),
                running: held.running,
                panes: held.panes,
                client: None,
            });
        }
        // **A khor shell with `tmux` typed in it holds the session that
        // client shows** (SESS-BRIEF 1b). The client descends from the
        // shell, but what it shows lives under the tmux server, so the
        // descent stopped at the client: the shell said 「正在运行 tmux」
        // and the claude in the pane was another row's. The session's
        // processes are the shell's now, so the shell wears what its
        // panes hold — an agent's form, as a shell running claude
        // directly does — and the session is no second row. A session
        // with a row of its own (a bridge, an imported row) keeps it.
        let mut taken: Vec<usize> = Vec::new();
        for k in 0..hosts.len() {
            // Asked only of a khor shell with a tmux client somewhere in
            // it: the lookup runs `ps` and asks tmux, and a list poll
            // must not pay that for every shell.
            let Some(child) = hosts[k].own.and_then(|i| registered[i].host_child) else {
                continue;
            };
            if hosts[k].tmux.is_some()
                || !hosts[k]
                    .holds
                    .iter()
                    .any(|&p| procs.get(p).is_some_and(|x| x.name == "tmux"))
            {
                continue;
            }
            let Some((session, front)) = self.discovery.attached_in(child) else {
                continue;
            };
            if let Some(m) = hosts
                .iter()
                .position(|h| h.tmux.is_some() && h.own.is_none() && h.id == session)
            {
                let held = hosts[m].holds.clone();
                hosts[k].holds.extend(held);
                hosts[k].panes = hosts[m].panes.clone();
                // With no agent in the session, the shell says what the
                // client shows in front, not 「正在运行 tmux」 (2c).
                let found = hosts[m].tmux.as_ref().map(|f| &f.sighting);
                let pane = front
                    .as_ref()
                    .and_then(|p| hosts[m].panes.iter().find(|x| &x.id == p));
                hosts[k].client = found.map(|f| Client {
                    word: f.word,
                    at_ms: f.at_ms,
                    running: pane.map_or(hosts[m].running.clone(), |p| p.running.clone()),
                    cwd: pane.map_or(f.cwd.clone(), |p| p.cwd.clone()),
                    pane: front,
                });
                taken.push(m);
            }
        }
        let mut m = 0;
        hosts.retain(|_| {
            m += 1;
            !taken.contains(&(m - 1))
        });
        // **An agent nobody registered, gone from a host that is still
        // there.** Nothing else remembers it — its status file and its
        // process went with it — so its note stands it back up as a row,
        // only for the seating below to seat like a registered one: on
        // the list it holds the seat in the host's current form, off it
        // it folds. A note whose host is gone brings nothing back; the
        // row then ends the way an unregistered one always has.
        for n in &notes {
            if out.iter().any(|r| r.id.0 == n.id) || !hosts.iter().any(|h| h.id.0 == n.seat) {
                continue;
            }
            raw.push(State::Done);
            live_pids.push(Vec::new());
            last_seat.push(Some(n.seat.clone()));
            out.push(Session {
                observation: None,
                can_control: false,
                id: SessionId(n.id.clone()),
                kind: Kind(khor_core::kind::TUI.to_owned()),
                title: n.title.clone(),
                home: self.device,
                // Never shown as it stands: the seating either dresses
                // it in the host's form or folds it.
                state: Some(StateStamp {
                    state: State::Done,
                    at: Millis(0),
                }),
                unread: 0,
                dead: true,
                unbacked: false,
                category: n.category.clone(),
                last: None,
                via: None,
                term: false,
                place: n.place.clone(),
                mux: false,
                doing: None,
            });
        }
        // Which host each running row sits in. A pid on record for a
        // process that has died cannot match: `live_pids` was built from
        // the live table.
        let mut sits_in: Vec<Option<usize>> = vec![None; out.len()];
        for (h, host) in hosts.iter().enumerate() {
            for (i, pids) in live_pids.iter().enumerate() {
                if Some(i) != host.own
                    && sits_in[i].is_none()
                    && pids.iter().any(|p| host.holds.contains(p))
                {
                    sits_in[i] = Some(h);
                }
            }
        }
        // **Where each agent row sits, remembered** — the only way to find
        // its host again once the agent has gone, since its status file
        // and its process both go with it. Written when it changes;
        // wiped when the agent is seen running outside every host, so an
        // agent resumed in some other terminal is not folded back into a
        // session it left.
        for (i, r) in registered.iter().enumerate() {
            if r.row.kind.0 != khor_core::kind::TUI {
                continue;
            }
            match sits_in[i] {
                Some(h) if r.seat.as_deref() != Some(hosts[h].id.0.as_str()) => {
                    note_seat(&r.dir, Some(&hosts[h].id.0));
                }
                None if !live_pids[i].is_empty() && r.seat.is_some() => note_seat(&r.dir, None),
                _ => {}
            }
        }
        // The same note for an agent on the list that nobody registered,
        // which has no registry dir to keep it in (`SeatNote`).
        for i in registered.len()..out.len() {
            let r = &out[i];
            if r.kind.0 != khor_core::kind::TUI {
                continue;
            }
            match sits_in[i] {
                Some(h) if listed(&r.id.0) => {
                    let note = SeatNote {
                        id: r.id.0.clone(),
                        seat: hosts[h].id.0.clone(),
                        title: r.title.clone(),
                        category: r.category.clone(),
                        place: r.place.clone(),
                    };
                    if notes.iter().find(|n| n.id == note.id) != Some(&note) {
                        self.write_seat_note(&note);
                    }
                }
                None if !live_pids[i].is_empty() && last_seat[i].is_some() => {
                    self.clear_seat_note(&r.id)
                }
                _ => {}
            }
        }
        let mut folded: Vec<SessionId> = Vec::new();
        let mut folded_bridges: Vec<SessionId> = Vec::new();
        let mut worn: Vec<(SessionId, SessionId)> = Vec::new();
        let mut forms: Vec<(usize, Form)> = Vec::new();
        let mut unbound_rows: Vec<usize> = hosted_holds
            .iter()
            .filter(|(_, holds)| holds.iter().any(|p| unbound.contains(p)))
            .map(|(i, _)| *i)
            .collect();
        for (h, host) in hosts.into_iter().enumerate() {
            // Prefer the pane actually in front. A background pane must not
            // change the current terminal's category or erase its preview.
            let front = host
                .client
                .as_ref()
                .and_then(|c| c.pane.as_ref())
                .and_then(|id| host.panes.iter().find(|p| &p.id == id))
                .or_else(|| host.panes.iter().find(|p| p.active));
            let front_holds = front.map(|p| procs.held_by(&[p.pid]));
            let thread_unbound = front_holds
                .as_ref()
                .unwrap_or(&host.holds)
                .iter()
                .any(|p| unbound.contains(p));
            let inside: Vec<usize> = (0..sits_in.len())
                .filter(|&i| sits_in[i] == Some(h))
                .collect();
            if host.tmux.is_some() {
                // The fold keeps what it computed: these rows' terminals
                // live inside this tmux session, so a host can be bridged
                // in for them (`attach_multiplexed` resolves the route by
                // redoing this intersection) — and the process sits in the
                // person's own tmux, which for a `gui` ghost is the fact
                // that sends it to the terminal face (`Session::mux`).
                for &i in &inside {
                    out[i].term = true;
                    out[i].mux = true;
                }
            }
            let agents: Vec<usize> = inside
                .iter()
                .copied()
                .filter(|&i| out[i].kind.0 == khor_core::kind::TUI)
                .collect();
            // Agent rows last seen here whose agent is nowhere now —
            // registered or not (`SeatNote`). Over the rows as they stood
            // before this loop: a tmux session's own row appended below
            // is a host, never an agent that left one.
            let gone: Vec<usize> = (0..sits_in.len())
                .filter(|&i| {
                    Some(i) != host.own
                        && sits_in[i].is_none()
                        && live_pids[i].is_empty()
                        && out[i].kind.0 == khor_core::kind::TUI
                        && last_seat[i].as_deref() == Some(host.id.0.as_str())
                })
                .collect();
            let newest =
                |rows: &[usize]| rows.iter().copied().max_by_key(|&i| out[i].activity_at());
            let open = |i: usize| registered.get(i).is_some_and(|r| r.open);
            let on_list = |i: usize| listed(&out[i].id.0);
            let which = |rows: &[usize], keep: &dyn Fn(usize) -> bool| -> Vec<usize> {
                rows.iter().copied().filter(|&i| keep(i)).collect()
            };
            let own_seat = host.own.map_or(Holder::Found, Holder::Row);
            // **Who holds the seat, in order** (docs/SESSION.md): a row
            // with a terminal open on it — the tab and the connection are
            // on it; a row on the list; the agent inside now; the host's
            // own row. The last only when nothing inside is a row already
            // — a `khor run` in a pane is that pane's session.
            let holder = host
                .own
                .filter(|&i| open(i))
                .map(Holder::Row)
                .or_else(|| newest(&which(&agents, &open)).map(Holder::Row))
                .or_else(|| newest(&which(&gone, &open)).map(Holder::Row))
                .or_else(|| newest(&which(&agents, &on_list)).map(Holder::Row))
                .or_else(|| listed(&host.id.0).then_some(own_seat))
                .or_else(|| newest(&which(&gone, &on_list)).map(Holder::Row))
                .or_else(|| newest(&agents).map(Holder::Row))
                .or_else(|| inside.is_empty().then_some(own_seat));
            let wear = newest(&agents);
            // Runtime workspace location follows the pane currently in view,
            // even when its command marks still describe the previous pane.
            let runtime_cwd = host
                .client
                .as_ref()
                .and_then(|c| c.cwd.clone())
                .or_else(|| {
                    host.panes
                        .iter()
                        .find(|p| p.active)
                        .and_then(|p| p.cwd.clone())
                });
            let holder_row = match holder {
                // An agent inside holds: it wears its own form, as it
                // always did.
                Some(Holder::Row(k)) if agents.contains(&k) => Some(k),
                Some(Holder::Row(k)) => {
                    match (wear, &host.tmux) {
                        (Some(w), _) => forms.push((
                            k,
                            Form::Agent {
                                from: w,
                                mux: host.tmux.is_some(),
                            },
                        )),
                        (None, Some(found)) => forms.push((
                            k,
                            Form::Terminal {
                                word: found.sighting.word,
                                at_ms: found.sighting.at_ms,
                                running: host.running.clone(),
                                cwd: found.sighting.cwd.clone(),
                                panes: host.panes.clone(),
                            },
                        )),
                        // khor's own terminal with nothing inside is
                        // already in its own form: it is the terminal —
                        // unless what runs in it is a tmux client.
                        (None, None) => {
                            if let Some(c) = host.client {
                                forms.push((
                                    k,
                                    Form::Client {
                                        word: c.word,
                                        at_ms: c.at_ms,
                                        running: c.running,
                                        cwd: c.cwd,
                                        pane: c.pane,
                                    },
                                ));
                            }
                        }
                    }
                    Some(k)
                }
                Some(Holder::Found) => {
                    // The tmux session nobody registered, as found: a
                    // terminal. Every host of this shape is a tmux one.
                    let Some(found) = &host.tmux else {
                        unreachable!("a khor terminal always has its own row")
                    };
                    let sighting = &found.sighting;
                    raw.push(sighting.word);
                    out.push(Session {
                        observation: None,
                        can_control: false,
                        id: host.id.clone(),
                        kind: Kind(found.kind.to_owned()),
                        title: sighting.title.clone(),
                        home: self.device,
                        state: Some(StateStamp {
                            state: sighting.word,
                            at: Millis(sighting.at_ms.max(0) as u64),
                        }),
                        // A multiplexer cannot say 完成, so there is
                        // nothing here that could be unread (docs/UX.md
                        // 角标可归零).
                        unread: 0,
                        // Listed by a live multiplexer server, so
                        // something is running behind it by construction…
                        dead: false,
                        // …and the listing is the voucher.
                        unbacked: false,
                        category: Some(found.category.to_owned()),
                        last: None,
                        via: None,
                        // A discovered multiplexer session is exactly
                        // what the bridge can attach to.
                        term: true,
                        place: place_of(sighting.cwd.as_deref()),
                        mux: true,
                        // No marks cross from a person's tmux pane, so
                        // tmux's own reading (roadmap 2.1): the pane's
                        // program when it is not the shell, and where
                        // the session stands.
                        doing: Some(khor_core::Doing {
                            running: host.running.is_some(),
                            command: host.running.clone(),
                            cwd: sighting.cwd.clone(),
                            ..Default::default()
                        }),
                    });
                    let k = out.len() - 1;
                    if let Some(w) = wear {
                        forms.push((k, Form::Agent { from: w, mux: true }));
                    }
                    Some(k)
                }
                None => None,
            };
            if let Some(k) = holder_row.filter(|_| thread_unbound) {
                unbound_rows.push(k);
            }
            if let (Some(k), Some(cwd)) = (holder_row, runtime_cwd) {
                out[k].doing.get_or_insert_with(Default::default).cwd = Some(cwd);
            }
            // Only a holder that is not itself one of the agents wears
            // one; an agent that holds keeps its own form, and the other
            // agents in the same session stay rows of their own.
            if let (Some(k), Some(w)) = (holder_row, wear)
                && !agents.contains(&k)
            {
                worn.push((out[k].id.clone(), out[w].id.clone()));
                folded.push(out[w].id.clone());
            }
            for &g in &gone {
                if Some(g) != holder_row {
                    folded.push(out[g].id.clone());
                }
            }
            if let Some(o) = host.own
                && Some(o) != holder_row
            {
                folded.push(out[o].id.clone());
                // A bridge whose host still runs, folded all the same:
                // its terminal shows a row that is not on the list, so
                // it is stopped (`reap_folded_bridges`). The order above
                // makes this unreachable today — an open terminal holds
                // the seat — and it is kept so that the reaper and the
                // list can never disagree about which bridge is spare.
                if open(o) {
                    folded_bridges.push(out[o].id.clone());
                }
            }
        }
        // The preview post-pass, one place for every claude-backed row —
        // discovered, hooked, hosted or GUI alike: whatever the vendor's
        // transcript last said. **The transcript outranks the hosted
        // screen line**, not the other way around: an agent TUI's last
        // non-empty screen line is its chrome ("⏵⏵ bypass permissions
        // on…"), which is true of the screen and useless as a preview,
        // so an agent row without a transcript says nothing rather than
        // that (SESS-BRIEF 1d). The leaf is the vendor's own where a hook recorded
        // one (`Meta::vendor_leaf` — a khor-minted id names no vendor
        // file), else the id's leaf, which for discovered rows is the
        // vendor's id already. Cached by (path, mtime) inside
        // `last_said`, so a list poll costs stats, not reads.
        let mut vendor_leaves: std::collections::BTreeMap<String, String> = registered
            .iter()
            .filter_map(|r| r.vendor_leaf.clone().map(|v| (r.row.id.0.clone(), v)))
            .collect();
        vendor_leaves.extend(learned);
        let claude = crate::adaptor::claude::Claude::at(crate::adaptor::claude_home(&self.root));
        let codex = crate::adaptor::codex::Codex::at(crate::adaptor::codex_home(&self.root));
        for r in out.iter_mut() {
            // **An agent row never says its screen's bottom line**
            // (SESS-BRIEF 1d): that line is the program's chrome, not
            // what was said. Until a transcript answers below — a
            // session just opened, a vendor khor cannot read yet — the
            // line is absent. A shell keeps its own line: it is what the
            // shell runs or where it stands.
            if r.kind.0 == khor_core::kind::TUI {
                r.last = None;
            }
            // **Claude's rows, and rows nobody filed.** The preview and
            // the title below are claude's answers and go only on rows
            // known to be claude's; but a `gui` row started without a
            // vendor declared (`category: None`) can still name a
            // transcript in claude's own directory, and whichever
            // store resolves a leaf owns the session — `Node::takeover`
            // has followed that rule since before this loop existed.
            // Measured 2026-09-01: exactly one such row on the author's
            // machine, and it was the last claude row left as
            // 不在任何地方. So an unfiled row gets as far as the place
            // read, and no further.
            let filed = r.category.as_deref();
            let leaf = vendor_leaves.get(&r.id.0).cloned().or_else(|| {
                r.id.0
                    .strip_prefix(&format!("{}/", khor_core::kind::TUI))
                    .map(str::to_owned)
            });
            // codex's rows read codex's rollout the same way: its newest
            // sentence, past the token counts (SESS-BRIEF 1c).
            if filed == Some(crate::adaptor::codex::VENDOR) {
                if let Some(l) = leaf {
                    r.last = codex.last_said(&l);
                }
                continue;
            }
            if filed.is_some() && filed != Some(crate::adaptor::claude::VENDOR) {
                continue;
            }
            // **One resolution, two answers.** Both of these walk the
            // same projects directory to find the same file, so the
            // leaf is computed once and the two readers share the trip
            // — and both cache, so a poll over an unchanged transcript
            // costs stats rather than reads.
            if let Some(l) = leaf {
                // **A row from before places were recorded still has
                // one — in its transcript.** Every hook-registered
                // session older than 2026-09-01 has no place in its
                // note and, once its process is gone, no status file to
                // read one from; measured on the machine this was
                // written on, that was ten of seventeen rows, tokens
                // sessions among them, filed as 不在任何地方 while their
                // transcripts said `cwd` on line 1. Read here, folded
                // like every other source, and only as a fallback: a
                // place already on the row is the birth fact and stands.
                if r.place.is_none()
                    && let Some(cwd) = claude.recorded_cwd(&l)
                {
                    r.place = place_of(Some(&cwd.display().to_string()));
                }
                if filed.is_none() {
                    continue;
                }
                if let Some(said) = claude.last_said(&l) {
                    r.last = Some(said);
                }
                // Conversation-generated names belong to the vendor UI.
                // The row keeps its own identity; user renames overlay it later.
            }
        }
        // The forms go on last, after the preview pass has given every
        // row its own preview: a worn form carries the agent's preview,
        // while the id, title, place and pin stay the holder's
        // (docs/SESSION.md 形态随进程变).
        for (k, form) in forms {
            match form {
                Form::Terminal {
                    word,
                    at_ms,
                    running,
                    cwd,
                    panes,
                } => {
                    // **Marks when the front pane sent any, else tmux's
                    // reading — never an empty row** (item 21's follow-up).
                    // Until the person's shells load the integration, the
                    // ledger has nothing for the pane in front; then the
                    // row says what tmux says that pane runs and where it
                    // stands, as an unattached tmux row does. A pane that
                    // marked has a directory (`OSC 7` at every prompt).
                    let saved = self
                        .dir_of(&out[k].id)
                        .and_then(|d| crate::commands::read_saved(&d));
                    let pane = saved.as_ref().and_then(|s| s.pane.clone());
                    // The pane in front never marked (item 65): the face
                    // says why its history is empty, as for a khor
                    // shell's tmux client. Unknown is not unmarked.
                    let pane_unmarked = pane.as_ref().is_some_and(|p| {
                        saved
                            .as_ref()
                            .and_then(|s| s.marked_panes.as_ref())
                            .is_some_and(|m| !m.contains(p))
                    });
                    let r = &mut out[k];
                    let marked = r
                        .doing
                        .as_ref()
                        .is_some_and(|d| d.running || d.command.is_some() || d.cwd.is_some());
                    if !marked {
                        let front = pane.and_then(|p| panes.into_iter().find(|x| x.id == p));
                        let (running, cwd) = match front {
                            Some(f) => (f.running, f.cwd),
                            None => (running, cwd),
                        };
                        let title = r.doing.as_ref().and_then(|d| d.title.clone());
                        r.doing = Some(khor_core::Doing {
                            running: running.is_some(),
                            command: running,
                            cwd,
                            title,
                            pane_unmarked,
                            ..Default::default()
                        });
                    }
                    r.kind = Kind(khor_core::kind::SHELL.to_owned());
                    r.category = Some(khor_core::category::SHELL.to_owned());
                    r.state = Some(StateStamp {
                        state: word,
                        at: Millis(at_ms.max(0) as u64),
                    });
                    r.unread = 0;
                    // The tmux session is there — nothing ended.
                    r.dead = false;
                    r.unbacked = false;
                    // A tmux client's bottom line is its status bar
                    // (`host::host_serving`'s `mux_client`).
                    r.last = None;
                    r.term = true;
                    r.mux = true;
                }
                Form::Agent { from, mux } => {
                    let agent = out[from].clone();
                    let Some(stamp) = agent.state else {
                        continue;
                    };
                    let (state, unread) =
                        settle_done(raw[from], stamp.at.0 as i64, watermark(&out[k].id.0));
                    let r = &mut out[k];
                    r.kind = agent.kind;
                    r.category = agent.category;
                    r.state = Some(StateStamp {
                        state,
                        at: stamp.at,
                    });
                    r.unread = unread;
                    r.dead = false;
                    r.unbacked = false;
                    r.last = agent.last;
                    // The shell's own reading is 「正在运行 claude」 (or
                    // tmux), and the face draws a command before the
                    // preview: a row whose session is an agent says the
                    // agent's sentence (SESS-BRIEF 1). Where it stands and
                    // what it is called stay.
                    if let Some(d) = r.doing.as_mut() {
                        *d = khor_core::Doing {
                            cwd: d.cwd.take(),
                            title: d.title.take(),
                            notice: d.notice.take(),
                            notice_ms: d.notice_ms,
                            bell_ms: d.bell_ms,
                            ..Default::default()
                        };
                    }
                    r.term = true;
                    r.mux = mux;
                }
                Form::Client {
                    word,
                    at_ms,
                    running,
                    cwd,
                    pane,
                } => {
                    // **Whether the pane in front marks its commands**
                    // (SESS-BRIEF 3): the shell's ledger lists every pane
                    // that sent a mark. One it never heard from has no
                    // commands in the history, and the face says why.
                    // Unknown (no pane named, a host older than the list)
                    // is not unmarked.
                    let saved = self
                        .dir_of(&out[k].id)
                        .and_then(|d| crate::commands::read_saved(&d));
                    let marked = saved.as_ref().and_then(|s| s.marked_panes.as_ref());
                    let pane_unmarked = pane
                        .as_ref()
                        .is_some_and(|p| marked.is_some_and(|m| !m.contains(p)));
                    // **A pane that marks says its own reading** (item
                    // 64): the command as typed, how it ended, where it
                    // stands. Its ledger saying a command runs while tmux
                    // sees the pane at its prompt is an end lost out of
                    // sight (tmux drops a hidden pane's marks): tmux's
                    // reading then, as for a pane that never marked.
                    let own_pane = pane
                        .as_ref()
                        .and_then(|p| saved.and_then(|mut s| s.panes.remove(p)))
                        .filter(|d| d.running || d.command.is_some() || d.cwd.is_some())
                        .filter(|d| !d.running || running.is_some());
                    // The shell's own marks saw it start the client and
                    // nothing since: its command is `tmux`, and its last
                    // line is the client's status bar. What it shows is
                    // the pane in front; where it is called and any
                    // notice stay the shell's.
                    let r = &mut out[k];
                    let own = r.doing.take().unwrap_or_default();
                    let front = match own_pane {
                        Some(d) => khor_core::Doing {
                            cwd: cwd.or(d.cwd).or(own.cwd),
                            ..d
                        },
                        None => khor_core::Doing {
                            running: running.is_some(),
                            command: running,
                            cwd: cwd.or(own.cwd),
                            ..Default::default()
                        },
                    };
                    r.doing = Some(khor_core::Doing {
                        title: own.title,
                        notice: own.notice,
                        notice_ms: own.notice_ms,
                        pane_unmarked,
                        bell_ms: own.bell_ms,
                        ..front
                    });
                    r.state = Some(StateStamp {
                        state: word,
                        at: Millis(at_ms.max(0) as u64),
                    });
                    // A multiplexer cannot say 完成.
                    r.unread = 0;
                    r.last = None;
                }
            }
        }
        // **A shell whose last command failed reads 失败** (the terminal
        // row, the owner 2026-10-08), alive: `dead` stays false, so
        // nothing offers a restart. Until its next command starts or the
        // row is looked at past the command's end ([`failed_command`]).
        for r in out.iter_mut() {
            if let Some(at) = failed_command(r, watermark(&r.id.0)) {
                r.state = Some(StateStamp {
                    state: State::Failed,
                    at: Millis(at),
                });
                r.unread = 0;
                r.dead = false;
            }
        }
        // Only a bound conversation wears the agent's form. Without one,
        // keep a usable shell and its own command reading, never another
        // thread's state, preview or unread.
        for k in unbound_rows {
            let row = &mut out[k];
            row.term = true;
            let doing = row.doing.get_or_insert_with(Default::default);
            // The scoped interactive process is live; this is shell-command
            // activity, not a claim about the agent's current turn.
            doing.running = true;
            doing.command.get_or_insert_with(|| "codex".to_owned());
            doing.thread_unbound = true;
            row.fallback_unbound_terminal();
        }
        // Native selection metadata is transport state, never a display name.
        // Keep the raw ledger untouched so the next discovery can read it.
        for row in &mut out {
            if selection_hosts.contains(&row.id.0)
                && let Some(doing) = row.doing.as_mut()
                && doing
                    .title
                    .as_deref()
                    .is_some_and(crate::codex_terminal::metadata_title)
            {
                if row.last == doing.title {
                    row.last = doing.cwd.as_deref().map(tilde);
                }
                doing.title = None;
            }
        }
        // Compute unnamed terminal identity from its latest directory. The
        // synced user-name overlay is applied by Node after this reading.
        for row in &mut out {
            if matches!(
                row.title.trim(),
                "" | "shell"
                    | "sh"
                    | "bash"
                    | "zsh"
                    | "fish"
                    | "pwsh"
                    | "cmd"
                    | "-sh"
                    | "-bash"
                    | "-zsh"
                    | "-fish"
            ) && let Some(cwd) = row
                .doing
                .as_ref()
                .and_then(|d| d.cwd.as_deref())
                .filter(|s| !s.trim().is_empty())
                .or(row.place.as_deref())
            {
                row.title = cwd
                    .rsplit(['/', '\\'])
                    .find(|s| !s.is_empty())
                    .unwrap_or(cwd)
                    .to_owned();
            }
        }
        for row in &mut out {
            let resource = registered
                .iter()
                .find(|entry| entry.row.id == row.id)
                .is_some_and(|entry| entry.row.kind.0 == khor_core::kind::SHELL);
            row.can_control = if resource || row.id.0.starts_with("shell/") {
                row.term
            } else {
                self.owns_agent_endpoint(&row.id, procs)
            };
        }
        let input_owners = out
            .iter()
            .filter(|row| row.can_control)
            .map(|row| row.id.clone())
            .collect();
        out.retain(|r| !folded.contains(&r.id));
        Seated {
            input_owners,
            rows: out,
            worn,
            folded_bridges,
        }
    }

    /// Registry rows, each with the pid on record for it. Whether there
    /// is one decides whether anyone can pronounce the session dead;
    /// which one it is decides whether a multiplexer session is already
    /// on this list (see [`LiveKind::rows`]).
    /// The ids of every registry entry a khor host was started for —
    /// what khor itself opened, as against what a hook or a sweep put
    /// here. The session list's one-time upgrade reads it (docs/SHELL.md
    /// 第八轮: khor 新建的照旧，自动发现的不保留).
    pub fn hosted_ids(&self) -> Vec<SessionId> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(sessions_dir(&self.root)) else {
            return out;
        };
        for e in rd.flatten() {
            if crate::host::read_host_file(&e.path()).is_err() {
                continue;
            }
            let Ok((meta, _)) = read_pair(&e.path()) else {
                continue;
            };
            let Some(name) = e.file_name().to_str().map(String::from) else {
                continue;
            };
            let id = if meta.id.is_empty() {
                let Some(leaf) = name.strip_prefix(&format!("{}-", meta.kind)) else {
                    continue;
                };
                SessionId(format!("{}/{leaf}", meta.kind))
            } else {
                SessionId(meta.id.clone())
            };
            out.push(id);
        }
        out
    }

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
            let (raw, at, dead, unbacked) = face(&e.path(), &meta, &state);
            let (word, unread) = settle_done(raw, at, watermark(&id.0));
            // Two different terminals can stand on a registry dir, and
            // the seating (`rows`) tells them apart: a **host** is khor's
            // own pty (or its bridge onto a tmux session), a **lens** is a
            // viewer looking into somebody else's tmux. Either one running
            // is a terminal a person has open on this row right now.
            let host = crate::host::read_host_file(&e.path())
                .ok()
                .filter(|hf| crate::link::pid_running(hf.host_pid));
            let lens = crate::host::read_view_file(&e.path())
                .is_ok_and(|vf| crate::link::pid_running(vf.host_pid));
            let mut doing = crate::commands::read_doing(&e.path());
            // **A hosted shell's row never shows its prompt** (item 26;
            // roadmap 2.1's rule, broken for shells whose host predates
            // the shim and so has no ledger: the owner's row read a
            // powerlevel10k prompt line, glyphs and all). With no command
            // from the ledger the row reads where the shell stands —
            // `OSC 7`'s directory, else the title, else the shell's own
            // working directory as the OS reports it — and never the
            // screen's last line.
            let last = match &host {
                Some(hf)
                    if meta.kind == khor_core::kind::SHELL
                        && doing.as_ref().is_none_or(|d| d.command.is_none()) =>
                {
                    shell_stands(doing.as_ref(), hf.child_pid)
                }
                _ => crate::host::read_last(&e.path()),
            };
            // Shells without integration still expose their live directory.
            // Never rewrite Meta.place: it records where the session began.
            if doing.as_ref().and_then(|d| d.cwd.as_ref()).is_none()
                && meta.kind == khor_core::kind::SHELL
                && !id
                    .0
                    .strip_prefix("shell/")
                    .is_some_and(crate::adaptor::tmux::is_tmux_leaf)
                && let Some(cwd) = host.as_ref().and_then(|hf| cwd_of(hf.child_pid))
            {
                doing.get_or_insert_with(Default::default).cwd =
                    Some(cwd.to_string_lossy().into_owned());
            }
            out.push(RegistryRow {
                word_outranks_disk: state.word_outranks_disk(),
                vendor_leaf: meta.vendor_leaf.clone(),
                raw,
                host_child: host.as_ref().map(|hf| hf.child_pid),
                open: host.is_some() || lens,
                seat: read_seat(&e.path()),
                dir: e.path(),
                row: Session {
                    observation: None,
                    can_control: false,
                    id,
                    kind: Kind(meta.kind.clone()),
                    title: meta.title.clone(),
                    home: self.device,
                    state: Some(StateStamp {
                        state: word,
                        at: Millis(at.max(0) as u64),
                    }),
                    unread,
                    dead,
                    unbacked,
                    category: meta.category.clone(),
                    // The host keeps the terminal's last non-empty line
                    // beside its state; a row with no host keeps `None`
                    // here and may still earn a preview from a
                    // transcript (`rows`'s post-pass — which also
                    // outranks this line for agent rows).
                    last,
                    via: None,
                    // A host here is a terminal to be had here — a
                    // *running* host (`Node::is_hosted` has why the
                    // file alone cannot answer this). A row whose host
                    // died but whose process is still sitting in a
                    // multiplexer earns `term` back in the loop below,
                    // which is where that route is resolved anyway.
                    term: host.is_some() || lens,
                    place: place_of(meta.place.as_deref()),
                    mux: false,
                    // The host's command ledger (`crate::commands`),
                    // when its shell marks commands.
                    doing,
                },
                pid: meta.pid,
            });
        }
        out
    }

    /// How this machine's session ended — `None` for a session that is
    /// not dead, or not this machine's ([`Ending`]).
    pub fn ending(&self, id: &SessionId) -> Option<Ending> {
        let dir = self.existing_dir(id).ok()?;
        let (meta, state) = read_pair(&dir).ok()?;
        ending_of(&dir, &meta, &state)
    }

    /// A dead session's page ([`Ended`]); `None` while it is not dead.
    pub fn ended(&self, id: &SessionId) -> Option<Ended> {
        let ending = self.ending(id)?;
        let dir = self.existing_dir(id).ok()?;
        Some(Ended {
            ending,
            screen: crate::host::read_ring_file(&dir),
            recent: crate::commands::read_saved(&dir)
                .map(|s| s.recent)
                .unwrap_or_default(),
        })
    }

    fn existing_dir(&self, id: &SessionId) -> Result<PathBuf, String> {
        let dir = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        if !dir.join("meta.json").exists() {
            return Err(msg::no_such_session(&id.0));
        }
        Ok(dir)
    }

    #[cfg(test)]
    fn write_state(
        &self,
        dir: &Path,
        word: State,
        exit: Option<i32>,
        source: Source,
    ) -> Result<(), String> {
        let gate = RowGate::acquire(dir)?;
        if gate.read_meta()?.gui_run.is_some() {
            return Err("GUI execution requires its current run key".into());
        }
        gate.write_state(word, exit, source, None)
    }
}

/// What the seating in [`LiveKind::rows`] decided, beyond the rows.
struct Seated {
    rows: Vec<Session>,
    input_owners: Vec<SessionId>,
    /// `(holder, agent)`: the holder wears the agent row's form and the
    /// agent row folded into it.
    worn: Vec<(SessionId, SessionId)>,
    /// Bridge rows folded while their host still runs — what
    /// [`LiveKind::reap_folded_bridges`] stops.
    folded_bridges: Vec<SessionId>,
}

/// One host the seating seats rows on: a tmux session, or khor's own pty.
struct Host {
    /// The host's own session id — the tmux session's, or the hosted
    /// terminal's. What a `seat` note names.
    id: SessionId,
    /// Every live pid inside it.
    holds: Vec<u32>,
    /// Its own row, where one is already on the list being built.
    own: Option<usize>,
    /// The tmux session as the sweep found it; `None` for khor's pty.
    tmux: Option<crate::adaptor::Found>,
    /// What tmux says a pane runs, when it is not the shell.
    running: Option<String>,
    /// tmux's reading of each pane.
    panes: Vec<crate::adaptor::PaneReading>,
    /// khor's own terminal showing a person's tmux client: what is in
    /// front of it (SESS-BRIEF 2c).
    client: Option<Client>,
}

/// What a khor shell's tmux client shows: the session's word, and the
/// front pane's program and place (the session's reading where tmux
/// named no front pane).
struct Client {
    word: State,
    at_ms: i64,
    running: Option<String>,
    cwd: Option<String>,
    /// The pane in front, where tmux named one (`%3`).
    pane: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Holder {
    Row(usize),
    /// The tmux session's own row, not yet on the list (nobody
    /// registered it).
    Found,
}

/// The form a holder takes on.
enum Form {
    /// A plain terminal: nothing recognisable is running in the host.
    /// With tmux's own reading, for a row whose shell sent no marks.
    Terminal {
        word: State,
        at_ms: i64,
        running: Option<String>,
        cwd: Option<String>,
        panes: Vec<crate::adaptor::PaneReading>,
    },
    /// The agent at row `from` — kind, category, word, preview.
    Agent { from: usize, mux: bool },
    /// What a khor shell's tmux client shows in front ([`Client`]).
    Client {
        word: State,
        at_ms: i64,
        running: Option<String>,
        cwd: Option<String>,
        pane: Option<String>,
    },
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
    /// The word before 完成 was settled against this row's watermark
    /// ([`face`]). The seating needs it: a row that **wears** this
    /// one's form settles it against its own watermark instead, or a
    /// badge on the holder could only be cleared by looking at a row
    /// that is no longer on the list.
    raw: State,
    /// The child of khor's own host for this row, while that host runs —
    /// the root of the process tree a hosted terminal holds.
    host_child: Option<u32>,
    /// A terminal is open on this row now: its host, or a lens onto a
    /// tmux session (`host::spawn_viewer_host`), is running.
    open: bool,
    /// The host this agent row was last seen sitting in (the `seat`
    /// file, docs/SESSION.md 形态随进程变).
    seat: Option<String>,
    dir: PathBuf,
}

/// Where a registry row remembers the host it was last seen in.
///
/// **A file of its own, not a field in `meta.json`**: it is written from
/// the listing, which runs in every process that lists (serve, app, CLI)
/// while hooks rewrite `meta.json` in theirs. Sharing the file would make
/// every seat note a read-modify-write race with a hook's `learn_*`, and
/// the loser's write would silently vanish.
fn seat_path(dir: &Path) -> PathBuf {
    dir.join("seat")
}

fn read_seat(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(seat_path(dir)).ok()?;
    let seat = text.trim();
    (!seat.is_empty()).then(|| seat.to_owned())
}

/// Writes (or, with `None`, clears) the seat. Best effort: a note that
/// did not land is written again on the next listing that sees the row.
fn note_seat(dir: &Path, seat: Option<&str>) {
    match seat {
        Some(s) => {
            let _ = write_whole(&seat_path(dir), s.as_bytes());
        }
        None => {
            let _ = fs::remove_file(seat_path(dir));
        }
    }
}

/// Where an agent row **khor never registered** was last seen sitting —
/// the `seat` file's job for a row that has no registry dir to keep one
/// in (no hook, never opened in khor). Without it such an agent, once it
/// left, had nothing to find its host by: its status file and its
/// process go with it, so the row the person imported vanished and the
/// tmux session came back as a row nobody put on the list (2026-09-30
/// account, docs/SESSION.md 形态随进程变).
///
/// Written only for a row **on the list** — nobody else misses one — and
/// it carries what the row showed, because after the agent is gone
/// nothing else can say it. One file per row under `.khor/seats`, apart
/// from the registry so that `claims` and the registry listing never see
/// it; wiped when the agent is seen running outside every host, the
/// registered note's rule.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Debug)]
struct SeatNote {
    id: String,
    /// The host's session id, as a registered row's `seat` file says it.
    seat: String,
    title: String,
    category: Option<String>,
    place: Option<String>,
}

fn seats_dir(root: &Path) -> PathBuf {
    root.join(".khor").join("seats")
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
/// The third position is [`khor_core::Session::dead`]: whether
/// anything will ever move this row again on its own. Said here rather
/// than re-derived from the word, because the word loses it — a shell
/// that exited cleanly and was looked at reads 空闲, exactly like one
/// sitting alive at a prompt.
///
/// The fourth is [`khor_core::Session::unbacked`]: nobody can vouch for
/// the word — true only on the one arm where no verdict exists
/// (`Backing::Unpronounceable`, the pid-less hook-observed row). Every
/// other arm has a real judge: an exit record, a pid that answered or
/// refused to, a living child. A sweep sighting later landing on the
/// same row clears it (`LiveKind::rows`' merge) — a vendor file being
/// written is a voucher too.
///
/// **完成 is left unsettled** (2026-09-30): this answers the word, its
/// stamp, `dead` and `unbacked`, and the caller settles 完成 against a
/// watermark ([`settle_done`]) — because since the seating in
/// [`LiveKind::rows`] it is not always this row's own: a row that
/// **wears** this one's form settles it against the holder's watermark,
/// the only one that looking at the holder can move. Every arm but the
/// reported word and the clean shell exit is some other word, and
/// settling those is a no-op, so the two halves recombine into exactly
/// the answer this function used to give.
fn face(dir: &Path, meta: &Meta, state: &LiveState) -> (State, i64, bool, bool) {
    if let Some(code) = state.exit {
        if code != 0 {
            return (State::Failed, state.at_ms, true, false);
        }
        if meta.kind == khor_core::kind::TUI {
            return (State::Idle, state.at_ms, true, false);
        }
        return (State::Done, state.at_ms, true, false);
    }
    match backing(dir, meta) {
        Backing::Gone => (State::Failed, state.at_ms, true, false),
        // The child is alive; the row can move again the moment a new
        // host stands up — not dead.
        Backing::HostLost => (State::Errored, state.at_ms, false, false),
        Backing::Running => (state.word, state.at_ms, false, false),
        Backing::Unpronounceable => (state.word, state.at_ms, false, true),
    }
}

/// **How a dead row ended, as far as this machine knows** (P5-SHELL
/// item 3, the owner 2026-10-12): 「your command failed」 and 「khor lost
/// it」 both read 失败 by the rule above, and the row's second line says
/// which. Derived from the same two facts [`face`] reads — the recorded
/// exit first, then the backing — so the two cannot disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The host recorded how its child ended.
    Exit(i32),
    /// The host is gone and nothing was recorded: it was killed (a
    /// `pkill`, a crash, the machine going down) before its child ended.
    HostGone,
}

/// [`Ending`] for one registry entry: `None` while anything still backs
/// it (a running host, a living child under a lost host, no pid at all).
fn ending_of(dir: &Path, meta: &Meta, state: &LiveState) -> Option<Ending> {
    if let Some(code) = state.exit {
        return Some(Ending::Exit(code));
    }
    (backing(dir, meta) == Backing::Gone).then_some(Ending::HostGone)
}

/// **What a dead terminal row's page shows** (P5-SHELL items 1 and 2):
/// how it ended, its last screen when the host left one (`ring.bin`), and
/// the last commands its shell marked (`commands.json`, newest last) —
/// the page draws the screen, else the commands.
#[derive(Debug, Clone)]
pub struct Ended {
    pub ending: Ending,
    pub screen: Option<crate::host::RingFile>,
    pub recent: Vec<crate::commands::Command>,
}

/// Exit codes that are not a failure: the person's ^C (130) and a
/// closed pipe (141, `cmd | head`).
pub const QUIET_EXITS: [i32; 2] = [130, 141];

/// **When a shell row's last command failed, if it still says so**: a
/// terminal row whose shell sits at its prompt (空闲), whose last command
/// ended with a code that is a failure (not 0, not [`QUIET_EXITS`]), and
/// which nobody has looked at since that end (`seen`, the row's
/// watermark). The answer is the end, the stamp the row then wears — so
/// looking at the row covers it and the word goes back to 空闲. A command
/// that starts is running, and the next end decides again.
fn failed_command(r: &Session, seen: i64) -> Option<u64> {
    if r.kind.0 != khor_core::kind::SHELL
        || !r.state.is_some_and(|stamp| stamp.state == State::Idle)
    {
        return None;
    }
    let d = r.doing.as_ref()?;
    let code = d.exit?;
    let end = d.ended_ms?;
    (!d.running && code != 0 && !QUIET_EXITS.contains(&code) && end as i64 > seen).then_some(end)
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
    // GUI state and generation are one atomically replaced Meta snapshot. A
    // list never acquires a row lock, waits, or drops a row due to contention.
    let meta = read_meta(dir)?;
    let state = if let Some(run) = &meta.gui_run {
        run.state.clone()
    } else {
        let text =
            fs::read_to_string(dir.join("state.json")).map_err(msg::cant_read_session_state)?;
        serde_json::from_str(&text).map_err(msg::session_state_garbled)?
    };
    if meta.gui_run.as_ref().map(|run| run.id.as_str()) != state.gui_run.as_deref() {
        return Err("session metadata and state name different GUI executions".into());
    }
    Ok((meta, state))
}

/// This retires only the Khor execution slot, not the provider process. A failed
/// cleanup can still publish exit=1; recovery admission must separately reject
/// an Unknown provider even when this is true and the journal lock is free.
/// Legacy state.json and a mismatched embedded execution cannot retire a slot.
fn gui_slot_ended(meta: &Meta) -> bool {
    meta.gui_run.as_ref().is_some_and(|run| {
        run.state.gui_run.as_deref() == Some(run.id.as_str()) && run.state.exit.is_some()
    })
}

fn check_run(meta: &Meta, key: &RunKey) -> Result<(), String> {
    if meta.id != key.session.0
        || meta.gui_native.as_ref() != Some(&key.native)
        || meta.gui_run.as_ref().map(|run| &run.id) != Some(&key.run_id)
    {
        return Err("GUI execution has been superseded".into());
    }
    Ok(())
}

/// Only this invocation's synchronous pre-supervisor observer constructs these
/// facts. No serialized PID or later process lookup can replace this capture.
pub(crate) struct GuiBirth {
    host: Option<recovery_attempt::ProcessStamp>,
    provider: Option<recovery_attempt::ProcessStamp>,
}
pub(crate) fn observe_gui_birth(actual_pid: Option<u32>) -> GuiBirth {
    let host = proc_identity::current_host().ok();
    let provider = host.as_ref().and_then(|host| proc_identity::child(actual_pid, host).ok());
    GuiBirth { host, provider }
}

/// Child-only runtime permission reconstructed from one durable preghost claim,
/// its actual current host, and the very same physical Journal admission.
pub(crate) struct GuiRecovery {
    directory: PathBuf,
    key: recovery_attempt::AttemptKey,
    host: recovery_attempt::ProcessStamp,
    journal: crate::session_content::journal::JournalLease,
    control: Option<Arc<crate::agent_process::Control>>,
    actual_pid: Option<u32>,
}

/// The successful New invocation retains the same Journal and Control owners
/// through publication and the outer, actual process cleanup result.
pub(crate) struct GuiInitial {
    directory: PathBuf,
    authority: recovery_attempt::InitialNewAuthority,
    control: Arc<crate::agent_process::Control>,
}
impl LiveKind {
    pub(crate) fn gui_ghost_spawn_failed(&self, encoded: &str) -> Result<(), String> {
        if encoded.len() > 16 * 1024 { return Err("GUI claim exceeds its byte budget".into()); }
        let key: recovery_attempt::AttemptKey = serde_json::from_str(encoded).map_err(|error| error.to_string())?;
        let (home, id) = key.scope();
        if home != self.device { return Err("GUI claim belongs to another home".into()); }
        let directory = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let mut gate = RowGate::acquire(&directory)?;
        recovery_attempt::Held::load(&mut gate)?.spawn_failed(&key)
    }

    pub(crate) fn claim_gui_recovery(
        &self,
        reservation: &crate::session_content::journal::JournalReservation,
        launch: &[u8],
        encoded: &str,
    ) -> Result<GuiRecovery, String> {
        if encoded.len() > 16 * 1024 { return Err("GUI claim exceeds its byte budget".into()); }
        let key: recovery_attempt::AttemptKey = serde_json::from_str(encoded).map_err(|error| error.to_string())?;
        let native = reservation.identity();
        self.check_gui_source(native)?;
        let id = crate::adaptor::id_for(&native.native_id);
        let directory = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        reservation.check_binding(&directory)?;
        let host = proc_identity::current_host().map_err(|_| "actual recovery host birth is unknown".to_owned())?;
        let mut gate = RowGate::acquire(&directory)?;
        let meta = gate.read_meta_or_new()?;
        let expected = meta.and_then(|meta| meta.gui_run.map(|run| run.id));
        let arguments = recovery_attempt::Arguments::native(native.clone(), expected, launch)?;
        let mut held = recovery_attempt::Held::load(&mut gate)?;
        held.check_claim(&key, &arguments)?;
        held.claim_host(&key, host.clone())?;
        held.starting_provider(&key)?;
        Ok(GuiRecovery { directory, key, host, journal: reservation.lease(), control: None, actual_pid: None })
    }

    pub(crate) fn attach_gui_recovery(
        &self,
        recovery: &mut GuiRecovery,
        control: &Arc<crate::agent_process::Control>,
        actual_pid: Option<u32>,
        birth: GuiBirth,
    ) -> Result<(), String> {
        recovery.control = Some(control.clone());
        recovery.actual_pid = actual_pid;
        if birth.host.as_ref() != Some(&recovery.host) {
            return Err("provider observation has another actual recovery host".into());
        }
        let provider = birth.provider.ok_or("actual provider birth is unknown")?;
        if Some(provider.pid) != actual_pid { return Err("provider observation differs from its real Session".into()); }
        let mut gate = RowGate::acquire(&recovery.directory)?;
        recovery_attempt::Held::load(&mut gate)?.owned_provider(&recovery.key, provider)
    }

    pub(crate) fn record_gui_recovery_support(
        &self,
        recovery: &GuiRecovery,
        support: khor_acp::RecoveryCapabilities,
    ) -> Result<(), String> {
        if recovery.control.is_none() || recovery.actual_pid.is_none() {
            return Err("Initialize facts lost their actual transport owner".into());
        }
        let mut gate = RowGate::acquire(&recovery.directory)?;
        recovery_attempt::Held::load(&mut gate)?.record_support(&recovery.key, support)
    }

    pub(crate) fn publish_gui_recovery(
        &self,
        recovery: &GuiRecovery,
        journal: &mut crate::session_content::journal::Journal,
        title: &str,
        category: Option<&str>,
        word: State,
        ready: &Path,
    ) -> Result<RunKey, String> {
        if recovery.control.is_none() || !journal.owns_lease(&recovery.journal) {
            return Err("recovery lost its actual transport or Journal owner".into());
        }
        if journal.is_pending() { journal.prepare_base()?; } else { journal.sync_received_prefix()?; }
        let native = journal.identity().clone();
        let id = crate::adaptor::id_for(&native.native_id);
        let generation = journal.generation().map(str::to_owned);
        let mut gate = RowGate::acquire(&recovery.directory)?;
        let mut held = recovery_attempt::Held::load(&mut gate)?;
        held.publish_base(&recovery.key, generation.as_deref(), |gate| {
            if journal.is_pending() { journal.publish_base()?; }
            journal.bind(&gate.dir)
        })?;
        held.publish_run(&recovery.key, |gate, run_id| {
            let key = self.begin_gui_run_held(gate, &id, title, std::process::id(), &native,
                category, true, None, run_id, word, None)?;
            fs::write(ready, &id.0).map_err(|error| error.to_string())?;
            Ok(key)
        })
    }

    pub(crate) fn settle_gui_recovery(
        &self,
        recovery: &GuiRecovery,
        receipt: Result<&khor_acp::ProcessExit, ()>,
        slot_code: i32,
    ) -> Result<(), String> {
        let mut gate = RowGate::acquire(&recovery.directory)?;
        recovery_attempt::Held::load(&mut gate)?.settle_recovery(
            &recovery.key, &recovery.host, recovery.actual_pid, receipt,
            |gate, id, native, run, pid| retire_gui_slot_held(gate, id, native, run, pid, slot_code),
        )
    }

    pub(crate) fn seed_gui_new(
        &self,
        reservation: &crate::session_content::journal::JournalReservation,
        launch: &[u8],
        control: &Arc<crate::agent_process::Control>,
        actual_pid: Option<u32>,
        birth: GuiBirth,
        support: Option<khor_acp::RecoveryCapabilities>,
    ) -> Result<GuiInitial, String> {
        let native = reservation.identity();
        self.check_gui_source(native)?;
        let id = crate::adaptor::id_for(&native.native_id);
        let directory = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        fs::create_dir_all(&directory).map_err(msg::cant_make_session_dir)?;
        let mut gate = RowGate::acquire(&directory)?;
        let authority = recovery_attempt::Held::load(&mut gate)?.seed_initial_new(
            recovery_attempt::Arguments::initial_new(native.clone(), launch)?,
            recovery_attempt::NewInvocation {
                reservation, control, actual_pid, host: birth.host, provider: birth.provider, support,
            },
        )?;
        Ok(GuiInitial { directory, authority, control: control.clone() })
    }

    pub(crate) fn publish_gui_new(
        &self,
        initial: &GuiInitial,
        journal: &mut crate::session_content::journal::Journal,
        title: &str,
        category: Option<&str>,
        word: State,
        place: Option<&Path>,
        ready: &Path,
    ) -> Result<RunKey, String> {
        // The potentially long finite prefix is durable before the short gate.
        journal.prepare_base()?;
        let native = journal.identity().clone();
        let id = crate::adaptor::id_for(&native.native_id);
        let mut gate = RowGate::acquire(&initial.directory)?;
        let mut held = recovery_attempt::Held::load(&mut gate)?;
        held.publish_initial_base(&initial.authority, journal, &initial.control, |gate, journal| {
            journal.publish_base()?;
            journal.bind(&gate.dir)
        })?;
        held.publish_initial_run(&initial.authority, journal, &initial.control, |gate, _, run_id| {
            let key = self.begin_gui_run_held(
                gate, &id, title, std::process::id(), &native, category, false, None, run_id, word, place,
            )?;
            // This is the actual existing opener marker, not a synthetic Ready.
            // Any failure leaves durable PublishingRun/partial publication and
            // cannot reenter the callback using a lost or repeated token.
            fs::write(ready, &id.0).map_err(|error| error.to_string())?;
            Ok(key)
        })
    }

    pub(crate) fn confirm_gui_ready(
        &self,
        id: &SessionId,
        ghost_pid: u32,
        native_id: Option<&str>,
        agent: Option<&str>,
        claim: Option<&str>,
    ) -> Result<(), String> {
        let directory = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        // Never spend a fresh one-second row wait on every poll. The opener
        // retries only inside its original absolute startup deadline.
        let mut gate = RowGate::try_acquire(&directory)?;
        let meta = gate.read_meta()?;
        let native = meta.gui_native.as_ref().ok_or("GUI Ready has no exact native source")?;
        if meta.id != id.0 || meta.kind != khor_core::kind::GUI
            || native.home != self.device || meta.pid != Some(ghost_pid)
            || crate::adaptor::id_for(&native.native_id) != *id
            || native_id.is_some_and(|expected| expected != native.native_id)
            || agent.is_some_and(|expected| expected != native.agent)
        {
            return Err("GUI Ready belongs to another home, source or spawned ghost".into());
        }
        let run = meta.gui_run.as_ref().ok_or("GUI Ready has no current run")?;
        if run.state.exit.is_some() || run.state.gui_run.as_ref() != Some(&run.id) {
            return Err("GUI Ready names an ended or inconsistent execution".into());
        }
        let expected = claim.map(|claim| {
            if claim.len() > 16 * 1024 {
                return Err("GUI recovery claim exceeds its byte budget".to_owned());
            }
            serde_json::from_str::<recovery_attempt::AttemptKey>(claim).map_err(|error| error.to_string())
        }).transpose()?;
        let held = recovery_attempt::Held::load(&mut gate)?;
        if held.ready_run(native, ghost_pid, expected.as_ref())? != run.id {
            return Err("GUI Ready result and current Meta run disagree".into());
        }
        Ok(())
    }

    pub(crate) fn quarantine_gui_new(
        &self,
        reservation: &crate::session_content::journal::JournalReservation,
    ) -> Result<(), String> {
        let native = reservation.identity();
        if native.home != self.device {
            return Err("unconfirmed New belongs to another home".into());
        }
        let id = crate::adaptor::id_for(&native.native_id);
        let directory = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let mut gate = RowGate::acquire(&directory)?;
        recovery_attempt::Held::load(&mut gate)?.quarantine_new(reservation)
    }

    pub(crate) fn settle_gui_new(
        &self,
        initial: &GuiInitial,
        receipt: Result<&khor_acp::ProcessExit, ()>,
        slot_code: i32,
    ) -> Result<(), String> {
        let mut gate = RowGate::acquire(&initial.directory)?;
        recovery_attempt::Held::load(&mut gate)?.settle_initial(
            &initial.authority, &initial.control, receipt,
            |gate, id, native, run_id, host_pid| {
                retire_gui_slot_held(gate, id, native, run_id, host_pid, slot_code)
            },
        )
    }
}

/// Partial publication may have committed Meta before returning its RunKey.
/// Only that exact execution may be retired here; an old/new run is untouched.
/// This Khor-slot ending is never a provider exit receipt.
fn retire_gui_slot_held(
    gate: &RowGate,
    id: &SessionId,
    native: &crate::session_content::journal::Identity,
    run_id: &str,
    host_pid: u32,
    code: i32,
) -> Result<(), String> {
    let Some(meta) = gate.read_meta_or_new()? else { return Ok(()); };
    let Some(run) = &meta.gui_run else { return Ok(()); };
    if meta.id != id.0 || meta.gui_native.as_ref() != Some(native)
        || meta.pid != Some(host_pid) || run.id != run_id
        || run.state.gui_run.as_deref() != Some(run_id) || run.state.exit.is_some()
    {
        return Ok(());
    }
    gate.write_state(run.state.word, Some(code),
        run.state.source.unwrap_or(Source::Reported), Some(run_id))?;
    gate.sync_meta()
}

/// One short cross-process row transaction. No caller keeps this over a spawn,
/// network request or await. Held helpers never reacquire their own file lock.
struct RowGate {
    dir: PathBuf,
    _file: fs::File,
}
impl RowGate {
    fn acquire(dir: &Path) -> Result<Self, String> {
        Self::acquire_with_wait(dir, true)
    }
    fn try_acquire(dir: &Path) -> Result<Self, String> {
        Self::acquire_with_wait(dir, false)
    }
    fn acquire_with_wait(dir: &Path, wait: bool) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(dir).map_err(|e| e.to_string())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("session row is not a directory".into());
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        let file = options
            .open(dir.join("row.lock"))
            .map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() != 0 {
            return Err("session row lock is not an empty regular file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 1 || metadata.uid() != unsafe { libc::geteuid() } {
                return Err("session row lock has another owner or link".into());
            }
        }
        let mut waiting_until: Option<Instant> = None;
        loop {
            if waiting_until.is_some_and(|until| Instant::now() >= until) {
                return Err("session row is busy".into());
            }
            match file.try_lock() {
                Ok(()) => {
                    return Ok(Self {
                        dir: dir.into(),
                        _file: file,
                    });
                }
                Err(fs::TryLockError::WouldBlock) => {
                    if !wait {
                        return Err("session row is busy".into());
                    }
                    // Like ordinary Node admission, only actual contention
                    // spends the one-second wait allowance.
                    let until = *waiting_until
                        .get_or_insert_with(|| Instant::now() + Duration::from_secs(1));
                    let left = until.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Err("session row is busy".into());
                    }
                    std::thread::sleep(left.min(Duration::from_millis(2)));
                }
                Err(fs::TryLockError::Error(why)) => return Err(why.to_string()),
            }
        }
    }

    fn read_meta(&self) -> Result<Meta, String> {
        read_meta(&self.dir)
    }
    fn read_meta_or_new(&self) -> Result<Option<Meta>, String> {
        match fs::read(self.dir.join("meta.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(msg::session_meta_garbled),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(why) => Err(msg::cant_read_session_meta(why)),
        }
    }
    fn read_pair(&self) -> Result<(Meta, LiveState), String> {
        read_pair(&self.dir)
    }
    fn write_meta(&self, meta: &Meta) -> Result<(), String> {
        write_whole(
            &self.dir.join("meta.json"),
            &serde_json::to_vec(meta).map_err(|e| e.to_string())?,
        )
    }
    fn write_state(
        &self,
        word: State,
        exit: Option<i32>,
        source: Source,
        run: Option<&str>,
    ) -> Result<(), String> {
        let state = LiveState {
            gui_run: run.map(str::to_owned),
            word,
            at_ms: now_ms(),
            exit,
            source: Some(source),
        };
        if let Some(run) = run {
            let mut meta = self.read_meta()?;
            let owned = meta
                .gui_run
                .as_mut()
                .filter(|owned| owned.id == run)
                .ok_or_else(|| "GUI execution has been superseded".to_owned())?;
            owned.state = state;
            self.write_meta(&meta)
        } else {
            write_whole(
                &self.dir.join("state.json"),
                &serde_json::to_vec(&state).map_err(|e| e.to_string())?,
            )
        }
    }
    fn sync_meta(&self) -> Result<(), String> {
        fs::File::open(self.dir.join("meta.json"))
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::File::open(&self.dir)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn native_binding(&self) -> Result<Option<crate::session_content::journal::Identity>, String> {
        let file = match fs::File::open(self.dir.join("journal.json")) {
            Ok(file) => file,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(why.to_string()),
        };
        let mut bytes = Vec::new();
        file.take(4097)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 4096 {
            return Err("session journal identity exceeds its byte budget".into());
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| e.to_string())
    }
    fn check_native(
        &self,
        id: &SessionId,
        meta: Option<&Meta>,
        native: &crate::session_content::journal::Identity,
        legacy: Option<&(Option<String>, crate::session_content::journal::Identity)>,
    ) -> Result<(), String> {
        let binding = self.native_binding()?;
        if binding.as_ref().is_some_and(|bound| bound != native) {
            return Err("session already names another full native identity".into());
        }
        if let Some(meta) = meta {
            if !meta.id.is_empty() && meta.id != id.0 {
                return Err("session metadata belongs to another identity".into());
            }
            if meta
                .gui_native
                .as_ref()
                .is_some_and(|bound| bound != native)
            {
                return Err("GUI row already names another full native identity".into());
            }
            if meta.gui_native.is_none()
                && binding.is_none()
                && !legacy.is_some_and(|(category, bound)| {
                    meta.category.as_deref() == category.as_deref() && bound == native
                })
            {
                return Err("GUI recovery has no exact existing native binding".into());
            }
        }
        Ok(())
    }
}

/// Whole-file write via tmp+rename: concurrent readers (serve deriving
/// rows) and writers (hooks) never meet a half-written file.
pub(crate) fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
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
    status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
}

#[cfg(not(unix))]
fn exit_code_of(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch tree that **takes itself away however the test leaves**
    /// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
    /// cleanup written at the end of a test is skipped by the assertion
    /// that fails, so a red run leaks a directory every time it is red).
    /// A copy per file, which is this repo's convention for test helpers —
    /// it costs no dev-dependency.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn tmp(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-live-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    fn kind_at(root: &Path) -> LiveKind {
        LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
    }

    fn sid(s: &str) -> SessionId {
        SessionId(s.to_owned())
    }

    fn gui_source(native_id: &str) -> crate::session_content::journal::Identity {
        crate::session_content::journal::Identity {
            home: DeviceId([9; 32]),
            agent: "fixture".into(),
            native_id: native_id.into(),
        }
    }

    #[test]
    fn gui_run_rejects_old_and_unqualified_reports_without_changing_the_new_pair() {
        let root = tmp("gui-run-stale");
        let live = kind_at(&root);
        let native = gui_source("gui-generation");
        let id = crate::adaptor::id_for(&native.native_id);
        let a = live
            .begin_gui_run(&id, "first", std::process::id(), &native, None, false)
            .unwrap();
        let birth = read_meta(&a.directory).unwrap().started_ms;
        live.report_run(&a, State::Busy, Source::Reported).unwrap();
        let b = live
            .begin_gui_run(&id, "second", std::process::id(), &native, None, true)
            .unwrap();
        assert_ne!(a.run_id, b.run_id);
        assert_eq!(
            read_meta(&b.directory).unwrap().started_ms,
            birth,
            "logical birth is retained"
        );
        live.report_run(&b, State::Blocked, Source::Reported)
            .unwrap();
        let pair =
            || ["meta.json", "state.json"].map(|name| fs::read(b.directory.join(name)).unwrap());
        let before = pair();
        assert!(
            live.report_run(&a, State::Done, Source::Reported)
                .unwrap_err()
                .contains("superseded")
        );
        assert!(
            live.record_exit_run(&a, 0)
                .unwrap_err()
                .contains("superseded")
        );
        assert!(live.report(&id, State::Done, Source::Screen).is_err());
        assert!(live.record_exit(&id, 7).is_err());
        assert_eq!(pair(), before);
        live.record_exit_run(&b, 0).unwrap();
        live.record_exit_run(&b, 9).unwrap();
        assert_eq!(
            read_pair(&b.directory).unwrap().1.exit,
            Some(0),
            "first current-run exit wins"
        );
    }

    #[test]
    fn gui_run_check_and_write_keep_the_row_gate_until_publication() {
        use std::sync::mpsc;
        let root = tmp("gui-run-held");
        let live = kind_at(&root);
        let native = gui_source("gui-held-generation");
        let id = crate::adaptor::id_for(&native.native_id);
        let a = live
            .begin_gui_run(&id, "first", std::process::id(), &native, None, false)
            .unwrap();
        let (tx, rx) = mpsc::channel();
        let next_live = live.clone();
        let next_native = native.clone();
        let next_id = id.clone();
        let worker = live.with_run(&a, |gate, _, _| {
            // This is the same checked-write helper used by both report_run
            // and record_exit_run, paused after their ownership check.
            let lock_was_held = RowGate::try_acquire(&a.directory).is_err();
            let (started, ready) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                started.send(()).unwrap();
                let result = next_live.begin_gui_run(
                    &next_id,
                    "next",
                    std::process::id(),
                    &next_native,
                    None,
                    true,
                );
                tx.send(result).unwrap();
            });
            let started = ready.recv_timeout(Duration::from_millis(100));
            let before_release = rx.recv_timeout(Duration::from_millis(20));
            let written = gate.write_state(State::Done, None, Source::Reported, Some(&a.run_id));
            // Join only after with_run has released its guard.
            Ok((worker, lock_was_held, started, before_release, written))
        });
        let (worker, lock_was_held, started, before_release, written) = worker.unwrap();
        let after_release = rx.recv_timeout(Duration::from_secs(1));
        let joined = worker.join();
        assert!(lock_was_held);
        started.unwrap();
        assert!(matches!(
            before_release,
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        written.unwrap();
        joined.unwrap();
        let b = after_release.unwrap().unwrap();
        assert_eq!(read_pair(&b.directory).unwrap().1.word, State::Idle);
        assert!(live.record_exit_run(&a, 0).is_err());
    }

    #[test]
    fn gui_full_native_collision_refuses_before_meta_or_binding_changes() {
        let root = tmp("gui-run-native");
        let live = kind_at(&root);
        let a = gui_source("abcdefghijklmnopqrstuvwx-one");
        let b = gui_source("abcdefghijklmnopqrstuvwx-two");
        let id = crate::adaptor::id_for(&a.native_id);
        assert_eq!(
            id,
            crate::adaptor::id_for(&b.native_id),
            "the fixture really collides after 24 characters"
        );
        let run = live
            .begin_gui_run(&id, "first", std::process::id(), &a, Some("fixture"), false)
            .unwrap();
        let before = fs::read(run.directory.join("meta.json")).unwrap();
        let state = fs::read(run.directory.join("state.json")).unwrap();
        assert!(
            live.begin_gui_run(&id, "wrong", std::process::id(), &b, Some("fixture"), true)
                .unwrap_err()
                .contains("full native identity")
        );
        assert!(
            live.check_gui_source(&b)
                .unwrap_err()
                .contains("full native identity")
        );
        assert_eq!(fs::read(run.directory.join("meta.json")).unwrap(), before);
        assert_eq!(fs::read(run.directory.join("state.json")).unwrap(), state);
        assert!(!run.directory.join("journal.json").exists());
        // A pre-existing full canonical binding is also authoritative when a
        // legacy row has no GuiRun or gui_native field yet.
        let other = kind_at(&root.join("legacy"));
        other.register(&id, "gui", "legacy", None, None).unwrap();
        let dir = other.dir_of(&id).unwrap();
        fs::write(dir.join("journal.json"), serde_json::to_vec(&a).unwrap()).unwrap();
        let before = fs::read(dir.join("meta.json")).unwrap();
        assert!(
            other
                .begin_gui_run(&id, "wrong", std::process::id(), &b, None, true)
                .unwrap_err()
                .contains("full native identity")
        );
        assert_eq!(fs::read(dir.join("meta.json")).unwrap(), before);
        assert_eq!(
            fs::read(dir.join("journal.json")).unwrap(),
            serde_json::to_vec(&a).unwrap()
        );
    }

    #[test]
    fn gui_row_metadata_learning_preserves_the_current_execution() {
        let root = tmp("gui-run-learn");
        let live = kind_at(&root);
        let native = gui_source("gui-learn-generation");
        let id = crate::adaptor::id_for(&native.native_id);
        let run = live
            .begin_gui_run(&id, "first", std::process::id(), &native, None, false)
            .unwrap();
        live.learn_category(&id, Some("fixture")).unwrap();
        live.learn_place(&id, &root).unwrap();
        live.learn_vendor_leaf(&id, "before").unwrap();
        live.follow_vendor_leaf(&id, "after").unwrap();
        let meta = read_meta(&run.directory).unwrap();
        assert_eq!(
            meta.gui_run.as_ref().map(|value| &value.id),
            Some(&run.run_id)
        );
        assert_eq!(meta.gui_native.as_ref(), Some(&native));
        assert_eq!(meta.vendor_leaf.as_deref(), Some("after"));
        assert!(live.set_pid(&id, std::process::id()).is_err());
        assert!(
            live.reseat(&id, "tui", "unowned replacement", None)
                .is_err()
        );
        live.report_run(&run, State::Done, Source::Reported)
            .unwrap();
    }

    #[test]
    fn busy_writer_does_not_hide_or_delay_a_complete_gui_snapshot() {
        let root = tmp("gui-run-read-busy");
        let live = kind_at(&root);
        let native = gui_source("gui-read-generation");
        let id = crate::adaptor::id_for(&native.native_id);
        let run = live
            .begin_gui_run(&id, "readable", std::process::id(), &native, None, false)
            .unwrap();
        live.report_run(&run, State::Blocked, Source::Reported)
            .unwrap();
        let before =
            ["meta.json", "state.json"].map(|name| fs::read(run.directory.join(name)).unwrap());
        let gate = RowGate::acquire(&run.directory).unwrap();
        let at = Instant::now();
        let (meta, state) = read_pair(&run.directory).unwrap();
        assert_eq!(meta.gui_run.unwrap().id, run.run_id);
        assert_eq!(state.word, State::Blocked);
        let rows = live.rows(|_| 0);
        let row = rows
            .iter()
            .find(|row| row.id == id)
            .expect("busy writer cannot hide the GUI row");
        assert_eq!(
            row.state.as_ref().map(|stamp| stamp.state),
            Some(State::Blocked)
        );
        assert!(
            at.elapsed() < Duration::from_millis(250),
            "a GUI list read never waits for a writer"
        );
        drop(gate);
        assert_eq!(
            ["meta.json", "state.json"].map(|name| fs::read(run.directory.join(name)).unwrap()),
            before
        );
        // A pre-run legacy row still reads its original state without creating
        // a row coordination file or changing any semantic data.
        let legacy = sid("shell/legacy");
        live.register(&legacy, "shell", "legacy", None, None)
            .unwrap();
        let dir = live.dir_of(&legacy).unwrap();
        fs::remove_file(dir.join("row.lock")).unwrap();
        assert!(read_pair(&dir).is_ok());
        assert!(!dir.join("row.lock").exists());
    }






    /// A pid that is certainly dead: spawn `true`, reap it.
    fn dead_pid() -> u32 {
        let mut c = std::process::Command::new("true").spawn().unwrap();
        let pid = c.id();
        c.wait().unwrap();
        pid
    }

    /// **Where khor started a session reaches its row, as the directory
    /// itself** (工作区批, the fold taken back out in 批③). Three claims
    /// in one walk, because they are one path: the note is written at
    /// birth, it arrives unfolded, and a session nobody placed carries
    /// `None` rather than a neighbour's answer.
    ///
    /// The subdirectory is the assertion, and the repo above it is
    /// there to make it one: `place` answered the checkout until
    /// 2026-09-15, and a tree with no `.git` in it would pass either
    /// way (`place_of` on why it stopped folding).
    #[test]
    fn a_session_carries_the_directory_it_was_started_in() {
        let root = tmp("place");
        let k = kind_at(&root);
        let repo = root.join("repo");
        let deep = repo.join("apps/gui");
        fs::create_dir_all(&deep).unwrap();
        fs::create_dir_all(repo.join(".git")).unwrap();

        let placed = sid("shell/placed");
        k.register(&placed, "shell", "in the repo", None, None)
            .unwrap();
        k.learn_place(&placed, &deep).unwrap();

        // The control arm, and the reason this is not one assertion: a
        // row nobody placed must come back `None`. An implementation
        // that answered with the process's own directory — or with the
        // last place it saw — would pass the first claim alone.
        let bare = sid("shell/bare");
        k.register(&bare, "shell", "nowhere in particular", None, None)
            .unwrap();

        let rows = k.rows(|_| 0);
        let of = |id: &SessionId| {
            rows.iter()
                .find(|r| &r.id == id)
                .unwrap_or_else(|| panic!("{id:?} lists"))
                .place
                .clone()
        };
        assert_eq!(
            of(&placed),
            Some(deep.display().to_string()),
            "the row says where it started, not the checkout above it",
        );
        assert_eq!(
            of(&bare),
            None,
            "and a session nobody placed is not filed under a neighbour"
        );

        // Write-once: a later opener standing somewhere else does not
        // move a session that already has a birthplace.
        k.learn_place(&placed, &root.join("elsewhere")).unwrap();
        assert_eq!(
            k.rows(|_| 0).iter().find(|r| r.id == placed).unwrap().place,
            Some(deep.display().to_string()),
            "a birth fact is not corrected later"
        );
    }

    /// **A row from before places were recorded reads its place off its
    /// transcript.** The registry note is empty (that is what "before"
    /// means), nothing is running, so no status file — and the
    /// transcript under `projects/` says `cwd` on its first line.
    ///
    /// The control arm is a transcript with no `cwd` at all: that row
    /// stays `None`, rather than taking the other row's answer or the
    /// process's own directory.
    #[test]
    fn an_old_claude_row_is_placed_from_its_transcript() {
        let root = tmp("backfill");
        let k = kind_at(&root);
        let repo = root.join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir_all(repo.join("src")).unwrap();
        let projects = root.join(".claude/projects/p");
        fs::create_dir_all(&projects).unwrap();
        let line = |cwd: Option<&Path>| {
            let mut v = serde_json::json!({"type":"user","message":{"role":"user","content":"x"}});
            if let Some(c) = cwd {
                v["cwd"] = serde_json::Value::String(c.display().to_string());
            }
            format!("{v}\n")
        };
        let placed_uuid = "aaaaaaaa-1111-4222-8333-444444444444";
        let bare_uuid = "bbbbbbbb-1111-4222-8333-444444444444";
        fs::write(
            projects.join(format!("{placed_uuid}.jsonl")),
            line(Some(&repo.join("src"))),
        )
        .unwrap();
        fs::write(projects.join(format!("{bare_uuid}.jsonl")), line(None)).unwrap();

        // Registered the way a hook of that era registered them: claude's,
        // under the vendor's own leaf, and with nothing said about where.
        let placed = crate::adaptor::id_for(placed_uuid);
        let bare = crate::adaptor::id_for(bare_uuid);
        k.register(
            &placed,
            khor_core::kind::TUI,
            "old one",
            None,
            Some("claude"),
        )
        .unwrap();
        k.register(
            &bare,
            khor_core::kind::TUI,
            "older one",
            None,
            Some("claude"),
        )
        .unwrap();

        let rows = k.rows(|_| 0);
        let of = |id: &SessionId| {
            rows.iter()
                .find(|r| &r.id == id)
                .expect("lists")
                .place
                .clone()
        };
        assert_eq!(
            of(&placed),
            Some(repo.join("src").display().to_string()),
            "read off the transcript, and off it unchanged",
        );
        assert_eq!(
            of(&bare),
            None,
            "a transcript that never said where places nothing"
        );
    }

    /// The registry's half of 常去的地方: a session khor started in a
    /// directory is a visit to that directory, and one it started
    /// nowhere in particular is not a visit anywhere.
    #[test]
    fn the_registry_remembers_where_it_started_things() {
        let root = tmp("registry-places");
        let k = kind_at(&root);
        let here = root.join("here");
        fs::create_dir_all(&here).unwrap();
        let placed = sid("shell/placed");
        k.register(&placed, "shell", "x", None, None).unwrap();
        k.learn_place(&placed, &here).unwrap();
        let bare = sid("shell/bare");
        k.register(&bare, "shell", "y", None, None).unwrap();

        let places = k.places();
        assert_eq!(places.len(), 1, "{places:?}");
        assert_eq!(places[0].0, here);
        assert_eq!(places[0].1, 1, "one visit");
    }

    /// **`mux` says where the person is.** A registered agent whose
    /// process a tmux session holds carries it; an agent under a plain
    /// shell in no tmux does not — and that second half is the control,
    /// because `term` is true of the first and a `mux` that merely
    /// copied `term` would send every khor conversation to the terminal.
    #[test]
    fn a_row_held_by_tmux_is_multiplexed_and_one_that_is_not_is_not() {
        use crate::adaptor::{Discovery, Proc, Procs, tmux::Tmux};

        let root = tmp("mux");
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let shell = |pid: u32| {
            (
                pid,
                Proc {
                    name: "-zsh".into(),
                    started_ms: 1_700_000_000_000,
                    cwd: None,
                    ppid: Some(1),
                },
            )
        };
        let claude = |pid: u32, started_ms: i64, parent: u32| {
            (
                pid,
                Proc {
                    name: "claude".into(),
                    started_ms,
                    cwd: None,
                    ppid: Some(parent),
                },
            )
        };
        let procs = Procs::of([
            shell(3001),
            shell(3002),
            claude(4001, 1_700_000_000_000, 3001),
            claude(4002, 1_700_000_100_000, 3002),
        ]);
        // tmux holds 3001's pane and nothing else.
        let holding = crate::adaptor::tmux::fake_tmux(
            "mux-held",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/tmp\x1f%1\x1fwith-claude\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&holding)),
        ));
        let rows = k.rows(|_| 0);
        let of = |id: &str| {
            rows.iter()
                .find(|r| r.id.0 == id)
                .unwrap_or_else(|| panic!("{id} lists: {rows:?}"))
        };
        assert!(
            !of("tui/11111111-1111-4111-8111").can_control,
            "a foreign agent in tmux is read-only"
        );
        assert!(
            k.require_input_owner(&sid("tui/11111111-1111-4111-8111"))
                .unwrap_err()
                .contains("outside Khor")
        );
        assert!(
            of("tui/11111111-1111-4111-8111").mux,
            "its process sits in tmux"
        );
        assert!(
            !of("tui/22222222-2222-4222-8222").mux,
            "control: in no tmux, not multiplexed"
        );
    }

    #[test]
    fn input_capability_requires_current_host_and_child_ownership() {
        use crate::adaptor::{Proc, Procs};
        let root = tmp("input-owner");
        let k = kind_at(&root);
        let id = sid("tui/ownership");
        k.register(&id, "tui", "fixture", Some(4100), Some("claude"))
            .unwrap();
        let dir = k.dir_of(&id).unwrap();
        let proc = |name: &str, parent| Proc {
            name: name.into(),
            started_ms: 0,
            cwd: None,
            ppid: parent,
        };
        let procs = Procs::of([
            (4100, proc("khor", None)),
            (4101, proc("claude", Some(4100))),
        ]);
        assert!(
            !k.owns_agent_endpoint(&id, &procs),
            "hook metadata alone cannot grant input"
        );
        let marker = crate::host::HostFile {
            port: 1234,
            cookie: "fixture".into(),
            host_pid: 4100,
            child_pid: 4101,
        };
        fs::write(
            crate::host::view_file_path(&dir),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();
        assert!(
            !k.owns_agent_endpoint(&id, &procs),
            "a foreign tmux lens is not the agent owner"
        );
        fs::write(
            crate::host::host_file_path(&dir),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();
        assert!(k.owns_agent_endpoint(&id, &procs));
        let foreign = Procs::of([(4101, proc("claude", Some(1)))]);
        assert!(
            !k.owns_agent_endpoint(&id, &foreign),
            "a dead host marker cannot control an outside writer"
        );
        let detached = Procs::of([(4100, proc("khor", None)), (4101, proc("claude", Some(1)))]);
        assert!(!k.owns_agent_endpoint(&id, &detached));
        let future = Proc {
            started_ms: now_ms() + 100_000,
            ..proc("another process", None)
        };
        assert!(
            !k.owns_agent_endpoint(
                &id,
                &Procs::of([(4100, future), (4101, proc("claude", Some(4100)))])
            ),
            "a PID reused after the marker is not its owner"
        );
        k.reseat(&id, "gui", "fixture", Some(4100)).unwrap();
        let ghost = crate::host::HostFile {
            child_pid: 4100,
            ..marker
        };
        fs::write(
            crate::host::host_file_path(&dir),
            serde_json::to_vec(&ghost).unwrap(),
        )
        .unwrap();
        assert!(
            k.owns_agent_endpoint(&id, &procs),
            "the GUI host owns its agent connection directly"
        );
        let own_pid = std::process::id();
        k.reseat(&id, "gui", "fixture", Some(own_pid)).unwrap();
        let own_host = crate::host::HostFile {
            host_pid: own_pid,
            child_pid: own_pid,
            port: 1234,
            cookie: "host".into(),
        };
        let view = crate::host::HostFile {
            host_pid: own_pid,
            child_pid: own_pid,
            port: 4321,
            cookie: "foreign-view".into(),
        };
        fs::write(
            crate::host::host_file_path(&dir),
            serde_json::to_vec(&own_host).unwrap(),
        )
        .unwrap();
        fs::write(
            crate::host::view_file_path(&dir),
            serde_json::to_vec(&view).unwrap(),
        )
        .unwrap();
        let viewer = k.discovering(Arc::new(crate::adaptor::Discovery::empty().with_procs(
            Procs::of([
                (
                    own_pid,
                    Proc {
                        name: "khor-fixture".into(),
                        started_ms: 0,
                        cwd: None,
                        ppid: None,
                    },
                ),
                (
                    4101,
                    Proc {
                        name: "agent-fixture".into(),
                        started_ms: 0,
                        cwd: None,
                        ppid: Some(own_pid),
                    },
                ),
            ]),
        )));
        assert_eq!(
            viewer.terminal_address(&id).unwrap().port,
            1234,
            "the protocol reaches its own GUI host"
        );
        assert!(
            viewer
                .terminal_view_address(&id)
                .err()
                .expect("GUI cannot target a terminal view")
                .contains("external terminal view is read-only"),
            "a live GUI host cannot authorize input into the foreign view beside it"
        );
        viewer.reseat(&id, "tui", "fixture", Some(own_pid)).unwrap();
        let own_pty = crate::host::HostFile {
            child_pid: 4101,
            ..own_host
        };
        fs::write(
            crate::host::host_file_path(&dir),
            serde_json::to_vec(&own_pty).unwrap(),
        )
        .unwrap();
        let address = viewer.terminal_view_address(&id).unwrap();
        assert_eq!(
            (address.port, address.cookie.as_str()),
            (1234, "host"),
            "the agent terminal only targets its proved PTY, never a foreign view"
        );
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
        k.register(&id, "gui", "stub", Some(std::process::id()), None)
            .unwrap();
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
        k.register(&id, "tui", "khor", Some(std::process::id()), None)
            .unwrap();

        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.unwrap().state, row.unread), (State::Busy, 0));

        k.report(&id, State::Blocked, Source::Reported).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(
            (row.state.unwrap().state, row.unread),
            (State::Blocked, 0),
            "waiting for approval"
        );

        k.report(&id, State::Done, Source::Reported).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(
            (row.state.unwrap().state, row.unread),
            (State::Done, 1),
            "turn done, not looked at"
        );
        let row = &k.rows(|_| i64::MAX)[0];
        assert_eq!(
            (row.state.unwrap().state, row.unread),
            (State::Idle, 0),
            "looked at"
        );

        k.record_exit(&id, 0).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(
            (row.state.unwrap().state, row.unread),
            (State::Idle, 0),
            "a clean tui exit is idle"
        );

        let id2 = sid("tui/def456");
        k.register(&id2, "tui", "khor", Some(std::process::id()), None)
            .unwrap();
        k.record_exit(&id2, 3).unwrap();
        let row = k.rows(|_| 0).into_iter().find(|r| r.id == id2).unwrap();
        assert_eq!(
            (row.state.unwrap().state, row.unread),
            (State::Failed, 0),
            "failed sinks, no badge"
        );
    }

    /// Shell exit mapping differs from tui: a clean exit is 完成 until
    /// looked at (the tui showed its ending on screen; a wrapped command
    /// may have ended long after you left).
    #[test]
    fn a_clean_shell_exit_waits_to_be_looked_at() {
        let root = tmp("shell");
        let k = kind_at(&root);
        let id = sid("shell/run1");
        k.register(&id, "shell", "build", Some(std::process::id()), None)
            .unwrap();
        k.record_exit(&id, 0).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.unwrap().state, row.unread), (State::Done, 1));
        let row = &k.rows(|_| i64::MAX)[0];
        assert_eq!((row.state.unwrap().state, row.unread), (State::Idle, 0));
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
        k.register(&id, "shell", "x", Some(dead_pid()), None)
            .unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.state.unwrap().state, State::Failed);
        assert!(
            k.report(&id, State::Busy, Source::Reported).unwrap_err() == msg::session_over(&id.0),
            "a dead session refuses reports by name"
        );

        let id2 = sid("tui/nopid1");
        k.register(&id2, "tui", "x", None, None).unwrap();
        k.report(&id2, State::Busy, Source::Reported).unwrap();
        let row = k.rows(|_| 0).into_iter().find(|r| r.id == id2).unwrap();
        assert_eq!(
            row.state.unwrap().state,
            State::Busy,
            "no pid on record = the word stands"
        );
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
        assert_eq!(k.rows(|_| 0)[0].state.unwrap().state, State::Failed);

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
            row.state.unwrap().state,
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
        assert_eq!(k.rows(|_| 0)[0].state.unwrap().state, State::Failed);
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
    }

    /// Nothing a hook sends may leave the registry, name a failed state,
    /// or resurrect an ended session.
    #[test]
    fn hostile_and_out_of_order_input_is_refused_by_name() {
        let root = tmp("hostile");
        let k = kind_at(&root);

        for bad in [
            "tui/../../etc",
            "tui/",
            "/leaf",
            "noslash",
            "tui/UPPER CASE!",
        ] {
            assert!(
                k.register(&sid(bad), "tui", "x", None, None).is_err(),
                "{bad:?} should not register"
            );
        }
        assert_eq!(clean_leaf("../../etc/passwd"), "etcpasswd");
        assert_eq!(clean_leaf("A-B_C.9"), "a-bc9");

        let id = sid("tui/ok1");
        k.register(&id, "tui", "x", None, None).unwrap();
        assert_eq!(
            k.report(&id, State::Failed, Source::Reported).unwrap_err(),
            msg::failed_is_not_reportable()
        );
        k.record_exit(&id, 0).unwrap();
        assert_eq!(
            k.report(&id, State::Busy, Source::Reported).unwrap_err(),
            msg::session_over(&id.0)
        );
        k.record_exit(&id, 7).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(
            row.state.unwrap().state,
            State::Idle,
            "the first recorded ending wins"
        );
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
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc {
                name: "claude".into(),
                started_ms: 1_700_000_000_000,
                cwd: None,
                ppid: None,
            },
        )]);
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(procs));
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(discovery);

        // Discovery alone: one row, wearing the word from disk.
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.0, "tui/11111111-1111-4111-8111");
        assert_eq!(rows[0].state.unwrap().state, State::Blocked);
        assert_eq!(rows[0].title, "one-aa");

        // Now the hook registers the same claude session and reports a
        // different word.
        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        // The hook's own shape: it names no pid (`claude.rs`'s `ensure`
        // passes `None`) — a hook is the session's process speaking, not
        // a host beside it, and `rows` ranks a report by that.
        k.register(&id, "tui", "one", None, None).unwrap();
        k.report(&id, State::Done, Source::Reported).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "one session, one row — not two");
        assert_eq!(rows[0].id, id);
        assert_eq!(
            (rows[0].state.unwrap().state, rows[0].unread),
            (State::Done, 1),
            "the registered row wins: it is the only one that can say 完成"
        );
        assert_eq!(rows[0].title, "one");
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
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc {
                name: "claude".into(),
                started_ms: 1_700_000_000_000,
                cwd: None,
                ppid: None,
            },
        )]);
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(procs));
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(discovery);

        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        // The hook's own shape: it names no pid (`claude.rs`'s `ensure`
        // passes `None`) — a hook is the session's process speaking, not
        // a host beside it, and `rows` ranks a report by that.
        k.register(&id, "tui", "one", None, None).unwrap();

        // The screen reader says the session is idle. The vendor's own
        // file says it is sitting on a permission prompt.
        k.report(&id, State::Idle, Source::Screen).unwrap();
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "still one session");
        assert_eq!(
            rows[0].state.unwrap().state,
            State::Blocked,
            "the vendor's own file outranks a guess about its screen",
        );

        // Control: the identical disagreement, reported instead of read.
        k.report(&id, State::Idle, Source::Reported).unwrap();
        assert_eq!(
            k.rows(|_| 0)[0].state.unwrap().state,
            State::Idle,
            "control: a reported word still wins, so the source is the only thing deciding",
        );
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
        k.register(&id, "tui", "agent", Some(std::process::id()), None)
            .unwrap();
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
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc {
                name: "claude".into(),
                started_ms: 1_700_000_000_000,
                cwd: None,
                ppid: None,
            },
        )]);
        let discovery = Arc::new(Discovery::at(&vendors).with_procs(procs));
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(discovery);

        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        // The hook's own shape: it names no pid (`claude.rs`'s `ensure`
        // passes `None`) — a hook is the session's process speaking, not
        // a host beside it, and `rows` ranks a report by that.
        k.register(&id, "tui", "one", None, None).unwrap();
        k.report(&id, State::Idle, Source::Reported).unwrap();

        // Rewrite the entry the way an older khor would have left it.
        // Built by *deleting* the key from what this khor writes rather
        // than by spelling the old shape out here: that keeps the other
        // fields encoded exactly as the real writer encodes them, so the
        // test cannot pass because it happened to hand-write a word the
        // way the parser likes it.
        let dir = k.dir(&id).unwrap();
        let path = dir.join("state.json");
        let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        doc.as_object_mut()
            .unwrap()
            .remove("source")
            .expect("the new writer put one there");
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
            k.rows(|_| 0)[0].state.unwrap().state,
            State::Idle,
            "an entry from before this field stands against the sweep",
        );
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
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let procs = Procs::of([(
            4001,
            Proc {
                name: "claude".into(),
                started_ms: 1_700_000_000_000,
                cwd: None,
                ppid: None,
            },
        )]);
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
            .discovering(Arc::new(Discovery::at(&vendors).with_procs(procs)));

        // The registry claims the id first, with nothing to say about
        // whose it is — what `khor run --tui` leaves behind.
        let id = crate::adaptor::id_for("11111111-1111-4111-8111-111111111111");
        k.register(&id, "tui", "one", Some(std::process::id()), None)
            .unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "still one row");
        assert_eq!(
            rows[0].category.as_deref(),
            Some("claude"),
            "the sweep read claude's own files, so it can place what the registry could not"
        );
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
        use crate::adaptor::{Discovery, Proc, Procs, tmux::Tmux};

        let root = tmp("tmux-dedup");
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
        let shell = |pid: u32, parent: u32| {
            (
                pid,
                Proc {
                    name: "-zsh".into(),
                    started_ms: 1_700_000_000_000,
                    cwd: None,
                    ppid: Some(parent),
                },
            )
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
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/tmp\x1f%2\x1fwith-claude\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs.clone())
                .with_tmux(Tmux::at_binary(&holding)),
        ));
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "one running process is one row");
        assert_eq!(rows[0].id.0, "tui/11111111-1111-4111-8111");
        assert_eq!(
            rows[0].category.as_deref(),
            Some("claude"),
            "and it is claude's, not a shell"
        );
        assert!(
            rows[0].term,
            "the fold keeps what it computed: this agent's terminal lives in that tmux session"
        );

        // The control: the same session, holding nothing anyone listed.
        let empty = crate::adaptor::tmux::fake_tmux(
            "empty-pane",
            "$7\x1f1786245338\x1f1786245400\x1f3002\x1f0\x1fzsh\x1f/tmp\x1f%3\x1fjust-a-shell\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&empty)),
        ));
        let mut rows = k.rows(|_| 0);
        rows.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        assert_eq!(rows.len(), 2, "a bare tmux session is a row of its own");
        assert_eq!(rows[0].id.0, "shell/1786245338-7");
        assert_eq!(rows[0].title, "just-a-shell");
        assert_eq!(
            rows[0].category.as_deref(),
            Some(khor_core::category::SHELL)
        );
        assert_eq!(rows[0].state.unwrap().state, State::Idle);
        assert!(
            rows[0].term,
            "a discovered tmux session is exactly what the bridge attaches to"
        );
        assert!(
            !rows[1].term,
            "control: an agent held by no multiplexer has no route to a terminal here"
        );
    }

    /// The fold against a **registry entry**: a bridge row left behind
    /// for a tmux session — its dir is there, **its host is not
    /// running**, and nobody put it on the list — is the duplicate
    /// wearing a registry dir: one running process, one row, and the row
    /// is the agent's, wearing the route.
    ///
    /// Narrower since 2026-09-30 (docs/SESSION.md 形态随进程变): a bridge
    /// whose host runs is a terminal somebody has open, and it holds the
    /// seat instead — `a_terminal_that_starts_an_agent_becomes_that_agent_and_stays_open`.
    #[test]
    fn an_old_bridge_row_folds_into_the_agent_row_it_duplicates() {
        use crate::adaptor::{Discovery, Proc, Procs, tmux::Tmux};

        let root = tmp("bridge-fold");
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude");
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
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/tmp\x1f%4\x1fwith-claude\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&vendors)
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&holding)),
        ));
        // The bridge row, registered the way `attach_multiplexed` would
        // have registered it for this very session.
        let bridge = sid("shell/1786245338-7");
        k.register(
            &bridge,
            "shell",
            "with-claude",
            None,
            Some(khor_core::category::SHELL),
        )
        .unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.len(),
            1,
            "the bridge row stops listing: one process, one row"
        );
        assert_eq!(
            rows[0].id.0, "tui/11111111-1111-4111-8111",
            "and the row is the agent's"
        );
        assert!(rows[0].term, "wearing the route");
    }

    /// A grouped twin session is furniture, not a second row: tmux lists
    /// the same panes again under the twin's own id (measured — see the
    /// comment in `rows`), and the twin a bridge's client creates would
    /// otherwise appear as a phantom row for exactly as long as someone
    /// is looking at the terminal face.
    #[test]
    fn a_grouped_twin_of_a_listed_session_is_not_a_second_row() {
        use crate::adaptor::{Discovery, Proc, Procs, tmux::Tmux};

        let root = tmp("tmux-twin");
        let procs = Procs::of([(
            3001,
            Proc {
                name: "-zsh".into(),
                started_ms: 1_700_000_000_000,
                cwd: None,
                ppid: Some(1),
            },
        )]);
        // The same pane, listed twice: once by the original session,
        // once by a grouped twin created later.
        let fake = crate::adaptor::tmux::fake_tmux(
            "twin",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/tmp\x1f%5\x1fbase\n\
             $9\x1f1786245400\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/tmp\x1f%6\x1fbase-1\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::empty()
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&fake)),
        ));
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "one set of panes is one row");
        assert_eq!(
            rows[0].id.0, "shell/1786245338-7",
            "and the original session is the row"
        );
    }

    /// The transcript outranks the hosted screen line for an agent row
    /// (会话身份批): a TUI's last non-empty screen line is its chrome —
    /// "⏵⏵ bypass permissions on…" — true of the screen, useless as a
    /// preview. Three rows, one edge each:
    /// - a row whose id leaf finds a transcript: the transcript wins;
    /// - a row with no transcript anywhere: no line at all, since the
    ///   screen's is chrome (SESS-BRIEF 1d);
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
        k.register(&hooked, "tui", "one", None, Some("claude"))
            .unwrap();
        fs::write(
            k.dir_of(&hooked).unwrap().join("last.txt"),
            "⏵⏵ bypass permissions on",
        )
        .unwrap();

        let bare = sid("tui/no-transcript-here");
        k.register(&bare, "tui", "two", None, Some("claude"))
            .unwrap();
        fs::write(k.dir_of(&bare).unwrap().join("last.txt"), "$ make test").unwrap();

        let minted = sid("tui/khormint00");
        k.register(&minted, "tui", "three", None, Some("claude"))
            .unwrap();
        k.learn_vendor_leaf(&minted, &clean_leaf(uuid)).unwrap();
        fs::write(
            k.dir_of(&minted).unwrap().join("last.txt"),
            "⏵⏵ auto mode on",
        )
        .unwrap();

        let rows = k.rows(|_| 0);
        let last = |id: &SessionId| rows.iter().find(|r| r.id == *id).unwrap().last.clone();
        assert_eq!(last(&hooked).as_deref(), Some("from the transcript"));
        assert_eq!(
            last(&bare),
            None,
            "no transcript: no line, not the screen's"
        );
        assert_eq!(
            last(&minted).as_deref(),
            Some("from the transcript"),
            "a khor-minted row reaches its transcript through the vendor leaf"
        );
    }

    /// A claude started from khor's own form (`claude --session-id`):
    /// the row id's leaf is the transcript's, no hook ever ran, the host
    /// is live and its screen line is Claude Code's bottom line. The
    /// transcript holds the conversation first and the CLI's bookkeeping
    /// after it — attachments, system notes, modes — and the answer is
    /// still the conversation's newest sentence (SESS-BRIEF 1a).
    #[test]
    fn a_khor_started_claude_row_says_its_transcript_past_the_bookkeeping() {
        // Measured on the owner's (lead, 2026-10-07): a `prompt_snapshot`
        // attachment of 131 887 bytes on one line right after the answer,
        // so one bookkeeping line alone outgrew the old 64 KiB tail.
        for (tag, pad) in [("small", 200usize), ("large", 130 * 1024)] {
            let root = tmp(&format!("preview-owner-{tag}"));
            let proj = root.join(".claude/projects/-Users-someone");
            fs::create_dir_all(&proj).unwrap();
            let uuid = "20ccc275-8480-4c8e-ba73-0123456789ab";
            let blob = "x".repeat(pad);
            let lines = [
                r#"{"type":"user","message":{"role":"user","content":"what is in this repo"},"sessionId":"S"}"#.to_owned(),
                r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Three crates and an app."}]},"sessionId":"S"}"#.to_owned(),
                format!(r#"{{"type":"attachment","attachment":{{"type":"prompt_snapshot","content":"{blob}"}},"sessionId":"S"}}"#),
                r#"{"type":"system","subtype":"turn_duration","durationMs":5123,"sessionId":"S"}"#.to_owned(),
                r#"{"type":"system","subtype":"stop_hook_summary","sessionId":"S"}"#.to_owned(),
                r#"{"type":"last-prompt","lastPrompt":"what is in this repo","sessionId":"S"}"#.to_owned(),
                r#"{"type":"mode","mode":"normal","sessionId":"S"}"#.to_owned(),
                r#"{"type":"permission-mode","permissionMode":"auto","sessionId":"S"}"#.to_owned(),
                r#"{"type":"atis-latch","sessionId":"S"}"#.to_owned(),
            ];
            fs::write(proj.join(format!("{uuid}.jsonl")), lines.join("\n") + "\n").unwrap();
            let k = kind_at(&root);
            let id = sid(&format!("tui/{}", clean_leaf(uuid)));
            k.register(&id, "tui", "claude", None, Some("claude"))
                .unwrap();
            let dir = k.dir_of(&id).unwrap();
            let me = std::process::id();
            fs::write(
                crate::host::host_file_path(&dir),
                format!(r#"{{"port":1,"cookie":"c","host_pid":{me},"child_pid":{me}}}"#),
            )
            .unwrap();
            fs::write(dir.join("last.txt"), "⏵⏵ auto mode on (shift+tab to cycle)").unwrap();
            let rows = k.rows(|_| 0);
            let row = rows.iter().find(|r| r.id == id).unwrap();
            assert_eq!(
                row.last.as_deref(),
                Some("Three crates and an app."),
                "{tag} bookkeeping after the answer"
            );
        }
        // An agent khor cannot read the words of yet: no line, not its chrome.
        let root = tmp("preview-unread-vendor");
        let k = kind_at(&root);
        let id = sid("tui/some-codex-session");
        k.register(&id, "tui", "codex", None, Some("codex"))
            .unwrap();
        fs::write(
            k.dir_of(&id).unwrap().join("last.txt"),
            "› Ask Codex to do anything",
        )
        .unwrap();
        let rows = k.rows(|_| 0);
        assert_eq!(rows.iter().find(|r| r.id == id).unwrap().last, None);
    }

    /// A codex row says codex's newest sentence, as a claude row says
    /// claude's (SESS-BRIEF 1c): the rollout behind the id's leaf, its
    /// `event_msg` words, past the token counts after them — and not
    /// the composer's bottom line from the screen.
    #[test]
    fn a_codex_row_says_its_rollouts_newest_sentence() {
        let root = tmp("preview-codex");
        let day = root.join(".codex/sessions/2026/10/06");
        fs::create_dir_all(&day).unwrap();
        let uuid = "019fd755-0000-7000-8000-0000000000aa";
        let lines = [
            r#"{"type":"session_meta","payload":{"id":"x","cwd":"/w"}}"#,
            r#"{"type":"event_msg","payload":{"type":"user_message","message":"run the tests"}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"All 40 pass."}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count","info":null}}"#,
            r#"{"type":"turn_context","payload":{"cwd":"/w"}}"#,
        ];
        fs::write(
            day.join(format!("rollout-2026-10-06T00-00-00-{uuid}.jsonl")),
            lines.join("\n") + "\n",
        )
        .unwrap();
        let k = kind_at(&root);
        let id = crate::adaptor::id_for(uuid);
        k.register(&id, "tui", "codex", None, Some("codex"))
            .unwrap();
        fs::write(
            k.dir_of(&id).unwrap().join("last.txt"),
            "› Ask Codex to do anything",
        )
        .unwrap();
        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.iter().find(|r| r.id == id).unwrap().last.as_deref(),
            Some("All 40 pass.")
        );
    }

    /// **A hosted agent row learns its vendor's session from the sweep,
    /// not from hooks** (SESS-BRIEF 2b). `khor run --tui claude` mints
    /// its own id and no hook on the owner's Mac ever names the
    /// session, so the row could find no transcript, and the claude the
    /// sweep joined by pid was a second row. The pid sits under the
    /// row's host: the row records the leaf (and the vendor, where it
    /// was unfiled), wears the sighting, and says the transcript's
    /// newest sentence — on this poll and on the next.
    #[test]
    fn a_hosted_claude_row_learns_its_session_from_the_pid_under_its_host() {
        use crate::adaptor::{Discovery, Procs};

        let root = tmp("hosted-learns-claude");
        let uuid = "11111111-1111-4111-8111-111111111111";
        let proj = root.join(".claude/projects/-w-one");
        fs::create_dir_all(&proj).unwrap();
        fs::write(
            proj.join(format!("{uuid}.jsonl")),
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"From the transcript."}]}}"#.to_owned() + "\n",
        )
        .unwrap();
        let host = FakeHost::up();
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&fixture_vendors()).with_procs(Procs::of([proc_under(
                4001,
                "claude",
                host.pid(),
            )])),
        ));
        let id = sid("tui/khormint01");
        k.register(&id, "tui", "claude", Some(host.pid()), None)
            .unwrap();
        let dir = k.dir_of(&id).unwrap();
        host.serve(&dir, 4001);
        fs::write(dir.join("last.txt"), "⏵⏵ auto mode on").unwrap();

        for poll in ["first", "second"] {
            let rows = k.rows(|_| 0);
            let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
            assert_eq!(
                ids,
                vec![id.0.as_str()],
                "{poll} poll: one row, the hosted one"
            );
            let row = &rows[0];
            assert_eq!(row.category.as_deref(), Some("claude"), "{poll} poll");
            assert_eq!(
                row.state.unwrap().state,
                State::Blocked,
                "{poll} poll: claude's own word"
            );
            assert_eq!(
                row.last.as_deref(),
                Some("From the transcript."),
                "{poll} poll"
            );
        }
        assert_eq!(
            k.vendor_leaf_of(&id),
            Some(clean_leaf(uuid)),
            "the leaf is on record"
        );
        assert_eq!(
            read_meta(&dir).unwrap().category.as_deref(),
            Some("claude"),
            "and the vendor, where the row was unfiled"
        );
    }

    /// **The newest session in the host wins.** A `/clear` (or any new
    /// claude session) in the same host gives the same pid a new
    /// session id; a leaf recorded once and kept would have the row read
    /// a transcript nobody writes any more. The pid join says the live
    /// leaf every sweep, so the recorded one follows it.
    #[test]
    fn a_hosted_claude_row_follows_a_new_session_in_the_same_process() {
        use crate::adaptor::{Discovery, Procs};

        let root = tmp("hosted-follows-claude");
        let sessions = root.join(".claude/sessions");
        let proj = root.join(".claude/projects/-w-one");
        fs::create_dir_all(&sessions).unwrap();
        fs::create_dir_all(&proj).unwrap();
        let status =
            fs::read_to_string(fixture_vendors().join(".claude/sessions/4001.json")).unwrap();
        let first = "11111111-1111-4111-8111-111111111111";
        let second = "22222222-2222-4222-8222-222222222222";
        let third = "33333333-3333-4333-8333-333333333333";
        for (uuid, said) in [
            (first, "The first."),
            (second, "The second."),
            (third, "The third."),
        ] {
            fs::write(
                proj.join(format!("{uuid}.jsonl")),
                format!(r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{said}"}}]}}}}"#) + "\n",
            )
            .unwrap();
        }
        let host = FakeHost::up();
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&root).with_procs(Procs::of([proc_under(4001, "claude", host.pid())])),
        ));
        let id = sid("tui/khormint03");
        k.register(&id, "tui", "claude", Some(host.pid()), Some("claude"))
            .unwrap();
        host.serve(&k.dir_of(&id).unwrap(), 4001);

        for (uuid, said) in [
            (first, "The first."),
            (second, "The second."),
            (third, "The third."),
        ] {
            fs::write(sessions.join("4001.json"), status.replace(first, uuid)).unwrap();
            let rows = k.rows(|_| 0);
            let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
            assert_eq!(ids, vec![id.0.as_str()], "{said} one row, the hosted one");
            assert_eq!(
                rows[0].last.as_deref(),
                Some(said),
                "the session running now"
            );
            assert_eq!(
                k.vendor_leaf_of(&id),
                Some(clean_leaf(uuid)),
                "{said} on record"
            );
        }
    }

    /// The same for codex (SESS-BRIEF 2b), which cannot be told a
    /// session id at start at all: `khor run --tui codex` reaches its
    /// rollout only through the process the sweep joined to it.
    #[test]
    fn a_hosted_codex_row_learns_its_rollout_from_the_pid_under_its_host() {
        use crate::adaptor::{Discovery, Proc, Procs};

        let root = tmp("hosted-learns-codex");
        let uuid = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
        let day = root.join(".codex/sessions/2026/08/06");
        fs::create_dir_all(&day).unwrap();
        let lines = [
            r#"{"type":"session_meta","payload":{"id":"x","cwd":"/w/alpha"}}"#,
            r#"{"type":"event_msg","payload":{"type":"agent_message","message":"Done, all green."}}"#,
        ];
        fs::write(
            day.join(format!("rollout-2026-08-06T21-49-18-{uuid}.jsonl")),
            lines.join("\n") + "\n",
        )
        .unwrap();
        let host = FakeHost::up();
        let codex = Proc {
            name: "codex".into(),
            started_ms: 1_786_024_158_000,
            cwd: Some(PathBuf::from("/w/alpha")),
            ppid: Some(host.pid()),
        };
        let vendors =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/codex");
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
            .discovering(Arc::new(Discovery::at(&vendors).with_procs(
                Procs::of([(700, codex)]).with_commands([(700, vec!["codex".into()])]),
            )));
        let id = sid("tui/khormint02");
        k.register(&id, "tui", "codex", Some(host.pid()), Some("codex"))
            .unwrap();
        let dir = k.dir_of(&id).unwrap();
        host.serve(&dir, 700);
        fs::write(dir.join("last.txt"), "› Ask Codex to do anything").unwrap();

        for poll in ["first", "second"] {
            let rows = k.rows(|_| 0);
            let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
            assert_eq!(
                ids,
                vec![id.0.as_str()],
                "{poll} poll: one row, the hosted one"
            );
            assert_eq!(
                rows[0].last.as_deref(),
                Some("Done, all green."),
                "{poll} poll"
            );
        }
        assert_eq!(k.vendor_leaf_of(&id), Some(clean_leaf(uuid)));
    }

    #[test]
    #[cfg(unix)]
    fn default_daemon_client_keeps_category_without_borrowing_thread_state() {
        use crate::adaptor::{Discovery, Proc, Procs};
        use std::io::Write;

        let root = tmp("shell-codex-daemon");
        let vendors = root.to_path_buf();
        let codex_home = vendors.join(".codex");
        let day = codex_home.join("sessions/2026/08/06");
        let locks = codex_home.join("thread-writer-locks");
        fs::create_dir_all(&day).unwrap();
        fs::create_dir_all(&locks).unwrap();
        let uuid = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
        let path = day.join(format!("rollout-2026-08-06T21-49-18-{uuid}.jsonl"));
        fs::write(&path, format!("{}\n{}\n{}\n",
            serde_json::json!({"type":"session_meta","payload":{"id":uuid,"cwd":"/project","originator":"codex-tui"}}),
            serde_json::json!({"type":"event_msg","payload":{"type":"user_message","message":"latest question"}}),
            serde_json::json!({"timestamp":"2026-08-06T13:49:19Z","type":"event_msg","payload":{"type":"task_started"}}),
        )).unwrap();
        fs::File::create(locks.join(".coordination.lock")).unwrap();
        let writer = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(locks.join(format!("{uuid}.lock")))
            .unwrap();
        writer.try_lock().unwrap();
        let host = FakeHost::up();
        let shell = proc_under(5001, "zsh", host.pid());
        let codex = (
            700,
            Proc {
                name: "codex".into(),
                started_ms: 1_786_024_158_308 - 66_000,
                cwd: Some("/project".into()),
                ppid: Some(5001),
            },
        );
        let procs = Procs::of([shell, codex]).with_commands([(700, vec!["codex".into()])]);
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
            .discovering(Arc::new(Discovery::at(&vendors).with_procs(procs)));
        let id = sid("shell/codex-terminal");
        k.register(&id, "shell", "khor-codex", Some(host.pid()), Some("shell"))
            .unwrap();
        host.serve(&k.dir_of(&id).unwrap(), 5001);
        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.len(),
            2,
            "unbound terminal and loaded thread have separate identities"
        );
        let terminal = rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(terminal.kind.0, khor_core::kind::TUI);
        assert_eq!(terminal.category.as_deref(), Some("codex"));
        assert!(!terminal.state_known());
        assert!(terminal.last.is_none());
        assert_eq!(terminal.unread, 0);
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}\n{}",
            serde_json::json!({"type":"event_msg","payload":{"type":"agent_message","message":"latest answer"}}),
            serde_json::json!({"timestamp":"2026-08-06T13:49:20Z","type":"event_msg","payload":{"type":"task_complete"}}),
        ).unwrap();
        drop(file);
        for _ in 0..2 {
            let rows = k.rows(|_| 0);
            let terminal = rows.iter().find(|r| r.id == id).unwrap();
            assert!(!terminal.state_known());
            assert!(terminal.last.is_none());
            assert_eq!(terminal.unread, 0);
            let thread = rows.iter().find(|r| r.id != id).unwrap();
            assert_eq!(thread.last.as_deref(), Some("latest answer"));
            assert_eq!(
                (thread.state.unwrap().state, thread.unread),
                (State::Done, 1)
            );
        }
        drop(writer);
        // Parallel process spawns can briefly inherit the writer descriptor
        // until exec closes it. Wait for observed lock release, not a sleep.
        let released = fs::File::open(locks.join(format!("{uuid}.lock"))).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match released.try_lock_shared() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                Err(error) => panic!("fixture writer did not release: {error:?}"),
            }
        }
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].category.as_deref(), Some("codex"));
        assert!(!rows[0].state_known());
        assert!(!rows[0].dead, "losing the writer does not end the client");
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
        use crate::adaptor::{Discovery, Proc, Procs, tmux::Tmux};

        let root = tmp("tmux-registry");
        let k_root = root.to_path_buf();
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
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/tmp\x1f%7\x1frunning-khor\n",
        );
        let k = LiveKind::new(k_root, DeviceId([9; 32])).discovering(Arc::new(
            Discovery::empty()
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&fake)),
        ));
        // The registry's row, its pid a child of the pane.
        let id = sid("shell/run1");
        k.register(&id, "shell", "build", Some(me), None).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.len(),
            1,
            "the command khor started is the row; the pane is not a second"
        );
        assert_eq!(rows[0].id, id);
    }

    // ── 形态随进程变 (docs/SESSION.md, 2026-09-30) ──────────────
    //
    // Three walks over a fake process tree, one per host shape, each
    // ending in the control that only a host going away ends a row.
    // The claude is the vendors fixture's pid 4001 (session
    // 11111111-…, waiting on a permission prompt: 待批); what moves is
    // whether that pid is in the table, and under whom.

    const T_ID: &str = "shell/1786245338-7";
    const AGENT: &str = "tui/11111111-1111-4111-8111";

    fn fixture_vendors() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/claude")
    }

    fn proc_under(pid: u32, name: &str, parent: u32) -> (u32, crate::adaptor::Proc) {
        (
            pid,
            crate::adaptor::Proc {
                name: name.into(),
                started_ms: 1_700_000_000_000,
                cwd: None,
                ppid: Some(parent),
            },
        )
    }

    /// A stand-in for a running khor host: a real process, so the
    /// registry reads its host as running, and **never this test's own
    /// pid** — a reaper that wrongly closed the row would signal it.
    struct FakeHost(std::process::Child);

    impl FakeHost {
        fn up() -> FakeHost {
            FakeHost(
                std::process::Command::new("sleep")
                    .arg("60")
                    .spawn()
                    .unwrap(),
            )
        }

        fn pid(&self) -> u32 {
            self.0.id()
        }

        fn still_up(&mut self) -> bool {
            self.0.try_wait().unwrap().is_none()
        }

        /// Writes the host marker a running host leaves in `dir`.
        fn serve(&self, dir: &Path, child: u32) {
            fs::write(
                crate::host::host_file_path(dir),
                serde_json::to_vec(&crate::host::HostFile {
                    port: 1,
                    cookie: "fake".into(),
                    host_pid: self.pid(),
                    child_pid: child,
                })
                .unwrap(),
            )
            .unwrap();
        }
    }

    impl Drop for FakeHost {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// **tmux, the agent leaves.** The row the person has stays on the
    /// list as the same row — same id — in the terminal's form; the
    /// tmux session does not come back as a second row; nothing is
    /// 已结束. Only once the tmux session itself is gone does the row end.
    /// **A hosted shell's row never shows its prompt** (item 26; the
    /// owner's row read a powerlevel10k prompt line under a host from
    /// before the shim). With no command from the ledger the row reads the
    /// shell's own working directory, then the ledger's `OSC 7` directory
    /// once there is one; a command still wins.
    #[test]
    fn a_hosted_shell_without_marks_reads_where_it_stands_not_its_prompt() {
        let root = tmp("row-glyphs");
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]));
        let id = sid("shell/rowglyphs");
        k.register(&id, "shell", "shell", None, None).unwrap();
        let dir = k.dir_of(&id).unwrap();
        let host = FakeHost::up();
        // The child is a real process whose cwd the OS reports: the host
        // stand-in itself, started in this test's own directory.
        host.serve(&dir, host.pid());
        fs::write(dir.join("last.txt"), "╭─ ◖ /Library ◗ ✔ 20:58").unwrap();
        let row = || {
            k.rows(|_| 0)
                .into_iter()
                .find(|r| r.id == id)
                .expect("the row")
        };
        let here = std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            row().doing.and_then(|d| d.cwd).as_deref(),
            Some(here.as_str()),
            "workspace receives the live shell cwd without OSC marks"
        );
        let last = row().last.expect("a line");
        assert!(!last.contains('◖'), "the prompt's glyphs: {last:?}");
        assert_eq!(last, tilde(&here));
        let saved = |doing: khor_core::Doing| {
            crate::commands::write_saved(
                &dir,
                &crate::commands::Saved {
                    doing,
                    ..Default::default()
                },
            )
        };
        saved(khor_core::Doing {
            cwd: Some("/w/proj".into()),
            ..Default::default()
        });
        assert_eq!(row().last.as_deref(), Some("/w/proj"));
        assert_eq!(
            row().title,
            "proj",
            "unnamed identity follows current directory"
        );
        saved(khor_core::Doing {
            cwd: Some("/w/ACL2027".into()),
            title: Some("✳ vendor task".into()),
            ..Default::default()
        });
        assert_eq!(
            row().title,
            "ACL2027",
            "OSC status must not become the name"
        );
        saved(khor_core::Doing {
            title: Some("vim notes.md".into()),
            ..Default::default()
        });
        assert_eq!(row().last.as_deref(), Some("vim notes.md"));
        saved(khor_core::Doing {
            command: Some("make".into()),
            exit: Some(0),
            cwd: Some("/w/proj".into()),
            ..Default::default()
        });
        let r = row();
        assert_eq!(r.doing.and_then(|d| d.command).as_deref(), Some("make"));
        drop(host);
    }

    /// **A shell whose last command failed reads 失败, alive** (the
    /// terminal row, the owner 2026-10-08): from its prompt, after a
    /// non-zero end nobody has looked at; not after ^C (130) or a closed
    /// pipe (141), not once the row is looked at past the end, not while
    /// the next command runs; never `dead`.
    #[test]
    fn a_shell_whose_last_command_failed_reads_failed_until_looked_at() {
        let root = tmp("cmd-failed");
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]));
        let id = sid("shell/cmdfailed");
        k.register(&id, "shell", "shell", None, None).unwrap();
        let dir = k.dir_of(&id).unwrap();
        let host = FakeHost::up();
        host.serve(&dir, host.pid());
        k.report(&id, State::Idle, Source::Reported).unwrap();
        let ended = |exit: i32, running: bool| {
            let doing = khor_core::Doing {
                command: Some("make test".into()),
                running,
                started_ms: 5_000,
                ended_ms: (!running).then_some(9_000),
                exit: (!running).then_some(exit),
                ..Default::default()
            };
            crate::commands::write_saved(
                &dir,
                &crate::commands::Saved {
                    doing,
                    ..Default::default()
                },
            )
        };
        let row = |seen: i64| {
            let r = k
                .rows(|_| seen)
                .into_iter()
                .find(|r| r.id == id)
                .expect("the row");
            (r.state.unwrap().state, r.state.unwrap().at.0, r.dead)
        };
        ended(2, false);
        assert_eq!(
            row(0),
            (State::Failed, 9_000, false),
            "failed, alive, stamped at its end"
        );
        assert_eq!(row(9_000).0, State::Idle, "looked at past its end");
        ended(130, false);
        assert_eq!(row(0).0, State::Idle, "^C is not a failure");
        ended(141, false);
        assert_eq!(row(0).0, State::Idle, "a closed pipe is not a failure");
        ended(0, false);
        assert_eq!(row(0).0, State::Idle);
        ended(0, true);
        assert_eq!(
            row(0).0,
            State::Idle,
            "the next command running: the failure is over"
        );
        drop(host);
    }

    #[test]
    #[cfg(unix)]
    fn a_stopped_codex_does_not_demote_its_hosts_active_selected_thread() {
        use crate::adaptor::{Discovery, Proc, Procs};
        let root = tmp("hosted-codex-stopped-job");
        let a = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
        let b = "019fd755-e465-7aaa-8aaa-bbbbbbbbbbbb";
        let sessions = root.join(".codex/sessions/2026/08/06");
        let locks = root.join(".codex/thread-writer-locks");
        fs::create_dir_all(&sessions).unwrap();
        fs::create_dir_all(&locks).unwrap();
        fs::write(locks.join(".coordination.lock"), "").unwrap();
        let _writers: Vec<_> = [(a, "ANSWER_A"), (b, "ANSWER_B")].into_iter().map(|(uuid, answer)| {
            let meta = serde_json::json!({"type":"session_meta","payload":{"id":uuid,"cwd":"/project","originator":"codex-tui","source":"cli"}});
            let latest = serde_json::json!({"timestamp":"2026-08-06T21:49:20Z","type":"event_msg","payload":{"type":"agent_message","message":answer}});
            let done = serde_json::json!({"timestamp":"2026-08-06T21:49:21Z","type":"event_msg","payload":{"type":"task_complete"}});
            fs::write(sessions.join(format!("rollout-2026-08-06T21-49-18-{uuid}.jsonl")), format!("{meta}\n{latest}\n{done}\n")).unwrap();
            let file = fs::OpenOptions::new().create(true).truncate(false).read(true).write(true)
                .open(locks.join(format!("{uuid}.lock"))).unwrap();
            file.try_lock().unwrap();
            file
        }).collect();
        let host = FakeHost::up();
        let process = |name: &str, parent| Proc {
            name: name.into(),
            started_ms: 100,
            cwd: Some("/project".into()),
            ppid: parent,
        };
        let argv: Vec<String> = ["codex", "-c", "tui.terminal_title=[\"thread-id\"]"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let procs = Procs::of([
            (5001, process("zsh", Some(host.pid()))),
            (700, process("codex", Some(5001))),
            (701, process("codex", Some(5001))),
        ])
        .with_commands([(700, argv.clone()), (701, argv)]);
        let id = sid("shell/stopped-codex-host");
        let initial = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]));
        initial
            .register(
                &id,
                "shell",
                "SJTWorkshop fixture",
                Some(host.pid()),
                Some("shell"),
            )
            .unwrap();
        let dir = initial.dir_of(&id).unwrap();
        host.serve(&dir, 5001);
        for (label, stopped, title, selected) in [
            (
                "old suspended, current selected",
                vec![700],
                b,
                Some((b, 701, "ANSWER_B")),
            ),
            (
                "old resumed, new suspended",
                vec![701],
                a,
                Some((a, 700, "ANSWER_A")),
            ),
            ("both runnable stay ambiguous", vec![], b, None),
            ("all suspended cannot reuse title", vec![700, 701], b, None),
            (
                "current resumed after fallback",
                vec![700],
                b,
                Some((b, 701, "ANSWER_B")),
            ),
        ] {
            crate::commands::write_saved(
                &dir,
                &crate::commands::Saved {
                    doing: khor_core::Doing {
                        title: Some(format!("{}...", &title[..29])),
                        command: Some("codex".into()),
                        running: true,
                        cwd: Some("/project".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
                Discovery::at(&root).with_procs(procs.clone().with_suspended(stopped)),
            ));
            let rows = k.rows(|_| 0);
            let terminal = rows
                .iter()
                .find(|r| r.id == id)
                .expect("the original hosted terminal");
            if let Some((thread, pid, answer)) = selected {
                assert_eq!(
                    terminal.category.as_deref(),
                    Some("codex"),
                    "{label}: final host must keep its bound agent"
                );
                assert_eq!(terminal.kind.0, khor_core::kind::TUI, "{label}");
                assert!(terminal.state_known(), "{label}");
                assert_eq!(terminal.state.unwrap().state, State::Done, "{label}");
                assert_eq!(terminal.last.as_deref(), Some(answer), "{label}");
                assert_eq!(terminal.unread, 1, "{label}");
                assert_eq!(
                    k.sighting_of(&crate::adaptor::id_for(thread)).unwrap().1,
                    vec![pid],
                    "{label}"
                );
            } else {
                assert_eq!(terminal.category.as_deref(), Some("shell"), "{label}");
                assert_eq!(terminal.kind.0, khor_core::kind::SHELL, "{label}");
                assert!(
                    !matches!(terminal.last.as_deref(), Some("ANSWER_A" | "ANSWER_B")),
                    "{label}: no unrelated transcript"
                );
                assert_eq!(terminal.unread, 0, "{label}");
            }
        }
    }

    #[test]
    fn native_codex_selection_title_is_hidden_from_unnamed_shell_rows() {
        use crate::adaptor::{Discovery, Proc, Procs};
        let root = tmp("codex-title-hidden");
        let host = FakeHost::up();
        let process = |name: &str, ppid| Proc {
            name: name.into(),
            started_ms: 100,
            cwd: None,
            ppid,
        };
        let procs = Procs::of([
            (3001, process("zsh", None)),
            (501, process("codex", Some(3001))),
        ])
        .with_commands([(
            501,
            ["codex", "-c", "tui.terminal_title=[\"thread-id\"]"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )]);
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
            .discovering(Arc::new(Discovery::empty().with_procs(procs)));
        let id = sid("shell/titlemetadata");
        k.register(&id, "shell", "shell", Some(host.pid()), None)
            .unwrap();
        let dir = k.dir_of(&id).unwrap();
        host.serve(&dir, 3001);
        let title = "019fd755-e464-7aaa-8aaa-aaaaa...";
        let write = |title: &str| {
            crate::commands::write_saved(
                &dir,
                &crate::commands::Saved {
                    doing: khor_core::Doing {
                        title: Some(title.into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
        };
        write(title);
        let row = || k.rows(|_| 0).into_iter().find(|r| r.id == id).unwrap();
        assert!(
            row().doing.unwrap().title.is_none(),
            "an unnamed session must not display the native UUID"
        );
        assert_ne!(row().last.as_deref(), Some(title));
        assert_eq!(
            crate::commands::read_doing(&dir).unwrap().title.as_deref(),
            Some(title),
            "raw selection metadata survives for discovery"
        );
        write("my useful title");
        assert_eq!(
            row().doing.unwrap().title.as_deref(),
            Some("my useful title")
        );
    }

    #[test]
    fn native_codex_title_switches_identity_and_reopen_routes_with_old_writer_loaded() {
        use crate::adaptor::{Discovery, Proc, Procs, tmux::Tmux};
        let root = tmp("codex-title-switch");
        let a = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
        let b = "019fd755-e465-7aaa-8aaa-bbbbbbbbbbbb";
        let c = "019fd755-e466-7aaa-8aaa-cccccccccccc";
        let sessions = root.join(".codex/sessions/2026/08/06");
        let locks = root.join(".codex/thread-writer-locks");
        fs::create_dir_all(&sessions).unwrap();
        fs::create_dir_all(&locks).unwrap();
        fs::write(locks.join(".coordination.lock"), "").unwrap();
        let writers: Vec<_> = [a, b].into_iter().map(|id| {
            let metadata = serde_json::json!({"type":"session_meta","payload":{"id":id,"cwd":"/same","originator":"codex_cli_rs","source":"cli"}});
            let latest = serde_json::json!({"timestamp":"2026-08-06T21:49:20Z","type":"event_msg","payload":{"type":"agent_message","message":if id == a { "LATEST_A" } else { "LATEST_B" }}});
            let done = serde_json::json!({"timestamp":"2026-08-06T21:49:21Z","type":"event_msg","payload":{"type":"task_complete"}});
            fs::write(sessions.join(format!("rollout-2026-08-06T21-49-18-{id}.jsonl")), format!("{metadata}\n{latest}\n{done}\n")).unwrap();
            let f = fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(locks.join(format!("{id}.lock"))).unwrap(); f.try_lock().unwrap(); f
        }).collect();
        let pending = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(locks.join(format!("{c}.lock")))
            .unwrap();
        pending.try_lock().unwrap();
        let tmux = crate::adaptor::tmux::fake_tmux("codex-title-switch", "");
        fs::write(
            root.join(".codex/config.toml"),
            "[tui]\nterminal_title=[\"thread-id\"]\n",
        )
        .unwrap();
        let proc = |name: &str, parent| Proc {
            name: name.into(),
            started_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            cwd: Some("/same".into()),
            ppid: parent,
        };
        let procs = Procs::of([(3001, proc("zsh", None)), (501, proc("codex", Some(3001)))])
            .with_commands([(
                501,
                ["codex", "resume", a]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )]);
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&root)
                .with_procs(procs.clone())
                .with_tmux(Tmux::at_binary(&tmux)),
        ));
        let aid = crate::adaptor::id_for(a);
        let bid = crate::adaptor::id_for(b);
        k.register(&aid, "tui", "old", Some(501), Some("codex"))
            .unwrap();
        // A guessed screen state must yield to the rollout's explicit turn state.
        k.write_state(&k.dir_of(&aid).unwrap(), State::Idle, None, Source::Screen)
            .unwrap();
        let select = |id: &str| {
            let title = format!("{}...", &id[..29]);
            let output = format!(
                "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fcodex\x1f/same\x1f%1\x1fwork\x1f1\x1f1\x1f{title}\n"
            );
            fs::write(&*tmux, format!("#!/bin/sh\ncat <<'END'\n{output}END\n")).unwrap();
        };
        select(a);
        assert_eq!(k.sighting_of(&aid).unwrap().1, vec![501]);
        assert!(k.held_in_mux(&aid));
        assert_eq!(
            k.rows(|_| 0)
                .iter()
                .find(|r| r.id == aid)
                .unwrap()
                .last
                .as_deref(),
            Some("LATEST_A")
        );
        // Model/status-line edits rewrite the config after this client started.
        // Both this observer and a fresh app must retain the selected identity,
        // its actual state and preview, including when other writers remain live.
        let config = root.join(".codex/config.toml");
        fs::write(&config, "model='changed'\n[tui]\nterminal_title=['thread-id']\nstatus_line=['model-with-reasoning']\n").unwrap();
        let changed_at = procs.get(501).unwrap().started_ms + 10_000;
        fs::File::options()
            .write(true)
            .open(&config)
            .unwrap()
            .set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_millis(changed_at as u64),
            )
            .unwrap();
        let reopened = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&root)
                .with_procs(procs.clone())
                .with_tmux(Tmux::at_binary(&tmux)),
        ));
        for app in [&k, &reopened] {
            let rows = app.rows(|_| 0);
            let current = rows.iter().find(|r| r.id == aid).unwrap();
            assert_eq!(current.category.as_deref(), Some("codex"));
            assert!(current.state_known());
            assert_eq!(current.state.unwrap().state, State::Done);
            assert_eq!(current.last.as_deref(), Some("LATEST_A"));
        }
        select(b);
        assert!(k.sighting_of(&aid).unwrap().1.is_empty());
        assert!(!k.held_in_mux(&aid));
        assert_eq!(k.sighting_of(&bid).unwrap().1, vec![501]);
        assert!(k.held_in_mux(&bid));
        assert!(
            k.attach_multiplexed(&aid).is_err(),
            "old loaded thread cannot reopen the current thread's terminal"
        );
        assert!(
            !k.holds_terminal_form(&aid),
            "a different bound agent is not a shell alias"
        );
        let rows = k.rows(|_| 0);
        let current = rows.iter().find(|r| r.id == bid).unwrap();
        assert_eq!(current.category.as_deref(), Some("codex"));
        assert_eq!(current.last.as_deref(), Some("LATEST_B"));
        select(c);
        let cid = crate::adaptor::id_for(c);
        let rows = k.rows(|_| 0);
        let current = rows.iter().find(|r| r.id == cid).unwrap();
        assert_eq!(current.category.as_deref(), Some("codex"));
        assert_eq!(current.state.unwrap().state, State::Idle);
        assert!(
            current.last.is_none(),
            "a selected new thread has no invented preview"
        );
        assert!(k.held_in_mux(&cid));
        assert!(k.sighting_of(&bid).unwrap().1.is_empty());
        select(a);
        assert_eq!(k.sighting_of(&aid).unwrap().1, vec![501]);
        assert!(k.held_in_mux(&aid));
        assert_eq!(
            k.rows(|_| 0)
                .iter()
                .find(|r| r.id == aid)
                .unwrap()
                .last
                .as_deref(),
            Some("LATEST_A")
        );
        // Disk settings govern future clients. This exact client already
        // published a verified title from its own runtime settings.
        fs::remove_file(root.join(".codex/config.toml")).unwrap();
        for app in [&k, &reopened] {
            let rows = app.rows(|_| 0);
            let current = rows.iter().find(|r| r.id == aid).unwrap();
            assert_eq!(current.category.as_deref(), Some("codex"));
            assert_eq!(current.last.as_deref(), Some("LATEST_A"));
        }
        select(b);
        assert_eq!(reopened.sighting_of(&bid).unwrap().1, vec![501]);
        assert!(reopened.sighting_of(&aid).unwrap().1.is_empty());
        let row = reopened
            .rows(|_| 0)
            .into_iter()
            .find(|r| r.id == bid)
            .unwrap();
        assert_eq!(row.last.as_deref(), Some("LATEST_B"));
        // A replacement process cannot inherit the old client's title contract,
        // even when the OS reuses its PID and the terminal retains its UUID.
        select(a);
        let mut replacement = proc("codex", Some(3001));
        replacement.started_ms = procs.get(501).unwrap().started_ms + 30_000;
        let defaults = Procs::of([(3001, proc("zsh", None)), (501, replacement)])
            .with_commands([(501, vec!["codex".into()])]);
        let limited = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&root)
                .with_procs(defaults)
                .with_tmux(Tmux::at_binary(&tmux)),
        ));
        let rows = limited.rows(|_| 0);
        let terminal = rows
            .iter()
            .find(|r| r.term && r.mux && r.id != aid && r.id != bid && r.id != cid)
            .expect("unbound default client must retain an attachable terminal row");
        assert_eq!(terminal.kind.0, khor_core::kind::SHELL);
        assert_eq!(terminal.category.as_deref(), Some("shell"));
        assert!(terminal.state_known());
        assert_eq!(terminal.state.unwrap().state, State::Busy);
        assert_eq!(
            terminal.doing.as_ref().unwrap().command.as_deref(),
            Some("codex")
        );
        assert_eq!(terminal.current_cwd(), Some("/same"));
        assert!(terminal.last.is_none());
        assert_eq!(terminal.unread, 0);
        assert!(limited.sighting_of(&aid).unwrap().1.is_empty());
        assert!(
            limited.holds_terminal_form(&aid),
            "an imported agent ID in shell form still reaches its original tmux seat"
        );
        drop(pending);
        drop(writers);
    }

    #[test]
    fn workspace_directory_follows_active_tmux_pane_even_with_old_marks() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};
        let root = tmp("workspace-active-pane");
        let tmux = crate::adaptor::tmux::fake_tmux("workspace-active-pane", "");
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::empty()
                .with_procs(Procs::of([
                    proc_under(3001, "zsh", 1),
                    proc_under(3002, "zsh", 1),
                ]))
                .with_tmux(Tmux::at_binary(&tmux)),
        ));
        let id = sid(T_ID);
        k.register(&id, "shell", "work", None, None).unwrap();
        let dir = k.dir_of(&id).unwrap();
        crate::commands::write_saved(
            &dir,
            &crate::commands::Saved {
                doing: khor_core::Doing {
                    cwd: Some("/birth".into()),
                    ..Default::default()
                },
                pane: Some("%1".into()),
                ..Default::default()
            },
        );
        let read = |first: &str, second: &str, active: usize| {
            let output = format!(
                "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f{first}\x1f%1\x1fwork\x1f{}\x1f1\n$7\x1f1786245338\x1f1786245400\x1f3002\x1f0\x1fzsh\x1f{second}\x1f%2\x1fwork\x1f{}\x1f1\n",
                usize::from(active == 1),
                usize::from(active == 2)
            );
            fs::write(&*tmux, format!("#!/bin/sh\ncat <<'END'\n{output}END\n")).unwrap();
            k.rows_listed(|_| 0, |s| s == T_ID)
                .into_iter()
                .find(|r| r.id == id)
                .unwrap()
                .doing
                .unwrap()
                .cwd
        };
        assert_eq!(
            read("/after-cd", "/second", 1).as_deref(),
            Some("/after-cd")
        );
        assert_eq!(read("/after-cd", "/second", 2).as_deref(), Some("/second"));
        assert_eq!(
            read("/returned-and-cd", "/second", 1).as_deref(),
            Some("/returned-and-cd")
        );
    }

    /// **A tmux row khor is attached to, whose pane in front sent no
    /// marks, reads tmux for that pane** (item 21's follow-up): never an
    /// empty row while the person's shells have no integration. A pane
    /// that marked keeps its own reading.
    #[test]
    fn an_attached_tmux_row_reads_tmux_for_a_front_pane_with_no_marks() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let root = tmp("front-pane");
        let tmux = crate::adaptor::tmux::fake_tmux(
            "front-pane",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%1\x1fwork\n\
             $7\x1f1786245338\x1f1786245400\x1f3002\x1f0\x1fsleep\x1f/w/two\x1f%2\x1fwork\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&fixture_vendors())
                .with_procs(Procs::of([
                    proc_under(3001, "zsh", 1),
                    proc_under(3002, "zsh", 1),
                ]))
                .with_tmux(Tmux::at_binary(&tmux)),
        ));
        let row = sid(T_ID);
        k.register(&row, "shell", "work", None, None).unwrap();
        let dir = k.dir_of(&row).unwrap();
        let ledger_marked = |doing: khor_core::Doing, pane: &str, marked: Option<&[&str]>| {
            crate::commands::write_saved(
                &dir,
                &crate::commands::Saved {
                    doing,
                    recent: Vec::new(),
                    pane: Some(pane.into()),
                    marked_panes: marked.map(|m| m.iter().map(|p| p.to_string()).collect()),
                    ..Default::default()
                },
            )
        };
        let ledger = |doing: khor_core::Doing, pane: &str| ledger_marked(doing, pane, None);
        let doing = || {
            let rows = k.rows_listed(|_| 0, |s| s == T_ID);
            let r = rows.iter().find(|r| r.id == row).expect("the tmux row");
            r.doing.clone().unwrap_or_default()
        };
        ledger(khor_core::Doing::default(), "%2");
        let d = doing();
        assert_eq!(
            (d.command.as_deref(), d.running, d.cwd.as_deref()),
            (Some("sleep"), true, Some("/w/two"))
        );
        ledger(khor_core::Doing::default(), "%1");
        let d = doing();
        assert_eq!(
            (d.command.as_deref(), d.running, d.cwd.as_deref()),
            (None, false, Some("/w/one"))
        );
        let marked = khor_core::Doing {
            command: Some("make".into()),
            exit: Some(0),
            cwd: Some("/w/m".into()),
            ..Default::default()
        };
        ledger(marked.clone(), "%2");
        let d = doing();
        assert_eq!(
            (d.command.as_deref(), d.cwd.as_deref()),
            (Some("make"), Some("/w/m"))
        );
        assert!(!d.pane_unmarked, "a host older than the list: unknown");

        // Item 65: the pane in front marks nothing while another does:
        // the row says so, as a khor shell's tmux client does.
        ledger_marked(khor_core::Doing::default(), "%1", Some(&["%2"]));
        let d = doing();
        assert!(d.pane_unmarked, "%1 never marked");
        assert_eq!(
            (d.command.as_deref(), d.cwd.as_deref()),
            (None, Some("/w/one")),
            "tmux's reading still"
        );
        ledger_marked(khor_core::Doing::default(), "%1", Some(&[]));
        assert!(doing().pane_unmarked, "no pane marked yet");
        ledger_marked(marked, "%2", Some(&["%2"]));
        assert!(!doing().pane_unmarked, "the pane in front marks");
    }

    #[test]
    fn an_agent_leaving_its_tmux_session_turns_its_row_into_that_terminal() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let root = tmp("seat-leave");
        let tmux = crate::adaptor::tmux::fake_tmux(
            "seat-leave",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%8\x1fwork\n",
        );
        let at = |procs: Procs, tmux: &str| {
            LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
                Discovery::at(&fixture_vendors())
                    .with_procs(procs)
                    .with_tmux(Tmux::at_binary(tmux)),
            ))
        };
        let pane = proc_under(3001, "zsh", 1);
        let claude = proc_under(4001, "claude", 3001);
        let agent = sid(AGENT);
        let on_list = |s: &str| s == AGENT;

        // claude in pane 3001; its hook registered the row, and the
        // person put it on their list (imported it).
        let k = at(Procs::of([pane.clone(), claude.clone()]), &tmux);
        k.register(&agent, "tui", "one", None, Some("claude"))
            .unwrap();
        let rows = k.rows_listed(|_| 0, on_list);
        assert_eq!(
            rows.len(),
            1,
            "a tmux session holding one claude is one row"
        );
        assert_eq!(rows[0].id, agent);
        assert_eq!(rows[0].kind.0, khor_core::kind::TUI);
        assert_eq!(rows[0].category.as_deref(), Some("claude"));
        assert!(rows[0].mux, "and it sits in the person's tmux");
        assert_eq!(
            read_seat(&k.dir_of(&agent).unwrap()).as_deref(),
            Some(T_ID),
            "the row remembers which host it was seen in"
        );

        // claude exits: gone from the process table, and its SessionEnd
        // hook recorded the ending on its own row.
        let k = at(Procs::of([pane.clone()]), &tmux);
        k.record_exit(&agent, 0).unwrap();
        let rows = k.rows_listed(|_| 0, on_list);
        assert_eq!(
            rows.len(),
            1,
            "the tmux session does not come back as a second row"
        );
        let row = &rows[0];
        assert_eq!(
            row.id, agent,
            "the same row — its tab and its pin are on it"
        );
        assert_eq!(row.kind.0, khor_core::kind::SHELL, "now a terminal");
        assert_eq!(row.category.as_deref(), Some(khor_core::category::SHELL));
        assert!(!row.dead, "nothing ended: the tmux session is still there");
        assert_eq!(
            row.state.unwrap().state,
            State::Idle,
            "the terminal's own word — a shell at its prompt — not the agent's ending"
        );
        assert!(row.term && row.mux, "and its terminal is the tmux session");
        assert_eq!(row.unread, 0);

        // Nobody put the claude on the list: the session as found holds
        // its own seat and the finished claude folds into it.
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.0, T_ID);
        assert_eq!(
            rows[0].category.as_deref(),
            Some(khor_core::category::SHELL)
        );

        // The control: the tmux session is gone as well. Now the host is
        // gone, and now — only now — the row has ended.
        let none = crate::adaptor::tmux::fake_tmux("seat-leave-none", "");
        let k = at(Procs::default(), &none);
        let rows = k.rows_listed(|_| 0, on_list);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, agent);
        assert!(rows[0].dead, "its host is gone: the row has ended");
        assert_eq!(
            rows[0].category.as_deref(),
            Some("claude"),
            "and it ends as what it last was"
        );
    }

    /// **tmux, an agent khor never registered leaves** (the 2026-09-30
    /// account, docs/handoff/001.md). A claude with no hook that nobody
    /// opened in khor has no registry entry, so it had nowhere to write
    /// the seat note above: once it left, the row the person had
    /// imported went with it, and the tmux session came back as a row
    /// nobody put on the list — from the sidebar the row was simply
    /// gone. It has to turn into that tmux session's terminal exactly
    /// like a registered one, whether claude crashed (its status file
    /// left behind, still news) or exited cleanly (the file gone too).
    #[test]
    fn an_unregistered_agent_leaving_its_tmux_session_turns_its_row_into_that_terminal() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let root = tmp("seat-unregistered");
        // A status file of our own, fresh, so the crash below is still
        // news to the sweep (the fixture's is from 2023).
        let vendors = root.join("vendors");
        let sessions = vendors.join(".claude/sessions");
        fs::create_dir_all(&sessions).unwrap();
        let status = sessions.join("4001.json");
        let write_status = || {
            fs::write(
                &status,
                serde_json::to_vec(&serde_json::json!({
                    "pid": 4001,
                    "sessionId": "11111111-1111-4111-8111-111111111111",
                    "cwd": "/w/one",
                    "startedAt": 1_700_000_000_000i64,
                    "name": "one-aa",
                    "status": "busy",
                    "statusUpdatedAt": now_ms() - 1_000,
                }))
                .unwrap(),
            )
            .unwrap()
        };
        write_status();
        let tmux = crate::adaptor::tmux::fake_tmux(
            "seat-unregistered",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%9\x1fwork\n",
        );
        let at = |procs: Procs, tmux: &str| {
            LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
                Discovery::at(&vendors)
                    .with_procs(procs)
                    .with_tmux(Tmux::at_binary(tmux)),
            ))
        };
        let pane = proc_under(3001, "zsh", 1);
        let claude = proc_under(4001, "claude", 3001);
        let agent = sid(AGENT);
        let on_list = |s: &str| s == AGENT;
        let terminal_of = |rows: &[Session], how: &str| {
            let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
            assert_eq!(
                ids,
                vec![AGENT],
                "{how}: the row the person imported is still there, and the tmux session is not a second row"
            );
            let row = &rows[0];
            assert_eq!(row.kind.0, khor_core::kind::SHELL, "{how}: now a terminal");
            assert_eq!(
                row.category.as_deref(),
                Some(khor_core::category::SHELL),
                "{how}: filed as a terminal"
            );
            assert!(
                !row.dead,
                "{how}: nothing ended — the tmux session is still there"
            );
            assert_eq!(
                row.state.unwrap().state,
                State::Idle,
                "{how}: the terminal's own word, not the agent's ending"
            );
            assert!(
                row.term && row.mux,
                "{how}: its terminal is the tmux session"
            );
            assert_eq!(row.title, "one-aa", "{how}: the name it had");
        };

        // claude in pane 3001, found by the sweep alone: no hook ever
        // registered it. The person imported it from the candidates.
        let k = at(Procs::of([pane.clone(), claude.clone()]), &tmux);
        let rows = k.rows_listed(|_| 0, on_list);
        assert!(!k.claims(&agent), "nothing registered it");
        assert_eq!(
            rows.len(),
            1,
            "a tmux session holding one claude is one row"
        );
        assert_eq!(rows[0].id, agent);
        assert_eq!(rows[0].kind.0, khor_core::kind::TUI);
        assert!(rows[0].mux);

        // claude crashes: gone from the process table, its status file
        // left behind.
        let k = at(Procs::of([pane.clone()]), &tmux);
        terminal_of(&k.rows_listed(|_| 0, on_list), "crashed");

        // …or exits cleanly, and the status file goes with it.
        fs::remove_file(&status).unwrap();
        let k = at(Procs::of([pane.clone()]), &tmux);
        terminal_of(&k.rows_listed(|_| 0, on_list), "exited");
        // Asked again: the same answer, not a one-off.
        terminal_of(&k.rows_listed(|_| 0, on_list), "asked again");

        // Nobody put the claude on the list: the session as found holds
        // its own seat, as it always did.
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.0, T_ID);

        // The control: the tmux session is gone as well. Nothing is left
        // to wear, so the row is not dressed as a terminal of nothing —
        // with the crash still news, it is the 失败 the sweep reads.
        write_status();
        let none = crate::adaptor::tmux::fake_tmux("seat-unregistered-none", "");
        let k = at(Procs::default(), &none);
        let rows = k.rows_listed(|_| 0, on_list);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, agent);
        assert_eq!(
            rows[0].state.unwrap().state,
            State::Failed,
            "its host is gone: the ending stands"
        );
        assert_eq!(rows[0].category.as_deref(), Some("claude"));
    }

    /// Two claudes in one tmux session are two conversations, and stay
    /// two rows whoever holds the session's seat: the one on the list
    /// holds it in its own form, and the other — newer, and so the one a
    /// non-agent holder would wear — is not folded into it.
    #[test]
    fn two_agents_in_one_tmux_session_stay_two_rows() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let root = tmp("seat-two");
        let tmux = crate::adaptor::tmux::fake_tmux(
            "seat-two",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%10\x1fwork\n",
        );
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&fixture_vendors())
                .with_procs(Procs::of([
                    proc_under(3001, "zsh", 1),
                    proc_under(4001, "claude", 3001),
                    (
                        4002,
                        crate::adaptor::Proc {
                            name: "claude".into(),
                            started_ms: 1_700_000_100_000,
                            cwd: None,
                            ppid: Some(3001),
                        },
                    ),
                ]))
                .with_tmux(Tmux::at_binary(&tmux)),
        ));
        let mut rows = k.rows_listed(|_| 0, |s| s == AGENT);
        rows.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
        assert_eq!(
            ids,
            vec![AGENT, "tui/22222222-2222-4222-8222"],
            "both conversations list, and the tmux session is not a third row"
        );
        assert!(
            rows.iter()
                .all(|r| r.category.as_deref() == Some("claude") && r.mux)
        );
    }

    /// **tmux, an agent arrives.** A tmux session the person has open in
    /// khor (a bridge whose host runs) becomes the claude started in it:
    /// the same row wears claude's kind, category and word, the
    /// discovered claude row folds into it, and the terminal stays up —
    /// the reaper that used to close it finds nothing. When claude
    /// leaves, the row is a terminal again.
    #[test]
    fn a_terminal_that_starts_an_agent_becomes_that_agent_and_stays_open() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let root = tmp("seat-arrive");
        let tmux = crate::adaptor::tmux::fake_tmux(
            "seat-arrive",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%11\x1fwork\n",
        );
        let at = |procs: Procs| {
            LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
                Discovery::at(&fixture_vendors())
                    .with_procs(procs)
                    .with_tmux(Tmux::at_binary(&tmux)),
            ))
        };
        let pane = proc_under(3001, "zsh", 1);
        let claude = proc_under(4001, "claude", 3001);
        let bridge = sid(T_ID);

        // The bridge `attach_multiplexed` stands up for a tmux row: a
        // registry row under the session's own id, its host running.
        let mut host = FakeHost::up();
        let k = at(Procs::of([pane.clone()]));
        k.register(
            &bridge,
            "shell",
            "work",
            Some(host.pid()),
            Some(khor_core::category::SHELL),
        )
        .unwrap();
        host.serve(&k.dir_of(&bridge).unwrap(), dead_pid());
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, bridge);
        assert_eq!(
            rows[0].category.as_deref(),
            Some(khor_core::category::SHELL)
        );

        // claude starts in the pane.
        let k = at(Procs::of([pane.clone(), claude.clone()]));
        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.len(),
            1,
            "one row: the discovered claude folds into the open terminal"
        );
        let row = &rows[0];
        assert_eq!(
            row.id, bridge,
            "the same row — the tab and the connection are on it"
        );
        assert_eq!(row.kind.0, khor_core::kind::TUI, "now an agent");
        assert_eq!(row.category.as_deref(), Some("claude"));
        assert_eq!(
            row.state.unwrap().state,
            State::Blocked,
            "wearing claude's own word"
        );
        assert!(row.term && row.mux);
        assert_eq!(
            k.worn_by(&bridge, |_| false).map(|s| s.0),
            Some(AGENT.to_owned()),
            "a press on it acts on that claude"
        );
        k.reap_folded_bridges();
        assert!(
            host.still_up(),
            "the terminal is not closed under the person"
        );
        assert!(k.claims(&bridge), "and its row is not forgotten");

        // claude leaves: a terminal again, still the same row.
        let k = at(Procs::of([pane.clone()]));
        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, bridge);
        assert_eq!(rows[0].kind.0, khor_core::kind::SHELL);
        assert_eq!(
            rows[0].category.as_deref(),
            Some(khor_core::category::SHELL)
        );
        assert!(!rows[0].dead);
    }

    /// **A khor shell with `tmux` typed in it** (SESS-BRIEF 1b): the
    /// client descends from the shell, the claude in the session's pane
    /// from the tmux server. The shell wears that claude, as it would
    /// a claude started in it directly; its 「正在运行 tmux」 gives way to
    /// the agent's preview; and neither the claude nor the session is a
    /// second row.
    #[test]
    fn a_khor_shell_attached_to_tmux_wears_the_agent_in_that_session() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let root = tmp("seat-shell-tmux");
        let tmux = crate::adaptor::tmux::fake_tmux(
            "seat-shell-tmux",
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%11\x1fwork\n",
        );
        let host = FakeHost::up();
        let shell = proc_under(5001, "zsh", host.pid());
        let client = proc_under(5002, "tmux", 5001);
        let pane = proc_under(3001, "zsh", 1);
        let claude = proc_under(4001, "claude", 3001);
        let term = sid("shell/a1b2c3d4e5f6");
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
            Discovery::at(&fixture_vendors())
                .with_procs(Procs::of([shell, client, pane, claude]))
                .with_tmux(Tmux::at_binary(&tmux))
                .with_attached(vec![(5001, sid(T_ID))]),
        ));
        k.register(
            &term,
            "shell",
            "zsh",
            Some(host.pid()),
            Some(khor_core::category::SHELL),
        )
        .unwrap();
        let dir = k.dir_of(&term).unwrap();
        host.serve(&dir, 5001);
        crate::commands::write_saved(
            &dir,
            &crate::commands::Saved {
                doing: khor_core::Doing {
                    command: Some("tmux".into()),
                    running: true,
                    started_ms: 1,
                    cwd: Some("/w/one".into()),
                    ..Default::default()
                },
                recent: Vec::new(),
                pane: None,
                marked_panes: None,
                ..Default::default()
            },
        );

        let rows = k.rows(|_| 0);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
        assert_eq!(ids, vec![term.0.as_str()], "one row: the shell");
        let row = &rows[0];
        assert_eq!(row.kind.0, khor_core::kind::TUI, "wearing the agent");
        assert_eq!(row.category.as_deref(), Some("claude"));
        let doing = row.doing.as_ref().unwrap();
        assert_eq!(doing.command, None, "not 「正在运行 tmux」");
        assert_eq!(
            doing.cwd.as_deref(),
            Some("/w/one"),
            "and still where it stands"
        );
        assert_eq!(
            k.worn_by(&term, |_| false).map(|s| s.0),
            Some(AGENT.to_owned()),
            "a press on it acts on that claude"
        );
    }

    /// **A khor shell with `tmux` typed in it and no agent in the
    /// session** (SESS-BRIEF 2c) says what the pane in front of the
    /// client runs and where it stands, not 「正在运行 tmux」: the
    /// shell's own marks only ever saw it start the client. Where tmux
    /// named no front pane, the session's reading stands in, as for an
    /// attached tmux row. The session is no second row either way.
    #[test]
    fn a_khor_shell_attached_to_tmux_says_what_the_front_pane_runs() {
        use crate::adaptor::{Discovery, Procs, tmux::Tmux};

        let panes = concat!(
            "$7\x1f1786245338\x1f1786245400\x1f3001\x1f0\x1fzsh\x1f/w/one\x1f%11\x1fwork\n",
            "$7\x1f1786245338\x1f1786245400\x1f3002\x1f0\x1fvim\x1f/w/two\x1f%12\x1fwork\n",
        );
        let row_reading = |tag: &str,
                           front: Option<&str>,
                           marked: Option<&[&str]>,
                           own: Option<(&str, khor_core::Doing)>| {
            let root = tmp(&format!("seat-shell-tmux-front-{tag}"));
            let tmux = crate::adaptor::tmux::fake_tmux(&format!("front-{tag}"), panes);
            let host = FakeHost::up();
            let procs = Procs::of([
                proc_under(5001, "zsh", host.pid()),
                proc_under(5002, "tmux", 5001),
                proc_under(3001, "zsh", 1),
                proc_under(3002, "zsh", 1),
            ]);
            let discovery = Discovery::empty()
                .with_procs(procs)
                .with_tmux(Tmux::at_binary(&tmux));
            let discovery = match front {
                Some(p) => discovery.with_attached_in(5001, sid(T_ID), p),
                None => discovery.with_attached(vec![(5001, sid(T_ID))]),
            };
            let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32]))
                .discovering(Arc::new(discovery));
            let term = sid("shell/a1b2c3d4e5f6");
            k.register(
                &term,
                "shell",
                "zsh",
                Some(host.pid()),
                Some(khor_core::category::SHELL),
            )
            .unwrap();
            let dir = k.dir_of(&term).unwrap();
            host.serve(&dir, 5001);
            crate::commands::write_saved(
                &dir,
                &crate::commands::Saved {
                    doing: khor_core::Doing {
                        command: Some("tmux".into()),
                        running: true,
                        started_ms: 1,
                        cwd: Some("/w/start".into()),
                        ..Default::default()
                    },
                    recent: Vec::new(),
                    pane: None,
                    marked_panes: marked.map(|m| m.iter().map(|p| p.to_string()).collect()),
                    panes: own.into_iter().map(|(p, d)| (p.to_owned(), d)).collect(),
                },
            );
            fs::write(dir.join("last.txt"), "work | 0:zsh | 12:00").unwrap();
            let rows = k.rows(|_| 0);
            let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
            assert_eq!(ids, vec![term.0.as_str()], "{tag}: one row, the shell");
            assert_eq!(rows[0].kind.0, khor_core::kind::SHELL, "{tag}");
            assert_eq!(rows[0].last, None, "{tag}: not the client's status bar");
            rows[0].doing.clone().unwrap()
        };
        let row_with = |tag: &str, front: Option<&str>, marked: Option<&[&str]>| {
            row_reading(tag, front, marked, None)
        };

        let doing = row_with("busy", Some("%12"), None);
        assert_eq!(
            doing.command.as_deref(),
            Some("vim"),
            "the front pane's program, not tmux"
        );
        assert!(doing.running);
        assert_eq!(doing.cwd.as_deref(), Some("/w/two"));
        assert!(
            !doing.pane_unmarked,
            "a host older than the list: unknown, not unmarked"
        );

        let doing = row_with("idle", Some("%11"), None);
        assert_eq!(
            doing.command, None,
            "a front pane at its prompt runs nothing"
        );
        assert!(!doing.running);
        assert_eq!(doing.cwd.as_deref(), Some("/w/one"));

        let doing = row_with("unnamed", None, Some(&[]));
        assert_eq!(
            doing.command.as_deref(),
            Some("vim"),
            "the session's reading"
        );
        assert!(!doing.pane_unmarked, "no pane named: unknown");

        // SESS-BRIEF 3: which panes mark their commands reaches the row.
        assert!(
            row_with("unmarked", Some("%12"), Some(&["%11"])).pane_unmarked,
            "vim's pane never marked"
        );
        assert!(row_with("none-yet", Some("%12"), Some(&[])).pane_unmarked);
        assert!(
            !row_with("marked", Some("%11"), Some(&["%11"])).pane_unmarked,
            "the front pane marks"
        );

        // Item 64: a front pane that marks says its own reading, the
        // command as typed and how it ended, not tmux's.
        let ran = khor_core::Doing {
            command: Some("make -j8 test".into()),
            started_ms: 4,
            ended_ms: Some(9),
            exit: Some(2),
            cwd: Some("/w/three".into()),
            ..Default::default()
        };
        let doing = row_reading(
            "own",
            Some("%11"),
            Some(&["%11"]),
            Some(("%11", ran.clone())),
        );
        assert_eq!(doing.command.as_deref(), Some("make -j8 test"));
        assert!(!doing.running);
        assert_eq!(
            (doing.ended_ms, doing.exit),
            (Some(9), Some(2)),
            "how it ended"
        );
        assert_eq!(
            doing.cwd.as_deref(),
            Some("/w/one"),
            "live pane cwd outranks an earlier command mark"
        );
        assert!(!doing.pane_unmarked);

        let editing = khor_core::Doing {
            command: Some("vim notes.md".into()),
            running: true,
            started_ms: 7,
            cwd: Some("/w/two".into()),
            ..Default::default()
        };
        let doing = row_reading(
            "own-running",
            Some("%12"),
            Some(&["%11", "%12"]),
            Some(("%12", editing.clone())),
        );
        assert_eq!(
            doing.command.as_deref(),
            Some("vim notes.md"),
            "as typed, not tmux's `vim`"
        );
        assert!(doing.running);
        assert_eq!(doing.started_ms, 7);

        // The ledger says a command runs while tmux sees the pane at its
        // prompt: the end was lost out of sight, so tmux's reading.
        let doing = row_reading(
            "lost-end",
            Some("%11"),
            Some(&["%11"]),
            Some(("%11", editing)),
        );
        assert_eq!(doing.command, None, "tmux sees zsh at its prompt");
        assert!(!doing.running);
        assert_eq!(doing.cwd.as_deref(), Some("/w/one"));

        // Another pane's reading is not the front pane's.
        let doing = row_reading(
            "other",
            Some("%12"),
            Some(&["%11", "%12"]),
            Some(("%11", ran)),
        );
        assert_eq!(
            doing.command.as_deref(),
            Some("vim"),
            "tmux's reading of %12"
        );
    }

    /// **khor's own terminal.** A shell khor hosts wears the claude
    /// started inside it — claude's hooks write claude's own row, which
    /// folds into the terminal — and returns to being a shell when
    /// claude leaves, with nothing ended. Only the shell exiting ends
    /// it; then the claude row, whose host is gone, shows as ended too.
    #[test]
    fn a_khor_terminal_wears_the_agent_inside_it_and_ends_only_with_its_shell() {
        use crate::adaptor::{Discovery, Procs};

        let root = tmp("seat-pty");
        let at = |procs: Procs| {
            LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(Arc::new(
                Discovery::at(&fixture_vendors()).with_procs(procs),
            ))
        };
        let host = FakeHost::up();
        let shell = proc_under(5001, "zsh", host.pid());
        let claude = proc_under(4001, "claude", 5001);
        let term = sid("shell/a1b2c3d4e5f6");
        let agent = sid(AGENT);
        let payload = |event: &str| {
            format!(
                r#"{{"session_id":"11111111-1111-4111-8111-111111111111","cwd":"/w/one","hook_event_name":"{event}"}}"#
            )
        };

        let k = at(Procs::of([shell.clone()]));
        k.register(
            &term,
            "shell",
            "zsh",
            Some(host.pid()),
            Some(khor_core::category::SHELL),
        )
        .unwrap();
        host.serve(&k.dir_of(&term).unwrap(), 5001);
        assert_eq!(k.rows(|_| 0)[0].kind.0, khor_core::kind::SHELL);

        // claude starts in it. Its hooks carry the terminal's
        // KHOR_SESSION, and must write claude's own row.
        let k = at(Procs::of([shell.clone(), claude.clone()]));
        crate::adaptor::claude::hook_under(&k, &payload("SessionStart"), Some(term.0.clone()))
            .unwrap();
        crate::adaptor::claude::hook_under(&k, &payload("Stop"), Some(term.0.clone())).unwrap();
        assert!(k.claims(&agent), "the hook wrote claude's own row");
        assert_eq!(
            k.on_record(&term).map(|(kind, _)| kind).as_deref(),
            Some(khor_core::kind::SHELL),
            "and left the terminal's alone"
        );
        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.len(),
            1,
            "claude's row folds into the terminal it runs in"
        );
        let row = &rows[0];
        assert_eq!(row.id, term);
        assert_eq!(row.kind.0, khor_core::kind::TUI);
        assert_eq!(row.category.as_deref(), Some("claude"));
        assert_eq!(
            (row.state.unwrap().state, row.unread),
            (State::Done, 1),
            "claude's 完成, unread against the terminal's own watermark"
        );
        assert!(!row.mux, "khor's own terminal, not a tmux seat");
        let seen = |s: &str| if s == term.0 { i64::MAX } else { 0 };
        assert_eq!(
            k.rows(seen)[0].unread,
            0,
            "looking at the terminal clears the badge it shows"
        );
        assert!(
            k.shown_stamp(&term, |_| false).unwrap() >= k.stamp(&agent).unwrap(),
            "and 「seen」 on it covers claude's 完成"
        );

        // claude exits: SessionEnd is claude's ending, not the shell's.
        crate::adaptor::claude::hook_under(&k, &payload("SessionEnd"), Some(term.0.clone()))
            .unwrap();
        let k = at(Procs::of([shell.clone()]));
        let rows = k.rows(|_| 0);
        assert_eq!(
            rows.len(),
            1,
            "the finished claude is not a second, ended row"
        );
        assert_eq!(rows[0].id, term);
        assert_eq!(rows[0].kind.0, khor_core::kind::SHELL, "a terminal again");
        assert!(!rows[0].dead, "the shell is still there: nothing ended");

        // The control: the shell itself exits — the pty's child is gone.
        k.record_exit(&term, 0).unwrap();
        let mut rows = k.rows(|_| 0);
        rows.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, term);
        assert!(rows[0].dead, "the terminal's host ended: now it is 已结束");
        assert_eq!(rows[1].id, agent);
        assert!(
            rows[1].dead,
            "and the claude row, whose host is gone, ends with it"
        );
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
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(discovery);
        k.register(&id, "tui", "hooked", None, None).unwrap();
        k.report(&id, State::Busy, Source::Reported).unwrap();

        let rows = k.rows(|_| 0);
        assert_eq!(rows.len(), 1, "still one row");
        assert_eq!(
            rows[0].state.unwrap().state,
            State::Failed,
            "the registry had no pid; the vendor file did, and it says the process is gone"
        );
        assert_eq!(rows[0].unread, 0, "失败 sinks and never holds the badge");

        // The control: with the process alive, the registry's own word
        // is what shows — so the 失败 above came from the ending rule and
        // not from the merge always preferring disk.
        let alive = Arc::new(Discovery::at(&vendors).with_procs(Procs::of([(
            8001,
            crate::adaptor::Proc {
                name: "claude".into(),
                started_ms: at - 1000,
                cwd: None,
                ppid: None,
            },
        )])));
        let k = LiveKind::new(root.to_path_buf(), DeviceId([9; 32])).discovering(alive);
        assert_eq!(k.rows(|_| 0)[0].state.unwrap().state, State::Busy);
    }

    /// **失败 says which** (P5-SHELL item 3): a recorded exit is the exit;
    /// a host gone with nothing recorded is the host, killed; a session
    /// still backed has no ending — and the word is 失败 in both dead
    /// cases, as before.
    #[test]
    fn a_dead_row_says_how_it_ended() {
        let root = tmp("ending");
        let k = kind_at(&root);
        let exited = sid("shell/exit1");
        k.register(&exited, "shell", "x", Some(std::process::id()), None)
            .unwrap();
        k.record_exit(&exited, 1).unwrap();
        let killed = sid("shell/killed1");
        k.register(&killed, "shell", "y", Some(dead_pid()), None)
            .unwrap();
        let alive = sid("shell/alive1");
        k.register(&alive, "shell", "z", Some(std::process::id()), None)
            .unwrap();
        assert_eq!(k.ending(&exited), Some(Ending::Exit(1)));
        assert_eq!(k.ending(&killed), Some(Ending::HostGone));
        assert_eq!(k.ending(&alive), None, "a running session has not ended");
        assert_eq!(k.ending(&sid("shell/nobody")), None);
        let rows = k.rows(|_| 0);
        let word = |id: &SessionId| {
            rows.iter()
                .find(|r| &r.id == id)
                .unwrap()
                .state
                .unwrap()
                .state
        };
        assert_eq!(
            (word(&exited), word(&killed)),
            (State::Failed, State::Failed),
            "the word stays 失败"
        );
    }

    /// **A dead row's page**: the last screen when the host left
    /// `ring.bin`, and the commands either way — a host from before the
    /// ring, or one killed before its first write, leaves only those.
    #[test]
    fn a_dead_rows_page_is_its_screen_or_its_commands() {
        let root = tmp("ended");
        let k = kind_at(&root);
        let id = sid("shell/gone2");
        k.register(&id, "shell", "x", Some(dead_pid()), None)
            .unwrap();
        let dir = k.dir_of(&id).unwrap();
        let ended = k.ended(&id).expect("dead");
        assert_eq!(ended.ending, Ending::HostGone);
        assert!(
            ended.screen.is_none() && ended.recent.is_empty(),
            "nothing left: nothing shown"
        );
        let command = crate::commands::Command {
            n: 1,
            text: Some("make".into()),
            cwd: Some("/w".into()),
            started_ms: 1,
            ended_ms: Some(2),
            exit: Some(2),
            at: 0,
            end_at: Some(10),
            pane: None,
            last_line: None,
        };
        crate::commands::write_saved(
            &dir,
            &crate::commands::Saved {
                recent: vec![command.clone()],
                ..Default::default()
            },
        );
        let ended = k.ended(&id).unwrap();
        assert!(ended.screen.is_none());
        assert_eq!(ended.recent, vec![command]);
        let screen = crate::host::RingFile {
            form: crate::host::RING_FORM_SNAPSHOT,
            cols: 80,
            rows: 24,
            at_ms: 3,
            bytes: b"$ make".to_vec(),
        };
        crate::host::write_ring_file(&dir, &screen).unwrap();
        assert_eq!(k.ended(&id).unwrap().screen, Some(screen));
        let alive = sid("shell/alive2");
        k.register(&alive, "shell", "z", Some(std::process::id()), None)
            .unwrap();
        assert!(
            k.ended(&alive).is_none(),
            "a live shell never reads the file"
        );
    }
}
