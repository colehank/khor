//! The last day of this machine's four readings, five minutes at a
//! stride — what the chart under the machine card's dials draws
//! (2026-09-02:「最近24小时cpu」; 2026-09-03:「GPU、CPU、Mem、Disk 都
//! 绘制」).
//!
//! **Four series, one file, and three of them are optional.** CPU is
//! always readable; memory is too; a machine with no card khor can ask
//! has no GPU figure, and a filesystem it cannot read has no disk one.
//! Absent is written as absent, so a machine that grew a GPU last week
//! has a day that says so instead of a week of zeros.
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
    /// Per bucket: the CPU sum, the sample count, then the memory, GPU
    /// and disk sums — each `None` when no beat in that bucket could
    /// read it. A bucket written by an older recorder simply has fewer
    /// columns, and the ones it never knew stay absent rather than
    /// becoming zero.
    buckets: BTreeMap<u64, Bucket>,
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
                    // The tail columns are memory, GPU and disk, in
                    // that order — a line stops wherever the recorder
                    // that wrote it stopped, and everything past its
                    // end is absent. Positional because the file is
                    // two hundred short lines the recorder rewrites
                    // whole; a keyed format would be a parser to keep.
                    let mut tail = || it.next().and_then(|m| m.parse::<f64>().ok());
                    let (mem, gpu, disk, vram) = (tail(), tail(), tail(), tail());
                    buckets.insert(idx, Bucket { cpu: sum, n, mem, gpu, disk, vram });
                }
            }
        }
        CpuHistory { buckets, file }
    }

    /// Takes one reading into its bucket and prunes the day's far edge.
    /// Writes through on every call: the file is at most [`BUCKETS`]
    /// short lines, and a crash between beats then costs one beat, not
    /// a day.
    pub fn record(&mut self, at_ms: u64, r: Reading) {
        let idx = at_ms / BUCKET_MS;
        let slot = self.buckets.entry(idx).or_insert(Bucket::default());
        slot.cpu += f64::from(r.cpu_pct);
        slot.n += 1;
        slot.mem = Some(slot.mem.unwrap_or(0.0) + f64::from(r.mem_pct));
        // **A reading nobody could take adds nothing, not zero.** The
        // sum stays absent until some beat in this bucket manages one,
        // and the mean below divides by the bucket's beat count — so a
        // bucket where the GPU answered once out of five reads as that
        // one answer's share, which is the honest thing a mean of what
        // was measured can say.
        if let Some(g) = r.gpu_pct {
            slot.gpu = Some(slot.gpu.unwrap_or(0.0) + f64::from(g));
        }
        if let Some(d) = r.disk_pct {
            slot.disk = Some(slot.disk.unwrap_or(0.0) + f64::from(d));
        }
        if let Some(v) = r.vram_pct {
            slot.vram = Some(slot.vram.unwrap_or(0.0) + f64::from(v));
        }
        let floor = idx.saturating_sub(BUCKETS as u64);
        self.buckets.retain(|k, _| *k > floor);
        let text: String = self
            .buckets
            .iter()
            .map(|(k, b)| {
                let mut line = format!("{k} {} {}", b.cpu, b.n);
                // **Every column is written, absent ones as `-`.**
                // Stopping at the first absent field was the first
                // version and it was wrong: a machine with no GPU but a
                // readable disk (every card-less Linux box) writes
                // `mem` then nothing, and the disk figure it *did*
                // measure never reaches the file. A placeholder costs
                // one character and `parse::<f64>` already reads it as
                // absent, so the shorter lines an older recorder wrote
                // still decode by running out of columns.
                for v in [b.mem, b.gpu, b.disk, b.vram] {
                    match v {
                        Some(x) => line.push_str(&format!(" {x}")),
                        None => line.push_str(" -"),
                    }
                }
                line.push('\n');
                line
            })
            .collect();
        if let Some(parent) = self.file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.file, text);
    }

    /// The last day as a face draws it, `now` at the right edge.
    pub fn day(&self, now_ms: u64) -> CpuDay {
        let newest = now_ms / BUCKET_MS;
        let at = |back: u64| newest.wrapping_sub(BUCKETS as u64 - 1 - back);
        let mean = |pick: fn(&Bucket) -> Option<f64>| -> Vec<Option<f32>> {
            (0..BUCKETS as u64)
                .map(|back| {
                    let b = self.buckets.get(&at(back))?;
                    if b.n == 0 {
                        return None;
                    }
                    pick(b).map(|sum| (sum / f64::from(b.n)) as f32)
                })
                .collect()
        };
        CpuDay {
            bucket_ms: BUCKET_MS,
            now_ms,
            buckets: mean(|b| Some(b.cpu)),
            mem: mean(|b| b.mem),
            gpu: mean(|b| b.gpu),
            disk: mean(|b| b.disk),
            vram: mean(|b| b.vram),
        }
    }
}

/// One bucket's running sums. Absent means no beat in this bucket could
/// read that figure — see the type's field docs.
#[derive(Default, Clone, Copy)]
struct Bucket {
    cpu: f64,
    n: u32,
    mem: Option<f64>,
    gpu: Option<f64>,
    disk: Option<f64>,
    vram: Option<f64>,
}

