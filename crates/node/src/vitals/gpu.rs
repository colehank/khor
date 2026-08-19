//! The graphics hardware's share of a reading.
//!
//! # The one rule
//!
//! **A reading that cannot be taken leaves no field.** Everything below
//! returns `None` rather than a zero, a guess, or a "0 / 0", because
//! [`khor_core::Gpu`] is painted only when it is there and an invented
//! ring is worse than a missing one. Each platform's `sample` is written
//! so that every way of failing — no hardware, no driver, a key that
//! moved, a number that is not a number — lands on the same `None`.
//!
//! # Why there is no trait here
//!
//! Two real implementations and no abstraction over them is deliberate,
//! the same seam rule the vendor adaptors follow: **the signature is the
//! seam**. `fn sample() -> Option<Gpu>` is the whole contract, and a
//! trait would only add a name for it. There is nothing a caller can do
//! with a macOS GPU that it cannot do with a Linux one, and nothing khor
//! wants to ask that is not this one question.
//!
//! # Why khor carries this itself
//!
//! `silicon-monitor` (5.1.0) was measured against the "prefer a mature
//! crate" rule on 2026-08-17 and **not adopted**, on three findings, each
//! of which is on its own enough:
//!
//! - **Its macOS backend shells out.** `src/gpu/apple.rs` runs
//!   `system_profiler` and `sysctl` to enumerate, and reads utilisation by
//!   parsing `powermetrics` — which additionally wants root. That is the
//!   one thing this batch exists to avoid, and it is the platform khor
//!   runs on today.
//! - **It is AGPL-3.0-or-later.** khor is distributed to users; a copyleft
//!   that reaches the network is a licensing decision for the whole
//!   product, not a dependency choice.
//! - **Its default feature is `full`**, which pulls a TUI and a GUI
//!   (`clap`, `ratatui`, `crossterm`, `reqwest`, `eframe`, `egui`,
//!   `egui_plot`) into a tree that wants one integer.
//!
//! What it would have supplied on Linux is `nvml-wrapper`, which is used
//! here directly. Its AMD backend reads the same sysfs file the note in
//! docs/handoff describes, which is why not adopting it costs nothing
//! when that lands.
//!
//! # What is here, and what is deliberately not
//!
//! | platform | how | verified |
//! |---|---|---|
//! | macOS | IOKit registry, in process | on this machine, against `ioreg` |
//! | Linux + NVIDIA | NVML, loaded at run time | yes — two RTX A5000, `nvidia.rs` |
//! | Linux + AMD | — | not written |
//! | Windows | — | not written |
//!
//! **AMD is not written because there is no AMD card to run it on.** The
//! route is known and small (`/sys/class/drm/card*/device/gpu_busy_percent`
//! is a plain file read, no library at all), and it was checked for on
//! both Linux machines in the fleet on 2026-08-17: neither has the file,
//! because neither has an AMD GPU. Writing it would mean shipping a
//! reader that has never seen its own hardware — and unlike the NVIDIA
//! one, there is no machine on which that could be fixed later this week.
//! **Trigger: a machine with an AMD GPU.** Until then a khor on an AMD
//! Linux box reports no GPU, which is the same answer it gives for
//! Windows and for a Mac whose driver went quiet — an absence, never a
//! neighbouring value.

use khor_core::Gpu;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "linux")]
mod nvidia;

/// This machine's graphics hardware, or `None` when khor cannot say.
///
/// **Blocking**, like everything else a reading is made of — the one
/// caller inside async code goes through `spawn_blocking` (`link.rs`).
/// Measured cost is on each platform's own `sample`.
///
/// Platforms with no implementation answer `None`, which is the truthful
/// answer rather than a placeholder: khor on Windows genuinely cannot say
/// yet, and a machine reporting no GPU field is exactly what that means.
pub(super) fn sample() -> Option<Gpu> {
    #[cfg(target_os = "macos")]
    return mac::sample();
    #[cfg(target_os = "linux")]
    return nvidia::sample();
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return None;
}

