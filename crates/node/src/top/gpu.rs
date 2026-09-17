//! What each process is doing to the graphics cards, by pid.
//!
//! The machine card's process table asks two questions the CPU columns
//! cannot answer: **which process is on the GPU, and which one is
//! holding its memory.** On a box with two 3090s those are the columns
//! somebody actually opens the card for — a card can read 0% busy while
//! holding 43 GiB, and only the memory says why the next job will not
//! start (`vitals::gpu::nvidia` measured exactly that on turing).
//!
//! # Only one platform can answer, and the other one says so
//!
//! NVIDIA's driver reports per-process figures through NVML. macOS
//! reports **none** — the per-process accounting simply is not in
//! IOKit's accelerator statistics, and neither `ioreg` nor
//! `powermetrics` breaks the shared window down by pid. So on a Mac
//! every row's GPU columns are `None`, which the card draws as an empty
//! cell.
//!
//! **`None` is not zero, and the whole design rests on that.** A zero
//! would say "this process is not using the GPU" on a machine that
//! never looked — and the reader would believe it, because a zero in a
//! column of numbers is a reading. Every failure below (no driver, a
//! card too old for the call, a call that errors) lands on absence, and
//! absence draws as nothing.
//!
//! # Which way this can be wrong
//!
//! - **A process on two cards is summed**, the way `cpu_pct` sums over
//!   cores: 200 means both cards, not a bug.
//! - **Within one card, compute and graphics contexts are `max`ed, not
//!   added.** NVML lists a pid under both when it has both, and the two
//!   figures overlap in ways the library does not describe; taking the
//!   larger cannot report more memory than the card holds, which adding
//!   could.
//! - **Utilisation is sampled by the driver, not by khor.** NVML hands
//!   back whatever samples it has buffered since a timestamp; khor
//!   remembers the newest timestamp per card so a later ask reads the
//!   window since the last one rather than replaying the buffer. A card
//!   that was idle for the whole window reports nothing at all — which
//!   is the one place absence and zero genuinely coincide, and it draws
//!   as empty rather than as 0%.

use std::collections::HashMap;

/// One process's use of this machine's graphics cards.
#[derive(Default, Clone, Copy)]
pub(super) struct ProcGpu {
    /// Summed utilisation over the cards this pid is on, percent.
    pub pct: Option<f32>,
    /// Summed memory held over those cards, bytes.
    pub mem: Option<u64>,
}

#[cfg(target_os = "linux")]
pub(super) fn by_pid() -> HashMap<u32, ProcGpu> {
    nvidia::by_pid()
}

#[cfg(not(target_os = "linux"))]
pub(super) fn by_pid() -> HashMap<u32, ProcGpu> {
    HashMap::new()
}

#[cfg(target_os = "linux")]
mod nvidia {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use nvml_wrapper::enums::device::UsedGpuMemory;

    use super::ProcGpu;

    /// The newest sample timestamp seen per card, so each ask reads the
    /// window since the last one. Without it every ask would replay the
    /// driver's whole buffer and report a process that was busy a
    /// minute ago as busy now.
    static SEEN: Mutex<Option<HashMap<u32, u64>>> = Mutex::new(None);

    pub(super) fn by_pid() -> HashMap<u32, ProcGpu> {
        let mut out: HashMap<u32, ProcGpu> = HashMap::new();
        // The same handle the card readings use — opened once, and a
        // failure to open is cached as "no NVIDIA here"
        // (`vitals::gpu::nvidia` carries that judgment).
        let Some(nvml) = crate::vitals::gpu::nvidia::handle() else {
            return out;
        };
        let Ok(count) = nvml.device_count() else {
            return out;
        };
        let mut seen = SEEN.lock().unwrap_or_else(|p| p.into_inner());
        let seen = seen.get_or_insert_with(HashMap::new);
        for index in 0..count {
            let Ok(card) = nvml.device_by_index(index) else {
                continue;
            };
            // Memory: this card's own view, compute and graphics
            // contexts `max`ed (see the module head), then added to
            // whatever the other cards said about the same pid.
            let mut here: HashMap<u32, u64> = HashMap::new();
            for list in [
                card.running_compute_processes(),
                card.running_graphics_processes(),
            ] {
                for p in list.unwrap_or_default() {
                    if let UsedGpuMemory::Used(bytes) = p.used_gpu_memory {
                        let slot = here.entry(p.pid).or_insert(0);
                        *slot = (*slot).max(bytes);
                    }
                }
            }
            for (pid, bytes) in here {
                let row = out.entry(pid).or_default();
                row.mem = Some(row.mem.unwrap_or(0) + bytes);
            }
            // Utilisation: the samples since the last ask. A card that
            // does not support the call is skipped whole — its
            // processes keep whatever memory said, and their share
            // stays absent rather than becoming a zero.
            let since = seen.get(&index).copied();
            let Ok(samples) = card.process_utilization_stats(since) else {
                continue;
            };
            // The newest sample per pid is the reading; older ones in
            // the same buffer are the same process earlier in the
            // window, and summing them would report 400% for a process
            // that held one card.
            let mut newest: HashMap<u32, (u64, u32)> = HashMap::new();
            let mut high = since.unwrap_or(0);
            for s in samples {
                high = high.max(s.timestamp);
                let slot = newest.entry(s.pid).or_insert((0, 0));
                if s.timestamp >= slot.0 {
                    *slot = (s.timestamp, s.sm_util);
                }
            }
            if high > 0 {
                seen.insert(index, high);
            }
            for (pid, (_, sm)) in newest {
                // Off-scale is evidence the wrong thing was read; the
                // figure is dropped rather than clamped (the card
                // reading's own scar).
                if sm > 100 {
                    continue;
                }
                let row = out.entry(pid).or_default();
                row.pct = Some(row.pct.unwrap_or(0.0) + sm as f32);
            }
        }
        out
    }
}
