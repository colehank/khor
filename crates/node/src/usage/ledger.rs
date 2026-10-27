//! What khor has already counted, kept so that a transcript the vendor
//! deletes does not take its history with it.
//!
//! # Why khor keeps its own record at all
//!
//! The meters read the vendors' files, and the vendors delete them:
//! Claude Code prunes old transcripts on its own schedule. Measured
//! 2026-10-07 on this machine, 2026-08-10 reads 32.5 M tokens off what is
//! left on disk where Claude's own stats cache remembers 4.06 G — most of
//! July and August had simply gone from khor's chart. The answer for a
//! past day must not shrink because a file was tidied away, so khor
//! writes down what it counted, and a past period is answered from
//! whichever of the two — the files or this — says more.
//!
//! **Not Claude's `stats-cache.json`.** It keeps history, but its numbers
//! sum every transcript line (the 1.7–2.4× the claude meter refuses) and
//! do not split the four kinds; importing it would put a second rule
//! beside the meter's and no way to tell them apart.
//!
//! # What is kept: fifteen-minute buckets, never days
//!
//! [`Meters::tally_in`](super::Meters::tally_in) cuts days in whatever
//! zone is asking, so a ledger of civil days would be one zone's opinion
//! frozen on disk. What is kept is finer than any zone's day: tokens per
//! **fifteen minutes of UTC time**, per (vendor, model, whether a third
//! party answered), in the four kinds.
//!
//! Fifteen because every offset in use is a whole number of quarter
//! hours — :30 (India, Newfoundland, Adelaide), :45 (Nepal, Chatham,
//! Eucla) — and so is every daylight-saving step (Lord Howe moves by
//! half an hour). A quarter hour counted from the epoch therefore lies
//! wholly inside one civil day in every zone, and the day it belongs to
//! is the day its first instant falls on. Thirty minutes would cut
//! Nepal's midnight in half; an hour, India's.
//!
//! # What is recorded, and when
//!
//! **Only buckets that closed at least [`SETTLE_S`] ago.** A claude reply
//! is written several times while it streams, and until its last writing
//! the meter holds a partial reading — sometimes stamped in the bucket
//! before the one the final writing lands in. Recording that partial and
//! then the final would count one reply twice, in two buckets. Two hours
//! is far longer than any reply streams; nothing is lost by waiting,
//! because the vendor deletes weeks-old files, not the last two hours.
//!
//! # The merge rule, and whether it can overstate
//!
//! Per bucket and key, the answer is the **larger of the ledger and the
//! files for each kind**, and the ledger is raised the same way. A
//! pruned transcript lowers the files' reading and leaves the ledger's,
//! so a past period never reads less than what was recorded.
//!
//! A per-kind maximum invents tokens in exactly one case: two readings of
//! the **same** spending that split it differently between the kinds.
//! The claude meter meets that case inside one reply — a later writing
//! moves input from cache-write to cache-read (the head of `claude` has
//! the measured pair) — and refuses a per-kind maximum there for that
//! reason. Here it cannot meet it, and the settle rule is why: a bucket
//! is recorded only once its replies are final, and a final reading
//! does not change afterwards. What can still change a settled bucket is
//! records leaving (a deleted file: the files read less, the ledger
//! keeps its own) or records arriving (a transcript copied in late: the
//! files read more, the ledger is raised). The one mix of the two — one
//! reply gone and another arrived in the same quarter hour, same model —
//! gives at most the larger of each kind, which is **at most the true
//! sum**: it can undercount, never overcount.
//!
//! # A rule change starts a new ledger
//!
//! The ledger holds what the meters counted under the rules they had
//! when they counted it. A change to those rules — which replies are the
//! subscription's (`claude::official_id`), how a reply is deduplicated,
//! which field a kind is read from — would leave old buckets counted one
//! way beside new ones counted another, with nothing to tell them apart.
//! So the rules have a number, [`VERSION`], and it is in the file's name:
//! bumping it starts an empty ledger beside the old one rather than
//! mixing the two, and the old file is left where it is.
//!
//! # Size, writes, and failure
//!
//! Measured 2026-10-07 on this machine: 3 474 buckets across 66 days of
//! claude transcripts, 53 a day on average and 158 on the busiest — a
//! few hundred kilobytes a year. [`MAX_ROWS`] caps it at about five years
//! of that, dropping the oldest buckets first.
//!
//! Written whole, beside the file and renamed over it, and **only when a
//! settled bucket changed** — about once a quarter hour while an agent
//! works, never while nothing does. A ledger that is missing, unreadable
//! or of another shape reads as empty: the page falls back to what the
//! files say, which is what it said before there was a ledger.
//!
//! Two processes on one khor home (the window and `khor usage`) can both
//! write it. Each re-reads the file and keeps the larger of each kind
//! before it writes, so the one that writes second keeps what the first
//! recorded.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use khor_core::Tokens;

/// The rules the ledger was counted under. See the module head for what
/// changes it, and [`FILE`] for where it shows.
pub const VERSION: u32 = 1;

/// The ledger's name under the khor home's `.khor`. **The version is in
/// the name** so a rule change starts a file of its own.
pub const FILE: &str = "usage-ledger-v1.json";

