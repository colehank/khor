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

import { fetchConfig, hear, type ConfigRow } from "@/api";
import { CONFIG_TOLD, heardFrom } from "@/lib/other-windows";

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

  // **Coming back to the window re-reads the store** (2026-09-27). The
  // store has other writers — `khor config set`, the web face, another
  // window — and a setting changed there used to reach this face only
  // after a restart. Focus is when somebody looks again, so it is when
  // what is on screen should catch up. The native gate
  // (`scripts/check-native.mjs`) switches theme this way, without
  // pressing anything in the window.
  //
  // Two roads to one fact: the page's `focus` (a browser face), and the
  // app's `window:focused` — WKWebView does not hand the page a `focus`
  // when the app comes back to the front, so the window says it.
  //
  // **And at once when another window changed one** (fix-d, 2026-10-08):
  // the Mac's 设置 is a window beside this one, and the main window is not
  // focused while somebody picks 深色 there — it stayed light until
  // clicked. `setConfig` says so to every other window.
  useEffect(() => {
    window.addEventListener("focus", reread);
    const drop = hear<null>("window:focused", reread);
    const told = heardFrom(CONFIG_TOLD, reread);
    return () => {
      window.removeEventListener("focus", reread);
      drop();
      told();
    };
  }, [reread]);

  return { rows, reread };
}
