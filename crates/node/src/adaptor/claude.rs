//! Claude Code: one vendor, two signals.
//!
//! Claude keeps a live status file per running session at
//! `~/.claude/sessions/<pid>.json` — a flat JSON of some seventeen
//! fields, rewritten whenever the session's status changes. That file is
//! the discovery signal and it is why this batch exists: it needs no
//! configuration, so khor's list is full the first second it is
//! installed, including sessions started before khor existed.
//!
//! The hook ([`hook`]) is the second signal and stays because the file
//! cannot say everything. Together they are the layering the ledger
//! describes: disk discovers and backstops, hooks sharpen.
//!
//! # What disk can and cannot say
//!
//! The file's `status` is `busy` / `waiting` / `idle`, and `waitingFor`
//! distinguishes the one kind of waiting that is 待批 from the kind that
//! must never be (docs/SESSION.md: "等你说下一句" is not 待批, or the
//! badge never reaches zero). That covers four of the six words —
//! 忙碌, 待批, 空闲, and 失败 by the crash rule.
//!
//! **完成 is the word disk cannot give, and it shows as 空闲.** The file
//! writes `idle` both when a turn just ended and when a session has been
//! sitting untouched since it started, and nothing in it separates the
//! two; mapping `idle` to 完成 would put an unclearable badge on every
//! idle session, and that is the exact failure 待批 is defined to avoid.
//! The `Stop` hook is what upgrades a discovered row to 完成, which is
//! the whole reason the hook is still worth installing.
//!
//! 中断 is likewise not on disk: an API error that leaves the process
//! alive does not change `status`.

use std::path::{Path, PathBuf};

use khor_catalog::msg;
use khor_core::{SessionId, State};

use super::{id_for, read_text, Adaptor, Procs, Sighting, Sweep, CRASH_GRACE_MS};

/// The one `waitingFor` reason that is 待批.
///
/// Matched exactly, and everything else counts as unmapped rather than
/// falling through to a word. Claude waits for several things; only a
/// permission prompt is an action stuck behind the user's approval
/// (docs/SESSION.md 六词映射), and the cost of guessing wrong here is
/// the badge that can never reach zero.
const WAITING_FOR_PERMISSION: &str = "permission prompt";

/// Claude's live status file. Unknown fields are ignored on purpose:
/// the vendor adds them between patch versions, and a session that
/// khor can otherwise read must not vanish because a field appeared.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusFile {
    pid: u32,
    session_id: String,
    #[serde(default)]
    cwd: Option<String>,
    /// When the process behind this session started. Measured 2-3s after
    /// the true process start across six live sessions, which is what
    /// `Procs::alive_since` tolerates.
    #[serde(default)]
    started_at: i64,
    /// Claude's own name for the session ("tokens-50"). Better than the
    /// directory name because it is unique per session: this machine has
    /// three claude sessions in one repo, and the directory name would
    /// print the same title three times.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    waiting_for: Option<String>,
    #[serde(default)]
    status_updated_at: Option<i64>,
    #[serde(default)]
    updated_at: Option<i64>,
}

impl StatusFile {
    /// When this session's word last became true.
    fn at_ms(&self) -> i64 {
        self.status_updated_at
            .or(self.updated_at)
            .unwrap_or(self.started_at)
    }

    fn title(&self) -> String {
        if let Some(name) = self.name.as_deref().filter(|n| !n.is_empty()) {
            return name.to_owned();
        }
        self.cwd
            .as_deref()
            .and_then(|c| Path::new(c).file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "claude".into())
    }
}

/// Claude's status vocabulary, mapped. `None` is "this adaptor does not
/// know", which becomes a count and never a row (see the module head of
/// [`super`]).
fn word_of(status: Option<&str>, waiting_for: Option<&str>) -> Option<State> {
    match status? {
        "busy" => Some(State::Busy),
        "idle" => Some(State::Idle),
        "waiting" => match waiting_for {
            Some(WAITING_FOR_PERMISSION) => Some(State::Blocked),
            // Waiting for something this adaptor has not been taught.
            // Not 待批 (that badge must be answerable) and not 空闲
            // (the session is stopped on something). No word.
            _ => None,
        },
        _ => None,
    }
}

pub struct Claude {
    root: PathBuf,
}

impl Claude {
    pub fn at(root: PathBuf) -> Claude {
        Claude { root }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    /// The recorded conversation of one session, read from the
    /// transcript claude itself writes (`projects/<project>/<sid>.jsonl`).
    ///
    /// `leaf` is a khor id's leaf, and [`crate::adaptor::id_for`]'s
    /// cleaning is lossy (24 chars of a 36-char uuid) — so instead of
    /// undoing it, the match runs the same cleaning over the file names.
    /// Two files cleaning alike is a uuid-prefix collision; the newest
    /// wins, and that choice is a judgment, not a guarantee.
    pub fn transcript(&self, leaf: &str) -> Result<Vec<Utterance>, String> {
        let (path, _) = self.transcript_path(leaf).ok_or_else(|| msg::no_such_session(leaf))?;
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        Ok(utterances_of(&text))
    }

    /// The transcript file behind a leaf, with its mtime — the shared
    /// resolution `transcript` and `last_said` both run.
    /// The vendor's **full** session id behind a lossy leaf — the
    /// transcript's own filename. A khor row id truncates the uuid
    /// (`clean_leaf` caps at 24), but `--resume` wants the whole thing;
    /// the file is the one place that never forgot it.
    pub(crate) fn full_session_id(&self, leaf: &str) -> Option<String> {
        let (path, _) = self.transcript_path(leaf)?;
        path.file_stem().map(|s| s.to_string_lossy().to_string())
    }

    /// The cwd the transcript itself records — every entry carries one.
    /// A takeover resumes the conversation somewhere else entirely, and
    /// the agent's world must not move with the resumer (`cagent`'s
    /// load-before-new branch reads this).
    pub(crate) fn recorded_cwd(&self, leaf: &str) -> Option<PathBuf> {
        use std::io::Read;
        let (path, _) = self.transcript_path(leaf)?;
        let mut head = String::new();
        std::fs::File::open(path).ok()?.take(64 * 1024).read_to_string(&mut head).ok()?;
        head.lines().find_map(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .ok()?
                .get("cwd")?
                .as_str()
                .map(PathBuf::from)
        })
    }

