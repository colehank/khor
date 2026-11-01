//! Codex rollouts and live thread writers, on macOS, Linux and Windows.
//!
//! Current Codex holds an exclusive OS lock on
//! `thread-writer-locks/<thread-id>.lock` for each loaded writer. A
//! nonblocking shared-lock probe distinguishes an owned thread from a
//! stale file without creating, rewriting or deleting vendor files.
//! See upstream `codex-rs/rollout/src/writer_lock.rs`. This works for a
//! daemon hosting several threads and for resumed threads: neither the
//! process's start time nor its cwd identifies those conversations.
//!
//! A writer lock establishes a loaded thread, NOT an attached terminal.
//! Codex can retain idle threads after a frontend disconnects. The lock
//! contains no frontend PID. Birth time and cwd cannot prove selection.
//! Subagent sources can inherit the same TUI originator and cwd; they
//! remain discoverable but never participate in terminal ownership.
//! Native terminal thread-id metadata identifies the selected thread, including
//! resumes and switches. Without that signal ambiguous/resumed threads stay PID-less. Homes
//! without the lock directory retain the older process/start-time join, whose
//! limitations include daemon threads and resumes.
//!
//! The rollout's `session_meta.id` identifies the thread; `session_id`
//! may be shared by its descendants. Reverted threads can have a new
//! `_<rollout-id>` filename suffix while keeping that thread ID.
//!
//! Visible messages are legacy `user_message` / `agent_message` events
//! or paginated `item_completed` UserMessage / AgentMessage events.
//! Response-item messages can contain injected context and duplicates;
//! only their tool calls are replayed here. See upstream rollout policy.
//! Turn events give busy, done and interrupted states. Approval waits
//! are not persisted, so disk discovery cannot report 待批.

use std::path::{Path, PathBuf};

use khor_core::State;

use super::{Adaptor, Proc, Procs, Sighting, Sweep};

/// The executable base name used by the legacy process join.
pub(super) const PROCESS_NAME: &str = "codex";

/// How far from a process's start its rollout may be stamped. Generous
/// forward (the file is created after the process, and a slow start
/// stretches that), barely negative (only clock granularity should ever
/// put the file first).
const JOIN_EARLIEST_MS: i64 = -5_000;
const JOIN_LATEST_MS: i64 = 60_000;

/// How many rollouts back [`Codex::places`] reads. One head read each
/// (93 on the machine this was written on); the bound is what keeps
/// that a constant rather than a function of how long codex has been
/// installed.
const PLACES_SCAN: usize = 300;

/// A rollout, identified without opening it.
struct Rollout {
    path: PathBuf,
    /// Session start, from the uuid v7.
    started_ms: i64,
}

/// The turn events that carry a word. Everything else in the transcript
/// (messages, reasoning, token counts) describes content, not state.
fn word_of(event: &str) -> Option<State> {
    match event {
        "task_started" | "turn_started" => Some(State::Busy),
        // Codex says this outright, so the row can hold a badge that
        // clears by being looked at.
        "task_complete" | "turn_complete" => Some(State::Done),
        // The user interrupted the turn. Not 完成 (nothing finished) and
        // not 中断, which docs/SESSION.md reserves for an error the
        // process survived. The turn is simply over.
        "turn_aborted" => Some(State::Idle),
        // An error with the process still alive — the definition of 中断.
        "error" => Some(State::Errored),
        _ => None,
    }
}

/// The ms encoded in a uuid v7, or `None` for any other version.
fn uuid_v7_ms(uuid: &str) -> Option<i64> {
    let hex: String = uuid.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    if hex.as_bytes()[12] != b'7' {
        return None;
    }
    i64::from_str_radix(&hex[..12], 16).ok()
}

/// `rollout-2026-08-06T21-49-18-019fd755-….jsonl` → its uuid.
fn uuid_in_name(name: &str) -> Option<&str> {
    let stem = name.strip_prefix("rollout-")?.strip_suffix(".jsonl")?;
    let thread = stem.split_once('_').map_or(stem, |(thread, _)| thread);
    thread.get(thread.len().checked_sub(36)?..)
}

pub struct Codex {
    root: PathBuf,
}

impl Codex {
    pub fn at(root: PathBuf) -> Codex {
        Codex { root }
    }

    /// Every rollout whose name gives a start instant. Names only — no
    /// file is opened here.
    fn rollouts(&self) -> Vec<Rollout> {
        let mut out = Vec::new();
        collect(&self.root.join("sessions"), &mut out, 0);
        // thread/revert leaves immutable older rollouts behind. Resolve
        // discovery, preview and replay to the same newest record.
        let mut threads = std::collections::BTreeMap::<String, Rollout>::new();
        for rollout in out {
            let Some(id) = rollout
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(uuid_in_name)
            else {
                continue;
            };
            let replace = threads.get(id).is_none_or(|old| {
                (mtime_ms(&rollout.path), &rollout.path) > (mtime_ms(&old.path), &old.path)
            });
            if replace {
                threads.insert(id.to_owned(), rollout);
            }
        }
        threads.into_values().collect()
    }

    /// The rollout a process is writing: nearest start inside the
    /// window, confirmed by working directory when both are known.
    fn rollout_of<'a>(&self, proc: &Proc, rollouts: &'a [Rollout], taken: &[&'a Path]) -> Join<'a> {
        let mut candidates: Vec<&Rollout> = rollouts
            .iter()
            .filter(|r| !taken.contains(&r.path.as_path()))
            .filter(|r| {
                let d = r.started_ms - proc.started_ms;
                (JOIN_EARLIEST_MS..=JOIN_LATEST_MS).contains(&d)
            })
            .collect();
        if candidates.is_empty() {
            return Join::NoSession;
        }
        candidates.sort_by_key(|r| (r.started_ms - proc.started_ms).abs());
        let mut unreadable = false;
        for r in candidates {
            let Some(meta) = read_meta(&r.path) else {
                unreadable = true;
                continue;
            };
            if meta.is_subagent {
                continue;
            }
            // The clock put them together; the directory confirms it.
            // Only when both sides know their cwd — sysinfo cannot
            // always read another process's, and an unknown must not
            // veto a match the clock already made.
            match (&proc.cwd, &meta.cwd) {
                (Some(a), Some(b)) if Path::new(b) != a.as_path() => continue,
                _ => return Join::Session(r, meta),
            }
        }
        // A candidate that would not open is the vendor's file moving;
        // one that opened and named another directory is simply not
        // this process's, and a process left with only those is a
        // process that started no session of its own.
        if unreadable {
            Join::Unreadable
        } else {
            Join::NoSession
        }
    }
}

/// Results of the legacy process/start-time join. A host without a
/// matching rollout is not counted as an unreadable session.
enum Join<'a> {
    Session(&'a Rollout, Meta),
    /// No rollout started anywhere near this process: a host, not a
    /// session. Not a row and not a count.
    NoSession,
    /// A rollout in the window, and khor could not read it — the
    /// 适配器过时 signal, counted.
    Unreadable,
}

impl Codex {
    /// The vendor's **full** thread id behind a lossy khor leaf — the
    /// rollout's own filename (`claude::full_session_id`'s twin: a row
    /// id truncates the uuid, resume wants the whole thing, and the
    /// file is the one place that never forgot it). This answering at
    /// all is also the vendor verdict: whichever store resolves a leaf
    /// owns the session (`Node::takeover`'s fork).
    pub fn full_session_id(&self, leaf: &str) -> Option<String> {
        let path = self.rollout_of_leaf(leaf)?;
        uuid_in_name(path.file_name()?.to_str()?).map(str::to_owned)
    }

    pub(crate) fn unique_session_id(
        &self,
        leaf: &str,
        exact: Option<&str>,
    ) -> Result<Option<String>, ()> {
        let mut chosen = None;
        for rollout in self.rollouts() {
            let Some(native) = rollout
                .path
                .file_name()
                .and_then(|part| part.to_str())
                .and_then(uuid_in_name)
            else {
                continue;
            };
            if crate::live::clean_leaf(native) != leaf || exact.is_some_and(|id| id != native) {
                continue;
            }
            if chosen.as_ref().is_some_and(|previous| previous != native) {
                return Err(());
            }
            chosen = Some(native.to_owned());
        }
        Ok(chosen)
    }
    /// A writer belongs to a thread, possibly inside a shared daemon. It is
    /// never permission to terminate that daemon or guess a terminal PID.
    pub(crate) fn has_writer(&self, uuid: &str) -> Result<bool, String> {
        writer_owned(
            &self
                .root
                .join("thread-writer-locks")
                .join(format!("{uuid}.lock")),
        )
        .map_err(|error| error.to_string())
    }

