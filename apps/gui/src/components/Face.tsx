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
 * come across verbatim from the prototype (`ring-rim-pips.html`).
 * **The radius especially must not be adjusted**: arc length follows
 * from it, and the check that this matches the drawing it came from
 * compares lengths.
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

/** One part under a dial: a card, a volume. `v` is its own reading on
    the same 0–100 scale the dial uses; `null` means the machine could
    not read that part, and **a part nobody could read gets no mark at
    all** rather than a mark sitting at zero. */
export type Pip = { word: string; v: number | null; strain?: Strain | null };

export function Face({
  size,
  stroke,
  fraction,
  strain,
  pips,
  unknown,
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
  /** The number in the middle. */
  children?: React.ReactNode;
}) {
  const S = size;
  const cx = S * 0.8;
  const cy = S * 0.6;
  const R = 30;
  const marks = (pips ?? []).filter((p): p is Pip & { v: number } => p.v !== null);
  const vals = marks.map((p) => p.v);
  const color = gaugeColor(strain);
  return (
    <svg
      data-dial-face
      viewBox={`0 0 ${S * 1.6} ${S}`}
      width={S * 1.6}
      height={S}
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
        const [x, y] = pol(cx, cy, R - stroke / 2 - 3, ang(mk / 100));
        return <circle key={mk} cx={x} cy={y} r={0.9} fill="currentColor" fillOpacity={0.45} />;
      })}
      {/* How far the parts are spread, as a length. Only with two or
          more to span between. */}
      {vals.length > 1 && (
        <path
          data-dial-span
          d={arc(
            cx,
            cy,
            R + stroke / 2 + 6,
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
        const [x1, y1] = pol(cx, cy, R + stroke / 2 + 2, a);
        const [x2, y2] = pol(cx, cy, R + stroke / 2 + 9, a);
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
            strokeWidth={1.7}
            strokeLinecap="round"
          >
            <title>{`${p.word} ${Math.round(p.v)}%`}</title>
          </line>
        );
      })}
      <text
        x={cx}
        y={cy + 6}
        fontSize={19}
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
