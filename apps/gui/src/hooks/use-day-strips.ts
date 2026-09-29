// The day strips, on their own beat.
//
// **Not on the row list's.** The rows are a two-second heartbeat, and a
// day of cells per row hung on that would be a log riding a pulse. A
// strip is also the slowest-moving thing on the screen: one cell is
// several minutes wide, so a beat measured in seconds is already far
// finer than anything a person could see change.
//
// It follows the rest of the app's poll discipline rather than inventing
// one: slowed while nobody is looking (`use-hidden` has why slowed and
// not stopped), and re-asked when the width changes, because the number
// of cells is part of the question.
import { useEffect, useState } from "react";

import { fetchDayStrips, type DayRun } from "@/api";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";

/** How often the strips are re-asked while somebody is looking. */
const POLL_MS = 10_000;

/**
 * How many CSS pixels one cell gets, at the least.
 *
 * **Six: the smallest pill the band can draw, plus the gap after it.**
 * Every stretch is a pill now (2026-09-29, 「状态条也弄成圆角的bar」) —
 * fully round ends on a 4 px band, 2 px apart — so the narrowest stretch
 * that can still be *seen* is a 4 px dot, and it needs its 2 px of gap.
 * A narrower cell has two honest outcomes and both are bad: its pill is
 * shaved to a sliver the rounding eats (a stretch of the day silently not
 * there), or it is floored at a dot and overlaps its neighbours (two
 * words painted on top of each other).
 *
 * So the strip asks the node for cells that wide and **the node merges**
 * — `khor_core::DayCell::rank`, the same merge that has always decided a
 * coarse cell, biased so a cell shows the word that most wanted you. A
 * lone 待批 minute still wins its cell; what is lost is where in the
 * cell it was. Merging here instead would be a second copy of that
 * judgment, which `DayStrip`'s head refuses.
 *
 * The cost, said with its numbers: on a pane 900 wide a cell is **about
 * ten minutes** (it was three at two pixels a cell), on a phone about
 * twenty-three. A cell says *at some point in here it was 待批*, never
 * *it was 待批 for ten minutes*. (Before that, two and not one: a cell
 * one pixel wide lands on a single device pixel and the browser blends
 * it into its neighbours.)
 */
const CELL_WIDTH = 6;

/** The fewest cells worth asking for. Below this the strip stops being
    a day and becomes a smear; a pane that narrow simply gets a coarse
    strip rather than a wrong one. */
const MIN_CELLS = 24;
/** One cell per minute — the record's own resolution. Asking for more
    would be asking the node to invent detail it does not have. */
const MAX_CELLS = 1440;

/**
 * How many cells to ask for when the width could not be measured.
 *
 * **A width nobody could read is not a reason to draw nothing.** That
 * was the shape of the second failure here: `width` began at zero, zero
 * meant "do not ask", and every path where a measurement was missing or
 * late — a collapsed pane, a box not laid out yet, a `ResizeObserver`
 * delivery that never came — silently switched the whole feature off,
 * with no error and nothing on screen to notice.
 *
 * It is this codebase's own rule turned on this file: **一档接不上不会
 * 变成「没有值」，它会变成旁边那一档**. "I do not know how wide" must
 * land on *a resolution*, not on *no strip*. Being wrong about the count
 * is a cosmetic error that the next measurement corrects and that anyone
 * can see; asking for none is the feature being absent, which is exactly
 * the thing nobody can see.
 *
 * The number is ten minutes a cell — what the pane of a common window
 * yields at six pixels a cell — so the common unmeasured case is not
 * merely safe but close to right. (On a pane narrower than that it draws
 * pills floored at a dot until the first measurement lands.)
 */
const CELLS_UNMEASURED = 144;

/** Cells for an axis `widthPx` wide. **Rounded down**, so a cell is never
    narrower than `CELL_WIDTH`: rounding to nearest could hand a 900 px
    axis 150 cells of 5.99, and the pill in a cell like that is under its
    own height. */
export function cellsFor(widthPx: number): number {
  if (!Number.isFinite(widthPx) || widthPx <= 0) return CELLS_UNMEASURED;
  const wanted = Math.floor(widthPx / CELL_WIDTH);
  return Math.min(MAX_CELLS, Math.max(MIN_CELLS, wanted));
}

/**
 * The band's axis, from the box this hook watches.
 *
 * **The axis is narrower than the box, and at six pixels a cell that
 * matters**: cells counted for pixels the axis does not have come out
 * under six, and a pill in such a cell is under its own height. Two
 * things stand between: `DayStrip` draws its pills inside `px-in` (off
 * the board's rounded corners), and on the wide face the board itself
 * sits `mx-in` inside this box. The inset is read from the token, not
 * written here.
 *
 * - With a session open, the band's row (`data-detail-day`, the width
 *   of the pane) is measured, less the strip's inset — exact.
 * - Without one, the box less **both** insets — exact on the wide face
 *   and sixteen pixels short on the narrow one, where the board has no
 *   margin. Short is the safe side: a few fewer cells is a slightly
 *   coarser strip, and a few more would be overlapping pills.
 */
