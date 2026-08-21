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

/** Stroke for a single-track ring. Fixed, because the diameter is. */
const STROKE = 5.5;

/**
 * The smallest hole a nested ring may close to.
 *
 * A stack that fills in stops reading as a stack — the hole is what says
 * "these are several, drawn around one another" rather than "this is a
 * thick ring".
 */
const HOLE = 6;

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
 * **A number in the middle means one track.** That is half of this
 * file's vocabulary; `NestedRing` is the other half, and the two are
 * meant to be told apart without reading either.
 */
export function Ring({
  fraction,
  color,
  size = RING_SIZE,
  children,
}: {
  /** `null` when there is no denominator — then only the track is drawn. */
  fraction: number | null;
  color: string;
  size?: number;
  children?: React.ReactNode;
}) {
  const r = (size - STROKE) / 2;
  return (
    <span
      data-ring
      className="relative inline-grid place-items-center"
      style={{ width: size, height: size }}
    >
      <svg width={size} height={size} aria-hidden="true" className="col-start-1 row-start-1">
        <Arc cx={size / 2} r={r} stroke={STROKE} color={color} fraction={fraction} />
      </svg>
      {/* The centre sits on top of the same grid cell rather than inside
          the SVG: it is text the app styles like all its other text, and
          `<text>` would put it outside everything the type scale reaches. */}
      {children != null && (
        <span data-ring-center className="col-start-1 row-start-1 text-fact tabular-nums">
          {children}
        </span>
      )}
    </span>
  );
}

/** One ring of a stack: its own share, its own colour. */
export type Track = { id: string; fraction: number | null; color: string };

/**
 * **A stack of measures, drawn as one hollow set of concentric rings**,
 * outermost first.
 *
 * The other half of the vocabulary: *a number in the middle is one
 * measure; a hollow set of rings is several*. They are distinguishable
 * at a glance, which is the whole point — a reader should not have to
 * work out which kind of thing they are looking at before reading it.
 *
 * **The middle stays empty.** mandala tried a name in there and got an
 * ellipsis: after three strokes the hole is only a few dozen points
 * across, and anything a user named themselves does not fit. Empty, it
 * goes back to being a mark rather than a squeezed piece of typography.
 *
 * **The diameter does not move; the stroke does.** Rings on one screen
 * have to be one size — a few pixels of difference says nothing and only
 * reads as misalignment — so more tracks means thinner ones, down to
 * whatever still leaves the hole open.
 */
export function NestedRing({
  tracks,
  size = RING_SIZE,
}: {
  tracks: Track[];
  size?: number;
}) {
  const n = Math.max(1, tracks.length);
  // Gaps are a fixed share of the stroke, so there is only one unknown:
  //   size/2 - stroke * (1 + 1.57(n-1)) ≥ HOLE
  const stroke = Math.min(3.5, (size / 2 - HOLE) / (1 + 1.57 * (n - 1)));
  const gap = stroke * 0.57;
  const outer = (size - stroke) / 2;

  return (
    <span data-ring data-ring-tracks={n} className="inline-block" style={{ width: size, height: size }}>
      <svg width={size} height={size} aria-hidden="true">
        {tracks.map((t, i) => (
          <Arc
            key={t.id}
            cx={size / 2}
            r={outer - i * (stroke + gap)}
            stroke={stroke}
            color={t.color}
            fraction={t.fraction}
          />
        ))}
      </svg>
    </span>
  );
}
