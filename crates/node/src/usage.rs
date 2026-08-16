//! What the agents on this machine have spent, read off their own files.
//!
//! # Why this is not part of the adaptor seam
//!
//! [`crate::adaptor`] answers "what is running here", and every answer it
//! gives is checked against the process table — a session with no live
//! process is history, and history is not a row. Spending is the opposite
//! question in exactly that respect: **a conversation that ended last week
//! still cost what it cost.** Hanging this off `Adaptor::sweep` would tie
//! the answer to a `Procs::snapshot` that costs 5 ms and decides nothing
//! here, and would make every future vendor implement a liveness rule to
//! answer a question that has no use for one.
//!
//! So it is a seam of its own, cut the same way and for the same reason
//! the other one was cut at two implementations: what claude-on-disk and
//! codex-on-disk genuinely share is one sentence — *read your own files,
//! come back with tokens per day and a count of what you could not read* —
//! and that sentence is [`Meter`], one method wide.
//!
//! What they do **not** share is the arithmetic, and it is not a small
//! difference:
//!
//! | | claude | codex |
//! |---|---|---|
//! | a record is | one assistant message | one turn |
//! | the numbers are | that message's own | a running total **and** that turn's |
//! | repeats in the file | the same message, several times, output growing | the same reading, twice in a row |
//! | so the rule is | one reading per `message.id`, the largest | drop a reading identical to the one before it |
//!
//! Summing the lines of either file is wrong, and wrong in a different
//! direction each time. Measured on this machine 2026-08-17: claude's
//! 1069 transcripts hold 109 792 assistant lines carrying only **55 568**
//! distinct messages, so line-summing roughly doubles the answer; codex's
//! `total_token_usage` is cumulative, so line-summing it produces a number
//! with no meaning at all.
//!
//! # The category is stamped here, not by the meter
//!
//! Same judgment as [`crate::adaptor::Found`]: whose tokens these were is
//! the name of whoever recognised them, so [`Meters::tally`] attaches it
//! and a meter has nowhere to write it. A field a meter filled in would be
//! a field a meter could fill in wrongly.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use khor_core::{Tokens, Usage, UsageDay};

/// One vendor's spending, before anyone has said whose it is.
#[derive(Debug, Default, Clone)]
pub struct Tally {
    /// Local civil date (`YYYY-MM-DD`) to what was spent on it.
    pub by_day: HashMap<String, Tokens>,
    /// Records this meter found and could not read. See
    /// [`khor_core::Usage::unreadable`].
    pub unreadable: u64,
}

impl Tally {
    fn add(&mut self, day: String, tokens: Tokens) {
        self.by_day.entry(day).or_default().add(tokens);
    }
}

/// One vendor's spending surface.
///
/// Deliberately one method, and deliberately without a process table. See
/// the module head.
pub trait Meter: Send + Sync {
    /// The vendor's name, which becomes the category on every row this
    /// meter produced.
    fn vendor(&self) -> &'static str;

    /// The directory this meter reads.
    ///
    /// **Not for reading — for asking cheaply whether anything in it
    /// changed.** It is the second method only because a full pass costs
    /// seconds (see [`Meters::tally`]) and the check that avoids one has
    /// to know where to look. Keeping the layout knowledge here rather
    /// than in [`Meters`] is the same rule the sweep follows: where a
    /// vendor keeps its files is the vendor module's business.
    fn root(&self) -> PathBuf;

    /// Everything this vendor's files on this machine say was spent.
    fn tally(&self, zone: &jiff::tz::TimeZone) -> Tally;
}