/// The width of a bucket, in seconds. See the module head.
pub const BUCKET_S: i64 = 15 * 60;

/// How long after a bucket closes it is recorded. See the module head.
pub const SETTLE_S: i64 = 2 * 60 * 60;

/// The most buckets the ledger keeps — about five years at this
/// machine's pace; the oldest go first.
pub const MAX_ROWS: usize = 100_000;

/// Whose tokens a bucket holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    pub vendor: Arc<str>,
    pub model: Option<Arc<str>>,
    /// [`super::Kept::third_party`].
    pub third_party: bool,
}

/// Tokens by (bucket, key).
pub type Book = HashMap<(i64, Key), Tokens>;

/// The bucket an instant falls in.
pub fn bucket_of(at: jiff::Timestamp) -> i64 {
    at.as_second().div_euclid(BUCKET_S)
}

/// The first instant of a bucket — the one that names its day.
pub fn start_of(bucket: i64) -> jiff::Timestamp {
    jiff::Timestamp::from_second(bucket.saturating_mul(BUCKET_S))
        .unwrap_or(jiff::Timestamp::UNIX_EPOCH)
}

/// The second a bucket closes.
fn closes(bucket: i64) -> i64 {
    bucket.saturating_add(1).saturating_mul(BUCKET_S)
}

/// The larger of two readings, kind by kind. See the module head for why
/// this cannot overstate here.
fn larger(a: Tokens, b: Tokens) -> Tokens {
    Tokens {
        input: a.input.max(b.input),
        cached_input: a.cached_input.max(b.cached_input),
        cache_write: a.cache_write.max(b.cache_write),
        output: a.output.max(b.output),
    }
}

/// One ledger file and what this process has read of it.
pub struct Ledger {
    path: PathBuf,
    /// Read on first use and kept: the file only changes when somebody
    /// writes it, and this process re-reads it before it does.
    book: Mutex<Option<Book>>,
}

impl Ledger {
    pub fn at(path: PathBuf) -> Ledger {
        Ledger {
            path,
            book: Mutex::new(None),
        }
    }

    /// Records the settled part of `fresh`, and answers `fresh` with
    /// everything recorded folded in — the larger of the two for each
    /// kind.
    ///
    /// The second half of the answer is when the oldest bucket left
    /// unrecorded becomes old enough to record, so a caller that keeps
    /// answers knows when to ask again even if no file moves.
    pub fn merge(&self, mut fresh: Book, now: jiff::Timestamp) -> (Book, Option<jiff::Timestamp>) {
        let ripe = now.as_second().saturating_sub(SETTLE_S);
        let mut held = self.book.lock().unwrap_or_else(|p| p.into_inner());
        let book = held.get_or_insert_with(|| read(&self.path));
        let mut changed = false;
        let mut due: Option<i64> = None;
        for ((bucket, key), tokens) in &fresh {
            if closes(*bucket) > ripe {
                let at = closes(*bucket).saturating_add(SETTLE_S);
                due = Some(due.map_or(at, |d| d.min(at)));
                continue;
            }
            if tokens.is_zero() {
                continue;
            }
            let had = book.entry((*bucket, key.clone())).or_default();
            let both = larger(*had, *tokens);
            if both != *had {
                *had = both;
                changed = true;
            }
        }
        if changed {
            for (k, tokens) in read(&self.path) {
                let had = book.entry(k).or_default();
                *had = larger(*had, tokens);
            }
            prune(book, MAX_ROWS);
            // A write that fails is retried at the next change; the
            // answer below does not depend on it.
            let _ = write(&self.path, book);
        }
        for (k, tokens) in book.iter() {
            let got = fresh.entry(k.clone()).or_default();
            *got = larger(*got, *tokens);
        }
        (
            fresh,
            due.and_then(|s| jiff::Timestamp::from_second(s).ok()),
        )
    }
}

/// The file as it is written.
///
/// **Keys once, rows as numbers**, because the rows are many and the keys
/// are a handful: a row is `[bucket, key, input, cached_input,
/// cache_write, output]`, about forty bytes.
#[derive(serde::Serialize, serde::Deserialize)]
struct Disk {
    version: u32,
    bucket_s: i64,
    keys: Vec<(String, Option<String>, bool)>,
    rows: Vec<(i64, usize, u64, u64, u64, u64)>,
}

/// What the ledger at `path` holds, or nothing.
///
/// **Nothing, never an error**: a missing file is a ledger that has not
/// been written yet, and a broken one is not a reason to blank the page.
/// A row naming a key the file does not have is dropped, the rest kept.
fn read(path: &Path) -> Book {
    let mut book = Book::new();
    let Ok(bytes) = std::fs::read(path) else {
        return book;
    };
    let Ok(disk) = serde_json::from_slice::<Disk>(&bytes) else {
        return book;
    };
    if disk.version != VERSION || disk.bucket_s != BUCKET_S {
        return book;
    }
    let keys: Vec<Key> = disk
        .keys
        .into_iter()
        .map(|(vendor, model, third_party)| Key {
            vendor: Arc::from(vendor),
            model: model.map(Arc::from),
            third_party,
        })
        .collect();
    for (bucket, k, input, cached_input, cache_write, output) in disk.rows {
        let Some(key) = keys.get(k) else { continue };
        let tokens = Tokens {
            input,
            cached_input,
            cache_write,
            output,
        };
        let had = book.entry((bucket, key.clone())).or_default();
        *had = larger(*had, tokens);
    }
    book
}

