// **A list row's number that the face derives** (round NET-8, for
// UX-15): a quota row's window — which of an account's windows the row
// speaks for, and when it resets. The other rows' facts are fields.
// ⚠️ 墓碑 (UX-48): `runningOn`, a machine row's count of running
// sessions; the owner's machine row reads latency, CPU, memory and GPU.
import type { AccountReading } from "@/gen/bindings/AccountReading";

/** The window a quota row speaks for: **the fullest whole-plan window**
    (a subscription's 5-hour or 7-day, whichever is nearer its limit —
    the one that will stop the work first), else the account's own
    figure (an API account's budget). `left` is what remains, 0–1;
    `resets_at` unix seconds, `null` when the provider does not say.
    `null` when there is no number at all. */
export function rowWindow(
  r: Pick<AccountReading, "fill"> & { windows?: AccountReading["windows"] | null; resets_at?: number | null },
): { left: number; resets_at: number | null } | null {
  let best: { used: number; resets_at: number | null } | null = null;
  for (const w of r.windows ?? []) {
    if (w.scope !== null) continue;
    const used = w.used_pct / 100;
    if (!best || used > best.used) best = { used, resets_at: w.resets_at };
  }
  if (best) return { left: Math.max(0, 1 - best.used), resets_at: best.resets_at };
  if (r.fill !== null && r.fill !== undefined) return { left: Math.max(0, 1 - r.fill), resets_at: r.resets_at ?? null };
  return null;
}
