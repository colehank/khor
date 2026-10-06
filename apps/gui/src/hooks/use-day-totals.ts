// 今天's two numbers in the 附栏's 最近 (UX-37 §5): how long this
// session was 忙碌 and how long 空闲 in the last 24 hours.
//
// **Counted at the record's own resolution, or not at all** (`DayStrip`'s
// head: a coarse cell shows the word that most wanted you, so a total
// summed from the strip's cells would count twelve cells of a minute's
// 待批 as two hours). So the node is asked for one cell per minute — the
// record's resolution, which it merges nothing into — and the minutes of
// each word are counted here. Asked when the row is shown and once a
// minute while it is.
import { useEffect, useState } from "react";

import { fetchDayStrips } from "@/api";

/** One cell per minute: the record's own resolution (`khor_node::day`). */
const MINUTES = 1440;
const POLL_MS = 60_000;

export type DayTotals = { busy: number; idle: number };

/** The session's busy and idle time in ms, `null` until answered (or for
    a session the node has no day for). */
export function useDayTotals(session: string | null, on: boolean): DayTotals | null {
  const [totals, setTotals] = useState<DayTotals | null>(null);
  useEffect(() => {
    setTotals(null);
    if (!on || session === null) return;
    let live = true;
    const ask = () =>
      fetchDayStrips(MINUTES)
        .then((all) => {
          if (!live) return;
          const runs = all[session];
          if (!runs) return setTotals(null);
          let busy = 0;
          let idle = 0;
          for (const r of runs) {
            if (typeof r.mark !== "object") continue;
            if (r.mark.word === "busy") busy += r.cells;
            if (r.mark.word === "idle") idle += r.cells;
          }
          setTotals({ busy: busy * 60_000, idle: idle * 60_000 });
        })
        .catch(() => {});
    void ask();
    const t = setInterval(ask, POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [session, on]);
  return totals;
}
