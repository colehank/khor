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

/** The type scale's steps, in px (tokens.css `--fs-*` at scale 1 — the
    34 is `--fs-gauge`, W-V13). */
const TYPE_STEPS = [11, 12, 13, 15, 17, 22, 26, 34];
/** The scale step nearest a size computed from a box. */
export function snapType(px: number): number {
  return TYPE_STEPS.reduce((best, s) => (Math.abs(s - px) < Math.abs(best - px) ? s : best), TYPE_STEPS[0]);
}

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

/** How far from the centre **any** mark can sit: the rim, plus a pip's
    whole length, plus half its round cap. The one expression the crop
    and the drawing both read — see [`Face`]'s `crop`. */
function reachOf(size: number, stroke: number): number {
  const k = size / REF_SIZE;
  return 30 * k + stroke / 2 + 9 * k + 0.85 * Math.max(0.7, k);
}

/**
 * The `ratio` whose box is exactly as wide as the drawing.
 *
 * **`crop` does this for the height; this is the same thing sideways**,
 * and it is one expression rather than a number because the two have to
 * move together — a box cut to the ink on one axis and to a guess on
 * the other is the shape that made the machine card's dials sit 90px
 * apart while the row said 18 (2026-09-08).
 *
 * ⚠️ **It reserves a pip's room whether or not this dial draws one**,
 * because `reachOf` does: a box cut to the arc would clip a pip on the
 * dial that has one, and four boxes of two different widths would put
 * the arcs at four different offsets inside them. So the *boxes* come
 * out equal and the *ink* still varies — arcs measured 60 / 75.8 / 60 /
 * 68 across on one row, and no width can make those equal without
 * cutting something off.
 */
export function hugRatio(size: number, stroke: number): number {
  return (2 * reachOf(size, stroke)) / size;
}

/** How tall a **cropped** box comes out for a dial of this size. */
function croppedHeight(size: number, stroke: number): number {
  const reach = reachOf(size, stroke);
  return reach * (1 + Math.sin((ARC0 + SPAN) * D2R)) + stroke / 2;
}

/**
 * The `size` whose cropped box comes out `h` tall — [`croppedHeight`]
 * run backwards.
 *
 * ⚠️ **Solved, not written down, and that is the whole point.** The
 * answer comes out a fraction rather than a round number (about 49 for
 * a 32-tall box at this row's stroke — **an illustration, not a
 * promise**: it moves with `reach`), and a `49` sitting in the source
 * would go on looking reasonable **for ever** after somebody changed a
 * pip's length or the stroke: the box would quietly stop
 * being the height it is supposed to share, and nothing would break —
 * the sort of stale number this repo has watched outlive its own
 * argument. Bisection because `croppedHeight` is piecewise (the pip's
 * cap has a floor under it), so an algebraic inverse would be a second
 * expression to keep in step with the first.
 */
/** The size whose drawing, pips included, is `w` wide (`hugRatio`). */
function sizeForWidth(w: number, stroke: number): number {
  let lo = 1;
  let hi = 1000;
  for (let i = 0; i < 50; i++) {
    const mid = (lo + hi) / 2;
    if (2 * reachOf(mid, stroke) < w) lo = mid;
    else hi = mid;
  }
  return (lo + hi) / 2;
}

function sizeForCroppedHeight(h: number, stroke: number): number {
  let lo = 1;
  let hi = 1000;
  for (let i = 0; i < 50; i++) {
    const mid = (lo + hi) / 2;
    if (croppedHeight(mid, stroke) < h) lo = mid;
    else hi = mid;
  }
  return (lo + hi) / 2;
}

