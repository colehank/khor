// **A row slid left shows what can be done to it** (the audit fix plan
// F7.5; narrow P-C; UX-44 §3): iOS's trailing swipe actions, as Mail and
// Messages draw them. One component for every list that takes it: the
// list puts `useSwipeRows` on its box once, and draws each row through
// `SwipeRow` with the actions it offers. Narrow only — under a mouse the
// row's menu is the way, and its right-click offers the same acts.
//
// - **The direction is decided by the first movement**: a touch that
//   goes down first is the list's scroll (or its pull to refresh, or the
//   sheet's pull) and closes an open row; one that starts within `EDGE`
//   of the screen's left side is the back swipe's (`use-edge-swipe`).
// - Released past half the actions' width, the row stays open; a press
//   on an action does it. One row open at a time; a touch anywhere else
//   closes it.
// - **A full swipe** (UX-44 §3, the owner 2026-10-07) does the row's
//   destructive act at once — only on a row that says so (`full`): 最近
//   forgetting a record, 浏览器 closing a page. ⚠️ Elsewhere **a full
//   swipe only reveals** (F7.5): ending a session because a thumb went a
//   little too far is the accident that rule refuses, and it stands.
//
// Moved by variables on the row (`--row-reveal`, `app.css`
// `[data-row-swipe]`), never by React state.
import { useEffect, type ComponentProps, type RefObject } from "react";

import type { RowAction } from "@/components/RowMenu";
import { feel } from "@/lib/haptics";

const EDGE = 20;
const SLOP = 8;
/** How far across the row a full swipe has to carry it, as a share of
    the row's width — iOS's own point, about where the action fills it. */
const FULL = 0.6;

const reveal = (r: HTMLElement, px: number) => r.style.setProperty("--row-reveal", `${px}px`);

/** Puts a row back, its actions hidden. */
function shutRow(r: HTMLElement) {
  r.removeAttribute("data-row-revealed");
  r.removeAttribute("data-row-full");
  reveal(r, 0);
}

