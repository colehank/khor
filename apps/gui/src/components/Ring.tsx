// Ported from mandala's `apps/desktop/src/components/Ring.tsx` — the arc
// geometry and the one constant that was measured rather than chosen.
//
// **What is deliberately not ported**, since a port is also a decision
// about what to leave behind:
//
//   - `severityColor()`. It grades `used / total` into four colours in
//     the frontend, and khor already answers that question in Rust
//     (`Strain`, whose own doc says the thresholds are a judgment and
//     must not live in a screen, because two frontends eyeballing the
//     same two numbers disagree about the same machine). Colour is a
//     prop here; who decides it is the caller's business and, for
//     strain, the node's.
//   - `RING_SIZE_MINI`. It exists for mandala's menubar panel. khor has
//     no menubar panel, and a constant with no caller is a size the next
//     person will reach for by accident.
//   - `strokeFor()`'s size ladder. It branches on 56/50/42, which only
//     means anything when several diameters coexist — and the rule below
//     is that they do not.
//
// The centre of a ring is the caller's: a number, a word, or nothing.
import { cn } from "@/lib/utils";


/**
 * How opaque the **unfilled** part of a track is.
 *
 * **Measured, not chosen**, and it is the whole reason this file has a
 * constant instead of an inline number. With the track too faint, a ring
 * at 5% is just a short arc floating in nothing, and it reads as a
 * *smaller circle* than a ring at 95% — even though both have the same
 * radius and the same stroke width, to the pixel. mandala's user put it
 * as「可别圆环 0% 就比 75% 小了,这不对」.
 *
 * The band is narrow at both ends. Below this the track sinks into the
 * card (it was .15, and on the dark theme it was effectively invisible);
 * above it the track starts competing with the filled arc, and "how full
 * is this one" gets harder to read rather than easier.
 *
 * One number for every theme and every colour: this layer decides **how
 * present the track is**, which is not a question about which hue is
 * drawing it.
 */
export const TRACK_OPACITY = 0.35;

/**
 * The one diameter, for every ring this app draws.
 *
 * **The rule is the point, not the number.** Several sizes on one screen
 * make size itself say something — and what it says is false: a
 * machine's CPU is not more or less important than its disk because one
 * circle came out larger. mandala grew four sizes side by side before
 * collapsing them to one.
 *
 * (The comment on mandala's constant claims the single size is 58 while
 * the constant is 64 and the paragraph under it says 64. The number 58
 * appears nowhere else in that file. Only the rule survived the port.)
 */
export const RING_SIZE = 64;

/**
 * The diameter a ring takes **inside a list row**.
 *
 * **Equal to `--avatar` on purpose, and that is the single-diameter rule
 * being kept rather than broken.** Two circles on one row are two facts;
 * the moment one is bigger, size starts claiming one of them matters
 * more, which is exactly what the rule guards against. Today's caller
 * is the 额度 page's account rows — the machine rows' reading stack,
 * this constant's first customer, left on 2026-09-02 (「这个环很抽象,
 * 看不明白。取消」).
 */
export const RING_SIZE_ROW = 32;
/** The machine card's rings (2026-09-03, to the user's reference): a
    third again larger than a row's and drawn thinner, so the number
    inside leads and the arc reads as a gauge rather than a badge. */
export const RING_SIZE_CARD = 88;
export const RING_STROKE_CARD = 4;

/** Stroke for a single-track ring. Fixed, because the diameter is. */
const STROKE = 5.5;

/** A ring's arc, and nothing else. `fraction` of `null` draws track only. */
function Arc({
  cx,
  r,
  stroke,
  color,
  fraction,
}: {
  cx: number;
  r: number;
  stroke: number;
  color: string;
  fraction: number | null;
}) {
  const circumference = 2 * Math.PI * r;
  const filled = fraction == null ? null : Math.max(0, Math.min(1, fraction));
  return (
    <>
      {/* The track. Drawn even when there is nothing to fill: a ring with
          no reading is a ring that says so, and an absent circle would
          read as a reading of zero. */}
      <circle
        cx={cx}
        cy={cx}
        r={r}
        fill="none"
        stroke={color}
        strokeOpacity={filled == null ? 1 : TRACK_OPACITY}
        strokeWidth={stroke}
      />
      {filled != null && filled > 0 && (
        <circle
          // The arc that carries the reading. Named because something
          // outside has to be able to ask "did a gauge actually draw",
          // and the honest answer is this element — a wrapper would be
          // present whether or not there was anything to show.
          data-ring-fill
          cx={cx}
          cy={cx}
          r={r}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={`${circumference * filled} ${circumference}`}
          // Twelve o'clock, not three: a gauge that starts anywhere else
          // has to be read before it can be compared.
          transform={`rotate(-90 ${cx} ${cx})`}
        />
      )}
    </>
  );
}

/**
 * One measure, drawn as one ring, with whatever the caller puts in the
 * middle.
 *
 * **A number in the middle means one track.** The stacked half of the
 * vocabulary (`NestedRing`, concentric tracks) left with the machine
 * rows' reading stack on 2026-09-02 (「这个环很抽象,看不明白。取消」)
 * — every ring this app draws now is one measure with its own centre.
 */
export function Ring({
  fraction,
  color,
  size = RING_SIZE,
  stroke = STROKE,
  centerClass = "text-fact",
  children,
}: {
  /** `null` when there is no denominator — then only the track is drawn. */
  fraction: number | null;
  color: string;
  size?: number;
  /** The arc's width. The default is the row size's; a large ring on
      the machine card is drawn thinner so the number inside leads. */
  stroke?: number;
  /**
   * The size tier the centre reads at.
   *
   * **A tier, not a free class**, and there is exactly one caller that
   * needs a second one: the subscription's arcs, where the percentage is
   * the whole reason the panel exists. `--text-hero`'s own note in
   * tokens.css names「a quota arc's centre」as what that size is for,
   * and nothing had ever used it there — the number sat at `--text-fact`
   * in a panel of `--text-aux` lines, a step of one and a half units
   * standing in for "this is the fact". (Sizes named, not spelled: this
   * file is inside the px gate.)
   *
   * Everywhere else the default holds: a machine's rings sit four to a
   * row and none of them outranks another.
   */
  centerClass?: string;
  children?: React.ReactNode;
}) {
  const r = (size - stroke) / 2;
  return (
    <span
      data-ring
      className="relative inline-grid place-items-center"
      style={{ width: size, height: size }}
    >
      <svg width={size} height={size} aria-hidden="true" className="col-start-1 row-start-1">
        <Arc cx={size / 2} r={r} stroke={stroke} color={color} fraction={fraction} />
      </svg>
      {/* The centre sits on top of the same grid cell rather than inside
          the SVG: it is text the app styles like all its other text, and
          `<text>` would put it outside everything the type scale reaches. */}
      {children != null && (
        <span
          data-ring-center
          className={cn("col-start-1 row-start-1 font-title tabular-nums", centerClass)}
        >
          {children}
        </span>
      )}
    </span>
  );
}

/** One measure of a set: its own share, its own colour — the shape
    `vitalsTracks` hands the mark's node card, which draws each as its
    own `Ring`. (It fed a concentric `NestedRing` stack on the machine
    rows until 2026-09-02 —「这个环很抽象,看不明白」— and the stack
    went with that judgment; a set is several rings now, never one
    ring of several tracks.) */
export type Track = { id: string; fraction: number | null; color: string };
