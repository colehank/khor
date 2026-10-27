//! The day strip: what this machine's list said about each session over
//! the last 24 hours, one minute at a time.
//!
//! # Why a stored history does not contradict "a stored copy is stale"
//!
//! Everything else about state in this crate refuses to keep a copy —
//! `live.rs`'s head says the word "is derived fresh on every ask and
//! never syncs", `vitals.rs`'s says a reading "is never stored". This is
//! the first record in the tree that keeps anything, so the line it
//! stands on has to be drawn here or the next person reads it as a
//! violation of docs/NET.md.
//!
//! That rule governs **answers to "how is it now"**. What it forbids is
//! serving a saved word as the current one, because the thing being
//! copied moves. A history makes a different claim — *at 10:04 the list
//! said 忙碌* — and that claim never becomes false. Nothing here moves,
//! so nothing here goes stale.
//!
//! **The boundary, and it is structural rather than a promise**: this
//! module offers no way to ask what a session's word is. There is no
//! `current`, no `latest`, no accessor that hands back the newest cell
//! on its own. "How is it now" has exactly one road (`LiveKind::rows`
//! → `Node::sessions`) and it is not this one. The two may legitimately
//! disagree at the edge — the newest cell is up to one sample old.
//!
//! It is also local and private: not in a replicated document (a CRDT's
//! history cannot be deleted, and docs/NET.md 同步 puts only small
//! decision-class data there — an observation log is neither), and not
//! on the wire (`khor_core::Session` is untouched, so `proto.rs` did
//! not move for this).
//!
//! # What is recorded, and why it cannot be the registry
//!
//! **The derived row, not `state.json`.** Half the words a list shows
//! were never written anywhere: `live::face` derives 失败 from a dead
//! process and 中断 from a lost host at read time, `live::settle_done`
//! turns 完成 into 空闲 when a watermark another machine moved covers
//! it, and the whole discovered family (docs/HOOKS.md 读盘发现, the
//! default path) is synthesised per sweep and has no registry entry at
//! all. A recorder hooked to `live::write_state` would paint a claude
//! that crashed as 忙碌 for the rest of the day and would not see the
//! zero-install sessions at any point.
//!
//! # Three answers per cell, two of which are kinds of nothing
//!
//! [`khor_core::DayCell`] holds that judgment and the argument for it.
//! What this module adds is where each comes from: the words come from
//! a row that was on the list, [`DayCell::Absent`] from a minute that
//! was sampled with no such row in it, and [`DayCell::Unwatched`] from
//! a minute nothing sampled — which is why the machine-wide "was
//! anyone looking" ledger exists beside the per-session rows. Without
//! it, a night with khor shut down and a session that started this
//! morning are the same blank.
//!
//! # Bounded by construction, in both directions
//!
//! - **Time**: cells are addressed by absolute minute, and the window
//!   is [`MINUTES`] wide. Rolling it forward drops the head; a row left
//!   with nothing in the window disappears with it. There is no reaper
//!   to forget to run.
//! - **Rows**: [`MAX_ROWS`], evicting whoever was seen longest ago. A
//!   machine that opened and closed ten thousand short sessions today
//!   keeps the last [`MAX_ROWS`] of them and its file does not grow.
//!
//! The encoding is fixed-width on purpose — one byte per minute per row
//! — so the **worst case is the typical case times the row count**, and
//! neither a session flapping between words every minute nor any other
//! input can make it bigger. A run-length encoding would be smaller at
//! rest and unbounded under exactly that flapping, which is the input a
//! bound is for. Measured sizes are on [`MAX_ROWS`].
//!
//! # The known cost: a lost update between two residents
//!
//! Writing is read-merge-write, and `write_whole`'s tmp+rename makes
//! each write atomic — **but atomic is not the same as serialised**. If
//! two residents overlap (iroh allows one live endpoint per key, so
//! this is the moment one knocks the other off), A can read, B read, A
//! write, B write, and A's sample is gone. The merge is still worth
//! having: without it B's write would drop everything A ever recorded
//! rather than one sample of it. What is lost is one 10-second sample
//! of one minute, and the same word is re-established by the next
//! sample — the cell only changes if the word changed inside that
//! window. Do not read the merge as making concurrent writes safe.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use khor_core::{DayCell, State};

