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
//! # NOT VERIFIED ON REAL HARDWARE — and what would verify it
//!
//! **This has never run against a GPU.** Saying so plainly because a
//! compile is not a verification (docs/handoff 编译级验证不算验), and
//! because the next person reading this will otherwise assume the numbers
//! below came from a run.
//!
//! The fleet was surveyed on 2026-08-17, read-only over ssh:
//!
//! - **aliyun**: `Cirrus Logic GD 5446` — QEMU's virtual VGA. No
//!   `libnvidia-ml`, no `/dev/nvidia*`, no `gpu_busy_percent`. No GPU.
//! - **turing**: **two NVIDIA RTX A5000**, driver 580.126.09,
//!   `libnvidia-ml.so.1` present. Real hardware — but no Rust toolchain,
//!   `load average 45`, 43 logged-in users, and a home on NFS. Building
//!   khor there would mean installing a toolchain on somebody else's
//!   machine and pushing load onto a room full of people who are waiting
//!   on their own jobs. Cross-compiling from the Mac was checked too: no
//!   linux target, no cross linker, no container runtime.
//!
//! So the trigger for a real run is: **a Linux machine with an NVIDIA GPU
//! that khor may build on.** When there is one, run
//! `cargo test -p khor-node --lib gpu` there and compare against
//! `nvidia-smi --query-gpu=index,utilization.gpu,memory.used,memory.total
//! --format=csv`, the same shape of control the macOS side uses.
//!
//! **What that run should find on turing**, recorded now so the
//! comparison is against a prediction rather than against whatever comes
//! out:
//!
//! - `cards == 2` — **a hard prediction**, it does not drift;
//! - memory **total ≈ 48 GiB** (two cards of 24564 MiB) — also hard, it
//!   is a property of the cards;
//! - memory **used ≈ 43 GiB** — *not* hard: that is what the two cards
//!   were holding during the survey (21811 and 22189 MiB), and somebody
//!   else's job owns it. A run that finds a different `used` has found
//!   nothing wrong; a run that finds a different `total` or `cards` has.
//!
//! (All three in GiB, the base the CLI and the card both print in:
//! 44000 MiB of 49128 MiB.)
//!
//! At the time of the survey both cards read **0% utilisation while
//! holding 43 GiB**, which is worth keeping as the reason [`Gpu::mem`]
//! exists at all: **the utilisation said idle and the memory said there
//! is no room**, and only one of those two would have told somebody why
//! their job will not start.
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