/// One beat's four readings, as the recorder took them.
///
/// A struct rather than four positional arguments because two of them
/// are optional percentages of the same shape — and `record(t, 12.0,
/// 40.0, None, Some(96.0))` is a call nobody can read, let alone check.
#[derive(Clone, Copy)]
pub struct Reading {
    pub cpu_pct: f32,
    pub mem_pct: f32,
    /// The machine's graphics use, when khor can ask at all.
    pub gpu_pct: Option<f32>,
    /// How full the filesystem khor lives on is — the same reading the
    /// disk dial aggregates, so the day and the dial agree.
    pub disk_pct: Option<f32>,
    /// How full the machine's cards are, summed. `None` on a machine
    /// with no card khor can ask **and on a unified-memory one**, where
    /// those bytes are already `mem_pct` — see `khor_core::CpuDay::vram`.
    pub vram_pct: Option<f32>,
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

    /// **A gap in one series does not take the next one with it.**
    ///
    /// The file is positional, so the question is what happens to a
    /// machine that reads a disk but has no GPU to read — every
    /// card-less Linux box. The first version of the writer stopped at
    /// the first absent column and silently dropped the disk figure it
    /// had actually measured; this reads it back through a second
    /// instance (the restart rule: the verb in the claim has to happen
    /// in the check) and asserts the hole is where it belongs and the
    /// reading after it survived.
    #[test]
    fn an_absent_series_does_not_swallow_the_one_behind_it() {
        let d = dir("placeholder");
        let t0 = 500 * BUCKET_MS;
        {
            let mut h = CpuHistory::open(&d);
            // No card to ask, but a readable disk **and** a readable
            // card memory behind it — so the hole is in the middle of
            // the tail, which is the arrangement a positional format
            // gets wrong.
            h.record(
                t0,
                Reading { cpu_pct: 10.0, mem_pct: 40.0, gpu_pct: None, disk_pct: Some(96.0), vram_pct: Some(71.0) },
            );
        }
        // A second instance, so what is asserted is what reached disk.
        let day = CpuHistory::open(&d).day(t0);
        let last = day.buckets.len() - 1;
        assert_eq!(day.buckets[last], Some(10.0), "the reader finds what is there");
        assert_eq!(day.mem[last], Some(40.0));
        assert_eq!(day.gpu[last], None, "no card, no figure — and not a zero");
        assert_eq!(day.disk[last], Some(96.0), "the disk was measured, so it must have survived the write");
        assert_eq!(day.vram[last], Some(71.0), "and so must the column after it");
        let _ = std::fs::remove_dir_all(&d);
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
        h.record(t0, Reading { cpu_pct: 10.0, mem_pct: 40.0, gpu_pct: None, disk_pct: None, vram_pct: None });
        h.record(t0 + 1000, Reading { cpu_pct: 30.0, mem_pct: 40.0, gpu_pct: None, disk_pct: None, vram_pct: None });
        h.record(t0 + 2 * BUCKET_MS, Reading { cpu_pct: 50.0, mem_pct: 40.0, gpu_pct: None, disk_pct: None, vram_pct: None }); // one whole bucket skipped
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
            h.record(t0, Reading { cpu_pct: 42.0, mem_pct: 40.0, gpu_pct: None, disk_pct: None, vram_pct: None });
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
        h.record(t0, Reading { cpu_pct: 99.0, mem_pct: 40.0, gpu_pct: None, disk_pct: None, vram_pct: None });
        h.record(t0 + (BUCKETS as u64 + 1) * BUCKET_MS, Reading { cpu_pct: 1.0, mem_pct: 40.0, gpu_pct: None, disk_pct: None, vram_pct: None });
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

/// Memory rides the same buckets as CPU (2026-09-03, the card's dashed
/// line): recorded beside it, averaged the same way, and **absent — not
/// zero — for a bucket written before the recorder knew about memory**,
/// which is what an older `cpu` file's three-column lines are.
#[cfg(test)]
mod memory_day {
    use super::*;

    fn fresh_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("khor-history-mem-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn memory_is_recorded_beside_cpu_and_averaged_per_bucket() {
        let d = fresh_dir("avg");
        let mut h = CpuHistory::open(&d);
        let at = 10 * BUCKET_MS;
        h.record(at, Reading { cpu_pct: 50.0, mem_pct: 30.0, gpu_pct: None, disk_pct: None, vram_pct: None });
        h.record(at + 1_000, Reading { cpu_pct: 70.0, mem_pct: 50.0, gpu_pct: None, disk_pct: None, vram_pct: None });
        let day = h.day(at);
        let last = BUCKETS - 1;
        assert_eq!(day.buckets[last], Some(60.0), "cpu is the bucket's mean");
        assert_eq!(day.mem[last], Some(40.0), "memory is the bucket's mean too");
        assert_eq!(day.mem.len(), day.buckets.len(), "one memory slot per cpu slot");
        // And it comes back the same from a second instance — the file
        // carries the fourth column.
        let again = CpuHistory::open(&d).day(at);
        assert_eq!(again.mem[last], Some(40.0));
    }

    #[test]
    fn a_bucket_from_before_memory_draws_as_absence_not_zero() {
        let d = fresh_dir("old");
        std::fs::create_dir_all(&d).unwrap();
        // A line the previous recorder wrote: index, cpu sum, count — no memory.
        std::fs::write(d.join("cpu"), "10 120.0 2\n").unwrap();
        let day = CpuHistory::open(&d).day(10 * BUCKET_MS);
        let last = BUCKETS - 1;
        assert_eq!(day.buckets[last], Some(60.0), "the old cpu reading still reads");
        assert_eq!(day.mem[last], None, "and its memory is a gap, not 0%");
    }
}