/// Every vendor khor can read spending from, rooted at one pretend-home.
///
/// Rooted for the reason [`crate::adaptor::Discovery`] is: a test, or the
/// second instance of a dual-instance verification, has no claim on the
/// real user's agents — and a check that reads the developer's own
/// transcripts is a check whose numbers nobody else can reproduce.
pub struct Meters {
    meters: Vec<Box<dyn Meter>>,
    /// The last answer and the shape of the tree it was read from.
    /// See [`Meters::tally`].
    cached: Mutex<Option<Cached>>,
    /// How many full passes over the files this registry has made.
    ///
    /// **Without it, "it is cached" is not a claim anybody can check** —
    /// a cache that quietly re-read everything would still answer
    /// correctly, only slowly, and nothing would say so.
    ///
    /// A field rather than a process-wide static, which is where
    /// `crate::adaptor::snapshots_taken` sits: that one counts something
    /// process-wide, and paid for it with a test that has to be run
    /// serially or it counts its neighbours' work too (`tests/cost.rs`
    /// says so at length). This counts one registry's passes, so the
    /// question is answerable without asking anybody to run anything a
    /// particular way.
    passes: AtomicU64,
}

/// An answer, and what the tree looked like when it was given.
struct Cached {
    tree: Shape,
    usage: Usage,
}

/// A tree, cheaply: how many files, how many bytes, and the newest one.
///
/// Enough to notice everything that can happen to these trees, because
/// they are **append-only logs**: a turn adds bytes and moves an mtime, a
/// new session adds a file. It would miss an edit that kept a file's size
/// and its timestamp, which is not something a vendor writing a log does
/// — and the honest alternative, hashing 1.5 GB, costs more than the pass
/// it is trying to avoid.
#[derive(Debug, Default, PartialEq, Eq)]
struct Shape {
    files: u64,
    bytes: u64,
    newest_ms: i64,
}

impl Meters {
    /// Reads nothing. The closed default, so that reading a real home is
    /// an explicit decision made in one place.
    pub fn empty() -> Meters {
        Meters { meters: Vec::new(), cached: Mutex::new(None), passes: AtomicU64::new(0) }
    }

    /// The vendors under `home`.
    pub fn at(home: &Path) -> Meters {
        Meters {
            meters: vec![
                Box::new(claude::Claude::at(home.join(".claude"))),
                Box::new(codex::Codex::at(home.join(".codex"))),
            ],
            cached: Mutex::new(None),
            passes: AtomicU64::new(0),
        }
    }

    /// How many times this registry has read every file. See
    /// [`Meters::passes`].
    pub fn passes(&self) -> u64 {
        self.passes.load(Ordering::Relaxed)
    }

    /// Adds one meter. How a test names its own vendor.
    pub fn with(mut self, meter: Box<dyn Meter>) -> Meters {
        self.meters.push(meter);
        self
    }

    /// This machine's spending, re-read only when the files have moved.
    ///
    /// # Why this one is cached and `khor_core::Vitals` is not
    ///
    /// A reading of a machine is true for seconds, so caching it would be
    /// caching a lie. Spending is the opposite: **what was spent
    /// yesterday will never change again**, and the only part of the
    /// answer that moves is today's, when an agent writes another line.
    /// So the whole answer is kept and thrown away the moment the tree
    /// underneath it looks different ([`Shape`]).
    ///
    /// **The measurement is what makes this necessary rather than
    /// tidy.** On this machine 2026-08-17, debug build: one pass is
    /// **18.5 s** over 1149 files and 1482 MiB, and the check that
    /// replaces it is a walk of directory entries. The app polls every
    /// two seconds. Without the cache this could not be asked from
    /// anywhere near a list.
    ///
    /// **Blocking, and it holds the lock while it reads** — so the one
    /// async caller goes through `spawn_blocking`, the same way vitals
    /// does. Holding the lock is deliberate: a second asker arriving
    /// mid-pass waits and then finds the answer, instead of starting an
    /// identical eighteen-second walk beside the first.
    pub fn tally(&self) -> Usage {
        let zone = jiff::tz::TimeZone::system();
        let tree = self.shape();
        let mut cached = self.cached.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(had) = cached.as_ref() {
            if had.tree == tree {
                return had.usage.clone();
            }
        }
        let usage = self.tally_in(&zone);
        *cached = Some(Cached { tree, usage: usage.clone() });
        usage
    }

