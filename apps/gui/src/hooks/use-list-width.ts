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

/** Where one seam's width is remembered. **Parameterised since
    2026-09-12** (工作区批 收尾): the workspace has two seams of its own
    (the file tree and the sessions column) and they must not share the
    list pane's number — one person's narrow tree is not their narrow
    session list.

    The names are written here rather than passed as free strings, so
    the set of seams this app has is one list somebody can read.

    ⚠️ **`ws-code` was one of them for a day** (2026-09-16 → 2026-09-17):
    the editor was a column of the workspace, and then a window of its
    own (docs/WORKSPACE.md 第六轮 46). A seam with no column is a width
    remembered about nothing. */
export type Seam = "list" | "ws-tree" | "ws-sessions";
const STORE = (seam: Seam) => `khor.${seam}-width`;

/**
 * **The shell's one list pane, wherever it happens to be mounted.**
 *
 * `components/ListPane` draws every secondary sidebar in this app
 * (docs/SHELL.md 第 6 条) and each landing gives it a name of its own,
 * because a gate and a smoke reach for *the workspace's tree* rather
 * than for *a pane*. The seam's width, though, is measured against
 * whichever of them is on screen — and only one ever is at a time,
 * since they belong to different landings.
 *
 * ⚠️ **This was `[data-list]` alone until 批二, and that was a bug I
 * shipped in 批一.** The 额度 landing got the shell's seam and kept
 * drawing `[data-quota-pane]`, so `querySelector` found nothing and the
 * drag fell back to measuring the *seam element* — whose box overhangs
 * the border by its hit margin (`-mx-1`), so grabbing it jumped the
 * pane by that overhang before it started tracking. The browser's
 * column would have landed the same way. One selector naming all of
 * them is what makes 「the pane the seam is beside」 a thing this hook
 * can actually find.
 */
export const LIST_PANE = "[data-list],[data-quota-pane],[data-browse-tabs]";

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

