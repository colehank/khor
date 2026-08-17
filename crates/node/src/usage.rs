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
//! come back with what was spent and a count of what you could not read* —
//! and that sentence is [`Meter`], two methods wide.
//!
//! What they do **not** share is the arithmetic, and it is not a small
//! difference:
//!
//! | | claude | codex | gemini |
//! |---|---|---|---|
//! | a record is | one assistant message | one turn | one answer |
//! | the numbers are | that message's own | a running total **and** that turn's | that answer's own |
//! | repeats in the file | the same message, several times, output growing | the same reading, twice in a row | the same answer, twice, identical |
//! | so the rule is | one reading per `message.id`, the largest | drop a reading identical to the one before it | one reading per `id`, the largest |
//!
//! Summing the lines of any of them is wrong, and wrong in a different
//! direction each time. Measured on this machine 2026-08-17: claude's
//! 1069 transcripts hold 109 792 assistant lines carrying only **55 568**
//! distinct messages, so line-summing roughly doubles the answer; codex's
//! `total_token_usage` is cumulative, so line-summing it produces a number
//! with no meaning at all; gemini's two recordings hold 106 records
//! carrying **58** distinct answers.
//!
//! # Which instant names the day
//!
//! **The one the answer was written at**, which is the line a meter reads
//! the numbers off. So a turn asked at 23:55 and answered at 00:05 counts
//! entire against the second day.
//!
//! Two reasons, and the second is the load-bearing one. The tokens are
//! spent producing the answer, so the answer's own clock is the one that
//! saw them go; and a meter reads forward through an append-only file
//! ([`Files`]), where the line that asked may sit in another file
//! altogether — a rule that reached back for it would be a rule that
//! cannot be applied to a tail.
//!
//! It is a real choice rather than a detail, and the alternative is in
//! use: `tokscale` stamps an answer with the request that started it, and
//! against this machine's transcripts the two rules disagree about 21
//! answers, moving them across a midnight in one direction or the other
//! (net zero — nothing is gained or lost, days only trade).
//!
//! # Nothing in here knows what day it is
//!
//! A meter keeps [`Kept`] records stamped with the instant they happened,
//! and the calendar is applied once, at the very end, in
//! [`Meters::tally_in`]. That is not tidiness: it is what lets the reading
//! be **cached across calls**, since a cache holding civil dates would be
//! a cache of one time zone's opinion, and it is what keeps two vendors
//! from straddling a daylight saving change and filing two halves of one
//! afternoon under different rules.
//!
//! # The category is stamped here, not by the meter
//!
//! Same judgment as [`crate::adaptor::Found`]: whose tokens these were is
//! the name of whoever recognised them, so [`Meters::tally_in`] attaches
//! it and a meter has nowhere to write it. A field a meter filled in would
//! be a field a meter could fill in wrongly.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use khor_core::{Tokens, Usage, UsageDay};

/// One record a meter kept: what was spent, and when.
///
/// **The instant, not the day.** See the module head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kept {
    pub at: jiff::Timestamp,
    pub tokens: Tokens,
}

/// One vendor's spending, before anyone has said whose it is or what day
/// it was.
#[derive(Debug, Default, Clone)]
pub struct Tally {
    pub kept: Vec<Kept>,
    /// Records this meter found and could not read. See
    /// [`khor_core::Usage::unreadable`].
    pub unreadable: u64,
}

/// One vendor's spending surface.
pub trait Meter: Send + Sync {
    /// The vendor's name, which becomes the category on every row this
    /// meter produced.
    fn vendor(&self) -> &'static str;

    /// The directory this meter reads.
    ///
    /// **Not for reading — for asking cheaply whether anything in it
    /// changed.** It is the second method only because even an
    /// incremental pass has to visit every file to find out
    /// ([`Meters::tally`] has the figures). Keeping the layout knowledge
    /// here rather than in [`Meters`] is the same rule the sweep follows:
    /// where a vendor keeps its files is the vendor module's business.
    fn root(&self) -> PathBuf;

