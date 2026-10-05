// **The 我的 khor dial's maths** (UX-26, the owner 2026-10-06, agreed on
// `audit/my-khor-dial.html`): this machine in the middle, every other
// machine at the radius its latency to this one says, spread evenly
// round the dial in name order; a drag turns a seat round its ring and
// never changes its radius, because the radius is a reading.
//
// Pure functions over numbers and device rows, with no DOM and no
// catalog, so `scripts/check-dial.mjs` can run them in Node: the radius
// per ms, the label's side per angle, the clipping of a road to the two
// faces' edges, and which road the table knows between any two machines.
import type { DeviceRow } from "@/api";
import type { RoadKind } from "@/gen/bindings/RoadKind";
import { roadOf } from "@/lib/topology";

/** One road between two machines as the dial draws it. */
export type Link = { road: RoadKind; ms: number | null; relay: string | null };

/** The rim: a road this slow (or none) sits on the 不通 ring. The log
    scale runs `ln(1 + ms) / ln(1 + RIM_MS)`. */
export const RIM_MS = 300;

/** The radius a round trip sits at, between the inner radius `r0`
    (0 ms) and the rim `r1`. Log scale: 30 ms and 120 ms are the two
    hairline rings, and the spacing between 1 and 10 ms is as wide as
    between 30 and 300. */
export function radiusFor(ms: number, r0: number, r1: number): number {
  const t = Math.min(Math.log(1 + Math.max(0, ms)) / Math.log(1 + RIM_MS), 1);
  return r0 + (r1 - r0) * t;
}

/** A link's radius: no road, or a road with no reading, is the rim. */
export function radiusOf(link: Link | null, r0: number, r1: number): number {
  if (!link || link.road === "none" || link.ms === null) return r1;
  return radiusFor(link.ms, r0, r1);
}

/** The dial's frame on a box `w × h`: the middle and the two radii. The
    narrow face lifts the middle a little (the deck takes the foot) and
    keeps more room under the rim for the key. */
export type Frame = { cx: number; cy: number; r0: number; r1: number };

/** The seat sizes the frame keeps room for: a seat's face, this
    machine's, and the halo's reach past it (the three tokens). */
export type Seats = { seat: number; me: number; halo: number };

export function frame(w: number, h: number, narrow: boolean, seats: Seats = { seat: 40, me: 72, halo: 22 }): Frame {
  const u = Math.max(0, Math.min(w, h - (narrow ? 100 : 60)));
  // The rim stays inside the box with a seat's face on it — a phone is
  // narrower than the dial wants to be (its names stack over and under
  // the faces, `labelSide`); and the inner radius keeps a 0 ms seat's
  // centre past the halo, so it stands on the halo's rim at worst.
  const r1 = Math.max(0, Math.min(u * 0.46, w / 2 - seats.seat / 2 - 12));
  const r0 = Math.min(r1 * 0.6, Math.max(u * 0.16, seats.me / 2 + seats.halo));
  return { cx: w / 2, cy: h / 2 + (narrow ? -16 : 0), r0, r1 };
}

/** Where each seat sits round the dial: the remembered angle where
    there is one, else an even spread in the order given, starting at
    the top. Angles in radians, `0` to the right, clockwise positive (the
    screen's y runs down). */
export function spread(ids: readonly string[], kept: Readonly<Record<string, number>>): Map<string, number> {
  const n = ids.length;
  const out = new Map<string, number>();
  ids.forEach((id, i) => {
    const k = kept[id];
    out.set(id, typeof k === "number" && Number.isFinite(k) ? k : -Math.PI / 2 + (i + 0.5) * ((2 * Math.PI) / n));
  });
  return out;
}

/** The angle of a point from the middle — a drag's new seat. */
export function angleTo(cx: number, cy: number, x: number, y: number): number {
  return Math.atan2(y - cy, x - cx);
}

export type Side = "l" | "r" | "c";
export type Vert = "t" | "b" | "m";