/// How wide the window is. A day, in minutes.
pub const MINUTES: usize = 24 * 60;

/// One cell of the record. Not the cell a strip is drawn at — screens
/// get far fewer than 1440 pixels for this, and [`Day::strip_of`]
/// merges down to whatever they have.
const CELL_MS: i64 = 60_000;

/// The most sessions one machine's record holds.
///
/// A bound, not a capacity estimate: what it exists to stop is a
/// machine that churns through short sessions all day turning an
/// observation log into an unbounded one.
pub const MAX_ROWS: usize = 512;

/// How often the resident takes a sample.
///
/// Six per cell. The six words are **states, not events** — a session
/// does not flicker through 待批 and back inside ten seconds, because
/// 待批 by definition waits for a person — so a coarser beat than the
/// 5s sync pump loses nothing worth a cell and costs a sixth as much.
/// One sample is one `Node::sessions`, measured at 13.2 ms on this
/// machine's debug build (`adaptor::Procs::snapshot`).
pub const SAMPLE_EVERY: std::time::Duration = std::time::Duration::from_secs(10);

/// A minute in which nothing sampled.
const UNWATCHED: u8 = b'-';
/// A minute in which something sampled.
const WATCHED: u8 = b'+';
/// A sampled minute in which this session had no row.
const ABSENT: u8 = b'.';

/// One byte per word.
///
/// Spelled here rather than taken from `State::key()`'s first letter,
/// because 忙碌 and 待批 both start with `b` and a collision would make
/// two words one silently. The match is exhaustive, so a seventh word
/// could not be added without landing here.
const fn byte_of(word: State) -> u8 {
    match word {
        State::Busy => b'b',
        State::Blocked => b'k',
        State::Done => b'd',
        State::Errored => b'e',
        State::Failed => b'f',
        State::Idle => b'i',
    }
}

fn word_of(byte: u8) -> Option<State> {
    State::ALL.into_iter().find(|w| byte_of(*w) == byte)
}

/// One stretch of a strip: a cell repeated. What the face paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub cell: DayCell,
    pub cells: u32,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("day.json")
}

/// The record as it stands right now.
///
/// [`Day::load`] takes the clock as an argument so tests can place the
/// window where they want it; every caller that is not a test wants
/// this one, and having it here keeps `now_ms` from being spelled at
/// each call site — where one of them would eventually be spelled
/// differently.
pub fn read(root: &Path) -> Day {
    Day::load(root, crate::live::now_ms())
}

/// The record as it sits on disk. Fixed-width strings, one byte per
/// minute, so a malformed length is detectable rather than shifting
/// every cell by an unknown amount.
#[derive(serde::Serialize, serde::Deserialize)]
struct Stored {
    /// The absolute minute (Unix ms / 60000) of index 0.
    from: i64,
    watched: String,
    rows: BTreeMap<String, String>,
}

/// The last day, in memory.
#[derive(Clone)]
pub struct Day {
    from: i64,
    watched: Vec<u8>,
    rows: BTreeMap<String, Vec<u8>>,
}

fn blank(fill: u8) -> Vec<u8> {
    vec![fill; MINUTES]
}

impl Day {
    /// An empty record whose window ends at `now_ms`.
    pub fn empty(now_ms: i64) -> Day {
        Day {
            from: minute(now_ms) - MINUTES as i64 + 1,
            watched: blank(UNWATCHED),
            rows: BTreeMap::new(),
        }
    }

    /// Reads the record, or an empty one.
    ///
    /// **An unreadable or malformed file is an empty record, not an
    /// error.** The alternative is a resident that refuses to record
    /// anything because of one bad byte in a file nothing else depends
    /// on; the cost of this direction is that a corrupted record is
    /// silently rebuilt, and what it costs is history nobody could have
    /// read anyway.
    pub fn load(root: &Path, now_ms: i64) -> Day {
        let mut day = std::fs::read_to_string(path(root))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .and_then(Day::from_stored)
            .unwrap_or_else(|| Day::empty(now_ms));
        day.roll(minute(now_ms));
        day
    }

    fn from_stored(s: Stored) -> Option<Day> {
        let watched = s.watched.into_bytes();
        if watched.len() != MINUTES {
            return None;
        }
        let mut rows = BTreeMap::new();
        for (id, cells) in s.rows {
            let cells = cells.into_bytes();
            if cells.len() != MINUTES {
                return None;
            }
            rows.insert(id, cells);
        }
        Some(Day {
            from: s.from,
            watched,
            rows,
        })
    }

