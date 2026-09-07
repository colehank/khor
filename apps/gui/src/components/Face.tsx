/**
 * A reading drawn as a **210° dial**, with the parts under it marked on
 * the rim.
 *
 * # Why not the full circle it replaced
 *
 * A ring says how much of the whole is taken. It cannot say **how that
 * amount is spread over the things it is made of** — and on a machine
 * with four cards those are two different questions with the same
 * answer on screen. One card pinned and three idle averages to the same
 * arc as four cards half busy, and the person deciding where to run a
 * job needs the idle card, not the mean (`khor_core::Gpu::util_pct`
 * carries that ruling in the crate).
 *
 * So each part gets a mark on the rim, **at the angle of its own
 * reading, on the same scale the summary arc uses**. Four marks spread
 * across most of the dial is one busy card among idle ones; four marks
 * stacked in one place is four cards doing the same thing — while the
 * summary arc is identical in both. A hairline outside the rim spans
 * from the lowest mark to the highest, so "spread out or not" has a
 * length, and a length can be measured.
 *
 * # The geometry is copied, not re-derived
 *
 * `ARC0`, `SPAN`, the radius, the milestone dots and the two rim radii
 * come across verbatim from the prototype (`ring-rim-pips.html`) — as
 * **proportions of the size they were measured at** (`REF_SIZE`), so
 * the same drawing comes out at any size and the machine card's, at 88,
 * is those numbers unchanged.
 *
 * ⚠️ **The line above used to end「and the check that this matches the
 * drawing it came from compares lengths」. There is no such check** —
 * grepped 2026-09-07 for `ring-rim-pips` and `getTotalLength` across
 * `apps/gui` / `crates` / `docs` and the only hit is this paragraph
 * (the finder proved on `data-dial-pip`, which is found where it
 * lives). A sentence pointing at a gate that does not exist is worse
 * than no sentence: it tells the next person the radius is guarded.
 * **It is guarded by the dumps in `Face`'s `ratio` note instead**, and
 * those are a thing somebody ran.
 */
import { gaugeColor } from "@/lib/vitals";
import type { Strain } from "@/api";

/** The dial opens at −195° and sweeps 210°, leaving its gap at the
    bottom — a gauge, not a pie. 0° is three o'clock. */
export const ARC0 = -195;
export const SPAN = 210;

const D2R = Math.PI / 180;
const pol = (cx: number, cy: number, r: number, deg: number): [number, number] => [
  cx + r * Math.cos(deg * D2R),
  cy + r * Math.sin(deg * D2R),
];