    fn transcript_path(&self, leaf: &str) -> Option<(PathBuf, std::time::SystemTime)> {
        let mut hit: Option<(std::time::SystemTime, PathBuf)> = None;
        let projects = std::fs::read_dir(self.projects_dir()).ok()?;
        for proj in projects.flatten() {
            let Ok(rd) = std::fs::read_dir(proj.path()) else { continue };
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
                if crate::live::clean_leaf(stem) != leaf {
                    continue;
                }
                let at = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if hit.as_ref().is_none_or(|(t, _)| at > *t) {
                    hit = Some((at, path));
                }
            }
        }
        hit.map(|(t, p)| (p, t))
    }

    /// The newest utterance in a leaf's transcript, as one preview line —
    /// what the row's 最近一条消息 shows for a claude-backed session.
    ///
    /// A list polls this for every claude row every few seconds, so the
    /// answer is cached by (path, mtime): an unchanged transcript costs
    /// directory stats, not a read. On change only the tail is read — the
    /// last utterance lives at the end, and a long conversation's file
    /// does not get re-parsed whole for one preview line.
    pub fn last_said(&self, leaf: &str) -> Option<String> {
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};
        static CACHE: OnceLock<Mutex<HashMap<PathBuf, (std::time::SystemTime, Option<String>)>>> =
            OnceLock::new();
        let (path, mtime) = self.transcript_path(leaf)?;
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some((t, said)) = cache.lock().unwrap().get(&path) {
            if *t == mtime {
                return said.clone();
            }
        }
        let said = tail_said(&path);
        cache.lock().unwrap().insert(path, (mtime, said.clone()));
        said
    }
}

/// The last utterance out of a transcript's tail. Reads the final 64 KiB
/// and drops the first (possibly cut) line before parsing — a line split
/// mid-JSON must read as absent, not as garbage.
fn tail_said(path: &std::path::Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    const TAIL: u64 = 64 * 1024;
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let start = len.saturating_sub(TAIL);
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut text = String::new();
    f.take(TAIL).read_to_string(&mut text).ok()?;
    let text = if start > 0 {
        match text.split_once('\n') {
            Some((_, rest)) => rest,
            None => return None,
        }
    } else {
        text.as_str()
    };
    utterances_of(text).into_iter().rev().find_map(|u| match u {
        // The marks come off before the line is folded: a said line is
        // markdown, and a preview has nowhere to render it to
        // (`khor_core::plain`). `plain` first — its list and heading
        // marks only exist at the head of a line, and `preview` is what
        // takes the lines away.
        Utterance::User(t) | Utterance::Agent(t) => Some(khor_core::preview(&khor_core::plain(&t))),
        // A thought or a tool name is the machine's bookkeeping — a
        // preview line should say what was *said*, and skipping back to
        // the newest real sentence reads better than "Read".
        Utterance::Thought(_) | Utterance::Tool(_) => None,
    })
}

/// One utterance of a recorded conversation, in speaking order.
#[derive(Debug, PartialEq)]
pub enum Utterance {
    User(String),
    Agent(String),
    Thought(String),
    /// A tool call, by name only: a name is a fact about the
    /// conversation, an input is a payload nobody asked a chat view to
    /// carry.
    Tool(String),
}

