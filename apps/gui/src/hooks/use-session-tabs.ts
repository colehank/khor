// The sessions a person has open, as tabs — the iTerm arrangement the
// user asked for on 2026-09-02:「打开的session做成标签页,类似iterm的那种」.
//
// A sibling of `browse/use-browse-tabs` with one difference that is the
// whole file: a session tab *is* its session. There is nothing to type
// into a tab, so no blank tab is ever minted, opening a row that is
// already open focuses it rather than adding a second, and the last
// tab closing leaves nothing open (the empty state stands) instead of
// a fresh one. The id is the identity, so the state is a list of ids
// and which of them is in front.
//
// Kept per screen in `localStorage`, like the browsing tabs: what a
// screen has open belongs to that screen (docs/UX.md 标签页). Restore
// is validated field by field, and the first render never writes —
// `restore` may have just answered "I could not read", and writing that
// back would turn a one-off storage failure into an erasure.
//
// **Closing a tab is not closing the session.** The tab is a view the
// person opened; the session lives on, on its row, exactly as it was.
// Ending a session is the header's 关闭, which goes through the node —
// and when the row then leaves the list, the shell closes the tab
// (`App`'s gone-row rule), not the other way round.
import { useCallback, useEffect, useRef, useState } from "react";

import { stalestTab } from "@/browse/tab-limit";

const STORE_KEY = "khor.sessions.tabs";

type Tabs = { tabs: string[]; active: string | null };

const EMPTY: Tabs = { tabs: [], active: null };

/**
 * **A new tab is a tab before it is a session** (UX-45, the owner
 * 2026-10-07: 「+ 应该不给终端，直接新建一个标签页，这个标签页上进行选择」):
 * ＋ puts one in front at once, its page the new-session choices; made,
 * it becomes that session in its own place (`become`). Its id is this
 * prefix and a nonce, never a row's, so the gone-row rule never takes it
 * (it was never listed) and nothing else mistakes it for a session.
 * **Not kept across launches**: closed before it was made, it leaves
 * nothing behind, a reload included.
 */
const NEW_TAB = "new-tab:";
export const isNewTab = (id: string | null | undefined): boolean => !!id && id.startsWith(NEW_TAB);
export const newTabId = (): string => `${NEW_TAB}${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;

function restore(): Tabs {
  try {
    const raw = window.localStorage.getItem(STORE_KEY);
    if (!raw) return EMPTY;
    const v: unknown = JSON.parse(raw);
    if (typeof v !== "object" || v === null) return EMPTY;
    const o = v as { tabs?: unknown; active?: unknown };
    const tabs = Array.isArray(o.tabs)
      ? o.tabs.filter((t): t is string => typeof t === "string" && t.length > 0 && !isNewTab(t))
      : [];
    const active = typeof o.active === "string" && tabs.includes(o.active) ? o.active : null;
    return { tabs, active };
  } catch {
    return EMPTY;
  }
}

export type SessionTabs = {
  tabs: string[];
  active: string | null;
  /** Open a session in a tab: appends it if it is not open, and brings
      it to the front either way. */
  open: (id: string) => void;
  /** Bring an open tab to the front; a no-op for an id that is not open. */
  focus: (id: string) => void;
  /** How many times `open` or `focus` has been asked, the tab in front
      included (fix-a): the strip brings the front tab into view on each,
      so a pick that changes nothing still shows the tab it named
      (`TabStrip` `seek`). */
  picks: number;
  /** Close one tab (the neighbour on the right takes the front, falling
      back to the left at the end — the browser convention). */
  close: (id: string) => void;
  /** Close every tab among `ids` — the shell's gone-row rule, in one
      state change so the front is decided once. */
  drop: (ids: string[]) => void;
  /** A new tab made into the session it chose (UX-45): `id` takes its
      place in the strip and the front; already open elsewhere, the new
      tab simply goes and `id` comes forward. */
  become: (draft: string, id: string) => void;
};

/**
 * `limit` and `keep` are 标签页上限 (docs/SHELL.md 第三轮 第 4 条): past
 * the limit, opening one more closes the tab touched longest ago that
 * `keep` does not hold — a pinned session. Closing a tab is closing the
 * tab, never the session, the way every close on this strip already is.
 */
export function useSessionTabs(
  limit: number | null = null,
  keep: (id: string) => boolean = () => false,
): SessionTabs {
  const [state, setState] = useState<Tabs>(restore);
  const wrote = useRef(false);
  /** When each tab was last put in front. A ref, read inside the state
      updaters below; and the two inputs ride refs too, so `open` keeps
      one identity (`App` lists it as an effect dependency). */
  const touched = useRef(new Map<string, number>());
  const rules = useRef({ limit, keep });
  rules.current = { limit, keep };
  const [picks, setPicks] = useState(0);

  useEffect(() => {
    if (!wrote.current) {
      wrote.current = true;
      return;
    }
    try {
      window.localStorage.setItem(STORE_KEY, JSON.stringify(state));
    } catch {
      /* the tabs still work; only the memory of them is lost */
    }
  }, [state]);

  const open = useCallback((id: string) => {
    touched.current.set(id, Date.now());
    setPicks((n) => n + 1);
    setState((was) => {
      if (was.tabs.includes(id)) return { ...was, active: id };
      const now = [...was.tabs, id];
      const { limit, keep } = rules.current;
      // Nor the tab that was in front (UX-45): a new tab's 取消 goes back
      // to it, and a tab restored at launch has no time to be judged by.
      const gone = stalestTab(now, limit, touched.current, (t) => t === id || t === was.active || keep(t));
      return { tabs: gone === null ? now : now.filter((t) => t !== gone), active: id };
    });
  }, []);

  const focus = useCallback((id: string) => {
    touched.current.set(id, Date.now());
    setPicks((n) => n + 1);
    setState((was) => (was.tabs.includes(id) ? { ...was, active: id } : was));
  }, []);

  const closeMany = (was: Tabs, gone: Set<string>): Tabs => {
    const left = was.tabs.filter((t) => !gone.has(t));
    if (left.length === was.tabs.length) return was;
    if (was.active === null || !gone.has(was.active)) return { tabs: left, active: was.active };
    if (left.length === 0) return { tabs: [], active: null };
    // The right-hand neighbour takes the front: in the new order it is
    // the tab at the index of however many survivors sat to the left of
    // the old front, clamped to the end of the strip.
    const at = was.tabs.indexOf(was.active);
    const before = was.tabs.slice(0, at).filter((t) => !gone.has(t)).length;
    return { tabs: left, active: left[Math.min(before, left.length - 1)] };
  };

  const close = useCallback((id: string) => {
    setState((was) => closeMany(was, new Set([id])));
  }, []);

  const drop = useCallback((ids: string[]) => {
    setState((was) => closeMany(was, new Set(ids)));
  }, []);

  const become = useCallback((draft: string, id: string) => {
    touched.current.set(id, Date.now());
    setState((was) => {
      if (was.tabs.includes(id)) return { tabs: was.tabs.filter((t) => t !== draft), active: id };
      const at = was.tabs.indexOf(draft);
      if (at < 0) return { tabs: [...was.tabs, id], active: id };
      return { tabs: was.tabs.map((t) => (t === draft ? id : t)), active: id };
    });
  }, []);

  return { tabs: state.tabs, active: state.active, open, focus, picks, close, drop, become };
}
