//! What is eating this machine right now: the processes at the top of
//! the CPU, the answer 进程 top on the machine card draws
//! (2026-09-02:「环下面显示进程top」).
//!
//! **Read-only, and staying that way.** No kill, no renice, no signal:
//! khor's card answers 这台机器在忙什么, and the verbs that change a
//! machine belong to a terminal a person opened on purpose (`开终端`
//! is one press away on the same card). A kill button beside a reading
//! is the reading daring you to use it.
//!
//! # Which way this can be wrong
//!
//! - **The window is "since the last ask", not a fixed second.** The
//!   same [`sysinfo`]-held-delta design as `vitals::Sampler`, and the
//!   same reason: a rate needs two looks, and whoever asks decides the
//!   gap. The first ask in a process pays
//!   [`sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`] once so a one-shot
//!   answer is a rate rather than a lifetime average — the `ps` trap
//!   (`%cpu` there is a decaying average on macOS and a lifetime one
//!   on Linux) that made shelling out the wrong road.
//! - **A figure is per core**, the way `top` prints it: a process
//!   holding four cores reads 400, not 40 — capping it at 100 would
//!   make the busiest kind of process read like an ordinary one.
//! - **Short-lived processes are invisible**: something that starts
//!   and dies inside the window contributes nothing. That is every
//!   sampler's blindness, and a card polling every few beats inherits
//!   it honestly rather than pretending to be an audit.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

/// One row of the top: what to call it, who it is, how hard it is
/// eating, and how much it holds.
///
/// Plain serde for now: the TS export lives with every other binding
/// type in `khor_core`, and this struct moves there when the wire op
/// lands (kept here first so the sampler and its tests do not wait on
/// a file another batch has open).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopRow {
    pub pid: u32,
    /// The process's own short name, as the platform reports it.
    pub name: String,
    /// CPU over the sampling window, percent **of one core** — see the
    /// module head for why it may exceed 100.
    pub cpu_pct: f32,
    /// Resident memory, bytes.
    pub mem: u64,
}

/// The process table's own `System`, apart from the vitals sampler's on
/// purpose: the two answer on different beats, and sharing one would
/// let either ask reset the other's CPU window (the shared-window trap
/// `vitals::gpu::mac` documents from the machine's other end).
static SAMPLER: OnceLock<Mutex<sysinfo::System>> = OnceLock::new();

fn refresh(sys: &mut sysinfo::System) {
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        // Dead processes leave the table — a row for a pid that is gone
        // would be a reading of nothing.
        true,
        sysinfo::ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );
}

/// The `n` hungriest processes right now, by CPU then by memory.
/// **Blocking** — the one caller inside async code goes through
/// `spawn_blocking`, the vitals rule.
pub fn sample_top(n: usize) -> Vec<TopRow> {
    let mut sys = SAMPLER
        .get_or_init(|| {
            let mut sys = sysinfo::System::new();
            // The first look, so the reading below has something to
            // subtract — `vitals::Sampler`'s arrangement.
            refresh(&mut sys);
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            Mutex::new(sys)
        })
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    refresh(&mut sys);
    let mut rows: Vec<TopRow> = sys
        .processes()
        .values()
        .map(|p| TopRow {
            pid: p.pid().as_u32(),
            name: p.name().to_string_lossy().into_owned(),
            cpu_pct: p.cpu_usage(),
            mem: p.memory(),
        })
        .collect();
    // CPU first; memory breaks the tie so an idle machine still ranks
    // by something a person recognises, and the whole thing is stable
    // on pid so two asks in a quiet second agree.
    rows.sort_by(|a, b| {
        b.cpu_pct
            .total_cmp(&a.cpu_pct)
            .then(b.mem.cmp(&a.mem))
            .then(a.pid.cmp(&b.pid))
    });
    rows.truncate(n);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reading is of a real machine, so the assertions are the ones
    /// that hold on every machine (`vitals`' rule) — and the positive
    /// half is this very test's own spin: a thread this process pins
    /// must put this process on its own top, with a rate that says so.
    /// That is the finder proving itself before anything trusts a
    /// quiet list.
    #[test]
    fn the_top_finds_the_process_doing_the_work() {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let spin = std::thread::spawn(move || {
            let mut n: u64 = 0;
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                n = n.wrapping_add(1);
            }
            n
        });
        // A window for the spin to be visible in: the first ask arms
        // the sampler, the second reads a real delta.
        let _ = sample_top(8);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let rows = sample_top(usize::MAX);
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = spin.join();

        assert!(!rows.is_empty(), "a live machine runs processes");
        let me = std::process::id();
        let mine = rows.iter().find(|r| r.pid == me).expect("this very process is on the table");
        assert!(
            mine.cpu_pct > 0.0,
            "one thread was pinned for the whole window and reads {}",
            mine.cpu_pct
        );
        assert!(mine.mem > 0, "a running process holds memory");
        assert!(!mine.name.is_empty(), "a process has a name");
        for r in &rows {
            assert!(r.cpu_pct >= 0.0, "a rate is not negative: {} on {}", r.cpu_pct, r.name);
        }
    }

    /// The cut ranks by hunger: asking for one row returns the top of
    /// the same table, not an arbitrary member of it.
    #[test]
    fn the_cut_keeps_the_hungriest() {
        let all = sample_top(usize::MAX);
        let one = sample_top(1);
        assert_eq!(one.len(), 1.min(all.len()));
        if let (Some(first), Some(head)) = (one.first(), all.first()) {
            // Two asks are two windows, so the *figures* may differ;
            // what must hold is that a single-row answer is a top, not
            // a tail: its one row carries at least as much CPU as the
            // full table's median.
            let mut cpus: Vec<f32> = all.iter().map(|r| r.cpu_pct).collect();
            cpus.sort_by(f32::total_cmp);
            let median = cpus[cpus.len() / 2];
            assert!(
                first.cpu_pct >= median || first.pid == head.pid,
                "a one-row top ({} at {}) reads under the median {}",
                first.name,
                first.cpu_pct,
                median
            );
        }
    }
}
