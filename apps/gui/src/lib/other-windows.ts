// **A setting set in another window of this app** (fix-d, 2026-10-08).
//
// On the Mac 设置 is a window of its own (UX-10): the same page, on the
// same store, beside the main window. Every face setting kept in
// `localStorage` told its readers through an event on its own `window`
// (`khor:term-font`, `khor:nav-order` …), and an event on one window is
// never heard in another — so the settings window's row moved and the
// main window wore the old value until the next launch. fix-c mended
// 字号 alone (`lib/type-size`); the other rows had the same gap.
//
// The store already says so: a write to `localStorage` fires `storage` on
// every **other** page of the origin (the Mac's windows share one
// WKWebsiteDataStore; on the web face, another tab). One listener here,
// and a setting names its key and what to do — almost always forget the
// session's copy and tell its own readers again, as if the row were in
// this window.
//
// The store's settings (`khor config`, the lock) are not in
// `localStorage`; the window that changes one stamps a key here
// (`tellOthers`) and the others read the store again (`heardFrom`).
/** The stamps for the store's settings and for the lock. */
export const CONFIG_TOLD = "khor.told.config";
export const LOCK_TOLD = "khor.told.lock";

const heard = new Map<string, Set<() => void>>();
let armed = false;

function arm() {
  armed = true;
  window.addEventListener("storage", (e) => {
    try {
      if (e.storageArea && e.storageArea !== window.localStorage) return;
    } catch {
      return;
    }
    // `null` is the whole store cleared: every setting is back at its
    // default.
    const runs = e.key === null ? [...heard.values()].flatMap((s) => [...s]) : [...(heard.get(e.key) ?? [])];
    for (const run of runs) run();
  });
}

/** Calls `run` whenever another window of this origin writes `key`;
    answers the way to stop. */
export function heardFrom(key: string, run: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  if (!armed) arm();
  const runs = heard.get(key) ?? new Set();
  runs.add(run);
  heard.set(key, runs);
  return () => runs.delete(run);
}

/** Tells every other window that what `key` stands for changed: a fresh
    stamp, so the write is a change even when nothing else differs. */
export function tellOthers(key: string) {
  try {
    window.localStorage.setItem(key, `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`);
  } catch {
    // A face without storage has no other window to tell.
  }
}