/// Transcript JSONL in, utterances out. Only `user` and `assistant`
/// lines speak; the rest of the file (snapshots, modes, attachments,
/// summaries) is the vendor's bookkeeping. Within a user line, a string
/// opening with one of the CLI's own tags is plumbing a person never
/// typed, and a `tool_result` block is the machine answering itself —
/// both stay unpainted.
fn utterances_of(text: &str) -> Vec<Utterance> {
    const PLUMBING: [&str; 4] = [
        "<command-name>",
        "<local-command-stdout>",
        "<local-command-caveat>",
        "<system-reminder>",
    ];
    let user_text = |s: &str| -> Option<Utterance> {
        if s.is_empty() || PLUMBING.iter().any(|tag| s.starts_with(tag)) {
            None
        } else {
            Some(Utterance::User(s.to_owned()))
        }
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let from_user = match v.get("type").and_then(|t| t.as_str()) {
            Some("user") => true,
            Some("assistant") => false,
            _ => continue,
        };
        let Some(content) = v.get("message").and_then(|m| m.get("content")) else { continue };
        match content {
            serde_json::Value::String(s) if from_user => out.extend(user_text(s)),
            serde_json::Value::String(s) => out.push(Utterance::Agent(s.clone())),
            serde_json::Value::Array(blocks) => {
                for b in blocks {
                    let field = |name: &str| b.get(name).and_then(|x| x.as_str()).unwrap_or("");
                    match (b.get("type").and_then(|t| t.as_str()), from_user) {
                        (Some("text"), true) => out.extend(user_text(field("text"))),
                        (Some("text"), false) if !field("text").is_empty() => {
                            out.push(Utterance::Agent(field("text").to_owned()));
                        }
                        (Some("thinking"), false) if !field("thinking").is_empty() => {
                            out.push(Utterance::Thought(field("thinking").to_owned()));
                        }
                        (Some("tool_use"), false) if !field("name").is_empty() => {
                            out.push(Utterance::Tool(field("name").to_owned()));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// This vendor's name, which is also the category its rows carry. Named
/// once: the disk sweep reports it through [`Adaptor::vendor`] and the
/// hook stamps it directly, and those two must be the same string or one
/// session ends up in two groups.
pub const VENDOR: &str = "claude";

impl Adaptor for Claude {
    fn vendor(&self) -> &'static str {
        VENDOR
    }

    fn sweep(&self, procs: &Procs) -> Sweep {
        let now = crate::live::now_ms();
        let mut sweep = Sweep::default();
        let Ok(rd) = std::fs::read_dir(self.sessions_dir()) else {
            return sweep;
        };
        // The directory also holds `<pid>.<hash>.key` files; only the
        // status files are ours to read.
        let mut seen: Vec<(String, i64, Sighting)> = Vec::new();
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Some(text) = read_text(&path) else { continue };
            let Ok(file) = serde_json::from_str::<StatusFile>(&text) else {
                // A status file khor cannot parse at all: the vendor
                // changed the layout. Exactly what the count is for.
                sweep.unmapped += 1;
                continue;
            };
            let at_ms = file.at_ms();
            let word = match procs.alive_since(file.pid, file.started_at) {
                Some(_) => match word_of(file.status.as_deref(), file.waiting_for.as_deref()) {
                    Some(w) => w,
                    None => {
                        sweep.unmapped += 1;
                        continue;
                    }
                },
                // The file outlived its process. Claude deletes it on a
                // clean exit, so a survivor is an ending nobody recorded
                // — the same missing-ending rule live.rs applies to its
                // own registry. Only while it is still news.
                None => {
                    if now.saturating_sub(at_ms) > CRASH_GRACE_MS {
                        continue;
                    }
                    State::Failed
                }
            };
            let title = file.title();
            let pid = file.pid;
            seen.push((
                file.session_id.clone(),
                at_ms,
                Sighting {
                    vendor_session_id: file.session_id,
                    title,
                    word,
                    at_ms,
                    // Only while it is running. A crashed session's file
                    // still names its pid, but that number belongs to
                    // nobody now, and claiming it would let a dead
                    // claude hide the tmux session it used to sit in.
                    pids: if word == State::Failed { vec![] } else { vec![pid] },
                },
            ));
        }
        // One session id can have two live status files: `claude -r`
        // resumes a session into a new process while the old process is
        // still running, and both keep writing under the same
        // `sessionId`. Observed on this machine — two live pids, one id,
        // one of them two days stale. They are one session and one row,
        // and the freshest word is the true one.
        seen.sort_by(|a, b| b.1.cmp(&a.1));
        let mut kept: Vec<(String, Sighting)> = Vec::new();
        for (session_id, _, sighting) in seen {
            match kept.iter_mut().find(|(sid, _)| *sid == session_id) {
                // The row is the fresher file's, but it stands for both
                // processes. Dropping the older pid here would leave a
                // live claude that nothing on the list accounts for, and
                // the tmux session around it would then appear as a
                // shell of its own (`Sighting::pids`).
                Some((_, first)) => first.pids.extend(sighting.pids),
                None => kept.push((session_id, sighting)),
            }
        }
        sweep.rows = kept.into_iter().map(|(_, s)| s).collect();
        sweep
    }
}

// ── the hook (docs/HOOKS.md) ────────────────────────────────

/// What `khor state --hook` did, for the caller to print (or not).
pub enum Hooked {
    Updated(SessionId, State),
    Ended(SessionId),
    /// The event carries no state change (a passing notification, an
    /// event this glue does not map).
    Ignored,
}

/// Reads one Claude Code hook payload and moves the session it belongs
/// to. Claude-version-specific glue: the mapping from hook events to the
/// six words is this function and nothing else.
///
/// The session: `KHOR_SESSION` when the agent runs under `khor run` (one
/// session, reliable pid, real exit code); otherwise the id is minted
/// from claude's own session id through [`id_for`] — **the same call the
/// disk sweep makes**, which is what keeps a session that is both hooked
/// and discovered to one row.
///
/// Word mapping — the one deliberate narrowing is Notification: only a
/// permission ask becomes 待批. Claude also notifies on long idle, and
/// "等你说下一句" is exactly what 待批 must never mean (docs/SESSION.md).
pub fn hook(live: &crate::live::LiveKind, payload: &str) -> Result<Hooked, String> {
    use khor_catalog::msg;

    let v: serde_json::Value =
        serde_json::from_str(payload).map_err(msg::hook_payload_garbled)?;
    let event = v["hook_event_name"].as_str().unwrap_or("");

    let id = match std::env::var("KHOR_SESSION") {
        Ok(sid) if live.claims(&SessionId(sid.clone())) => {
            let id = SessionId(sid);
            // `khor run --tui -- claude` registered this row before any
            // vendor could be named, so it has no category. This hook
            // arriving *is* the moment somebody can name it: the hook is
            // claude's own. Reading the command line instead would be a
            // guess, and wrong for an alias or a wrapper.
            live.learn_category(&id, Some(VENDOR))?;
            // The hook also carries claude's own session id — the name
            // its transcript file wears. Recording it is what lets the
            // conversation view find that file for a khor-opened row,
            // whose khor-minted leaf matches nothing in projects/.
            if let Some(raw) = v["session_id"].as_str() {
                live.learn_vendor_leaf(&id, &crate::live::clean_leaf(raw))?;
            }
            id
        }
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
            let id = id_for(raw);
            live.ensure(&id, khor_core::kind::TUI, &title, None, Some(VENDOR))?;
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
            live.report(&id, w, crate::live::Source::Reported)?;
            Ok(Hooked::Updated(id, w))
        }
        None => Ok(Hooked::Ignored),
    }
}

// ── installing the hook (docs/HOOKS.md) ─────────────────────

/// The events khor asks claude to tell it about, and why exactly these.
///
/// Each maps to a word in [`hook`], and the set is the smallest one that
/// covers the walk: a session appears (`SessionStart`), a turn begins
/// (`UserPromptSubmit`), an approval is asked for (`Notification`), a
/// turn ends (`Stop` — **the only source of 完成 for discovered
/// sessions**; an ACP-opened one hears its turn end first-hand, `gui_host`),
/// the session ends (`SessionEnd`).
///
/// Asking for more would not make the row better and would make claude
/// run khor more often; asking for fewer loses a word. `Stop` is the one
/// that makes installing worth doing at all.
pub const HOOK_EVENTS: [&str; 5] =
    ["SessionStart", "UserPromptSubmit", "Notification", "Stop", "SessionEnd"];

/// The tail every khor hook command ends with, and the marker that says
/// an entry in someone's settings file is khor's.
///
/// Recognising khor's own entry by **the verb it invokes** rather than
/// by the path in front of it is what makes installing idempotent
/// across a khor that moved: the path is the part allowed to change.
const HOOK_VERB: &str = " state --hook";

/// Where claude keeps the settings a hook goes into.
pub fn settings_path(vendor_home: &Path) -> PathBuf {
    vendor_home.join(".claude").join("settings.json")
}

/// What khor is on record as, for one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// Nothing of khor's under this event.
    Missing,
    /// A khor hook, but not this khor — the binary moved or a second
    /// install is on this machine. Carries the recorded command, because
    /// "it is installed" and "it is installed and points somewhere else"
    /// must not read the same.
    Elsewhere(String),
    /// This binary.
    Here,
}

/// What `khor hooks` found.
#[derive(Debug, Clone)]
pub struct HookReport {
    pub path: PathBuf,
    /// Whether the settings file exists at all. A machine where claude
    /// has never run has none, and that is not a problem to report —
    /// installing creates it.
    pub settings_exist: bool,
    pub events: Vec<(&'static str, Installed)>,
}

/// What one install did, per event. Three lists rather than one count:
/// **"installed twice equals installed once" is only visible if the
/// second run can say it changed nothing.**
#[derive(Debug, Default, Clone)]
pub struct HookInstall {
    pub path: PathBuf,
    pub added: Vec<&'static str>,
    /// Khor was already here under another path and now points at this
    /// binary.
    pub repointed: Vec<&'static str>,
    pub unchanged: Vec<&'static str>,
}

/// The command claude should run: this binary, then khor's hook verb.
///
/// **An absolute path, not `khor`.** A hook is run by claude, which may
/// have been started from a GUI with a PATH that has nothing of the
/// user's shell in it — the same trap `super::tmux::CANDIDATES` guards
/// against, arriving from the other direction. The cost is that a khor
/// which later moves leaves a hook pointing at nothing, and that is why
/// [`hooks_report`] says so out loud instead of only saying "installed".
///
/// Single quotes because the path is going into a shell command line and
/// may hold spaces; `'` inside it is escaped the only way POSIX allows.
pub fn hook_command() -> Result<String, String> {
    let exe = crate::self_exe()?;
    Ok(format!("{}{HOOK_VERB}", shell_quoted(&exe.to_string_lossy())))
}

fn shell_quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Whether this settings entry is a khor hook.
fn is_khors(command: &str) -> bool {
    command.ends_with(HOOK_VERB)
}

/// Reads claude's settings and says, per event, what khor is on record
/// as. Never writes.
pub fn hooks_report(vendor_home: &Path) -> Result<HookReport, String> {
    let path = settings_path(vendor_home);
    let ours = hook_command()?;
    let settings = read_settings(&path)?;
    let events = HOOK_EVENTS
        .iter()
        .map(|event| {
            let found = recorded_command(settings.as_ref(), event);
            let state = match found {
                None => Installed::Missing,
                Some(c) if c == ours => Installed::Here,
                Some(c) => Installed::Elsewhere(c),
            };
            (*event, state)
        })
        .collect();
    Ok(HookReport { path, settings_exist: settings.is_some(), events })
}

/// Adds khor's hooks to claude's settings, leaving everything else in
/// that file exactly as it was.
///
/// **Merge, never replace.** The file belongs to the user and usually
/// already holds their own hooks — on this machine, mandala's, under
/// eight events. So this reaches only into `hooks.<event>`, appends one
/// group of its own, and touches no other key, no other event, and no
/// other group under the same event.
///
/// Idempotent by the same reach: an event that already names khor is
/// left alone, or has just its path corrected if this binary is not the
/// one recorded.
///
/// Refuses rather than repairs a file it cannot parse. A settings file
/// with a syntax error is one the user is in the middle of editing, or
/// one khor has misunderstood; overwriting it with khor's idea of what
/// it contained would destroy the only copy.
pub fn install_hooks(vendor_home: &Path) -> Result<HookInstall, String> {
    let path = settings_path(vendor_home);
    let ours = hook_command()?;
    let mut settings = read_settings(&path)?.unwrap_or_else(|| serde_json::json!({}));
    let mut out = HookInstall { path: path.clone(), ..Default::default() };

    let root = settings
        .as_object_mut()
        .ok_or_else(|| msg::settings_not_an_object(path.display()))?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| msg::settings_hooks_not_an_object(path.display()))?;

    for event in HOOK_EVENTS {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .ok_or_else(|| msg::settings_event_not_a_list(path.display(), event))?;
        match khors_command_mut(groups) {
            Some(slot) if *slot == ours => out.unchanged.push(event),
            Some(slot) => {
                *slot = serde_json::Value::String(ours.clone());
                out.repointed.push(event);
            }
            None => {
                groups.push(serde_json::json!({
                    "hooks": [{ "type": "command", "command": ours }]
                }));
                out.added.push(event);
            }
        }
    }

    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    write_keeping_mode(&path, format!("{text}\n").as_bytes())?;
    Ok(out)
}

/// What one uninstall did, per event.
///
/// Two lists for the same reason [`HookInstall`] has three: **"removed
/// twice equals removed once" is only visible if the second run can say
/// it found nothing.** Nobody can check somebody else's settings file by
/// hand, so the run has to say what it did.
#[derive(Debug, Default, Clone)]
pub struct HookUninstall {
    pub path: PathBuf,
    pub removed: Vec<&'static str>,
    /// Events that held no khor hook to begin with.
    pub absent: Vec<&'static str>,
}

/// Takes khor's hooks back out of claude's settings, leaving everything
/// else in that file exactly as it was.
///
/// **The inverse of [`install_hooks`], reach for reach.** It removes the
/// command entries [`is_khors`] recognises and nothing else — which
/// means a hook belonging to *another* khor (a different path, the
/// [`Installed::Elsewhere`] case) comes out too, and that is the point:
/// the alternative is a machine where `khor hooks` reports something
/// installed that nothing can remove.
///
/// **Empty containers are pruned, up to and including the `hooks`
/// object.** An empty group, an event whose list has nothing in it, and
/// an empty `hooks` map all mean exactly what their absence means, so
/// leaving them behind is khor's litter in a file khor does not own. A
/// group that still holds somebody else's entry is left standing, and so
/// is every event khor never touched.
///
/// Refuses rather than repairs a file it cannot parse, for the reason
/// [`install_hooks`] gives: a settings file with a syntax error is one
/// somebody is in the middle of editing.
pub fn uninstall_hooks(vendor_home: &Path) -> Result<HookUninstall, String> {
    let path = settings_path(vendor_home);
    let mut out = HookUninstall { path: path.clone(), ..Default::default() };
    let Some(mut settings) = read_settings(&path)? else {
        // No file at all: nothing of khor's is in it, which is the
        // answer rather than an error. Nothing is written either —
        // creating a settings file in order to remove nothing from it
        // would leave a trace of an operation that did nothing.
        out.absent = HOOK_EVENTS.to_vec();
        return Ok(out);
    };

    let root = settings
        .as_object_mut()
        .ok_or_else(|| msg::settings_not_an_object(path.display()))?;
    // No `hooks` key: same as no file, minus the writing.
    let Some(hooks) = root.get_mut("hooks") else {
        out.absent = HOOK_EVENTS.to_vec();
        return Ok(out);
    };
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| msg::settings_hooks_not_an_object(path.display()))?;

    for event in HOOK_EVENTS {
        let Some(groups) = hooks.get_mut(event) else {
            out.absent.push(event);
            continue;
        };
        let groups = groups
            .as_array_mut()
            .ok_or_else(|| msg::settings_event_not_a_list(path.display(), event))?;
        let mut took = false;
        for g in groups.iter_mut() {
            let Some(entries) = g.get_mut("hooks").and_then(|h| h.as_array_mut()) else {
                continue;
            };
            let before = entries.len();
            entries.retain(|e| !e["command"].as_str().is_some_and(is_khors));
            took |= entries.len() != before;
        }
        if !took {
            out.absent.push(event);
            continue;
        }
        // **Pruned only where khor removed something.** A group khor
        // emptied is a group khor made — install always appends its own
        // holding exactly one entry — and an event array left empty by
        // that is khor's leftover. An empty group or event khor never
        // touched is somebody else's business and stays: this reaches
        // exactly as far as it wrote.
        groups.retain(|g| !g["hooks"].as_array().is_some_and(|h| h.is_empty()));
        if groups.is_empty() {
            hooks.remove(event);
        }
        out.removed.push(event);
    }
    // …and the same one level up. Only when khor emptied it: a `hooks`
    // object that was already empty before this ran is not khor's to
    // tidy away.
    if !out.removed.is_empty() && hooks.is_empty() {
        root.remove("hooks");
    }

    // Only written when something changed: a run that removed nothing
    // must not rewrite somebody's file (a fresh trailing newline, a
    // re-serialised number) and call it idempotent.
    if !out.removed.is_empty() {
        let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
        write_keeping_mode(&path, format!("{text}\n").as_bytes())?;
    }
    Ok(out)
}

/// `None` when the file is absent, which is a normal state and not an
/// error — claude has simply never been configured here.
fn read_settings(path: &Path) -> Result<Option<serde_json::Value>, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(Some(serde_json::json!({})));
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| msg::settings_garbled(path.display(), e))
}

