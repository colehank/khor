//! Linux: NVIDIA cards, through the driver's own library.
//!
//! # Why a shared library is not an external command
//!
//! NVML ships with the NVIDIA driver, and `nvml-wrapper` opens
//! `libnvidia-ml.so` with `libloading` at run time rather than linking
//! against it. So khor on Linux links against nothing NVIDIA, runs
//! nothing, and needs nothing installed that was not already installed by
//! whoever installed the GPU driver. **The thing khor promises not to
//! require is something the user would have to go and get** — a driver's
//! own library on a machine with that driver is not that.
//!
//! `nvidia-smi` is the same information behind a process spawn, and it is
//! this module's control rather than its implementation, exactly as
//! `ioreg` is on the macOS side.
//!
//! # One build cannot ask at all, and that is an install decision
//!
//! **A fully static binary has no dynamic loader**, so `libloading` — and
//! therefore NVML — can never open anything. Measured side by side on one
//! machine at one moment (two RTX A5000, driver 580.126.09): the
//! glibc-linked build reported `GPU 0% / 2 卡 显存 43.0G / 48.0G`, the
//! `x86_64-unknown-linux-musl` build reported no GPU line at all.
//!
//! Both answers are this module's `None`, and that is the problem: the
//! rule below ("every failure lands on an absent field") is what makes
//! absence safe, and it holds only while absence means *this machine has
//! no card*. A static build turns it into *this build cannot ask*, on a
//! machine with two of them — the neighbouring value the ledger keeps
//! warning about, arriving through the linker instead of through a
//! branch.
//!
//! It is not fixed here, because it cannot be: no branch inside this file
//! can make a static binary dlopen. It is fixed **where the build is
//! chosen** — `scripts/onset.sh` ships the glibc build by default and
//! refuses to send a static one to a machine whose driver is installed.
//!
//! # Every failure lands on an absent field
//!
//! No driver, no card, a card that will not answer, a library that is not
//! there: all of them return `None`, and none of them return a zero. That
//! is the parent module's one rule, and here it is also what makes the
//! module safe to ship without the real-hardware run below — **the ways
//! it can be wrong all end in "khor did not report a GPU", which is
//! exactly what a machine without one looks like.** It cannot invent a
//! ring.
//!
//! # VERIFIED ON REAL HARDWARE, 2026-08-17 — against its own prediction
//!
//! An earlier revision of this header said "this has never run against a
//! GPU" and recorded predictions so the eventual run would compare
//! against a claim rather than against whatever came out. The run
//! happened the same day (user-approved, ~3 s on turing's two RTX A5000):
//! this module compiled **verbatim** into a probe binary — the source
//! symlinked, not copied — cross-built from the Mac (`zig cc` pinned to
//! turing's glibc 2.31; the survey's "no cross linker" was true when
//! written and fixed the same day), and run beside the same `nvidia-smi`
//! control the macOS side uses.
//!
//! Prediction vs. run:
//!
//! - `cards == 2` — hard prediction, **found 2**;
//! - memory total ≈ 48 GiB, two cards of 24564 MiB — hard, **found
//!   51514441728 bytes = 49128.0 MiB exactly**, digit for digit twice
//!   24564;
//! - memory used ≈ 43 GiB — soft (somebody else's jobs own it), **found
//!   43999.5 MiB** against the control's 21811 + 22189 = 44000 MiB; the
//!   half-MiB is the control printing each card rounded, not a
//!   discrepancy.
//!
//! Both cards again read **0% utilisation while holding 43 GiB**, live
//! this time — kept as the reason [`Gpu::mem`] exists at all: the
//! utilisation said idle and the memory said there is no room, and only
//! one of those two tells somebody why their job will not start.
//!
//! **What the run did not exercise**: utilisation was 0.0 on both cards,
//! so the averaging in `super::across` has still only run against zeros
//! on real hardware (its unit tests cover the rest). A busy-GPU run
//! would close that; nothing observed argues it is wrong.
//!
//! The fleet survey that found the hardware (read-only over ssh, same
//! day): **aliyun** is QEMU's virtual VGA — no `libnvidia-ml`, no
//! `/dev/nvidia*`, no GPU; **turing** carries the two A5000s, driver
//! 580.126.09, no Rust toolchain, home on NFS — which is why the run
//! went as a cross-built probe binary in `/tmp`, deleted in the same
//! shell that ran it.
//!
//! # Only what NVML states, and nothing derived from it
//!
//! Every figure below is reported as the library gives it. There is no
//! arithmetic here beyond summing across cards, and in particular `used`
//! is **read**, not computed from `total - free` — the note at that line
//! says why the disk reading's opposite choice does not transfer. The
//! rule matters most for code that cannot be run: **a derivation is a
//! claim, and an unverified claim about hardware nobody here can see is
//! exactly the thing that should not ship.**

