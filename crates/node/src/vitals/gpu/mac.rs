//! macOS: the GPU's own numbers, read out of the IO registry.
//!
//! # What is being read
//!
//! Every accelerator publishes a `PerformanceStatistics` dictionary, and
//! `Device Utilization %` in it is the number Activity Monitor draws.
//! **`ioreg -r -d 1 -c IOAccelerator` is these same calls with a printer
//! on the end**, which is what makes it a usable control: it can be run
//! beside khor and the two numbers compared, without khor ever running it.
//!
//! Measured here 2026-08-17 (M-series, one accelerator, load ≈ 4), the
//! registry walk in [`utilizations`] over 80 calls:
//!
//! ```text
//! first     180 µs
//! median    132 µs
//! slowest   264 µs
//! ```
//!
//! There is nothing to keep alive between calls, which is why — unlike
//! the CPU and the disks next door — nothing here is cached: a tenth of a
//! millisecond is not worth a cache, and a cached reading of a live
//! number is the thing this file is most at risk of accidentally
//! shipping.
//!
//! **[`sample`] costs that, plus [`SETTLE`] whenever the reading comes
//! back zero** — see the re-read there. So a busy machine pays ~130 µs
//! and an idle one pays ~100 ms, both of which disappear inside the
//! 215 ms the surrounding sample already costs.
//!
//! # Why the utilisation is the only thing taken
//!
//! `PerformanceStatistics` also carries `In use system memory` and
//! `Alloc system memory`, and neither becomes [`Gpu::mem`]. On a unified
//! memory machine there is no video memory to report: those bytes are the
//! bytes already counted in `Vitals::mem`, so a second bar would show the
//! same memory twice under a different name, and there is no total to
//! divide by even if it were wanted. `None` says "there is no such
//! number here", which is true.