    /// What the vendors' trees look like right now.
    fn shape(&self) -> Shape {
        let mut shape = Shape::default();
        for m in &self.meters {
            let mut stack = vec![m.root()];
            while let Some(dir) = stack.pop() {
                let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                for e in rd.flatten() {
                    match e.file_type() {
                        Ok(t) if t.is_dir() => stack.push(e.path()),
                        Ok(t) if t.is_file() => {
                            let Ok(meta) = e.metadata() else { continue };
                            shape.files += 1;
                            shape.bytes = shape.bytes.saturating_add(meta.len());
                            if let Ok(at) = meta.modified() {
                                let ms = at
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_millis() as i64)
                                    .unwrap_or(0);
                                shape.newest_ms = shape.newest_ms.max(ms);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        shape
    }

    /// One pass over every vendor, in a given time zone, reading
    /// everything. **Not cached** — this is what [`Meters::tally`] calls
    /// when it has decided a pass is due.
    ///
    /// **The zone is handed down rather than asked for by each meter**:
    /// two meters asking separately could straddle a daylight saving
    /// change and file two halves of one afternoon under different rules.
    /// It is also the seam a test uses to pin a zone, since an assertion
    /// about which day a timestamp falls on is otherwise an assertion
    /// about where the machine running the tests happens to be.

    /// The same pass in a given zone.
    pub fn tally_in(&self, zone: &jiff::tz::TimeZone) -> Usage {
        self.passes.fetch_add(1, Ordering::Relaxed);
        let mut rows: HashMap<(String, &'static str), Tokens> = HashMap::new();
        let mut unreadable = 0u64;
        for m in &self.meters {
            let tally = m.tally(zone);
            unreadable = unreadable.saturating_add(tally.unreadable);
            for (day, tokens) in tally.by_day {
                rows.entry((day, m.vendor())).or_default().add(tokens);
            }
        }
        let mut days: Vec<UsageDay> = rows
            .into_iter()
            // A day on which a vendor spent nothing is not a row. It can
            // happen: a file khor can open, holding only records it
            // cannot read, would otherwise produce a row of zeroes that
            // reads as "this agent ran and cost nothing".
            .filter(|(_, tokens)| !tokens.is_zero())
            .map(|((day, category), tokens)| UsageDay {
                day,
                category: category.to_owned(),
                tokens,
            })
            .collect();
        // Oldest first, and within a day by vendor name, so that two
        // machines asked the same question answer in the same order —
        // a face that had to sort would be the second sort this repo
        // keeps refusing to grow (`crate::list`).
        days.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.category.cmp(&b.category)));
        Usage { days, unreadable }
    }
}

// ── reading the files ───────────────────────────────────────

/// Every `.jsonl` under `root`, at any depth, sorted.
///
/// **At any depth, and that is a correction rather than generality for its
/// own sake.** The ledger recorded claude's transcripts as
/// `projects/<project>/<sid>.jsonl`; measured on this machine 2026-08-17,
/// only 81 of 1069 of them are shaped like that. The rest sit under
/// `<sid>/subagents/…` and `<sid>/subagents/workflows/<wf>/…`, and those
/// are a subagent's tokens — spent by this machine, on this account,
/// exactly as much as the ones in the file above them.
///
/// Sorted so that two runs over an unchanged tree produce the same answer;
/// directory order is the filesystem's business and it changes.
fn jsonl_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            match e.file_type() {
                // Symlinks are not followed: a link into the tree would
                // make one transcript two, and every token in it count
                // twice.
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => {
                    if path.extension().and_then(|x| x.to_str()) == Some("jsonl") {
                        out.push(path);
                    }
                }
                _ => {}
            }
        }
    }
    out.sort();
    out
}

/// One vendor file, line by line, with an unterminated tail forgiven.
///
/// **Buffered rather than read whole, and that is a size decision.** The
/// largest transcript on this machine is 280 MB (measured 2026-08-17);
/// slurping it would put that much on the heap to look at four numbers per
/// line, on a walk that visits a thousand files.
///
/// **A line that no newline terminates is a write in progress, not a
/// format khor has stopped understanding.** These files are appended to by
/// a program that is running right now, and the honest test for "half
/// written" is the missing terminator — not "it was the last line", which
/// would also forgive a genuinely broken final record that the vendor
/// finished writing. Without the distinction the drift alarm below would
/// tick up whenever an agent happened to be mid-write, and an alarm that
/// cries wolf is worse than none.
///
/// Returns how many records it could not read — **the meter says so by
/// returning [`Read::Unreadable`], never by reaching for a counter**, so
/// that "this line was json but not a shape I know" and "this line was not
/// json" are added up in one place and cannot drift apart.
fn for_each_record(path: &Path, mut f: impl FnMut(&serde_json::Value) -> Read) -> u64 {
    use std::io::BufRead;

    let mut unreadable = 0u64;
    let Ok(file) = std::fs::File::open(path) else {
        // A file khor cannot open at all is not a record it misread — it
        // may be somebody else's permissions, or a file mid-replacement.
        return 0;
    };
    let mut reader = std::io::BufReader::new(file);
    // One buffer for the whole file: a tool result can be megabytes, and
    // re-allocating per line would do that a thousand times over.
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            // Not text at all. That is a format khor cannot read, and it
            // says so once rather than pretending the rest of the file
            // is empty.
            Err(_) => {
                unreadable = unreadable.saturating_add(1);
                break;
            }
        }
        let terminated = buf.ends_with('\n');
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        let read = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => f(&v),
            Err(_) if !terminated => Read::Fine,
            Err(_) => Read::Unreadable,
        };
        if matches!(read, Read::Unreadable) {
            unreadable = unreadable.saturating_add(1);
        }
    }
    unreadable
}

