// **The list rows' numbers that the face already holds** (round NET-8,
// for UX-15). Two of the four rows' facts need nothing new from the
// network; they are derived here, once, so every row that shows them
// says the same thing.
//
// - A machine's activity line: how many of its sessions are running,
//   counted from the session rows this face already has — the very rows
//   the 会话 list paints, so the two never disagree.
// - A quota row's window: which of an account's windows the row speaks
//   for, and when it resets.
//
// The other two facts are fields: `DeviceRow.rtt_ms` (the latency) and
// the workspace row's `changed` / `changed_capped` / `changed_ms`.
import type { DeviceRow, SessionRow } from "@/api";
import type { AccountReading } from "@/gen/bindings/AccountReading";

/** The state words that are a session running: 忙碌, and 待批 — waiting
    on a person is still work in hand, not idle. */
const RUNNING = new Set(["busy", "blocked"]);

/** How many sessions are running on `machine`. A row with no `source`
    is this machine's own; another machine's rows carry its name. */
export function runningOn(rows: readonly SessionRow[], machine: Pick<DeviceRow, "name" | "me">): number {
  let n = 0;
  for (const r of rows) {
    const on = r.source ? r.source.device === machine.name : machine.me;
    if (on && RUNNING.has(r.word)) n += 1;
  }
  return n;
}

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