/**
 * The height a mark gets in a list row, **mirroring `--avatar`**
 * (`apps/gui/src/styles/tokens.css:936`, `32px`).
 *
 * # Why a mirror rather than a reference
 *
 * 2026-09-07:「把这些环和机器头像…一视同仁,他们都是一种类型/实体」.
 * A row's avatar and a row's dial are one size slot, and the slot is a
 * CSS token — but `Face` is sized in numbers, so the token cannot be
 * *referenced* here without moving how every dial gets its size, which
 * is a change the machine card would ride along on.
 *
 * **So the number is copied, and a gate is what makes the copy safe**:
 * `check-face` measures the rendered height of a 額度 row's dial
 * against the rendered height of a session row's avatar and requires
 * them equal. A mirror that drifts goes red on the thing a person
 * actually cares about — **two marks on screen being the same size** —
 * rather than on two symbols pointing at one constant, which a stray
 * coefficient inside `Face` could satisfy while the screen disagreed.
 *
 * ⚠️ **If this slot is meant to change, change `--avatar`** and bring
 * this line with it. Changing only one of the two is the failure the
 * gate exists to catch, and its wording says so.
 *
 * # Why 32 is a size the dial can actually take
 *
 * **It could not, before the crop.** The ring's middle has to hold
 * `124$`, and the reading that decides it is the string against the
 * rim's inner diameter. Both numbers carry the state they were taken
 * in, on the same line, because this exact comparison was got wrong
 * twice in one day by reading two states as one:
 *
 * ```
 * 历史 · 裁框前,S 写死 40:    124$ = 24.4  vs  inner = 24.7   余 0.3
 * 历史 · 一次性跑,S 钉死 49:  124$ = 29.3  vs  inner = 30.8   余 1.5
 * 当前 · S = DIAL_ROW_SIZE:   124$ = 30.3  vs  inner = 32   余 1.7
 * ```
 *
 * ⚠️ **前两行是历史,而且它们里的 `40` 和 `49` 是死数。** 代码里那个
 * size 特意写成了 `sizeForCroppedHeight(...)`,理由是「写死的话
 * `reach` 一变它就静默变错」——**那条理由对注释里的数字一字不差地
 * 成立**,所以当前那一行只用名字。前两行留着是因为它们记的是**当时
 * 量到的事实**,不是现在的承诺;标上「历史」是它们和第三行唯一的区别,
 * 而没有这个标,三行读起来是一张表。
 *
 * ⚠️ **The middle line is not the state that shipped.** It was taken
 * in a throwaway run with `DIAL_ROW_SIZE` pinned to a flat number
 * while the ladder's `USDT` row was flipped to `$` so that a `124$`
 * existed to measure at all. What shipped is the *solved* size, a box
 * of **51 × 32** on screen.
 *
 * **The last line sat at 「未量」 for an hour rather than carrying an
 * interpolated number, and that was the right way round.** 29.3 and 32
 * side by side invite a 「余 2.6」 that would have read exactly like
 * the two measured lines — the failure this notation exists to stop,
 * committed twice in one evening before it was written down. The real
 * figure came from putting a `$`-marked row of ~124 back into the
 * fixture for one run and reading `ring middles:`; **it is 1.7, not
 * the 2.6 the arithmetic offered.**
 *
 * ⚠️ **Cropping does not widen the rim** — `R = 30k` is untouched by
 * it. What changed is that **a box 32 tall now means a larger `size`**,
 * because the box stopped carrying 35% of dead band.
 *
 * ⚠️ **There are three 32s on this page and no two of them are the
 * same quantity:**
 *
 * | where | what it is |
 * |---|---|
 * | `AVATAR_BOX` / `--avatar` | a **box height**, and the only one of the three anybody chose |
 * | the box the row's dial renders at | that height, **arrived at** by solving for it |
 * | `inner` in the table above | the rim's **inner diameter**, a length inside the drawing |
 *
 * **The third one is a coincidence and must not be read as an
 * identity.** `inner` is `2R − stroke`; the box's height is
 * `reach·(1+sin15°) + stroke/2`. Those two expressions move apart the
 * moment `stroke` changes — a thicker rim *shrinks* `inner` while it
 * *grows* the box — and they are equal today only because the size was
 * solved to make the second one 32 and the first happened to land
 * there too. **Nothing keeps them together, and nothing will say so
 * when they part.**
 *
 * That is the whole hazard in one place: all three are bare integers
 * spelled `32`, which is exactly how the first two got compared as
 * though they were one number, an evening before this was written.
 *
 * ⚠️ **The two marks share a height, not a footprint.** An avatar is
 * 32 × 32; a 210° dial at this height measures **51 across** on
 * screen, because its
 * ink is 1.46 times as wide as it is tall. Squaring it up would mean
 * squashing the arc or cutting its ends off — breaking the drawing to
 * satisfy a sentence about the drawing. The user's words are about
 * *standing* (「一种类型/实体」), and the arc draws progress where the
 * avatar draws identity; the thing they share is the slot they sit in.
 */
export const AVATAR_BOX = 32;

/** The default box: twice the drawing across, one size down. The
    machine card's, and the only one until 2026-09-07. */
const BOX_RATIO = 1.6;

/**
 * The dial in a list row: **the same drawing one size down**, not a
 * circle that happens to be little.
 *
 * ⚠️ **Everything below the next paragraph is history, and the sizes
 * in it are dead numbers.** This constant is *solved* now
 * (`sizeForCroppedHeight(AVATAR_BOX, DIAL_ROW_STROKE)`), so `32` and
 * `40` below are two earlier values it held, not two options it has —
 * and the readings taken at them describe those worlds, not this one.
 * The trail is kept because it records **why the row's dial stopped
 * being an avatar's size and then came back to one**, which is a
 * question the next person will ask; the numbers in it are evidence
 * from a date, not measurements of the current build.
 *
 * ⚠️ **It was 32 — `--avatar` — until 2026-09-07** (and it is a
 * 32-tall *box* again since, by a different route: the box is cropped
 * to the drawing now, so the same height needs a bigger `size`). That
 * number came
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
 * three-digit figure **requires** more room than a 32-`size` dial had.
 * (**That sentence is true at any pair of sizes**, which is exactly
 * why the `40` it was written about could go stale in it without
 * anything breaking — the rim is wider again today, and by a mechanism
 * this paragraph predates.) A revert to 32 was
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
/** Thinner than the card's 4, in the proportion the sizes are: a row's
    arc at the card's stroke is a band with no hole in it. */