    /// The rollout behind a lossy khor leaf, by name alone.
    fn rollout_of_leaf(&self, leaf: &str) -> Option<PathBuf> {
        self.rollouts().into_iter().map(|r| r.path).find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(uuid_in_name)
                .is_some_and(|u| crate::live::clean_leaf(u) == leaf)
        })
    }

    /// The newest sentence in a leaf's rollout, as one preview line —
    /// claude's [`super::claude::Claude::last_said`] with codex's
    /// reading of the lines, and the same cache.
    pub fn last_said(&self, leaf: &str) -> Option<String> {
        let path = self.rollout_of_leaf(leaf)?;
        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
        super::claude::heard_last(path, mtime, utterances_of)
    }

    /// The rollout that belongs to this thread id — names carry the
    /// uuid, so no file is opened to find it.
    pub fn rollout_path(&self, uuid: &str) -> Option<PathBuf> {
        self.rollouts().into_iter().map(|r| r.path).find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(uuid_in_name)
                .is_some_and(|u| u == uuid)
        })
    }

    /// **Every directory this vendor remembers working in**, for the
    /// wizard's 「常去的地方」 (`khor_node::place`). One entry per
    /// rollout: `(cwd, 1, when the session started)`.
    ///
    /// **One per rollout, not one per directory** — unlike claude, this
    /// vendor files everything under a date, so there is no grouping on
    /// disk to borrow and each file has to say where it ran. Newest
    /// first and capped at [`PLACES_SCAN`]: the list this feeds is
    /// 「最近去过哪儿」, and the tail of a long history cannot change
    /// its head.
    pub fn places(&self) -> Vec<(PathBuf, u32, i64)> {
        let mut rollouts = self.rollouts();
        rollouts.sort_by_key(|r| std::cmp::Reverse(r.started_ms));
        rollouts.truncate(PLACES_SCAN);
        rollouts
            .iter()
            .filter_map(|r| Some((PathBuf::from(read_meta(&r.path)?.cwd?), 1, r.started_ms)))
            .collect()
    }

    /// The working directory the rollout's own first line records —
    /// what a takeover resumes into, wherever the resumer stands.
    pub fn recorded_cwd(&self, uuid: &str) -> Option<PathBuf> {
        let meta = read_meta(&self.rollout_path(uuid)?)?;
        meta.cwd.map(PathBuf::from)
    }

    /// The conversation as the rollout tells it, in the shared replay
    /// shapes ([`super::claude::Utterance`] — one mapping, two vendors,
    /// so both pasts read alike). `event_msg` lines carry the words
    /// (`user_message` / `agent_message`); tool acts are the
    /// `response_item` call lines. Reasoning is encrypted in the file
    /// and stays unread.
    pub fn transcript(&self, uuid: &str) -> Result<Vec<super::claude::Utterance>, String> {
        let path = self
            .rollout_path(uuid)
            .ok_or_else(|| khor_catalog::msg::takeover_no_record(uuid))?;
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        Ok(utterances_of(&text))
    }

    /// A bounded raw window for paginated history. Fragments and oversized
    /// records remain explicit; this does not change the legacy follow cursor.
    pub fn transcript_window(
        &self,
        uuid: &str,
        before: Option<u64>,
    ) -> Result<super::transcript_window::TranscriptWindow, String> {
        let path = self
            .rollout_path(uuid)
            .ok_or_else(|| khor_catalog::msg::takeover_no_record(uuid))?;
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        super::transcript_window::read_window(&mut file, before).map_err(|e| e.to_string())
    }

    /// The same conversation, **followed**: what the rollout has said
    /// since byte `from`, and where to continue.
    ///
    /// # Why this exists at all
    ///
    /// Without it a codex row read its whole record once and then never
    /// again — `Node::transcript_follow` answered `following: false`
    /// for it, and the pane stopped where the first read stopped. A
    /// conversation that is still going, painted once and then frozen,
    /// is worse than one that says it cannot be followed: nothing on
    /// screen says which.
    ///
    /// **The reason recorded for not doing it was wrong**, and it is
    /// worth saying so rather than quietly fixing it: the note said
    /// codex's rollout "is a different shape" and cannot seek. It is
    /// line-delimited JSON, exactly what claude's is for this purpose —
    /// the *contents* of the lines differ, and that difference is
    /// entirely inside [`utterances_of`], which both paths already
    /// share. A claim about one layer was used to close a question at
    /// another.
    ///
    /// The three cases and their answers are claude's, deliberately —
    /// see [`super::claude::Claude::transcript_since`], which has the
    /// argument in full. Grow: the new lines, `at` at the new end.
    /// Stay: nothing, `at` unchanged. Shrink or be replaced: the file
    /// whole with `rewound`, because byte 400 of a new file is not byte
    /// 400 of the old one. A half-written last line is left for next
    /// time — `at` stops at the last newline.
    pub fn transcript_since(&self, uuid: &str, from: u64) -> Result<super::claude::Follow, String> {
        use super::claude::Follow;
        use std::io::{Read, Seek, SeekFrom};
        let path = self
            .rollout_path(uuid)
            .ok_or_else(|| khor_catalog::msg::takeover_no_record(uuid))?;
        let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        let rewound = len < from;
        let start = if rewound { 0 } else { from };
        if len == start {
            return Ok(Follow {
                said: Vec::new(),
                at: len,
                rewound,
                following: true,
            });
        }
        f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        f.take(len - start)
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        let end = match buf.iter().rposition(|b| *b == b'\n') {
            Some(i) => i + 1,
            None => {
                return Ok(Follow {
                    said: Vec::new(),
                    at: start,
                    rewound,
                    following: true,
                });
            }
        };
        let text = String::from_utf8_lossy(&buf[..end]);
        Ok(Follow {
            said: utterances_of(&text),
            at: start + end as u64,
            rewound,
            following: true,
        })
    }
}