export function useListWidth(
  narrow: boolean,
  seam: Seam = "list",
  /** Which side of the pane the seam is on. **The sessions column is
      dragged from its left edge and grows leftwards**, so the width is
      the distance from the pointer to the pane's *right* edge — the
      same gesture, mirrored, rather than a second set of rules. */
  side: "left" | "right" = "left",
  /** Which element the width is measured against. The shell's list
      pane answers to [`LIST_PANE`] whichever landing drew it; the
      workspace's two seams name their own. */
  paneSel: string = LIST_PANE,
  /** How wide this pane may be dragged, asked at the moment of the drag
      (2026-09-17 ruling 52:「session无法再往右拖了, 似乎被固定了最大宽
      度, 不应该这样」— the user).
   *
   * **The default ceiling is half the window, and it is the two-column
   * answer.** A list beside a detail may fairly take half the room; a
   * workspace is three columns, so half the window is reached while the
   * sessions column is still barely past the middle — and the drag
   * simply stopped, with nothing on the glass saying why.
   *
   * What replaces it is not a bigger ratio but a different question:
   * how much room is there actually left. [`roomBeside`] answers it by
   * measuring, which is why this is a function and not a number — the
   * other columns move. */
  max: () => number = () => Math.floor(window.innerWidth / 2),
) {
  const bounds = useRef<{ def: number; min: number; snap: number; cap: number } | null>(null);
  if (bounds.current === null) {
    // **The wide list's bounds are what is drawn** (the audit fix plan
    // F2.8; W19): its column cannot be drawn under `--wide-column-min`
    // (the top row's room), so that is its floor here too — a stored width
    // under the drawn one was a dead zone the pointer had to cross back
    // through. A drag past the floor folds it (F6.7), and it is capped at
    // `--list-w-max`, where rows stop being rows.
    const list = seam === "list";
    const min = list ? tokenPx("--wide-column-min", 340) : tokenPx("--list-w-min", 180);
    bounds.current = {
      def: tokenPx("--list-w", 288),
      min,
      snap: list ? min - tokenPx("--list-w-snap", 100) / 2 : tokenPx("--list-w-snap", 100),
      cap: list ? tokenPx("--list-w-max", 420) : Infinity,
    };
  }
  const b = bounds.current;

  const [pref, setPref] = useState<ListPref>(() => {
    try {
      const raw = localStorage.getItem(STORE(seam));
      if (raw) {
        const p: unknown = JSON.parse(raw);
        if (
          typeof p === "object" &&
          p !== null &&
          typeof (p as ListPref).w === "number" &&
          Number.isFinite((p as ListPref).w)
        ) {
          return {
            w: Math.min(Math.max(b.min, (p as ListPref).w), b.cap),
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
      localStorage.setItem(STORE(seam), JSON.stringify(p));
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
    const pane = document.querySelector(paneSel);
    const box = (pane ?? e.currentTarget).getBoundingClientRect();
    drag.current = {
      left: side === "left" ? box.left : box.right,
      origin: { ...pref },
      live: { ...pref },
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    // Mirrored for a right-hand seam: the pane grows as the pointer
    // moves *towards* the pane's far edge either way.
    const raw = side === "left" ? e.clientX - d.left : d.left - e.clientX;
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
      d.live = { w: Math.min(Math.max(raw, b.min), Math.max(Math.min(max(), b.cap), b.min)), collapsed: false };
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
   * (`--list-w`). The fourth is the ceiling the drag stops at, which
   * since 2026-09-17 is `max()` rather than half the window. So the
   * keys walk that ladder.
   *
   * **What it does not give you**: an arbitrary width. That stays a
   * pointer's to choose, and a person who wants 350 will drag for it.
   * The keyboard's job here is that the pane can be put away and
   * brought back at all, which before this it could not be.
   */
  const step = (dir: -1 | 1) => {
    const ceiling = Math.max(Math.min(max(), b.cap), b.min);
    // Away, floor, default, ceiling — in order, and de-duplicated in
    // case a window is narrow enough that half of it is under the
    // floor (then the ladder simply has fewer rungs).
    const rungs: ListPref[] = [
      { w: pref.w, collapsed: true },
      { w: b.min, collapsed: false },
      { w: b.def, collapsed: false },
      { w: ceiling, collapsed: false },
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
    // Wider is always「away from the pane's own edge」, so the two keys
    // swap on a right-hand seam — an arrow that shrank the thing it
    // points at would be the one gesture nobody forgives.
    if (e.key === "ArrowLeft") step(side === "left" ? -1 : 1);
    else if (e.key === "ArrowRight") step(side === "left" ? 1 : -1);
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

  /** Open, or put away — said as **a state rather than a flip**.
   *
   * ⚠️ **An effect that declares a default must be idempotent, and a
   * flip is not.** React runs an effect twice on mount in development
   * on purpose, and both runs read the same render's `collapsed`, so a
   * caller spelling 「put it away」 as 「toggle if it is open」 toggles
   * twice and lands exactly where it started. That is what the
   * workspace's 「the sessions column starts folded when the list has a
   * column of its own」 default had been doing since it was written:
   * the column came up open regardless, which on a phone is three
   * columns in 390 pixels and a file view 39 wide. Nothing said so
   * until the file began to wrap and `check-face`'s touch floor
   * measured the editor (工作区批⑨). */
  const show = useCallback((on: boolean) => {
    setPref((p) => {
      if (p.collapsed === !on) return p;
      const next = { ...p, collapsed: !on };
      persist(next);
      return next;
    });
  }, []);

  return {
    width: pref.w,
    collapsed: pref.collapsed,
    expand,
    toggle,
    show,
    handleProps: { onPointerDown, onPointerMove, onPointerUp, onDoubleClick, onKeyDown },
    /** What the seam reports about itself, for anything that reads it
        aloud: where it sits between its two ends. */
    aria: {
      "aria-valuenow": pref.collapsed ? 0 : Math.round(pref.w),
      // 0 is the folded pane; the top is the ceiling the drag stops at.
      "aria-valuemin": 0,
      "aria-valuemax": Math.round(Math.max(Math.min(max(), b.cap), b.min)),
    },
  };
}

/**
 * What one seam reports about the pane it sizes.
 *
 * Named so the shell can take it as a prop (`components/ListPane`):
 * every secondary sidebar in this app is drawn by that one component,
 * and it has to be able to say「this pane's width, whichever seam owns
 * it」 without naming the hook's return shape three times.
 */
export type PaneWidth = ReturnType<typeof useListWidth>;

/**
 * A seam's ceiling on the workspace page: **everything the middle
 * column can spare above its floor** (2026-09-17 ruling 52).
 *
 * 「上限 = 窗口宽 − 另外两栏的宽 − 中栏的地板」. Written as a
 * measurement of two boxes rather than as that subtraction, because the
 * two are the same number and only one of them stays true when a column
 * is added: what this pane may grow to is what it is now plus whatever
 * the file column has over `--ws-mid-min`. Every other column on the
 * row, the rail included, is already accounted for by the middle one's
 * width being what it is.
 *
 * **It is a fixed point under the drag.** Widening this pane narrows
 * the middle one by the same pixels, so the sum holds still, and a drag
 * that reaches the ceiling stops there rather than oscillating.
 *
 * Away from a workspace there is no middle column and the old ceiling
 * stands: half the window, which is the list-and-detail answer and
 * right for the panes that have two columns.
 */
export function roomBeside(paneSel: string): () => number {
  return () => {
    const half = Math.floor(window.innerWidth / 2);
    const mid = document.querySelector("[data-ws-mid]");
    if (mid === null) return half;
    const me = document.querySelector(paneSel);
    const mine = me === null ? 0 : me.getBoundingClientRect().width;
    return Math.round(mine + mid.getBoundingClientRect().width - tokenPx("--ws-mid-min", 240));
  };
}
