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
import { useEffect, useState, type RefObject } from "react";

import { fetchDayStrips, type DayRun } from "@/api";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";

/** How often the strips are re-asked while somebody is looking. */
const POLL_MS = 10_000;

/**
 * How many CSS pixels one cell gets.
 *
 * **Two, not one.** A day holds 1440 minutes and a row's band is a
 * couple of hundred pixels wide, so something has to merge — and the
 * node does (`khor_core::DayCell::rank`). What is decided here is only
 * how many cells to ask for, and one per pixel is the tempting wrong
 * answer: a cell that thin lands on a single device pixel at the
 * commonest ratio and the browser blends it with its neighbours, so a
 * lone 待批 cell — the one thing the strip exists to make visible —
 * comes out as a tint of whatever surrounds it. Twice that survives.
 *
 * The cost is honest and worth saying: on a band a few hundred wide a
 * cell is **about a quarter of an hour**, and the node's merge is
 * biased so a cell shows the word that most wanted you. So a cell says
 * *at some point in here it was 待批*, never *it was 待批 for a quarter
 * of an hour*.
 */
const CELL_WIDTH = 2;

/** The fewest cells worth asking for. Below this the strip stops being
    a day and becomes a smear; a pane that narrow simply gets a coarse
    strip rather than a wrong one. */
const MIN_CELLS = 24;
/** One cell per minute — the record's own resolution. Asking for more
    would be asking the node to invent detail it does not have. */
const MAX_CELLS = 1440;

export function cellsFor(widthPx: number): number {
  const wanted = Math.round(widthPx / CELL_WIDTH);
  return Math.min(MAX_CELLS, Math.max(MIN_CELLS, wanted));
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
 */
export function useStripWidth(box: RefObject<HTMLElement | null>): number {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const watch = new ResizeObserver(([entry]) => {
      // `contentRect`, so padding is not counted as strip: the band is
      // full-bleed inside this box and that is exactly its width.
      setWidth(entry.contentRect.width);
    });
    watch.observe(el);
    return () => watch.disconnect();
  }, [box]);
  return width;
}

/**
 * Each row's last day, keyed by session id.
 *
 * A row with no entry paints no strip. That is a poll being one tick
 * out of step with the row list, not a fact about the session — the
 * node answers for every row it can see, including reported ones (they
 * come back 不知道 end to end, `khor_node::day::unwatched`), so
 * "missing" here never means "nothing to show".
 */
export function useDayStrips(width: number): Record<string, DayRun[]> {
  const [strips, setStrips] = useState<Record<string, DayRun[]>>({});
  const hidden = useHidden();
  const cells = width > 0 ? cellsFor(width) : 0;
  useEffect(() => {
    if (cells === 0) return;
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
