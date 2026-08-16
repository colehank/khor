//! Codex: the second vendor, and the one that shows why the seam had to
//! stay narrow.
//!
//! Codex keeps no live status file. It writes an append-only transcript
//! per session at `~/.codex/sessions/<year>/<month>/<day>/rollout-<local
//! timestamp>-<uuid>.jsonl`, and nothing in it names a pid — measured:
//! zero of 83 rollout files on this machine mention one. So liveness
//! cannot come from the files at all, and this adaptor gets it the only
//! honest way, from the process table.
//!
//! That inversion is the whole argument for per-vendor adaptors. Claude
//! reads a file and asks the process table to confirm it; codex reads
//! the process table and asks a file to explain it.
//!
//! # Joining a process to its transcript
//!
//! Two facts make the join cheap and timezone-free:
//!
//! - the rollout's uuid is a **v7**, so its first 48 bits are the
//!   session's start in ms — readable from the file *name*, with no file
//!   opened and no local-time parsing (the timestamp in the name is
//!   local, the uuid is not);
//! - a rollout is created within a second of the process that writes it
//!   (measured: uuid says 21:49:18.308, the process started 21:49:18).
//!
//! So: nearest rollout inside a window around the process start, then
//! confirmed against the process's own working directory. Files older
//! than every live codex are never opened, which is what keeps 83
//! transcripts from costing anything.
//!
//! Rollouts whose uuid is a **v4** predate this scheme; their first bits
//! are random and would decode to a timestamp in the year 5657, so the
//! version nibble is checked rather than assumed. Those files also carry
//! the older line format (`{"record_type":"state"}` and bare items, no
//! `session_meta` envelope), which this adaptor does not read.
//!
//! # What codex can and cannot say
//!
//! 待批 is not reachable. Codex's transcript has no approval event of any
//! kind — measured across 4569 `event_msg` payloads in 83 files, the
//! vocabulary is agent/user messages, reasoning, token counts, and the
//! four turn events below. An approval prompt in codex leaves no trace
//! on disk, so this adaptor never spells 待批 rather than approximating
//! it from a turn that has been running a long time.
//!
//! 完成 *is* reachable here, unlike claude: `task_complete` says exactly
//! "this turn ended", where claude's `idle` conflates a finished turn
//! with a session that never started one.

use std::path::{Path, PathBuf};

use khor_core::State;

use super::{read_text, Adaptor, Proc, Procs, Sighting, Sweep};

/// The process name a codex session runs under. Exact — this machine
/// also runs `codex-code-mode-host`, which is a helper, not a session.
pub(super) const PROCESS_NAME: &str = "codex";

/// How far from a process's start its rollout may be stamped. Generous
/// forward (the file is created after the process, and a slow start
/// stretches that), barely negative (only clock granularity should ever
/// put the file first).
const JOIN_EARLIEST_MS: i64 = -5_000;
const JOIN_LATEST_MS: i64 = 60_000;

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
        "task_started" => Some(State::Busy),
        // Codex says this outright, so the row can hold a badge that
        // clears by being looked at.
        "task_complete" => Some(State::Done),
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
    (stem.len() >= 36).then(|| &stem[stem.len() - 36..])
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
        out
    }

    /// The rollout a process is writing: nearest start inside the
    /// window, confirmed by working directory when both are known.
    fn rollout_of<'a>(
        &self,
        proc: &Proc,
        rollouts: &'a [Rollout],
        taken: &[&'a Path],
    ) -> Option<(&'a Rollout, Meta)> {
        let mut candidates: Vec<&Rollout> = rollouts
            .iter()
            .filter(|r| !taken.contains(&r.path.as_path()))
            .filter(|r| {
                let d = r.started_ms - proc.started_ms;
                (JOIN_EARLIEST_MS..=JOIN_LATEST_MS).contains(&d)
            })
            .collect();
        candidates.sort_by_key(|r| (r.started_ms - proc.started_ms).abs());
        for r in candidates {
            let Some(meta) = read_meta(&r.path) else { continue };
            // The clock put them together; the directory confirms it.
            // Only when both sides know their cwd — sysinfo cannot
            // always read another process's, and an unknown must not
            // veto a match the clock already made.
            match (&proc.cwd, &meta.cwd) {
                (Some(a), Some(b)) if Path::new(b) != a.as_path() => continue,
                _ => return Some((r, meta)),
            }
        }
        None
    }
}

impl Adaptor for Codex {
    fn vendor(&self) -> &'static str {
        "codex"
    }

    fn sweep(&self, procs: &Procs) -> Sweep {
        let mut sweep = Sweep::default();
        // The process table is the whole machine's, while the root is
        // this node's. When khor is not pointed at a codex home at all —
        // a second instance, a test — the codex running over there is
        // none of its business, and counting it would report "khor
        // cannot read this vendor" when the truth is "khor is not
        // looking at it".
        if !self.root.join("sessions").is_dir() {
            return sweep;
        }
        let live = procs.named(PROCESS_NAME);
        if live.is_empty() {
            // Nothing running means nothing to show. The transcripts on
            // disk are history and history is not a row.
            return sweep;
        }
        let rollouts = self.rollouts();
        let mut taken: Vec<&Path> = Vec::new();
        for (_, proc) in live {
            let Some((rollout, meta)) = self.rollout_of(proc, &rollouts, &taken) else {
                // A codex is running and khor cannot say which session
                // it is: an old-format transcript, a layout change, or a
                // rollout outside the window. Counted, never guessed at.
                sweep.unmapped += 1;
                continue;
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
                vendor_session_id: meta.session_id,
                title: title_of(meta.cwd.as_deref()),
                word,
                at_ms,
            });
        }
        sweep
    }
}

/// The first line of a rollout: what session this is.
struct Meta {
    session_id: String,
    cwd: Option<String>,
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
        session_id: p["session_id"]
            .as_str()
            .or_else(|| p["id"].as_str())?
            .to_owned(),
        cwd: p["cwd"].as_str().map(str::to_owned),
    })
}

/// The last state-bearing event in the transcript, and when it happened.
///
/// The timestamp is the file's own mtime rather than the event's ISO
/// string: a rollout is appended to on every event, so its mtime *is*
/// the last event's instant (measured equal to the millisecond), and
/// taking it there costs no date parsing and no timezone assumption.
fn last_word(path: &Path) -> Option<(State, i64)> {
    let text = read_text(path)?;
    let mut word = None;
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v["type"].as_str() != Some("event_msg") {
            continue;
        }
        if let Some(w) = v["payload"]["type"].as_str().and_then(word_of) {
            word = Some(w);
        }
    }
    Some((word?, mtime_ms(path)?))
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
    let Ok(rd) = std::fs::read_dir(dir) else { return };
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
            out.push(Rollout { path, started_ms: ms });
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
        assert_eq!(uuid_v7_ms("aaaaaaaa-bbbb-40cc-8ddd-eeeeeeeeeeee"), None, "v4");
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
    #[test]
    fn an_unreadable_transcript_is_counted_and_does_not_stop_the_others() {
        let procs = Procs::of([
            (700, codex_proc(1_786_024_158_000, Some("/w/alpha"))),
            // Started long before any rollout in the tree: nothing joins.
            (701, codex_proc(1_000_000_000_000, None)),
        ]);
        let sweep = Codex::at(fixture_root()).sweep(&procs);
        assert_eq!(sweep.rows.len(), 1);
        assert_eq!(sweep.unmapped, 1);
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
        assert_eq!(sweep.unmapped, 0, "not looking is not the same as not understanding");
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
        assert_eq!(beta.rows[0].title, "beta", "not gamma, which the clock preferred");
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
}
