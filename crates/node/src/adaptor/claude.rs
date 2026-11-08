//! Claude Code discovery and state from native status files and transcripts.

use std::path::{Path, PathBuf};

use khor_catalog::msg;
use khor_core::State;

mod state;

use super::{Adaptor, CRASH_GRACE_MS, Procs, Sighting, Sweep, id_for, read_text};

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
        // The person is running a shell command of their own (`!`) in
        // the foreground. **The agent is idle**: nothing of claude's is
        // running and nothing is waiting on the person — the six words
        // describe the agent, and 忙碌 here would pulse for somebody's
        // own `ls`. Seen 2026-09-01 as one of the "6 sessions khor
        // cannot read" on the author's machine.
        "shell" => Some(State::Idle),
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

/// A file time as the epoch milliseconds every khor stamp is in.
fn ms_of(at: std::time::SystemTime) -> i64 {
    at.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// The working directory a transcript records: the first line that
/// carries one, read forward from the head.
///
/// Every conversation entry carries one, so the first is the answer;
/// the bound is there because these files reach hundreds of megabytes
/// and this is read once per directory when the wizard opens.
///
/// # Lines until one says, not a fixed head
///
/// **It used to read the first 64 KiB.** Claude Code now opens a
/// transcript with attachments that carry no `cwd` and run to tens of
/// KiB each — on the owner's, a `skill_listing` of 49 777 bytes and a
/// `deferred_tools_delta` of 17 814 — so the first entry with one can
/// start past any small head (SESS-BRIEF 2a). Lines are read until one
/// carries `cwd`, up to `MOST` bytes in all.
///
/// # Why the bytes are decoded lossily, and what it cost not to
///
/// A byte bound cuts wherever it lands, and a character does not have
/// to end there. `read_to_string` **fails the whole read** when it
/// does — not the last line, the whole thing — so a transcript with any
/// multi-byte text near the bound answered "no working directory
/// recorded" while holding one in plain sight on line 6. Each line is
/// decoded on its own now, lossily, for the line the bound cuts.
///
/// Measured on this machine 2026-09-01: the newest transcript of the
/// project khor itself is worked on cut mid-character at exactly 65534,
/// so **that** conversation was one of the ones this could not place.
/// And the answer did not come back empty where it was used — the
/// resume path reads this and falls back to the home directory, so a
/// 接管 of such a conversation quietly resumed the agent *somewhere
/// else*. Lossy decoding costs one possibly-mangled character at the
/// tail of a bounded read, on a line nothing parses.
///
/// # A hit is kept forever, a miss until the file changes
///
/// The list's post-pass asks this for every claude row that has no
/// place of its own, every poll. A transcript's `cwd` never changes
/// once written, so a found answer is a fact about the file and is kept
/// for the life of the process — the same reasoning `ai_title` gives
/// for its permanent cache. A **miss is kept against the file's length
/// and mtime**: an unchanged file is not read again, so the placeless
/// rows are not the ones read on every poll; but a transcript caught
/// before its first entry is written gets that entry, and a miss
/// cached forever filed the row under no directory for good.
fn cwd_in(path: &Path) -> Option<PathBuf> {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Read};
    use std::sync::{Mutex, OnceLock};
    /// Far past any opening run of attachments seen (about 68 KiB on
    /// the owner's), and still a bounded cost for a file that has none.
    const MOST: u64 = 4 * 1024 * 1024;
    type Stamp = (u64, std::time::SystemTime);
    static KNOWN: OnceLock<Mutex<HashMap<PathBuf, (Option<Stamp>, Option<PathBuf>)>>> =
        OnceLock::new();
    let known = KNOWN.get_or_init(|| Mutex::new(HashMap::new()));
    let stamp = std::fs::metadata(path)
        .ok()
        .and_then(|m| Some((m.len(), m.modified().ok()?)));
    if let Some((at, hit)) = known.lock().ok().and_then(|k| k.get(path).cloned()) {
        if hit.is_some() || at == stamp {
            return hit;
        }
    }
    let read = || -> Option<PathBuf> {
        let mut lines = BufReader::new(std::fs::File::open(path).ok()?.take(MOST));
        let mut line = Vec::new();
        loop {
            line.clear();
            if lines.read_until(b'\n', &mut line).ok()? == 0 {
                return None;
            }
            let text = String::from_utf8_lossy(&line);
            // Cheap reject before parsing: the attachments this reads
            // past are the large lines.
            if !text.contains("\"cwd\"") {
                continue;
            }
            let cwd = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("cwd")?.as_str().map(PathBuf::from));
            if cwd.is_some() {
                return cwd;
            }
        }
    };
    let found = read();
    if let Ok(mut k) = known.lock() {
        k.insert(path.to_path_buf(), (stamp, found.clone()));
    }
    found
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
        let (path, _) = self
            .transcript_path(leaf)
            .ok_or_else(|| msg::no_such_session(leaf))?;
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        Ok(utterances_of(&text))
    }

    /// A bounded raw window for paginated history. Fragments and oversized
    /// records remain explicit; this does not change the legacy follow cursor.
    pub fn transcript_window(
        &self,
        leaf: &str,
        before: Option<u64>,
    ) -> Result<super::transcript_window::TranscriptWindow, String> {
        let path = self
            .transcript_path(leaf)
            .map(|(path, _)| path)
            .ok_or_else(|| msg::no_such_session(leaf))?;
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        super::transcript_window::read_window(&mut file, before).map_err(|e| e.to_string())
    }

    /// What was appended to a transcript since byte `from`.
    ///
    /// **Only the new bytes are read.** A watched conversation's file
    /// grows all afternoon and the biggest one on this machine is
    /// 280 MB; re-reading it to find the last paragraph is the cost
    /// that made a discovered session read-once in the first place.
    /// Seeking past what has been seen turns that into a cost
    /// proportional to what was *said*, which is a few hundred bytes.
    ///
    /// # The three things a file can do between two reads
    ///
    /// - **Grow** — the ordinary case, and the whole point: the new
    ///   utterances come back and `at` moves to the new end.
    /// - **Stay put** — nothing to say. Empty, and `at` unchanged.
    /// - **Shrink, or be replaced** — a compaction, a rotation, a
    ///   different file behind the same leaf. Byte 400 of the new file
    ///   is not byte 400 of the old one, so continuing from `from`
    ///   would splice two conversations together. Answered with
    ///   `rewound: true` and the file whole, and the caller repaints
    ///   rather than appends.
    ///
    /// **A partial last line is left for next time.** The writer
    /// appends a whole JSON object per line but not atomically, so a
    /// read can land mid-object; `at` stops at the last newline, and
    /// the tail is read again when it is complete. Parsing what is
    /// there and skipping the rest would drop that utterance forever.
    pub fn transcript_since(&self, leaf: &str, from: u64) -> Result<Follow, String> {
        use std::io::{Read, Seek, SeekFrom};
        let (path, _) = self
            .transcript_path(leaf)
            .ok_or_else(|| msg::no_such_session(leaf))?;
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
        // Cut at the last newline: everything after it is a line the
        // writer has not finished.
        let end = match buf.iter().rposition(|b| *b == b'\n') {
            Some(i) => i + 1,
            // Not one whole line yet — wait rather than parse a
            // fragment.
            None => {
                return Ok(Follow {
                    said: Vec::new(),
                    at: start,
                    rewound,
                    following: true,
                });
            }
        };
        // Lossy on purpose: a byte range that begins mid-codepoint
        // cannot happen (the cut is at a newline both times), and a
        // genuinely corrupt line is one `utterances_of` already skips.
        let text = String::from_utf8_lossy(&buf[..end]);
        Ok(Follow {
            said: utterances_of(&text),
            at: start + end as u64,
            rewound,
            following: true,
        })
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
        cwd_in(&self.transcript_path(leaf)?.0)
    }

    /// **Every directory this vendor remembers working in**, for the
    /// wizard's 「常去的地方」 (`khor_node::place`). One entry per
    /// project directory: `(cwd, how many transcripts it holds, when
    /// the newest was written)`.
    ///
    /// # One file read per directory, not per transcript
    ///
    /// claude files transcripts under a directory named after the cwd,
    /// so the grouping is already done on disk — measured here on
    /// 2026-09-01, **929 transcripts in 51 directories**. Reading the
    /// newest transcript of each is 51 reads instead of 929, and the
    /// count comes from `read_dir` without opening anything.
    ///
    /// **The directory name is not read**, though it looks like the
    /// cheapest answer of all: claude's encoding replaces every slash
    /// with a dash *and leaves real dashes alone*, so
    /// `-Users-zgh-my-repo` has several possible originals and no way
    /// to tell which. The record inside says it outright.
    pub fn places(&self) -> Vec<(PathBuf, u32, i64)> {
        let mut out = Vec::new();
        let Ok(projects) = std::fs::read_dir(self.projects_dir()) else {
            return out;
        };
        for project in projects.flatten() {
            let Ok(entries) = std::fs::read_dir(project.path()) else {
                continue;
            };
            let mut held = 0u32;
            let mut newest: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                    continue;
                }
                held += 1;
                let at = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                newest.push((at, path));
            }
            newest.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
            let Some(&(at, _)) = newest.first() else {
                continue;
            };
            // **The newest transcript answers, and if it cannot, the
            // one behind it does.** They are all in one directory
            // because claude filed them under one working directory, so
            // any of them answers for all of them — and a transcript
            // that records no `cwd` at all (a session that died in its
            // first breath) would otherwise take its whole project down
            // with it. Three tries, because the fourth would be paying
            // for a directory that is telling us it has nothing.
            let Some(cwd) = newest.iter().take(3).find_map(|(_, p)| cwd_in(p)) else {
                continue;
            };
            out.push((cwd, held, ms_of(at)));
        }
        out
    }

    /// The name claude gave this conversation itself, if it has given
    /// one — what its own UI shows instead of `tokens-50`.
    ///
    /// # Why the head, and why 256 lines
    ///
    /// claude writes an `ai-title` record into the transcript, and
    /// writes it **over and over**: measured across this machine's
    /// transcripts on 2026-08-30, one 26 MB file carried 441 of them
    /// and **every one held the same string**. So the first is the
    /// answer and there is nothing later to prefer.
    ///
    /// The bound matters because these files get large — the biggest
    /// here is 280 MB, and a list poll touches one per row. Measured on
    /// that file, reading 128 lines costs **0.30 ms**. Across 93
    /// transcripts the first `ai-title` sat at line 1 at the earliest,
    /// 17 in the median, and **130 at the worst**; 256 is that worst
    /// case with room, and still under a millisecond.
    ///
    /// **What a session past the bound degrades to is the row it has
    /// today** — claude's own `tokens-50` — not a blank and not a
    /// guess. That is the whole reason a bound is allowed here.
    ///
    /// # The cache is stronger than `last_said`'s, for a measured reason
    ///
    /// A found title is cached **forever**, with no mtime check,
    /// because the value does not change within a file. A miss is
    /// re-checked whenever the file moves: 49 of 93 transcripts had no
    /// title at all, and a short session that has not earned one yet
    /// may earn one later.
    pub fn ai_title(&self, leaf: &str) -> Option<String> {
        use std::collections::HashMap;
        use std::io::{BufRead, BufReader};
        use std::sync::{Mutex, OnceLock};
        /// The worst first-occurrence seen was 130. See above.
        const MAX_LINES: usize = 256;
        type Cache = HashMap<PathBuf, (std::time::SystemTime, Option<String>)>;
        static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
        let (path, mtime) = self.transcript_path(leaf)?;
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some((t, title)) = cache.lock().unwrap().get(&path) {
            // A hit is final; a miss is only as good as the file it was
            // taken from.
            if title.is_some() || *t == mtime {
                return title.clone();
            }
        }
        let found = std::fs::File::open(&path).ok().and_then(|f| {
            BufReader::new(f)
                .lines()
                .take(MAX_LINES)
                .map_while(Result::ok)
                .find_map(|l| {
                    // Cheap reject before parsing: most of these lines
                    // are large assistant messages, and parsing 256 of
                    // them per row per poll is the cost this avoids.
                    if !l.contains("\"ai-title\"") {
                        return None;
                    }
                    let v: serde_json::Value = serde_json::from_str(&l).ok()?;
                    if v.get("type")?.as_str()? != "ai-title" {
                        return None;
                    }
                    let t = v.get("aiTitle")?.as_str()?.trim();
                    (!t.is_empty()).then(|| t.to_owned())
                })
        });
        cache.lock().unwrap().insert(path, (mtime, found.clone()));
        found
    }

    pub(crate) fn transcript_path(&self, leaf: &str) -> Option<(PathBuf, std::time::SystemTime)> {
        let mut hit: Option<(std::time::SystemTime, PathBuf)> = None;
        let projects = std::fs::read_dir(self.projects_dir()).ok()?;
        for proj in projects.flatten() {
            let Ok(rd) = std::fs::read_dir(proj.path()) else {
                continue;
            };
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
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

    /// A source binding must never choose between distinct native IDs sharing a leaf.
    pub(crate) fn unique_transcript_path(
        &self,
        leaf: &str,
        exact: Option<&str>,
    ) -> Result<Option<(PathBuf, String)>, ()> {
        let Ok(projects) = std::fs::read_dir(self.projects_dir()) else {
            return Ok(None);
        };
        let mut chosen: Option<(PathBuf, String, std::time::SystemTime)> = None;
        for project in projects.flatten() {
            let Ok(entries) = std::fs::read_dir(project.path()) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|part| part.to_str()) != Some("jsonl") {
                    continue;
                }
                let Some(native) = path.file_stem().and_then(|part| part.to_str()) else {
                    continue;
                };
                if crate::live::clean_leaf(native) != leaf || exact.is_some_and(|id| id != native) {
                    continue;
                }
                let at = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                if let Some((_, prior, _)) = &chosen {
                    if prior != native {
                        return Err(());
                    }
                }
                if chosen
                    .as_ref()
                    .is_none_or(|(_, _, previous)| at > *previous)
                {
                    chosen = Some((path.clone(), native.to_owned(), at));
                }
            }
        }
        Ok(chosen.map(|(path, native, _)| (path, native)))
    }
    /// The newest utterance in a leaf's transcript, as one preview line —
    /// what the row's 最近一条消息 shows for a claude-backed session.
    ///
    /// A list polls this for every claude row every few seconds, so the
    /// answer is cached by (path, mtime): an unchanged transcript costs
    /// directory stats, not a read. On change only what was appended
    /// since is read ([`newest_said`]).
    pub fn last_said(&self, leaf: &str) -> Option<String> {
        let (path, mtime) = self.transcript_path(leaf)?;
        heard_last(path, mtime, utterances_of)
    }
}