/// Writes the whole ledger beside `path` and renames it over, so a reader
/// never sees half of it. The temp name carries this process, for the
/// reason `crate::tabs` gives: two processes write here.
fn write(path: &Path, book: &Book) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut keys: Vec<&Key> = book.keys().map(|(_, k)| k).collect();
    keys.sort();
    keys.dedup();
    let index: HashMap<&Key, usize> = keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    let mut rows: Vec<(i64, usize, u64, u64, u64, u64)> = book
        .iter()
        .map(|((bucket, key), t)| {
            (
                *bucket,
                index[key],
                t.input,
                t.cached_input,
                t.cache_write,
                t.output,
            )
        })
        .collect();
    // Sorted so the same ledger is the same bytes.
    rows.sort_unstable();
    let disk = Disk {
        version: VERSION,
        bucket_s: BUCKET_S,
        keys: keys
            .iter()
            .map(|k| {
                (
                    k.vendor.to_string(),
                    k.model.as_ref().map(|m| m.to_string()),
                    k.third_party,
                )
            })
            .collect(),
        rows,
    };
    let bytes = serde_json::to_vec(&disk).map_err(std::io::Error::other)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = path.with_file_name(format!("{name}.{}", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Drops the oldest buckets until about `max` rows are left. **About**:
/// the cut is made between buckets, never inside one, so a bucket is
/// kept or dropped whole.
fn prune(book: &mut Book, max: usize) {
    if book.len() <= max {
        return;
    }
    let mut buckets: Vec<i64> = book.keys().map(|(b, _)| *b).collect();
    buckets.sort_unstable();
    let cut = buckets[buckets.len() - max];
    book.retain(|(b, _), _| *b >= cut);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(model: &str) -> Key {
        Key {
            vendor: Arc::from("claude"),
            model: Some(Arc::from(model)),
            third_party: false,
        }
    }

    fn out(n: u64) -> Tokens {
        Tokens {
            output: n,
            ..Tokens::default()
        }
    }

    #[test]
    fn the_file_name_carries_the_version() {
        assert_eq!(FILE, format!("usage-ledger-v{VERSION}.json"));
    }

    /// **Every zone in use cuts its days on a bucket boundary**, which is
    /// the whole reason the bucket is fifteen minutes: midnight in each
    /// of these is a whole number of buckets from the epoch.
    #[test]
    fn a_bucket_never_straddles_a_midnight_anywhere() {
        for minutes in [0, 330, 345, 525, 765, -210, -570, 60, 480] {
            let zone =
                jiff::tz::TimeZone::fixed(jiff::tz::Offset::from_seconds(minutes * 60).unwrap());
            let midnight = jiff::civil::date(2026, 8, 17)
                .to_zoned(zone)
                .unwrap()
                .timestamp();
            assert_eq!(
                midnight.as_second().rem_euclid(BUCKET_S),
                0,
                "midnight at offset {minutes} min falls inside a bucket"
            );
        }
    }

    /// The oldest go first, and a bucket goes whole.
    #[test]
    fn a_full_ledger_drops_its_oldest_buckets() {
        let mut book = Book::new();
        for b in 0..10 {
            book.insert((b, key("a")), out(1));
            book.insert((b, key("b")), out(1));
        }
        prune(&mut book, 6);
        let mut left: Vec<i64> = book.keys().map(|(b, _)| *b).collect();
        left.sort_unstable();
        left.dedup();
        assert_eq!(
            left,
            vec![7, 8, 9],
            "the three newest buckets, both rows each"
        );
        assert_eq!(book.len(), 6);
    }

    /// Written and read back, the same book — and a row whose key is not
    /// in the file is dropped rather than taken for another.
    #[test]
    fn a_ledger_reads_back_what_it_wrote() {
        let dir = crate::testdir::TempTree::fresh("ledger-round");
        let path = dir.join(FILE);
        let mut book = Book::new();
        book.insert(
            (7, key("a")),
            Tokens {
                input: 1,
                cached_input: 2,
                cache_write: 3,
                output: 4,
            },
        );
        book.insert(
            (
                9,
                Key {
                    vendor: Arc::from("claude"),
                    model: None,
                    third_party: true,
                },
            ),
            out(5),
        );
        write(&path, &book).unwrap();
        assert_eq!(read(&path), book);

        std::fs::write(
            &path,
            format!(r#"{{"version":{VERSION},"bucket_s":{BUCKET_S},"keys":[["claude","a",false]],"rows":[[1,0,0,0,0,9],[2,5,0,0,0,9]]}}"#),
        )
        .unwrap();
        let back = read(&path);
        assert_eq!(back.len(), 1, "the row naming key 5 is dropped: {back:?}");
    }
}
