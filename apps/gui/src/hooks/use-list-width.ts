// The list pane's width, dragged at the seam between list and detail.
//
// Dragging below `--list-w-snap` collapses the pane whole — collapse is
// the far end of the same gesture, not a control of its own. The way
// back is either edge: the seam stays grabbable at width zero, and any
// rail landing reopens the pane, both to the width it had before it
// went away (the width survives the collapse; only `collapsed` flips).
//
// This is a viewer's own convenience, not a decision about the session
// — so it lives in localStorage, never in anything shared. Storage can
// be absent or throwing (private window, blocked site data), and every
// touch of it is wrapped: unreadable simply means "never dragged".
import { useCallback, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import type React from "react";

const STORE = "khor.list-width";

// Numbers live in tokens.css; these fallbacks only catch the token
// failing to resolve at all, and mirror the values written there.
function tokenPx(name: string, fallback: number): number {
  try {
    const v = Number.parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue(name),
    );
    return Number.isFinite(v) && v > 0 ? v : fallback;
  } catch {
    return fallback;
  }
}

type ListPref = { w: number; collapsed: boolean };

export function useListWidth(narrow: boolean) {
  const bounds = useRef<{ def: number; min: number; snap: number } | null>(null);
  if (bounds.current === null) {
    bounds.current = {
      def: tokenPx("--list-w", 288),
      min: tokenPx("--list-w-min", 180),
      snap: tokenPx("--list-w-snap", 100),
    };
  }
  const b = bounds.current;

  const [pref, setPref] = useState<ListPref>(() => {
    try {
      const raw = localStorage.getItem(STORE);
      if (raw) {
        const p: unknown = JSON.parse(raw);
        if (
          typeof p === "object" &&
          p !== null &&
          typeof (p as ListPref).w === "number" &&
          Number.isFinite((p as ListPref).w)
        ) {
          return {
            w: Math.max(b.min, (p as ListPref).w),
            collapsed: (p as ListPref).collapsed === true,
          };
        }
      }
    } catch {
      /* unreadable storage reads as "never dragged" */
    }
    return { w: b.def, collapsed: false };
  });

  const persist = (p: ListPref) => {
    try {
      localStorage.setItem(STORE, JSON.stringify(p));
    } catch {
      /* a viewer whose storage refuses writes still gets the session */
    }
  };

  // The drag paints through React state each move: a poll-driven
  // re-render mid-drag would otherwise repaint a stale width over the
  // live one. Persisting waits for the pointer to lift.
  // `origin` is the pref as the gesture began: one gesture is one
  // intent, and "the width it had before" means the width at the grab,
  // not whatever intermediate frame the pointer happened to cross last.
  const drag = useRef<{ left: number; origin: ListPref; live: ListPref } | null>(null);

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (narrow || e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    // The width is measured from the pane's own left edge, read off the
    // DOM at grab time rather than assumed from the rail token — the
    // handle's box won't do, it overhangs the seam by its hit margin.
    const pane = document.querySelector("[data-list]");
    const left = pane ? pane.getBoundingClientRect().left : e.currentTarget.getBoundingClientRect().left;
    drag.current = { left, origin: { ...pref }, live: { ...pref } };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    const raw = e.clientX - d.left;
    if (raw < b.snap) {
      // Keep the width of the grab: the comeback restores it.
      d.live = { w: d.origin.w, collapsed: true };
    } else if (raw < b.min) {
      // Between the snap line and the floor nothing fits, so nothing
      // lives here: a pane pulled out of collapse springs back to the
      // width it went away at, a pane being narrowed holds the floor.
      d.live = d.origin.collapsed
        ? { w: d.origin.w, collapsed: false }
        : { w: b.min, collapsed: false };
    } else {
      const max = Math.floor(window.innerWidth / 2);
      d.live = { w: Math.min(Math.max(raw, b.min), max), collapsed: false };
    }
    setPref(d.live);
  };

  const onPointerUp = () => {
    const d = drag.current;
    if (!d) return;
    drag.current = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    persist(d.live);
  };

  const onDoubleClick = () => {
    const next = { w: b.def, collapsed: false };
    setPref(next);
    persist(next);
  };

  /**
   * The same seam, from a keyboard.
   *
   * **Stops, not nudges, and that is a judgment about this pane rather
   * than a shortcut.** A splitter usually moves by some pixels per
   * press, and picking that number would mean inventing one — this repo
   * does not let a threshold be chosen by feel. This pane has exactly
   * three widths that mean anything, and every one of them is already a
   * token: away (`--list-w-snap` is where the drag lets go), the floor
   * (`--list-w-min`, below which nothing fits), and the default
   * (`--list-w`). The fourth is half the window, which the drag already
   * treats as the ceiling. So the keys walk that ladder.
   *
   * **What it does not give you**: an arbitrary width. That stays a
   * pointer's to choose, and a person who wants 350 will drag for it.
   * The keyboard's job here is that the pane can be put away and
   * brought back at all, which before this it could not be.
   */
  const step = (dir: -1 | 1) => {
    const max = Math.floor(window.innerWidth / 2);
    // Away, floor, default, ceiling — in order, and de-duplicated in
    // case a window is narrow enough that half of it is under the
    // floor (then the ladder simply has fewer rungs).
    const rungs: ListPref[] = [
      { w: pref.w, collapsed: true },
      { w: b.min, collapsed: false },
      { w: b.def, collapsed: false },
      { w: max, collapsed: false },
    ].filter((r, i, all) => i === 0 || r.collapsed || r.w > all[i - 1].w || all[i - 1].collapsed);
    const at = pref.collapsed
      ? 0
      : rungs.findIndex((r) => !r.collapsed && r.w >= pref.w - 1);
    const next = rungs[Math.min(rungs.length - 1, Math.max(0, (at < 0 ? rungs.length - 1 : at) + dir))];
    setPref(next);
    persist(next);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (narrow) return;
    if (e.key === "ArrowLeft") step(-1);
    else if (e.key === "ArrowRight") step(1);
    else if (e.key === "Home") {
      const next = { w: b.def, collapsed: false };
      setPref(next);
      persist(next);
    } else return;
    e.preventDefault();
  };

  /** A rail landing was chosen: whatever it shows deserves its list. */
  const expand = useCallback(() => {
    setPref((p) => {
      if (!p.collapsed) return p;
      const next = { ...p, collapsed: false };
      persist(next);
      return next;
    });
  }, []);

  /** The open landing's own glyph pressed again: put the list away, or
      bring it back — the activity-bar gesture (2026-09-02:「再次点击侧
      边栏icon,应该收起这个列表,再次点击打开」). The width survives and
      only `collapsed` flips, the same promise the drag's snap makes, so
      the comeback — from here, the seam, or another landing — is the
      width it had. */
  const toggle = useCallback(() => {
    setPref((p) => {
      const next = { ...p, collapsed: !p.collapsed };
      persist(next);
      return next;
    });
  }, []);

  return {
    width: pref.w,
    collapsed: pref.collapsed,
    expand,
    toggle,
    handleProps: { onPointerDown, onPointerMove, onPointerUp, onDoubleClick, onKeyDown },
    /** What the seam reports about itself, for anything that reads it
        aloud: where it sits between its two ends. */
    aria: {
      "aria-valuenow": pref.collapsed ? 0 : Math.round(pref.w),
      "aria-valuemin": 0,
      "aria-valuemax": Math.round(b.def),
    },
  };
}
