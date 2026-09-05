// Whether a box is too narrow for the layout it wants — asked of **the
// box**, not of the window.
//
// The machine card's part rows want three columns: a fixed name, a
// shared bar track, a fixed column of numbers. Whether those fit is a
// fact about the card, and the card is not the window: a rail and a
// list pane sit beside it, so at any given viewport the card is several
// hundred narrower than the screen. Keying the layout off the window's
// breakpoint put an overflow band right above it — measured 2026-09-04,
// 106 pixels of sideways overflow from 720 through 830, found by the
// full-bandwidth scan and by nothing else.
//
// So this measures the element. It is a container query written in TS,
// for the reason a container query would be right: the question is
// 「does this fit in here」 and only the box knows.
import { useEffect, useState, type RefObject } from "react";

export function useTight(ref: RefObject<HTMLElement | null>, min: number): boolean {
  const [tight, setTight] = useState(false);
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    const ro = new ResizeObserver(([entry]) => {
      setTight(entry.contentRect.width < min);
    });
    ro.observe(node);
    return () => ro.disconnect();
  }, [ref, min]);
  return tight;
}

/**
 * Whether a grid's tracks do not fit the box around it — **measured on
 * the glass rather than computed from a list of widths.**
 *
 * `useTight` above answers the same shape of question against a
 * constant, and that is right where the layout wants a fixed number of
 * fixed things. It is wrong where **the number of tracks changes**: the
 * process table has between seven and eleven columns depending on what
 * the machine can fill, so the width at which it stops fitting is not a
 * number anybody can write down once.
 *
 * It was written down anyway, first, as a table of per-column pixel
 * costs beside the `--cw-*` tokens they were copied from. Two lists to
 * keep in agreement, and the estimate was **64px too high** on the
 * eleven-column set — which stacked the table at a width where it fit
 * perfectly well, and took the column heads (and with them the sorting)
 * off a card that had room for them. The browser already knows the
 * answer; asking it is both shorter and correct.
 *
 * # Why it needs to remember
 *
 * `scrollWidth` is an honest minimum **only while the grid is laid out
 * as a grid**. Once the narrow layout has replaced it with stacked
 * rows there is nothing left to measure, so 「can we go back」 cannot be
 * asked the same way. It is answered from the box instead: try again
 * once the box is clear of the width where it stopped fitting.
 *
 * **And it remembers by how much**, not by a round number. A constant
 * was tried first (24px) and was wrong in the expensive direction:
 * measured 2026-09-05, the table came up 3px short at a box of 690, so
 * it stacked — and then would not come back at 710, because 710 is not
 * 690 + 24. A card with room for its eleven columns drew none of its
 * heads. The shortfall is a number the browser already told us, so the
 * retry uses it: come back when the box has grown by at least what was
 * missing.
 *
 * **Monotone, so it cannot oscillate**: a retry that overflows again
 * records the new width and the new shortfall, so each attempt raises
 * the bar rather than bouncing off it.
 */
export function useOverflows(
  box: RefObject<HTMLElement | null>,
  grid: RefObject<HTMLElement | null>,
  deps: unknown,
): boolean {
  const [stacked, setStacked] = useState<{ at: number; short: number } | null>(null);
  // A different set of columns is a different question — start over
  // rather than judge the new set by the old set's width.
  useEffect(() => setStacked(null), [deps]);
  useEffect(() => {
    const b = box.current;
    if (!b) return;
    const look = () => {
      const w = b.clientWidth;
      const g = grid.current;
      // One pixel of tolerance: sub-pixel track rounding otherwise
      // reads as an overflow on a table that fits exactly.
      const short = g ? g.scrollWidth - w : 0;
      setStacked((was) => {
        if (was === null) return short > 1 ? { at: w, short } : null;
        return w >= was.at + was.short ? null : was;
      });
    };
    look();
    const ro = new ResizeObserver(look);
    ro.observe(b);
    // A frame later too: the first pass can land before the browser has
    // resolved the tracks, and `scrollWidth` is then the box's own.
    const raf = requestAnimationFrame(look);
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  }, [box, grid, deps]);
  return stacked !== null;
}

/**
 * How many rows of a scrolling box are below its bottom edge right now.
 *
 * **Counted, not computed from a total.** 「the list has six and four
 * fit」 is a different claim from 「two are out of sight」, and only the
 * second one goes away when somebody scrolls — a note that never
 * changes stops being a state and becomes part of the heading.
 *
 * **Measured on the cells, not on the rows.** A row here joins its
 * grid with `display: contents` and therefore has no box at all;
 * `getBoundingClientRect()` on one returns zeros, and a count built on
 * that reports 「six visible of six」 about a panel that is visibly
 * scrolling. So each row is judged by its first child, which is a real
 * box.
 */
export function useHiddenRows(ref: RefObject<HTMLElement | null>, deps: unknown): number {
  const [hidden, setHidden] = useState(0);
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    const count = () => {
      const bottom = node.getBoundingClientRect().bottom;
      let below = 0;
      for (const row of node.querySelectorAll("[data-part]")) {
        const cell = row.firstElementChild;
        if (!cell) continue;
        // **Not fully visible counts as below.** A row clipped in half
        // by the box's edge is half a row you cannot read, and dragging
        // is what brings it up — counting it as 「visible」 undercounts
        // exactly what the note is offering (seen on the real face:
        // the note said 1 while two rows were out of sight).
        if (cell.getBoundingClientRect().bottom > bottom + 1) below += 1;
      }
      setHidden(below);
    };
    count();
    // **Watch the content, not just the box.** This box has a fixed
    // height on purpose, so it never resizes — and a `ResizeObserver`
    // on it alone fires once at mount, before the rows have been laid
    // out, and then never again. The count stayed at whatever it was
    // in that first frame: zero, on a panel with three rows out of
    // sight. So every child is observed too, which is what actually
    // changes when the drawer switches groups.
    const ro = new ResizeObserver(count);
    ro.observe(node);
    for (const child of node.children) ro.observe(child);
    node.addEventListener("scroll", count, { passive: true });
    // A frame later as well: the first pass can land before the
    // browser has finished laying the rows out.
    const raf = requestAnimationFrame(() => requestAnimationFrame(count));
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
      node.removeEventListener("scroll", count);
    };
  }, [ref, deps]);
  return hidden;
}