/// The rollout's lines as the shared replay shapes — **the one place
/// codex's file format is read**, so the whole record and a followed
/// tail cannot drift into two readings of the same bytes.
///
/// `event_msg` lines carry legacy or paginated visible messages;
/// tool acts are the `response_item` call lines. Reasoning is not replayed.
pub(crate) fn utterances_of(text: &str) -> Vec<super::claude::Utterance> {
    use super::claude::Utterance;
    let mut out = Vec::new();
    for line in text.lines() {
        {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            match v["type"].as_str() {
                Some("event_msg") => {
                    let p = &v["payload"];
                    match (p["type"].as_str(), p["message"].as_str()) {
                        (Some("user_message"), Some(m)) => out.push(Utterance::User(m.to_owned())),
                        (Some("agent_message"), Some(m)) => {
                            out.push(Utterance::Agent(m.to_owned()))
                        }
                        (Some("item_completed"), _) => {
                            let item = &p["item"];
                            let kind = item["type"].as_str();
                            if matches!(kind, Some("UserMessage" | "AgentMessage")) {
                                let text = item["content"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter(|c| matches!(c["type"].as_str(), Some("text" | "Text")))
                                    .filter_map(|c| c["text"].as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                if !text.is_empty() {
                                    out.push(if kind == Some("UserMessage") {
                                        Utterance::User(text)
                                    } else {
                                        Utterance::Agent(text)
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Some("response_item") => {
                    let p = &v["payload"];
                    if let Some(name) = tool_call_name(p) {
                        let input = decoded_tool_input(p);
                        // Shaped as claude's `tool_use` block so
                        // that both vendors' pasts go through the
                        // one translation (`cagent::said`). codex
                        // records an id and arguments under its own
                        // names; they are carried across rather
                        // than dropped, so an opened line shows
                        // what was actually run here too.
                        out.push(Utterance::Tool(serde_json::json!({
                            "type": "tool_use",
                            "id": p["call_id"].as_str().unwrap_or(name),
                            "name": name,
                            "input": input,
                        })));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Both normal replay and bounded scaffold preparation select the same
/// recorded input. A null arguments field deliberately falls back to input.
fn tool_call_name(payload: &serde_json::Value) -> Option<&str> {
    matches!(
        payload["type"].as_str(),
        Some("custom_tool_call" | "function_call" | "local_shell_call")
    )
    .then(|| payload["name"].as_str())
    .flatten()
}
fn tool_input_field(payload: &serde_json::Value) -> Option<(&'static str, &serde_json::Value)> {
    payload
        .get("arguments")
        .filter(|value| !value.is_null())
        .map(|value| ("arguments", value))
        .or_else(|| payload.get("input").map(|value| ("input", value)))
}
fn decoded_tool_input(payload: &serde_json::Value) -> serde_json::Value {
    decoded_tool_input_state(payload).0
}
fn decoded_tool_input_state(payload: &serde_json::Value) -> (serde_json::Value, bool) {
    let input = tool_input_field(payload).map(|(_, value)| value);
    if let Some(decoded) = input
        .and_then(|value| value.as_str())
        .and_then(|text| serde_json::from_str(text).ok())
    {
        return (decoded, true);
    }
    (input.cloned().unwrap_or(serde_json::Value::Null), false)
}
/// A long encoded input must first prepare its inner JSON with the same
/// bounded lexer. This is a field-selection fact, not a second vendor parser.
pub(crate) fn scaffold_json_input(value: &serde_json::Value) -> Option<(String, bool)> {
    let payload = &value["payload"];
    if value["type"].as_str() != Some("response_item") {
        return None;
    }
    tool_call_name(payload)?;
    let (field, input) = tool_input_field(payload)?;
    input.as_str()?;
    Some((
        format!("/payload/{field}"),
        decoded_tool_input_state(payload).1,
    ))
}

/// This vendor's name, which is also the category its rows carry — and,
/// since `crate::usage`, the category its spending carries too. Named
/// once for the reason [`super::claude::VENDOR`] gives: two spellings of
/// one vendor put one session in two groups, and nothing would report an
/// error about it.
pub const VENDOR: &str = "codex";

impl Adaptor for Codex {
    fn vendor(&self) -> &'static str {
        VENDOR
    }

    fn sweep(&self, procs: &Procs) -> Sweep {
        let mut sweep = Sweep::default();
        // The process table is the whole machine's, while the root is
        // this node's. When khor is not pointed at a codex home at all —
        // a second instance, a test — the codex running over there is
        // none of its business, and counting it would report "khor
        // cannot read this vendor" when the truth is "khor is not
        // looking at it".
        if !self.root.join("sessions").is_dir() && !self.root.join("thread-writer-locks").is_dir() {
            return sweep;
        }
        let locks = self.root.join("thread-writer-locks");
        match std::fs::metadata(&locks) {
            Ok(meta) if meta.is_dir() => return self.sweep_writers(&locks, procs),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            _ => {
                sweep.unmapped += 1;
                return sweep;
            }
        }
        let live = procs.named(PROCESS_NAME);
        if live.is_empty() {
            // Nothing running means nothing to show. The transcripts on
            // disk are history and history is not a row.
            return sweep;
        }
        let rollouts = self.rollouts();
        let mut taken: Vec<&Path> = Vec::new();
        for (pid, proc) in live {
            let (rollout, meta) = match self.rollout_of(proc, &rollouts, &taken) {
                Join::Session(r, m) => (r, m),
                // A host, not a session (`Join::NoSession`).
                Join::NoSession => continue,
                // A codex is running and khor cannot read the session it
                // started: an old-format transcript, a layout change.
                // Counted, never guessed at.
                Join::Unreadable => {
                    sweep.unmapped += 1;
                    continue;
                }
            };
            taken.push(&rollout.path);
            let (word, at_ms) = last_word(&rollout.path)
                // A transcript khor understands completely that records
                // no turn: the session started and nothing has happened
                // in it. That is 空闲, and it is a reading rather than a
                // guess — the absence of turn events is itself the
                // evidence.
                .unwrap_or((State::Idle, rollout.started_ms));
            sweep.rows.push(Sighting {
                vendor_session_id: meta.session_id.clone(),
                title: title_of(meta.cwd.as_deref()),
                word,
                at_ms,
                // Only the legacy join can identify a process.
                pids: vec![pid],
                // The rollout's own first line, already read for the
                // title one field up.
                cwd: meta.cwd,
            });
        }
        sweep
    }
}

impl Codex {
    fn sweep_writers(&self, locks: &Path, _procs: &Procs) -> Sweep {
        let mut sweep = Sweep::default();
        // Surface access failures instead of silently showing an idle machine.
        if std::fs::read_dir(locks).is_err() {
            sweep.unmapped += 1;
            return sweep;
        }
        let mut rollout_ids = std::collections::BTreeSet::new();
        for rollout in self.rollouts() {
            let Some(id) = rollout
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(uuid_in_name)
            else {
                continue;
            };
            rollout_ids.insert(id.to_owned());
            match writer_owned(&locks.join(format!("{id}.lock"))) {
                Ok(false) => continue,
                Err(_) => {
                    sweep.unmapped += 1;
                    continue;
                }
                Ok(true) => {}
            }
            let Some(meta) = read_meta(&rollout.path).filter(|meta| meta.session_id == id) else {
                sweep.unmapped += 1;
                continue;
            };
            let (word, at_ms) =
                last_word(&rollout.path).unwrap_or((State::Idle, rollout.started_ms));
            if !meta.is_subagent {
                sweep.terminal_threads.push(meta.session_id.clone());
            }
            sweep.rows.push(Sighting {
                vendor_session_id: meta.session_id.clone(),
                title: title_of(meta.cwd.as_deref()),
                word,
                at_ms,
                pids: Vec::new(),
                cwd: meta.cwd,
            });
        }
        // A new TUI thread can own its writer before its first rollout is
        // materialized. Do not invent a row yet; native selection must name it.
        if let Ok(entries) = std::fs::read_dir(locks) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(id) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .filter(|id| id.len() == 36 && crate::codex_terminal::metadata_title(id))
                else {
                    continue;
                };
                if path.extension().and_then(|s| s.to_str()) != Some("lock")
                    || rollout_ids.contains(id)
                {
                    continue;
                }
                match writer_owned(&path) {
                    Ok(true) => sweep.pending_threads.push(id.to_owned()),
                    Ok(false) => {}
                    Err(_) => sweep.unmapped += 1,
                }
            }
        }
        // A writer proves thread liveness, never which terminal selected it.
        // A native title may bind it later in codex_terminal::select.
        sweep
    }
}

/// Probe an existing lock without waiting or changing its contents. Shared
/// probes do not conflict with other observers; Codex holds an exclusive lock.
fn writer_owned(path: &Path) -> std::io::Result<bool> {
    if !path.try_exists()? {
        return Ok(false);
    }
    // Codex checks its writer lock while holding this coordination lock.
    // Join that protocol so our short shared probe cannot make a concurrent
    // resume/create fail with "thread already has an active rollout writer".
    let coordination = std::fs::File::open(path.with_file_name(".coordination.lock"))?;
    match coordination.try_lock_shared() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Err(std::io::ErrorKind::WouldBlock.into()),
        Err(std::fs::TryLockError::Error(e)) => return Err(e),
    }
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    match file.try_lock_shared() {
        Ok(()) => Ok(false), // Closing the handle immediately releases our probe.
        Err(std::fs::TryLockError::WouldBlock) => Ok(true),
        Err(std::fs::TryLockError::Error(e)) => Err(e),
    }
}

/// The first line of a rollout: what session this is.
struct Meta {
    session_id: String,
    cwd: Option<String>,
    is_subagent: bool,
}

fn read_meta(path: &Path) -> Option<Meta> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).ok()?;
    let mut first = String::new();
    std::io::BufReader::new(file).read_line(&mut first).ok()?;
    let v: serde_json::Value = serde_json::from_str(&first).ok()?;
    // The older format's first line has no envelope at all; refusing by
    // type here is what keeps those files from being half-read.
    if v["type"].as_str() != Some("session_meta") {
        return None;
    }
    let p = &v["payload"];
    Some(Meta {
        session_id: p["id"]
            .as_str()
            .or_else(|| p["session_id"].as_str())?
            .to_owned(),
        cwd: p["cwd"].as_str().map(str::to_owned),
        // originator is inherited by spawned threads. The source records
        // that role explicitly, independently of the shared session_id.
        is_subagent: p["source"].get("subagent").is_some(),
    })
}

// Discovery is called by both the list and background recording. Reading the
// entire rollout into a String and materializing every JSON value on each call
// made a 666 MiB rollout exhaust application memory (2026-10-22). Only these
// small fields are needed; serde skips tool results, images and context without
// constructing their strings or object trees.
#[derive(serde::Deserialize, Default)]
struct StateRecord {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    payload: StatePayload,
}

#[derive(serde::Deserialize, Default)]
struct StatePayload {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    error: Option<serde::de::IgnoredAny>,
}

/// A streaming view of exactly one JSONL record, including its terminator.
/// Even a multi-megabyte line never becomes a multi-megabyte allocation.
struct StateLine<'a, R> {
    reader: &'a mut R,
    bytes: u64,
    complete: bool,
    chunk_left: usize,
}

impl<R: std::io::BufRead> std::io::Read for StateLine<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.complete || out.is_empty() {
            return Ok(0);
        }
        let buf = self.reader.fill_buf()?;
        if self.chunk_left == 0 {
            self.chunk_left = buf
                .iter()
                .position(|b| *b == b'\n')
                .map_or(buf.len(), |i| i + 1);
        }
        let n = self.chunk_left.min(out.len());
        out[..n].copy_from_slice(&buf[..n]);
        self.complete = n > 0 && buf[n - 1] == b'\n';
        self.reader.consume(n);
        self.chunk_left -= n;
        self.bytes += n as u64;
        Ok(n)
    }
}

struct StateCursor {
    metadata: std::fs::Metadata,
    upto: u64,
    word: Option<(State, Option<i64>)>,
    answer: Option<(State, Option<i64>)>,
    used: std::time::Instant,
    #[cfg(test)]
    scanned: u64,
}

fn same_rollout(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (a.dev(), a.ino()) == (b.dev(), b.ino())
    }
    #[cfg(not(unix))]
    {
        a.created().ok() == b.created().ok()
    }
}

#[derive(Default)]
struct StateCache(std::collections::HashMap<PathBuf, StateCursor>);

impl StateCache {
    fn read(&mut self, path: &Path) -> Option<(State, i64)> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).ok()?;
        let metadata = file.metadata().ok()?;
        let previous = self
            .0
            .get_mut(path)
            .filter(|c| same_rollout(&c.metadata, &metadata));
        if let Some(c) = previous.as_ref()
            && c.metadata.len() == metadata.len()
            && c.metadata.modified().ok() == metadata.modified().ok()
        {
            let c = self.0.get_mut(path)?;
            c.used = std::time::Instant::now();
            #[cfg(test)]
            {
                c.scanned = 0;
            }
            return c
                .answer
                .and_then(|(w, at)| Some((w, at.or_else(|| mtime_ms(path))?)));
        }
        // A shorter file, a replacement, or an in-place rewrite of equal
        // length starts over. A growing file resumes at the last newline.
        let (from, mut word) = previous
            .filter(|c| metadata.len() > c.metadata.len())
            .map_or((0, None), |c| (c.upto, c.word));
        file.seek(SeekFrom::Start(from)).ok()?;
        let mut reader = std::io::BufReader::new(file.take(metadata.len() - from));
        let mut upto = from;
        let mut provisional = None;
        #[cfg(test)]
        let mut scanned = 0;
        loop {
            let mut line = StateLine {
                reader: &mut reader,
                bytes: 0,
                complete: false,
                chunk_left: 0,
            };
            let record = serde_json::from_reader::<_, StateRecord>(&mut line).ok();
            // Malformed records must not prevent later valid state events.
            std::io::copy(&mut line, &mut std::io::sink()).ok()?;
            #[cfg(test)]
            {
                scanned += line.bytes;
            }
            if line.bytes == 0 {
                break;
            }
            if let Some(r) = record.filter(|r| r.kind == "event_msg")
                && let Some(mut w) = word_of(&r.payload.kind)
            {
                if matches!(
                    r.payload.kind.as_str(),
                    "turn_aborted" | "task_complete" | "turn_complete"
                ) && r.payload.error.is_some()
                {
                    w = State::Errored;
                }
                let at = r
                    .timestamp
                    .and_then(|t| t.parse::<jiff::Timestamp>().ok())
                    .map(|t| t.as_millisecond());
                if line.complete {
                    word = Some((w, at));
                } else {
                    provisional = Some((w, at));
                }
            }
            if !line.complete {
                break;
            }
            upto += line.bytes;
        }
        let answer = provisional.or(word);
        // One process normally sees a handful of active threads. Old paths
        // must not turn a long-running registry into another unbounded cache.
        if self.0.len() >= 256
            && !self.0.contains_key(path)
            && let Some(old) = self
                .0
                .iter()
                .min_by_key(|(_, c)| c.used)
                .map(|(p, _)| p.clone())
        {
            self.0.remove(&old);
        }
        self.0.insert(
            path.to_owned(),
            StateCursor {
                metadata,
                upto,
                word,
                answer,
                used: std::time::Instant::now(),
                #[cfg(test)]
                scanned,
            },
        );
        answer.and_then(|(w, at)| Some((w, at.or_else(|| mtime_ms(path))?)))
    }
}

/// Serialize same-file refreshes across concurrent list/background callers.
fn last_word(path: &Path) -> Option<(State, i64)> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<StateCache>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .read(path)
}

fn mtime_ms(path: &Path) -> Option<i64> {
    let m = std::fs::metadata(path).ok()?.modified().ok()?;
    let d = m.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(d.as_millis() as i64)
}

/// A codex session's title is its working directory's name — the
/// transcript carries no session name of its own the way claude's status
/// file does.
fn title_of(cwd: Option<&str>) -> String {
    cwd.and_then(|c| Path::new(c).file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| PROCESS_NAME.to_owned())
}

/// Walks the `<year>/<month>/<day>` tree. Depth-capped: the layout is
/// three levels and a symlink loop under someone's home must not hang a
/// session list.
fn collect(dir: &Path, out: &mut Vec<Rollout>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            collect(&path, out, depth + 1);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if let Some(ms) = uuid_in_name(name).and_then(uuid_v7_ms) {
            out.push(Rollout {
                path,
                started_ms: ms,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/codex/.codex")
    }

    fn codex_proc(started_ms: i64, cwd: Option<&str>) -> Proc {
        Proc {
            name: PROCESS_NAME.into(),
            started_ms,
            cwd: cwd.map(PathBuf::from),
            ppid: None,
        }
    }

    /// A v4 uuid's leading bits are random; decoding them as a timestamp
    /// lands in the year 5657, which is exactly the kind of number that
    /// looks like data until someone joins on it.
    #[test]
    fn only_a_v7_uuid_carries_a_timestamp() {
        assert_eq!(
            uuid_v7_ms("019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa"),
            Some(1_786_024_158_308)
        );
        assert_eq!(
            uuid_v7_ms("aaaaaaaa-bbbb-40cc-8ddd-eeeeeeeeeeee"),
            None,
            "v4"
        );
        assert_eq!(uuid_v7_ms("not-a-uuid"), None);
        assert_eq!(
            uuid_in_name("rollout-2026-08-06T21-49-18-019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa.jsonl"),
            Some("019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa")
        );
        assert_eq!(uuid_in_name("session_index.jsonl"), None);
    }

    /// The graveyard control group. The fixture tree holds several
    /// historical rollouts and one that a live process is writing; only
    /// the live one may become a row.
    #[test]
    fn history_is_not_a_row_and_the_live_one_is() {
        let codex = Codex::at(fixture_root());
        assert!(
            codex.rollouts().len() >= 4,
            "the finder sees the whole tree — otherwise the emptiness below proves nothing"
        );

        let empty = codex.sweep(&Procs::default());
        assert_eq!(empty.rows.len(), 0, "no codex running, no rows");
        assert_eq!(empty.unmapped, 0);

        // The live one: started when the alpha fixture rollout says.
        let procs = Procs::of([(700, codex_proc(1_786_024_158_000, Some("/w/alpha")))]);
        let sweep = codex.sweep(&procs);
        assert_eq!(sweep.rows.len(), 1, "one live process, one row");
        assert_eq!(sweep.rows[0].title, "alpha");
        assert_eq!(sweep.rows[0].word, State::Busy);
    }

    /// Every turn event the fixture tree spells, and the narrowing that
    /// matters: codex has no approval event, so 待批 is unreachable here
    /// no matter what the transcript says.
    #[test]
    fn the_turn_events_map_and_blocked_is_not_among_them() {
        assert_eq!(word_of("task_started"), Some(State::Busy));
        assert_eq!(word_of("task_complete"), Some(State::Done));
        assert_eq!(word_of("turn_aborted"), Some(State::Idle));
        assert_eq!(word_of("error"), Some(State::Errored));
        assert_eq!(word_of("agent_message"), None);
        assert_eq!(word_of("token_count"), None);
        for event in ["task_started", "task_complete", "turn_aborted", "error"] {
            assert_ne!(
                word_of(event),
                Some(State::Blocked),
                "codex has no approval event; {event} must not stand in for one"
            );
        }
    }

    /// A running codex whose transcript khor cannot read is counted, not
    /// guessed at — and the readable one beside it still appears, which
    /// is what proves the sweep was working when it declined.
    /// Three live processes, three answers (`Join`), and only one of
    /// them is a count. This test used to call the middle one
    /// "unreadable" — a process with no rollout anywhere near it — and
    /// that conflation is what put VS Code's codex host on the author's
    /// screen as "a session khor cannot read" for a day (2026-09-01).
    #[test]
    fn an_unreadable_transcript_is_counted_and_a_host_is_not() {
        let procs = Procs::of([
            // Joins the alpha rollout: a row.
            (700, codex_proc(1_786_024_158_000, Some("/w/alpha"))),
            // Started long before any rollout in the tree: a process that
            // began no session. Not a row, not a count.
            (701, codex_proc(1_000_000_000_000, None)),
            // Started a second before the 2024-01-01 fixture rollout,
            // whose first line is not a `session_meta` envelope — the
            // old format. A rollout in the window that will not open is
            // the count's whole meaning.
            (702, codex_proc(1_704_067_200_000 - 1_000, None)),
        ]);
        let sweep = Codex::at(fixture_root()).sweep(&procs);
        assert_eq!(sweep.rows.len(), 1, "{:?}", sweep.rows);
        assert_eq!(sweep.unmapped, 1, "the old-format one, and only it");
    }

    /// A node not pointed at any codex home says nothing about the
    /// codex running on the machine — it must not report the vendor as
    /// unreadable, which is what "khor is out of date" means.
    ///
    /// Caught by the control group rather than by reasoning: an isolated
    /// instance listed one line of "1 running session khor cannot read"
    /// while the only thing wrong was that it had been pointed
    /// elsewhere.
    #[test]
    fn a_node_looking_somewhere_else_reports_no_vendor_trouble() {
        let nowhere = Codex::at(PathBuf::from("/nonexistent/khor-test/.codex"));
        let sweep = nowhere.sweep(&Procs::of([(700, codex_proc(1_786_024_158_000, None))]));
        assert_eq!(sweep.rows.len(), 0);
        assert_eq!(
            sweep.unmapped, 0,
            "not looking is not the same as not understanding"
        );
    }

    /// The working directory decides when the clock picks wrong. This
    /// process started 500ms from the gamma rollout and 2s from the beta
    /// one, so the nearest-in-time candidate is gamma — and the cwd is
    /// the only thing that stops khor filing this session under the
    /// wrong transcript.
    #[test]
    fn the_working_directory_overrules_the_nearer_clock() {
        let codex = Codex::at(fixture_root());
        let start = 1_786_024_127_000;
        let beta = codex.sweep(&Procs::of([(700, codex_proc(start, Some("/w/beta")))]));
        assert_eq!(beta.rows.len(), 1);
        assert_eq!(
            beta.rows[0].title, "beta",
            "not gamma, which the clock preferred"
        );
        assert_eq!(
            beta.rows[0].word,
            State::Done,
            "the beta transcript ends on task_complete"
        );

        // Same instant, different directory: the clock's favourite wins
        // when nothing contradicts it. Without this the assertion above
        // would also pass if cwd matching simply rejected everything.
        let gamma = codex.sweep(&Procs::of([(700, codex_proc(start, Some("/w/gamma")))]));
        assert_eq!(gamma.rows.len(), 1);
        assert_eq!(gamma.rows[0].title, "gamma");
        assert_eq!(gamma.rows[0].word, State::Busy);
    }

    #[test]
    fn a_lossy_leaf_resolves_to_its_rollout_and_the_past_reads_back() {
        // The takeover fork's premise (`Node::takeover`): whichever
        // store resolves a leaf owns the session — so resolution must
        // answer for its own uuids and stay silent for strangers'.
        let home = std::env::temp_dir().join(format!("khor-codex-adaptor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let day = home.join("sessions/2026/08/20");
        std::fs::create_dir_all(&day).unwrap();
        let full = "01a01e00-0000-7000-8000-0000000000aa";
        std::fs::write(
            day.join(format!("rollout-2026-08-20T00-00-00-{full}.jsonl")),
            concat!(
                r#"{"type":"session_meta","payload":{"session_id":"x","cwd":"/somewhere/deep"}}"#,
                "\n",
                r#"{"type":"event_msg","payload":{"type":"user_message","message":"hello from the file"}}"#,
                "\n",
                r#"{"type":"response_item","payload":{"type":"custom_tool_call","name":"shell"}}"#,
                "\n",
                r#"{"type":"event_msg","payload":{"type":"agent_message","message":"answered"}}"#,
                "\n",
            ),
        )
        .unwrap();
        let codex = Codex::at(home.clone());
        let leaf = crate::live::clean_leaf(full);
        assert_eq!(codex.full_session_id(&leaf).as_deref(), Some(full));
        assert!(
            codex.full_session_id("0000000000000000000000ff").is_none(),
            "a stranger's leaf resolves nowhere — that silence IS the vendor verdict"
        );
        assert_eq!(
            codex.recorded_cwd(full),
            Some(std::path::PathBuf::from("/somewhere/deep"))
        );
        let past = codex.transcript(full).unwrap();
        use crate::adaptor::claude::Utterance;
        assert!(
            matches!(&past[0], Utterance::User(t) if t == "hello from the file"),
            "{past:?}"
        );
        // Shaped as claude's `tool_use` block, because both vendors'
        // pasts go through one translation now (`cagent::replayed`) —
        // two of them is how a replayed tool line went on saying
        // `Bash` for as long as the live one had been improved.
        assert!(
            matches!(&past[1], Utterance::Tool(b) if b["name"] == "shell" && b["type"] == "tool_use"),
            "{past:?}"
        );
        assert!(
            matches!(&past[2], Utterance::Agent(t) if t == "answered"),
            "{past:?}"
        );
        assert_eq!(
            past.len(),
            3,
            "codex's bookkeeping lines must not leak into the past"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[cfg(test)]
mod current {
    use super::*;
    use crate::adaptor::claude::Utterance;
    use std::fs::{File, OpenOptions};
    use std::io::Write;

    const THREAD: &str = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
    const OTHER: &str = "019fd755-e465-7aaa-8aaa-bbbbbbbbbbbb";

    struct Home(PathBuf);

    impl Home {
        fn new(label: &str) -> Self {
            let base =
                std::env::temp_dir().join(format!("khor-codex-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            let path = base.join(".codex");
            std::fs::create_dir_all(path.join("sessions/2026/08/06")).unwrap();
            Self(path)
        }

        fn rollout(&self, id: &str, suffix: &str, events: &str) -> PathBuf {
            let path = self
                .0
                .join("sessions/2026/08/06")
                .join(format!("rollout-2026-08-06T21-49-18-{id}{suffix}.jsonl"));
            let meta = serde_json::json!({"type":"session_meta","payload":{
                "id": id, "session_id": "shared-parent-session", "cwd":"/project",
                "history_mode":"paginated", "originator":"codex-tui"
            }});
            std::fs::write(&path, format!("{meta}\n{events}")).unwrap();
            path
        }

        fn writer(&self, id: &str) -> File {
            let dir = self.0.join("thread-writer-locks");
            std::fs::create_dir_all(&dir).unwrap();
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(dir.join(".coordination.lock"))
                .unwrap();
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(dir.join(format!("{id}.lock")))
                .unwrap();
            file.try_lock().unwrap();
            file
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    #[test]
    fn an_owned_writer_without_rollout_is_only_a_pending_selection_candidate() {
        let home = Home::new("pending-native-selection");
        let _writer = home.writer(THREAD);
        let codex = Codex::at(home.0.clone());
        let found = codex.sweep(&Procs::of([]));
        assert!(found.rows.is_empty());
        assert_eq!(found.pending_threads, vec![THREAD.to_owned()]);
        let bad = home.rollout(THREAD, "", "");
        std::fs::write(&bad, "not valid metadata\n").unwrap();
        let found = codex.sweep(&Procs::of([]));
        assert!(found.rows.is_empty());
        assert!(
            found.pending_threads.is_empty(),
            "an unreadable existing rollout is not a new idle conversation"
        );
        assert_eq!(found.unmapped, 1);
    }

    #[test]
    fn a_fresh_tui_writer_cannot_prove_its_terminal_client_pid() {
        let home = Home::new("tui-client");
        home.rollout(THREAD, "", &(item("AgentMessage", "latest answer") + "\n"));
        let _writer = home.writer(THREAD);
        // A shared daemon owns the writer, but the interactive client is
        // inside khor's terminal. Bootstrap took 66 seconds on the real TUI.
        for name in ["codex", "codex.exe", "CODEX.EXE"] {
            let procs = Procs::of([(
                700,
                Proc {
                    name: name.into(),
                    started_ms: uuid_v7_ms(THREAD).unwrap() - 66_000,
                    cwd: Some("/project".into()),
                    ppid: Some(500),
                },
            )])
            .with_commands([(700, vec![name.into()])]);
            let found = Codex::at(home.0.clone()).sweep(&procs);
            assert_eq!(found.rows.len(), 1);
            assert!(
                found.rows[0].pids.is_empty(),
                "birth time and cwd cannot identify a selected thread: {name}"
            );
        }
    }

    #[test]
    fn subagents_do_not_take_away_the_parent_terminal_identity() {
        let home = Home::new("tui-subagents");
        let parent = home.rollout(THREAD, "", &(item("AgentMessage", "parent answer") + "\n"));
        let child = home.rollout(OTHER, "", &(item("AgentMessage", "child answer") + "\n"));
        // Real daemon-backed sessions can say vscode while retaining their
        // TUI originator. Children inherit that originator and cwd too.
        for (path, source) in [
            (parent, serde_json::json!("vscode")),
            (
                child,
                serde_json::json!({"subagent":{"thread_spawn":{
                    "parent_thread_id": THREAD, "depth": 1, "agent_path": "/root/review"
                }}}),
            ),
        ] {
            let body = std::fs::read_to_string(&path).unwrap();
            let (first, rest) = body.split_once('\n').unwrap();
            let mut meta: serde_json::Value = serde_json::from_str(first).unwrap();
            meta["payload"]["source"] = source;
            std::fs::write(path, format!("{meta}\n{rest}")).unwrap();
        }
        let parent_writer = home.writer(THREAD);
        let _child_writer = home.writer(OTHER);
        let codex = Codex::at(home.0.clone());
        for name in ["codex", "codex.exe", "CODEX.EXE"] {
            let procs = Procs::of([(
                700,
                Proc {
                    name: name.into(),
                    started_ms: uuid_v7_ms(THREAD).unwrap() - 66_000,
                    cwd: Some("/project".into()),
                    ppid: Some(500),
                },
            )])
            .with_commands([(700, vec![name.into()])]);
            let rows = codex.sweep(&procs).rows;
            assert_eq!(rows.len(), 2, "subagents remain discoverable");
            assert!(
                rows.iter()
                    .find(|r| r.vendor_session_id == THREAD)
                    .unwrap()
                    .pids
                    .is_empty()
            );
            assert!(
                rows.iter()
                    .find(|r| r.vendor_session_id == OTHER)
                    .unwrap()
                    .pids
                    .is_empty()
            );
            assert_eq!(
                codex.last_said(&crate::live::clean_leaf(THREAD)).as_deref(),
                Some("parent answer")
            );
        }
        drop(parent_writer);
        let procs = Procs::of([(
            700,
            Proc {
                name: "codex".into(),
                started_ms: uuid_v7_ms(OTHER).unwrap() - 66_000,
                cwd: Some("/project".into()),
                ppid: Some(500),
            },
        )])
        .with_commands([(700, vec!["codex".into()])]);
        let rows = codex.sweep(&procs).rows;
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].pids.is_empty(),
            "a lone child is still not a terminal frontend"
        );
    }

    #[test]
    fn legacy_process_join_does_not_claim_a_subagent_rollout() {
        let home = Home::new("legacy-subagent");
        let path = home.rollout(THREAD, "", "");
        let body = std::fs::read_to_string(&path).unwrap();
        let mut meta: serde_json::Value = serde_json::from_str(body.trim()).unwrap();
        meta["payload"]["source"] = serde_json::json!({"subagent":"review"});
        std::fs::write(path, format!("{meta}\n")).unwrap();
        let procs = Procs::of([(
            700,
            Proc {
                name: "codex".into(),
                started_ms: uuid_v7_ms(THREAD).unwrap(),
                cwd: Some("/project".into()),
                ppid: None,
            },
        )]);
        let found = Codex::at(home.0.clone()).sweep(&procs);
        assert!(found.rows.is_empty());
        assert_eq!(
            found.unmapped, 0,
            "a known child is not an unreadable terminal"
        );
    }

    #[test]
    fn a_writer_is_not_assigned_to_an_ambiguous_or_noninteractive_client() {
        let home = Home::new("tui-client-refusals");
        let path = home.rollout(THREAD, "", "");
        let _writer = home.writer(THREAD);
        let proc = Proc {
            name: "codex".into(),
            started_ms: uuid_v7_ms(THREAD).unwrap() - 66_000,
            cwd: Some("/project".into()),
            ppid: None,
        };
        let base = Procs::of([(700, proc.clone())]).with_commands([(700, vec!["codex".into()])]);
        let codex = Codex::at(home.0.clone());
        for argv in [
            vec!["codex", "app-server"],
            vec!["codex", "exec"],
            vec!["codex", "resume", THREAD],
        ] {
            let table = base
                .clone()
                .with_commands([(700, argv.into_iter().map(str::to_owned).collect())]);
            assert!(codex.sweep(&table).rows[0].pids.is_empty());
        }
        assert!(
            codex.sweep(&Procs::of([(700, proc.clone())])).rows[0]
                .pids
                .is_empty(),
            "unknown command line"
        );
        for changed in [
            Proc {
                cwd: None,
                ..proc.clone()
            },
            Proc {
                cwd: Some("/other".into()),
                ..proc.clone()
            },
            Proc {
                started_ms: uuid_v7_ms(THREAD).unwrap() + 60_000,
                ..proc.clone()
            },
        ] {
            let table = Procs::of([(700, changed)]).with_commands([(700, vec!["codex".into()])]);
            assert!(codex.sweep(&table).rows[0].pids.is_empty());
        }
        let two = Procs::of([(700, proc.clone()), (701, proc)])
            .with_commands([(700, vec!["codex".into()]), (701, vec!["codex".into()])]);
        assert!(
            codex.sweep(&two).rows[0].pids.is_empty(),
            "two clients in one directory"
        );
        // A daemon can have several TUI-originated threads loaded in one cwd.
        home.rollout(OTHER, "", "");
        let other_writer = home.writer(OTHER);
        assert!(
            codex.sweep(&base).rows.iter().all(|r| r.pids.is_empty()),
            "two loaded threads in one directory"
        );
        drop(other_writer);
        let body = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, body.replace("codex-tui", "Codex Desktop")).unwrap();
        assert!(
            codex.sweep(&base).rows[0].pids.is_empty(),
            "a desktop thread is not the terminal's"
        );
    }

    #[test]
    fn a_writer_backed_row_updates_its_preview_state_and_unread_without_a_pid() {
        use crate::adaptor::Discovery;
        use crate::live::LiveKind;
        use khor_core::DeviceId;
        use std::sync::Arc;

        let home = Home::new("live-row");
        let path = home.rollout(THREAD, "", &(item("UserMessage", "question") + "\n"
            + "{\"timestamp\":\"2026-08-06T13:49:19Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n"));
        let file = OpenOptions::new().write(true).open(&path).unwrap();
        file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
            .unwrap();
        drop(file);
        let writer = home.writer(THREAD);
        let base = home.0.parent().unwrap();
        let kind = LiveKind::new(base.to_path_buf(), DeviceId([9; 32]))
            .discovering(Arc::new(Discovery::at(base).with_procs(Procs::default())));
        let first = kind.rows(|_| 0);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].state.unwrap().state, State::Busy);

        let mut f = OpenOptions::new().append(true).open(path).unwrap();
        writeln!(f, "{}", item("AgentMessage", "latest answer")).unwrap();
        writeln!(
            f,
            "{}",
            serde_json::json!({"timestamp":"2026-08-06T13:49:20Z",
            "type":"event_msg","payload":{"type":"task_complete"}})
        )
        .unwrap();
        drop(f);
        let next = kind.rows(|_| 0);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].last.as_deref(), Some("latest answer"));
        assert_eq!(
            (next[0].state.unwrap().state, next[0].unread),
            (State::Done, 1)
        );
        let seen_at = "2026-08-06T13:49:20Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .as_millisecond();
        let seen = kind.rows(|_| seen_at);
        assert_eq!(
            (seen[0].state.unwrap().state, seen[0].unread),
            (State::Idle, 0)
        );
        drop(writer);
        assert!(kind.rows(|_| 0).is_empty());
    }

    /// Manual, read-only check against a running Codex home. Does not print messages.
    #[test]
    #[ignore = "set KHOR_CODEX_PROBE_HOME to a real Codex home with loaded threads"]
    fn installed_codex_probe() {
        let root = std::env::var_os("KHOR_CODEX_PROBE_HOME").expect("explicit probe home required");
        let codex = Codex::at(root.into());
        let start = std::time::Instant::now();
        let found = codex.sweep(&Procs::snapshot());
        assert_eq!(found.unmapped, 0);
        assert!(
            !found.rows.is_empty(),
            "start a Codex thread before running the probe"
        );
        let busy = found.rows.iter().filter(|r| r.word == State::Busy).count();
        if let Ok(pid) = std::env::var("KHOR_CODEX_PROBE_PID") {
            let pid: u32 = pid.parse().unwrap();
            let thread =
                std::env::var("KHOR_CODEX_PROBE_THREAD").expect("explicit expected thread");
            assert!(
                found
                    .rows
                    .iter()
                    .any(|r| r.vendor_session_id == thread && r.pids.contains(&pid)),
                "the running TUI must be associated with its actual thread"
            );
        }
        let mut conversations = 0;
        for row in &found.rows {
            let said = codex.transcript(&row.vendor_session_id).unwrap();
            if said
                .iter()
                .any(|u| matches!(u, Utterance::User(_) | Utterance::Agent(_)))
            {
                conversations += 1;
            }
        }
        println!(
            "loaded_threads={} busy={} conversations_with_visible_messages={} elapsed_ms={}",
            found.rows.len(),
            busy,
            conversations,
            start.elapsed().as_millis()
        );
        assert!(conversations > 0);
    }

    #[test]
    fn daemon_threads_and_resumed_sessions_do_not_depend_on_a_process_clock() {
        let home = Home::new("writers");
        home.rollout(
            THREAD,
            "",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
        );
        home.rollout(
            OTHER,
            "_019fd755-e466-7aaa-8aaa-cccccccccccc",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n",
        );
        let first_writer = home.writer(THREAD);
        let second_writer = home.writer(OTHER);
        let codex = Codex::at(home.0.clone());
        // A daemon can predate both threads; its cwd and PID cannot identify them.
        let found = codex.sweep(&Procs::default());
        assert_eq!(found.unmapped, 0);
        assert_eq!(
            found.rows.len(),
            2,
            "both writer-owned threads must be discovered"
        );
        assert!(
            found
                .rows
                .iter()
                .any(|r| r.vendor_session_id == THREAD && r.word == State::Busy)
        );
        assert!(
            found
                .rows
                .iter()
                .any(|r| r.vendor_session_id == OTHER && r.word == State::Done)
        );
        assert!(
            found.rows.iter().all(|r| r.pids.is_empty()),
            "a lock does not identify a frontend PID"
        );
        assert_eq!(
            codex
                .full_session_id(&crate::live::clean_leaf(OTHER))
                .as_deref(),
            Some(OTHER)
        );

        // A released lock file may survive a crash. It must not keep history alive,
        // even if an unrelated process happens to match the old start-time heuristic.
        drop(first_writer);
        drop(second_writer);
        let procs = Procs::of([(
            7,
            Proc {
                name: "codex".into(),
                started_ms: uuid_v7_ms(THREAD).unwrap(),
                cwd: None,
                ppid: None,
            },
        )]);
        assert!(codex.sweep(&procs).rows.is_empty());
        assert!(
            home.0
                .join(format!("thread-writer-locks/{THREAD}.lock"))
                .exists(),
            "probing must not remove Codex's files"
        );
    }

    #[test]
    fn an_owned_but_unreadable_rollout_is_reported() {
        let home = Home::new("unreadable-writer");
        let path = home.rollout(THREAD, "", "");
        std::fs::write(path, "{\"type\":\"unknown_meta\"}\n").unwrap();
        let _writer = home.writer(THREAD);
        let found = Codex::at(home.0.clone()).sweep(&Procs::default());
        assert!(found.rows.is_empty());
        assert_eq!(found.unmapped, 1);
    }

    #[test]
    fn discovery_never_probes_writer_locks_during_a_codex_ownership_change() {
        let home = Home::new("coordination");
        home.rollout(THREAD, "", "");
        let _writer = home.writer(THREAD);
        let coordination = OpenOptions::new()
            .read(true)
            .write(true)
            .open(home.0.join("thread-writer-locks/.coordination.lock"))
            .unwrap();
        coordination.try_lock().unwrap();
        // Nonblocking: a concurrent resume owns the coordinator, so skip the
        // probe. Never compete for the per-thread lock it is about to acquire.
        assert_eq!(
            writer_owned(&home.0.join(format!("thread-writer-locks/{THREAD}.lock")))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
        drop(coordination);
        assert_eq!(
            Codex::at(home.0.clone())
                .sweep(&Procs::default())
                .rows
                .len(),
            1
        );
    }

    #[test]
    fn reverted_threads_have_one_row_and_replay_the_new_rollout() {
        let home = Home::new("reverted");
        let old = home.rollout(THREAD, "", &(item("AgentMessage", "old") + "\n"));
        // Make selection deterministic even on filesystems with coarse mtimes.
        let file = OpenOptions::new().write(true).open(&old).unwrap();
        file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
            .unwrap();
        drop(file);
        home.rollout(
            THREAD,
            "_019fd755-e466-7aaa-8aaa-cccccccccccc",
            &(item("AgentMessage", "reverted") + "\n"),
        );
        let _writer = home.writer(THREAD);
        let codex = Codex::at(home.0.clone());
        assert_eq!(codex.sweep(&Procs::default()).rows.len(), 1);
        assert!(matches!(&codex.transcript(THREAD).unwrap()[..],
            [Utterance::Agent(s)] if s == "reverted"));
    }

    fn item(kind: &str, text: &str) -> String {
        let tag = if kind == "UserMessage" {
            "text"
        } else {
            "Text"
        };
        serde_json::json!({"type":"event_msg","payload":{"type":"item_completed",
        "turn_id":"turn", "item":{"type":kind,"id":"item", "content":[
            {"type":tag,"text":text}, {"type":"image","image_url":"hidden"}
        ]}}})
        .to_string()
    }

    #[test]
    fn paginated_messages_follow_incrementally_and_tool_input_is_preserved() {
        let home = Home::new("paginated");
        let path = home.rollout(THREAD, "", &(item("UserMessage", "question") + "\n"));
        let codex = Codex::at(home.0.clone());
        let first = codex.transcript_since(THREAD, 0).unwrap();
        assert!(matches!(&first.said[..], [Utterance::User(s)] if s == "question"));
        let tool = serde_json::json!({"type":"response_item","payload":{
            "type":"custom_tool_call","name":"apply_patch","call_id":"call",
            "input":"*** Begin Patch\n*** End Patch"
        }});
        let mut f = OpenOptions::new().append(true).open(path).unwrap();
        writeln!(f, "{tool}\n{}", item("AgentMessage", "answer")).unwrap();
        // response_item messages duplicate visible events and can also contain
        // injected context; they must not become a second visible utterance.
        writeln!(f, "{}", serde_json::json!({"type":"response_item","payload":{
            "type":"message","role":"assistant","content":[{"type":"output_text","text":"answer"}]
        }})).unwrap();
        drop(f);
        let tail = codex.transcript_since(THREAD, first.at).unwrap();
        assert_eq!(tail.said.len(), 2);
        assert!(matches!(&tail.said[0], Utterance::Tool(v)
            if v["input"] == "*** Begin Patch\n*** End Patch"));
        assert!(matches!(&tail.said[1], Utterance::Agent(s) if s == "answer"));
        assert_eq!(codex.transcript(THREAD).unwrap().len(), 3);
        assert!(
            codex
                .transcript_since(THREAD, tail.at)
                .unwrap()
                .said
                .is_empty()
        );
    }

    #[test]
    fn state_time_is_the_event_time_and_not_later_bookkeeping() {
        let home = Home::new("event-time");
        let path = home.rollout(THREAD, "", concat!(
            "{\"timestamp\":\"2026-08-06T13:49:19.123Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n",
            "{\"timestamp\":\"2026-08-07T13:00:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"}}\n"));
        let expected = "2026-08-06T13:49:19.123Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .as_millisecond();
        assert_eq!(last_word(&path), Some((State::Done, expected)));
    }

    #[test]
    fn state_cache_only_reads_new_bytes_and_revisits_partial_records() {
        use std::io::Write;
        let home = Home::new("state-cursor");
        let path = home.rollout(
            THREAD,
            "",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
        );
        let mut cache = StateCache::default();
        assert_eq!(cache.read(&path).unwrap().0, State::Busy);
        assert!(cache.0[&path].scanned > 0);
        assert_eq!(cache.read(&path).unwrap().0, State::Busy);
        assert_eq!(
            cache.0[&path].scanned, 0,
            "unchanged files are not reparsed"
        );
        let before = cache.0[&path].upto;
        let tail = "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}";
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(f, "{tail}").unwrap();
        f.flush().unwrap();
        assert_eq!(cache.read(&path).unwrap().0, State::Busy);
        assert_eq!(cache.0[&path].upto, before);
        writeln!(f, "}}").unwrap();
        f.flush().unwrap();
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        assert_eq!(cache.0[&path].scanned, tail.len() as u64 + 2);
        // A complete JSON record without its final newline is still visible,
        // but its bytes remain uncommitted until the writer terminates it.
        write!(
            f,
            "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"task_started\"}}}}"
        )
        .unwrap();
        f.flush().unwrap();
        assert_eq!(cache.read(&path).unwrap().0, State::Busy);
        assert_eq!(cache.read(&path).unwrap().0, State::Busy);
    }

    #[test]
    fn state_scan_skips_large_irrelevant_content_and_recovers_after_bad_lines() {
        use std::io::Write;
        let home = Home::new("state-large");
        let path = home.rollout(THREAD, "", "");
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(
            f,
            "{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"text\":\""
        )
        .unwrap();
        let chunk = [b'x'; 8192];
        for _ in 0..2048 {
            f.write_all(&chunk).unwrap();
        }
        writeln!(f, "\"}}}}\nnot-json\n{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"task_complete\",\"error\":null}}}}").unwrap();
        f.flush().unwrap();
        let mut cache = StateCache::default();
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        assert_eq!(cache.0[&path].upto, f.metadata().unwrap().len());
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        assert_eq!(cache.0[&path].scanned, 0);
    }

    #[test]
    fn state_cache_restarts_on_rotation_and_same_length_rewrite() {
        let home = Home::new("state-replace");
        let path = home.rollout(
            THREAD,
            "",
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n",
        );
        let mut cache = StateCache::default();
        assert_eq!(cache.read(&path).unwrap().0, State::Done);
        std::fs::rename(&path, path.with_extension("old")).unwrap();
        std::fs::write(
            &path,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
        )
        .unwrap();
        assert_eq!(cache.read(&path).unwrap().0, State::Busy);
        let previous = cache.0[&path].metadata.modified().unwrap();
        std::fs::write(
            &path,
            "{\"type\":\"event_msg\",\"payload\":{\"type\":\"turn_aborted\"}}\n",
        )
        .unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(previous + std::time::Duration::from_secs(1))
            .unwrap();
        assert_eq!(cache.read(&path).unwrap().0, State::Idle);
    }

    #[test]
    fn turn_aliases_and_abort_errors_keep_their_distinct_states() {
        assert_eq!(word_of("turn_started"), Some(State::Busy));
        assert_eq!(word_of("turn_complete"), Some(State::Done));
        let home = Home::new("abort-error");
        for event in ["turn_aborted", "task_complete", "turn_complete"] {
            let line = serde_json::json!({"timestamp":"2026-08-06T13:49:20Z",
                "type":"event_msg","payload":{"type":event,"reason":"interrupted",
                    "error":{"message":"transport failed"}}});
            let path = home.rollout(THREAD, "", &(line.to_string() + "\n"));
            assert_eq!(last_word(&path).unwrap().0, State::Errored, "{event}");
        }
    }

    #[test]
    fn windows_executable_names_are_recognized_without_accepting_helpers() {
        let codex = Codex::at(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors/codex/.codex"),
        );
        for name in ["codex.exe", "CODEX.EXE"] {
            let found = codex.sweep(&Procs::of([(
                7,
                Proc {
                    name: name.into(),
                    started_ms: 1_786_024_158_000,
                    cwd: Some("/w/alpha".into()),
                    ppid: None,
                },
            )]));
            assert_eq!(found.rows.len(), 1, "{name}");
        }
    }
}

#[cfg(test)]
mod following {
    use super::*;
    use crate::adaptor::claude::Utterance;

    /// A rollout this test owns, so it can grow and be replaced.
    fn rollout(dir: &std::path::Path, uuid: &str, lines: &[&str]) -> PathBuf {
        let day = dir.join("sessions").join("2026").join("09").join("01");
        std::fs::create_dir_all(&day).unwrap();
        let p = day.join(format!("rollout-2026-09-01T00-00-00-{uuid}.jsonl"));
        std::fs::write(&p, lines.join("\n") + "\n").unwrap();
        p
    }

    fn said(m: &str) -> String {
        format!(r#"{{"type":"event_msg","payload":{{"type":"agent_message","message":"{m}"}}}}"#)
    }

    fn words(said: &[Utterance]) -> Vec<String> {
        said.iter()
            .filter_map(|u| match u {
                Utterance::Agent(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    /// **A codex conversation can be followed**, which until now it
    /// could not: `transcript_follow` answered `following: false` for
    /// one, so the pane read it once and then froze — a conversation
    /// still going, painted static, with nothing on screen saying it
    /// had stopped being read.
    ///
    /// The three things a file can do between two reads, each with the
    /// answer claude's reader gives (`claude::transcript_since`).
    #[test]
    fn a_rollout_that_grows_is_read_from_where_the_last_read_stopped() {
        let dir = std::env::temp_dir().join(format!("khor-codex-follow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let uuid = "019fd755-1111-7aaa-8aaa-aaaaaaaaaaaa";
        let path = rollout(&dir, uuid, &[&said("one")]);
        let codex = Codex::at(dir.clone());

        let first = codex.transcript_since(uuid, 0).unwrap();
        assert_eq!(words(&first.said), ["one"]);
        assert!(first.following, "a rollout is followable");
        assert!(!first.rewound);

        // **Grow**: only what is new comes back, and the whole point is
        // that it is only what is new — the control is that "one" is
        // *not* in the second answer.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        use std::io::Write;
        writeln!(f, "{}", said("two")).unwrap();
        drop(f);
        let second = codex.transcript_since(uuid, first.at).unwrap();
        assert_eq!(words(&second.said), ["two"], "the tail, not the file");
        assert!(second.at > first.at);

        // **Stay put**: nothing to say, and `at` does not move.
        let third = codex.transcript_since(uuid, second.at).unwrap();
        assert!(third.said.is_empty());
        assert_eq!(third.at, second.at);

        // **Shrink or be replaced**: byte 400 of a new file is not byte
        // 400 of the old one, so continuing would splice two
        // conversations. The whole file comes back, marked `rewound`.
        std::fs::write(&path, said("fresh") + "\n").unwrap();
        let fourth = codex.transcript_since(uuid, second.at).unwrap();
        assert!(fourth.rewound, "a shorter file must not be continued into");
        assert_eq!(words(&fourth.said), ["fresh"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **A half-written last line is left for next time.** The writer
    /// appends a whole JSON object per line but not atomically, so a
    /// read can land mid-object; parsing what is there and skipping the
    /// rest would drop that utterance for good.
    #[test]
    fn a_line_the_writer_has_not_finished_is_waited_for() {
        let dir = std::env::temp_dir().join(format!("khor-codex-partial-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let uuid = "019fd755-2222-7aaa-8aaa-aaaaaaaaaaaa";
        let path = rollout(&dir, uuid, &[&said("whole")]);
        let codex = Codex::at(dir.clone());
        let first = codex.transcript_since(uuid, 0).unwrap();
        assert_eq!(words(&first.said), ["whole"]);

        // Half an object, no newline yet.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        use std::io::Write;
        write!(f, "{}", &said("torn")[..20]).unwrap();
        drop(f);
        let mid = codex.transcript_since(uuid, first.at).unwrap();
        assert!(mid.said.is_empty(), "a fragment is not an utterance");
        assert_eq!(mid.at, first.at, "and the fragment is read again next time");

        // Finished: it arrives whole, exactly once.
        std::fs::write(&path, said("whole") + "\n" + &said("torn") + "\n").unwrap();
        let done = codex.transcript_since(uuid, mid.at).unwrap();
        assert_eq!(words(&done.said), ["torn"], "once the line is whole, once");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The whole read and the followed read are one reading of the
    /// file.** Two parsers over one format drift, and the drift shows
    /// up as a conversation that says different things depending on
    /// whether the pane happened to be open while it was written.
    ///
    /// That is why `transcript` and `transcript_since` both go through
    /// `utterances_of` and neither has a loop of its own — this asserts
    /// the property that arrangement exists for.
    #[test]
    fn reading_it_whole_and_reading_it_in_two_halves_say_the_same_thing() {
        let dir = std::env::temp_dir().join(format!("khor-codex-same-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let uuid = "019fd755-3333-7aaa-8aaa-aaaaaaaaaaaa";
        let path = rollout(&dir, uuid, &[&said("a"), &said("b"), &said("c")]);
        let codex = Codex::at(dir.clone());

        let whole = words(&codex.transcript(uuid).unwrap());
        assert_eq!(
            whole,
            ["a", "b", "c"],
            "the control: the record really has three lines"
        );

        // Split at the first line's end, so this is genuinely two reads
        // of one record rather than one read and an empty one.
        let cut = std::fs::read_to_string(&path).unwrap().find('\n').unwrap() as u64 + 1;
        let head = codex.transcript_since(uuid, 0).unwrap();
        let tail = codex.transcript_since(uuid, cut).unwrap();
        assert_eq!(
            words(&head.said),
            whole,
            "from zero, the follow reads it all"
        );
        assert_eq!(
            words(&tail.said),
            ["b", "c"],
            "and from a cut, only what is past it"
        );

        let mut halves = vec!["a".to_owned()];
        halves.extend(words(&tail.said));
        assert_eq!(halves, whole, "one format, one reading");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
