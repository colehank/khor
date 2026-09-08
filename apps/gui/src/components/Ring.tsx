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

/* ⚠️ **Tombstone: `RING_SIZE_ROW` (32) is gone, 2026-09-07.**

   Its last caller was the 额度 page's rows, and those draw a **dial**
   now, not a ring (「环全部和机器卡对齐」). The size itself survived the
   move — 32 is still `--avatar`, still for the same reason — but it
   moved to the drawing that uses it: `Face`'s `DIAL_ROW_SIZE`, beside
   the stroke that goes with it.

   Left here rather than deleted silently because a constant with no
   caller is a size the next person reaches for by accident, and one
   that vanished with no word is a size the next person re-invents. */
/** The machine card's rings (2026-09-03, to the user's reference): a
    third again larger than a row's and drawn thinner, so the number
    inside leads and the arc reads as a gauge rather than a badge. */
/**
 * ⚠️ **A reference scale, not a diameter.** Nothing on screen is
 * `RING_SIZE_CARD` across.
 *
 * `Face` treats 88 as its reference and multiplies every measured
 * length by `k = size / 88`; the rim's radius is `30k`. So:
 *
 *     弧的直径 = size × 60 / 88      (88 → 60, 132 → 90)
 *     绘图宽   = 2 × reachOf(size, stroke)
 *
 * **The note is here because the misreading has already happened, twice
 * in one afternoon (2026-09-08)** — once in a report that used 88 to
 * work out a row's layout, once in the ruling that followed it. A
 * constant named `SIZE` is read as a size, and the drawing comes out a
 * third smaller than the arithmetic expects.
 *
 * # 88 → 132, and the two readings that decided it
 *
 * The user: 「明明可以把环绘制的更大,页面更紧凑,没有充分利用空间.」
 *
 * **What a bigger ring costs is width, and what pays for it is the box
 * no longer being 1.6× the drawing** (`hugRatio`). Measured at the
 * 1120 posture, where the row is 710 across:
 *
 *     盒宽@132×BOX_RATIO = 211.2  四个 + 三个 18 = 898.8  → 超出 188.8,换行
 *     盒宽@132 贴着墨    = 123.5  四个 + 三个 18 = 548.2  → 余 161.8
 *
 * ⚠️ **The first of those two was measured at the 1400 posture first
 * (row 990) and it said 132 fits.** It does not; the machine card is
 * drawn at 1120 in the walk and the row is 710 there. Two readings from
 * two postures compared as if they were one — the same failure this
 * repo records for a ratio taken across two dial sizes.
 *
 * And the reading in the middle, both numbers at the same size, taken
 * by putting the widest string a dial can say into the product's own
 * `<text>` and measuring it there:
 *
 *     100%@88  = 57.25  vs  inner@88  = 56.00   → 溢出 1.25
 *     100%@132 = 84.20  vs  inner@132 = 86.00   → 余 1.80
 *
 * **88 did not fit a four-character reading and 132 does** — a machine
 * at three digits of CPU is ordinary, so the enlargement closes a
 * latent overflow rather than opening one.
 */
export const RING_SIZE_CARD = 132;
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
 * **Every ring this app draws is one measure with its own centre**, and
 * that sentence is true again as of 2026-09-07.
 *
 * ⚠️ **Tombstone: `NestedRing` is gone a second time, and this time it
 * is not coming back as a ring.**
 *
 * Concentric tracks left first on 2026-09-02 with the machine rows'
 * reading stack (「这个环很抽象,看不明白。取消」). They came back on
 * 2026-09-06 (`727d030`) for the 额度 rows under a rule that answered
 * what was missing then: same quantity, stated order, centre chosen by
 * a rule. **That rule was satisfied and the shape was still wrong** —
 * 2026-09-07:「嵌套环和机器那一样,有外弧和卡点」.
 *
 * What replaced it is not a different ring but a different drawing: the
 * machine card's 210° dial (`Face`), where the arc is the fullest window
 * and **every window is a pip on the rim**. The reason concentric arcs
 * lost is the reason the dial exists at all — several readings of one
 * quantity have to be *comparable at a glance*, and「further out is
 * longer」asks a reader to learn a legend, while two marks at the same
 * angle are visibly the same number.
 *
 * The one thing the nesting could say and the dial cannot is which
 * layer is which by radius alone. The dial answers that with words in
 * the panel beside it, one line per pip — which was already true when
 * the nesting was here (`NestedRing`'s own third precondition:「颜色是
 * 谜题,词才是答案」), so nothing was lost with it.
 */
export function Ring({
  fraction,
  color,
  size = RING_SIZE,
  stroke = STROKE,
  children,
}: {
  /** `null` when there is no denominator — then only the track is drawn. */
  fraction: number | null;
  color: string;
  size?: number;
  /** The arc's width. The default goes with the one diameter above;
      nothing passes another today, and a caller that wants one owes a
      reason, because the width is part of what makes two rings on one
      screen comparable. */
  stroke?: number;
  /* ⚠️ **Tombstone: `centerClass` is gone, 2026-09-07.** It existed for
     one caller — the 额度 detail's per-window arcs, which wanted a
     bigger number in the middle than a machine's rings do. Those arcs
     are a dial now (`Face`), and the last caller went with them; what
     is left (the mark's place) has always used the default. A prop
     nobody passes is the same trap as a constant nobody reads: the next
     person reaches for it and re-opens the question of what size a
     centre is. */
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
          className="col-start-1 row-start-1 font-title tabular-nums text-fact"
        >
          {children}
        </span>
      )}
    </span>
  );
}

/** One measure of a set: its own share, its own colour — the shape
    `vitalsTracks` hands the mark's node card, which draws each as its
    own `Ring`. (A set is several rings. Several readings of *one*
    quantity is the dial's case now, not a ring's — `Face`.) */
export type Track = { id: string; fraction: number | null; color: string };
