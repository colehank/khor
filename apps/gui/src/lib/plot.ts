import { useEffect, useRef, useState } from "react";

/**
 * **One drawing method for every figure in this app.**
 *
 * 2026-09-08, the user: 「x和y的label和机器对不上啊,位置、样式.」 and
 * 「二者统一出一个绘图方法,包括样式、字号等.二者绘图都复用这个方法.」
 *
 * # What was actually different, and it was not the control
 *
 * Both pages already drew their axis labels with the same component
 * (`components/AxisPick`). What disagreed was **the coordinate system
 * they were placed into**, measured 2026-09-08:
 *
 * | | machine card | 額度 card |
 * |---|---|---|
 * | canvas | `<svg width={W} height={H}>`, W/H from a `ResizeObserver` — **one unit is one pixel** | `viewBox="0 0 880 280"` + `h-auto w-full` — **scaled** |
 * | y label | `left: padL, top: (padT - 44) / 2` | `left: "6.59%", top: 0` |
 * | tick ink | `fontSize={10}` | `fontSize={8}` |
 *
 * **The percentages are the tell.** `6.59%` is `58 / 880` and `2.05%`
 * is `18 / 880`: a pixel offset divided by a viewBox width, because on
 * a scaling canvas a pixel constant means nothing. So the labels could
 * not be placed the way the machine card places them, and a second set
 * of numbers grew to approximate the first.
 *
 * # So the unit is the pixel, on both
 *
 * A scaling `viewBox` cannot honour a pixel: an 8px tick on an 880-unit
 * box drawn 984 wide is 8.9px, and a 44px finger target is 44 only at
 * one width. The machine card's own note says why it stopped stretching
 * (a circle under a non-uniform stretch is an ellipse, and its halo
 * promises a radius). **This module is that decision, made once**: the
 * box is measured, the svg is drawn at that size, and every number
 * below is a real pixel on both pages.
 *
 * # ⚠️ What this cannot hold
 *
 * **A figure's height.** The two drawings hold different things — a
 * scatter of ten labelled models needs more air than one of two hundred
 * unlabelled processes — so each page picks its own box height and
 * says so. What is shared is everything that decides *how a figure is
 * drawn*, not how much room it is given.
 *
 * And **sharing the numbers is not the same as using them**. A page can
 * import this and then write its own `fontSize={8}` two lines later;
 * nothing here would notice. That is what the cross-page gate in
 * `check-face.mjs` is for — it measures both figures on one run and
 * compares what was drawn, not what was imported.
 */
export const PLOT = {
  /** Room for the y ticks' words. 46 fits 「3.8G」 and 「240.5%」, which
      are the widest either page produces. */
  padL: 46,
  padR: 14,
  /** The two label strips, outside the plotting area and outside the
      tick gutters: 22 at the top for the vertical label, 34 at the
      bottom for the horizontal one and its ticks. A label inside the
      data area trades one collision for a worse one — points can be
      anywhere, tick numbers cannot. */
  padT: 22,
  padB: 34,
  /** A margin at both ends of each axis, **in pixels rather than as a
      fraction of the ruler**: the two extreme readings sit exactly on
      `at = 0` and `at = 1`, and a dot drawn on the frame line is half a
      dot. Eight clears the dot and its press target. */
  inset: 8,
  /** Tick words. 10, the machine card's — the 額度 card drew 8, which
      on its scaled canvas rendered near 9 and looked like neither. */
  tick: 10,
  /** How many ticks an axis carries, ends included. Seven is the
      machine card's count; the 額度 card drew five, so the same axis
      told two different stories about how finely it could be read. */
  ticks: 7,
  /** The frame's own two lines, and the little marks off them. Ink
      rather than width: a hairline is 1px and what varies is how much
      of the page shows through. */
  frameInk: 0.15,
  tickInk: 0.3,
  tickWordInk: 0.5,
  /** A coloured line needs about 1.8× the weight of an ink one to read
      as the same (`day/day-core.js` draws its hairlines at 0.6). */
  hair: 1.1,
  /** The floor a finger needs, so a label strip can be centred on it.
      Mirrors `--touch`; `AxisPick` wears the class, this is the number
      the strip's offset is computed from. */
  touch: 44,
} as const;

/**
 * The even fractions an axis's ticks sit at, ends included.
 *
 * Where they *land* is the ruler's business (`lib/ruler`) — on a square
 * root these are fractions of the value and crowd toward the top, on a
 * log they are fractions of the axis and the labels come out uneven.
 * This only decides how many there are.
 */
export function tickFractions(n: number = PLOT.ticks): number[] {
  return Array.from({ length: n }, (_, k) => k / (n - 1));
}

/**
 * A box that reports its own size in pixels, so the drawing inside it
 * can be one unit to one pixel.
 *
 * The initial guess is drawn once before the observer answers; it is
 * deliberately close to what either page gives a figure, so the first
 * paint is not a figure of a different shape jumping into place.
 */
export function usePlotBox(initial: { w: number; h: number } = { w: 900, h: 170 }) {
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState(initial);
  useEffect(() => {
    const node = box.current;
    if (!node) return;
    const ro = new ResizeObserver(([e]) =>
      setSize({ w: Math.max(1, e.contentRect.width), h: Math.max(1, e.contentRect.height) }),
    );
    ro.observe(node);
    return () => ro.disconnect();
  }, []);
  return { box, ...size };
}

/**
 * Where an axis label sits, as an inline style.
 *
 * **The one thing the two pages could not agree on**, because it is the
 * only place a pixel constant had to survive the trip into a scaled
 * canvas. Both strips are `PLOT.touch` tall while the strip they sit in
 * is not, so each is centred on its strip and overflows it invisibly:
 * what has to land inside is the ink, not the hit area.
 */
export function labelSeat(which: "x" | "y" | "settings"): React.CSSProperties {
  // **Three corners, and the fourth is left empty on purpose.** The
  // two labels take the corners a scatter is least likely to put a
  // point in (min-x max-y, max-x min-y); the settings button takes the
  // third, bottom-left, so a panel opening upward from it covers the
  // stretch of axis nearest the origin rather than the data.
  // ⚠️ **A label is centred on its strip; a button is centred on its
  // own height.** Until 2026-09-08 every seat here assumed a
  // `min-h-touch` control and offset by `(strip - 44) / 2`, which is
  // negative — the control hung out of the strip on purpose, because
  // what had to land inside was the ink, not the hit area. The labels
  // stopped being controls that day and became ~18px of text, and the
  // same negative offset put them **outside the drawing**; the gate
  // caught it (「a label in the flow spends a row of the card on one
  // word」). So a label gets the strip as its box and centres in it,
  // which holds at any text size, and only the button keeps the old
  // arithmetic.
  const middle = { display: "flex", alignItems: "center" } as const;
  if (which === "y") return { left: PLOT.padL, top: 0, height: PLOT.padT, ...middle };
  if (which === "x") return { right: PLOT.padR, bottom: 0, height: 12, ...middle };
  return { left: PLOT.padL, bottom: (12 - PLOT.touch) / 2 };
}