/// [`newest_said`] behind a cache keyed by file and stamped with the
/// mtime it was read at — shared by every vendor whose transcript is
/// JSONL appended in speaking order.
pub(crate) fn heard_last(
    path: PathBuf,
    mtime: std::time::SystemTime,
    parse: fn(&str) -> Vec<Utterance>,
) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (std::time::SystemTime, Heard)>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let before = match cache.lock().unwrap().get(&path) {
        Some((t, heard)) if *t == mtime => return heard.said.clone(),
        Some((_, heard)) => Some(heard.clone()),
        None => None,
    };
    let heard = newest_said(&path, before.as_ref(), parse)?;
    let said = heard.said.clone();
    cache.lock().unwrap().insert(path, (mtime, heard));
    said
}

/// A transcript's newest sentence, and how far the file was read for it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Heard {
    pub said: Option<String>,
    /// Just past the last whole line read: a line still being written
    /// is read again next time, whole.
    pub upto: u64,
}

/// The newest user or agent sentence in a transcript.
///
/// # Back from the end until a sentence, not a fixed tail
///
/// **It used to read the last 64 KiB and give up.** Claude Code writes
/// its own bookkeeping after an answer — `attachment` lines carrying
/// skill listings and tool lists, `system`, `last-prompt`, `mode`,
/// `permission-mode` — and a fresh session's attachments alone can
/// outgrow 64 KiB. The row then had no sentence and kept the screen's
/// bottom line, Claude Code's own chrome (SESS-BRIEF 1a). So the read
/// widens backwards, four times each step, until a sentence or the
/// file's head; `MOST` bounds what one look may cost.
///
/// # Only what was appended, once a look has been had
///
/// Transcripts only grow, so with `before` the read starts where the
/// last one stopped: a sentence in the new part is the answer, else the
/// old answer stands. A file shorter than that point was rewritten, and
/// is read afresh.
pub(crate) fn newest_said(
    path: &Path,
    before: Option<&Heard>,
    parse: fn(&str) -> Vec<Utterance>,
) -> Option<Heard> {
    use std::io::{Read, Seek, SeekFrom};
    const FIRST: u64 = 64 * 1024;
    /// Past this much bookkeeping a row reads as saying nothing yet,
    /// rather than costing a poll more.
    const MOST: u64 = 16 * 1024 * 1024;
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let mut read = |from: u64, to: u64| -> Option<Vec<u8>> {
        f.seek(SeekFrom::Start(from)).ok()?;
        let mut buf = Vec::with_capacity((to - from) as usize);
        (&mut f).take(to - from).read_to_end(&mut buf).ok()?;
        Some(buf)
    };
    // A last line with no newline yet is parsed — it may be whole, and
    // one still being written fails to parse and reads as absent — but
    // the next look starts at its head again.
    let whole = |buf: &[u8]| buf.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    if let Some(b) = before.filter(|b| b.upto <= len) {
        let buf = read(b.upto, len)?;
        let end = whole(&buf);
        let said = said_in(&buf, parse).or_else(|| b.said.clone());
        return Some(Heard {
            said,
            upto: b.upto + end as u64,
        });
    }
    // Just past the file's last newline, once a window has reached it.
    let mut upto: Option<u64> = None;
    // Everything at or past `end` has been looked at.
    let mut end = len;
    let mut window = FIRST;
    loop {
        let start = end.saturating_sub(window).max(len.saturating_sub(MOST));
        let buf = read(start, end)?;
        if upto.is_none() && whole(&buf) > 0 {
            upto = Some(start + whole(&buf) as u64);
        }
        // The head of the window may be the tail of a line before it,
        // unless the window starts the file.
        let head = match start {
            0 => 0,
            _ => buf
                .iter()
                .position(|&b| b == b'\n')
                .map_or(buf.len(), |i| i + 1),
        };
        let heard = |said| Heard {
            said,
            upto: upto.unwrap_or(0),
        };
        if let Some(said) = said_in(&buf[head..], parse) {
            return Some(heard(Some(said)));
        }
        if start == len.saturating_sub(MOST) {
            return Some(heard(None));
        }
        end = start + head as u64;
        window *= 4;
    }
}

