//! The last day of CPU, five minutes at a stride — what the band under
//! the machine card's CPU ring draws (2026-09-02:「最近24小时cpu」).
//!
//! # A gap is a gap
//!
//! A bucket nobody sampled — the machine slept, the serve was down —
//! stays `None` and is **drawn as absence, never interpolated**: a line
//! bridging a gap claims the machine was doing something nobody
//! measured (docs/UX.md 不知道就说不知道). This is the vitals ledger's
//! one rule arriving on the time axis.
//!
//! # It survives a restart, and the word is meant literally
//!
//! The buckets are written to disk on every close of a bucket and read
//! back on open. The test that says so opens a **second** instance
//! from the same directory — the ledger's #52 lesson: a verb in the
//! assertion ("survives a restart") has to happen in the verification,
//! and two instances with no shared memory are the restart.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How often the resident takes a reading into the day
/// (`Node::cpu_recorder`). A minute: five readings to a bucket is a
/// mean with something in it, and a sample is a few milliseconds of
/// `sysinfo` — the beat could be faster, and nothing would be truer.
pub const RECORD_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// Five minutes to a bucket…
pub const BUCKET_MS: u64 = 5 * 60 * 1000;
/// …and a day of them on the board.
pub const BUCKETS: usize = 288;

pub use khor_core::CpuDay;

/// One machine's CPU history: what it has recorded and where it keeps
/// it. The recorder is the serve's minute beat (`link`); everybody else
/// only reads a [`CpuDay`].
pub struct CpuHistory {
    /// Sum and count per absolute bucket index (`at_ms / BUCKET_MS`),
    /// pruned to the day — a mean needs both halves until the bucket
    /// closes.
    buckets: BTreeMap<u64, (f64, u32)>,
    file: PathBuf,
}

impl CpuHistory {
    /// Opens the history kept under `dir`, reading back whatever an
    /// earlier run wrote. A missing or unreadable file is an empty
    /// history — a band of gaps, which is the truthful account of time
    /// nobody measured.
    pub fn open(dir: &Path) -> CpuHistory {
        let file = dir.join("cpu");
        let mut buckets = BTreeMap::new();
        if let Ok(text) = std::fs::read_to_string(&file) {
            for line in text.lines() {
                let mut it = line.split_whitespace();
                if let (Some(Ok(idx)), Some(Ok(sum)), Some(Ok(n))) = (
                    it.next().map(str::parse::<u64>),
                    it.next().map(str::parse::<f64>),
                    it.next().map(str::parse::<u32>),
                ) {
                    buckets.insert(idx, (sum, n));
                }
            }
        }
        CpuHistory { buckets, file }
    }

    /// Takes one reading into its bucket and prunes the day's far edge.
    /// Writes through on every call: the file is at most [`BUCKETS`]
    /// short lines, and a crash between beats then costs one beat, not
    /// a day.
    pub fn record(&mut self, at_ms: u64, cpu_pct: f32) {
        let idx = at_ms / BUCKET_MS;
        let slot = self.buckets.entry(idx).or_insert((0.0, 0));
        slot.0 += f64::from(cpu_pct);
        slot.1 += 1;
        let floor = idx.saturating_sub(BUCKETS as u64);
        self.buckets.retain(|k, _| *k > floor);
        let text: String = self
            .buckets
            .iter()
            .map(|(k, (sum, n))| format!("{k} {sum} {n}\n"))
            .collect();
        if let Some(parent) = self.file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.file, text);
    }

    /// The last day as a face draws it, `now` at the right edge.
    pub fn day(&self, now_ms: u64) -> CpuDay {
        let newest = now_ms / BUCKET_MS;
        let buckets = (0..BUCKETS as u64)
            .map(|back| {
                let idx = newest.wrapping_sub(BUCKETS as u64 - 1 - back);
                self.buckets.get(&idx).map(|(sum, n)| (sum / f64::from(*n)) as f32)
            })
            .collect();
        CpuDay { bucket_ms: BUCKET_MS, now_ms, buckets }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("khor-history-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Two beats in one bucket average; the bucket nobody sampled stays
    /// a hole. The gap assertion only counts because the two filled
    /// neighbours prove the reader finds what is there (否定式断言之前
    /// 先证明测法还活着).
    #[test]
    fn beats_average_within_a_bucket_and_a_gap_stays_a_hole() {
        let d = dir("gap");
        let mut h = CpuHistory::open(&d);
        let t0 = 100 * BUCKET_MS;
        h.record(t0, 10.0);
        h.record(t0 + 1000, 30.0);
        h.record(t0 + 2 * BUCKET_MS, 50.0); // one whole bucket skipped
        let day = h.day(t0 + 2 * BUCKET_MS);
        let n = day.buckets.len();
        assert_eq!(n, BUCKETS);
        assert_eq!(day.buckets[n - 3], Some(20.0), "two beats in one bucket average");
        assert_eq!(day.buckets[n - 2], None, "the unmeasured bucket is a hole, not a bridge");
        assert_eq!(day.buckets[n - 1], Some(50.0));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// "Survives a restart" with the restart actually happening: a
    /// second instance, opened cold from the same directory, reads
    /// what the first recorded (the #52 rule — the verb in the claim
    /// occurs in the verification).
    #[test]
    fn a_second_instance_opened_cold_reads_what_the_first_recorded() {
        let d = dir("restart");
        let t0 = 500 * BUCKET_MS;
        {
            let mut h = CpuHistory::open(&d);
            h.record(t0, 42.0);
        }
        let again = CpuHistory::open(&d);
        let day = again.day(t0);
        assert_eq!(day.buckets[BUCKETS - 1], Some(42.0), "the reading came back from disk");
        assert!(
            day.buckets[..BUCKETS - 1].iter().all(Option::is_none),
            "…and nothing was invented around it"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The day's far edge really is an edge: a reading older than the
    /// board falls off it on the next record, on disk too.
    #[test]
    fn a_reading_older_than_the_board_falls_off_its_edge() {
        let d = dir("prune");
        let t0 = 1000 * BUCKET_MS;
        let mut h = CpuHistory::open(&d);
        h.record(t0, 99.0);
        h.record(t0 + (BUCKETS as u64 + 1) * BUCKET_MS, 1.0);
        let reopened = CpuHistory::open(&d);
        let day = reopened.day(t0 + (BUCKETS as u64 + 1) * BUCKET_MS);
        assert_eq!(day.buckets[BUCKETS - 1], Some(1.0));
        assert!(
            day.buckets[..BUCKETS - 1].iter().all(Option::is_none),
            "the day-old reading is off the board, not lurking under it"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// An unreadable line in the file is dropped, not read as a
    /// neighbouring value — and the lines around it survive.
    #[test]
    fn a_mangled_line_is_dropped_without_taking_the_file_with_it() {
        let d = dir("mangle");
        std::fs::write(d.join("cpu"), "700 40.0 2\nnot a line\n701 10.0 1\n").unwrap();
        let h = CpuHistory::open(&d);
        let day = h.day(701 * BUCKET_MS);
        assert_eq!(day.buckets[BUCKETS - 1], Some(10.0));
        assert_eq!(day.buckets[BUCKETS - 2], Some(20.0));
        let _ = std::fs::remove_dir_all(&d);
    }
}