    fn to_stored(&self) -> Stored {
        Stored {
            from: self.from,
            watched: String::from_utf8_lossy(&self.watched).into_owned(),
            rows: self
                .rows
                .iter()
                .map(|(id, cells)| (id.clone(), String::from_utf8_lossy(cells).into_owned()))
                .collect(),
        }
    }

    /// Which slot an absolute minute sits in, if the window covers it.
    fn slot(&self, min: i64) -> Option<usize> {
        let i = min.checked_sub(self.from)?;
        (0..MINUTES as i64).contains(&i).then_some(i as usize)
    }

    /// Moves the window so it ends at `now_min`, dropping what falls off
    /// the front.
    ///
    /// **This is the whole of the time bound.** Nothing sweeps or
    /// prunes; a minute leaves because the window moved past it, and a
    /// row whose every minute has left goes with it.
    ///
    /// A `now_min` already inside the window does nothing — including
    /// one *behind* it, which is a clock that went backwards. Rolling
    /// the window back would throw away the newer end for a reading
    /// nobody can trust anyway.
    fn roll(&mut self, now_min: i64) {
        let want = now_min - MINUTES as i64 + 1;
        if want <= self.from {
            return;
        }
        let shift = (want - self.from) as usize;
        if shift >= MINUTES {
            self.watched = blank(UNWATCHED);
            self.rows.clear();
        } else {
            shift_left(&mut self.watched, shift, UNWATCHED);
            for cells in self.rows.values_mut() {
                shift_left(cells, shift, ABSENT);
            }
            self.rows
                .retain(|_, cells| cells.iter().any(|b| *b != ABSENT));
        }
        self.from = want;
    }

    /// Records one sample: these rows were on the list at `now_ms`.
    ///
    /// The rows are this machine's own. A row another device reported is
    /// deliberately not recorded — see [`Day::unwatched`].
    pub fn observe<I: IntoIterator<Item = (String, State)>>(&mut self, now_ms: i64, rows: I) {
        let min = minute(now_ms);
        self.roll(min);
        let Some(i) = self.slot(min) else {
            return;
        };
        self.watched[i] = WATCHED;
        for (id, word) in rows {
            if !self.rows.contains_key(&id) {
                self.make_room();
            }
            let cells = self.rows.entry(id).or_insert_with(|| blank(ABSENT));
            // Several samples land in one cell, and the one that
            // survives is the one the list ranks first
            // (`khor_core::DayCell::rank` has why, and why it is not a
            // second copy of that order).
            let keep = match word_of(cells[i]) {
                Some(had) if had.rank() <= word.rank() => had,
                _ => word,
            };
            cells[i] = byte_of(keep);
        }
    }

    /// The row exists, but no source could identify its current state.
    pub fn unavailable(&mut self, now_ms: i64, ids: impl IntoIterator<Item = String>) {
        self.roll(minute(now_ms));
        let Some(i) = self.slot(minute(now_ms)) else {
            return;
        };
        for id in ids {
            if !self.rows.contains_key(&id) {
                self.make_room();
            }
            self.rows.entry(id).or_insert_with(|| blank(ABSENT))[i] = UNWATCHED;
        }
    }

    /// Drops the least recently seen row when the record is full.
    ///
    /// Least recently *seen*, not oldest: a session that has been on the
    /// list all day is the one worth a strip, and evicting by age of
    /// first sighting would throw it out in favour of whatever started
    /// a minute ago.
    fn make_room(&mut self) {
        while self.rows.len() >= MAX_ROWS {
            let Some(stalest) = self
                .rows
                .iter()
                .min_by_key(|(id, cells)| (cells.iter().rposition(|b| *b != ABSENT), (*id).clone()))
                .map(|(id, _)| id.clone())
            else {
                return;
            };
            self.rows.remove(&stalest);
        }
    }