function axisOf(box: HTMLElement, boxWidth: number): number {
  const inset = parseFloat(getComputedStyle(box).getPropertyValue("--gap-in")) || 0;
  const row = box.querySelector<HTMLElement>("[data-detail-day]");
  const rowWidth = row ? row.getBoundingClientRect().width : 0;
  return rowWidth > 0 ? rowWidth - 2 * inset : boxWidth - 4 * inset;
}

/**
 * How wide the strips are, in CSS pixels.
 *
 * **Measured, not assumed.** The list is a resizable column
 * (`use-list-width`), the window is resizable, and the pane changes
 * shape between the wide and narrow postures — so any constant here
 * would be right on one screen and wrong on every other, and the way it
 * would be wrong is invisible: too many cells is a smear, too few is a
 * strip that has silently stopped being a day.
 *
 * A `ResizeObserver` rather than a resize listener, because most of the
 * ways this box changes width are not window resizes.
 *
 * # `present` exists because a ref is not a dependency
 *
 * The box being watched does not exist on the first render: the list
 * paints its empty state until the first poll answers, and the wrapper
 * this ref points at is not rendered on that path. An effect keyed only
 * on the ref object runs once, finds `current` null, gives up, and
 * **never runs again** — the ref object itself never changes, so
 * nothing re-triggers it when the node finally arrives.
 *
 * Measured, not reasoned: the strip shipped this way for one round and
 * the browser showed zero requests for a day strip on a list with four
 * rows and a wrapper 287 wide. Nothing threw, nothing was red, and the
 * band simply was not there — the failure of a ref-only dependency is
 * always silent, because "no node yet" and "no node ever" look the same
 * from inside the effect.
 *
 * So the caller says when there is something to observe, and that is a
 * value, which is a thing an effect can depend on.
 *
 * **…and since 2026-09-29 the box itself is a value too** (a callback
 * ref's state, not a ref object). The same silence had a second door:
 * the narrow face keys its screen box by the screen, so opening a
 * session *replaced* the node this watched while `present` stayed true.
 * The observer was left on a detached node, reported it as zero, and the
 * strip fell back to a count measured for no pane at all — invisible at
 * two pixels a cell, overlapping pills at six. A new node is now a new
 * value, and the effect follows it.
 */
export function useStripWidth(el: HTMLElement | null, present: boolean): number {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    if (!el) return;
    // **Read it here, synchronously, before trusting the observer with
    // anything.** An effect runs after layout, so the box already has
    // its size — and taking it now means the first strip does not wait
    // on an asynchronous delivery that may be late, may arrive as zero
    // while the pane is collapsed, or may never come at all in a tab
    // that is not being rendered. The observer's job is narrowed to
    // what it is actually good at: telling us when the width *changes*.
    setWidth(axisOf(el, el.getBoundingClientRect().width));
    const watch = new ResizeObserver(([entry]) => {
      // `contentRect`, so the box's own padding is not counted as strip:
      // the band's row is full-bleed inside this box, and its axis is
      // that less the strip's inset (`axisOf`).
      setWidth(axisOf(el, entry.contentRect.width));
    });
    watch.observe(el);
    return () => watch.disconnect();
  }, [el, present]);
  return width;
}

/**
 * Each row's last day, keyed by session id — or `null` until the first
 * answer has come back, which is what lets the band draw its skeleton
 * while the day is on its way instead of an empty row (`DayStrip`'s
 * 「Hidden until asked for」). After that the last answer stands while
 * the next is asked; it is never cleared back to `null`.
 *
 * A row with no entry paints no strip. That is a poll being one tick
 * out of step with the row list, not a fact about the session — the
 * node answers for every row it can see, including reported ones (they
 * come back 不知道 end to end, `khor_node::day::unwatched`), so
 * "missing" here never means "nothing to show".
 */
export function useDayStrips(width: number): Record<string, DayRun[]> | null {
  const [strips, setStrips] = useState<Record<string, DayRun[]> | null>(null);
  const hidden = useHidden();
  // No "zero means do not ask" branch any more, and its absence is the
  // fix: an unmeasured width now resolves to a resolution rather than to
  // silence (`cellsFor`). While the pane is collapsed this costs one
  // request for a band nobody can see, which is the right side of that
  // trade — the alternative was a band nobody could see *because it was
  // never fetched*, on a pane that was wide open.
  const cells = cellsFor(width);
  useEffect(() => {
    let live = true;
    const tick = () => {
      fetchDayStrips(cells)
        .then((s) => live && setStrips(s))
        // Silent: a strip that could not be fetched keeps the one it
        // has. Every other pane reports its own staleness and this one
        // has nothing to add — a row whose word is fresh and whose
        // strip is ten seconds old is not a state anybody can act on.
        .catch(() => {});
    };
    tick();
    const t = setInterval(tick, hidden ? HIDDEN_MS : POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [cells, hidden]);
  return strips;
}