/// What one line was to a meter.
enum Read {
    /// Nothing was missed: the meter either counted this line or knows it
    /// is not a spending record. **The two are one answer on purpose** —
    /// most lines in these files are conversation, and a meter that had
    /// to distinguish "not mine" from "counted" would be keeping a second
    /// tally nobody reads.
    Fine,
    /// Plainly a spending record, and khor could not make sense of it.
    /// See [`khor_core::Usage::unreadable`].
    Unreadable,
}

/// The local civil date a UTC instant falls on, `YYYY-MM-DD`.
///
/// `None` for a timestamp khor cannot read, which the caller counts: a
/// record whose numbers are legible but whose day is not cannot be filed
/// anywhere, and filing it under today would move last month's spending
/// onto this morning.
fn day_of(ts: &str, zone: &jiff::tz::TimeZone) -> Option<String> {
    let stamp: jiff::Timestamp = ts.parse().ok()?;
    Some(stamp.to_zoned(zone.clone()).date().to_string())
}

pub mod claude {
    //! Claude Code: one record per assistant message, repeated.
    //!
    //! Each assistant line carries `message.usage` with the four billing
    //! kinds. The catch is that **one message is written several times** —
    //! once as each block of it arrives — and the repeats are not copies:
    //! `output_tokens` grows across them while the input numbers stay put.
    //! Measured 2026-08-17 over every transcript on this machine, 15 593
    //! messages were written more than once with differing numbers, and in
    //! **every one of them** the output was non-decreasing.
    //!
    //! So the rule is one reading per `message.id`, the one with the
    //! largest output — a whole reading, not a per-field maximum. That
    //! distinction is load-bearing: in 8 of those messages the final
    //! writing also **moved** input from cache-write to cache-read (one
    //! observed pair: `cache_write 34020, cache_read 0` becoming
    //! `24352, 9664`). A per-field maximum would keep the 34020 and add
    //! the 9664 to it, inventing tokens nobody spent.
    //!
    //! **And the dedup is across files, not within one.** A resumed
    //! session copies its history into the new transcript: 179 message ids
    //! on this machine appear in two files. Per-file dedup would bill a
    //! resumed conversation twice for everything said before the resume.

