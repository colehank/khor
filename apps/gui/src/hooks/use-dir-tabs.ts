// What this screen currently has open on the files landing.
//
// **Kept on this device and not on the network, deliberately** — the
// same split the arrangement and the collapse state already follow
// (`App.tsx` 储存的 arrange), and `khor_sync::recents` states the other
// half of it: *having been* somewhere is a fact about a person and
// travels, *having it open right now* is a fact about one screen and
// does not. A desk with four directories open says nothing about what
// the phone in a queue should be showing.
//
// Every read and every write is inside a try/catch, and a store that
// refuses (private mode, a full quota, a browser with site data off) is
// not an error state: the tabs still work, they just will not outlive
// the reload. What a read cannot do is **turn "I could not read" into
// "there is nothing"** — either way the answer is one fresh tab,
// because a landing with no tab at all has nothing to draw.
import { useCallback, useEffect, useRef, useState } from "react";

/** One tab: a machine and a place on it. */
export type DirTab = {
  /** This screen's own handle for the tab. Never shown, never sent —
      two tabs on one machine and one path are two tabs, so the identity
      cannot be derived from what the tab points at. */
  id: string;
  /** The machine's device id, or `null` on a tab that has not been
      pointed anywhere yet — the fresh-tab state, which is a real state
      and not a missing one. */
  device: string | null;
  /** Where to open: absolute, or `""` for that machine's home. The
      answer carries the real path back (`DirListing.path`) and this is
      rewritten to it, so a restored tab needs no second resolution. */
  path: string;
};

const KEY = "khor.files.tabs";

/**
 * A fresh handle.
 *
 * Time plus a counter rather than `crypto.randomUUID`: the ids are
 * written to storage and read back, so they have to survive a reload,
 * and two tabs minted in the same millisecond have to differ. Neither
 * needs a random source.
 */
let minted = 0;
function mintId(): string {
  minted += 1;
  return `t${Date.now().toString(36)}-${minted}`;
}

export function blankTab(): DirTab {
  return { id: mintId(), device: null, path: "" };
}

/** The stored shape, checked field by field rather than trusted. A
    value written by another version of this app — or by a person with a
    console open — must not be able to put the landing into a state that
    draws nothing. */
function restore(): { tabs: DirTab[]; active: string } {
  const fresh = () => {
    const one = blankTab();
    return { tabs: [one], active: one.id };
  };
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return fresh();
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return fresh();
    const { tabs, active } = parsed as { tabs?: unknown; active?: unknown };
    if (!Array.isArray(tabs)) return fresh();
    const kept: DirTab[] = [];
    for (const t of tabs) {
      if (typeof t !== "object" || t === null) continue;
      const { id, device, path } = t as Record<string, unknown>;
      if (typeof id !== "string" || typeof path !== "string") continue;
      if (device !== null && typeof device !== "string") continue;
      kept.push({ id, device, path });
    }
    if (kept.length === 0) return fresh();
    // An `active` naming a tab that is not there is the same failure as
    // no `active` at all, and both land on the first tab rather than on
    // nothing.
    const found = typeof active === "string" && kept.some((t) => t.id === active);
    return { tabs: kept, active: found ? (active as string) : kept[0].id };
  } catch {
    return fresh();
  }
}

export type DirTabs = {
  tabs: DirTab[];
  /** The open one. Never undefined: closing the last tab mints a fresh
      one rather than leaving the landing with nothing to draw. */
  current: DirTab;
  focus: (id: string) => void;
  /** A fresh tab, focused — the "+" and the switcher's own button. */
  add: () => void;
  /** Points the **open** tab somewhere. What a pin, a recent and the
      address bar all do: you are already looking at the tab, so a new
      one would be a tab you did not ask for (Chrome's own behaviour on
      a new-tab page's shortcuts). */
  goTo: (device: string, path: string) => void;
  /** Opens a **new** tab, focused. What arriving from somewhere else
      does — the machine card's 看它的文件 — because there the tab you
      are looking at is not part of the request. */
  open: (device: string, path: string) => void;
  /** Points the open tab at nothing: taking the machine off the address
      bar. **Not `close` and not `goTo("")`** — the tab is still the tab
      the person is looking at, it has simply stopped naming a place,
      and that is the fresh-tab state rather than a missing one. */
  clear: () => void;
  close: (id: string) => void;
};

export function useDirTabs(): DirTabs {
  const [state, setState] = useState(restore);
  // The first render must not write: `restore` may have just answered
  // "I could not read", and writing that answer back would turn a
  // one-off storage failure into a real erasure of the tabs.
  const wrote = useRef(false);

  useEffect(() => {
    if (!wrote.current) {
      wrote.current = true;
      return;
    }
    try {
      window.localStorage.setItem(KEY, JSON.stringify(state));
    } catch {
      /* the tabs still work; only the memory of them is lost */
    }
  }, [state]);

  const focus = useCallback((id: string) => {
    setState((was) => (was.tabs.some((t) => t.id === id) ? { ...was, active: id } : was));
  }, []);

  const add = useCallback(() => {
    setState((was) => {
      const one = blankTab();
      return { tabs: [...was.tabs, one], active: one.id };
    });
  }, []);

  const goTo = useCallback((device: string, path: string) => {
    setState((was) => ({
      ...was,
      tabs: was.tabs.map((t) => (t.id === was.active ? { ...t, device, path } : t)),
    }));
  }, []);

  const open = useCallback((device: string, path: string) => {
    setState((was) => {
      const one: DirTab = { id: mintId(), device, path };
      return { tabs: [...was.tabs, one], active: one.id };
    });
  }, []);

  const clear = useCallback(() => {
    setState((was) => ({
      ...was,
      tabs: was.tabs.map((t) => (t.id === was.active ? { ...t, device: null, path: "" } : t)),
    }));
  }, []);

  const close = useCallback((id: string) => {
    setState((was) => {
      const at = was.tabs.findIndex((t) => t.id === id);
      if (at < 0) return was;
      const left = was.tabs.filter((t) => t.id !== id);
      if (left.length === 0) {
        // Chrome closes the window here. This landing is not a window —
        // the rail is still on screen and the region has to hold
        // something — so the last close is a reset to a fresh tab.
        const one = blankTab();
        return { tabs: [one], active: one.id };
      }
      if (was.active !== id) return { ...was, active: was.active, tabs: left };
      // The neighbour on the right takes the place, falling back to the
      // left at the end of the strip — the browser convention, and the
      // one that keeps the eye where the hand already is.
      const next = left[Math.min(at, left.length - 1)];
      return { tabs: left, active: next.id };
    });
  }, []);

  const current = state.tabs.find((t) => t.id === state.active) ?? state.tabs[0];
  return { tabs: state.tabs, current, focus, add, goTo, open, clear, close };
}