/// The newest sentence in a run of whole lines.
fn said_in(bytes: &[u8], parse: fn(&str) -> Vec<Utterance>) -> Option<String> {
    parse(&String::from_utf8_lossy(bytes))
        .into_iter()
        .rev()
        .find_map(|u| match u {
            // The marks come off before the line is folded: a said line is
            // markdown, and a preview has nowhere to render it to
            // (`khor_core::plain`). `plain` first — its list and heading
            // marks only exist at the head of a line, and `preview` is what
            // takes the lines away.
            Utterance::User(t) | Utterance::Agent(t) => {
                Some(khor_core::preview(&khor_core::plain(&t)))
            }
            // A thought or a tool name is the machine's bookkeeping — a
            // preview line should say what was *said*, and skipping back to
            // the newest real sentence reads better than "Read".
            Utterance::Thought(_) | Utterance::Tool(_) | Utterance::ToolDone(_) | Utterance::Content { .. } => None,
        })
}

/// What a transcript said since it was last read.
#[derive(Debug)]
pub struct Follow {
    /// The new utterances, in speaking order. Empty when nothing was
    /// said — which is most reads, and is not an error.
    pub said: Vec<Utterance>,
    /// Where to continue from next time.
    pub at: u64,
    /// The file was shorter than the cursor, so this is the whole of it
    /// and the caller must repaint rather than append. **Not an error**
    /// — a compaction is an ordinary thing for a long conversation.
    pub rewound: bool,
    /// Whether asking again will ever say anything new.
    ///
    /// **`false` means「this is all there is, stop asking」**, and it
    /// exists because the alternative was silently expensive: a
    /// transcript nothing can seek falls back to being read whole, and
    /// without a way to say so the caller re-read the entire file every
    /// 700 ms and repainted the pane from scratch each time. The smoke
    /// caught it as a flake — a page rebuilding itself mid-assertion —
    /// which is the cheapest way that particular waste ever announces
    /// itself.
    pub following: bool,
}