/** Which side of its face a seat's name sits on: **the far side from the
    middle**, so a road passes behind a name, never through it. `r`/`l`
    past ±0.35 of the circle's horizontal, else centred; `b`/`t` the same
    on the vertical, else level — and a name dead level and centred (a
    seat exactly on the horizontal) falls back below the face. */
export function labelSide(angle: number, stacked = false): { side: Side; vert: Vert } {
  const cx = Math.cos(angle);
  const cy = Math.sin(angle);
  // Stacked (the narrow face): every name over or under its face, never
  // beside it, so a seat at the rim's side keeps its name on the screen.
  if (stacked) return { side: "c", vert: cy < 0 ? "t" : "b" };
  const side: Side = cx > 0.35 ? "r" : cx < -0.35 ? "l" : "c";
  let vert: Vert = cy > 0.35 ? "b" : cy < -0.35 ? "t" : "m";
  if (side === "c" && vert === "m") vert = "b";
  return { side, vert };
}

/** The label's anchor, relative to the seat's point: `r` away from it
    along the angle on the axes the side uses. */
export function labelOffset(angle: number, r: number, stacked = false): { x: number; y: number; side: Side; vert: Vert } {
  const { side, vert } = labelSide(angle, stacked);
  if (stacked) return { x: 0, y: vert === "t" ? -r : r, side, vert };
  const x = side === "c" ? 0 : Math.cos(angle) * r;
  const y = vert === "m" ? 0 : Math.sin(angle) * r;
  return { x, y: side === "c" && Math.abs(Math.sin(angle)) <= 0.35 ? r : y, side, vert };
}

export type Pt = { x: number; y: number };

/** The segment between two faces' edges: from `air` past the first
    face's radius to `air` short of the second's, along the line between
    their centres. Two faces on one point give a zero-length segment at
    that point rather than NaN. */
export function clip(a: Pt, b: Pt, ra: number, rb: number, air = 3): { x1: number; y1: number; x2: number; y2: number } {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const d = Math.max(Math.hypot(dx, dy), 1);
  const ux = dx / d;
  const uy = dy / d;
  return { x1: a.x + ux * (ra + air), y1: a.y + uy * (ra + air), x2: b.x - ux * (rb + air), y2: b.y - uy * (rb + air) };
}

/** This machine's road to another, first-hand (`DeviceRow.hop`,
    `rtt_ms`, `relay` — the live wire's readings). A satellite's road is
    its sponsor's. */
export function firstHand(row: DeviceRow, rows: readonly DeviceRow[]): Link {
  const via = row.sponsor ? (rows.find((r) => r.name === row.sponsor) ?? row) : row;
  return { road: via.me ? "direct" : roadOf(via), ms: via.rtt_ms ?? null, relay: via.relay ?? null };
}

/** What `from` wrote into the table about its road to `to` (NET-12,
    `DeviceRow.roads`); `null` when it wrote nothing — an older peer —
    which the face says as 未知, never as 不通. */
export function reported(from: DeviceRow, to: string): Link | null {
  const r = (from.roads ?? []).find((x) => x.to === to);
  return r ? { road: r.road, ms: r.ms, relay: r.relay } : null;
}

/** The road between any two machines, as well as this machine knows it:
    its own wire where it is one end (the same reading from both ends),
    else what either of the two reported, else unknown. */
export function linkOf(a: DeviceRow, b: DeviceRow, rows: readonly DeviceRow[]): Link | null {
  if (a.id === b.id) return null;
  if (a.me) return firstHand(b, rows);
  if (b.me) return firstHand(a, rows);
  return reported(a, b.id) ?? reported(b, a.id);
}

/** The matrix cell `a` reports about `b`: first-hand on this machine's
    row, the book on every other, `null` for an older peer's silence. */
export function cellOf(a: DeviceRow, b: DeviceRow, rows: readonly DeviceRow[]): Link | null {
  if (a.id === b.id) return null;
  if (a.me) return firstHand(b, rows);
  return reported(a, b.id);
}