    /// Everything this vendor's files on this machine say was spent.
    ///
    /// **Reads only what has been appended since last time** — see
    /// [`Files`]. Takes `&self` because that bookkeeping is the meter's
    /// own business and no caller should be able to get it wrong.
    fn tally(&self) -> Tally;
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
    /// How many times this registry has folded its meters' records into
    /// an answer.
    ///
    /// **Without it, "it is cached" is not a claim anybody can check** —
    /// a cache that quietly redid everything would still answer
    /// correctly, only slowly, and nothing would say so.
    ///
    /// A field rather than a process-wide static, which is where
    /// `crate::adaptor::snapshots_taken` sits: that one counts something
    /// process-wide, and paid for it with a test that has to be run
    /// serially or it counts its neighbours' work too (`tests/cost.rs`
    /// says so at length). This counts one registry's work, so the
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
/// — and the honest alternative, hashing gigabytes, costs more than the
/// work it is trying to avoid.
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
        Meters {
            meters: Vec::new(),
            cached: Mutex::new(None),
            passes: AtomicU64::new(0),
        }
    }

    /// The vendors under `home`.
    ///
    /// A vendor whose directory is not there reads nothing and costs one
    /// failed `read_dir`, which is what makes it cheap to know about
    /// agents nobody here runs (see [`pi`]).
    pub fn at(home: &Path) -> Meters {
        let mut meters: Vec<Box<dyn Meter>> = vec![
            Box::new(claude::Claude::at(home.join(".claude"))),
            Box::new(codex::Codex::at(home.join(".codex"))),
            Box::new(gemini::Gemini::at(home.join(".gemini"))),
        ];
        for (vendor, root) in pi::ROOTS {
            meters.push(Box::new(pi::PiFormat::at(vendor, home.join(root))));
        }
        for (vendor, root) in roo::roots() {
            meters.push(Box::new(roo::Roo::at(vendor, home.join(root))));
        }
        meters.push(Box::new(qwen::Qwen::at(home.join(".qwen"))));
        meters.push(Box::new(junie::Junie::at(home.join(".junie"))));
        meters.push(Box::new(amp::Amp::at(home.join(".local/share/amp"))));
        Meters { meters, cached: Mutex::new(None), passes: AtomicU64::new(0) }
    }

    /// Adds one meter. How a test names its own vendor.
    pub fn with(mut self, meter: Box<dyn Meter>) -> Meters {
        self.meters.push(meter);
        self
    }

    /// How many times this registry has folded an answer. See
    /// [`Meters::passes`].
    pub fn passes(&self) -> u64 {
        self.passes.load(Ordering::Relaxed)
    }

    /// This machine's spending, in the machine's own time zone.
    ///
    /// # What it costs, measured, and what the measurement decided
    ///
    /// Measured on this machine 2026-08-17 (debug build, 1149 transcript
    /// files, 1483 MiB), and the three numbers are the design:
    ///
    /// ```text
    /// cold, every byte                18.0 – 18.8 s
    /// nothing appended, folded again   0.11 – 0.14 s
    /// nothing appended, not folded     under 10 ms
    /// ```
    ///
    /// A spread rather than a number, because these were taken on a
    /// machine with other work on it, and a single figure would invite
    /// somebody to treat a later one as a regression.
    ///
    /// The app polls every two seconds, so the first figure decides two
    /// things rather than one.
    ///
    /// - **It cannot be re-read from scratch when something changes.**
    ///   While an agent is working, *something is always changing* — that
    ///   is what a transcript is — so a cache that threw everything away
    ///   on any change would pay the full 18.5 s on nearly every ask. The
    ///   meters therefore read **only the bytes appended since last
    ///   time** ([`Files`]) — the 18.40 s becomes 0.14 s.
    /// - **And it must not re-fold when nothing changed either.** That
    ///   0.14 s is still tens of thousands of records folded to produce
    ///   an answer nobody's files changed; [`Shape`] is the cheap
    ///   question that skips it, and the whole cost of an unchanged tree
    ///   is one walk of directory entries.
    ///
    /// **Blocking, and it holds the lock while it reads** — so the one
    /// async caller goes through `spawn_blocking`, the same way vitals
    /// does. Holding the lock is deliberate: a second asker arriving
    /// mid-pass waits and then finds the answer, instead of starting an
    /// identical walk beside the first.
    ///
    /// # Why this one is cached and `khor_core::Vitals` is not
    ///
    /// A reading of a machine is true for seconds, so caching it would be
    /// caching a lie. Spending is the opposite: **what was spent
    /// yesterday will never change again**, and the only part of the
    /// answer that moves is today's.
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

    /// The same answer with the calendar of a given zone. **Not cached**
    /// — this is what [`Meters::tally`] calls once it has decided the
    /// answer is due, and it is the seam a test uses to pin a zone, since
    /// an assertion about which day a timestamp falls on is otherwise an
    /// assertion about where the machine running the tests happens to be.
    pub fn tally_in(&self, zone: &jiff::tz::TimeZone) -> Usage {
        self.passes.fetch_add(1, Ordering::Relaxed);
        let mut rows: HashMap<(String, &'static str), Tokens> = HashMap::new();
        let mut unreadable = 0u64;
        for m in &self.meters {
            let tally = m.tally();
            unreadable = unreadable.saturating_add(tally.unreadable);
            for k in tally.kept {
                let day = k.at.to_zoned(zone.clone()).date().to_string();
                rows.entry((day, m.vendor())).or_default().add(k.tokens);
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

/// The registry reading `home`, shared by everyone in this process who
/// asks for it.
///
/// # Why it outlives the call at all
///
/// Same shape and same reason as `crate::vitals`'s sampler: **the thing
/// worth keeping is not the answer, it is what makes the next answer
/// cheap.** A `Meters` remembers how far it has read into every
/// transcript, and a fresh one has to read all of them — 18 s on this
/// machine. The GUI's data layer opens a `Node` per call
/// (`khor_gui_core`), so without this every press of a button would pay
/// that again, and the feature would be unusable in exactly the place it
/// is for.
///
/// Keyed by home, unlike the sampler, which is a single global: a machine
/// has one set of readings but a process can be rooted at several homes
/// — the dual-instance verifications are, and so is every test. Two nodes
/// on one home sharing this is right; two nodes on two homes must not.
///
/// The map only grows, which is bounded in production (one home) and
/// costs a test suite one entry per temp directory it opens.
pub fn meters_for(home: &Path) -> std::sync::Arc<Meters> {
    static ALL: std::sync::OnceLock<Mutex<HashMap<PathBuf, std::sync::Arc<Meters>>>> =
        std::sync::OnceLock::new();
    let mut all = ALL
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    all.entry(home.to_path_buf())
        .or_insert_with(|| std::sync::Arc::new(Meters::at(home)))
        .clone()
}

/// Today, where this machine stands.
///
/// **The same zone that cut the days**, which is the whole reason the
/// window below is computed here rather than by whoever is drawing one: a
/// face that worked out "today" its own way would eventually disagree
/// with the rows it is filtering, and the disagreement would only show up
/// around midnight.
fn today() -> jiff::civil::Date {
    jiff::Timestamp::now().to_zoned(jiff::tz::TimeZone::system()).date()
}

/// The earliest day a window of `days` ending today takes in.
///
/// `days = 1` is today alone; `days = 0` is not a window anybody can mean
/// and callers refuse it before getting here, so it is treated as one day
/// rather than given a meaning of its own.
///
/// **No upper end.** A machine east of here cuts its days where it
/// stands, so its "today" can be this machine's tomorrow — and a window
/// that stopped at today would hide real spending from a real machine
/// (`khor_core::UsageDay` says why every machine's day is its own).
///
/// Comparing a **peer's** day against a window cut here is off by at most
/// a day at the boundary, since that peer cut its days where *it* stands.
/// That is the cost of every machine's day being its own
/// (`khor_core::UsageDay`), and it is the smaller of the two errors
/// available.
pub fn window_start(days: usize) -> String {
    let back = jiff::Span::new().days(days.saturating_sub(1).min(36_500) as i64);
    today().saturating_sub(back).to_string()
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

/// Every `.json` under `root`, at any depth, sorted. The sibling of
/// [`jsonl_under`] for vendors that keep one document per conversation.
fn json_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => {
                    if path.extension().and_then(|x| x.to_str()) == Some("json") {
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

/// Every file called `name` under `root`, at any depth, sorted.
///
/// The sibling of [`jsonl_under`] for vendors that keep one document per
/// task in a directory of its own — the name is the whole match, because
/// those directories hold several JSON files and only one of them is the
/// log ([`roo`]).
fn named_under(root: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(path),
                Ok(t) if t.is_file() => {
                    if path.file_name().and_then(|x| x.to_str()) == Some(name) {
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

/// What one line was to a meter.
pub enum Read {
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

/// A meter's memory of the files it has already read.
///
/// **The whole point is not to read a byte twice.** These are append-only
/// logs; a full re-read of this machine's is 18.5 s, and while an agent is
/// working there is always something new at the end of one of them. So
/// each file remembers how far it has been consumed and what came out of
/// it, and a pass reads the tail.
///
/// `S` is whatever the vendor accumulates per file — the two vendors keep
/// different things, which is exactly why this holds it rather than
/// defining it.
struct Files<S> {
    by_path: HashMap<PathBuf, Held<S>>,
}

struct Held<S> {
    /// Bytes folded in so far, **always at a line boundary**. See
    /// [`Files::fold`] for why that is the same rule as forgiving a
    /// half-written tail.
    consumed: u64,
    unreadable: u64,
    state: S,
}

impl<S: Default> Default for Files<S> {
    fn default() -> Files<S> {
        Files { by_path: HashMap::new() }
    }
}

impl<S: Default> Files<S> {
    /// Brings every file under `root` up to date, then hands the caller
    /// each file's accumulated state.
    ///
    /// A file that **shrank** since last time is not the file that was
    /// read: a log does not lose its beginning, so a shorter one has been
    /// replaced or rotated, and its state is thrown away and rebuilt. A
    /// file that vanished takes its state with it — otherwise a machine
    /// that deletes old transcripts would keep billing for them forever.
    fn refresh(
        &mut self,
        root: &Path,
        mut fold: impl FnMut(&mut S, &serde_json::Value) -> Read,
    ) -> (Vec<&S>, u64) {
        let present = jsonl_under(root);
        self.by_path.retain(|p, _| present.binary_search(p).is_ok());
        for path in present {
            let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let held = self.by_path.entry(path.clone()).or_insert_with(|| Held {
                consumed: 0,
                unreadable: 0,
                state: S::default(),
            });
            if len < held.consumed {
                *held = Held { consumed: 0, unreadable: 0, state: S::default() };
            }
            if len == held.consumed {
                continue;
            }
            let (consumed, unreadable) =
                Self::fold(&path, held.consumed, &mut held.state, &mut fold);
            held.consumed = consumed;
            held.unreadable = held.unreadable.saturating_add(unreadable);
        }
        let mut unreadable = 0u64;
        let mut states = Vec::with_capacity(self.by_path.len());
        for held in self.by_path.values() {
            unreadable = unreadable.saturating_add(held.unreadable);
            states.push(&held.state);
        }
        (states, unreadable)
    }

    /// Reads `path` from `from` to the last **complete** line, folding
    /// each record in. Returns the new offset and what it could not read.
    ///
    /// **Buffered rather than read whole, and that is a size decision.**
    /// The largest transcript on this machine is 280 MB (measured
    /// 2026-08-17); slurping it would put that much on the heap to look
    /// at four numbers per line.
    ///
    /// **A line that no newline terminates is a write in progress, so it
    /// is neither counted nor consumed.** These files are appended to by
    /// a program running right now, and the honest test for "half
    /// written" is the missing terminator — not "it was the last line",
    /// which would also forgive a record the vendor finished writing and
    /// khor genuinely cannot read. Leaving it unconsumed is the same rule
    /// seen from the other side: the next pass picks it up once it is
    /// whole. Without the distinction the drift alarm would go off
    /// whenever somebody was working, and an alarm that cries wolf is
    /// worse than none.
    fn fold(
        path: &Path,
        from: u64,
        state: &mut S,
        fold: &mut impl FnMut(&mut S, &serde_json::Value) -> Read,
    ) -> (u64, u64) {
        use std::io::{BufRead, Seek};

        let mut unreadable = 0u64;
        let Ok(file) = std::fs::File::open(path) else {
            // A file khor cannot open at all is not a record it misread —
            // it may be somebody else's permissions, or a file being
            // replaced. Nothing is consumed, so a later pass may still
            // read it.
            return (from, 0);
        };
        let mut reader = std::io::BufReader::new(file);
        if from > 0 && reader.seek(std::io::SeekFrom::Start(from)).is_err() {
            return (from, 0);
        }
        let mut at = from;
        // One buffer for the whole file: a tool result can be megabytes,
        // and re-allocating per line would do that a thousand times over.
        let mut buf = String::new();
        loop {
            buf.clear();
            let read = match reader.read_line(&mut buf) {
                Ok(0) => break,
                Ok(n) => n as u64,
                // Not text at all. That is a format khor cannot read, and
                // it says so once rather than pretending the rest of the
                // file is empty.
                Err(_) => {
                    unreadable = unreadable.saturating_add(1);
                    break;
                }
            };
            if !buf.ends_with('\n') {
                break;
            }
            at += read;
            let line = buf.trim();
            if line.is_empty() {
                continue;
            }
            let outcome = match serde_json::from_str::<serde_json::Value>(line) {
                Ok(v) => fold(state, &v),
                Err(_) => Read::Unreadable,
            };
            if matches!(outcome, Read::Unreadable) {
                unreadable = unreadable.saturating_add(1);
            }
        }
        (at, unreadable)
    }
}

/// A meter's memory of files it has to read **whole**.
///
/// The opposite bookkeeping from [`Files`], and the difference is forced
/// rather than chosen: a JSON document is rewritten in place as it grows
/// — the closing bracket moves — so there is no offset to resume from and
/// no half-written line to forgive. Each pass re-reads a file whose size
/// or mtime moved and **replaces** what it knew about it.
///
/// **Replacing is the whole of it.** [`Files`] adds each pass's findings
/// to what it already had, because it only ever reads bytes it has never
/// seen. An implementation that did the same here would bill every task
/// in a file again every time anything in that file changed, and the
/// number would climb while the user did nothing. The unreadable count is
/// replaced for the same reason.
///
/// Size **and** mtime, where [`Shape`] uses size and mtime across a whole
/// tree: an edit that leaves a JSON document exactly as long as it was is
/// not what a log does, but it is precisely what rewriting one entry in
/// place looks like.
struct Whole<S> {
    by_path: HashMap<PathBuf, WholeFile<S>>,
}

/// One file as it was last read.
struct WholeFile<S> {
    len: u64,
    modified_ms: i64,
    state: S,
    unreadable: u64,
}

impl<S> Default for Whole<S> {
    fn default() -> Whole<S> {
        Whole { by_path: HashMap::new() }
    }
}

impl<S> Whole<S> {
    /// Brings every file in `present` up to date, then hands the caller
    /// each one's state. `present` must be sorted; a file that vanished
    /// takes its records with it, for [`Files::refresh`]'s reason.
    fn refresh(
        &mut self,
        present: Vec<PathBuf>,
        mut parse: impl FnMut(&Path) -> (S, u64),
    ) -> (Vec<&S>, u64) {
        self.by_path.retain(|p, _| present.binary_search(p).is_ok());
        for path in present {
            let meta = std::fs::metadata(&path).ok();
            let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified_ms = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            if let Some(had) = self.by_path.get(&path) {
                if had.len == len && had.modified_ms == modified_ms {
                    continue;
                }
            }
            let (state, unreadable) = parse(&path);
            self.by_path.insert(path, WholeFile { len, modified_ms, state, unreadable });
        }
        let mut unreadable = 0u64;
        let mut states = Vec::with_capacity(self.by_path.len());
        for held in self.by_path.values() {
            unreadable = unreadable.saturating_add(held.unreadable);
            states.push(&held.state);
        }
        (states, unreadable)
    }
}

/// Whether the cached tokens have to come out of the input column,
/// asked of the record's own total rather than assumed.
///
/// **The two vendors above it disagree about this** — claude reports
/// input already net of the cache, codex does not — so a third format
/// guessing would be a coin toss. Where a vendor publishes its own total,
/// the question is answerable per record: if the total counts the cached
/// part **separately**, then it is genuinely extra input and the input
/// column already excludes it. Anything else and it comes out.
///
/// # Why the default is to subtract
///
/// It is the only answer that cannot report the same tokens twice.
/// [`khor_core::Tokens::input`] promises input "never including what came
/// out of a cache", and it sits beside `cached_input`; leaving the cached
/// part in both columns states the same spending in two places at once,
/// which is the one direction this module may not be wrong in. Taking it
/// out when it was in fact separate loses that much from the input column
/// instead — the side khor is allowed to be wrong on.
///
/// So the witness is used to *decline* to subtract, not to permit it. No
/// total, or a total that adds up to neither form, means subtract.
///
/// Shared by [`gemini`] and [`qwen`], which is not a coincidence worth
/// hiding: Qwen Code is a fork of the Gemini CLI, so the arithmetic is
/// the same lineage even though one writes the CLI's own six fields and
/// the other writes the API's `usageMetadata` verbatim. **The shared
/// thing is the judgment, not the field names**, which is why this is one
/// function taking numbers rather than two copies taking documents.
fn cached_comes_out_of_input(
    input: u64,
    output: u64,
    cached: u64,
    thoughts: u64,
    tool: u64,
    total: Option<u64>,
) -> bool {
    if cached == 0 {
        return false;
    }
    let Some(total) = total else { return true };
    let apart = input
        .saturating_add(output)
        .saturating_add(thoughts)
        .saturating_add(tool);
    // Only one shape says the cached part was counted on its own line.
    total != apart.saturating_add(cached)
}

/// The instant a record carries, or nothing when khor cannot read it.
fn at_of(v: &serde_json::Value, key: &str) -> Option<jiff::Timestamp> {
    v.get(key).and_then(serde_json::Value::as_str)?.parse().ok()
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
    //! Each file keeps its own best-per-id and the files are merged at the
    //! end, which is how the cross-file rule survives reading one file's
    //! tail without re-reading the rest.

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{at_of, Files, Kept, Meter, Read, Tally};

    /// This vendor's name — the same string [`crate::adaptor::claude`]
    /// stamps on its rows, so a session and its tokens land in one
    /// category.
    pub const VENDOR: &str = crate::adaptor::claude::VENDOR;

    pub struct Claude {
        root: PathBuf,
        /// One entry per transcript. About 55 568 messages across this
        /// machine's, measured 2026-08-17 — a few megabytes held for as
        /// long as the node runs, which is the price of not re-reading
        /// 1482 MiB.
        files: Mutex<Files<HashMap<String, Kept>>>,
    }

    impl Claude {
        pub fn at(root: PathBuf) -> Claude {
            Claude { root, files: Mutex::new(Files::default()) }
        }

        fn projects_dir(&self) -> PathBuf {
            self.root.join("projects")
        }
    }

    /// The numbers off one `message.usage`, mapped onto the four names.
    ///
    /// **`input_tokens` is carried across unchanged**, unlike codex's:
    /// this vendor already reports it net of the cache, which is visible
    /// in the files as a two-digit input beside a five-digit cache read.
    fn tokens_of(usage: &serde_json::Value) -> Tokens {
        let n = |k: &str| usage.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        Tokens {
            input: n("input_tokens"),
            cached_input: n("cache_read_input_tokens"),
            cache_write: n("cache_creation_input_tokens"),
            output: n("output_tokens"),
        }
    }

    /// One line into one file's best-per-message map.
    fn fold(best: &mut HashMap<String, Kept>, v: &serde_json::Value) -> Read {
        if v.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
            return Read::Fine;
        }
        let Some(message) = v.get("message") else { return Read::Fine };
        let Some(usage) = message.get("usage").filter(|u| u.is_object()) else {
            return Read::Fine;
        };
        // An assistant message that bills something and says neither
        // which message it is nor when it happened cannot be
        // deduplicated or filed. That is a shape khor has stopped
        // understanding, which is what the count is for — measured zero
        // on this machine today.
        let (Some(id), Some(at)) = (
            message.get("id").and_then(serde_json::Value::as_str),
            at_of(v, "timestamp"),
        ) else {
            return Read::Unreadable;
        };
        let tokens = tokens_of(usage);
        match best.get(id) {
            // The same message written again, no further along than last
            // time.
            Some(seen) if seen.tokens.output >= tokens.output => {}
            _ => {
                best.insert(id.to_owned(), Kept { at, tokens });
            }
        }
        Read::Fine
    }

    impl Meter for Claude {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.projects_dir()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(&self.projects_dir(), fold);
            // The cross-file half of the rule: one reading per message id
            // across the whole tree, the fullest one.
            let mut best: HashMap<&str, &Kept> = HashMap::new();
            for state in states {
                for (id, kept) in state {
                    match best.get(id.as_str()) {
                        Some(seen) if seen.tokens.output >= kept.tokens.output => {}
                        _ => {
                            best.insert(id, kept);
                        }
                    }
                }
            }
            Tally { kept: best.into_values().copied().collect(), unreadable }
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
    //! is dropped. **The running total is part of what "identical" means**,
    //! and that is what makes the rule exact rather than nearly right: two
    //! genuine turns that happened to bill the same amount still differ,
    //! because the total behind them has advanced.
    //!
    //! **The rule is checked against something khor did not compute**: in
    //! the 69 sessions whose total never reset, the sum of the
    //! deduplicated turns equals the session's own final
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
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{at_of, Files, Kept, Meter, Read, Tally};

    /// This vendor's name — the same string
    /// [`crate::adaptor::codex`] stamps on its rows.
    pub const VENDOR: &str = crate::adaptor::codex::VENDOR;

    /// One rollout's turns, and the reading that came just before, so that
    /// the repeat rule survives a pass that only read the file's tail.
    #[derive(Default)]
    pub struct Rollout {
        turns: Vec<Kept>,
        previous: Option<(Tokens, u64)>,
    }

    pub struct Codex {
        root: PathBuf,
        files: Mutex<Files<Rollout>>,
    }

    impl Codex {
        pub fn at(root: PathBuf) -> Codex {
            Codex { root, files: Mutex::new(Files::default()) }
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

    fn fold(roll: &mut Rollout, v: &serde_json::Value) -> Read {
        let Some(payload) = v.get("payload") else { return Read::Fine };
        if payload.get("type").and_then(serde_json::Value::as_str) != Some("token_count") {
            return Read::Fine;
        }
        // `info: null` is codex saying it has no numbers for this event —
        // a rate-limit notice rides the same event type. Nothing to read,
        // nothing missed.
        let Some(info) = payload.get("info").filter(|i| i.is_object()) else {
            return Read::Fine;
        };
        // A token_count that reports no turn, or one khor cannot place in
        // time, is a shape it has stopped understanding.
        let Some(last) = info.get("last_token_usage").filter(|u| u.is_object()) else {
            return Read::Unreadable;
        };
        let Some(at) = at_of(v, "timestamp") else {
            return Read::Unreadable;
        };
        let running = info
            .get("total_token_usage")
            .and_then(|t| t.get("total_tokens"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let tokens = tokens_of(last);
        if roll.previous == Some((tokens, running)) {
            return Read::Fine;
        }
        roll.previous = Some((tokens, running));
        roll.turns.push(Kept { at, tokens });
        Read::Fine
    }

    impl Meter for Codex {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.sessions_dir()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(&self.sessions_dir(), fold);
            let kept = states.into_iter().flat_map(|r| r.turns.iter().copied()).collect();
            Tally { kept, unreadable }
        }
    }
}

pub mod gemini {
    //! Gemini CLI: one answer per record, written twice, and the vendor's
    //! own total says how to read it.
    //!
    //! A chat recording under `tmp/<project>/chats/` is one JSON object per
    //! line: a header, `$set` updates to it, the user's turns, and the
    //! answers. Only an answer (`"type": "gemini"`) carries `tokens`, and
    //! that object is six numbers — `input`, `output`, `cached`,
    //! `thoughts`, `tool`, `total`.
    //!
    //! # `cached` sits inside `input`, and the file says so rather than
    //! this module assuming it
    //!
    //! The two vendors above disagree about this (claude reports input net
    //! of the cache, codex does not), so a third one guessing would be a
    //! coin toss. Gemini publishes its own `total`, which makes the
    //! question answerable per record — and the rule is spelled out on
    //! [`super::cached_comes_out_of_input`]: the cached part comes out of
    //! input **unless** the total shows it counted on its own line.
    //!
    //! Measured 2026-08-17 on this machine's two recordings, 106 records:
    //! the "already inside" form held in **106 of 106**, and
    //! `cached <= input` in all of them. The other branch is upstream
    //! knowledge (`tokscale`'s
    //! `normalize_gemini_session_input_and_cache` carries the same test)
    //! and **is not exercised by anything on this machine** — it is here
    //! because guessing the other way would silently halve a real answer.
    //!
    //! # `thoughts` is output; `tool` is input
    //!
    //! `thoughts` is added to output on the rule
    //! [`khor_core::Tokens::output`] already states — reasoning is output
    //! wherever a vendor separates the two — and gemini plainly separates
    //! them: its own total counts `thoughts` **beside** `output` rather
    //! than inside it. That is the opposite finding from codex, where
    //! `reasoning_output_tokens <= output_tokens` always, and the two are
    //! reconciled by asking the vendor's arithmetic instead of the field
    //! name.
    //!
    //! `tool` is added to input, which is what `tokscale` does. **On this
    //! machine it is zero in all 106 records**, so that placement is
    //! borrowed and unverified; what is not a guess is that it must go
    //! somewhere, because gemini's total counts it.
    //!
    //! # What khor does not read here
    //!
    //! Only the spelling above. `tokscale` also accepts `prompt`,
    //! `promptTokenCount`, `candidatesTokenCount`,
    //! `cachedContentTokenCount` and friends for older layouts and for the
    //! API's own shape — none of which occur on this machine, so reading
    //! them here would be arithmetic nobody could test. The drift alarm
    //! covers the gap loudly instead: a record whose own `total` says
    //! tokens were spent while every name khor knows reads zero is counted
    //! [`Read::Unreadable`] rather than billed as nothing.

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{at_of, cached_comes_out_of_input, Files, Kept, Meter, Read, Tally};

    /// This vendor's name.
    ///
    /// **Declared here rather than borrowed from an adaptor, because there
    /// is no gemini adaptor**: a row in the session list has to be backed
    /// by a live process, and what this machine holds is two finished
    /// recordings from 2026-05-15 — enough to bill, and nothing to list.
    /// Spending needs no live process at all (see this module's parent).
    /// The direction is the only thing that differs from
    /// [`super::claude::VENDOR`] — a gemini adaptor, when there is one,
    /// must take this string rather than spell its own, or one agent lands
    /// in two categories on two screens with nothing reporting an error.
    pub const VENDOR: &str = "gemini";

    pub struct Gemini {
        root: PathBuf,
        files: Mutex<Files<HashMap<String, Kept>>>,
    }

    impl Gemini {
        pub fn at(root: PathBuf) -> Gemini {
            Gemini { root, files: Mutex::new(Files::default()) }
        }

        fn chats_dir(&self) -> PathBuf {
            self.root.join("tmp")
        }
    }

    /// The six numbers off one `tokens`, mapped onto the four khor bills,
    /// or nothing when this is a shape khor has stopped understanding.
    fn tokens_of(t: &serde_json::Value) -> Option<Tokens> {
        let n = |k: &str| t.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        let (input, output, cached, thoughts, tool) =
            (n("input"), n("output"), n("cached"), n("thoughts"), n("tool"));
        let total = t.get("total").and_then(serde_json::Value::as_u64);
        // The drift alarm, and it is worth the four extra comparisons:
        // gemini renaming its fields would otherwise show up as a machine
        // that ran an agent all day and spent nothing, which is a lie no
        // test can see. Its own total is the one number that catches it.
        let silent = input == 0 && output == 0 && cached == 0 && thoughts == 0 && tool == 0;
        if silent && total.is_some_and(|t| t > 0) {
            return None;
        }
        let fresh = if cached_comes_out_of_input(input, output, cached, thoughts, tool, total) {
            input.saturating_sub(cached)
        } else {
            input
        };
        Some(Tokens {
            input: fresh.saturating_add(tool),
            cached_input: cached,
            // Gemini reports no cache creation of its own — there is no
            // number here to leave out.
            cache_write: 0,
            output: output.saturating_add(thoughts),
        })
    }

    /// One line into one file's best-per-answer map.
    fn fold(best: &mut HashMap<String, Kept>, v: &serde_json::Value) -> Read {
        if v.get("type").and_then(serde_json::Value::as_str) != Some("gemini") {
            return Read::Fine;
        }
        // An answer with no `tokens` at all is a message, not a billing
        // record khor failed to read — the same call claude's meter makes
        // about an assistant message carrying no `usage`.
        let Some(tokens) = v.get("tokens").filter(|t| t.is_object()) else {
            return Read::Fine;
        };
        let (Some(id), Some(at)) = (
            v.get("id").and_then(serde_json::Value::as_str),
            at_of(v, "timestamp"),
        ) else {
            return Read::Unreadable;
        };
        let Some(tokens) = tokens_of(tokens) else {
            return Read::Unreadable;
        };
        match best.get(id) {
            // The same answer written again, no further along than last
            // time. **On this machine the repeats are byte-identical** (0
            // exceptions in 48 repeated ids), so "the largest" is
            // inherited from claude's rule rather than demonstrated here;
            // it is the safe direction if gemini ever streams a partial,
            // and a no-op otherwise.
            Some(seen) if seen.tokens.output >= tokens.output => {}
            _ => {
                best.insert(id.to_owned(), Kept { at, tokens });
            }
        }
        Read::Fine
    }

    impl Meter for Gemini {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.chats_dir()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(&self.chats_dir(), fold);
            // Across files as well as within one, for claude's reason: a
            // resumed conversation that copied its history would
            // otherwise be billed twice. Nothing on this machine resumes
            // that way, so this costs one merge and rules the case out.
            let mut best: HashMap<&str, &Kept> = HashMap::new();
            for state in states {
                for (id, kept) in state {
                    match best.get(id.as_str()) {
                        Some(seen) if seen.tokens.output >= kept.tokens.output => {}
                        _ => {
                            best.insert(id, kept);
                        }
                    }
                }
            }
            Tally { kept: best.into_values().copied().collect(), unreadable }
        }
    }
}

pub mod pi {
    //! The Pi format, and the three agents that write it.
    //!
    //! `badlogic/pi-mono` publishes a session format that its descendants
    //! kept: one JSONL file per session, a `{"type":"session"}` header
    //! carrying the session id, then `{"type":"message"}` entries whose
    //! `message.usage` is already spelled in khor's own four names —
    //! `input`, `output`, `cacheRead`, `cacheWrite`. Nothing has to be
    //! subtracted or added; of the three vendors above, only this one
    //! needed no arithmetic at all.
    //!
    //! # Three vendors, one reader, and why that is not a shortcut
    //!
    //! Pi, Senpi (OmO Native) and Kimchi differ in **where** they keep
    //! their sessions and in nothing else that matters here — upstream
    //! says so by having two of the three delegate to the first one's
    //! parser outright. So the vendor name and the root are the parameters
    //! and the reading is shared, which also means a fixture proves the
    //! reading for all three at once. What a shared reader must not do is
    //! blur *whose* tokens these were: each root gets its own meter and
    //! stamps its own name, because a row in the spending list saying
    //! `pi` when the tokens were Kimchi's is a wrong answer that looks
    //! right.
    //!
    //! # What is verified here and what is borrowed
    //!
    //! **Upstream knowledge, not measurement.** This machine has exactly
    //! one Pi session (`~/.pi/agent/sessions`, 2026-06-27) and **every
    //! number in it is zero** — a call that failed before it billed
    //! anything. So it pins the *shape* (khor reads that file without
    //! counting it unreadable) and pins **nothing about the arithmetic**:
    //! an implementation that read no numbers at all would agree with it.
    //! The numbers below are held by the fixture alone, and the two other
    //! roots have never existed on this machine.
    //!
    //! That is the honest half of the trade the ledger asks for: being
    //! wrong here makes khor read *less* than was spent, never more,
    //! because a shape khor does not recognise is counted
    //! ([`Read::Unreadable`]) rather than billed as zero.
    //!
    //! # Two places khor deliberately differs from upstream
    //!
    //! - **A record with no model is still counted.** Upstream drops it,
    //!   because it prices tokens per model and cannot price what it
    //!   cannot name. khor bills no money and never reads the model, so
    //!   dropping the record would throw away tokens somebody spent for a
    //!   reason khor does not have.
    //! - **Deduplication is scoped to the session**, which is upstream's
    //!   default (`parse_pi_format_file`). Message ids in this format are
    //!   per-session — the upstream tests use `msg_001` — so merging on a
    //!   bare id across files would collapse two different agents' first
    //!   answers into one. Upstream lets one client opt into a
    //!   cross-session key; khor waits for a real sample that needs it
    //!   rather than guessing which way the collision goes.

    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{at_of, Files, Kept, Meter, Read, Tally};

    /// Every agent known to write this format, and where each keeps its
    /// sessions, relative to the home khor was rooted at.
    ///
    /// A table rather than three modules because the difference between
    /// these three genuinely is two strings — and the moment one of them
    /// needs a fourth, it stops being a row here and becomes its own
    /// meter, the same way `gemini` is.
    pub const ROOTS: [(&str, &str); 3] = [
        ("pi", ".pi/agent/sessions"),
        ("senpi", ".senpi/agent/sessions"),
        ("kimchi", ".config/kimchi/harness/sessions"),
    ];

    /// One session file's answers, and the session they belong to.
    #[derive(Default)]
    pub struct Session {
        /// From the header line. Kept in the state rather than re-read,
        /// because a later pass reads only the file's tail and the header
        /// is long behind it.
        id: Option<String>,
        best: HashMap<String, Kept>,
    }

    pub struct PiFormat {
        vendor: &'static str,
        root: PathBuf,
        files: Mutex<Files<Session>>,
    }

    impl PiFormat {
        pub fn at(vendor: &'static str, root: PathBuf) -> PiFormat {
            PiFormat { vendor, root, files: Mutex::new(Files::default()) }
        }
    }

    /// The four names off one `usage`, or nothing when this is a shape
    /// khor has stopped understanding.
    ///
    /// `reasoning` is read by upstream and deliberately not added: this
    /// format documents it as a **subset of** `output`, which is the same
    /// finding codex gives and the opposite of gemini's. Each vendor is
    /// asked rather than assumed.
    fn tokens_of(usage: &serde_json::Value) -> Option<Tokens> {
        let n = |k: &str| usage.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        let (input, output, cache_read, cache_write) =
            (n("input"), n("output"), n("cacheRead"), n("cacheWrite"));
        // The same drift alarm gemini gets, on this format's own total:
        // a record that says it spent something while every name khor
        // knows reads zero is a renamed field, not a free answer.
        let silent = input == 0 && output == 0 && cache_read == 0 && cache_write == 0;
        let total = usage.get("totalTokens").and_then(serde_json::Value::as_u64);
        if silent && total.is_some_and(|t| t > 0) {
            return None;
        }
        Some(Tokens {
            input,
            cached_input: cache_read,
            cache_write,
            output,
        })
    }

    fn fold(session: &mut Session, v: &serde_json::Value) -> Read {
        match v.get("type").and_then(serde_json::Value::as_str) {
            Some("session") => {
                // The header, and the only line that says which session
                // this file is. A file without one is still read; it just
                // never merges with another (see `tally`).
                if let Some(id) = v.get("id").and_then(serde_json::Value::as_str) {
                    session.id = Some(id.to_owned());
                }
                return Read::Fine;
            }
            Some("message") => {}
            _ => return Read::Fine,
        }
        let Some(message) = v.get("message") else { return Read::Fine };
        if message.get("role").and_then(serde_json::Value::as_str) != Some("assistant") {
            return Read::Fine;
        }
        // An assistant turn that bills nothing is a turn, not a failure:
        // this format writes user and tool records through the same door.
        let Some(usage) = message.get("usage").filter(|u| u.is_object()) else {
            return Read::Fine;
        };
        let (Some(id), Some(at)) = (
            v.get("id").and_then(serde_json::Value::as_str),
            at_of(v, "timestamp"),
        ) else {
            return Read::Unreadable;
        };
        let Some(tokens) = tokens_of(usage) else {
            return Read::Unreadable;
        };
        match session.best.get(id) {
            Some(seen) if seen.tokens.output >= tokens.output => {}
            _ => {
                session.best.insert(id.to_owned(), Kept { at, tokens });
            }
        }
        Read::Fine
    }

    impl Meter for PiFormat {
        fn vendor(&self) -> &'static str {
            self.vendor
        }

        fn root(&self) -> PathBuf {
            self.root.clone()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(&self.root, fold);
            // Session-scoped, so a history copied into a second file is
            // billed once — and a file that never said which session it
            // is merges with nothing, rather than merging on an id this
            // format does not promise to be unique.
            let mut best: HashMap<String, &Kept> = HashMap::new();
            let mut loose: Vec<Kept> = Vec::new();
            for state in states {
                let Some(session) = state.id.as_deref() else {
                    loose.extend(state.best.values().copied());
                    continue;
                };
                for (id, kept) in &state.best {
                    let key = format!("{session}/{id}");
                    match best.get(&key) {
                        Some(seen) if seen.tokens.output >= kept.tokens.output => {}
                        _ => {
                            best.insert(key, kept);
                        }
                    }
                }
            }
            loose.extend(best.into_values().copied());
            Tally { kept: loose, unreadable }
        }
    }
}

pub mod roo {
    //! Roo Code, Cline and Kilo Code: one task per directory, one JSON
    //! document per task.
    //!
    //! These three are VS Code extensions rather than command-line agents,
    //! and they keep `tasks/<taskId>/ui_messages.json` — an **array**, not
    //! a log, holding every message the panel drew. A request khor can
    //! bill is an entry with `"type": "say"` and `"say":
    //! "api_req_started"`, whose `text` is **itself a JSON document, as a
    //! string**, carrying `tokensIn` / `tokensOut` / `cacheReads` /
    //! `cacheWrites`. Those four map onto khor's four with no arithmetic.
    //!
    //! # Why this one needed new machinery
    //!
    //! Everything above it here reads append-only logs and remembers how
    //! far it got ([`Files`]). A JSON array is rewritten in place as it
    //! grows, so there is nothing to resume from — hence [`Whole`], which
    //! re-reads a changed file and **replaces** what it knew. That is also
    //! why nothing here deduplicates: each `api_req_started` is one
    //! request, and a second reading of the file replaces the first rather
    //! than adding to it. **Those two facts hold each other up** — reading
    //! whole while accumulating would bill every task again on every
    //! change.
    //!
    //! # Twelve roots, three names
    //!
    //! Each extension keeps its tasks under VS Code's `globalStorage`,
    //! which sits in a different place on every platform and in a fifth
    //! for a remote server. khor registers one meter per (extension,
    //! location) and lets them all stamp their own three names, so a
    //! machine that has the same extension locally and over `vscode-server`
    //! has both counted without either root knowing about the other.
    //! Directories that are not there cost one failed `read_dir`.
    //!
    //! # Upstream knowledge, and thinner than the others
    //!
    //! **No sample of any of these three has ever been on this machine**,
    //! and the fixture is upstream's own test document copied across
    //! rather than one written from reading their parser — a fixture
    //! derived from the same misreading as the implementation would agree
    //! with it forever.
    //!
    //! This format also gives khor **less to check itself against than any
    //! other**: there is no per-record total, so the trick gemini and the
    //! Pi format allow — asking the vendor's own sum whether khor read the
    //! fields right — is not available. The drift alarm here is therefore
    //! **presence, not arithmetic**: an `api_req_started` payload khor can
    //! parse in which *none* of the four names appears is counted
    //! [`Read::Unreadable`], because a renamed field would otherwise read
    //! as a request that cost nothing. A payload with some of them and not
    //! others is taken at face value, which is the one gap left open here.

    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{named_under, Kept, Meter, Tally, Whole};

    /// The three extensions, and the id each one stores under.
    pub const EXTENSIONS: [(&str, &str); 3] = [
        ("roocode", "rooveterinaryinc.roo-cline"),
        ("cline", "saoudrizwan.claude-dev"),
        ("kilocode", "kilocode.kilo-code"),
    ];

    /// Where VS Code keeps `globalStorage`, per platform, relative to the
    /// home khor was rooted at. All of them are looked in on every
    /// platform: the cost of a wrong guess is one `read_dir` that fails,
    /// and the cost of guessing right only for this machine is a user on
    /// another one whose tokens quietly do not exist.
    pub const STORAGES: [&str; 4] = [
        "Library/Application Support/Code/User/globalStorage",
        ".config/Code/User/globalStorage",
        "AppData/Roaming/Code/User/globalStorage",
        ".vscode-server/data/User/globalStorage",
    ];

    /// The file inside a task directory that holds the requests. The
    /// directory holds others (`api_conversation_history.json` among
    /// them), so the name is the whole match.
    pub const LOG: &str = "ui_messages.json";

    /// Every (vendor, root) this format is read from, relative to a home.
    pub fn roots() -> Vec<(&'static str, PathBuf)> {
        let mut out = Vec::with_capacity(EXTENSIONS.len() * STORAGES.len());
        for (vendor, extension) in EXTENSIONS {
            for storage in STORAGES {
                out.push((vendor, PathBuf::from(storage).join(extension).join("tasks")));
            }
        }
        out
    }

    pub struct Roo {
        vendor: &'static str,
        root: PathBuf,
        files: Mutex<Whole<Vec<Kept>>>,
    }

    impl Roo {
        pub fn at(vendor: &'static str, root: PathBuf) -> Roo {
            Roo { vendor, root, files: Mutex::new(Whole::default()) }
        }
    }

    /// The four numbers off one `api_req_started` payload, or nothing when
    /// it names none of them. See the module head on why presence is the
    /// test here and arithmetic is elsewhere.
    fn tokens_of(payload: &serde_json::Value) -> Option<Tokens> {
        let n = |k: &str| payload.get(k).and_then(serde_json::Value::as_u64);
        let (input, output, cache_read, cache_write) =
            (n("tokensIn"), n("tokensOut"), n("cacheReads"), n("cacheWrites"));
        if input.is_none() && output.is_none() && cache_read.is_none() && cache_write.is_none() {
            return None;
        }
        Some(Tokens {
            input: input.unwrap_or(0),
            cached_input: cache_read.unwrap_or(0),
            cache_write: cache_write.unwrap_or(0),
            output: output.unwrap_or(0),
        })
    }

    /// The entry's clock, which this format writes either as a time or as
    /// milliseconds since the epoch — both occur in upstream's own tests.
    fn instant_of(ts: Option<&serde_json::Value>) -> Option<jiff::Timestamp> {
        let ts = ts?;
        if let Some(text) = ts.as_str() {
            return text.parse().ok();
        }
        jiff::Timestamp::from_millisecond(ts.as_i64()?).ok()
    }

    /// One task's whole document.
    fn parse(path: &Path) -> (Vec<Kept>, u64) {
        let mut kept = Vec::new();
        let Ok(text) = std::fs::read_to_string(path) else {
            // Unopenable is not misread, the same call `Files::fold`
            // makes: nothing is remembered, so a later pass may still get
            // it.
            return (kept, 0);
        };
        let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
            // **One count for the document, not one per entry.** khor
            // cannot see the entries to count them, and a number that
            // depended on how big the unreadable thing was would tell
            // nobody anything.
            return (kept, 1);
        };
        let mut unreadable = 0u64;
        for entry in entries {
            let say = |k: &str| entry.get(k).and_then(serde_json::Value::as_str);
            if say("type") != Some("say") || say("say") != Some("api_req_started") {
                continue;
            }
            // From here down every failure is a request that plainly
            // billed something and khor could not read — which is the
            // opposite of upstream, where a malformed payload is skipped
            // in silence. Silence is the one thing this count exists to
            // prevent.
            let Some(payload) = say("text")
                .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
            else {
                unreadable += 1;
                continue;
            };
            let (Some(tokens), Some(at)) = (tokens_of(&payload), instant_of(entry.get("ts")))
            else {
                unreadable += 1;
                continue;
            };
            kept.push(Kept { at, tokens });
        }
        (kept, unreadable)
    }

    impl Meter for Roo {
        fn vendor(&self) -> &'static str {
            self.vendor
        }

        fn root(&self) -> PathBuf {
            self.root.clone()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(named_under(&self.root, LOG), parse);
            let kept = states.into_iter().flat_map(|s| s.iter().copied()).collect();
            Tally { kept, unreadable }
        }
    }
}

pub mod qwen {
    //! Qwen Code: the Gemini CLI's cousin, writing the API's own
    //! `usageMetadata` instead of the CLI's six fields.
    //!
    //! One JSONL file per chat under `.qwen/projects/<project>/chats/`,
    //! and a spendable line is `"type": "assistant"` carrying
    //! `usageMetadata` — `promptTokenCount`, `candidatesTokenCount`,
    //! `thoughtsTokenCount`, `cachedContentTokenCount`.
    //!
    //! # The one question this format asks, and where the answer came from
    //!
    //! Is `cachedContentTokenCount` already inside `promptTokenCount`?
    //! **Upstream does not subtract it.** khor does, unless the record's
    //! own `totalTokenCount` shows the cached part counted on its own
    //! line — [`super::cached_comes_out_of_input`], the same rule gemini
    //! uses, because Qwen Code is a fork of the Gemini CLI.
    //!
    //! **This is a deliberate divergence from upstream and it is worth
    //! being explicit about why**, because there is no sample of this
    //! vendor on this machine to settle it:
    //!
    //! - The sibling CLI's real recordings **were** settled here: in all
    //!   106 of them the total omitted the cached part, so cached sat
    //!   inside input.
    //! - Following upstream would leave those tokens in the input column
    //!   while also reporting them in `cached_input` — the same spending
    //!   in two places, which is the one direction this tier is not
    //!   allowed to be wrong in.
    //! - So the absent-total case subtracts too. Losing a genuinely
    //!   separate cache read out of the input column is an under-count,
    //!   and that is the side khor is allowed to be wrong on.
    //!
    //! `thoughtsTokenCount` is added to output on the rule
    //! [`khor_core::Tokens::output`] states, the same as gemini and for
    //! the same reason: this family counts thinking beside the answer
    //! rather than inside it. Cache **writes** this format does not report
    //! at all.

    use std::path::PathBuf;
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{at_of, cached_comes_out_of_input, Files, Kept, Meter, Read, Tally};

    pub const VENDOR: &str = "qwen";

    pub struct Qwen {
        root: PathBuf,
        files: Mutex<Files<Vec<Kept>>>,
    }

    impl Qwen {
        pub fn at(root: PathBuf) -> Qwen {
            Qwen { root, files: Mutex::new(Files::default()) }
        }

        fn projects_dir(&self) -> PathBuf {
            self.root.join("projects")
        }
    }

    /// The API's usage object, mapped onto khor's four.
    fn tokens_of(usage: &serde_json::Value) -> Option<Tokens> {
        let n = |k: &str| usage.get(k).and_then(serde_json::Value::as_u64);
        let prompt = n("promptTokenCount");
        let candidates = n("candidatesTokenCount");
        let thoughts = n("thoughtsTokenCount");
        let cached = n("cachedContentTokenCount");
        // Presence, not value: a record naming none of these is a shape
        // that moved, and the total alone cannot tell khor what the parts
        // were.
        if prompt.is_none() && candidates.is_none() && thoughts.is_none() && cached.is_none() {
            return None;
        }
        let (prompt, candidates, thoughts, cached) = (
            prompt.unwrap_or(0),
            candidates.unwrap_or(0),
            thoughts.unwrap_or(0),
            cached.unwrap_or(0),
        );
        let total = n("totalTokenCount");
        let fresh = if cached_comes_out_of_input(prompt, candidates, cached, thoughts, 0, total) {
            prompt.saturating_sub(cached)
        } else {
            prompt
        };
        Some(Tokens {
            input: fresh,
            cached_input: cached,
            cache_write: 0,
            output: candidates.saturating_add(thoughts),
        })
    }

    /// One line onto the end of this file's records.
    ///
    /// **Nothing deduplicates here, and it is the reader that makes that
    /// safe**: [`Files`] hands each line over exactly once, so a record
    /// cannot arrive twice. Upstream needs a key because it re-parses
    /// whole files.
    fn fold(kept: &mut Vec<Kept>, v: &serde_json::Value) -> Read {
        if v.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
            return Read::Fine;
        }
        let Some(usage) = v.get("usageMetadata").filter(|u| u.is_object()) else {
            return Read::Fine;
        };
        let (Some(tokens), Some(at)) = (tokens_of(usage), at_of(v, "timestamp")) else {
            return Read::Unreadable;
        };
        kept.push(Kept { at, tokens });
        Read::Fine
    }

    impl Meter for Qwen {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.projects_dir()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(&self.projects_dir(), fold);
            Tally {
                kept: states.into_iter().flat_map(|k| k.iter().copied()).collect(),
                unreadable,
            }
        }
    }
}

pub mod junie {
    //! Junie: an event log, where one event can bill several models.
    //!
    //! `.junie/sessions/<session>/events.jsonl`, one JSON object per line.
    //! The ones that cost anything carry
    //! `event.agentEvent.kind == "LlmResponseMetadataEvent"` and a
    //! **`modelUsage` array** — one row per model the turn used, each with
    //! `inputTokens` and `outputTokens`.
    //!
    //! The array is the whole of what is interesting here. Every other
    //! format above bills one record per line; this one bills as many as
    //! the line says, so a reader that took the first row would quietly
    //! undercount a turn that used two models — and quietly is the word,
    //! because the answer would still look like a plausible number.
    //!
    //! **Two fields only.** This format reports no cache reads and no
    //! cache writes, so both are zero — not because khor could not find
    //! them but because nothing writes them, which is the same call
    //! `gemini` makes about cache writes.
    //!
    //! Upstream knowledge, fixture-driven: no Junie has ever run on this
    //! machine, and the fixture is upstream's own test event carried
    //! across.

    use std::path::PathBuf;
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{Files, Kept, Meter, Read, Tally};

    pub const VENDOR: &str = "junie";

    pub struct Junie {
        root: PathBuf,
        files: Mutex<Files<Vec<Kept>>>,
    }

    impl Junie {
        pub fn at(root: PathBuf) -> Junie {
            Junie { root, files: Mutex::new(Files::default()) }
        }

        fn sessions_dir(&self) -> PathBuf {
            self.root.join("sessions")
        }
    }

    fn tokens_of(row: &serde_json::Value) -> Option<Tokens> {
        let n = |k: &str| row.get(k).and_then(serde_json::Value::as_u64);
        let (input, output) = (n("inputTokens"), n("outputTokens"));
        if input.is_none() && output.is_none() {
            return None;
        }
        Some(Tokens {
            input: input.unwrap_or(0),
            cached_input: 0,
            cache_write: 0,
            output: output.unwrap_or(0),
        })
    }

    fn fold(kept: &mut Vec<Kept>, v: &serde_json::Value) -> Read {
        let Some(event) = v.pointer("/event/agentEvent") else { return Read::Fine };
        if event.get("kind").and_then(serde_json::Value::as_str)
            != Some("LlmResponseMetadataEvent")
        {
            return Read::Fine;
        }
        let Some(rows) = event.get("modelUsage").and_then(|u| u.as_array()) else {
            return Read::Unreadable;
        };
        // This format stamps the event, not the row, so every model the
        // turn used shares one instant. That is the vendor's own answer to
        // "when", and inventing a spread would be khor making one up.
        let at = v
            .get("timestampMs")
            .and_then(serde_json::Value::as_i64)
            .and_then(|ms| jiff::Timestamp::from_millisecond(ms).ok());
        let Some(at) = at else { return Read::Unreadable };
        let mut missed = false;
        for row in rows {
            match tokens_of(row) {
                Some(tokens) => kept.push(Kept { at, tokens }),
                None => missed = true,
            }
        }
        if missed {
            Read::Unreadable
        } else {
            Read::Fine
        }
    }

    impl Meter for Junie {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.sessions_dir()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(&self.sessions_dir(), fold);
            Tally {
                kept: states.into_iter().flat_map(|k| k.iter().copied()).collect(),
                unreadable,
            }
        }
    }
}

pub mod amp {
    //! Amp (Sourcegraph): one thread per JSON document, and **the same
    //! spending written down twice inside it**.
    //!
    //! A thread under `.local/share/amp/threads/` carries two accounts of
    //! what it cost: a `usageLedger.events[]` — the ledger proper, with
    //! its own timestamps — and a `usage` object on each assistant entry
    //! of `messages[]`. They overlap, and how much they overlap is not
    //! fixed: an event may point at a message (`toMessageId`), or match
    //! one only by having the same model and the same numbers, or answer
    //! to nothing at all.
    //!
    //! # Reading both naively is the one mistake this whole module exists
    //! to avoid
    //!
    //! Adding the two accounts together bills every answer twice. Reading
    //! only the messages loses whatever the ledger knows that they do not.
    //! So the rule is upstream's, ported rather than invented:
    //!
    //! 1. The **ledger is the account**. Every event is a record.
    //! 2. Each message is matched against an unconsumed event — first by
    //!    `toMessageId`, then by same model and identical numbers — and a
    //!    matched message adds **nothing**; it only lends its clock to an
    //!    event that had none.
    //! 3. A message that matches no event **is** added: the ledger did not
    //!    know about it.
    //! 4. A thread with no ledger at all is read from its messages.
    //!
    //! The matching walks forward from the last match and wraps, so two
    //! answers that billed identical numbers consume two different events
    //! rather than the same one twice.
    //!
    //! # What khor does differently, and which way it errs
    //!
    //! A message's own clock is **derived**, not written: the thread's
    //! `created` plus the message's id in seconds, which is upstream's
    //! construction and is only ever used to place a record on a day. A
    //! record khor cannot place in time at all — no event timestamp, no
    //! thread `created` — is counted [`Read::Unreadable`] rather than
    //! filed under the file's mtime as upstream does. **A file's mtime is
    //! not a fact about when tokens were spent**, and this module has no
    //! business inventing one.
    //!
    //! Upstream knowledge, fixture-driven: no Amp thread has ever been on
    //! this machine, and the fixture is built from upstream's own test
    //! documents.

    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use khor_core::Tokens;

    use super::{json_under, Kept, Meter, Tally, Whole};

    pub const VENDOR: &str = "amp";

    pub struct Amp {
        root: PathBuf,
        files: Mutex<Whole<Vec<Kept>>>,
    }

    impl Amp {
        pub fn at(root: PathBuf) -> Amp {
            Amp { root, files: Mutex::new(Whole::default()) }
        }

        fn threads_dir(&self) -> PathBuf {
            self.root.join("threads")
        }
    }

    /// One account of one answer, before the two accounts are reconciled.
    #[derive(Clone)]
    struct Record {
        model: String,
        at: Option<jiff::Timestamp>,
        /// Which message this event says it belongs to, if it says.
        to_message: Option<i64>,
        /// Which message this is, for a record read off `messages[]`.
        message: Option<i64>,
        tokens: Tokens,
    }

    fn ms(v: Option<&serde_json::Value>) -> Option<i64> {
        v.and_then(serde_json::Value::as_i64).filter(|ms| *ms != 0)
    }

    fn at_of_ms(ms: i64) -> Option<jiff::Timestamp> {
        jiff::Timestamp::from_millisecond(ms).ok()
    }

    /// The ledger's own spelling of the four counts.
    fn ledger_tokens(t: Option<&serde_json::Value>) -> Tokens {
        let n = |k: &str| {
            t.and_then(|t| t.get(k)).and_then(serde_json::Value::as_u64).unwrap_or(0)
        };
        Tokens {
            input: n("input"),
            cached_input: n("cacheReadInputTokens"),
            cache_write: n("cacheCreationInputTokens"),
            output: n("output"),
        }
    }

    /// A message's spelling of the same four, which is not the ledger's.
    fn message_tokens(u: &serde_json::Value) -> Tokens {
        let n = |k: &str| u.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        Tokens {
            input: n("inputTokens"),
            cached_input: n("cacheReadInputTokens"),
            cache_write: n("cacheCreationInputTokens"),
            output: n("outputTokens"),
        }
    }

    fn ledger_records(thread: &serde_json::Value, created: Option<i64>) -> (Vec<Record>, u64) {
        let mut out = Vec::new();
        let mut unreadable = 0;
        let events = thread
            .pointer("/usageLedger/events")
            .and_then(|e| e.as_array())
            .map(Vec::as_slice)
            .unwrap_or_default();
        for event in events {
            let Some(model) = event.get("model").and_then(serde_json::Value::as_str) else {
                unreadable += 1;
                continue;
            };
            let at = event
                .get("timestamp")
                .and_then(serde_json::Value::as_str)
                .and_then(|s| s.parse::<jiff::Timestamp>().ok())
                .or_else(|| created.and_then(at_of_ms));
            if at.is_none() {
                unreadable += 1;
                continue;
            }
            out.push(Record {
                model: model.to_owned(),
                at,
                to_message: event.get("toMessageId").and_then(serde_json::Value::as_i64),
                message: None,
                tokens: ledger_tokens(event.get("tokens")),
            });
        }
        (out, unreadable)
    }

    fn message_records(thread: &serde_json::Value, created: Option<i64>) -> (Vec<Record>, u64) {
        let mut out = Vec::new();
        let mut unreadable = 0;
        let messages = thread
            .get("messages")
            .and_then(|m| m.as_array())
            .map(Vec::as_slice)
            .unwrap_or_default();
        for message in messages {
            if message.get("role").and_then(serde_json::Value::as_str) != Some("assistant") {
                continue;
            }
            let Some(usage) = message.get("usage").filter(|u| u.is_object()) else {
                continue;
            };
            let Some(model) = usage.get("model").and_then(serde_json::Value::as_str) else {
                unreadable += 1;
                continue;
            };
            let id = message.get("messageId").and_then(serde_json::Value::as_i64).unwrap_or(0);
            // Upstream's construction: the thread's own start, plus this
            // message's place in it. Only ever used to pick a day.
            let Some(at) = created.and_then(|c| at_of_ms(c.saturating_add(id.saturating_mul(1000))))
            else {
                unreadable += 1;
                continue;
            };
            out.push(Record {
                model: model.to_owned(),
                at: Some(at),
                to_message: None,
                message: Some(id).filter(|id| *id > 0),
                tokens: message_tokens(usage),
            });
        }
        (out, unreadable)
    }

    /// The event this message is another account of, if there is one.
    ///
    /// Forward from the last match and then wrapping, which is upstream's
    /// order and is what keeps two answers that billed the same numbers
    /// from both claiming the same event.
    fn matching(
        events: &[Record],
        consumed: &[bool],
        from: usize,
        message: &Record,
    ) -> Option<usize> {
        let scan = |pick: &dyn Fn(usize) -> bool| {
            (from..events.len()).find(|i| pick(*i)).or_else(|| (0..from).find(|i| pick(*i)))
        };
        if let Some(id) = message.message {
            if let Some(i) = scan(&|i| !consumed[i] && events[i].to_message == Some(id)) {
                return Some(i);
            }
        }
        scan(&|i| {
            !consumed[i] && events[i].model == message.model && events[i].tokens == message.tokens
        })
    }

    fn parse(path: &Path) -> (Vec<Kept>, u64) {
        let Ok(text) = std::fs::read_to_string(path) else {
            return (Vec::new(), 0);
        };
        let Ok(thread) = serde_json::from_str::<serde_json::Value>(&text) else {
            return (Vec::new(), 1);
        };
        let created = ms(thread.get("created"));
        let (mut events, mut unreadable) = ledger_records(&thread, created);
        let (messages, missed) = message_records(&thread, created);
        unreadable += missed;

        if events.is_empty() {
            let kept = messages
                .into_iter()
                .filter_map(|r| r.at.map(|at| Kept { at, tokens: r.tokens }))
                .collect();
            return (kept, unreadable);
        }

        let mut consumed = vec![false; events.len()];
        let mut from = 0usize;
        let mut extra = Vec::new();
        for message in &messages {
            match matching(&events, &consumed, from, message) {
                Some(i) => {
                    consumed[i] = true;
                    from = i.saturating_add(1);
                    // The message adds no tokens. All it can lend is a
                    // clock, and only to an event that never had one.
                    if events[i].at.is_none() {
                        events[i].at = message.at;
                    }
                }
                None => extra.push(message.clone()),
            }
        }
        events.extend(extra);
        let kept = events
            .into_iter()
            .filter_map(|r| r.at.map(|at| Kept { at, tokens: r.tokens }))
            .collect();
        (kept, unreadable)
    }

    impl Meter for Amp {
        fn vendor(&self) -> &'static str {
            VENDOR
        }

        fn root(&self) -> PathBuf {
            self.threads_dir()
        }

        fn tally(&self) -> Tally {
            let mut files = self.files.lock().unwrap_or_else(|p| p.into_inner());
            let (states, unreadable) = files.refresh(json_under(&self.threads_dir()), parse);
            Tally {
                kept: states.into_iter().flat_map(|k| k.iter().copied()).collect(),
                unreadable,
            }
        }
    }
}

#[cfg(test)]
mod tests;