    /// One session's day, merged down to `cells` cells.
    ///
    /// An id the record has never held is not an error and not blank
    /// either: every sampled minute says 那时它不在 ([`DayCell::Absent`])
    /// and every unsampled one says 谁也没在看. That is the honest
    /// reading of a session that started five minutes ago.
    pub fn strip_of(&self, id: &str, cells: usize) -> Vec<Run> {
        let cells = cells.clamp(1, MINUTES);
        let row = self.rows.get(id);
        let mut out: Vec<Run> = Vec::new();
        for c in 0..cells {
            let lo = c * MINUTES / cells;
            let hi = ((c + 1) * MINUTES / cells).max(lo + 1);
            let mut best = DayCell::Absent;
            for i in lo..hi.min(MINUTES) {
                let here = self.cell_at(row, i);
                if here.rank() < best.rank() {
                    best = here;
                }
            }
            push(&mut out, best);
        }
        out
    }

    /// What one minute says about one row.
    ///
    /// **A minute nobody sampled is [`DayCell::Unwatched`] whatever the
    /// row holds, and it is never filled in from its neighbours.**
    /// Carrying the previous word across a gap is the tempting bug —
    /// it makes a strip look continuous — and it is the one this whole
    /// batch exists to refuse: it would paint a night khor spent shut
    /// down as a night the session spent 忙碌.
    fn cell_at(&self, row: Option<&Vec<u8>>, i: usize) -> DayCell {
        if self.watched[i] != WATCHED {
            return DayCell::Unwatched;
        }
        if row.is_some_and(|cells| cells[i] == UNWATCHED) {
            return DayCell::Unwatched;
        }
        match row.and_then(|cells| word_of(cells[i])) {
            Some(word) => DayCell::Word(word),
            None => DayCell::Absent,
        }
    }

    /// This record as it travels to another machine (`Request::Days`):
    /// the same shape the disk holds, because both are the same claim —
    /// *at 10:04 this machine's list said 忙碌* — and a claim about the
    /// past does not go stale in transit the way a word does
    /// (module head). Text rather than bytes so the wire and the file
    /// cannot drift into two encodings of one thing.
    pub fn to_wire(&self) -> String {
        serde_json::to_string(&self.to_stored()).unwrap_or_default()
    }

    /// A record another machine sent, **rolled to this machine's now**.
    ///
    /// The peer's day ends at the peer's clock when it answered; every
    /// minute between then and now is one this machine has no word for,
    /// and rolling fills exactly those with 没在看. That is the honest
    /// tail: a peer unreachable since noon shows a strip that stops at
    /// noon, not one that says 空闲 all afternoon (`unwatched` has the
    /// full argument). Two clocks are involved and the minute grid is
    /// the peer's; a skew between them shifts the strip by that skew,
    /// which the same-minute format cannot show and the record cannot
    /// know.
    pub fn from_wire(text: &str, now_ms: i64) -> Option<Day> {
        let mut day = Day::from_stored(serde_json::from_str::<Stored>(text).ok()?)?;
        day.roll(minute(now_ms));
        Some(day)
    }

    pub fn save(&self, root: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec(&self.to_stored()).map_err(|e| e.to_string())?;
        let file = path(root);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        crate::live::write_whole(&file, &bytes)
    }
}

/// A strip for a row this machine has no record of at all — every cell
/// 不知道.
///
/// **What every reported row gets** (远端这一批不做). A peer's row
/// arrives as one report with an age on it (`Node::cache_peer_rows`),
/// not as a history, and sampling that cache instead would be worse
/// than having nothing: the cached word survives the peer going
/// offline, so an afternoon nobody could reach turing would be painted
/// as an afternoon turing was 忙碌 — a stored copy served as a lived
/// fact, which is the thing docs/NET.md forbids. Doing it honestly
/// needs a gate on each report's age, and that is a batch of its own.
///
/// So the strip says what is true from here: this machine did not watch
/// that session for any of the last day. Not *no strip* — a row with no
/// band leaves a hole in the list, and on screen it would be
/// indistinguishable from a band that is [`DayCell::Absent`]
/// throughout, which claims the opposite (khor watched and there was
/// nothing).
pub fn unwatched(cells: usize) -> Vec<Run> {
    vec![Run {
        cell: DayCell::Unwatched,
        cells: cells.clamp(1, MINUTES) as u32,
    }]
}

fn push(out: &mut Vec<Run>, cell: DayCell) {
    match out.last_mut() {
        Some(run) if run.cell == cell => run.cells += 1,
        _ => out.push(Run { cell, cells: 1 }),
    }
}

