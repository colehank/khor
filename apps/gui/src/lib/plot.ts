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
 * (`components/PlotControls`, then still named `AxisPick`). What
 * disagreed was **the coordinate system they were placed into**,
 * measured 2026-09-08:
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
  /** Room for the y ticks' words. 50 fits 「3.8G」 and 「240.5%」, which
      are the widest either page produces — at `tick` 11 since
      2026-09-28 (46 fitted them at 10). */
  padL: 50,
  padR: 14,
  /** The two label strips, outside the plotting area and outside the
      tick gutters: at the top the figure's **head** — the vertical
      label at its left, the settings button at its right — and 34 at
      the bottom for the horizontal label and its ticks. A label inside
      the data area trades one collision for a worse one — points can be
      anywhere, tick numbers cannot.

      ⚠️ **The head is one finger tall (`touch`), and that is the fix,
      2026-09-28.** It was 22, and a 44px button cannot sit centred on a
      22px strip without 11px of it reaching into the data (the gate
      caught exactly that once: the button swallowed the press meant for
      `claude-opus-5`). So the button went to a corner outside the strip
      instead, and on the machine card it read as 「a lone ••• floating at
      the top-left」, level with nothing. With the head as tall as the
      button, the label and the button share one centre line and neither
      touches the data. What it costs is 22px of drawing, and each page
      gave its box that much more height rather than its data less. */
  padT: 44,
  padB: 34,
  /** A margin at both ends of each axis, **in pixels rather than as a
      fraction of the ruler**: the two extreme readings sit exactly on
      `at = 0` and `at = 1`, and a dot drawn on the frame line is half a
      dot. Eight clears the dot and its press target. */
  inset: 8,
  /** Tick words, at `caption` 11 — the floor of the type scale
      (docs/design-system 字). It was 10, the machine card's own, and the
      額度 card drew 8 before that; both were under the floor, and
      `check-face` now measures every word on the machine card against
      it. A number rather than the token because the drawing places
      words with it (`xTickY`); the gate is what keeps the two equal. */
  tick: 11,
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
      Mirrors `--touch`; `PlotControls` wears the class, this is the
      number the strip's offset is computed from. */
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
export function usePlotBox(initial: { w: number; h: number } = { w: 900, h: 200 }) {
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState(initial);
  useEffect(() => {
    const node = box.current;
    if (!node) return;
    const ro = new ResizeObserver(([e]) => {
      const { width, height } = e.contentRect;
      // **A zero is 「I cannot see it」, not 「it is that big」.** An
      // observer fires with 0×0 for an element that is display:none or
      // not laid out yet, and how early that happens is the engine's
      // business — WebKit reports it in places Chrome does not.
      //
      // This used to clamp to 1, and a height of 1 is worse than
      // useless: every figure here computes `ih = H - padT - padB`, so
      // H below 56 makes the inner height **negative** and the y axis
      // draws upside down. Measured 2026-09-10 on a real account: the
      // 0 tick landed 33px above the frame, on top of the title, with
      // 0.75$ below it — a chart that had turned over. Nothing errored.
      //
      // So a zero is dropped and the last real size stands. There is
      // nothing to draw for an element nobody can see, and the size it
      // had is the size it will have when it comes back.
      if (width <= 0 || height <= 0) return;
      // The floor is 「enough to draw in」, not 1: below this the axis
      // inverts, and a figure that inverts silently is the failure this
      // whole comment is about.
      setSize({ w: Math.max(FLOOR.w, width), h: Math.max(FLOOR.h, height) });
    });
    ro.observe(node);
    return () => ro.disconnect();
  }, []);
  return { box, ...size };
}

/**
 * The smallest box any figure here can be drawn in: the gutters plus a
 * sliver of plotting area. Anything smaller inverts an axis rather than
 * looking cramped, so it is a floor rather than a preference.
 */
const FLOOR = { w: PLOT.padL + PLOT.padR + 8, h: PLOT.padT + PLOT.padB + 8 };

/**
 * Where an axis label sits, as an inline style.
 *
 * **The one thing the two pages could not agree on**, because it is the
 * only place a pixel constant had to survive the trip into a scaled
 * canvas. Both strips are `PLOT.touch` tall while the strip they sit in
 * is not, so each is centred on its strip and overflows it invisibly:
 * what has to land inside is the ink, not the hit area.
 */
/**
 * **Where a tick's word sits under the horizontal axis.**
 *
 * One expression for every figure on both pages, because it was two:
 * the scatters wrote `H - padB + 14` and the 額度 trend chart wrote
 * `H - 6` — **14px lower, and straight through the strip
 * `labelSeat("x")` reserves for the axis's own name**. Measured
 * 2026-09-09: that is why the bar chart could not carry its label the
 * way the scatters do, and why its controls had to sit outside the
 * figure instead.
 *
 * `+ 14` puts the word's baseline below the axis line rather than on
 * it; `padB` is the room set aside under the drawing for exactly this.
 */
export function xTickY(h: number): number {
  return h - PLOT.padB + 14;
}