/// The average of what the cards reported, as a percentage, with however
/// many of them answered.
///
/// Shared by the platforms because the flattening is a judgment rather
/// than a detail — see [`khor_core::Gpu::util_pct`] — and two copies of a
/// judgment drift. `None` for an empty list: a machine where no card
/// answered has no reading, which is not the same as a machine at 0%.
///
/// # A figure off the scale is discarded, not clamped
///
/// **A number outside 0–100 is not a percentage that needs squeezing, it
/// is evidence that something other than a percentage was read** — so the
/// card it came from contributes nothing, exactly like a card that
/// answered nothing at all.
///
/// This was a clamp first, and the clamp was caught doing real harm.
/// Pointing the macOS reader at the neighbouring `In use system memory`
/// key — one word changed — made it read 1.2e9, which `clamp` turned into
/// a flawless `100.0`: **a machine reading the wrong field looked exactly
/// like a machine with its GPU pegged**, and the ordinary test suite
/// stayed green because 100 is a perfectly good percentage. Only the
/// `ioreg` control noticed, and controls are run by hand.
///
/// Discarding costs nothing when nothing is wrong and turns that silent
/// green into an absent field, which is this module's one rule.
#[cfg_attr(not(any(target_os = "macos", target_os = "linux")), allow(dead_code))]
fn across(cards: &[f32]) -> Option<(f32, u32)> {
    let usable: Vec<f32> = cards.iter().copied().filter(|p| (0.0..=100.0).contains(p)).collect();
    if usable.is_empty() {
        return None;
    }
    let mean = usable.iter().sum::<f32>() / usable.len() as f32;
    Some((mean, usable.len() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reading is of real hardware, so this asserts what holds on any
    /// machine — including one with no GPU at all, where `None` is the
    /// right answer and not a failure (the rule `tests/real_disk.rs`
    /// follows: a hard-coded number here goes red on somebody else's
    /// laptop).
    ///
    /// **The macOS half is stronger and lives in `mac.rs`**, where there
    /// is a machine to be sure about.
    #[test]
    fn a_gpu_reading_is_a_percentage_or_nothing_at_all() {
        let Some(g) = sample() else {
            return;
        };
        assert!(
            (0.0..=100.0).contains(&g.util_pct),
            "a percentage is a percentage: {}",
            g.util_pct
        );
        assert!(g.cards >= 1, "a reading came from at least one card: {}", g.cards);
        if let Some(m) = g.mem {
            assert!(m.total > 0, "reported video memory has a size: {m:?}");
            assert!(m.used <= m.total, "used cannot exceed the total: {m:?}");
        }
    }

    /// No cards means no reading, and one card means that card.
    ///
    /// The empty case is the one worth a test: an `across` that answered
    /// `Some(0.0)` there would put a machine khor cannot read at 0% on the
    /// screen, which is the whole failure this module is shaped against.
    #[test]
    fn nothing_to_average_is_an_absence_rather_than_a_zero() {
        assert_eq!(across(&[]), None);
        assert_eq!(across(&[42.0]), Some((42.0, 1)));
        assert_eq!(across(&[100.0, 0.0]), Some((50.0, 2)));
    }

    /// A figure that is not a percentage is dropped rather than squeezed
    /// into one — see the note on [`across`], where a clamp was measured
    /// dressing a wrong-field read as a busy GPU.
    ///
    /// **The last case is the one that matters**: one good card beside
    /// one nonsensical one reports the good card alone, and says it came
    /// from one card. Averaging the pair would move the figure, and
    /// keeping the count at two would claim a reading khor does not have.
    #[test]
    fn a_figure_off_the_scale_is_not_a_percentage_at_all() {
        assert_eq!(across(&[140.0]), None);
        assert_eq!(across(&[-3.0]), None);
        assert_eq!(across(&[1.2e9]), None);
        assert_eq!(across(&[40.0, 1.2e9]), Some((40.0, 1)));
    }
}