    use std::collections::HashMap;
    use std::path::PathBuf;

    use khor_core::Tokens;

    use super::{day_of, for_each_record, jsonl_under, Meter, Read, Tally};

    /// This vendor's name — the same string [`crate::adaptor::claude`]
    /// stamps on its rows, so a session and its tokens land in one
    /// category.
    pub const VENDOR: &str = crate::adaptor::claude::VENDOR;

    pub struct Claude {
        root: PathBuf,
    }

    impl Claude {
        pub fn at(root: PathBuf) -> Claude {
            Claude { root }
        }

        fn projects_dir(&self) -> PathBuf {
            self.root.join("projects")
        }
    }

    /// One assistant message's reading, as the file spells it.
    struct Reading {
        day: String,
        tokens: Tokens,
    }

    /// The numbers off one `message.usage`, mapped onto the four names.
    ///
    /// **`input_tokens` is carried across unchanged**, unlike codex's:
    /// this vendor already reports it net of the cache, which is visible
    /// in the files as a two-digit input beside a five-digit cache read.
    /// A record with no `usage` object is not this vendor's spending
    /// record and never reaches here.
    fn tokens_of(usage: &serde_json::Value) -> Tokens {
        let n = |k: &str| usage.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        Tokens {
            input: n("input_tokens"),
            cached_input: n("cache_read_input_tokens"),
            cache_write: n("cache_creation_input_tokens"),
            output: n("output_tokens"),
        }
    }

    impl Meter for Claude {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.projects_dir()
        }

        fn tally(&self, zone: &jiff::tz::TimeZone) -> Tally {
            let mut out = Tally::default();
            // Keyed by message id, holding the fullest reading seen so
            // far. Sized by the machine rather than by one file, because
            // the same message can arrive from two of them — measured
            // 55 568 entries here, a few megabytes.
            let mut best: HashMap<String, Reading> = HashMap::new();
            for path in jsonl_under(&self.projects_dir()) {
                out.unreadable = out.unreadable.saturating_add(for_each_record(&path, |v| {
                    if v.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
                        return Read::Fine;
                    }
                    let Some(message) = v.get("message") else { return Read::Fine };
                    let Some(usage) = message.get("usage").filter(|u| u.is_object()) else {
                        return Read::Fine;
                    };
                    // An assistant message that bills something and says
                    // neither who it is nor when it happened cannot be
                    // deduplicated or filed. That is a shape khor has
                    // stopped understanding, which is what the count is
                    // for — measured zero on this machine today.
                    let (Some(id), Some(ts)) = (
                        message.get("id").and_then(serde_json::Value::as_str),
                        v.get("timestamp").and_then(serde_json::Value::as_str),
                    ) else {
                        return Read::Unreadable;
                    };
                    let Some(day) = day_of(ts, zone) else {
                        return Read::Unreadable;
                    };
                    let tokens = tokens_of(usage);
                    match best.get(id) {
                        // The same message written again, no further
                        // along than last time.
                        Some(seen) if seen.tokens.output >= tokens.output => {}
                        _ => {
                            best.insert(id.to_owned(), Reading { day, tokens });
                        }
                    }
                    Read::Fine
                }));
            }
            for reading in best.into_values() {
                out.add(reading.day, reading.tokens);
            }
            out
        }
    }
}

