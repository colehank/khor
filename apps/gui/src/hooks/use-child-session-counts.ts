import { useEffect, useState } from "react";
import { khorAgent, type SessionRow } from "@/api";

type ChildSessionCount = { count: number; atLeast: boolean };
export const childSessionKey = (row: SessionRow) => JSON.stringify([row.id, row.execution_home]);

/** Read only retained parent links; full child status belongs to the expanded tree. */
export function useChildSessionCounts(rows: readonly SessionRow[]) {
  const owners = JSON.stringify(rows.filter(row => row.category === "khor" && row.execution_home)
    .map(row => [row.id, row.execution_home]));
  const [counts, setCounts] = useState<ReadonlyMap<string, ChildSessionCount>>(() => new Map());
  useEffect(() => {
    const parents = JSON.parse(owners) as [string, string][];
    const keys = new Set(parents.map(parent => JSON.stringify(parent)));
    setCounts(previous => new Map([...previous].filter(([key]) => keys.has(key))));
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const read = async () => {
      if (!document.hidden) {
        let next = 0;
        const found = new Map<string, ChildSessionCount>();
        await Promise.all(Array.from({length: Math.min(4, parents.length)}, async () => {
          while (!stopped && next < parents.length) {
            const parent = parents[next++];
            try {
              const { data } = await khorAgent("manage", parent[1], {operation: "children", session: parent[0], summary_only: true});
              const value = data as {count?: number; at_least?: boolean; children?: unknown[]} | null;
              if (typeof value?.count === "number" && Number.isSafeInteger(value.count) && value.count >= 0) {
                found.set(JSON.stringify(parent), {count: value.count, atLeast: value.at_least === true});
              } else if (Array.isArray(value?.children)) {
                // Older hosts return a bounded detail page, not an exact total.
                found.set(JSON.stringify(parent), {count: value.children.length, atLeast: value.children.length > 0});
              }
            } catch { /* An unavailable count must not prevent opening the session. */ }
          }
        }));
        if (!stopped) setCounts(found);
      }
      if (!stopped && parents.length) timer = setTimeout(read, 15_000);
    };
    void read();
    return () => { stopped = true; clearTimeout(timer); };
  }, [owners]);
  return counts;
}