/// **A day as the minutes since the asker's copy ended** (round NET-3):
/// what crosses instead of the whole record when the asker already holds
/// one. A record changes only at its newest minute — a sample lands in
/// the current cell, every older cell is history — so an asker whose
/// copy ended at minute `E` needs, per row, only the cells from `E` on
/// (`E` itself included: it may have been sampled again since), and the
/// list of rows still there.
///
/// Built by [`Day::since`], applied by [`apply_since`]. The asker checks
/// the result against the whole answer's digest the sender carries
/// alongside, and asks for the whole record if they differ — so this is
/// a saving, never a second source of truth.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DaySince {
    /// The window's first minute after this answer.
    pub from: i64,
    /// The cells from the asker's last minute on, for `watched`.
    pub watched: String,
    /// Every row still in the record, with its cells from the asker's
    /// last minute on. A row the asker holds and this does not name has
    /// left the record.
    pub rows: Vec<(String, String)>,
}

impl Day {
    /// The minutes from `held_end` (the last minute of the asker's copy)
    /// to this record's end — `None` when `held_end` is not inside this
    /// window, and the whole record is what to send.
    pub fn since(&self, held_end: i64) -> Option<DaySince> {
        let at = self.slot(held_end)?;
        let tail = |cells: &[u8]| String::from_utf8_lossy(&cells[at..]).into_owned();
        Some(DaySince {
            from: self.from,
            watched: tail(&self.watched),
            rows: self
                .rows
                .iter()
                .map(|(id, cells)| (id.clone(), tail(cells)))
                .collect(),
        })
    }

    /// The last minute a record in its wire form covers.
    pub fn end_of_wire(text: &str) -> Option<i64> {
        let stored = serde_json::from_str::<Stored>(text).ok()?;
        Some(stored.from + MINUTES as i64 - 1)
    }
}

/// The asker's copy (wire form) brought up to date by `since`, in wire
/// form — `None` when the two do not fit together (the caller then asks
/// for the whole record).
pub fn apply_since(held: &str, since: &DaySince) -> Option<String> {
    let mut day = Day::from_stored(serde_json::from_str::<Stored>(held).ok()?)?;
    day.roll(since.from + MINUTES as i64 - 1);
    if day.from != since.from {
        return None;
    }
    let lay = |cells: &mut Vec<u8>, tail: &str| -> Option<()> {
        let tail = tail.as_bytes();
        let at = MINUTES.checked_sub(tail.len())?;
        cells[at..].copy_from_slice(tail);
        Some(())
    };
    lay(&mut day.watched, &since.watched)?;
    let mut rows = BTreeMap::new();
    for (id, tail) in &since.rows {
        let mut cells = day.rows.remove(id).unwrap_or_else(|| blank(ABSENT));
        lay(&mut cells, tail)?;
        rows.insert(id.clone(), cells);
    }
    day.rows = rows;
    Some(day.to_wire())
}

fn shift_left(cells: &mut Vec<u8>, by: usize, fill: u8) {
    cells.drain(..by);
    cells.resize(MINUTES, fill);
}

