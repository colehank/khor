// **A long command that finished while nobody was looking** (roadmap 2.4).
//
// The host knows when a command ends (`khor_node::commands`) and the row
// carries it (`SessionRow.doing`); this watches the rows the app already
// polls and picks out the commands that were seen running, ran at least
// the chosen time, and ended while their session was not on screen (the
// app hidden, another page, another tab). iTerm2 ("Alert on next mark")
// and kitty (`notify_on_cmd_finish`, with its `invisible` default and a
// minimum duration) do the same from the same marks.
//
// On a Mac the system says it (`notify_finished`, the notification
// plugin) and the page keeps a notice that opens the session. A phone's
// page is asleep while the app is in the background, so v1 shows what it
// missed the moment it comes back; a notification pushed to a sleeping
// phone needs the host's word to travel to Apple's push service, a later
// net item.
//
// The threshold is this device's, in its `localStorage`.
import { useEffect, useState } from "react";

import type { SessionRow } from "@/api";
import { heardFrom } from "@/lib/other-windows";

export const FINISHED_CHOICES = [0, 10, 30, 60, 300] as const;

const KEY = "khor.finished-after";
const CHANGED = "khor:finished-after";
let session: number | null = null;

/** Seconds a command must run to be announced; 0 is never. */
export function finishedAfter(): number {
  if (session !== null) return session;
  try {
    const n = Number(window.localStorage.getItem(KEY) ?? "30");
    return (FINISHED_CHOICES as readonly number[]).includes(n) ? n : 30;
  } catch {
    return 30;
  }
}

export function setFinishedAfter(secs: number) {
  session = secs;
  try {
    window.localStorage.setItem(KEY, String(secs));
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: secs }));
}

// Set in another window of this app (the Mac's settings window,
// `lib/other-windows`): heard here as if set here.
heardFrom(KEY, () => {
  session = null;
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: finishedAfter() }));
});

export function useFinishedAfter(): number {
  const [n, setN] = useState(finishedAfter);
  useEffect(() => {
    const changed = (e: Event) => setN((e as CustomEvent<number>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return n;
}

export type Finished = {
  id: string;
  title: string;
  machine: string;
  command: string;
  exit: number | null;
  ms: number;
  /** A program's own notification (`OSC 9` / `OSC 777`, roadmap 4.1)
      rather than a finished command: said as it is. */
  notice?: string;
};

/** Watches successive row lists; see the module head. */
export class FinishedWatch {
  /** Sessions seen running, by when their command started. */
  private running = new Map<string, number>();
  /** The last program notification seen per session, by when. */
  private noticed = new Map<string, number>();

  see(rows: SessionRow[], onScreen: (id: string) => boolean): Finished[] {
    const after = finishedAfter() * 1000;
    const out: Finished[] = [];
    for (const r of rows) {
      const d = r.doing;
      // A program's own notification, newer than the last one seen here
      // (the first sight of a session only records where it stands).
      const at = d?.notice_ms ?? 0;
      const before = this.noticed.get(r.id);
      this.noticed.set(r.id, at);
      if (d?.notice && before !== undefined && at > before && !onScreen(r.id)) {
        out.push({ id: r.id, title: r.title, machine: r.home, command: "", exit: null, ms: 0, notice: d.notice });
      }
      if (!d || !d.command || d.started_ms === 0) continue;
      if (d.running) {
        this.running.set(r.id, d.started_ms);
        continue;
      }
      const was = this.running.get(r.id);
      if (was === undefined) continue;
      this.running.delete(r.id);
      if (was !== d.started_ms || d.ended_ms === null) continue;
      const ms = d.ended_ms - d.started_ms;
      if (after === 0 || ms < after || onScreen(r.id)) continue;
      out.push({ id: r.id, title: r.title, machine: r.home, command: d.command, exit: d.exit, ms });
    }
    return out;
  }
}