/// One utterance of a recorded conversation, in speaking order.
#[derive(Debug, PartialEq)]
pub enum Utterance {
    User(String),
    Agent(String),
    Thought(String),
    Content { user: bool, block: serde_json::Value },
    /// A tool call, as **claude's own `tool_use` block, whole**.
    ///
    /// **It used to be the name alone**, on the reasoning that "a name
    /// is a fact about the conversation, an input is a payload nobody
    /// asked a chat view to carry". That reasoning was wrong in a way
    /// only visible on screen: a replayed conversation came out as
    /// forty lines all reading `Bash`, because the name is the one
    /// thing every shell call has in common.
    ///
    /// Carrying the block rather than a widened tuple is what lets
    /// [`crate::cagent::said`] be the **only** translation from claude's
    /// shape to the protocol's. Two translations is how this bug
    /// happened: the live one was taught titles, statuses and diffs,
    /// and the replay one silently kept its own older answer.
    Tool(serde_json::Value),
    /// A tool call finishing, as claude's `tool_result` block, whole.
    ///
    /// New with the block above and for the same reason: a replayed
    /// conversation had no way to say which of its tools failed, so
    /// every one of them was drawn as though it had worked.
    ToolDone(serde_json::Value),
}

/// Transcript JSONL in, utterances out. Only `user` and `assistant`
/// lines speak; the rest of the file (snapshots, modes, attachments,
/// summaries) is the vendor's bookkeeping. Within a user line, a string
/// opening with one of the CLI's own tags is plumbing a person never
/// typed, and a `tool_result` block is the machine answering itself —
/// both stay unpainted.
///
/// # Not everything on a user line was typed by the user
///
/// **`user` is the CLI's word for「went in through the prompt」, not
/// for「a person wrote this」.** A message from another agent session
/// arrives on a user line too, and it used to be painted as the
/// person's own words: verbatim, in their bubble, on their side of the
/// conversation. Two things were wrong at once — the attribution, and
/// (because this face renders only the agent's text as markdown) a wall
/// of raw markdown source.
///
/// **The protocol cannot help here.** ACP has no notion of an author:
/// `SessionUpdate` offers `UserMessageChunk`, `AgentMessageChunk` and
/// `AgentThoughtChunk` and nothing else, and there is no `author` /
/// `role` / `sender` field anywhere in its schema (checked against
/// `agent-client-protocol-schema` 1.5.0, 2026-08-31). `_meta` exists,
/// and the spec says implementations **MUST NOT** make assumptions
/// about what is in it — so it is for two ends that agreed privately,
/// not for telling khor who spoke.
///
/// So the judgment has to be made **here, where the evidence is** —
/// reading the vendor's own file, which is the only place the envelope
/// is visible at all.
pub(crate) fn utterances_of(text: &str) -> Vec<Utterance> {
    const PLUMBING: [&str; 4] = [
        "<command-name>",
        "<local-command-stdout>",
        "<local-command-caveat>",
        "<system-reminder>",
    ];
    /// The sentence the CLI writes above a message from another agent
    /// session.
    ///
    /// **Matched at the start, like `PLUMBING`**, and not by looking
    /// for `<teammate-message` anywhere in the text: a person who
    /// *writes about* teammate messages — reporting one, quoting one,
    /// asking about the tag — would have their own words reattributed
    /// to a machine. `starts_with` on a whole sentence is a thing to
    /// type by accident and not by discussion.
    ///
    /// **If the CLI ever rewords this, the message goes back to being
    /// painted as the person's** — the state before this, which is
    /// wrong but not broken. That is the failure this is allowed to
    /// have; the other direction (a person's words moved to the
    /// machine's side) is not.
    const FROM_ANOTHER_AGENT: &str = "Another Claude session sent a message:";
    let user_text = |s: &str| -> Option<Utterance> {
        if s.is_empty() || PLUMBING.iter().any(|tag| s.starts_with(tag)) {
            return None;
        }
        // Machine text, so it is painted where machine text is painted
        // — the agent's side, rendered as the markdown it is. The line
        // it keeps says whose it was, which is why this does not need
        // a fourth kind of its own.
        if s.starts_with(FROM_ANOTHER_AGENT) {
            return Some(Utterance::Agent(s.to_owned()));
        }
        Some(Utterance::User(s.to_owned()))
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let from_user = match v.get("type").and_then(|t| t.as_str()) {
            Some("user") => true,
            Some("assistant") => false,
            _ => continue,
        };
        let Some(content) = v.get("message").and_then(|m| m.get("content")) else {
            continue;
        };
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
                            out.push(Utterance::Tool(b.clone()));
                        }
                        // The machine answering itself, on a user line.
                        // Skipped entirely before this, which is why a
                        // replayed tool could never be drawn as failed.
                        (Some("tool_result"), true) => {
                            out.push(Utterance::ToolDone(b.clone()));
                        }
                        (Some("image" | "document"), _) => {
                            if let Some(block) = super::media::claude(b) {
                                out.push(Utterance::Content { user: from_user, block });
                            }
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
            let Some(text) = read_text(&path) else {
                continue;
            };
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
                    // **No status at all is not a word khor cannot
                    // read — it is no word.** A claude driven over
                    // stream-json (`--print`; khor's own conversation
                    // shim is one) never writes `status`, because its
                    // driver already knows every turn. Three of the "6
                    // sessions khor cannot read" on the author's machine
                    // were khor's own ghosts, listed a few lines up from
                    // the complaint about them. Skipped without a count:
                    // the count is the 适配器过时 signal, and this is not
                    // the vendor changing its vocabulary.
                    None if file.status.is_none() => continue,
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
            let (word, at_ms) = if word == State::Idle && file.status.as_deref() == Some("idle") {
                self.unique_transcript_path(
                    &crate::live::clean_leaf(&file.session_id),
                    Some(&file.session_id),
                )
                .ok()
                .flatten()
                .and_then(|(path, _)| state::last_ending(&path))
                .filter(|(_, at)| *at >= file.started_at)
                .unwrap_or((word, at_ms))
            } else {
                (word, at_ms)
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
                    pids: if word == State::Failed {
                        vec![]
                    } else {
                        vec![pid]
                    },
                    // The status file already carries it; nothing is
                    // read to answer this.
                    cwd: file.cwd,
                },
            ));
        }
        // One session id can have two live status files: `claude -r`
        // resumes a session into a new process while the old process is
        // still running, and both keep writing under the same
        // `sessionId`. Observed on this machine — two live pids, one id,
        // one of them two days stale. They are one session and one row,
        // and the freshest word is the true one.
        seen.sort_by_key(|(_, at, _)| std::cmp::Reverse(*at));
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