export function useSwipeRows(ref: RefObject<HTMLElement | null>, on: boolean) {
  useEffect(() => {
    const list = ref.current;
    if (!list || !on) return;
    let row: HTMLElement | null = null;
    let open: HTMLElement | null = null;
    /** Where the touch began, how far the row stood open then, and the
        actions' own width — read once, at the touch's start: a full row's
        action grows with the drag, so its width mid-drag is the drag's. */
    let start: { x: number; y: number; base: number; actions: number; across: number; full: boolean } | null = null;
    let decided: "swipe" | "scroll" | null = null;
    let now = 0;
    // A touch that only closed an open row does nothing else (V16).
    let closing = false;
    const actionsOf = (r: HTMLElement) => r.querySelector<HTMLElement>("[data-row-actions-inner]")?.scrollWidth ?? 0;
    const shut = (r: HTMLElement | null) => {
      if (!r) return;
      shutRow(r);
      if (open === r) open = null;
    };
    /** The open row, if it still is one: a press on its action puts it
        back (or deletes it) behind this hook's back, and a row that was
        open then must not make the next touch a closing one. */
    const stillOpen = () => {
      if (open && (!open.isConnected || !open.hasAttribute("data-row-revealed"))) open = null;
      return open;
    };
    const down = (e: TouchEvent) => {
      if (closing) {
        start = null;
        return;
      }
      if (e.touches.length !== 1) return;
      const t = e.touches[0];
      const hit = (e.target as Element | null)?.closest<HTMLElement>("[data-row-swipe]") ?? null;
      // A touch on an open row's actions is a press on them.
      if (hit && stillOpen() === hit && (e.target as Element).closest("[data-row-actions]")) return;
      if (open && open !== hit) shut(open);
      if (!hit || t.clientX < EDGE) {
        start = null;
        return;
      }
      row = hit;
      const actions = actionsOf(hit);
      start = {
        x: t.clientX,
        y: t.clientY,
        base: open === hit ? actions : 0,
        actions,
        across: hit.getBoundingClientRect().width,
        full: hit.hasAttribute("data-row-full-swipe"),
      };
      decided = null;
      now = start.base;
    };
    const move = (e: TouchEvent) => {
      if (!start || !row) return;
      const t = e.touches[0];
      const mx = t.clientX - start.x;
      const my = t.clientY - start.y;
      if (decided === null) {
        if (Math.abs(mx) < SLOP && Math.abs(my) < SLOP) return;
        decided = Math.abs(mx) > Math.abs(my) && (mx < 0 || start.base > 0) ? "swipe" : "scroll";
        if (decided === "scroll") {
          // A scroll closes the open row, its own included (UX-44 §3).
          shut(open);
          start = null;
          return;
        }
        row.setAttribute("data-row-swiping", "");
      }
      e.preventDefault();
      e.stopPropagation();
      let r = start.base - mx;
      // Past the actions it gives, at a third of the finger's pace — a
      // row that a full swipe acts on follows the finger to its end.
      if (!start.full && r > start.actions) r = start.actions + (r - start.actions) / 3;
      now = Math.max(0, Math.min(r, start.across));
      reveal(row, now);
      if (start.full) {
        const past = now > Math.max(start.across * FULL, start.actions);
        if (past !== row.hasAttribute("data-row-full")) {
          row.toggleAttribute("data-row-full", past);
          // The tick iOS plays as the action takes the row.
          if (past) feel("selection");
        }
      }
    };
    const up = () => {
      if (!start || !row || decided !== "swipe") {
        start = null;
        return;
      }
      const { actions, across } = start;
      start = null;
      row.removeAttribute("data-row-swiping");
      if (row.hasAttribute("data-row-full")) {
        // Carried off the row's end, then done — the action's own press
        // (`SwipeRow`), which reads `data-row-full` and keeps it gone.
        reveal(row, across);
        if (open === row) open = null;
        row.querySelector<HTMLButtonElement>("[data-row-action][data-destructive]")?.click();
        return;
      }
      if (now > actions / 2) {
        reveal(row, actions);
        row.setAttribute("data-row-revealed", "");
        open = row;
      } else {
        shut(row);
      }
    };
    // Bubble, on the list: the scroller's pull to refresh is above it and
    // must not see a sideways swipe's moves.
    list.addEventListener("touchstart", down, { passive: true });
    list.addEventListener("touchmove", move, { passive: false });
    list.addEventListener("touchend", up);
    list.addEventListener("touchcancel", up);
    // **While a row is open, the next touch anywhere else only closes
    // it** (V16): its press is swallowed, so a tap on another row does
    // not also open that row — iOS's rule.
    const swallow = (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
    };
    const away = (e: Event) => {
      const was = stillOpen();
      if (!was || was.contains(e.target as Node)) return;
      shut(was);
      closing = true;
      document.addEventListener("click", swallow, { capture: true, once: true });
      const done = () => {
        closing = false;
        // A click that never came (a drag) must not swallow a later one.
        window.setTimeout(() => document.removeEventListener("click", swallow, { capture: true }), 400);
      };
      document.addEventListener("pointerup", done, { capture: true, once: true });
      document.addEventListener("pointercancel", done, { capture: true, once: true });
    };
    document.addEventListener("pointerdown", away, true);
    return () => {
      list.removeEventListener("touchstart", down);
      list.removeEventListener("touchmove", move);
      list.removeEventListener("touchend", up);
      list.removeEventListener("touchcancel", up);
      document.removeEventListener("pointerdown", away, true);
    };
  }, [ref, on]);
}

/**
 * One row that slides. With no `actions` it is the plain `div` it would
 * have been; with some, its children are the face that moves left and
 * the actions open from the right edge, in their own words — the
 * destructive one on the red, last, at the row's end.
 */
export function SwipeRow({
  actions,
  full = false,
  children,
  ...rest
}: ComponentProps<"div"> & {
  actions: RowAction[];
  /** A full swipe does the destructive action at once (UX-44 §3). */
  full?: boolean;
}) {
  if (actions.length === 0) return <div {...rest}>{children}</div>;
  return (
    <div {...rest} data-row-swipe="" data-row-full-swipe={full || undefined}>
      <div data-row-face className="flex min-w-0 flex-1 items-center">
        {children}
      </div>
      <div data-row-actions>
        <div data-row-actions-inner className="flex h-full">
          {actions.map((a) => (
            <button
              key={a.key}
              type="button"
              data-row-action={a.key}
              data-destructive={a.destructive || undefined}
              onClick={(e) => {
                const row = e.currentTarget.closest<HTMLElement>("[data-row-swipe]");
                // A full swipe leaves the row carried off while the act
                // runs; a press puts it back first.
                const gone = row?.hasAttribute("data-row-full") ?? false;
                if (row && !gone) shutRow(row);
                void Promise.resolve()
                  .then(a.run)
                  .then(
                    // Done and still drawn (the list kept it): back.
                    () => window.setTimeout(() => row?.isConnected && gone && shutRow(row), 600),
                    // Refused: the row comes back, as it was.
                    () => row && shutRow(row),
                  );
              }}
              className="min-w-(--row-action-w) flex-none px-in text-callout"
            >
              {/* Without the menu's ellipsis (UX-10; ux-9 I9): on a
                  swipe button 「删除…」 promised a form, and what
                  follows is the action sheet, not more input
                  (HIG Menus: the ellipsis is for an item that
                  needs more input before it runs). */}
              {a.label.replace(/…$/, "")}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
