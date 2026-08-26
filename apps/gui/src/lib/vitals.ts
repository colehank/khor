// What colour a machine's readings draw in, and how they stack.
//
// Shared because two screens paint the same readings — a row of separate
// gauges on the machine's card, one nested stack on its row in the list —
// and a machine that looked strained on one and calm on the other would
// be two machines.
import type { DeviceRow, Strain } from "@/api";
import type { Track } from "@/components/Ring";

/**
 * What colour a gauge draws in.
 *
 * **Neutral unless the node said otherwise, and there is no ramp.** A
 * busy CPU is a machine doing the work you asked for, not a warning, so
 * utilisation carries no hue at all; 内存, 磁盘 and 显存 have a "full"
 * to be near, and only they are handed a `Strain` (显存 joined in
 * 仪表环出声批 — it was the one the gauges whispered about while it
 * screamed: a 90% VRAM ring wearing a 2% CPU's grey).
 *
 * ── Why this is not mandala's `severityColor()` ──────────────────────
 *
 * The ring this file serves was ported from mandala, and its colour
 * function was deliberately left behind. Two reasons, and the second one
 * is the one that would have been rediscovered the hard way:
 *
 * **It would have reversed a standing user ruling.** `severityColor()`
 * grades a reading through four colours, ok → warn → serious → crit. But
 * on 2026-08-17 the user ruled that strain gets **one** hue, and that
 * the step from tight to critical is *weight*, not a second colour —
 * because 中断 already owns amber and 失败 owns red, and a 95% disk
 * wearing either would dress a machine up as a session. So the
 * escalation lives on the label's own `font-label`, which is where it
 * already was: one channel, not two.
 *
 * **And it would have copied a judgment back out of Rust.** It decides
 * the thresholds in the frontend, from `used / total`. khor answers that
 * question in the node (`Strain`), whose own doc says why: the
 * thresholds *are* the fact, and two frontends eyeballing the same two
 * numbers would disagree about the same machine. This function reads the
 * word it was handed and paints it; it never works one out.
 */
export function gaugeColor(strain: Strain | null | undefined): string {
  return strain ? "var(--strain)" : "var(--muted-foreground)";
}

/**
 * A machine's readings as one stack, outermost first.
 *
 * `null` when the machine has never been reached — a stack of empty
 * tracks would say "we asked and it is all zero", which is a different
 * and wrong answer. The card says 还没问到 in words; the row says it by
 * having nothing to draw.
 *
 * Each track's own `fraction` may still be null (a reading khor could
 * not take), and that draws as track-only inside a stack that otherwise
 * has arcs — the shape of "this one, specifically, we do not know".
 */
export function vitalsTracks(row: DeviceRow): Track[] | null {
  const reading = row.vitals;
  if (!reading) return null;
  const { vitals: v } = reading;
  const tracks: Track[] = [
    { id: "cpu", fraction: v.cpu_pct / 100, color: gaugeColor(null) },
    {
      id: "mem",
      fraction: v.mem.total > 0 ? v.mem.used / v.mem.total : null,
      color: gaugeColor(reading.mem_strain),
    },
    {
      id: "disk",
      fraction: v.disk && v.disk.total > 0 ? v.disk.used / v.disk.total : null,
      color: gaugeColor(reading.disk_strain),
    },
  ];
  // Absent rather than empty, the same asymmetry the card draws: a
  // machine with no GPU is an ordinary machine, and a fourth grey track
  // on every desktop without a card would report a non-event.
  if (v.gpu) {
    tracks.push({ id: "gpu", fraction: v.gpu.util_pct / 100, color: gaugeColor(null) });
  }
  return tracks;
}

/**
 * The overview's readings: what a machine is **doing**.
 *
 * `vitalsTracks` minus the disk, because that is the one thing in the
 * stack that is not an activity — the star map's seat already stated the
 * rule in words ("a disk filling up is not doing"), and the network's
 * now printed the same three. Filtered from the full stack rather than
 * built again: one arithmetic, three magnifications.
 */
export function doingTracks(row: DeviceRow): Track[] | null {
  const all = vitalsTracks(row);
  return all ? all.filter((t) => t.id !== "disk") : null;
}