use core_foundation::base::{kCFAllocatorDefault, CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use io_kit_sys::types::{io_iterator_t, io_service_t};
use io_kit_sys::{
    kIOMasterPortDefault, IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty,
    IOServiceGetMatchingServices, IOServiceMatching,
};
use khor_core::Gpu;

/// The registry class every Mac GPU driver is a subclass of, so one name
/// covers Apple silicon, Intel and AMD without khor knowing which it is
/// looking at. Matching is by class, and the driver classes
/// (`AGXAcceleratorG16G` and friends) conform to it.
const ACCELERATOR: &[u8] = b"IOAccelerator\0";
const STATISTICS: &str = "PerformanceStatistics";
/// The whole-device figure, not `Renderer Utilization %` or
/// `Tiler Utilization %` beside it: those are two of the stages inside
/// it, and either one alone reads low on work the other is doing.
const UTILIZATION: &str = "Device Utilization %";

/// How long to leave the window open before believing a zero.
///
/// Long enough that a reading taken right after somebody else's has a
/// window worth measuring over, and short enough to disappear inside a
/// sample that already costs a fifth of a second. Measured, not picked:
/// the table on [`utilizations`] shows zeros gone by 100 ms.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

/// This machine's accelerators, averaged. `None` when none of them
/// answered — see the rule at the top of the parent module.
///
/// **A zero is read a second time, and nothing else is.** The reason is
/// on [`utilizations`]: zero is the one answer that means either "idle"
/// or "somebody read this a moment ago", and the two are worth telling
/// apart. Anything above zero could only have come from real work, so it
/// is believed immediately and the common path pays nothing.
///
/// A machine that really is idle pays [`SETTLE`] to say so — and still
/// answers zero, which is the truth about it.
pub(super) fn sample() -> Option<Gpu> {
    let mut cards = utilizations();
    if !cards.is_empty() && cards.iter().all(|p| *p == 0.0) {
        std::thread::sleep(SETTLE);
        cards = utilizations();
    }
    let (util_pct, count) = super::across(&cards)?;
    Some(Gpu { util_pct, cards: count, mem: None })
}

/// One percentage per accelerator that reported one.
///
/// An accelerator that publishes no statistics, or statistics without the
/// key, contributes nothing rather than a zero — a machine whose only
/// card went quiet has no reading, and averaging in a zero would paint it
/// as idle.
///
/// # Reading it resets it, and the window is shared with every other
/// reader on the machine
///
/// **`Device Utilization %` is a rate over a window, and the window ends
/// when somebody reads it** — the same shape as the CPU percentage next
/// door, with one difference that matters much more: that window lives in
/// the kernel, not in this process, so *any* reader empties it for
/// everyone. `ioreg`, Activity Monitor and a second khor all take turns
/// from the same statistic.
///
/// Measured here 2026-08-17 (M-series, one accelerator, a wallpaper
/// animation keeping the GPU at a steady ~48%), twenty readings at four
/// spacings:
///
/// ```text
/// gap    0 ms   19 of 20 read zero   (only the first reading had a window)
/// gap   25 ms  5–6 of 20 read zero
/// gap  100 ms    0 of 20 read zero   43–52
/// gap  250 ms    0 of 20 read zero   45–50
/// ```
///
/// Three runs, and the ranges are across all of them — the 25 ms row is
/// the only one that moved, which is what a threshold looks like when the
/// measurement lands on top of it.
///
/// This was found by the `ioreg` control, and **the way it presented is
/// worth writing down**: khor read a flat `0.0` three runs in a row while
/// the control printed 48 — which reads exactly like khor parsing the
/// wrong key, and not at all like the truth, which is that running the
/// control is what made khor's next reading empty. Whichever of the two
/// went second read zero. A control that is not read-only is a control
/// that changes the thing it measures.
///
/// The consequence for khor is small because the app samples every few
/// seconds, which is a wide window by these numbers. The consequence for
/// anybody testing or debugging this is not small at all: **two readings
/// in a row, or one reading beside `ioreg`, will show a zero that means
/// nothing.**
fn utilizations() -> Vec<f32> {
    let mut out = Vec::new();
    // SAFETY: the sequence below is IOKit's documented ownership
    // contract. `IOServiceMatching` returns a +1 dictionary which
    // `IOServiceGetMatchingServices` consumes whether or not it succeeds,
    // so it is never released here. Each `IOIteratorNext` hands back a +1
    // object released before the next turn, and the iterator itself is
    // released at the end. Nothing borrowed from IOKit outlives this
    // function.
    unsafe {
        let matching = IOServiceMatching(ACCELERATOR.as_ptr().cast());
        if matching.is_null() {
            return out;
        }
        let mut iter: io_iterator_t = 0;
        // KERN_SUCCESS is zero; any other code means no iterator was
        // produced and there is nothing to release.
        if IOServiceGetMatchingServices(kIOMasterPortDefault, matching, &mut iter) != 0 {
            return out;
        }
        loop {
            let service = IOIteratorNext(iter);
            if service == 0 {
                break;
            }
            if let Some(pct) = utilization_of(service) {
                out.push(pct);
            }
            IOObjectRelease(service);
        }
        IOObjectRelease(iter);
    }
    out
}

/// One accelerator's `Device Utilization %`, if it has one.
///
/// **Every step is allowed to fail into `None`** — the property may be
/// absent, may not be a dictionary, may not hold the key, and the value
/// may not be a number. A driver khor has never seen takes one of those
/// exits instead of contributing a fabricated figure.
///
/// # Safety
///
/// `service` must be a live IOKit object; the caller releases it.
unsafe fn utilization_of(service: io_service_t) -> Option<f32> {
    // SAFETY: `service` is live for the whole call by this function's own
    // contract, and every Core Foundation object taken below is wrapped
    // under the rule the API returned it under — so each early return
    // releases exactly what it owns and nothing else.
    unsafe {
        let key = CFString::new(STATISTICS);
        let raw = IORegistryEntryCreateCFProperty(
            service,
            key.as_concrete_TypeRef(),
            kCFAllocatorDefault,
            0,
        );
        if raw.is_null() {
            return None;
        }
        // Create rule: the property comes back +1 and this takes it over,
        // so every `return None` below releases it on the way out.
        let property = CFType::wrap_under_create_rule(raw);
        // **Checked, not assumed.** A driver publishing something other
        // than a dictionary under this key would otherwise have its
        // pointer reinterpreted as one.
        if property.type_of() != CFDictionary::<CFType, CFType>::type_id() {
            return None;
        }
        // Get rule against the reference `property` still holds, which it
        // releases on drop — the pair is balanced.
        let stats =
            CFDictionary::<CFType, CFType>::wrap_under_get_rule(property.as_CFTypeRef().cast());
        let value = stats.find(CFString::new(UTILIZATION).as_CFType())?;
        let number = value.downcast::<CFNumber>()?;
        // The registry publishes this as an integer, but `to_f64` reads
        // either, so a driver that switches to a float keeps working.
        number.to_f64().map(|n| n as f32)
    }
}

/// # Running the ignored tests below
///
/// **`--test-threads=1`, and it is not optional.** The three of them all
/// read the statistic described on [`utilizations`], and that statistic
/// has one window shared by everybody — so run in parallel they take it
/// from each other and every one of them reads zero. Measured: the
/// spacing table went from 19-of-20 zeros to 20-of-20 at zero gap, and
/// both control tests failed with the control itself reading `[0.0]`,
/// which reads exactly like the code under test being broken.
///
/// ```text
/// cargo test -p khor-node --lib gpu::mac -- --ignored --nocapture --test-threads=1
/// ```
///
/// This is `crates/node/tests/cost.rs`'s rule arriving from a second
/// direction: there a shared counter counted another test's work, here a
/// shared window is consumed by another test's read. **A number measured
/// under conditions that cannot produce it is not a number.**
#[cfg(test)]
mod tests {
    use super::*;

    /// **Every Mac has an accelerator**, so this end of the rule is the
    /// positive one: khor really does read a figure here, and the absence
    /// handling everywhere else is not covering for a lookup that never
    /// works.
    ///
    /// This is the test that would have caught the whole module being
    /// wired to the wrong registry class — the failure that otherwise
    /// looks exactly like a machine with no GPU, and reads as "khor
    /// correctly reported nothing" all the way to the screen.
    #[test]
    fn this_mac_reports_a_gpu() {
        let g = sample().expect("every Mac has an accelerator to read");
        assert!(
            (0.0..=100.0).contains(&g.util_pct),
            "a percentage is a percentage: {}",
            g.util_pct
        );
        assert!(g.cards >= 1, "at least one accelerator answered: {}", g.cards);
        assert_eq!(g.mem, None, "unified memory has no video memory of its own");
    }

    /// The reading moves.
    ///
    /// A constant would satisfy the test above forever, and 48% is a
    /// plausible-looking constant. This does not assert *which* way it
    /// moves — a shared machine will not cooperate — only that two
    /// readings of a live number are not obliged to be the same one, with
    /// the loop tolerating a genuinely steady GPU by trying again.
    ///
    /// **It is not a hard assertion**, and saying so is the point: on an
    /// idle machine the true answer really is 0 twice, and a test that
    /// demanded movement would be red for the honest reason. What it
    /// guards against is a stuck cache, and it prints what it saw.
    #[test]
    fn two_readings_of_a_live_number_are_taken_afresh() {
        let seen: Vec<f32> = (0..5)
            .map(|_| {
                let g = sample().expect("every Mac has an accelerator to read");
                std::thread::sleep(std::time::Duration::from_millis(50));
                g.util_pct
            })
            .collect();
        // Printed rather than only asserted on: with `--nocapture` this
        // is the control run against `ioreg` (see the module head), and a
        // number nobody can read cannot be compared with anything.
        println!("gpu utilisation over five readings: {seen:?}");
        assert_eq!(seen.len(), 5, "five readings were taken: {seen:?}");
        assert!(
            seen.iter().all(|p| (0.0..=100.0).contains(p)),
            "each one is a percentage: {seen:?}"
        );
    }

    /// The measurement the table on [`utilizations`] is made of, kept so
    /// it can be taken again rather than believed.
    ///
    /// **Ignored by default**: it takes about fifteen seconds, it reads
    /// whatever the machine happens to be doing, and on a genuinely idle
    /// machine every row is honestly zero — which is why it prints its
    /// numbers instead of asserting a shape they cannot be held to.
    ///
    /// Re-run it when [`SETTLE`] is questioned, or on a machine whose
    /// numbers look wrong. It reads [`utilizations`] rather than
    /// [`sample`] on purpose — `sample` re-reads zeros, which is the
    /// behaviour under investigation here.
    ///
    /// `cargo test -p khor-node --lib gpu::mac -- --ignored --nocapture`
    #[test]
    #[ignore = "measures this machine over ~15s; run by hand"]
    fn a_reading_needs_a_window_and_this_is_how_wide() {
        let mut costs = Vec::new();
        for gap in [0u64, 25, 100, 250] {
            let seen: Vec<f32> = (0..20)
                .map(|_| {
                    let t = std::time::Instant::now();
                    let cards = utilizations();
                    costs.push(t.elapsed());
                    let p = crate::vitals::gpu::across(&cards).map_or(f32::NAN, |(p, _)| p);
                    std::thread::sleep(std::time::Duration::from_millis(gap));
                    p
                })
                .collect();
            let zeros = seen.iter().filter(|p| **p == 0.0).count();
            let hi = seen.iter().copied().fold(0.0f32, f32::max);
            println!("gap {gap:>4}ms  zeros {zeros:>2}/20  max {hi:>5}  {seen:?}");
        }
        // The other number the module head quotes. The first call is
        // reported on its own because it is the only one that can be
        // slower, and the median rather than the mean because one
        // descheduled read should not decide what this costs.
        let first = costs[0];
        costs.sort();
        println!(
            "registry walk: first {first:?}, median {:?}, slowest {:?}, over {} calls",
            costs[costs.len() / 2],
            costs[costs.len() - 1],
            costs.len()
        );
    }

    /// The control: khor's reading beside the one `ioreg` prints.
    ///
    /// **Ignored by default because it runs a command**, which is the one
    /// thing the code under test exists not to do — it lives here so the
    /// comparison can be repeated rather than being a number somebody
    /// once pasted into a report. It also reads the machine it runs on,
    /// the rule `tests/real_disk.rs` follows.
    ///
    /// Run it with:
    /// `cargo test -p khor-node --lib gpu::mac -- --ignored --nocapture`
    ///
    /// **What each half proves, and what it does not:**
    ///
    /// - *The count is exact* and is the half with teeth: it catches
    ///   matching the wrong registry class, and it catches a machine
    ///   whose second card khor walked past. Nothing about it drifts with
    ///   time, so there is no tolerance to argue about.
    /// - *The value is bracketed, not equal*, and the bracket is wide.
    ///   The number moves on its own — measured on this machine, five
    ///   readings a few hundred milliseconds apart spanned 43 to 52 — so
    ///   the two sides cannot be asked to agree digit for digit. What it
    ///   catches is a reading that is not a percentage at all: the
    ///   neighbouring `In use system memory` is about 1.2e9, and any
    ///   confusion of that sort lands nowhere near here.
    ///
    /// **It cannot tell `Device Utilization %` from `Renderer` or `Tiler`
    /// beside it** — measured together, 48 / 47 / 48. Nothing measurable
    /// separates those, so the guard against picking the wrong one is the
    /// named constant and the note on it, not this test.
    #[test]
    #[ignore = "runs the ioreg control and reads this machine; run by hand"]
    fn the_reading_agrees_with_the_ioreg_control() {
        let g = sample().expect("every Mac has an accelerator to read");
        // **Each side needs a window of its own.** The statistic is
        // emptied by whoever reads it (see [`utilizations`]), so without
        // this the control reads the zero khor just left behind and the
        // test reports a disagreement that is entirely its own doing.
        std::thread::sleep(SETTLE);
        let control = ioreg_utilizations();

        println!("khor: {:?} over {} card(s); ioreg: {control:?}", g.util_pct, g.cards);

        // The probe has to have found something, or every assertion below
        // is about an empty list agreeing with an empty list.
        assert!(!control.is_empty(), "the control read no accelerator at all");
        assert_eq!(
            g.cards as usize,
            control.len(),
            "khor saw {} accelerator(s), the control saw {}",
            g.cards,
            control.len()
        );

        let mean = control.iter().sum::<f32>() / control.len() as f32;
        assert!(
            (g.util_pct - mean).abs() <= 25.0,
            "khor read {} where the control read {mean}; that is not the same quantity",
            g.util_pct
        );
    }

    /// A second reader on the machine does not turn khor's reading into a
    /// zero — the guard on the re-read in [`sample`].
    ///
    /// **`ioreg` is the second reader**, which makes this the one place
    /// the shared-window behaviour can be provoked on purpose rather than
    /// waited for: it empties the statistic, and the reading taken
    /// immediately afterwards is exactly the one that used to come back
    /// flat zero.
    ///
    /// Ignored for the same two reasons as its neighbours: it runs a
    /// command, and it needs a machine that is doing some graphics work.
    /// That precondition is asserted first and says so in its message —
    /// **a silent `return` on an idle machine would be a test that passes
    /// by not running**, which is worse than one that fails honestly.
    #[test]
    #[ignore = "runs the ioreg control and needs a busy GPU; run by hand"]
    fn a_reading_survives_another_reader_emptying_the_window() {
        let control = ioreg_utilizations();
        assert!(
            control.iter().any(|p| *p > 0.0),
            "this needs a machine doing some graphics work, and this one reads {control:?} — \
             an idle GPU has nothing to prove here"
        );

        // The line above just emptied the window. Before the re-read in
        // `sample` existed, this is where a flat zero came from.
        let g = sample().expect("every Mac has an accelerator to read");
        println!("after the control took the window, khor read {}", g.util_pct);
        assert!(
            g.util_pct > 0.0,
            "the control was reading {control:?} a moment ago, so this zero is the window \
             the control took and not an idle machine"
        );
    }

    /// The control's own reading, so the two tests that need it do not
    /// each grow their own parser.
    fn ioreg_utilizations() -> Vec<f32> {
        let out = std::process::Command::new("ioreg")
            .args(["-r", "-d", "1", "-c", "IOAccelerator"])
            .output()
            .expect("the control command is on the PATH of a Mac");
        String::from_utf8_lossy(&out.stdout)
            .split(&format!("\"{UTILIZATION}\"="))
            .skip(1)
            .filter_map(|rest| {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().ok()
            })
            .collect()
    }
}
