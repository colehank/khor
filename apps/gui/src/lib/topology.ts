// **The network as a graph of links** (UX-16, 我的 khor's topology map;
// the machines landing's overview draws the same one). Worked out once
// from the device table this face already polls, so the picture on a
// wide face and the column on a narrow one cannot come to say two
// different things about one link.
//
// Every edge starts at this machine, or at the relay a link goes by, or
// at a satellite's sponsor: those are the links khor knows first-hand
// (`DeviceRow.hop`, `rtt_ms`, `relay`, `sponsor`). What two *other*
// machines have between them is not known here, so it is not drawn.
import type { DeviceRow } from "@/api";

/** How a link is drawn: a road that carries bytes straight, one that
    goes by a relay, or no road to show (未连接, serve 未运行, 说不出). */
export type Road = "direct" | "relay" | "none";

/** A relay some links go by: a node of its own on the map. */
export type RelayNode = {
  /** `relay:<name>` — the node's id beside the machines' ids. */
  id: string;
  /** The relay's name as the node gives it (`DeviceRow.relay`: its host,
      and port when not the scheme's own) — what a person reads. */
  name: string;
  /** The machines whose link goes by it, in the table's order. */
  rows: DeviceRow[];
  /** Its place on the square, in percent, like a machine's `seat`. */
  left: number;
  top: number;
};

/** One drawn stretch between two nodes. */
export type Segment = {
  /** Unique on the map: the far machine's id, or the relay node's id
      for the stretch from this machine to a relay. */
  id: string;
  from: string;
  to: string;
  road: Road;
  /** The word on it — the latency and, where the way is not drawn by a
      relay node, the way. `null` draws no word. */
  word: string | null;
};

export type Topology = {
  me: DeviceRow | null;
  others: DeviceRow[];
  relays: RelayNode[];
  segments: Segment[];
};

export const roadOf = (row: Pick<DeviceRow, "hop">): Road =>
  row.hop === "direct" ? "direct" : row.hop === "relay" ? "relay" : "none";

/** A machine this one has no road to right now: grey on the map, with
    when it was last seen. A satellite speaks through its sponsor and is
    as reachable as that machine is. */
export function unreachable(row: DeviceRow, rows: readonly DeviceRow[]): boolean {
  if (row.me) return false;
  if (row.sponsor) {
    const sponsor = rows.find((r) => r.name === row.sponsor);
    return !sponsor || (!sponsor.me && roadOf(sponsor) === "none");
  }
  return roadOf(row) === "none";
}

/** How long ago an unreachable machine was last seen, in ms: the wire's
    own `heard_ms` when the resident heard it since it started, else the
    age of its last reading. `null` when it was never reached. */
export function lastSeenMs(row: DeviceRow): number | null {
  return row.heard_ms ?? row.vitals?.age_ms ?? null;
}

/** Where a relay node sits: along the mean direction of the machines it
    carries, a little past halfway to their ring (clear of this machine's
    larger face and the relay's caption), so its links fan out from it. */
function placeRelay(rows: readonly DeviceRow[], me: DeviceRow | null): { left: number; top: number } {
  const cx = me?.seat.left ?? 50;
  const cy = me?.seat.top ?? 50;
  let dx = 0;
  let dy = 0;
  let r = 0;
  for (const row of rows) {
    const x = row.seat.left - cx;
    const y = row.seat.top - cy;
    const d = Math.hypot(x, y) || 1;
    dx += x / d;
    dy += y / d;
    r += d;
  }
  r /= Math.max(1, rows.length);
  let len = Math.hypot(dx, dy);
  // Machines on opposite sides cancel out: lean towards the first one.
  if (len < 0.2 && rows[0]) {
    dx = rows[0].seat.left - cx;
    dy = rows[0].seat.top - cy;
    len = Math.hypot(dx, dy) || 1;
  }
  return { left: cx + (dx / len) * r * 0.6, top: cy + (dy / len) * r * 0.6 };
}

/**
 * The graph. `ms` words a latency (`gui.conn_relay_ms`) and `way` a road
 * (`hopWord`); they are handed in so this file reads no catalog.
 */
export function topology(
  rows: readonly DeviceRow[],
  ms: (n: number) => string,
  way: (hop: string) => string,
): Topology {
  const me = rows.find((r) => r.me) ?? null;
  const others = rows.filter((r) => !r.me);
  const byRelay = new Map<string, DeviceRow[]>();
  for (const row of others) {
    if (row.sponsor || row.hop !== "relay" || !row.relay) continue;
    byRelay.set(row.relay, [...(byRelay.get(row.relay) ?? []), row]);
  }
  const relays: RelayNode[] = [...byRelay].map(([name, carried]) => ({
    id: `relay:${name}`,
    name,
    rows: carried,
    ...placeRelay(carried, me),
  }));
  const segments: Segment[] = [];
  if (!me) return { me, others, relays, segments };
  for (const r of relays) segments.push({ id: r.id, from: me.id, to: r.id, road: "relay", word: null });
  for (const row of others) {
    const latency = row.rtt_ms !== null ? ms(row.rtt_ms) : null;
    if (row.sponsor) {
      const sponsor = rows.find((r) => r.name === row.sponsor);
      if (sponsor) segments.push({ id: row.id, from: sponsor.id, to: row.id, road: sponsor.me ? "direct" : roadOf(sponsor), word: null });
      continue;
    }
    const road = roadOf(row);
    if (road === "relay" && row.relay) {
      // The way is the relay's node; the stretch to the machine says how long.
      segments.push({ id: row.id, from: `relay:${row.relay}`, to: row.id, road, word: latency });
      continue;
    }
    const said = row.hop ? way(row.hop) : null;
    segments.push({
      id: row.id,
      from: me.id,
      to: row.id,
      road,
      word: road === "none" ? said : [said, latency].filter(Boolean).join(" · ") || null,
    });
  }
  return { me, others, relays, segments };
}