pub mod codex {
    //! Codex: one record per turn, written twice, next to a running total.
    //!
    //! A `token_count` event carries both `total_token_usage` (everything
    //! this session has spent) and `last_token_usage` (that turn alone).
    //! **The per-turn number is the one khor sums**, for two reasons that
    //! each decide it on their own: the running total cannot be attributed
    //! to a day, and it *resets* when a session is resumed — measured on
    //! this machine in 2 of 71 sessions, where the final total came out
    //! well below the sum of the turns that made it.
    //!
    //! Each event is written twice in a row with identical numbers, so a
    //! reading identical to the one immediately before it in the same file
    //! is dropped. **That rule is checked against something khor did not
    //! compute**: in the 69 sessions whose total never reset, the sum of
    //! the deduplicated turns equals the session's own final
    //! `total_token_usage` exactly — a number the vendor wrote and khor
    //! only reads.
    //!
    //! # Where the two vendors' names disagree
    //!
    //! Codex's `input_tokens` **includes** the cached part, and claude's
    //! does not. So this one subtracts, and `khor_core::Tokens::input`
    //! means the same thing on both. Verified across all 5392 readings on
    //! this machine: `cached_input <= input` always, and
    //! `total == input + output` always, so the subtraction cannot go
    //! negative and does not lose a third category on the way.
    //!
    //! Reasoning output is **not** added to output: measured on the same
    //! readings, `reasoning_output_tokens <= output_tokens` always, which
    //! is what it looks like when the second is already inside the first.

    use std::path::PathBuf;

    use khor_core::Tokens;

    use super::{day_of, for_each_record, jsonl_under, Meter, Read, Tally};

    /// This vendor's name — the same string
    /// [`crate::adaptor::codex`] stamps on its rows.
    pub const VENDOR: &str = crate::adaptor::codex::VENDOR;

    pub struct Codex {
        root: PathBuf,
    }

    impl Codex {
        pub fn at(root: PathBuf) -> Codex {
            Codex { root }
        }

        fn sessions_dir(&self) -> PathBuf {
            self.root.join("sessions")
        }
    }

    /// One turn's numbers, mapped onto the four names. See the module head
    /// for why input is a subtraction here and is not one for claude.
    fn tokens_of(last: &serde_json::Value) -> Tokens {
        let n = |k: &str| last.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        let cached = n("cached_input_tokens");
        Tokens {
            input: n("input_tokens").saturating_sub(cached),
            cached_input: cached,
            // Only newer codex writes this; 44 of 5392 readings here have
            // it. Absent means the vendor did not say, and the only
            // number khor can put in a counter for something nobody said
            // is nothing.
            cache_write: n("cache_write_input_tokens"),
            output: n("output_tokens"),
        }
    }

    impl Meter for Codex {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.sessions_dir()
        }

        fn tally(&self, zone: &jiff::tz::TimeZone) -> Tally {
            let mut out = Tally::default();
            for path in jsonl_under(&self.sessions_dir()) {
                // The previous reading **in this file**. Two sessions
                // that happened to bill identically are two turns, and
                // nothing joins them.
                let mut previous: Option<(String, Tokens)> = None;
                let missed = for_each_record(&path, |v| {
                    let Some(payload) = v.get("payload") else { return Read::Fine };
                    if payload.get("type").and_then(serde_json::Value::as_str)
                        != Some("token_count")
                    {
                        return Read::Fine;
                    }
                    // `info: null` is codex saying it has no numbers for
                    // this event — a rate-limit notice rides the same
                    // event type. Nothing to read, nothing missed.
                    let Some(info) = payload.get("info").filter(|i| i.is_object()) else {
                        return Read::Fine;
                    };
                    // A token_count that reports no turn, or one khor
                    // cannot place in a day, is a shape it has stopped
                    // understanding.
                    let Some(last) = info.get("last_token_usage").filter(|u| u.is_object())
                    else {
                        return Read::Unreadable;
                    };
                    let Some(day) = v
                        .get("timestamp")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|ts| day_of(ts, zone))
                    else {
                        return Read::Unreadable;
                    };
                    let reading = (day, tokens_of(last));
                    if previous.as_ref() == Some(&reading) {
                        return Read::Fine;
                    }
                    out.add(reading.0.clone(), reading.1);
                    previous = Some(reading);
                    Read::Fine
                });
                out.unreadable = out.unreadable.saturating_add(missed);
            }
            out
        }
    }
}

#[cfg(test)]
mod tests;
