// The 我的 khor dial's maths (UX-26, the owner 2026-10-06, agreed on
// `audit/my-khor-dial.html`): `src/lib/dial.ts` and `src/lib/latency.ts`
// run in Node, no browser. What the brief puts in the gate:
//
//   1. **the radius per ms** — 0 ms sits on the inner radius, the rim is
//      no road or ≥ 300 ms, 30 and 120 ms are the two rings, and the
//      scale is a log (1 → 10 ms is as wide as 30 → 300), monotone;
//   2. **the label's side per angle** — a name sits on the far side from
//      the middle: right of a seat on the right, under one at the
//      bottom, left on the left, over one at the top; a seat exactly
//      level and centred falls back below its face;
//   3. **the clipping** — a road runs from 3 past the first face's edge to
//      3 short of the second's, along the line between their centres,
//      and two faces on one point give a point, not NaN;
//   4. **the bands** — under 30 fast, under 120 mid, from 120 slow, no
//      reading none; the four inks are four tokens;
//   5. **the spread** — an even spread in the order given, a remembered
//      angle kept, a nonsense one ignored;
//   6. **what is known between two machines** — this machine's wire from
//      either end, a peer's book for the pairs it is not on, and 未知
//      (null) where an older peer wrote nothing — never 不通.
//
// Run: `npm run dial`; also in `npm test`.
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const alias = {
  name: "alias",
  setup(b) {
    b.onResolve({ filter: /^@\// }, (a) => b.resolve(`./src/${a.path.slice(2)}`, { resolveDir: gui, kind: a.kind }));
  },
};
const out = await build({
  stdin: {
    contents: `import * as dial from "@/lib/dial"; import * as latency from "@/lib/latency"; globalThis.lab = { dial, latency };`,
    resolveDir: gui,
    loader: "ts",
  },
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "error",
  plugins: [alias],
});
await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
const { dial, latency } = globalThis.lab;

let bad = 0;
let read = 0;
const fail = (m) => {
  bad++;
  console.error(`dial gate: ${m}`);
};
const near = (a, b, eps = 1e-6) => Math.abs(a - b) <= eps;
const check = (ok, m) => {
  read++;
  if (!ok) fail(m);
};

// 1. the radius per ms
{
  const r0 = 100;
  const r1 = 400;
  check(near(dial.radiusFor(0, r0, r1), r0), "0 ms is not on the inner radius");
  check(near(dial.radiusFor(300, r0, r1), r1), "300 ms is not on the rim");
  check(near(dial.radiusFor(5000, r0, r1), r1), "a road slower than the rim is not held at the rim");
  check(near(dial.radiusFor(-3, r0, r1), r0), "a negative reading is not held at the inner radius");
  const r30 = dial.radiusFor(30, r0, r1);
  const r120 = dial.radiusFor(120, r0, r1);
  check(r0 < r30 && r30 < r120 && r120 < r1, `the rings are out of order: ${[r0, r30, r120, r1].join(" < ")}`);
  // Log: 10 → 100 as wide as 30 → 300 (ln 101 − ln 11 against ln 301 −
  // ln 31: the `1 +` only tells under 10 ms), and 0 → 10 wider than
  // 100 → 110 by far.
  const w1 = dial.radiusFor(100, r0, r1) - dial.radiusFor(10, r0, r1);
  const w2 = dial.radiusFor(300, r0, r1) - dial.radiusFor(30, r0, r1);
  check(Math.abs(w1 - w2) / w2 < 0.05, `not a log scale: 10→100 spans ${w1.toFixed(1)}, 30→300 spans ${w2.toFixed(1)}`);
  const lo = dial.radiusFor(10, r0, r1) - dial.radiusFor(0, r0, r1);
  const hi = dial.radiusFor(110, r0, r1) - dial.radiusFor(100, r0, r1);
  check(lo > 8 * hi, `not a log scale: 0→10 spans ${lo.toFixed(1)}, 100→110 spans ${hi.toFixed(1)}`);
  let last = r0;
  for (let ms = 1; ms <= 300; ms += 1) {
    const r = dial.radiusFor(ms, r0, r1);
    if (r < last) {
      fail(`the radius steps back at ${ms} ms`);
      break;
    }
    last = r;
  }
  read++;
  check(near(dial.radiusOf(null, r0, r1), r1), "no link is not the rim");
  check(near(dial.radiusOf({ road: "none", ms: 12, relay: null }, r0, r1), r1), "a `none` road with a stale ms is not the rim");
  check(near(dial.radiusOf({ road: "direct", ms: null, relay: null }, r0, r1), r1), "a road with no reading is not the rim");
  check(near(dial.radiusOf({ road: "relay", ms: 30, relay: "r" }, r0, r1), r30), "a 30 ms road is not on the 30 ms ring");
  console.log(`dial · radius: r0 ${r0} · 30 ms ${r30.toFixed(1)} · 120 ms ${r120.toFixed(1)} · rim ${r1}`);
}

// 2. the label's side per angle
{
  const cases = [
    [0, "r", "b"], // dead right: level would be centred on the road, so it drops below
    [Math.PI / 2, "c", "b"],
    [Math.PI, "l", "b"],
    [-Math.PI / 2, "c", "t"],
    [Math.PI / 4, "r", "b"],
    [-Math.PI / 4, "r", "t"],
    [(3 * Math.PI) / 4, "l", "b"],
    [(-3 * Math.PI) / 4, "l", "t"],
  ];
  for (const [ang, side, vert] of cases) {
    const got = dial.labelSide(ang);
    const want = ang === 0 || ang === Math.PI ? { side, vert: "m" } : { side, vert };
    check(got.side === want.side && got.vert === want.vert, `label at ${ang.toFixed(2)} rad is ${got.side}/${got.vert}, not ${want.side}/${want.vert}`);
  }
  // The far side from the middle: the offset points away from it.
  for (let i = 0; i < 16; i += 1) {
    const ang = -Math.PI + (i / 16) * 2 * Math.PI;
    const o = dial.labelOffset(ang, 28);
    const away = o.x * Math.cos(ang) + o.y * Math.sin(ang);
    check(away >= -1e-9, `label at ${ang.toFixed(2)} rad leans towards the middle (${o.x.toFixed(1)}, ${o.y.toFixed(1)})`);
  }
  const level = dial.labelOffset(0, 28);
  check(level.side === "r" && near(level.x, 28) && near(level.y, 0), `a seat dead right puts its name at (${level.x}, ${level.y}), not beside it`);
  const top = dial.labelOffset(-Math.PI / 2, 28);
  check(top.side === "c" && top.vert === "t" && near(top.y, -28), `a seat at the top puts its name at (${top.x}, ${top.y}), not over it`);
  const me = dial.labelOffset(Math.PI / 2, 44);
  check(me.vert === "b" && near(me.y, 44), "the middle's name is not under its face");
  // Stacked (the narrow face): over or under, never beside.
  for (let i = 0; i < 16; i += 1) {
    const ang = -Math.PI + (i / 16) * 2 * Math.PI;
    const o = dial.labelOffset(ang, 28, true);
    const up = Math.sin(ang) < 0;
    check(o.side === "c" && near(o.x, 0) && o.vert === (up ? "t" : "b") && near(o.y, up ? -28 : 28), `stacked label at ${ang.toFixed(2)} rad is ${o.side}/${o.vert} at (${o.x}, ${o.y})`);
  }
  console.log("dial · label sides: 8 angles, 16 directions away from the middle, 16 stacked");
}

// 2b. the frame: the rim inside the box with a seat beside it, the inner
//     radius clear of the halo
{
  const seats = { seat: 36, me: 60, halo: 22 };
  const phone = dial.frame(390, 760, true, seats);
  check(phone.cx + phone.r1 + seats.seat / 2 <= 390 - 8, `a phone's rim reaches ${phone.cx + phone.r1 + seats.seat / 2}, past the box's edge`);
  check(phone.r0 >= seats.me / 2 + seats.halo, `a phone's inner radius ${phone.r0} puts a 0 ms seat inside the halo`);
  check(phone.r0 < phone.r1, "a phone's inner radius is not inside its rim");
  const wide = dial.frame(1400, 848, false, { seat: 40, me: 72, halo: 22 });
  check(near(wide.r1, 788 * 0.46) && near(wide.r0, 788 * 0.16) && near(wide.cx, 700) && near(wide.cy, 424), `a wide frame reads ${JSON.stringify(wide)}`);
  const tiny = dial.frame(0, 0, true, seats);
  check([tiny.r0, tiny.r1].every((v) => Number.isFinite(v) && v >= 0), "an empty box gives a nonsense frame");
  console.log(`dial · frame: phone r0 ${phone.r0.toFixed(0)} r1 ${phone.r1.toFixed(0)} · wide r0 ${wide.r0.toFixed(0)} r1 ${wide.r1.toFixed(0)}`);
}

// 3. the clipping
{
  const c = dial.clip({ x: 0, y: 0 }, { x: 100, y: 0 }, 36, 20);
  check(near(c.x1, 39) && near(c.y1, 0) && near(c.x2, 77) && near(c.y2, 0), `a level road clips to (${c.x1}, ${c.x2}), not (39, 77)`);
  const d = dial.clip({ x: 0, y: 0 }, { x: 30, y: 40 }, 10, 10, 0);
  check(near(d.x1, 6) && near(d.y1, 8) && near(d.x2, 24) && near(d.y2, 32), `a 3-4-5 road clips to (${d.x1},${d.y1})-(${d.x2},${d.y2})`);
  const z = dial.clip({ x: 5, y: 5 }, { x: 5, y: 5 }, 10, 10);
  check([z.x1, z.y1, z.x2, z.y2].every(Number.isFinite), "two faces on one point give NaN");
  console.log("dial · clip: 3 past the first edge, 3 short of the second");
}

// 4. the bands
{
  const want = [
    [0, "fast"],
    [29, "fast"],
    [29.9, "fast"],
    [30, "mid"],
    [119, "mid"],
    [120, "slow"],
    [900, "slow"],
    [null, "none"],
    [undefined, "none"],
    [Number.NaN, "none"],
  ];
  for (const [ms, b] of want) check(latency.band(ms) === b, `band(${ms}) is ${latency.band(ms)}, not ${b}`);
  check(latency.FAST_UNDER_MS === 30 && latency.MID_UNDER_MS === 120, "the band edges moved");
  const inks = Object.values(latency.BAND_INK);
  check(inks.length === 4 && new Set(inks).size === 4 && inks.every((i) => /^var\(--[a-z-]+\)$/.test(i)), `the inks are not four tokens: ${inks.join(", ")}`);
  console.log(`dial · bands: ${want.map(([ms, b]) => `${ms}→${b}`).join(" ")}`);
}

// 5. the spread
{
  const ids = ["a", "b", "c", "d"];
  const s = dial.spread(ids, {});
  const angs = ids.map((id) => s.get(id));
  check(near(angs[0], -Math.PI / 2 + Math.PI / 4), `the first seat is at ${angs[0]}, not a half-step past the top`);
  for (let i = 1; i < angs.length; i += 1) check(near(angs[i] - angs[i - 1], Math.PI / 2), `seats ${i - 1} and ${i} are ${angs[i] - angs[i - 1]} apart, not a quarter turn`);
  const kept = dial.spread(ids, { b: 1.25, c: Number.NaN });
  check(near(kept.get("b"), 1.25), "a remembered angle is not kept");
  check(near(kept.get("c"), angs[2]), "a nonsense remembered angle is not ignored");
  check(near(dial.angleTo(50, 50, 50, 10), -Math.PI / 2), "a point straight up is not −π/2");
  console.log("dial · spread: even in order, remembered angles kept");
}

// 6. what is known between two machines
{
  const row = (id, extra) => ({ id, name: id, me: false, sponsor: null, hop: "direct", rtt_ms: 4, relay: null, roads: [], ...extra });
  const me = row("me", { me: true, hop: null, rtt_ms: null });
  const a = row("a", { hop: "relay", rtt_ms: 41, relay: "pk.relay", roads: [{ to: "b", road: "direct", relay: null, ms: 1 }] });
  const b = row("b", { hop: "direct", rtt_ms: 9, roads: [{ to: "a", road: "direct", relay: null, ms: 2 }, { to: "me", road: "direct", relay: null, ms: 9 }] });
  const old = row("old", { hop: "quiet", rtt_ms: null, roads: [] });
  const sat = row("sat", { sponsor: "b", hop: null, rtt_ms: null });
  const rows = [me, a, b, old, sat];
  const fh = dial.firstHand(a, rows);
  check(fh.road === "relay" && fh.ms === 41 && fh.relay === "pk.relay", `this machine's road to a reads ${JSON.stringify(fh)}`);
  const fromMe = dial.linkOf(me, a, rows);
  const toMe = dial.linkOf(a, me, rows);
  check(JSON.stringify(fromMe) === JSON.stringify(toMe) && fromMe.ms === 41, "this machine's wire does not read the same from both ends");
  const ab = dial.linkOf(a, b, rows);
  check(ab && ab.road === "direct" && ab.ms === 1, `a→b reads ${JSON.stringify(ab)}, not a's own book`);
  const cell = dial.cellOf(b, a, rows);
  check(cell && cell.ms === 2, `the cell b reports about a reads ${JSON.stringify(cell)}, not b's book`);
  check(dial.cellOf(old, a, rows) === null, "an older peer's silence is not 未知 (null)");
  check(dial.linkOf(old, a, rows) === null, "a pair nobody reported is drawn");
  check(dial.cellOf(me, old, rows).road === "none", "this machine's no-road to an older peer is not 不通");
  check(dial.cellOf(a, a, rows) === null && dial.linkOf(a, a, rows) === null, "a machine has a road to itself");
  const s = dial.firstHand(sat, rows);
  check(s.road === "direct" && s.ms === 9, `a satellite's road is ${JSON.stringify(s)}, not its sponsor's`);
  console.log("dial · links: the wire from both ends, the book for the rest, 未知 for an older peer");
}

if (read < 40) fail(`probe dead: only ${read} checks ran`);
if (bad) {
  console.error(`dial gate: ${bad} failure(s)`);
  process.exit(1);
}
console.log(`dial gate: clean (${read} checks)`);
