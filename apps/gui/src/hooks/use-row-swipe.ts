// **A row slid left shows what can be done to it** (the audit fix plan
// F7.5; narrow P-C): 置顶 / 取消置顶 and the destructive one, the row
// menu's own items in its words. Narrow only.
//
// - **The direction is decided before anything moves**: a touch that
//   goes down first is the list's scroll (or its pull to refresh), and
//   one that starts within `EDGE` of the screen's left side is the back
//   swipe's (`use-edge-swipe`).
// - **A full swipe only reveals.** Ending a session because a thumb went
//   a little too far is the accident this refuses: the actions are
//   pressed, never thrown.
// - One row open at a time; a touch anywhere else closes it.
//
// Moved by variables on the row (`--row-reveal`, `app.css`
// `[data-row-swipe]`), never by React state.
import { useEffect, type RefObject } from "react";

const EDGE = 20;
const SLOP = 8;

export function useRowSwipe(ref: RefObject<HTMLElement | null>, on: boolean) {
  useEffect(() => {
    const list = ref.current;
    if (!list || !on) return;
    let row: HTMLElement | null = null;
    let open: HTMLElement | null = null;
    let start: { x: number; y: number; base: number } | null = null;
    let decided: "swipe" | "scroll" | null = null;
    let reveal = 0;
    const width = (r: HTMLElement) => r.querySelector<HTMLElement>("[data-row-actions-inner]")?.scrollWidth ?? 0;
    const set = (r: HTMLElement, px: number) => r.style.setProperty("--row-reveal", `${px}px`);
    const shut = (r: HTMLElement | null) => {
      if (!r) return;
      r.removeAttribute("data-row-open");
      set(r, 0);
      if (open === r) open = null;
    };
    const down = (e: TouchEvent) => {
      if (e.touches.length !== 1) return;
      const t = e.touches[0];
      const hit = (e.target as Element | null)?.closest<HTMLElement>("[data-row-swipe]") ?? null;
      // A touch on an open row's actions is a press on them.
      if (hit && open === hit && (e.target as Element).closest("[data-row-actions]")) return;
      if (open && open !== hit) shut(open);
      if (!hit || t.clientX < EDGE) {
        start = null;
        return;
      }
      row = hit;
      start = { x: t.clientX, y: t.clientY, base: open === hit ? width(hit) : 0 };
      decided = null;
      reveal = start.base;
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
          start = null;
          return;
        }
        row.setAttribute("data-row-swiping", "");
      }
      e.preventDefault();
      e.stopPropagation();
      const full = width(row);
      let r = start.base - mx;
      // Past the actions it gives, at a third of the finger's pace.
      if (r > full) r = full + (r - full) / 3;
      reveal = Math.max(0, r);
      set(row, reveal);
    };
    const up = () => {
      if (!start || !row || decided !== "swipe") {
        start = null;
        return;
      }
      start = null;
      row.removeAttribute("data-row-swiping");
      const full = width(row);
      if (reveal > full / 2) {
        set(row, full);
        row.setAttribute("data-row-open", "");
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
    const away = (e: Event) => {
      if (open && !open.contains(e.target as Node)) shut(open);
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