/// khor's recorded command under one event, read-only.
fn recorded_command(settings: Option<&serde_json::Value>, event: &str) -> Option<String> {
    settings?["hooks"][event]
        .as_array()?
        .iter()
        .flat_map(|g| g["hooks"].as_array().into_iter().flatten())
        .filter_map(|h| h["command"].as_str())
        .find(|c| is_khors(c))
        .map(str::to_owned)
}

/// The same lookup, but handing back the slot so a moved khor can be
/// corrected in place rather than appended beside itself.
fn khors_command_mut(groups: &mut [serde_json::Value]) -> Option<&mut serde_json::Value> {
    for g in groups {
        let Some(entries) = g.get_mut("hooks").and_then(|h| h.as_array_mut()) else {
            continue;
        };
        for e in entries {
            let Some(slot) = e.get_mut("command") else { continue };
            if slot.as_str().is_some_and(is_khors) {
                return Some(slot);
            }
        }
    }
    None
}

/// Whole-file write via tmp+rename, **keeping the mode the file already
/// had**. This is not khor's file: turning someone's 644 settings into
/// khor's idea of a private file is a change they did not ask for and
/// would not see.
///
/// The temp name carries the pid, which is the rule `khor_sync::store`
/// had to learn the hard way — a fixed `.tmp` beside a shared path is
/// two writers racing to rename the same name.
fn write_keeping_mode(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(msg::cant_make_dir_for)?;
    }
    let tmp = path.with_extension(format!("khor-tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| msg::cant_write(tmp.display(), e))?;
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(path) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &tmp,
            std::fs::Permissions::from_mode(meta.permissions().mode()),
        );
    }
    std::fs::rename(&tmp, path).map_err(|e| msg::cant_place(path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptor::Proc;

    fn fixture_home() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors")
    }

    fn claude_at_fixture() -> Claude {
        Claude::at(fixture_home().join("claude/.claude"))
    }

    /// Every line shape a real transcript holds (sampled on this
    /// machine, 2026-08-17): a spoken user string, a plumbing user
    /// string, an assistant turn of thinking + text + tool_use blocks,
    /// a tool_result riding a user line, vendor bookkeeping, and a
    /// broken line. Only the speech survives, in order.
    #[test]
    fn a_transcript_keeps_the_speech_and_drops_the_plumbing() {
        let text = concat!(
            r#"{"type":"file-history-snapshot","messageId":"x"}"#, "\n",
            r#"{"type":"user","message":{"role":"user","content":"look at the docs"}}"#, "\n",
            r#"{"type":"user","message":{"role":"user","content":"<command-name>/compact</command-name>"}}"#, "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"reading","signature":"s"},{"type":"text","text":"here is what I found"},{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#, "\n",
            "not json at all\n",
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"...","is_error":false}]}}"#, "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"done"}]}}"#, "\n",
        );
        assert_eq!(
            utterances_of(text),
            vec![
                Utterance::User("look at the docs".into()),
                Utterance::Thought("reading".into()),
                Utterance::Agent("here is what I found".into()),
                Utterance::Tool("Bash".into()),
                Utterance::Agent("done".into()),
            ]
        );
    }

    /// The lookup runs `clean_leaf` over the file names because the id's
    /// leaf is the lossy product of that same cleaning — a full uuid
    /// (36 chars) must be found by its 24-char cleaned form.
    #[test]
    fn a_transcript_is_found_by_the_ids_lossy_leaf() {
        let dir = std::env::temp_dir().join(format!("khor-transcript-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let proj = dir.join(".claude/projects/-some-cwd");
        std::fs::create_dir_all(&proj).unwrap();
        let uuid = "2f1115e7-ea8d-4f30-bc3d-8516fc64c8a5";
        std::fs::write(
            proj.join(format!("{uuid}.jsonl")),
            r#"{"type":"user","message":{"role":"user","content":"hello"}}"#,
        )
        .unwrap();
        let leaf = crate::live::clean_leaf(uuid);
        assert_ne!(leaf, uuid, "the leaf is lossy, or this test tests nothing");
        let got = Claude::at(dir.join(".claude")).transcript(&leaf).unwrap();
        assert_eq!(got, vec![Utterance::User("hello".into())]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn proc_of(started_ms: i64) -> Proc {
        Proc { name: "claude".into(), started_ms, cwd: None, ppid: None }
    }

    fn word_at(sweep: &Sweep, id: &str) -> Option<State> {
        sweep
            .rows
            .iter()
            .find(|r| id_for(&r.vendor_session_id).0 == id)
            .map(|r| r.word)
    }

    /// Every status word the fixture tree spells, mapped. The narrowing
    /// is the point: an unrecognized `waitingFor` is not 待批 and not
    /// 空闲, it is nothing.
    #[test]
    fn the_status_vocabulary_maps_only_what_it_knows() {
        assert_eq!(word_of(Some("busy"), None), Some(State::Busy));
        assert_eq!(word_of(Some("idle"), None), Some(State::Idle));
        assert_eq!(
            word_of(Some("waiting"), Some(WAITING_FOR_PERMISSION)),
            Some(State::Blocked)
        );
        assert_eq!(word_of(Some("waiting"), Some("user input")), None);
        assert_eq!(word_of(Some("waiting"), None), None);
        assert_eq!(word_of(Some("meditating"), None), None);
        assert_eq!(word_of(None, None), None);
    }

    /// The flagship: a permission prompt on disk is 待批 on the row, and
    /// a busy session next to it is not.
    #[test]
    fn a_permission_prompt_on_disk_is_the_blocked_word() {
        // The fixture pids are alive only because this table says so —
        // no real process is involved, which is what makes the test
        // closed.
        let procs = Procs::of([
            (4001, proc_of(1_700_000_000_000)),
            (4002, proc_of(1_700_000_100_000)),
            (4003, proc_of(1_700_000_200_000)),
        ]);
        let sweep = claude_at_fixture().sweep(&procs);
        assert_eq!(
            word_at(&sweep, "tui/11111111-1111-4111-8111"),
            Some(State::Blocked),
            "waitingFor a permission prompt is the one waiting that is 待批"
        );
        assert_eq!(word_at(&sweep, "tui/22222222-2222-4222-8222"), Some(State::Busy));
        assert_eq!(word_at(&sweep, "tui/33333333-3333-4333-8333"), Some(State::Idle));
    }

    /// The graveyard control group, claude side: the same tree that just
    /// produced three rows produces none once nothing in it is running.
    /// Status files left behind by long-gone processes are history, and
    /// history is not a row.
    #[test]
    fn status_files_with_nothing_running_behind_them_are_not_rows() {
        let sweep = claude_at_fixture().sweep(&Procs::default());
        assert_eq!(sweep.rows.len(), 0);
        assert_eq!(
            sweep.unmapped, 1,
            "the one file khor cannot parse stays counted even here: its pid \
             is inside the part that did not parse, so whether it is history \
             or a live session khor has gone blind to is exactly what is unknown"
        );
    }

    /// Prove the finder is alive before believing an absence: the same
    /// sweep that finds the readable sessions is the one reporting the
    /// unreadable ones, and it counts them rather than dropping them
    /// silently.
    #[test]
    fn a_status_word_it_has_never_seen_makes_no_row_and_is_counted() {
        let procs = Procs::of([
            (4001, proc_of(1_700_000_000_000)),
            (4002, proc_of(1_700_000_100_000)),
            (4003, proc_of(1_700_000_200_000)),
            // The two unreadable ones: an unknown waitingFor, and a file
            // whose layout does not parse at all.
            (4004, proc_of(1_700_000_300_000)),
            (4005, proc_of(1_700_000_400_000)),
        ]);
        let sweep = claude_at_fixture().sweep(&procs);
        assert_eq!(
            sweep.rows.len(),
            3,
            "the three readable ones still appear — the finder works"
        );
        assert_eq!(word_at(&sweep, "tui/44444444-4444-4444-8444"), None);
        assert_eq!(
            sweep.unmapped, 2,
            "an unknown waitingFor and an unparseable file, both counted"
        );
    }

    /// A pid the process table does not vouch for means the file
    /// outlived its process: claude deletes it on a clean exit, so this
    /// is an ending nobody recorded.
    ///
    /// Written at test time rather than committed, because the assertion
    /// is about *recency*: a fixture with a baked-in timestamp would
    /// prove this today and quietly stop proving it once it aged past
    /// the window.
    #[test]
    fn a_status_file_that_outlived_its_process_is_failed_only_while_it_is_news() {
        let now = crate::live::now_ms();
        let dir = std::env::temp_dir().join(format!("khor-crash-{}", std::process::id()));
        let sessions = dir.join(".claude/sessions");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&sessions).unwrap();
        let write = |pid: u32, sid: &str, at: i64| {
            let body = serde_json::json!({
                "pid": pid, "sessionId": sid, "cwd": "/w/gone",
                "startedAt": at - 1000, "procStart": "Sat Aug 15 12:00:00 2026",
                "version": "2.1.233", "kind": "interactive", "entrypoint": "cli",
                "name": format!("gone-{pid}"), "nameSource": "derived",
                "status": "busy", "updatedAt": at, "statusUpdatedAt": at,
            });
            std::fs::write(
                sessions.join(format!("{pid}.json")),
                serde_json::to_vec_pretty(&body).unwrap(),
            )
            .unwrap();
        };
        write(6001, "66666666-6666-4666-8666-666666666666", now - 60_000);
        write(6002, "aaaaaaaa-6666-4666-8666-666666666666", now - CRASH_GRACE_MS - 60_000);

        // Nothing alive at all: every pid in that tree is gone.
        let sweep = Claude::at(dir.join(".claude")).sweep(&Procs::default());
        assert_eq!(sweep.rows.len(), 1, "the week-old crash is history, not a row");
        assert_eq!(sweep.rows[0].word, State::Failed);
        assert_eq!(
            id_for(&sweep.rows[0].vendor_session_id).0,
            "tui/66666666-6666-4666-8666"
        );

        // The control: with the process table vouching for them, the
        // same two files are ordinary live rows — which proves the
        // absence above came from the crash rule and not from the tree
        // being unreadable.
        let alive = Procs::of([
            (6001, proc_of(now - 61_000)),
            (6002, proc_of(now - CRASH_GRACE_MS - 61_000)),
        ]);
        let sweep = Claude::at(dir.join(".claude")).sweep(&alive);
        assert_eq!(sweep.rows.len(), 2);
        assert!(sweep.rows.iter().all(|r| r.word == State::Busy));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The hook glue end to end on payloads: an observed session
    /// registers itself on first sight; the idle-notification does NOT
    /// become 待批 (a badge that cannot reach zero is no badge); a
    /// permission ask does; SessionEnd settles it.
    #[test]
    fn hook_payloads_move_an_observed_session() {
        let root = std::env::temp_dir().join(format!("khor-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let k = crate::live::LiveKind::new(root.clone(), khor_core::DeviceId([9; 32]));
        let payload = |event: &str, extra: &str| {
            format!(
                r#"{{"session_id":"55E-fake.uuid","cwd":"/home/u/proj","hook_event_name":"{event}"{extra}}}"#
            )
        };

        hook(&k, &payload("SessionStart", "")).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!(row.id.0, "tui/55e-fakeuuid");
        assert_eq!(row.title, "proj");
        assert_eq!(row.state.state, State::Idle, "started, waiting for the first prompt");

        hook(&k, &payload("UserPromptSubmit", "")).unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Busy);

        let idle_note = payload("Notification", r#","message":"Claude is waiting for your input""#);
        assert!(matches!(hook(&k, &idle_note).unwrap(), Hooked::Ignored));
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Busy, "long-idle must not become 待批");

        let ask = payload("Notification", r#","message":"Claude needs your permission to use Bash""#);
        hook(&k, &ask).unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Blocked);

        hook(&k, &payload("Stop", "")).unwrap();
        let row = &k.rows(|_| 0)[0];
        assert_eq!((row.state.state, row.unread), (State::Done, 1));

        hook(&k, &payload("SessionEnd", "")).unwrap();
        assert_eq!(k.rows(|_| 0)[0].state.state, State::Idle, "a clean end is idle");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ── installing the hook ─────────────────────────────────

    /// A home of this test's own. **Every hook test runs against one**,
    /// and none of them can reach `~/.claude`: the path is a parameter
    /// all the way down (`super::vendor_home`), so there is no
    /// configuration this suite could touch by accident.
    fn hook_home(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("khor-hooks-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        dir
    }

    /// A settings file shaped like a real one: other top-level keys,
    /// somebody else's hooks, under events khor also wants.
    ///
    /// **Literal text, not `json!`, and the keys are deliberately not in
    /// alphabetical order.** Both halves are load-bearing: a fixture
    /// built as a `Value` would be ordered by the same machinery the
    /// order assertion is checking, so it would agree with any output
    /// however it was sorted — and alphabetical keys would agree with a
    /// sorting one by accident.
    const USED_SETTINGS: &str = r#"{
  "env": {
    "SOME_VAR": "1"
  },
  "permissions": {
    "allow": [
      "Bash(ls:*)"
    ]
  },
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "sh \"$HOME/.mandala/hook-report.sh\" running"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "sh /opt/audit.sh"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "sh \"$HOME/.mandala/hook-report.sh\" done"
          }
        ]
      }
    ]
  },
  "model": "some-model"
}
"#;

    /// Top-level key names in the order they appear **in the text**.
    ///
    /// Read off the file rather than off a parsed `Value`, because the
    /// parse is exactly what is under test: a `Value` whose map sorts
    /// would report a sorted order for both sides and agree with itself.
    /// Two-space indent is what `to_string_pretty` writes and what the
    /// fixture above is written in, so a top-level key is a line
    /// starting with exactly two spaces and a quote.
    fn top_level_keys(text: &str) -> Vec<String> {
        text.lines()
            .filter_map(|l| l.strip_prefix("  \""))
            .filter_map(|l| l.split_once("\":"))
            .map(|(k, _)| k.to_owned())
            .collect()
    }

    /// Every `(path, value)` leaf of a JSON tree, so that "nothing that
    /// was here changed" can be asserted rather than eyeballed.
    fn leaves(v: &serde_json::Value, at: String, out: &mut Vec<(String, String)>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    leaves(x, format!("{at}.{k}"), out);
                }
            }
            serde_json::Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    leaves(x, format!("{at}[{i}]"), out);
                }
            }
            other => out.push((at, other.to_string())),
        }
    }

    fn leaf_set(v: &serde_json::Value) -> Vec<(String, String)> {
        let mut out = Vec::new();
        leaves(v, String::new(), &mut out);
        out.sort();
        out
    }

    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// A machine where claude was never configured: installing writes
    /// the file, and every event khor needs is on it afterwards.
    #[test]
    fn installing_where_there_is_no_settings_file_writes_one() {
        let home = hook_home("fresh");
        let before = hooks_report(&home).unwrap();
        assert!(!before.settings_exist, "control: nothing there yet");
        assert!(
            before.events.iter().all(|(_, s)| *s == Installed::Missing),
            "and khor is in none of it"
        );

        let done = install_hooks(&home).unwrap();
        assert_eq!(done.added.len(), HOOK_EVENTS.len());
        assert!(done.repointed.is_empty() && done.unchanged.is_empty());

        let after = hooks_report(&home).unwrap();
        assert!(after.settings_exist);
        assert!(
            after.events.iter().all(|(_, s)| *s == Installed::Here),
            "every event khor asked for now names this binary"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// **Installed twice is installed once**, asserted on the bytes.
    ///
    /// Comparing the file text rather than counting entries is the point:
    /// an implementation that appended a second identical group every
    /// run would still report five events installed and would still pass
    /// a "khor is present" check.
    #[test]
    fn installing_twice_changes_nothing_the_second_time() {
        let home = hook_home("twice");
        install_hooks(&home).unwrap();
        let once = std::fs::read_to_string(settings_path(&home)).unwrap();

        let again = install_hooks(&home).unwrap();
        let twice = std::fs::read_to_string(settings_path(&home)).unwrap();
        assert_eq!(once, twice, "byte for byte");
        assert!(again.added.is_empty() && again.repointed.is_empty());
        assert_eq!(
            again.unchanged.len(),
            HOOK_EVENTS.len(),
            "and it says so, which is the only way a person can tell"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// **Only additions.** Somebody else's hooks, under the same events,
    /// beside other settings: everything that was there is still there,
    /// unchanged, in the same order.
    #[test]
    fn installing_beside_someone_elses_hooks_only_adds() {
        let home = hook_home("merge");
        std::fs::write(settings_path(&home), USED_SETTINGS).unwrap();
        let original: serde_json::Value = serde_json::from_str(USED_SETTINGS).unwrap();
        let keys_before = top_level_keys(USED_SETTINGS);
        assert_eq!(
            keys_before,
            vec!["env", "permissions", "hooks", "model"],
            "the probe reads the order off the text, and this order is not alphabetical \
             — a sorting implementation has something to disagree with"
        );

        install_hooks(&home).unwrap();
        let written = std::fs::read_to_string(settings_path(&home)).unwrap();
        let after = read_json(&settings_path(&home));

        // Nothing that existed is gone or different.
        let before_leaves = leaf_set(&original);
        let after_leaves = leaf_set(&after);
        let lost: Vec<&(String, String)> =
            before_leaves.iter().filter(|l| !after_leaves.contains(l)).collect();
        assert!(lost.is_empty(), "install changed or dropped: {lost:?}");

        // Including the order of the file's own keys — the reason
        // serde_json is built with `preserve_order` (Cargo.toml).
        assert_eq!(
            top_level_keys(&written),
            keys_before,
            "khor must not re-order someone's settings"
        );

        // Their hook is still the first thing under the events they had.
        assert_eq!(
            after["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"],
            original["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        );
        // Khor arrived beside it, not on top of it.
        assert_eq!(after["hooks"]["UserPromptSubmit"].as_array().unwrap().len(), 2);
        assert!(
            hooks_report(&home).unwrap().events.iter().all(|(_, s)| *s == Installed::Here)
        );
        // An event khor never asked for is untouched, matcher and all.
        assert_eq!(after["hooks"]["PreToolUse"], original["hooks"]["PreToolUse"]);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A khor that moved is corrected where it stands, not added a
    /// second time — and `hooks` says which of the two it is looking at
    /// before the fix, because "installed" and "installed elsewhere"
    /// have different answers.
    #[test]
    fn a_khor_that_moved_is_repointed_not_duplicated() {
        let home = hook_home("moved");
        let stale = "'/somewhere/old/khor' state --hook";
        std::fs::write(
            settings_path(&home),
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": stale }] }] }
            }))
            .unwrap(),
        )
        .unwrap();

        let before = hooks_report(&home).unwrap();
        let stop = before.events.iter().find(|(e, _)| *e == "Stop").unwrap();
        assert_eq!(
            stop.1,
            Installed::Elsewhere(stale.to_owned()),
            "not Here, and not Missing — a person needs to see which"
        );

        let done = install_hooks(&home).unwrap();
        assert_eq!(done.repointed, vec!["Stop"]);
        let after = read_json(&settings_path(&home));
        assert_eq!(
            after["hooks"]["Stop"].as_array().unwrap().len(),
            1,
            "corrected in place; a second entry would fire khor twice a turn"
        );
        assert_eq!(after["hooks"]["Stop"][0]["hooks"][0]["command"], hook_command().unwrap());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A settings file khor cannot parse is refused **and left alone**.
    /// It is the user's only copy, and khor's idea of what it contained
    /// is not a repair.
    #[test]
    fn a_settings_file_it_cannot_read_is_refused_and_not_touched() {
        let home = hook_home("garbled");
        let broken = "{ \"hooks\": { \"Stop\": [ } ";
        std::fs::write(settings_path(&home), broken).unwrap();

        assert!(install_hooks(&home).is_err());
        assert!(hooks_report(&home).is_err());
        assert_eq!(
            std::fs::read_to_string(settings_path(&home)).unwrap(),
            broken,
            "still exactly as the user left it"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    // ── taking the hook back out ────────────────────────────

    /// **The round trip on somebody's real file: install, uninstall, and
    /// the bytes are what they were.**
    ///
    /// Compared as text, not as a parsed `Value`, and against the
    /// fixture the install tests use — which holds other top-level keys
    /// and other people's hooks under events khor also wants. A
    /// comparison of parsed values would agree even if uninstall had
    /// reordered every key in the file, and reordering somebody's
    /// configuration is exactly the harm `preserve_order` is carried
    /// for.
    #[test]
    fn uninstalling_puts_the_file_back_byte_for_byte() {
        let home = hook_home("roundtrip");
        std::fs::write(settings_path(&home), USED_SETTINGS).unwrap();

        install_hooks(&home).unwrap();
        let installed = std::fs::read_to_string(settings_path(&home)).unwrap();
        assert_ne!(installed, USED_SETTINGS, "control: installing did change the file");

        let gone = uninstall_hooks(&home).unwrap();
        assert_eq!(gone.removed.len(), HOOK_EVENTS.len(), "every event khor added came out");
        assert!(gone.absent.is_empty());
        assert_eq!(
            std::fs::read_to_string(settings_path(&home)).unwrap(),
            USED_SETTINGS,
            "the file is the user's again, to the byte"
        );
        assert!(
            hooks_report(&home).unwrap().events.iter().all(|(_, s)| *s == Installed::Missing),
            "…and khor now reports itself as not installed"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// **Only khor's entries.** Somebody else's hooks under the very
    /// events khor uses survive, and so does every other key.
    ///
    /// Asserted leaf by leaf rather than by eyeballing one path: a
    /// removal that took a neighbour with it would still leave the
    /// event key standing.
    #[test]
    fn uninstalling_leaves_every_hook_that_is_not_khors() {
        let home = hook_home("neighbours");
        std::fs::write(settings_path(&home), USED_SETTINGS).unwrap();
        let mine = leaf_set(&read_json(&settings_path(&home)));

        install_hooks(&home).unwrap();
        uninstall_hooks(&home).unwrap();

        assert_eq!(leaf_set(&read_json(&settings_path(&home))), mine, "nothing else moved");
        assert_eq!(
            top_level_keys(&std::fs::read_to_string(settings_path(&home)).unwrap()),
            vec!["env", "permissions", "hooks", "model"],
            "and the user's key order is untouched"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// **Removed twice is removed once**, and the second run says so
    /// rather than only being harmless. Asserted on the bytes, because
    /// an implementation that rewrote the file every time would still
    /// report nothing removed and would still leave a working config.
    #[test]
    fn uninstalling_twice_changes_nothing_the_second_time() {
        let home = hook_home("twice-off");
        std::fs::write(settings_path(&home), USED_SETTINGS).unwrap();
        install_hooks(&home).unwrap();
        uninstall_hooks(&home).unwrap();
        let once = std::fs::read_to_string(settings_path(&home)).unwrap();

        let again = uninstall_hooks(&home).unwrap();
        assert!(again.removed.is_empty(), "nothing left to take: {again:?}");
        assert_eq!(again.absent.len(), HOOK_EVENTS.len(), "and it says which events those were");
        assert_eq!(std::fs::read_to_string(settings_path(&home)).unwrap(), once);
        let _ = std::fs::remove_dir_all(&home);
    }

    /// **A hook belonging to a different khor comes out too.**
    ///
    /// `hooks_report` calls that `Installed::Elsewhere` — a khor that
    /// moved, or a second copy on this machine — and the alternative
    /// here is a machine where khor reports something installed that
    /// nothing khor offers can remove.
    ///
    /// The control runs first: the report must actually say `Elsewhere`
    /// beforehand, or this would pass by removing something that was
    /// never recognised as khor's in the first place.
    #[test]
    fn a_hook_left_by_another_khor_can_still_be_taken_out() {
        let home = hook_home("stale-off");
        let stale = format!("'/somewhere/else/khor'{HOOK_VERB}");
        std::fs::write(
            settings_path(&home),
            serde_json::to_string_pretty(&serde_json::json!({
                "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": stale }] }] }
            }))
            .unwrap(),
        )
        .unwrap();
        let before = hooks_report(&home).unwrap();
        assert!(
            before.events.iter().any(|(e, s)| *e == "Stop" && *s == Installed::Elsewhere(stale.clone())),
            "control: the report has to see it as another khor's first: {before:?}"
        );

        let gone = uninstall_hooks(&home).unwrap();
        assert_eq!(gone.removed, vec!["Stop"]);
        assert!(
            hooks_report(&home).unwrap().events.iter().all(|(_, s)| *s == Installed::Missing),
            "nothing of any khor is left"
        );
        // Khor made the `hooks` object empty, so khor takes it away —
        // an empty container says exactly what its absence says.
        assert_eq!(read_json(&settings_path(&home)), serde_json::json!({}));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A machine where claude was never configured: there is nothing to
    /// remove, and **no file is created in order to remove it from**.
    #[test]
    fn uninstalling_where_there_is_no_settings_file_writes_nothing() {
        let home = hook_home("fresh-off");
        let gone = uninstall_hooks(&home).unwrap();
        assert!(gone.removed.is_empty());
        assert_eq!(gone.absent.len(), HOOK_EVENTS.len());
        assert!(
            !settings_path(&home).exists(),
            "removing nothing must not leave a trace that something happened"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// A file khor cannot parse is refused here too, and left alone —
    /// the same reason install refuses it.
    #[test]
    fn uninstalling_refuses_a_settings_file_it_cannot_read() {
        let home = hook_home("garbled-off");
        let broken = "{ \"hooks\": { \"Stop\": [ } ";
        std::fs::write(settings_path(&home), broken).unwrap();

        assert!(uninstall_hooks(&home).is_err());
        assert_eq!(
            std::fs::read_to_string(settings_path(&home)).unwrap(),
            broken,
            "still exactly as the user left it"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// The hook command names this binary by absolute path and ends with
    /// khor's own verb — the marker that makes an install idempotent
    /// across a move, and the reason a PATH-less claude can still run it.
    #[test]
    fn the_hook_command_is_this_binary_and_khors_own_verb() {
        let cmd = hook_command().unwrap();
        assert!(cmd.ends_with(HOOK_VERB), "{cmd}");
        assert!(is_khors(&cmd));
        assert!(cmd.starts_with('\''), "quoted, because a path may hold spaces: {cmd}");
        let exe = std::env::current_exe().unwrap();
        assert!(cmd.contains(&exe.to_string_lossy().to_string()));
        assert!(!is_khors("sh \"$HOME/.mandala/hook-report.sh\" running"));
        assert_eq!(shell_quoted("/a b/kh'or"), "'/a b/kh'\\''or'");
    }

    /// `claude -r` resumes a session into a new process while the old
    /// one is still running: two live status files, one `sessionId`.
    /// Observed on this machine, and it is one session — so it is one
    /// row, wearing the freshest of the two words.
    #[test]
    fn a_resumed_session_is_one_row_not_two() {
        let resumed = Claude::at(fixture_home().join("resume/.claude"));
        let procs = Procs::of([
            (5001, proc_of(1_700_000_000_000)),
            (5002, proc_of(1_700_000_600_000)),
        ]);
        let sweep = resumed.sweep(&procs);
        assert_eq!(sweep.rows.len(), 1, "one session id, one row");
        assert_eq!(sweep.rows[0].title, "resumed-now", "the freshest file wins");
        assert_eq!(sweep.rows[0].word, State::Busy);
        assert_eq!(sweep.unmapped, 0);
        // **Both processes, not just the winner's.** Found on this
        // machine: a claude resumed into a second process left the first
        // one accounted for by nothing, so the tmux session around it
        // turned up as a shell row of its own — one running agent shown
        // twice, once under its own name and once as furniture.
        let mut pids = sweep.rows[0].pids.clone();
        pids.sort_unstable();
        assert_eq!(
            pids,
            vec![5001, 5002],
            "the surviving row stands for every live process of that session"
        );
    }
}
