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
            live.report(&id, w)?;
            Ok(Hooked::Updated(id, w))
        }
        None => Ok(Hooked::Ignored),
    }
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