export function labelSeat(which: "x" | "y" | "settings"): React.CSSProperties {
  // **Three corners of four.** The two labels take the corners a
  // scatter is least likely to put a point in (min-x max-y, max-x
  // min-y); the settings button takes **top-right**, the one nothing
  // else was using.
  //
  // ⚠️ **It sat bottom-left until 2026-09-10**, with a note arguing
  // that a panel opening upward from there covers the stretch of axis
  // nearest the origin rather than the data. The user asked for the
  // top-right corner (「设置图表绘制的三个点,放在图表的右上角」), and
  // the old argument was answering a question nobody had: **where the
  // panel lands matters only while it is open, and while it is open
  // the panel is what you are reading.** Where the *button* sits
  // matters whenever the figure is on screen — and a control in the
  // top-right is where this app puts a thing's options everywhere
  // else. The panel now opens downward from it.
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
  //   ⚠️ **The head row's right end** (2026-09-28, from a walk of the
  //   whole face: 「a lone ••• floating at the card's top-left corner —
  //   controls belong in the card's head row, right end, aligned with the
  //   head's text」). The head is `padT` tall, which is exactly the
  //   button, so the button *is* the strip's height: its centre is the y
  //   label's centre, it ends where the data begins, and its glyph's
  //   right edge lands on the data's right edge (`padR` is 14, the glyph
  //   16 centred in 44). That is the corner where this app puts a
  //   thing's options everywhere else.
  //
  //   ⚠️ **Tombstone: top-left, `{ left: 0, top: padT - touch }`,
  //   2026-09-10 → 2026-09-28** (the user then:「三个点的x、y、面积设置
  //   按钮,放在图的左上角」). It spent one afternoon at bottom-left and
  //   one at top-right before that; what the corners taught still holds
  //   and is why this seat is safe now:
  //
  //   - **Inside the data is not a seat.** Centred on a 22px strip, 11px
  //     of a 44px hit area reached into the plotting area, and the gate
  //     caught it in one run: the button 「intercepts pointer events」
  //     for `claude-opus-5` and the press never landed. The box still
  //     ends exactly where the data begins — because the strip is now
  //     the button's own height (`padT`), not because the button hides
  //     in a corner.
  //   - A seat above the drawing, in a corner level with nothing, is
  //     what the 2026-09-28 walk read as floating.
  return { right: 0, top: 0 };
}

/**
 * **Which quantities may be offered as a dot's area: the ones every row
 * on this screen has a reading for.**
 *
 * 2026-09-08. The 額度 card encodes a third quantity as the dot's area
 * and the machine card was to do the same — until the measurement:
 * `gpu_mem` is null on **200 of 200 rows on two of the three machines**
 * (`zh.toml`'s `top-plot-off` records it; the gate's own fixture has it
 * on 2 of 23). Offered as an area, that draws every one of those dots
 * at the floor, and **a floor-sized dot reads as 「the smallest」** —
 * so the picture would be saying those two hundred processes use the
 * least video memory, which is the neighbouring-value error this plot
 * has always been built against.
 *
 * ⚠️ **This is stricter than the rule that decides whether a column is
 * drawn, and the difference is not an inconsistency.** That one asks
 * 「has *any* row ever filled this?」 (`Seen`), because what it decides
 * is whether a whole column exists, and a missing cell there is simply
 * an **empty cell** — a thing the reader can see is absent. The area
 * decides a **radius**, and the point has to exist regardless: it is
 * carrying X and Y. **There is no 「do not draw」 for it**, so a missing
 * reading has nowhere to go but the floor. The same question, asked of
 * two things whose 「missing」 looks different, gets two answers.
 *
 * **Nobody maintains a list.** The day a machine reports video memory
 * for every process, that quantity appears in the picker on its own;
 * the day one stops, it leaves. The gate asserts the property rather
 * than the names.
 */
export function everyRowHas<K extends string, R>(
  keys: readonly K[],
  rows: R[],
  read: (k: K, r: R) => number | null | undefined,
): K[] {
  if (rows.length === 0) return [];
  return keys.filter((k) => rows.every((r) => {
    const v = read(k, r);
    return v !== null && v !== undefined && Number.isFinite(v);
  }));
}

/**
 * **How big the dots may get, given how many there are.**
 *
 * ⚠️ **A fixed range cannot serve both figures.** The 額度 card draws
 * ten models across a wide band; the machine card draws two hundred
 * processes. The same `19` that is a comfortable dot for ten tiles the
 * drawing for two hundred — the machine card's own waiver says it in
 * as many words. So the range is a function of the count and the room,
 * and the rule is one rule.
 *
 * The floor is the machine card's drawn dot (1.8): the smallest reading
 * still has to be a dot rather than nothing. The ceiling is the square
 * root of the room each point would get if they were spread evenly,
 * quartered — a dot is not meant to fill its share, only to be
 * comparable with its neighbours — and capped where a dot stops being
 * a dot.
 */
export function dotSpan(n: number, w: number, h: number): { min: number; max: number } {
  const min = 1.8;
  const room = Math.sqrt(Math.max(w, 1) * Math.max(h, 1) / Math.max(n, 1)) / 4;
  return { min, max: Math.max(min + 0.5, Math.min(19, room)) };
}

/**
 * A dot's radius for a reading.
 *
 * **The area carries the value, so the radius is its square root** — a
 * radius read straight off the number makes a 4× value look 16× big.
 * The floor is not zero (the smallest dot still has to be there), so
 * the two ends are interpolated **in area** and taken back to a radius.
 */
export function dotRadius(v: number, vmax: number, span: { min: number; max: number }): number {
  const t = vmax > 0 ? Math.max(v, 0) / vmax : 0;
  return Math.sqrt(span.min * span.min + (span.max * span.max - span.min * span.min) * t);
}