/** One arc as an SVG path. Degrees, 0 at three o'clock. */
export function arc(cx: number, cy: number, r: number, a0: number, a1: number): string {
  const [x0, y0] = pol(cx, cy, r, a0);
  const [x1, y1] = pol(cx, cy, r, a1);
  const big = Math.abs(a1 - a0) > 180 ? 1 : 0;
  const sweep = a1 > a0 ? 1 : 0;
  return `M${x0.toFixed(2)} ${y0.toFixed(2)} A${r} ${r} 0 ${big} ${sweep} ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}

const ang = (frac: number) => ARC0 + Math.min(1, Math.max(0, frac)) * SPAN;

/** The size the dial's lengths were measured at — the machine card's.
    Not a size anybody has to draw at: it is the denominator that turns
    those measurements into proportions, so a dial at 32 is the same
    drawing and not a different one. */
const REF_SIZE = 88;

/** The default box: twice the drawing across, one size down. The
    machine card's, and the only one until 2026-09-07. */
const BOX_RATIO = 1.6;

/**
 * The dial in a list row: **the same drawing one size down**, not a
 * circle that happens to be little.
 *
 * ⚠️ **It was 32 — `--avatar` — until 2026-09-07.** That number came
 * from the single-diameter rule: two circles on one row are two facts,
 * and the moment one is bigger, size starts claiming one of them
 * matters more. **There is no second circle on these rows**, and the
 * reading moved *inside* this one (「最左环…右边是更新时间」), so the
 * rule it was keeping does not apply and the number it produced was
 * costing legibility.
 *
 * ⚠️ **It buys less than it looks like it buys, and it was very nearly
 * reverted for that.** The centre's type scales with the dial
 * (`fontSize = 19k`), so a bigger ring holds a *slightly* longer string,
 * not a proportionally longer one: `12.34` needed 22.6 units of a 19.2
 * inner at 32, and 27.7 of a 24.7 at 40 — **it fits at neither**, and
 * what fixed that band was the four-character rule (`RING_CHARS`), not
 * this number.
 *
 * ⚠️ **But 40 is load-bearing for a different string, and only a
 * measurement says which.** The rule 「round and keep the sign」 turns
 * 123.75 into `124$` — **four characters where it used to be three**,
 * because the sign is one of them. Measured 2026-09-07:
 *
 * | | width | inner | |
 * |---|---|---|---|
 * | `124$` at 32 | 19.8 | 19.2 | **over by 0.6** |
 * | `124$` at 40 | 24.4 | 24.7 | fits, 0.3 to spare |
 *
 * So the sign and this size are one decision: keeping the sign on a
 * three-digit figure **requires** the larger ring. A revert to 32 was
 * proposed on the reading 「`124` is 18.3, it fits」 — and that 18.3 was
 * `124` **without** the sign, at size **40**. Two numbers from two
 * states read exactly like one comparison.
 *
 * ⚠️ **The card's pair (`RING_SIZE_CARD` / `RING_STROKE_CARD`) still
 * lives in `Ring.tsx`**, which is where the machine card imports them
 * from. Two homes for one family is not the shape anybody wants; moving
 * them is a change to the machine card's imports, and that surface
 * belongs to another lane today.
 */
/* ⚠️ **The ceiling on this number is the row's *words*, not the row's
   height** (2026-09-07, and it is written here because the next person
   will arrive with the opposite intuition).

   The 額度 detail's dial can be opened up — its cell is 121 tall
   against 68 of box, and the bar column beside it has 624px of slack,
   so growth there costs nothing anyone can see. **A list row is the
   other case**: the box is already as tall as the row's usable 40, and
   every pixel this grows is taken sideways out of the column the name
   and its sub-line live in, because a 210° dial is wider than it is
   tall (ink 32.5 × 22.2, a ratio of 1.46) and `ratio` is 1 here.

   Measured that day, and the two numbers are close enough that the
   next widening should be measured rather than reasoned about:

   | box width | the sub-line |
   |---|---|
   | 49 | whole |
   | 51.2 | **truncated** (the 32 × 1.6 box, caught by `check-face`) |

   So: **「the detail's ring may grow, the row's may not」 is not a
   style preference, it is 2.2px of room.** */
export const DIAL_ROW_SIZE = 40;
/** The box a row gives the dial: **just the drawing**, no margins. See
    `Face`'s `ratio`. */
export const DIAL_ROW_RATIO = 1;
/** Thinner than the card's 4, in the proportion the sizes are: a row's
    arc at the card's stroke is a band with no hole in it. */
export const DIAL_ROW_STROKE = 2.6;

/**
 * The dial on a card that has **only this dial** — the 額度 page's.
 *
 * # The rule these three numbers come from
 *
 * **The box holds the drawing; the room around it is the layout's, not
 * the `viewBox`'s.** Measured 2026-09-07 on the 額度 card: the box was
 * 141 × 88 and the ink inside it 71 × 48 — **half the box was empty**,
 * and that emptiness was inherited rather than chosen. The default
 * `BOX_RATIO` of 1.6 is the *machine card's*: it needs a box wide
 * enough for the sentence on the dial's other face
 * (「内存 12.3G / 16.0G」). This card has no such sentence and took the
 * box anyway. **A number carried over from somewhere it had a reason,
 * to somewhere it has none, is what the white space was made of.**
 *
 * ⚠️ **And this is why 「make the ring bigger」 is not `size + 22`.**
 * Raising `size` alone scales the dead margin with the drawing: more
 * ring, and *more* white around it. The ring gets bigger **because the
 * box got tight** — first `ratio` down to 1, then the size up, and the
 * ink goes from 71 wide to about 90 in a box that shrank from 141 to
 * 110.
 *
 * The worst case the box must hold is `2 × (R + stroke/2 + 9k)` — the
 * arc reaches ±R horizontally (it sweeps through 0° and 180°) and a pip
 * can sit 9k beyond the rim. At these numbers that is ~102 in a box of
 * 110, so a mark at the widest angle still has room.
 */
export const DIAL_CARD_SIZE = 169;
/* ⚠️ **169 since 2026-09-07, and the two numbers that decided it are on
   one line.** 「点击后的用量界面的环…明明可以放的更大」 — and unlike the
   row above, here there was somewhere for it to go. What growth costs
   is the bar column beside it, so both were read in the same state:

   | | dial box | ink | bar beside it |
   |---|---|---|---|
   | before | 110 × 110 | 88.4 × 56.9 | 624 |
   | after  | 169 × 102 | 134.5 × 86.1 | 565 |

   **The box got taller by nothing and the drawing grew by half**: the
   crop took 45% of the height back, and `size` spent it. 102 still
   sits inside the 121 the cell already had, so no row moved; the bar
   gave up 59px of 624, which is the part of this that a person could
   notice, and it is the trade the ruling asked to see. */
/** Just the drawing. See above — the room around it is the layout's. */
export const DIAL_CARD_RATIO = 1;
/** The machine card's 4, in the proportion the sizes are (110/88), so
    this is the same drawing at a different size rather than a thinner
    one. */
export const DIAL_CARD_STROKE = 5;

/** One part under a dial: a card, a volume. `v` is its own reading on
    the same 0–100 scale the dial uses; `null` means the machine could
    not read that part, and **a part nobody could read gets no mark at
    all** rather than a mark sitting at zero. */
export type Pip = { word: string; v: number | null; strain?: Strain | null };

/** The same dial, small enough to sit in a heading — **the same 210°
    geometry, one size down**, not a circle that happens to be little.
    A miniature circle would say 「a ring lives here」 while the thing it
    stands for is a gauge, and the two vocabularies would sit one line
    apart on the same card.

    It is an identity mark, not a reading: the number it belongs to is
    on the big dial directly above, in view at the same time. (An
    earlier draft counted 「too small to read a value from」 against a
    glyph like this; there it was the only dial in its group, and here
    it is not.) */
export function Glyph({ fraction, strain }: { fraction: number | null; strain?: Strain | null }) {
  const S = 16;
  const R = 5.5;
  const W = 1.8;
  const cx = S / 2;
  const cy = S / 2;
  return (
    <svg data-ring-glyph viewBox={`0 0 ${S} ${S}`} width={S} height={S} aria-hidden="true" className="flex-none">
      <path
        d={arc(cx, cy, R, ARC0, ARC0 + SPAN)}
        fill="none"
        stroke="currentColor"
        strokeOpacity={0.2}
        strokeWidth={W}
        strokeLinecap="round"
      />
      {fraction !== null && (
        <path
          d={arc(cx, cy, R, ARC0, ang(fraction))}
          fill="none"
          stroke={gaugeColor(strain)}
          strokeWidth={W}
          strokeLinecap="round"
        />
      )}
    </svg>
  );
}

export function Face({
  size,
  stroke,
  fraction,
  strain,
  pips,
  unknown,
  ratio = BOX_RATIO,
  crop = false,
  children,
}: {
  size: number;
  stroke: number;
  /** `null` draws the track alone: a full sweep would read as 100% and
      an empty one as zero, and both are lies about a reading nobody
      could take. */
  fraction: number | null;
  strain?: Strain | null;
  /** The parts this reading is made of, or `[]`. **Only real parts** —
      cards and volumes. The processes under the CPU and memory dials
      are *occupants*, not parts: they do not partition the reading, and
      marking them would invent a meaning the drawing does not have. */
  pips?: Pip[];
  unknown?: boolean;
  /**
   * How wide the box is, as a multiple of `size`. The default is the
   * machine card's: **twice the drawing**, which is room the card
   * spends on purpose.
   *
   * A list row has no such room. The dial is ~25px wide at `size` 32
   * and the default box is 51 — **19px of nothing**, taken out of the
   * column the name and its line live in. Measured 2026-09-07: with
   * the default box the 额度 row's sub-line went from whole to
   * truncated at that column's 288, which is the failure `check-face`
   * was already watching for.
   *
   * **The drawing does not move when this changes**, only the window
   * onto it: the dial is centred in the box either way (`cx` is half
   * the width), and `1.6` reproduces `S * 0.8` to the bit.
   */
  ratio?: number;
  /**
   * Whether the box hugs the drawing **from above and below**.
   *
   * # What it is for
   *
   * A 210° dial is not as tall as it is wide, and until 2026-09-07 the
   * box was square-ish regardless: `viewBox` height was always `size`,
   * while the ink only ever reached from `cy - reach` to a little past
   * `cy`. Measured on the 額度 page that day: **the ink was 22.2 of a
   * 40 box and 59.6 of a 110 one — 55% both times**, and the 45% left
   * over was not a margin anybody chose. It is the same fault `ratio`
   * fixed horizontally, on the other axis, found later because a
   * horizontal gap pushes text sideways where a vertical one merely
   * looks like air.
   *
   * ⚠️ **This is why 「make the ring bigger」 is not `size + n`**:
   * raising `size` alone scales the dead band with the drawing. Bigger
   * ring, and proportionally more nothing around it.
   *
   * # What the bounds are
   *
   * `reach` is the furthest **any** mark can sit from the centre — the
   * rim plus a pip's full length plus its round cap — so the crop is
   * computed from the same expressions that draw, not from a number
   * measured off one screenshot. **A dial with no pips therefore keeps
   * a little slack**, and that is deliberate: the same component draws
   * both, and a box tight around a pip-less dial would clip the pips
   * off a dial that has them. Silently — clipping is not an error.
   *
   * The bottom is the arc's own end (`ARC0 + SPAN`, below centre by
   * `sin 15°`) plus half a stroke for the round cap; the centred text
   * sits above that.
   *
   * ⚠️ **Default `false`, and the machine card must stay that way
   * until somebody measures it.** Its box (141 × 88) is not spare
   * room — it holds the sentence on the dial's other face — and the
   * wrapper around it in `MachineCard` hard-codes `RING_SIZE_CARD` for
   * its height, so a cropped box there would shrink the drawing inside
   * a wrapper that did not shrink. That is a second change, not a
   * consequence of this one.
   */
  crop?: boolean;
  /** The number in the middle. */
  children?: React.ReactNode;
}) {
  const S = size;
  const W = S * ratio;
  const cx = W / 2;
  const cy = S * 0.6;
  // **Every length in here is a fraction of the dial it was drawn at.**
  // The prototype was measured at 88 across, and until 2026-09-07 the
  // radius and the six offsets below were the raw numbers off that
  // drawing — which meant a dial asked for at any other size drew its
  // arc outside its own box. At 32 (the size a list row has room for)
  // the viewBox is 51.2 × 32 and `R = 30` puts the whole gauge in the
  // margin: nothing visible, no error.
  //
  // So the numbers stay exactly what they were measured as and get
  // multiplied by how much of the reference size this one is.
  // **`k` is 1 at 88 to the bit**, which is the machine card's size and
  // stroke — the card's drawing is unchanged by construction, and that
  // was measured rather than assumed (dial dumps before and after are
  // the same string, with a deliberately wrong offset in between to
  // prove the comparison can fail at all).
  const k = S / REF_SIZE;
  const R = 30 * k;
  // **The box's vertical bounds, off the same numbers as the drawing.**
  // `reach`: the rim, plus a pip's whole length, plus half its round
  // cap — see `crop`. Kept beside `R` rather than at the `viewBox` so
  // that a change to either offset below breaks the reader's eye on
  // both at once.
  const reach = R + stroke / 2 + 9 * k + 0.85 * Math.max(0.7, k);
  const boxY = crop ? cy - reach : 0;
  const boxH = crop ? Math.sin((ARC0 + SPAN) * D2R) * reach + stroke / 2 + reach : S;
  const marks = (pips ?? []).filter((p): p is Pip & { v: number } => p.v !== null);
  const vals = marks.map((p) => p.v);
  const color = gaugeColor(strain);
  return (
    <svg
      data-dial-face
      viewBox={`0 ${boxY} ${W} ${boxH}`}
      width={W}
      height={boxH}
      role="presentation"
      className="text-foreground"
    >
      {/* The track, always. */}
      <path
        d={arc(cx, cy, R, ARC0, ARC0 + SPAN)}
        fill="none"
        stroke="currentColor"
        strokeOpacity={0.16}
        strokeWidth={stroke}
        strokeLinecap="round"
      />
      {fraction !== null && !unknown && (
        // **Named `data-ring-fill` still.** The shape changed from a
        // circle to a 210° dial; the fact anything outside asks of it —
        // "did a reading actually draw its gauge" — did not, and it
        // still points at the arc rather than at a box that would be
        // there either way.
        <path
          data-ring-fill
          d={arc(cx, cy, R, ARC0, ang(fraction))}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeLinecap="round"
        />
      )}
      {/* Milestones at a quarter, a half, three quarters. Without them a
          mark's angle cannot be read as a number. */}
      {[25, 50, 75].map((mk) => {
        const [x, y] = pol(cx, cy, R - stroke / 2 - 3 * k, ang(mk / 100));
        return <circle key={mk} cx={x} cy={y} r={0.9 * k} fill="currentColor" fillOpacity={0.45} />;
      })}
      {/* How far the parts are spread, as a length. Only with two or
          more to span between. */}
      {vals.length > 1 && (
        <path
          data-dial-span
          d={arc(
            cx,
            cy,
            R + stroke / 2 + 6 * k,
            ang(Math.min(...vals) / 100),
            ang(Math.max(...vals) / 100),
          )}
          fill="none"
          stroke="currentColor"
          strokeOpacity={0.55}
          strokeWidth={1}
        />
      )}
      {marks.map((p, i) => {
        const a = ang(p.v / 100);
        const [x1, y1] = pol(cx, cy, R + stroke / 2 + 2 * k, a);
        const [x2, y2] = pol(cx, cy, R + stroke / 2 + 9 * k, a);
        return (
          <line
            key={i}
            data-dial-pip={p.word}
            data-dial-pip-at={Math.round(p.v)}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            stroke={p.strain ? "var(--strain)" : "currentColor"}
            // **The one length with a floor under it**, and the floor
            // is what the mark is for: a pip is the thing being read off
            // the rim, and scaled the whole way down it thins to 0.6 at
            // a row's size — a hairline that reads as an artefact rather
            // than a reading. The track and the arc around it are wide
            // enough to survive the scale; this is not.
            strokeWidth={1.7 * Math.max(0.7, k)}
            strokeLinecap="round"
          >
            <title>{`${p.word} ${Math.round(p.v)}%`}</title>
          </line>
        );
      })}
      <text
        x={cx}
        y={cy + 6 * k}
        fontSize={19 * k}
        fontWeight={800}
        textAnchor="middle"
        fill="currentColor"
        className="tabular-nums"
      >
        {children}
      </text>
    </svg>
  );
}