fn minute(now_ms: i64) -> i64 {
    now_ms.div_euclid(CELL_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unbound_thread_records_no_invented_idle_state() {
        let now = 60_000_000;
        let mut day = Day::empty(now);
        day.observe(now, [("known".into(), State::Busy)]);
        day.unavailable(now, ["unknown".into()]);
        let restored = Day::from_stored(day.to_stored()).unwrap();
        let i = restored.slot(minute(now)).unwrap();
        assert_eq!(
            restored.cell_at(restored.rows.get("known"), i),
            DayCell::Word(State::Busy)
        );
        assert_eq!(
            restored.cell_at(restored.rows.get("unknown"), i),
            DayCell::Unwatched
        );
    }

    /// **The minutes since an asker's copy rebuild the record exactly**
    /// (round NET-3): a copy taken at 10:00, the record sampled on to
    /// 10:07 with a row joining and one leaving, and the copy brought up
    /// to date from what [`Day::since`] carries is byte for byte the
    /// record's own wire form — at a fraction of its size. The control:
    /// a copy from outside the window gets no delta at all.
    #[test]
    fn the_minutes_since_a_copy_rebuild_the_record() {
        let t0 = 1_700_000_000_000i64;
        let mut day = Day::empty(t0);
        for m in 0..30 {
            let at = t0 - (30 - m) * CELL_MS;
            day.observe(
                at,
                [
                    ("a".to_owned(), State::Busy),
                    ("gone".to_owned(), State::Idle),
                ],
            );
        }
        day.observe(t0, [("a".to_owned(), State::Busy)]);
        let copy = day.to_wire();
        let held_end = Day::end_of_wire(&copy).unwrap();
        // The current minute sampled again, then seven more minutes; a
        // row joins, and "gone" is evicted by hand the way make_room
        // would.
        day.observe(t0 + 30_000, [("a".to_owned(), State::Blocked)]);
        for m in 1..=7 {
            day.observe(
                t0 + m * CELL_MS,
                [
                    ("a".to_owned(), State::Idle),
                    ("new".to_owned(), State::Busy),
                ],
            );
        }
        day.rows.remove("gone");
        let since = day
            .since(held_end)
            .expect("the copy's end is in the window");
        let rebuilt = apply_since(&copy, &since).expect("they fit");
        assert_eq!(rebuilt, day.to_wire(), "byte for byte the record");
        let delta = serde_json::to_string(&since).unwrap().len();
        assert!(
            delta * 20 < day.to_wire().len(),
            "{delta} B against {}",
            day.to_wire().len()
        );

        assert!(
            day.since(held_end - MINUTES as i64 * 2).is_none(),
            "a copy from another day is sent whole"
        );
    }

    /// **Over the wire and rolled forward.** A record a peer sent
    /// thirty minutes ago must read as that peer's day up to then and
    /// 没在看 since — not 空闲, not the last word stretched. The control
    /// is the round trip at the same instant, which must change
    /// nothing: the roll is the only thing the crossing adds.
    #[test]
    fn a_peers_day_reads_as_theirs_until_they_answered_and_unwatched_since() {
        let t0 = 1_700_000_000_000i64;
        let mut theirs = Day::empty(t0);
        theirs.observe(t0, [("s".to_owned(), State::Busy)]);
        let text = theirs.to_wire();

        let same = Day::from_wire(&text, t0).expect("the wire round-trips");
        assert_eq!(
            same.strip_of("s", MINUTES),
            theirs.strip_of("s", MINUTES),
            "same instant, same strip"
        );

        let later = Day::from_wire(&text, t0 + 30 * 60_000).expect("rolls");
        let runs = later.strip_of("s", MINUTES);
        let tail = runs.last().expect("a strip has a tail");
        assert_eq!(
            tail.cell,
            DayCell::Unwatched,
            "the minutes since they answered are nobody's"
        );
        assert_eq!(tail.cells, 30, "exactly the thirty minutes in transit");
        let busy = runs
            .iter()
            .find(|r| r.cell == DayCell::Word(State::Busy))
            .expect("their word survives");
        assert_eq!(busy.cells, 1);
    }

    const M: i64 = CELL_MS;

    fn at(minutes: i64) -> i64 {
        minutes * M
    }

    fn words(runs: &[Run]) -> Vec<(DayCell, u32)> {
        runs.iter().map(|r| (r.cell, r.cells)).collect()
    }

    /// **The gate.** A minute nobody sampled is 不知道, and it stays
    /// 不知道 even with a recorded word on either side of it.
    ///
    /// The failure it refuses is the tempting one: filling the gap from
    /// its neighbours so the band looks continuous. That reads as "the
    /// session was busy all night" when what happened is that khor was
    /// not running — and 判据 5 of this batch is precisely that those
    /// two must not look alike.
    ///
    /// Drawn at 1440 cells so one cell is one minute: this asserts about
    /// the record, not about a merge.
    #[test]
    fn an_unwatched_minute_is_not_the_word_beside_it() {
        let mut day = Day::empty(at(1000));
        day.observe(at(1000), [("s".to_owned(), State::Busy)]);
        // Nothing sampled at 1001 and 1002 — the resident was down.
        day.observe(at(1003), [("s".to_owned(), State::Busy)]);
        // The whole strip, not a slice off the end: carrying the word
        // across the gap **merges runs together**, so any assertion that
        // indexes from the end would fail by running off the front and
        // report an out-of-bounds instead of the bug. A gate has to fail
        // in the words of the thing it guards.
        assert_eq!(
            words(&day.strip_of("s", MINUTES)),
            vec![
                // Before khor ever sampled.
                (DayCell::Unwatched, 1436),
                (DayCell::Word(State::Busy), 1),
                // The resident was down for these two.
                (DayCell::Unwatched, 2),
                (DayCell::Word(State::Busy), 1),
            ],
            "the gap must stay 不知道, not inherit 忙碌 from either side"
        );
    }

    /// The other half of that: a minute that **was** sampled and simply
    /// had no such row is a different answer from one nobody sampled.
    ///
    /// Without this the first test passes with the two folded together,
    /// and the fold is the expensive one — every session that started
    /// this morning would read as a morning khor was switched off.
    #[test]
    fn a_watched_minute_with_no_row_is_not_the_same_as_no_watcher() {
        let mut day = Day::empty(at(1000));
        // Sampled, with some other session on the list.
        day.observe(at(1000), [("other".to_owned(), State::Idle)]);
        day.observe(at(1001), [("s".to_owned(), State::Busy)]);
        let strip = day.strip_of("s", MINUTES);
        let end = &strip[strip.len() - 2..];
        assert_eq!(
            words(end),
            vec![(DayCell::Absent, 1), (DayCell::Word(State::Busy), 1)]
        );
        // And a minute before khor ever ran is the other nothing.
        assert_eq!(strip[0].cell, DayCell::Unwatched);
    }

    /// Several samples in one cell, and the merge keeps the one the list
    /// ranks first. 待批 is the case worth the whole strip: it lasts
    /// until a person answers, and a merge that let the 忙碌 after it
    /// win would hide exactly what someone is scanning for.
    #[test]
    fn the_word_that_wanted_you_survives_the_merge() {
        let mut day = Day::empty(at(1000));
        day.observe(at(1000), [("s".to_owned(), State::Busy)]);
        day.observe(at(1000) + 10_000, [("s".to_owned(), State::Blocked)]);
        day.observe(at(1000) + 20_000, [("s".to_owned(), State::Busy)]);
        let strip = day.strip_of("s", MINUTES);
        assert_eq!(
            strip.last().map(|r| r.cell),
            Some(DayCell::Word(State::Blocked))
        );
    }

    /// Merging down to a screen's worth of cells uses the same order.
    /// A single blocked minute inside a coarse cell is still visible —
    /// which is the whole reason the order is what it is.
    #[test]
    fn a_coarse_cell_still_shows_the_minute_that_wanted_you() {
        let mut day = Day::empty(at(1439));
        for m in 0..1440 {
            let word = if m == 7 { State::Blocked } else { State::Busy };
            day.observe(at(m), [("s".to_owned(), word)]);
        }
        // 120 cells, so twelve minutes each: minute 7 is in the first.
        let strip = day.strip_of("s", 120);
        assert_eq!(strip[0].cell, DayCell::Word(State::Blocked));
        assert_eq!(strip[1].cell, DayCell::Word(State::Busy));
    }

    /// The time bound, and it is the window itself doing it: a minute
    /// leaves because the window moved, and a row with nothing left in
    /// it leaves with the minute.
    #[test]
    fn the_day_never_holds_more_than_a_day() {
        let mut day = Day::empty(at(0));
        day.observe(at(0), [("old".to_owned(), State::Busy)]);
        assert!(day.rows.contains_key("old"));
        // A day and a minute later.
        day.observe(at(1441), [("new".to_owned(), State::Busy)]);
        assert!(
            !day.rows.contains_key("old"),
            "a row older than the window must fall out"
        );
        assert_eq!(day.watched.len(), MINUTES, "the window is a fixed width");
        assert_eq!(day.rows.get("new").map(Vec::len), Some(MINUTES));
        // And the strip of the row that fell out reads as never watched
        // for the part of the window nothing sampled.
        assert_eq!(day.strip_of("old", 4)[0].cell, DayCell::Unwatched);
    }

    /// The row bound. A machine that churns through short sessions keeps
    /// the last [`MAX_ROWS`] of them, and the ones it drops are the ones
    /// seen longest ago — never the session that has been open all day.
    #[test]
    fn a_full_record_drops_the_stalest_row_not_the_oldest() {
        let mut day = Day::empty(at(0));
        day.observe(at(0), [("all-day".to_owned(), State::Busy)]);
        for m in 1..(MAX_ROWS as i64 + 20) {
            day.observe(
                at(m),
                [
                    ("all-day".to_owned(), State::Busy),
                    (format!("short-{m}"), State::Idle),
                ],
            );
        }
        assert!(day.rows.len() <= MAX_ROWS, "held {}", day.rows.len());
        assert!(
            day.rows.contains_key("all-day"),
            "the row seen every minute must survive"
        );
        assert!(
            !day.rows.contains_key("short-1"),
            "the stalest short row must go"
        );
    }

    /// Every word has its own byte, and it survives the round trip. A
    /// collision here would make two words one on disk, silently —
    /// 忙碌 and 待批 both begin with `b`, which is why these are not
    /// derived from `State::key()`.
    #[test]
    fn every_word_has_its_own_byte() {
        let mut seen = std::collections::HashSet::new();
        for word in State::ALL {
            let b = byte_of(word);
            assert!(seen.insert(b), "{:?} shares a byte", word);
            assert_ne!(b, ABSENT);
            assert_ne!(b, UNWATCHED);
            assert_ne!(b, WATCHED);
            assert_eq!(word_of(b), Some(word));
        }
    }

    /// **What the bound actually costs, measured rather than reasoned.**
    ///
    /// The worst case is a full record ([`MAX_ROWS`] rows) whose every
    /// session changed word every minute — the input that makes a
    /// run-length encoding explode and that a fixed-width one does not
    /// notice. The number is asserted, not just printed: an encoding
    /// change that quietly made this grow would otherwise show up as a
    /// full disk on somebody's machine.
    ///
    /// **Measured 2026-08-30, printed by this test: 750_016 bytes** for
    /// a full 512-row record — and byte for byte the same when every
    /// session sits on one word all day, because the encoding is fixed
    /// width. A twenty-row machine, which is what one of these actually
    /// looks like, is **30_712 bytes**; at one sample every ten seconds
    /// that is 11 MB of writes an hour, and the full 512-row case would
    /// be 270 MB an hour. If the row bound is ever raised, or the sample
    /// beat shortened, this is the number that decides it.
    #[test]
    fn the_worst_case_file_is_the_size_it_claims_to_be() {
        let mut jittery = Day::empty(at(MINUTES as i64 - 1));
        let mut calm = Day::empty(at(MINUTES as i64 - 1));
        for m in 0..MINUTES as i64 {
            let rows: Vec<(String, State)> = (0..MAX_ROWS)
                .map(|r| {
                    // A cheap deterministic shuffle: every row lands on
                    // a different word from the minute before.
                    let word = State::ALL[((m as usize) + r) % State::ALL.len()];
                    (format!("tui/session-{r:04}"), word)
                })
                .collect();
            jittery.observe(at(m), rows.clone());
            calm.observe(at(m), rows.into_iter().map(|(id, _)| (id, State::Idle)));
        }
        assert_eq!(jittery.rows.len(), MAX_ROWS);
        let big = serde_json::to_vec(&jittery.to_stored()).unwrap().len();
        let flat = serde_json::to_vec(&calm.to_stored()).unwrap().len();
        assert_eq!(
            big, flat,
            "a fixed-width encoding does not care how much the words jump"
        );
        // 512 rows × 1440 minutes = 737_280 bytes of cells, plus the
        // watched ledger, the ids and JSON's own punctuation.
        assert!(big < 780_000, "worst case grew to {big} bytes");

        // And what one of these actually looks like, so the bound above
        // is read next to the ordinary case rather than on its own.
        let mut usual = Day::empty(at(MINUTES as i64 - 1));
        for m in 0..MINUTES as i64 {
            usual.observe(
                at(m),
                (0..20).map(|r| (format!("tui/session-{r:04}"), State::Idle)),
            );
        }
        let small = serde_json::to_vec(&usual.to_stored()).unwrap().len();
        println!(
            "worst case: {big} bytes ({} rows); twenty rows: {small} bytes",
            jittery.rows.len()
        );
    }

    /// A reported row's strip is 不知道 end to end — never [`Absent`],
    /// which would claim khor watched and saw nothing.
    #[test]
    fn a_row_this_machine_never_watched_reads_as_unknown() {
        let runs = unwatched(96);
        assert_eq!(words(&runs), vec![(DayCell::Unwatched, 96)]);
    }
}