export const DIAL_ROW_STROKE = 2.6;
/* **Declared before the size on purpose**: the size is *solved* from
   the stroke now (`sizeForCroppedHeight`), so a stroke declared after
   it would be a temporal-dead-zone crash at module load — the kind
   that takes the whole page white rather than drawing one dial wrong. */
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

   ⚠️ **Where that ceiling actually is has not been measured, and an
   earlier version of this note claimed it had.** It read 「49 whole,
   51.2 truncated, so there are 2.2px left」 — and those two numbers
   came from two different worlds: the 49 was measured that evening,
   the 51.2 was this file's own older note about a `32 × 1.6` box in
   the row layout **as it stood before this batch rearranged it**.
   Measured after the crop landed, **51 does not truncate** —
   `check-face`'s live `cut` assertion (`scrollWidth > clientWidth`, on
   the subscription's sub-line) stays green at the size shipped here.

   So the honest statement is the weaker one: **the row's dial is
   bounded by its words and not by its height, and nobody has found
   the edge.** Whoever widens it next has to measure — and the cheap
   way is to read that line's `scrollWidth - clientWidth` rather than
   to trust a remembered pixel count, this note included. */
/* **In the list's face slot, whole** (W-V11, 2026-10-03): the dial,
   its pips included, is as wide as every list's face (`AVATAR_BOX`), so
   nothing of it falls left of the list's line and the usual gap stands
   before the text column. ⚠️ 墓碑: sized to the slot's *height*, which
   made it 51 wide in a 32 slot — 9 over the line, 2.4 from the words. */
export const DIAL_ROW_SIZE = Math.min(
  sizeForCroppedHeight(AVATAR_BOX, DIAL_ROW_STROKE),
  sizeForWidth(AVATAR_BOX, DIAL_ROW_STROKE),
);
/** The box is the drawing's own width — `AVATAR_BOX`, pips included. */
export const DIAL_ROW_RATIO = hugRatio(DIAL_ROW_SIZE, DIAL_ROW_STROKE);

/**
 * **The row's dial, drawn fluid** (`Face` `fluid`): at the reference size
 * and scaled by the slot, so the stroke is the row's proportion of it —
 * `DIAL_ROW_STROKE` at `DIAL_ROW_SIZE`, carried up to `REF_SIZE`.
 */
export const DIAL_FLUID_STROKE = (DIAL_ROW_STROKE * REF_SIZE) / DIAL_ROW_SIZE;
export const DIAL_FLUID_RATIO = hugRatio(REF_SIZE, DIAL_FLUID_STROKE);
export const DIAL_FLUID_SIZE = REF_SIZE;

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
  fluid = false,
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
  /**
   * **Drawn to its container's width, the reading in HTML at caption
   * size** (2026-10-03, the owner on the phone: the quota page's dials
   * should be as large as the session page's avatars). A list row's dial
   * fills the list's face slot —
   * 40 under a finger, 32 with a mouse — however wide that slot is,
   * and its number stays the caption the scale names instead of
   * shrinking with the drawing.
   */
  fluid?: boolean;
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
  const boxY = crop ? cy - reachOf(S, stroke) : 0;
  const boxH = crop ? croppedHeight(S, stroke) : S;
  const marks = (pips ?? []).filter((p): p is Pip & { v: number } => p.v !== null);
  const vals = marks.map((p) => p.v);
  const color = gaugeColor(strain);
  const svg = (
    <svg
      data-dial-face
      viewBox={`0 ${boxY} ${W} ${boxH}`}
      width={fluid ? "100%" : W}
      height={fluid ? undefined : boxH}
      role="presentation"
      className={fluid ? "block text-foreground" : "text-foreground"}
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
      {/* **The number at a size the scale names, at 600** (the audit fix
          plan F5.2; SYS-12, SYS-13): `19 × k` drifted with the box (36.49,
          10.96) and 800 was a fourth weight. Snapped to the nearest step,
          and set a little higher in the bowl so a small dial's number
          clears its arc's ends (N26). */}
      {!fluid && (
        <text
          x={cx}
          y={cy + 5 * k}
          fontSize={snapType(19 * k)}
          fontWeight={600}
          textAnchor="middle"
          fill="currentColor"
          className="tabular-nums"
        >
          {children}
        </text>
      )}
    </svg>
  );
  if (!fluid) return svg;
  return (
    <span data-dial-fluid className="relative block w-full">
      {svg}
      {/* On the arc's centre, where the drawn number's middle was. */}
      <span
        data-dial-reading
        className="absolute left-1/2 -translate-x-1/2 -translate-y-1/2 text-caption leading-none font-title tabular-nums text-foreground"
        style={{ top: `${((cy - boxY) / boxH) * 100}%` }}
      >
        {children}
      </span>
    </span>
  );
}
