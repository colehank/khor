// 设置 (docs/MORE.md), as one piece of state the shell holds and hands
// down (`src/settings.ts` carries it the rest of the way).
//
// **One reader, beside the lock's.** Both answers are a file read off
// the store with no node opened, so both land before anything that uses
// them is drawn — and both are re-read rather than patched when a page
// changes one, so what is on screen is what the store says instead of
// what a button just sent.
//
// **A failure is `null`, never an empty list.** On a store this face
// cannot read, 「no settings」 would make every switch vanish and every
// reader fall back to the default; `null` says 「khor has not answered」,
// which is the state the readers already treat as the default without
// keeping a copy of it.
import { useCallback, useEffect, useState } from "react";

import { fetchConfig, type ConfigRow } from "@/api";

/** ⚠️ **Not `useSettings`** — that one is `src/settings.ts`, which reads
    the answer out of the context this hook fills. Two functions with one
    name in two modules is the trap where somebody imports the reader in
    the shell and starts a second fetch per screen. */
export type Settings = {
  rows: ConfigRow[] | null;
  /** Re-read the store: 设置 and 安全 both change these. */
  reread: () => void;
};

export function useStoredSettings(): Settings {
  const [rows, setRows] = useState<ConfigRow[] | null>(null);
  const [asked, setAsked] = useState(0);

  useEffect(() => {
    let alive = true;
    fetchConfig().then(
      (r) => {
        if (alive) setRows(r);
      },
      () => {
        // The shell draws the ordinary backend refusal on its own beat;
        // here the answer stays `null` and every reader goes on doing
        // what khor did before these keys existed.
        if (alive) setRows(null);
      },
    );
    return () => {
      alive = false;
    };
  }, [asked]);

  const reread = useCallback(() => setAsked((n) => n + 1), []);

  return { rows, reread };
}
