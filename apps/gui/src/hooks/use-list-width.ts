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

  /** A rail landing was chosen: whatever it shows deserves its list. */
  const expand = useCallback(() => {
    setPref((p) => {
      if (!p.collapsed) return p;
      const next = { ...p, collapsed: false };
      persist(next);
      return next;
    });
  }, []);

  return {
    width: pref.w,
    collapsed: pref.collapsed,
    expand,
    handleProps: { onPointerDown, onPointerMove, onPointerUp, onDoubleClick },
  };
}