#[cfg(test)]
mod tests {
    #[test]
    fn native_image_survives_replay_with_its_author_and_mime() {
        let record=serde_json::json!({"type":"user","message":{"content":[
            {"type":"text","text":"look"},
            {"type":"image","source":{"type":"base64","media_type":"image/png","data":"aW1hZ2U="}}
        ]}}).to_string();
        let said=utterances_of(&record);
        assert!(matches!(&said[0], Utterance::User(text) if text=="look"));
        let image=crate::cagent::replayed(utterances_of(&record).pop().unwrap()).unwrap();
        let image=serde_json::to_value(image).unwrap();
        assert_eq!(image["sessionUpdate"],"user_message_chunk");
        assert_eq!(image["content"]["mimeType"],"image/png");
        assert_eq!(image["content"]["data"],"aW1hZ2U=");
    }

    use super::*;
    use crate::adaptor::Proc;

    fn fixture_home() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors")
    }

    fn claude_at_fixture() -> Claude {
        Claude::at(fixture_home().join("claude/.claude"))
    }

    fn said_line(who: &str, text: &str) -> String {
        format!(r#"{{"type":"{who}","message":{{"role":"{who}","content":"{text}"}}}}"#) + "\n"
    }

    fn bookkeeping(bytes: usize) -> String {
        format!(
            r#"{{"type":"attachment","attachment":{{"content":"{}"}}}}"#,
            "x".repeat(bytes)
        ) + "\n"
    }

    /// `newest_said` past the bookkeeping, through growth, a line caught
    /// half written, a rewrite, and its bound (SESS-BRIEF 1a).
    #[test]
    fn the_newest_sentence_is_found_however_much_bookkeeping_follows_it() {
        let dir =
            Scratch(std::env::temp_dir().join(format!("khor-newest-said-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&*dir);
        std::fs::create_dir_all(&*dir).unwrap();
        let path = dir.join("t.jsonl");
        let append = |s: &str| {
            use std::io::Write;
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(s.as_bytes())
                .unwrap();
        };

        // Several windows of bookkeeping after the answer.
        let mut text = said_line("user", "hello") + &said_line("assistant", "the answer");
        for _ in 0..12 {
            text += &bookkeeping(30 * 1024);
        }
        std::fs::write(&path, &text).unwrap();
        let first = newest_said(&path, None, utterances_of).unwrap();
        assert_eq!(first.said.as_deref(), Some("the answer"));
        assert_eq!(first.upto, text.len() as u64);

        // More bookkeeping: the old answer stands, read from where it stopped.
        append(&bookkeeping(100));
        let grown = newest_said(&path, Some(&first), utterances_of).unwrap();
        assert_eq!(grown.said.as_deref(), Some("the answer"));

        // A sentence caught half written reads as absent, then whole.
        let next = said_line("user", "and now");
        let (a, b) = next.split_at(20);
        append(a);
        let half = newest_said(&path, Some(&grown), utterances_of).unwrap();
        assert_eq!(half.said.as_deref(), Some("the answer"));
        assert_eq!(
            half.upto, grown.upto,
            "the next look starts at the half line's head"
        );
        append(b);
        let whole = newest_said(&path, Some(&half), utterances_of).unwrap();
        assert_eq!(whole.said.as_deref(), Some("and now"));

        // A rewrite shorter than the last look is read afresh.
        std::fs::write(&path, said_line("assistant", "compacted")).unwrap();
        let fresh = newest_said(&path, Some(&whole), utterances_of).unwrap();
        assert_eq!(fresh.said.as_deref(), Some("compacted"));

        // Past the bound the row says nothing, rather than costing more.
        let mut text = said_line("user", "long ago");
        for _ in 0..(17 * 1024 / 64) {
            text += &bookkeeping(64 * 1024);
        }
        std::fs::write(&path, &text).unwrap();
        assert_eq!(newest_said(&path, None, utterances_of).unwrap().said, None);
    }

    /// **A bounded read must not lose the whole file to its own
    /// bound.** `cwd_in` stops at 64 KiB, and a character does not have
    /// to end there — decoding strictly made the *entire* head
    /// unreadable, so a transcript that records its working directory
    /// on line 6 answered "none".
    ///
    /// That answer is not a gap where it is used: the resume path falls
    /// back to the home directory, so a 接管 of one of these conversations
    /// silently moved the agent somewhere else. Found on this machine's
    /// own newest transcript, which cuts mid-character at byte 65534.
    ///
    /// The fixture reproduces the cut rather than asserting about the
    /// real file: 64 KiB of a 3-byte character does not divide evenly,
    /// so the bound lands inside one by construction.
    #[test]
    fn a_working_directory_survives_a_cut_that_lands_mid_character() {
        let dir = Scratch(std::env::temp_dir().join(format!("khor-cwdcut-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&*dir);
        std::fs::create_dir_all(&*dir).unwrap();
        let path = dir.join("t.jsonl");

        let mut text = String::new();
        text.push_str(r#"{"type":"user","cwd":"/somewhere/deep","message":{}}"#);
        text.push('\n');
        // Past the bound, in a character that cannot straddle it evenly.
        while text.len() < 70 * 1024 {
            text.push('字');
        }
        std::fs::write(&path, &text).unwrap();
        // The control: the fixture really does cut mid-character. A
        // padding that happened to align would make this test pass
        // against the strict read it exists to catch.
        assert!(
            std::str::from_utf8(&text.as_bytes()[..64 * 1024]).is_err(),
            "the fixture must cut mid-character or it proves nothing",
        );

        assert_eq!(cwd_in(&path), Some(PathBuf::from("/somewhere/deep")));
    }

    /// **The working directory past a head of attachments** (SESS-BRIEF
    /// 2a). Claude Code now opens a transcript with attachments of tens
    /// of KiB — measured on the owner's: a `skill_listing` of 49 777
    /// bytes and a `deferred_tools_delta` of 17 814 — so the first line
    /// carrying `cwd` can start past a 64 KiB head, and the row was
    /// filed under no directory for the life of the process.
    ///
    /// And a miss is only as good as the file it was read from: a
    /// transcript caught before its first entry with a `cwd` gets one
    /// later, and the next look must see it.
    #[test]
    fn a_working_directory_past_the_opening_attachments_is_found() {
        let dir = Scratch(std::env::temp_dir().join(format!("khor-cwdfar-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&*dir);
        std::fs::create_dir_all(&*dir).unwrap();

        let path = dir.join("far.jsonl");
        let mut text = bookkeeping(49_777) + &bookkeeping(17_814);
        assert!(
            text.len() > 64 * 1024,
            "the fixture must put the cwd past 64 KiB"
        );
        text.push_str(r#"{"type":"user","cwd":"/far/down","message":{}}"#);
        text.push('\n');
        std::fs::write(&path, &text).unwrap();
        assert_eq!(cwd_in(&path), Some(PathBuf::from("/far/down")));

        let path = dir.join("late.jsonl");
        std::fs::write(&path, bookkeeping(100)).unwrap();
        assert_eq!(cwd_in(&path), None);
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut f, br#"{"type":"user","cwd":"/later","message":{}}"#)
            .unwrap();
        std::io::Write::write_all(&mut f, b"\n").unwrap();
        drop(f);
        assert_eq!(
            cwd_in(&path),
            Some(PathBuf::from("/later")),
            "a miss must not outlive the file's next change"
        );
    }

    /// Every line shape a real transcript holds (sampled on this
    /// machine, 2026-08-17): a spoken user string, a plumbing user
    /// string, an assistant turn of thinking + text + tool_use blocks,
    /// a tool_result riding a user line, vendor bookkeeping, and a
    /// broken line. Only the speech survives, in order.
    #[test]
    fn a_transcript_keeps_the_speech_and_drops_the_plumbing() {
        let text = concat!(
            r#"{"type":"file-history-snapshot","messageId":"x"}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":"look at the docs"}}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":"<command-name>/compact</command-name>"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"reading","signature":"s"},{"type":"text","text":"here is what I found"},{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}]}}"#,
            "\n",
            "not json at all\n",
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"...","is_error":false}]}}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"done"}]}}"#,
            "\n",
        );
        assert_eq!(
            utterances_of(text),
            vec![
                Utterance::User("look at the docs".into()),
                Utterance::Thought("reading".into()),
                Utterance::Agent("here is what I found".into()),
                Utterance::Tool(
                    serde_json::json!({"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}})
                ),
                // **Tombstone: this line used to be absent, and the
                // absence was deliberate.** A `tool_result` was
                // classed as「vendor bookkeeping」and dropped with the
                // snapshots and the slash-commands. It is not
                // bookkeeping: it is the only record of whether the
                // tool worked, and dropping it meant a replayed
                // conversation drew every tool that had failed exactly
                // like every tool that had not. The rule this test's
                // name states still holds — speech in, plumbing out —
                // and what changed is which side of that line a result
                // sits on.
                Utterance::ToolDone(
                    serde_json::json!({"type":"tool_result","tool_use_id":"t1","content":"...","is_error":false})
                ),
                Utterance::Agent("done".into()),
            ]
        );
    }

    /// The lookup runs `clean_leaf` over the file names because the id's
    /// leaf is the lossy product of that same cleaning — a full uuid
    /// (36 chars) must be found by its 24-char cleaned form.
    #[test]
    fn a_transcript_is_found_by_the_ids_lossy_leaf() {
        let _dir_tree = crate::testdir::TempTree::at("transcript");
        let dir = _dir_tree.path();
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
    }

    fn proc_of(started_ms: i64) -> Proc {
        Proc {
            name: "claude".into(),
            started_ms,
            cwd: None,
            ppid: None,
        }
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
        assert_eq!(
            word_at(&sweep, "tui/22222222-2222-4222-8222"),
            Some(State::Busy)
        );
        assert_eq!(
            word_at(&sweep, "tui/33333333-3333-4333-8333"),
            Some(State::Idle)
        );
    }

    /// Two files from the day the "6 sessions khor cannot read" were
    /// counted (2026-09-01), and what each of them is:
    ///
    /// - `4006`: `status: "shell"` — the person is running a `!`
    ///   command. The agent is idle, and the row says so.
    /// - `4007`: no `status` at all, `entrypoint: "sdk-cli"` — a
    ///   claude driven over stream-json, which never writes one. Not
    ///   a row (there is no word to paint) and **not counted**: the
    ///   count means the vendor's vocabulary moved, and no word is not
    ///   a new word.
    ///
    /// The one count left is the control: `4005` does not parse at
    /// all, and that stays counted whoever is alive — the finder is
    /// still finding.
    #[test]
    fn a_shell_status_is_idle_and_no_status_is_no_word_and_no_count() {
        let procs = Procs::of([
            (4006, proc_of(1_700_000_500_000)),
            (4007, proc_of(1_700_000_600_000)),
        ]);
        let sweep = claude_at_fixture().sweep(&procs);
        assert_eq!(
            word_at(&sweep, "tui/66666666-6666-4666-8666"),
            Some(State::Idle),
            "a person's own shell command is not the agent working"
        );
        assert_eq!(
            word_at(&sweep, "tui/77777777-7777-4777-8777"),
            None,
            "no word, no row"
        );
        assert_eq!(sweep.rows.len(), 1, "{:?}", sweep.rows);
        assert_eq!(
            sweep.unmapped, 1,
            "only the file that does not parse is counted"
        );
    }

    #[test]
    fn native_completion_is_available_on_first_read_and_busy_takes_precedence() {
        let root =
            Scratch(std::env::temp_dir().join(format!("khor-native-state-{}", std::process::id())));
        std::fs::create_dir_all(root.join("sessions")).unwrap();
        let project = root.join("projects/fixture");
        std::fs::create_dir_all(&project).unwrap();
        let at = "2026-11-04T12:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .as_millisecond();
        let id = "11111111-1111-4111-8111-111111111111";
        let status = root.join("sessions/4001.json");
        let write_status = |word: &str| {
            std::fs::write(&status, format!(r#"{{"pid":4001,"sessionId":"{id}","status":"{word}","startedAt":{},"statusUpdatedAt":{at}}}"#, at-1000)).unwrap()
        };
        let procs = Procs::of([(4001, proc_of(at - 1000))]);
        write_status("idle");
        let transcript = project.join(format!("{id}.jsonl"));
        std::fs::write(&transcript, "{\"type\":\"assistant\",\"timestamp\":\"2026-11-04T12:00:00Z\",\"message\":{\"stop_reason\":\"end_turn\"}}\n").unwrap();
        for _ in 0..2 {
            let row = Claude::at(root.to_path_buf()).sweep(&procs).rows.remove(0);
            assert_eq!((row.word, row.at_ms), (State::Done, at));
        }
        write_status("busy");
        assert_eq!(
            Claude::at(root.to_path_buf()).sweep(&procs).rows[0].word,
            State::Busy
        );
        write_status("idle");
        std::fs::write(
            &transcript,
            "{\"type\":\"user\",\"message\":{\"content\":\"next\"}}\n",
        )
        .unwrap();
        assert_eq!(
            Claude::at(root.to_path_buf()).sweep(&procs).rows[0].word,
            State::Idle
        );
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
        let _dir_tree = crate::testdir::TempTree::at("crash");
        let dir = _dir_tree.path();
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
        write(
            6002,
            "aaaaaaaa-6666-4666-8666-666666666666",
            now - CRASH_GRACE_MS - 60_000,
        );

        // Nothing alive at all: every pid in that tree is gone.
        let sweep = Claude::at(dir.join(".claude")).sweep(&Procs::default());
        assert_eq!(
            sweep.rows.len(),
            1,
            "the week-old crash is history, not a row"
        );
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
    }

    /// The hook glue end to end on payloads: an observed session
    /// **A message from another agent session is machine text, and a
    /// person *writing about* one is not.**
    ///
    /// `user` is the CLI's word for「went in through the prompt」. A
    /// teammate message arrives on a user line, and painting it as the
    /// person's own words got two things wrong at once: whose words
    /// they were, and (because only the agent's side is rendered as
    /// markdown) a wall of raw source.
    ///
    /// The second case is the one that decides the *shape* of the
    /// check. Matching `<teammate-message` anywhere in the text would
    /// reattribute a person who quotes the tag — reporting one, asking
    /// about it — to a machine, **and that direction is the expensive
    /// one**: a wrongly-attributed machine message is untidy, a
    /// wrongly-attributed person is the app putting words in their
    /// mouth. So it is `starts_with` on a whole sentence.
    #[test]
    fn a_message_from_another_agent_is_not_the_person_talking() {
        let line = |content: &str| {
            format!(
                r#"{{"type":"user","message":{{"role":"user","content":{}}}}}"#,
                serde_json::Value::String(content.to_owned())
            )
        };
        let from_peer = concat!(
            "Another Claude session sent a message:\n",
            "<teammate-message teammate_id=\"lane-88\" color=\"pink\">\n",
            "## 收工\n\n88.3 是错的\n</teammate-message>"
        );
        let got = utterances_of(&line(from_peer));
        assert!(
            matches!(&got[0], Utterance::Agent(t) if t.starts_with("Another Claude session")),
            "machine text belongs on the machine's side: {got:?}"
        );

        // **The direction that must not happen.** Someone reporting a
        // teammate message, or asking about the tag, is a person
        // talking — and the whole text is theirs.
        for typed in [
            "队友那条 <teammate-message> 渲染成用户消息了,应该是 Claude 的",
            "what does <teammate-message teammate_id=\"x\"> mean?",
            "I read: Another Claude session sent a message: is that a bug?",
        ] {
            let got = utterances_of(&line(typed));
            assert!(
                matches!(&got[0], Utterance::User(t) if t == typed),
                "a person writing about it is still a person: {got:?}"
            );
        }

        // And the plumbing filter still holds — this did not widen it.
        assert!(utterances_of(&line("<system-reminder>x</system-reminder>")).is_empty());
    }

    /// **A followed transcript, through the three things a file can do
    /// between two reads.**
    ///
    /// Grows / stays put / is replaced. The third is the one that costs
    /// something to get wrong: byte 400 of a new file is not byte 400
    /// of the old one, so continuing from the cursor would splice two
    /// conversations together and the reader would never see the seam.
    ///
    /// The partial-line case is asserted too, because it is the one a
    /// test with tidy fixtures never produces: the writer appends whole
    /// JSON objects but not atomically, so a read lands mid-object
    /// often enough. Parsing what is there and moving the cursor past
    /// it would drop that utterance permanently.
    #[test]
    fn a_followed_transcript_grows_holds_still_and_starts_over() {
        let _root_tree = crate::testdir::TempTree::at("follow");
        let root = _root_tree.path();
        let _ = std::fs::remove_dir_all(&root);
        let proj = root.join("projects").join("-home-u-proj");
        std::fs::create_dir_all(&proj).unwrap();
        let c = Claude::at(root.clone());
        let uuid = "aaaaaaaa-9999-2222-3333-444444444444";
        let file = proj.join(format!("{uuid}.jsonl"));
        let leaf = crate::live::clean_leaf(uuid);
        let line = |t: &str| {
            format!(
                r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{t}"}}]}}}}"#
            )
        };

        // 1) First read of a file with one line.
        std::fs::write(&file, format!("{}\n", line("one"))).unwrap();
        let a = c.transcript_since(&leaf, 0).unwrap();
        assert_eq!(a.said, vec![Utterance::Agent("one".into())]);
        assert!(!a.rewound);
        assert_eq!(a.at, std::fs::metadata(&file).unwrap().len());

        // 2) Nothing was said. Empty, cursor unmoved — **not an error**,
        //    which is what most reads of a live file are.
        let b = c.transcript_since(&leaf, a.at).unwrap();
        assert!(b.said.is_empty());
        assert_eq!(b.at, a.at);
        assert!(!b.rewound);

        // 3) It grew. **Only the new line comes back** — the first is
        //    already on screen, and returning it again would paint it
        //    twice.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&file)
                .unwrap();
            writeln!(f, "{}", line("two")).unwrap();
        }
        let d = c.transcript_since(&leaf, b.at).unwrap();
        assert_eq!(d.said, vec![Utterance::Agent("two".into())]);
        assert!(!d.rewound);

        // 4) A line the writer has not finished. Nothing comes back and
        //    **the cursor does not move past it**, so it is read again
        //    when it is whole.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&file)
                .unwrap();
            write!(f, "{{\"type\":\"assist").unwrap();
        }
        let e = c.transcript_since(&leaf, d.at).unwrap();
        assert!(e.said.is_empty(), "half a line is not an utterance");
        assert_eq!(e.at, d.at, "and the cursor waits for the rest of it");

        // …and when it is finished, it arrives.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&file)
                .unwrap();
            writeln!(f, "ant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"text\",\"text\":\"three\"}}]}}}}").unwrap();
        }
        let f2 = c.transcript_since(&leaf, e.at).unwrap();
        assert_eq!(f2.said, vec![Utterance::Agent("three".into())]);

        // 5) Replaced by something shorter — a compaction. The whole
        //    file comes back and `rewound` says to repaint, because
        //    appending across that seam joins two conversations.
        std::fs::write(&file, format!("{}\n", line("fresh"))).unwrap();
        let g = c.transcript_since(&leaf, f2.at).unwrap();
        assert!(g.rewound, "a shorter file is a different file");
        assert_eq!(g.said, vec![Utterance::Agent("fresh".into())]);
        assert_eq!(g.at, std::fs::metadata(&file).unwrap().len());
    }

    /// **The agent's own name for a conversation, and the three ways it
    /// is not there.** Shapes taken from real transcripts on this
    /// machine (2026-08-30): the record is `{"type":"ai-title",
    /// "aiTitle":…,"sessionId":…}`, it repeats, and every repeat in a
    /// file carries the same string.
    ///
    /// The bound is asserted from both sides. A title at line 200 is
    /// found (the worst real first-occurrence was 130); one pushed past
    /// 256 is not, and **the miss is the point** — it is what lets a
    /// list poll touch a 280 MB file for a fraction of a millisecond,
    /// and what it degrades to is the row's existing name, tested
    /// beside it.
    #[test]
    fn a_conversation_can_carry_the_name_the_agent_gave_it() {
        let _root_tree = crate::testdir::TempTree::at("aititle");
        let root = _root_tree.path();
        let _ = std::fs::remove_dir_all(&root);
        let proj = root.join("projects").join("-home-u-proj");
        std::fs::create_dir_all(&proj).unwrap();
        let c = Claude::at(root.clone());

        let filler = |n: usize| {
            (0..n)
                .map(|i| format!(r#"{{"type":"user","uuid":"u{i}","cwd":"/home/u/proj"}}"#))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let title = |t: &str| format!(r#"{{"type":"ai-title","aiTitle":"{t}","sessionId":"s"}}"#);

        // 1) Found near the head, and **repeated** — the same string
        //    several times over, which is what claude actually writes.
        let a = "aaaaaaaa-1111-2222-3333-444444444444";
        std::fs::write(
            proj.join(format!("{a}.jsonl")),
            format!(
                "{}\n{}\n{}\n{}\n",
                filler(3),
                title("把工具调用的状态接上"),
                filler(5),
                title("把工具调用的状态接上")
            ),
        )
        .unwrap();
        assert_eq!(
            c.ai_title(&crate::live::clean_leaf(a)).as_deref(),
            Some("把工具调用的状态接上")
        );

        // 2) Deep but inside the bound: 200 lines in, past every real
        //    first-occurrence measured.
        let b = "bbbbbbbb-1111-2222-3333-444444444444";
        std::fs::write(
            proj.join(format!("{b}.jsonl")),
            format!("{}\n{}\n", filler(200), title("deep but reachable")),
        )
        .unwrap();
        assert_eq!(
            c.ai_title(&crate::live::clean_leaf(b)).as_deref(),
            Some("deep but reachable")
        );

        // 3) Past the bound: not found, on purpose. Asserted so that
        //    raising or dropping the bound is a decision somebody makes
        //    rather than a number that drifts.
        let d = "dddddddd-1111-2222-3333-444444444444";
        std::fs::write(
            proj.join(format!("{d}.jsonl")),
            format!("{}\n{}\n", filler(400), title("past the bound")),
        )
        .unwrap();
        assert_eq!(c.ai_title(&crate::live::clean_leaf(d)), None);

        // 4) No title at all — 49 of 93 transcripts here. Distinct from
        //    (3) in cause and identical in effect, which is the point:
        //    both leave the row wearing the name it already had.
        let e = "eeeeeeee-1111-2222-3333-444444444444";
        std::fs::write(proj.join(format!("{e}.jsonl")), format!("{}\n", filler(10))).unwrap();
        assert_eq!(c.ai_title(&crate::live::clean_leaf(e)), None);

        // 5) An empty title is not a title. A row wearing "" would be a
        //    session with no name at all on screen, which is worse than
        //    the mechanical one it replaced.
        let f = "ffffffff-1111-2222-3333-444444444444";
        std::fs::write(
            proj.join(format!("{f}.jsonl")),
            format!("{}\n{}\n", title("   "), filler(3)),
        )
        .unwrap();
        assert_eq!(c.ai_title(&crate::live::clean_leaf(f)), None);
    }

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

    #[test]
    fn a_resumed_session_is_one_row_not_two() {
        let resumed = Claude::at(fixture_home().join("resume/.claude"));
        let procs = Procs::of([
            (5001, proc_of(1_700_000_000_000)),
            (5002, proc_of(1_700_000_600_000)),
        ]);
        let sweep = resumed.sweep(&procs);
        assert_eq!(sweep.rows.len(), 1, "one session id, one row");
        assert_eq!(
            sweep.rows[0].title, "resumed",
            "directory identity ignores the vendor's generated name"
        );
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