use std::sync::OnceLock;

use khor_core::{Fill, Gpu};
use nvml_wrapper::Nvml;

/// The library handle, opened once.
///
/// Initialising NVML loads a shared library and talks to the driver, so
/// it is not something to do on every sample. **The cost is unmeasured**
/// — see the module head — which is another reason to pay it once.
///
/// The consequence of caching the *failure* too: a machine where the
/// driver was installed after khor started keeps reporting no GPU until
/// khor is restarted. That is the right trade at five-second sampling —
/// the alternative is retrying a library open forever on every desktop
/// that has no NVIDIA card at all, which is most of them.
static NVML: OnceLock<Option<Nvml>> = OnceLock::new();

/// This machine's NVIDIA cards, averaged. `None` when there is no driver,
/// no card, or nothing that would answer.
pub(super) fn sample() -> Option<Gpu> {
    let nvml = NVML.get_or_init(|| Nvml::init().ok()).as_ref()?;
    let count = nvml.device_count().ok()?;

    let mut utilisation = Vec::new();
    // `Some` while every card counted so far has also reported its
    // memory — see the note below on why one silent card poisons the sum.
    let mut memory = Some((0u64, 0u64));

    for index in 0..count {
        let Ok(card) = nvml.device_by_index(index) else {
            continue;
        };
        let Ok(rates) = card.utilization_rates() else {
            // No utilisation means this card contributes nothing at all,
            // rather than a zero that would drag the average down and
            // paint a busy machine as half idle.
            continue;
        };
        utilisation.push(rates.gpu as f32);

        // **A sum missing one card is a wrong number that looks like a
        // right one**, so it is dropped entirely rather than reported
        // short. The count and the memory then always describe the same
        // set of cards.
        match card.memory_info() {
            // **The same rule the utilisation follows one line up**, in
            // the shape memory takes: a figure that is not a possible
            // reading is evidence the wrong thing was read, so the card
            // is dropped rather than squeezed into shape (`super::across`
            // does exactly this for percentages, and it does it for both
            // platforms because it is shared). `used` above `total` is
            // that case here — a bar cannot be more than full, and
            // clamping it would hide the same class of mistake a clamp
            // once hid on the macOS side.
            Ok(info) if info.used <= info.total => {
                if let Some((used, total)) = memory.as_mut() {
                    // **Both figures as NVML reports them.** The disk
                    // reading next door derives `used` from what is free,
                    // and copying that here would be the wrong lesson: it
                    // does that because APFS reports three numbers that do
                    // not add up (purgeable space belongs to no column),
                    // so *somebody* has to decide what counts as used.
                    // NVML states `used` outright. Recomputing it would be
                    // khor overruling the driver about its own memory —
                    // and on a card holding back ECC or reserved regions
                    // the two genuinely differ.
                    //
                    // **The dividing line is whether the source answers
                    // the question**, not which arithmetic looks tidier.
                    *used += info.used;
                    *total += info.total;
                }
            }
            _ => memory = None,
        }
    }

    let (util_pct, cards) = super::across(&utilisation)?;
    Some(Gpu {
        util_pct,
        cards,
        // A total of zero is not a memory reading, it is a machine that
        // answered nothing useful.
        mem: memory.filter(|(_, total)| *total > 0).map(|(used, total)| Fill { used, total }),
    })
}
