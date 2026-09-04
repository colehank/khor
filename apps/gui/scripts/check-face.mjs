// The type gate, rendered half: what the letterforms actually weigh on
// the glass. The static half (`check-type.mjs`) proves nobody wrote a
// stray literal; this half proves the tokens land — a stylesheet layer,
// a purge, or a font stack missing a weight would leave every rule in
// place and every screen at 400, and no static check can see that.
//
// Claims:
//   1. the token classes resolve on a live page: a probe element wearing
//      font-title computes 600, font-label 500, tabular-nums carries
//      font-variant-numeric — the CSS chain end to end;
//   2. a real face uses them: with session rows and devices routed in,
//      the list row's title weighs title, a section label weighs label,
//      and every element that opted into tabular-nums got it — the
//      weight distribution is {400,500,600}, not the flat {400} this
//      gate was born from;
//   3. **every pressable thing on every surface the rail reaches** is at
//      least `--touch` on both sides, in both postures **and with a
//      finger on the glass** — or is waived by name, with why the floor
//      is the wrong criterion for that one and what the waiver does not
//      buy;
//   3b. and the breadcrumb spends that width only where the finger is —
//      with a mouse no step is padded more than its neighbour for
//      having a shorter name;
//   3c. and a directory listing's rows agree with each other on height
//      and on both gutters, whichever branch drew them;
//   4. and every press handler is on something that **says** it is a
//      target — the claim section 3's scope rests on, since a control
//      the DOM does not admit to is one section 3 cannot find either;
//   5. a dense pane has a reading order;
//   5b. no machine surface draws the subscription's arcs any more
//      (2026-09-02:「机器页面不需要有token usage的环」) — their one
//      home is the 额度 landing's foot, where section 8 proves the
//      finder the absences lean on;
//   6. zero page errors while measuring — a half-drawn page answers
//      probes too (the browser-check false-green family).
//
// The routed rows are shaped by hand from the bindings. If the shape
// drifts, the list draws nothing, the title probe finds no element, and
// this gate goes red saying so — loud drift, the survivable kind.
//
// Run: `npm run face` (boots its own vite, needs system Chrome).
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.FACE_PORT ?? 1451);
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
// A bounded poll where a fixed sleep used to be. Every「probe dead」in
// this file is the claim *this surface never drew*, and a fixed wait
// turns「slower than 500ms this once」into exactly that claim — a red
// about the machine wearing the words of a red about the product, which
// is the most expensive kind to receive: it sends somebody to look at a
// screen that is fine. Measured 2026-08-30 on an idle machine (load
// 1.75): two consecutive runs failed at two *different* stops
// (`wide · settings never drew`, `the peer's node card never opened`)
// and each was green on the next run. Waiting for the thing itself is
// both truthful and, in the ordinary case, faster than the sleep it
// replaces.
const until = async (ms, ok) => {
  const end = Date.now() + ms;
  for (;;) {
    if (await ok()) return true;
    if (Date.now() > end) return false;
    await wait(100);
  }
};
const fail = (m) => {
  console.error(m);
  process.exitCode = 1;
};

let vite = null;
let browser = null;
try {
  vite = spawn("node_modules/.bin/vite", ["--port", String(PORT), "--strictPort"], {
    cwd: gui,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let gone = null;
  vite.on("exit", (code) => (gone = `vite exited with ${code} — is ${PORT} already taken?`));
  let ready = false;
  vite.stdout.on("data", (d) => (ready ||= String(d).includes(`:${PORT}`)));
  for (let i = 0; i < 100 && !ready; i++) {
    if (gone) throw new Error(gone);
    await wait(100);
  }
  if (!ready) throw new Error(`vite never came up on ${PORT}`);

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const thrown = [];
  page.on("pageerror", (e) => thrown.push(String(e)));

  // Session rows and devices, so a title and a reading exist to measure.
  // Hand-shaped from the bindings (see the file head for why drift here
  // fails loud instead of quietly).
  //
  // **Two of each, and the second of each earns its place in section 3**
  // rather than being a spare: a 待批 row is the only thing that draws
  // the mesh screen's waiting rows and the mark's own call, and a
  // machine that has answered is the only thing that draws the card's
  // rings — controls that a one-row, one-machine fixture leaves off the
  // page entirely, which is a scope hole of exactly the kind section 3
  // was written about.
  const row = (n, word) => ({
    id: `tui/face-gate-${n}`,
    kind: "tui",
    title: n === 0 ? "face-gate-title" : `face-gate-title-${n}`,
    word,
    at_ms: Date.now(),
    unread: 0,
    source: null,
    pinned: false,
    stale: false,
    category: "shell",
    group: "",
    face: null,
    hosted: false,
    attachable: false,
    last: null,
    via: null,
    via_face: null,
    home: "gate",
  });
  const rows = [row(0, "idle"), row(1, "blocked")];
  const fill = (used, total) => ({ used, total });
  const devs = [
    {
      id: "e".repeat(64),
      name: "gate",
      me: true,
      face: null,
      pinned: false,
      vitals: null,
      hop: null,
      sponsor: null,
      // **`{ left, top }`, and they are percentages** — `khor_core::map::Seat`
      // (`CENTRE` is 50/50, the ring sits at 34 out; `ring(1)` puts the
      // one other machine at six o'clock).
      //
      // This fixture used to say `{ x, y }`, and what that did is worth
      // writing down because it is quieter than it sounds: React drops a
      // style value that is `undefined`, so the seats came out with **no
      // `left` and no `top` at all** — not a broken `"undefined%"` a
      // reader could grep for. Measured: with `{x,y}` not one element
      // under `[data-mark-place]` carried an inline `left`; with
      // `{left,top}` there are three, at 50%/50%, 50%/84% and the edge's
      // midpoint 50%/67%. No throw, no red, and **no number this gate
      // reads changed either way** (it measures sizes, not positions) —
      // which is exactly why it survived: the only witness was a picture
      // nobody was looking at.
      seat: { left: 50, top: 50 },
    },
    {
      id: "f".repeat(64),
      name: "peer",
      me: false,
      face: null,
      pinned: false,
      vitals: {
        vitals: {
          cpu_pct: 23,
          cores: 10,
          mem: fill(13_000_000_000, 16_000_000_000),
          disk: fill(400_000_000_000, 500_000_000_000),
          // **The aggregate says four cards; the per-card list has
          // two.** A real shape — an older node reports the count and
          // no list — and the one arrangement that can tell 「count what
          // the node claims」 apart from 「count what is drawn」. With
          // both numbers equal, an implementation reading the wrong one
          // passes, which is a control group whose arms were built
          // identical.
          gpu: { util_pct: 67, cards: 4, mem: fill(2_000_000_000, 8_000_000_000) },
          version: "0.1.0",
          // The surveyed shape (2026-09-02): cards one by one — one
          // named, one nameless (the word fallback) — and volumes one
          // by one, the second wearing a minted strain. Real data
          // always carries these fields (serde default fills them), so
          // a fixture without them is the loud-drift red this file's
          // head promises, not a case to support.
          gpus: [
            { name: "RTX A5000", util_pct: 90, mem: fill(1_000_000_000, 4_000_000_000), temp_c: 61 },
            { name: "", util_pct: 4, mem: fill(3_000_000_000, 4_000_000_000), temp_c: null },
          ],
          volumes: [
            { mount: "/", fill: fill(400_000_000_000, 500_000_000_000) },
            { mount: "/data", fill: fill(46_000_000_000, 50_000_000_000) },
          ],
          uptime_s: 86_400 * 125 + 3_600,
        },
        age_ms: 42_000,
        mem_strain: null,
        disk_strain: null,
        vram_strain: null,
        volume_strains: [null, "tight"],
      },
      hop: "direct",
      sponsor: null,
      // Where `map::ring(1)` actually puts the one other machine: half a
      // step past twelve with a single seat lands it at six o'clock,
      // 34 below the middle. Taken from the formula rather than made up,
      // so the picture this gate measures is the picture the product
      // draws.
      seat: { left: 50, top: 84 },
    },
    {
      id: "d".repeat(64),
      name: "elder",
      me: false,
      face: null,
      pinned: false,
      // Yesterday's reporter: the aggregate readings and nothing per
      // card or per volume — what an un-upgraded khor's frame decodes
      // to on this end (serde defaults, `khor_core::Vitals`). The card
      // must fall back to the one-ring shapes, and section 9 holds it.
      vitals: {
        vitals: {
          cpu_pct: 5,
          cores: 4,
          mem: fill(1_000_000_000, 8_000_000_000),
          disk: fill(10_000_000_000, 100_000_000_000),
          gpu: { util_pct: 30, cards: 2, mem: fill(2_000_000_000, 8_000_000_000) },
          version: "0.1.0",
          gpus: [],
          volumes: [],
          uptime_s: 0,
        },
        age_ms: 60_000,
        mem_strain: null,
        disk_strain: null,
        vram_strain: null,
        volume_strains: [],
      },
      hop: "relay",
      sponsor: null,
      // ring(2)'s other seat: opposite the peer, up top.
      seat: { left: 50, top: 16 },
    },
  ];
  await page.route("**/sessions", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(rows) }),
  );
  await page.route("**/devices", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(devs) }),
  );
  // What a machine is busy with (`MachineCard`'s `Busy`): the peer
  // answers a top of three and a day with holes in it; elder refuses,
  // the way a khor that predates the op does, and the refusal is a
  // sentence. The holes are placed by a rule (every seventh bucket
  // from the fourth) so their count is derived here, not typed twice.
  const dayBuckets = Array.from({ length: 288 }, (_, i) => (i % 7 === 3 ? null : 5 + (i % 60)));
  const dayGaps = dayBuckets.filter((b) => b === null).length;
  // The other three readings on the same buckets, with the same holes:
  // every row must break where the CPU row does, and the rows share one
  // time axis. **Disk is deliberately all holes** — that is a series
  // this fixture's machine records nothing for, and the chart must draw
  // no row at all rather than a named row of absence (`DayChart`).
  // **One bucket at 95** so 内存 actually wears the strain colour
  // somewhere: without it the strain assertion has only its negative
  // half, and a chart that never painted strain at all would pass.
  const memBuckets = dayBuckets.map((b, i) => (b === null ? null : i === 7 ? 95 : 40 + (i % 20)));
  const gpuBuckets = dayBuckets.map((b, i) => (b === null ? null : (i * 3) % 100));
  const diskBuckets = dayBuckets.map(() => null);
  // The cards' memory, on the same holes as the rest. **One bucket is
  // exactly 90.0** — the strain boundary — so a comparison written `>`
  // instead of `>=` reds on that bucket alone, the way
  // `strain_steps_at_ninety_and_ninety_five_exactly` pins its own.
  const vramBuckets = dayBuckets.map((b, i) => (b === null ? null : i === 11 ? 90 : 20 + (i % 55)));
  // How many rows the chart owes: one per series with a reading in it.
  // **Computed, not typed.** It used to be a literal `3`, under a
  // comment describing a rule that can be evaluated — so adding a
  // series here would have left it pointing at the old world while
  // staying green (the ledger's 「过期的数为什么能活那么久」).
  const daySeries = [dayBuckets, gpuBuckets, memBuckets, diskBuckets, vramBuckets];
  const dayRows = daySeries.filter((x) => x.some((v) => v !== null)).length;
  const load = {
    top: [
      // **One row without an age on purpose.** `run_secs` is optional
      // on the wire, and a machine that will not say how old a process
      // is must draw an empty cell rather than 0 秒 — which would name
      // it the newest process on the box. The kernel row carries no
      // command line either, for the same reason: absent is not empty.
      // prettier-ignore
      { pid: 101, name: "khor", user: "gate", cpu_pct: 240.5, mem: 512_000_000, gpu_pct: 44, gpu_mem: 8_000_000_000, run_secs: 90_061, arg: "serve" },
      // prettier-ignore
      { pid: 202, name: "node", user: "gate", cpu_pct: 12, mem: 128_000_000, gpu_pct: 5, gpu_mem: 2_000_000_000, run_secs: 45, arg: "scripts/check-face.mjs --with-a-very-long-tail-so-this-cell-cannot-possibly-fit-in-its-column-and-must-say-so" },
      // prettier-ignore
      { pid: 303, name: "kernel_task", user: "root", cpu_pct: 3.2, mem: 2_000_000_000, gpu_pct: null, gpu_mem: null, run_secs: null, arg: null },
      // **Twenty rows lifted from a real machine's distribution**
      // (turing, 2026-09-04, `dist-turing.json`): the median process
      // uses a fifth of one percent of a core while the busiest uses
      // forty-three. Three hand-made rows cannot say whether an axis
      // spreads a table out, and the crowding check simply declines to
      // run below eight points — a fixture that makes an assertion
      // unevaluable is a fixture that makes it useless.
      // prettier-ignore
      { pid: 900000, name: "python3", user: "gate", cpu_pct: 42.9910, mem: 8708096, gpu_pct: null, gpu_mem: null, run_secs: 11, arg: "tail/0.py" },
      // prettier-ignore
      { pid: 900001, name: "khor-2077930-6", user: "gate", cpu_pct: 41.9717, mem: 155160576, gpu_pct: null, gpu_mem: null, run_secs: 148, arg: "tail/1.py" },
      // prettier-ignore
      { pid: 900002, name: "netdata", user: "gate", cpu_pct: 32.0176, mem: 4024270848, gpu_pct: null, gpu_mem: null, run_secs: 285, arg: "tail/2.py" },
      // prettier-ignore
      { pid: 900003, name: "apps.plugin", user: "gate", cpu_pct: 13.5588, mem: 109076480, gpu_pct: null, gpu_mem: null, run_secs: 422, arg: "tail/3.py" },
      // prettier-ignore
      { pid: 900004, name: "fetch", user: "gate", cpu_pct: 7.9261, mem: 46661632, gpu_pct: null, gpu_mem: null, run_secs: 559, arg: "tail/4.py" },
      // prettier-ignore
      { pid: 900005, name: "tokio-rt-worke", user: "gate", cpu_pct: 5.4203, mem: 155160576, gpu_pct: null, gpu_mem: null, run_secs: 696, arg: "tail/5.py" },
      // prettier-ignore
      { pid: 900006, name: "tokio-rt-worke", user: "gate", cpu_pct: 3.8330, mem: 155160576, gpu_pct: null, gpu_mem: null, run_secs: 833, arg: "tail/6.py" },
      // prettier-ignore
      { pid: 900007, name: "spyder", user: "gate", cpu_pct: 2.2191, mem: 472055808, gpu_pct: null, gpu_mem: null, run_secs: 970, arg: "tail/7.py" },
      // prettier-ignore
      { pid: 900008, name: "top", user: "gate", cpu_pct: 1.6829, mem: 8065024, gpu_pct: null, gpu_mem: null, run_secs: 1107, arg: "tail/8.py" },
      // prettier-ignore
      { pid: 900009, name: "tokio-runtime-", user: "gate", cpu_pct: 0.5521, mem: 46661632, gpu_pct: null, gpu_mem: null, run_secs: 1244, arg: "tail/9.py" },
      // prettier-ignore
      { pid: 900010, name: "prometheus", user: "gate", cpu_pct: 0.3185, mem: 166809600, gpu_pct: null, gpu_mem: null, run_secs: 1381, arg: "tail/10.py" },
      // prettier-ignore
      { pid: 900011, name: "containerd", user: "gate", cpu_pct: 0.2230, mem: 24326144, gpu_pct: null, gpu_mem: null, run_secs: 1518, arg: "tail/11.py" },
      // prettier-ignore
      { pid: 900012, name: "UV_WORKER[201]", user: "gate", cpu_pct: 0.1911, mem: 4024270848, gpu_pct: null, gpu_mem: null, run_secs: 1655, arg: "tail/12.py" },
      // prettier-ignore
      { pid: 900013, name: "VLLM::EngineCo", user: "gate", cpu_pct: 0.1274, mem: 1630076928, gpu_pct: null, gpu_mem: null, run_secs: 1792, arg: "tail/13.py" },
      // prettier-ignore
      { pid: 900014, name: "python", user: "gate", cpu_pct: 0.1062, mem: 3850240, gpu_pct: null, gpu_mem: null, run_secs: 1929, arg: "tail/14.py" },
      // prettier-ignore
      { pid: 900015, name: "go.d.plugin", user: "gate", cpu_pct: 0.0903, mem: 73875456, gpu_pct: null, gpu_mem: null, run_secs: 2066, arg: "tail/15.py" },
      // prettier-ignore
      { pid: 900016, name: "kworker/5:2-ev", user: "gate", cpu_pct: 0.0796, mem: 0, gpu_pct: null, gpu_mem: null, run_secs: 2203, arg: "tail/16.py" },
      // prettier-ignore
      { pid: 900017, name: "kworker/4:1-ev", user: "gate", cpu_pct: 0.0743, mem: 0, gpu_pct: null, gpu_mem: null, run_secs: 2340, arg: "tail/17.py" },
      // prettier-ignore
      { pid: 900018, name: "kworker/u257:0", user: "gate", cpu_pct: 0.0690, mem: 0, gpu_pct: null, gpu_mem: null, run_secs: 2477, arg: "tail/18.py" },
      // prettier-ignore
      { pid: 900019, name: "go.d.plugin", user: "gate", cpu_pct: 0.0637, mem: 73875456, gpu_pct: null, gpu_mem: null, run_secs: 2614, arg: "tail/19.py" },
    ],
    cpu_day: {
      bucket_ms: 300_000,
      now_ms: Date.now(),
      buckets: dayBuckets,
      mem: memBuckets,
      gpu: gpuBuckets,
      disk: diskBuckets,
      vram: vramBuckets,
    },
  };
  const ELDER_REFUSAL = "elder-refuses-load";
  // **A held answer, so the in-flight window is something a test can
  // stand inside.** While `hold` is set the route waits before
  // answering, and answers with the rows reversed — which is what makes
  // 「the table did not move while the answer was out」 an assertion
  // with two distinguishable states rather than a wish.
  const held = { on: false, release: null, seen: 0 };
  await page.route("**/load", async (r) => {
    if (r.request().postDataJSON().machine === "elder") {
      return r.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify(ELDER_REFUSAL) });
    }
    if (held.on) {
      held.seen += 1;
      await new Promise((ok) => (held.release = ok));
      const flipped = { ...load, top: [...load.top].reverse() };
      return r.fulfill({ contentType: "application/json", body: JSON.stringify(flipped) });
    }
    return r.fulfill({ contentType: "application/json", body: JSON.stringify(load) });
  });
  // The shell polls sessions, devices and hooks in one Promise.all —
  // one rejection drops the whole batch, so the third leg answers too.
  await page.route("**/hooks_state", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify({ installed: false }) }),
  );
  // 额度: the registered accounts' rings (`QuotaLanding`), three rows
  // that are the three shapes a reading takes — a fraction (an arc and
  // a percent), numbers with no denominator (a spend, a track, no
  // percent), and a trouble key (no numbers, a sentence). Hand-shaped
  // from `AccountRing`/`AccountReading`; drift draws nothing and the
  // 额度 stop goes probe-dead saying so.
  const ring = (account, name, provider, patch) => ({
    reading: {
      account,
      name,
      provider,
      home: "e".repeat(64),
      fill: null,
      spent: null,
      limit: null,
      currency: null,
      unlimited: false,
      resets_at: null,
      trouble: null,
      at_ms: Date.now(),
      ...patch,
    },
    age_ms: 0,
    home_name: "gate",
    mine: true,
    strain: null,
    base_url: null,
    budget: null,
  });
  const board = [
    ring("acc-fill", "gateway", "newapi", {
      fill: 0.45,
      spent: 45,
      limit: 100,
      currency: "USD",
      resets_at: Math.floor(Date.now() / 1000) + 86_400,
    }),
    ring("acc-number", "vendor", "claude", { spent: 12.34, currency: "USD" }),
    // …and the refused one is carried from a peer, so the read-from
    // line (who read it, how long ago) is on the glass to be measured.
    {
      ...ring("acc-trouble", "refused", "openai", { trouble: "key-refused" }),
      age_ms: 42_000,
      home_name: "peer",
      mine: false,
    },
  ];
  await page.route("**/quota_board", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(board) }),
  );
  // **No `**/usage` route any more, and its absence is not an
  // oversight.** This used to answer a fortnight of spending because
  // the wall section 4 was written about only exists at that length.
  // Nothing in the app asks for it since 2026-08-29 — the network's-now
  // 今天 line was its last reader — so a route here would be this gate
  // holding a fixture open for a call that is never made, which is the
  // kind of scaffolding that later reads as evidence the call exists.
  await page.route("**/quota", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        quota: {
          windows: [
            { kind: "five_hour", used_pct: 4, resets_at: Math.floor(Date.now() / 1000) + 3600 },
            { kind: "seven_day", used_pct: 39, resets_at: Math.floor(Date.now() / 1000) + 86_400 },
          ],
          as_of: Math.floor(Date.now() / 1000) - 120,
        },
        trouble: null,
      }),
    }),
  );
  // A place to have been, so the browsing landings are not measured at
  // their empty state and nothing else. **Without these the walk reaches
  // the files landing and finds one button on it** — the whole of a
  // directory (a breadcrumb, its rows) never draws, and a gate that says
  // 「every stop」while stopping at an empty state is the scope hole
  // section 3 is about, one level in.
  const pin = { device: "f".repeat(64), name: "peer", path: "/home/peer/work" };
  await page.route("**/dir_pins", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify([pin]) }),
  );
  await page.route("**/web_pins", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify([{ device: pin.device, name: pin.name, url: "https://example.invalid/" }]),
    }),
  );
  await page.route("**/recents", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify([]) }),
  );
  // Landing somewhere writes a visit down; answered so the write is not
  // the thing that fails while something else is being measured.
  await page.route("**/recent_visit", (r) =>
    r.fulfill({ contentType: "application/json", body: "null" }),
  );
  await page.route("**/ls", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        path: pin.path,
        entries: [
          { name: "src", dir: true, size: 0, at_ms: Date.now() },
          { name: "notes.md", dir: false, size: 1234, at_ms: Date.now() },
        ],
        truncated: false,
      }),
    }),
  );
  // The settings dialog's body. **Without this the settings stop judged
  // one control — the dialog's own close button — while its name said
  // 「settings」**: `FaceSettings` paints nothing at all until
  // `face_choices` answers, so the walk opened the frame and reported on
  // the house, the same hole `files · a directory open` was cut for one
  // level in.
  //
  // Hand-shaped from the bindings like the rows above, and the keys are
  // the library's own (`avatar::PRESETS` ids, `Variant::ALL`,
  // `FaceShape::ALL`) — an invented key would paint a button whose word
  // falls back to the key, which is a different screen from the real one.
  // The three variants carry their three real paint shapes rather than
  // one shape three times: `AvatarFace` has a separate brush per variant,
  // and a fixture that exercised one of them would leave the other two's
  // drift to be found by somebody else.
  const av = (paint) => ({
    canvas: paint.variant === "beam" ? 36 : 80,
    background: "#1d1f21",
    radius_ratio: 0.5,
    paint,
  });
  const marble = {
    variant: "marble",
    blur: 7,
    veins: [
      { vein: "shard", scale: 1.2, rotate: 20, dx: 3, dy: -3, overlay: false, color: "#c5c8c6" },
      { vein: "sweep", scale: 1.4, rotate: 200, dx: -4, dy: 6, overlay: true, color: "#8abeb7" },
    ],
  };
  const bauhaus = {
    variant: "bauhaus",
    shapes: [
      { shape: "rect", x: 0, y: 0, w: 80, h: 80, rotate: 0, dx: 0, dy: 0, color: "#81a2be" },
      { shape: "circle", x: 10, y: 20, w: 40, h: 40, rotate: 20, dx: 4, dy: -4, color: "#f0c674" },
      { shape: "rect", x: 30, y: 50, w: 60, h: 12, rotate: 45, dx: -6, dy: 2, color: "#8abeb7" },
    ],
  };
  const beam = {
    variant: "beam",
    wrapper_rx: 18,
    wrapper_scale: 1.1,
    wrapper_rotate: 0,
    wrapper_dx: 0,
    wrapper_dy: 0,
    wrapper_color: "#81a2be",
    face_ink: "#000000",
    mouth_open: true,
    mouth_spread: 1,
    eye_spread: 2,
    face_rotate: 0,
    face_dx: 0,
    face_dy: 0,
  };
  const swatches = [marble, bauhaus, beam];
  const PRESET_IDS = ["liquid", "monokai", "mocha", "nord", "okabe", "warm", "mandala"];
  const slots = ["#1d1f21", "#c5c8c6", "#81a2be", "#8abeb7", "#f0c674"];
  await page.route("**/face_choices", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        face: av(marble),
        colors: slots,
        preset: "monokai",
        variant: "marble",
        shape: "circle",
        palettes: PRESET_IDS.map((key, i) => ({
          key,
          colors: slots,
          face: av(swatches[i % swatches.length]),
        })),
        variants: ["marble", "bauhaus", "beam"].map((key, i) => ({ key, face: av(swatches[i]) })),
        shapes: ["circle", "rounded", "square"].map((key) => ({ key, face: av(marble) })),
      }),
    }),
  );
  await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });
  await wait(800);

  // ── 1) the chain: token class → computed value ────────────────────
  const chain = await page.evaluate(() => {
    const probe = (cls) => {
      const el = document.createElement("span");
      el.className = cls;
      el.textContent = "0";
      document.body.appendChild(el);
      const cs = getComputedStyle(el);
      const out = { weight: cs.fontWeight, numeric: cs.fontVariantNumeric };
      el.remove();
      return out;
    };
    return {
      title: probe("font-title").weight,
      label: probe("font-label").weight,
      body: probe("font-body").weight,
      tabular: probe("tabular-nums").numeric,
    };
  });
  console.log("chain:", JSON.stringify(chain));
  if (chain.title !== "600") fail(`font-title computes ${chain.title}, not 600`);
  if (chain.label !== "500") fail(`font-label computes ${chain.label}, not 500`);
  if (chain.body !== "400") fail(`font-body computes ${chain.body}, not 400`);
  if (!chain.tabular.includes("tabular-nums")) {
    fail(`tabular-nums computes font-variant-numeric ${chain.tabular}`);
  }

  // ── 2) a real face wears them ─────────────────────────────────────
  // Probe first: the routed row actually drew. An assertion about a
  // list that never painted would be measuring an empty page.
  const face = await page.evaluate(() => {
    const rowTitle = [...document.querySelectorAll("[data-list] [data-session-title]")].find(
      (e) => e.textContent?.includes("face-gate-title"),
    ) ?? [...document.querySelectorAll("[data-list] *")].find(
      (e) => e.childElementCount === 0 && e.textContent?.trim() === "face-gate-title",
    );
    const weights = {};
    for (const el of document.querySelectorAll("*")) {
      const w = getComputedStyle(el).fontWeight;
      weights[w] = (weights[w] || 0) + 1;
    }
    const tabulars = [...document.querySelectorAll(".tabular-nums")].map((el) => ({
      ok: getComputedStyle(el).fontVariantNumeric.includes("tabular-nums"),
    }));
    return {
      rowDrawn: !!rowTitle,
      rowWeight: rowTitle ? getComputedStyle(rowTitle).fontWeight : null,
      weights,
      tabularCount: tabulars.length,
      tabularBad: tabulars.filter((t) => !t.ok).length,
    };
  });
  console.log("face:", JSON.stringify(face));
  if (!face.rowDrawn) fail("probe dead: the routed session row never drew");
  else if (face.rowWeight !== "600") {
    fail(`the row's title weighs ${face.rowWeight}, not 600 — the title token is not reaching the list`);
  }
  for (const w of ["400", "500", "600"]) {
    if (!face.weights[w]) fail(`no element on the face weighs ${w} — the distribution is flat again`);
  }
  if (face.tabularCount === 0) fail("probe dead: nothing on this face opted into tabular-nums");
  if (face.tabularBad > 0) fail(`${face.tabularBad} element(s) claim tabular-nums but compute without it`);

  // ── 3) touch targets: the whole face, not one strip ──────────────
  // The token is the criterion (`--touch`, tokens.css: HIG / WCAG floor)
  // and is read off the page rather than repeated here — a second copy
  // of the number is how a gate and a stylesheet drift apart. Measured
  // on the narrow face because that is the posture with fingers, and on
  // the wide face too: a desktop is where a trackpad's imprecision and
  // the web face on an iPad both live.
  //
  // **And measured with a finger on the glass, which it was not until
  // 2026-08-30.** The floor is a floor for fingers — the token says so,
  // HIG and WCAG say so, and every waiver below is argued in those
  // terms — while this walk ran in a plain desktop Chrome that reports
  // `(pointer: fine)` and `(hover: hover)`. Two postures of one
  // *viewport* is not two postures of one *pointer*, and the day a
  // control was made to wear the width floor only under `(pointer:
  // coarse)` this section went red against a screen that has no finger
  // anywhere near it — a red that was true of nothing (the breadcrumb,
  // `FilesKind.tsx`; 「目录显示很丑,有很多奇怪的空格」 was the same
  // width, seen from the other side). A gate that demands a finger's
  // floor on a mouse's screen does not merely miss things: it argues
  // for the bug.
  //
  // So the walk turns the finger on. `Emulation.setTouchEmulationEnabled`
  // is what does it — **and `Emulation.setEmulatedMedia` with a
  // `pointer` feature is not**, which is worth writing down because it
  // is the obvious thing to reach for and it fails silently: measured,
  // the call is accepted and `matchMedia("(pointer: coarse)")` stays
  // `false`, so a gate built on it would report a green it never
  // measured. Blink honours that override for `prefers-*` and friends,
  // not for the pointer. The touch flag is the same one smoke.mjs §18
  // reaches for, and it is asserted every posture rather than trusted:
  // an override that stopped taking would leave every stop measuring
  // the mouse again under a heading that says finger.
  //
  // **What it costs, and it is paid on purpose**: with touch on,
  // `(hover: hover)` is false, so hover-revealed controls are not
  // revealed. They are still laid out and still measured (the excuses
  // below turn on layout and visibility, not on opacity), and a control
  // a finger can never summon is not a control this section can speak
  // for anyway. The flag goes off again before section 5, and that the
  // pointer came back is asserted, not hoped for — the scar is
  // smoke.mjs's: one item's flag left on cost five other items their
  // meaning without a word.
  //
  // **The scope is the claim, and it used to be one sixth of it.** This
  // section measured `[data-rail-item]` — six buttons — and stayed green
  // while a search box was 18 tall, a row's pin 30×30, a card's three
  // verbs 24 tall and the ledger's rows 29. The assertion was true; it
  // was true about the rail. That is the same 假绿 the density section
  // below was born from (docs/DESIGN.md 二: 断言成立,只是成立在别处),
  // and the same shape twice in one day is what makes it a family
  // rather than an accident.
  //
  // So it walks: every surface the rail can reach, in both postures, and
  // every pressable element on each. Pressable is the selector list
  // below — what a pointer and a keyboard can both reach.
  //
  // **The excuses are written down, not filtered out.** A quiet filter
  // is exactly how a scope shrinks back to a strip: nothing in the
  // output would say a whole class of controls had stopped being
  // judged. Each one names why a finger cannot land there, and the
  // counts are printed under every stop.
  //
  // **What it cannot block:**
  //   - a bare `<div onClick>` — no role, no tabindex, nothing in the
  //     DOM says it is a target, so this section cannot see it. **That
  //     is section 4's job now**, and it is a separate section rather
  //     than a bullet here because the criterion is a different one: not
  //     how big the target is, but whether anything except a mouse can
  //     tell it is one. The `cursor: pointer` survey printed beside each
  //     stop stays **printed, never asserted** — a hover style on a row
  //     would make it a red that teaches people to widen the excuses,
  //     and section 4 reads the handler itself instead of guessing from
  //     a cursor;
  //   - a target that is big enough and sits under something else.
  //
  // **An excuse and a waiver are not the same thing, and they are kept
  // apart on purpose.** The four excuses below all say *a finger cannot
  // land here* — nothing is drawn, or the press goes elsewhere. A waiver
  // says the opposite: a finger can land here, it is under the floor,
  // and the floor is the wrong criterion for this one thing. Those are
  // written in `UNDER_FLOOR` below, by name, with what it still costs.
  // Filed together they would rot into one list of「reasons this is
  // fine」, and the day somebody wants a fifth excuse the list would
  // already read as permissive.
  const PRESSABLE = [
    "button",
    "a[href]",
    "input",
    "select",
    "textarea",
    "summary",
    '[role="button"]',
    '[role="tab"]',
    '[role="link"]',
    '[role="checkbox"]',
    '[role="switch"]',
    '[role="menuitem"]',
    '[role="option"]',
    '[contenteditable=""]',
    '[contenteditable="true"]',
    '[tabindex]:not([tabindex="-1"])',
  ].join(",");

  // Where a marker is written, so a red names a file instead of a shape.
  // Read once, from source: a gate that says「30×30」and nothing else
  // makes the reader grep for it, and a reader who has to grep will read
  // the number as somebody else's problem.
  const srcRoot = join(gui, "src");
  const sources = [];
  (function collect(dir) {
    for (const entry of readdirSync(dir)) {
      const p = join(dir, entry);
      if (statSync(p).isDirectory()) {
        if (entry !== "gen") collect(p);
      } else if (/\.(tsx?|css)$/.test(p)) {
        sources.push({ rel: relative(gui, p).replaceAll("\\", "/"), lines: readFileSync(p, "utf8").split("\n") });
      }
    }
  })(srcRoot);
  // **Every file the marker is written in, not the first one.** Two
  // lists share `data-row-open`, and a gate that named one of them would
  // be handing out a fact it had not checked — the expensive kind of
  // wrong (a report that sends somebody to edit a file that is fine).
  const whereIs = (attr) => {
    const hits = [];
    for (const f of sources) {
      const i = f.lines.findIndex((l) => l.includes(attr));
      if (i >= 0) hits.push(`${f.rel}:${i + 1}`);
    }
    return hits.length ? hits.slice(0, 3).join(" | ") : null;
  };
  // Three ways to name where a thing is written, in falling order of
  // certainty, and **the line says which one it used** — an approximate
  // location presented as an exact one is the kind of fact that sends
  // somebody to edit a file that is fine. Shared by sections 3 and 4:
  // section 4's offenders are precisely the ones with no marker of their
  // own (that is what makes them offenders), so it needs the fallbacks
  // more than section 3 does, and a red that could only say「somewhere」
  // is a red nobody can act on.
  const nameWhere = (t) => {
    const own = t.own.map((a) => [a, whereIs(a)]).find(([, at]) => at);
    if (own) return own[1];
    const byClass = t.cls && whereIs(t.cls.split(" ").slice(0, 3).join(" "));
    if (byClass) return `${byClass} (by class)`;
    const via = t.chain.map((a) => [a, whereIs(a)]).find(([, at]) => at);
    if (via) return `${via[1]} (the surface it is on, not where it is written)`;
    return "no marker and no class — grep for the words on it";
  };

  const sweep = () =>
    page.evaluate((PRESSABLE) => {
      const floor = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--touch"));
      // `data-state` and `data-slot` are radix's and shadcn's own, on
      // every second element — they name a library, not a control.
      const marks = (el) =>
        [...el.attributes]
          .map((a) => a.name)
          .filter((n) => n.startsWith("data-") && n !== "data-slot" && n !== "data-state");
      const label = (el) => {
        const chain = [];
        for (let p = el.parentElement; p && chain.length < 4; p = p.parentElement) {
          const m = marks(p);
          if (m.length) chain.push(m[0]);
        }
        return {
          tag: el.tagName.toLowerCase(),
          own: marks(el),
          chain,
          said: (el.getAttribute("aria-label") || el.textContent || "")
            .trim()
            .replace(/\s+/g, " ")
            .slice(0, 20),
          cls: (typeof el.className === "string" ? el.className : "").slice(0, 40),
        };
      };
      // A disabled control is deliberately **not** excused: it is
      // pressable again the moment the call it is waiting on returns,
      // and it is the same box either way.
      const excuse = (el, cs) => {
        if (el.getClientRects().length === 0) {
          return "not laid out — nothing is drawn there for a finger to land on";
        }
        if (cs.visibility === "hidden") return "visibility:hidden — drawn nowhere, as above";
        if (el.closest('[aria-hidden="true"],[data-aria-hidden],[inert]')) {
          return "out of reach behind a modal — judged on the surface where it is reachable";
        }
        if (cs.pointerEvents === "none" && !el.disabled) {
          return "pointer-events:none — the press goes through it (focus guards); not a target";
        }
        return null;
      };
      const targets = [];
      const excused = {};
      for (const el of document.querySelectorAll(PRESSABLE)) {
        const cs = getComputedStyle(el);
        const why = excuse(el, cs);
        if (why) {
          excused[why] = (excused[why] || 0) + 1;
          continue;
        }
        const r = el.getBoundingClientRect();
        targets.push({ ...label(el), w: Math.round(r.width), h: Math.round(r.height) });
      }
      const pointing = [...document.querySelectorAll("*")].filter(
        (el) =>
          getComputedStyle(el).cursor === "pointer" && !el.matches(PRESSABLE) && !el.closest(PRESSABLE),
      );
      // Section 4's reading, taken on the same pass: which elements
      // actually carry a press handler, read off React's own props
      // rather than guessed at from a cursor. See section 4 below for
      // what is done with it and why the cursor survey above stayed a
      // printed line while this one is asserted.
      const propsOf = (el) => {
        for (const k in el) if (k.startsWith("__reactProps$")) return el[k];
        return null;
      };
      const PRESS = ["onClick", "onMouseDown", "onPointerDown", "onDoubleClick"];
      let withProps = 0;
      let roledPress = 0;
      const bare = [];
      for (const el of document.querySelectorAll("*")) {
        const p = propsOf(el);
        if (p) withProps += 1;
        if (!p || !PRESS.some((k) => typeof p[k] === "function")) continue;
        if (el.matches(PRESSABLE) || el.closest(PRESSABLE)) {
          roledPress += 1;
          continue;
        }
        bare.push({ ...label(el), on: PRESS.filter((k) => typeof p[k] === "function").join("+") });
      }
      return { floor, targets, excused, pointing: pointing.length, withProps, roledPress, bare };
    }, PRESSABLE);

  // The walk. Each stop names the surface, how to get there, and one
  // anchor that has to be on screen when we arrive — the anchor is what
  // tells「this lane has not fixed its control yet」apart from「the gate
  // walked into a wall」: a renamed marker or a moved landing makes the
  // stop say *probe dead* instead of quietly measuring the surface we
  // were already on and reporting its offenders a second time.
  const rail = (key) => `[data-rail-item][data-landing="${key}"]`;
  const pressNth = async (sel, n) => {
    if ((await page.locator(sel).count()) <= n) return false;
    await page.locator(sel).nth(n).click();
    await wait(500);
    return true;
  };
  const press = (sel) => pressNth(sel, 0);
  // Reach a landing as a *switch*, never a re-click. Pressing the open
  // landing's own glyph now collapses its list (App.tsx 再点收起,
  // 2026-09-02), and this walk re-opens the same landing across
  // consecutive stops — so a raw re-press would put the list away and
  // the row a stop then reaches for would be unclickable. Hopping
  // through another landing makes the return a fresh switch, which
  // always leaves the list expanded. Only sessions/devices carry a
  // list that toggles; the others no-op through this unchanged.
  const open = async (tab) => {
    const on = await page.locator(rail(tab)).getAttribute("data-on").catch(() => null);
    if (on === "true") await press(rail(tab === "sessions" ? "devices" : "sessions"));
    return press(rail(tab));
  };
  const STOPS = [
    { name: "sessions list", anchor: "[data-list]", go: () => open("sessions") },
    {
      // A row's own menu — pin and close, on right-click and on a long
      // press (`RowMenu`). **It is the first surface in this walk that
      // no amount of pressing things can reach**, and that is the whole
      // reason it needs a stop: every other stop is a landing somebody
      // navigates to, so a walk built out of clicks arrives by
      // accident. A menu is asked for, and until it is asked for, its
      // controls are not in the DOM to be measured — a scope hole of
      // exactly the kind section 3 keeps being widened for, and the
      // one the controls this menu **replaced** used to be caught by.
      //
      // Right-click in both postures, because that is what this gate
      // can produce. **The long press is not covered here**: Radix opens
      // on a 700ms `pointerdown` whose `pointerType` is touch or pen,
      // and neither playwright's mouse nor `touchscreen.tap()` makes
      // one — it takes a raw CDP `Input.dispatchTouchEvent` held open,
      // which is a second input pathway this file has nowhere else. So
      // this stop measures the menu, not the two ways in; the touch way
      // in was verified by hand and stays unguarded, which is written
      // here rather than left to be assumed from the stop's name.
      name: "sessions list · a row's menu",
      // Only this surface draws it. An item rather than the menu's own
      // box, for the settings stop's reason: a menu that opened and
      // rendered nothing would still satisfy the box.
      anchor: "[data-row-menu] [data-menu-item]",
      go: async () => {
        if (!(await open("sessions"))) return false;
        const row = page.locator("[data-list] [data-row-open]").first();
        if ((await row.count()) === 0) return false;
        await row.click({ button: "right" });
        await wait(500);
        return true;
      },
      // Shut on the way out. A menu left open is a modal layer, and the
      // next stop would find its own controls `aria-hidden` behind it
      // and excuse them in bulk — which prints as a stop that was
      // measured and is a stop that was not.
      after: async () => {
        await page.keyboard.press("Escape");
        await wait(300);
      },
    },
    {
      name: "session detail",
      anchor: "[data-detail-header]",
      go: async () => (await open("sessions")) && press("[data-list] [data-row-open]"),
    },
    // The map rides along on the wide face, where it is what the pane
    // opens into; narrow has the list alone, so the anchor is the row —
    // an anchor that only exists in one posture would report the other
    // one as a dead probe forever.
    { name: "devices", anchor: "[data-device]", go: () => open("devices") },
    {
      // This machine's card: the only one carrying the hooks control,
      // and the only one with no readings (it never asks itself).
      name: "machine card · this machine",
      anchor: "[data-hooks]",
      go: async () => (await open("devices")) && press("[data-list] [data-row-open]"),
    },
    {
      // …and a machine that has answered, which is the only way the
      // card's rings are on screen to be measured at all. The anchor is
      // the readings themselves, so a drifted `VitalsReading` shape
      // says *probe dead* here rather than passing a card that drew a
      // 「never reached」line instead.
      name: "machine card · a machine that answered",
      anchor: "[data-vitals]",
      go: async () => (await open("devices")) && pressNth("[data-list] [data-row-open]", 1),
    },
    { name: "files", anchor: '[data-browse-landing="dir"]', go: () => press(rail("files")) },
    {
      // …and a directory actually open, which is where that landing's
      // controls are: the breadcrumb, the rows. The new-tab page is one
      // button and a list of places; stopping there would have been a
      // walk that visits the door and reports on the house.
      name: "files · a directory open",
      anchor: "[data-crumb]",
      go: async () => (await press(rail("files"))) && press("[data-place]"),
    },
    {
      // Folded behind「…」on the narrow bar (App.tsx `NARROW_SHOWN`),
      // so the way in is two presses there; the direct press is tried
      // first so wide stays one. The fold's own menu gets no stop of
      // its own — it closes on select, and an anchor only one posture
      // has would read probe-dead on the other (the devices stop's
      // rule).
      name: "browser",
      anchor: '[data-browse-landing="page"]',
      go: async () =>
        (await press(rail("browser"))) ||
        ((await press("[data-rail-more]")) && press('[data-more-landing="browser"]')),
    },
    {
      // 额度: one region, no list-beside-detail, and the anchor is a
      // row so a drifted `AccountRing` shape says probe dead instead of
      // measuring an empty page. The register dialog is deliberately
      // not a stop — none of the "+" dialogs are walked yet (New /
      // Tell / Invite / Join), and that hole is one hole, not five.
      name: "\u989d\u5ea6",
      anchor: "[data-quota-row]",
      go: () => press(rail("quota")),
    },
    { name: "the mark's place", anchor: "[data-mark-place]", go: () => press("[data-rail-mark]") },
    {
      // The body, not the frame. **This stop used to be named
      // 「settings (frame only)」and judged one control** — the dialog's
      // close button — because `FaceSettings` paints nothing until
      // `face_choices` answers and this fixture did not answer it. Naming
      // the hole in the stop's own name was better than counting zero
      // and moving on, and it was still a walk to the door: the palettes,
      // the color wells and the three axes of options are the settings
      // screen, and none of them were on the page.
      //
      // The anchor is an option **inside the last axis**, which is the
      // one thing only this screen draws and the one that is absent if
      // the body drew half of itself: a frame with a spinner in it, or a
      // `FaceChoices` whose shape drifted so the render threw partway,
      // both say *probe dead* here rather than passing.
      name: "settings",
      anchor: '[data-face-axis="shape"] [data-face-option]',
      // On the wide column 设置 is its own rail item — the one that
      // opens no landing and is not the mark, so it is named by what it
      // is not. On the narrow bar it moved inside the「…」fold
      // (2026-09-02 `NARROW_SHOWN`), so there it is two presses: open
      // the fold, then its own row. The direct press is tried first, so
      // a renamed wide glyph still says probe-dead rather than being
      // masked by the fold path.
      go: async () =>
        (await press(
          "[data-rail-item]:not([data-landing]):not([data-rail-more]):not(:has([data-rail-mark]))",
        )) || ((await press("[data-rail-more]")) && press("[data-more-settings]")),
      after: async () => {
        await page.keyboard.press("Escape");
        await wait(300);
      },
    },
  ];

  // ── 4) a thing that can be pressed says so in the DOM ─────────────
  // **This is not the touch floor and must not be filed under it.** The
  // floor asks how big a target is; this asks whether it is a target at
  // all as far as anything but a mouse is concerned. A bare
  // `<div onClick>` is pressed by a pointer and by nothing else: it
  // takes no focus, a keyboard cannot reach it, a screen reader does not
  // announce it, and — the reason it lands in this file — section 3
  // cannot measure it either, because `PRESSABLE` has no way to see it.
  // Every hole in the section above is downstream of this one, which is
  // why closing it is worth an assertion rather than a note.
  //
  // **The handler is read, not inferred.** The stop lines still print a
  // `cursor: pointer` survey and that one stays printed: a hover style
  // on a decorative row makes it a false red, and a false red is how
  // people learn to widen the excuses. This reads React's own props off
  // the element (`__reactProps$*`), so a hit is a handler somebody
  // wrote — there is no decorative way to acquire one, and that is what
  // makes it safe to assert where the cursor proxy is not.
  //
  // **A ratchet, not a clean sheet.** Run on the tree the day it was
  // written it found three carriers, so an unconditional「zero」would
  // have been red on arrival and turned off within the day. They are
  // written down here instead, each with what it costs, and the gate
  // moves in both directions: a carrier that is not on this list is red
  // (somebody wrote a new one), and a listed one that stops turning up
  // is **also** red (the debt was paid — delete the line). A list that
  // cannot go stale is the difference between a written-down exception
  // and a filter, which is the same judgment the excuses above are made
  // under.
  //
  // **What it cannot block**: a handler bound with `addEventListener`
  // instead of a prop (`ColorWell` binds `change` that way) — that is
  // invisible to this and to any DOM reading, and the reason the live
  // probe below asserts the reading works at all rather than trusting a
  // zero.
  const BARE_PRESSES = {
    "data-omnibox": [
      "src/components/Omnibox.tsx",
      "a forwarder, not a target: the handler fires only for " +
        "`e.target === e.currentTarget` (the strip's own blank space) and does " +
        "one thing — focus the `<input>` inside it. The target is in the DOM " +
        "and is that input; this widens its press area onto the field's padding.",
    ],
    "data-row-menu": [
      "src/components/RowMenu.tsx (the handler is Radix's, not ours)",
      "not a target and not ours: the carrier is the menu's own box, and the " +
        "`onMouseDown` on it belongs to `RovingFocusGroup` " +
        "(`@radix-ui/react-roving-focus`, `isClickFocusRef.current = true`) — it " +
        "sets a flag so the group can tell a click-entry from a keyboard-entry " +
        "and skip its auto-focus. It acts on nothing. **What this waiver does " +
        "not buy:** it says nothing about the two items inside, which carry the " +
        "real handlers and are `role=\"menuitem\"` — section 3 measures those, " +
        "and they are the reason this stop exists. The box itself is reachable " +
        "by keyboard regardless (Radix gives it `role=\"menu\"` and a tabindex), " +
        "so the one thing this section is about — a target only a pointer can " +
        "find — is not what is happening here.",
    ],
  };

  // Section 3's waivers: the floor does not apply here, and here is why
  // — a different claim from the four excuses above, which say a finger
  // cannot land at all. Same ratchet as `BARE_PRESSES`: named, printed
  // every run, red in both directions.
  //
  // **A waiver has to carry the part that is still broken.** Left as
  // 「the floor is the wrong criterion」a line like this reads as「no
  // problem here」, and the next person to meet the control believes it.
  // So each entry says what a waiver does not buy.
  const UNDER_FLOOR = {
    "data-list-seam": [
      "src/App.tsx",
      "the floor is a floor under things whose size is free to grow, and this " +
        "one's is not: it is the seam between two panes (`-mx-1 w-2`), and every " +
        "pixel it grows is taken off the list's right edge and the detail's left " +
        "— off the rows people actually press. That is WCAG 2.5.5's own Essential " +
        "carve-out, not a hole cut here. **What it does not buy:** a finger on a " +
        "wide touch screen — the iPad web face, which is the whole reason this " +
        "section measures the wide posture — can drag this seam today " +
        "(`onPointerDown` + `setPointerCapture`, and pointer events are touch " +
        "events) and will find 8 hard. That is open debt; the answer is a " +
        "separate touch affordance, not a wider seam. **It is not「no problem」.**",
    ],
    "data-cut": [
      "src/views/MachineCard.tsx",
      "a row of the process table, which is 18 tall because nine rows in a " +
        "fixed window is what the table is for — the height is the feature. " +
        "Growing this cell to 44 grows the row, and the window then holds four " +
        "rows instead of nine, so the floor here is not a floor under something " +
        "free to grow: it trades the readability of the whole table for one " +
        "cell's target. The target is also the **whole cell**, not the 10-pixel " +
        "square drawn in it — 332 wide at the narrow stop, which is a long thin " +
        "target rather than a tiny one. **What it does not buy:** a finger still " +
        "has 18 pixels of height to land in, and on a touch screen that is hard. " +
        "It is the only way to the full command line at narrow, so this is a " +
        "real hole, not a technicality — the answer is a row that opens on a " +
        "press anywhere along it, which is a bigger change to how the table " +
        "reads. **It is not「no problem」.**",
    ],
  };

  // Same control, same posture, same size, on four surfaces is one
  // thing to fix — collapsed so the list reads as a work queue.
  const under = new Map();
  // Same collapsing for section 4: one carrier, one line, however many
  // surfaces it rides on.
  const carriers = new Map();
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 1 });
  for (const [posture, vp] of [
    ["narrow", { width: 390, height: 844 }],
    ["wide", { width: 1400, height: 900 }],
  ]) {
    await page.setViewportSize(vp);
    // A fresh list each posture: the rail's collapse persists
    // (`khor.list-width`), and a walk that collapsed one posture would
    // start the next one hidden.
    await page.evaluate(() => {
      try { localStorage.removeItem("khor.list-width"); } catch { /* ignore */ }
    });
    await page.reload({ waitUntil: "load" });
    await wait(900);
    // The instrument, before the readings. A floor asserted on a screen
    // with no finger on it is the false red this section was rewritten
    // for; the same override quietly not taking would be its false
    // green, and the two look identical in the output.
    const finger = await page.evaluate(() => ({
      coarse: matchMedia("(pointer: coarse)").matches,
      hover: matchMedia("(hover: hover)").matches,
    }));
    if (!finger.coarse) {
      fail(`probe void: ${posture} is being measured with no finger on the glass — the touch override did not take, so every floor below is asserted against a mouse`);
    }
    console.log(`posture ${posture}: ${vp.width}×${vp.height}, pointer coarse ${finger.coarse}, hover ${finger.hover}`);
    for (const stop of STOPS) {
      await stop.go();
      const here = `${posture} · ${stop.name}`;
      if (!(await until(5_000, async () => (await page.locator(stop.anchor).count()) > 0))) {
        fail(`probe dead: ${here} never drew (${stop.anchor}) — the walk, not the controls`);
        await stop.after?.();
        continue;
      }
      const seen = await sweep();
      if (!(seen.floor > 0)) fail(`${here}: --touch does not resolve on the page`);
      if (seen.targets.length === 0) fail(`probe dead: ${here} has nothing pressable to measure`);
      const excuses = Object.entries(seen.excused)
        .map(([why, n]) => `${n} ${why}`)
        .join("; ");
      console.log(
        `touch ${here}: ${seen.targets.length} judged` +
          (excuses ? `, excused ${excuses}` : "") +
          (seen.pointing ? `, ${seen.pointing} unmarked cursor:pointer (printed, not asserted)` : ""),
      );
      // The live probe for section 4, per stop: a negative claim
      // (「nothing here is bare」) is worth nothing until the same reading
      // has hit something it should hit. If React's prop key ever moves,
      // both counts go to zero and every stop reports a clean sheet —
      // the `.nav-badge` shape, where an empty result reads as「this does
      // not exist」and means「I looked in the wrong place」.
      if (seen.withProps === 0) {
        fail(`probe dead: ${here} — no element carries React props, the handler reading is blind`);
      } else if (seen.roledPress === 0) {
        fail(`probe dead: ${here} — not one press handler found on anything with a role`);
      }
      console.log(
        `says-so ${here}: ${seen.roledPress} press handlers on elements that say they are targets` +
          `, ${seen.bare.length} on elements that do not (of ${seen.withProps} react nodes)`,
      );
      for (const b of seen.bare) {
        const key = b.own.join(" ") || `<${b.tag}> .${b.cls}`;
        const was = carriers.get(key);
        if (was) {
          was.n += 1;
          continue;
        }
        carriers.set(key, {
          n: 1,
          where: here,
          on: b.on,
          said: b.said,
          marker: b.own.find((a) => BARE_PRESSES[a]) ?? null,
          at: nameWhere(b),
        });
      }
      for (const t of seen.targets) {
        if (t.w >= seen.floor && t.h >= seen.floor) continue;
        const key = `${posture}|${t.own.join(" ") || t.cls}|${t.w}×${t.h}`;
        const was = under.get(key);
        if (was) {
          was.n += 1;
          continue;
        }
        under.set(key, {
          n: 1,
          where: here,
          what:
            `${t.own.join(" ") || `<${t.tag}> .${t.cls}`}` + (t.said ? ` "${t.said}"` : ""),
          size: `${t.w}×${t.h}`,
          floor: seen.floor,
          at: nameWhere(t),
          waived: t.own.find((a) => UNDER_FLOOR[a]) ?? null,
        });
      }
      await stop.after?.();
    }
  }
  let offenders = 0;
  for (const u of [...under.values()]) {
    const waiver = u.waived && UNDER_FLOOR[u.waived];
    if (waiver) {
      console.log(
        `touch · waived under the floor: ${u.what} ${u.size}` +
          (u.n > 1 ? ` ×${u.n}` : "") +
          ` — ${waiver[0]}: ${waiver[1]}`,
      );
      continue;
    }
    offenders += 1;
    fail(
      `under the ${u.floor} touch floor — ${u.where}: ${u.what} ${u.size}` +
        (u.n > 1 ? ` (×${u.n})` : "") +
        ` — ${u.at}. Wear the floor at the call site (min-h-touch / min-w-touch), ` +
        `or write it into UNDER_FLOOR in this file with why the floor is the ` +
        `wrong criterion **and what the waiver does not buy**.`,
    );
  }
  // The other direction, same as the census below: a waiver that has
  // stopped being needed is a line nobody will delete unless something
  // says so, and a stale waiver is how a list of one becomes a list of
  // four.
  for (const marker of Object.keys(UNDER_FLOOR)) {
    if ([...under.values()].some((u) => u.waived === marker)) continue;
    fail(
      `waiver stale: ${marker} is written into UNDER_FLOOR but clears the floor ` +
        `on every stop — either it grew (delete its line) or the walk stopped ` +
        `reaching the surface it lives on (fix the walk).`,
    );
  }
  if (offenders === 0) {
    console.log("touch: every pressable thing on every stop clears the floor or is waived by name");
  }

  // Section 4's verdict, both ways round.
  for (const [key, c] of carriers) {
    const known = c.marker && BARE_PRESSES[c.marker];
    if (known) {
      console.log(
        `says-so · known bare press: ${key} (${c.on})` +
          (c.n > 1 ? ` ×${c.n}` : "") +
          ` — ${known[0]}: ${known[1]}`,
      );
      continue;
    }
    fail(
      `a press handler on something that does not say it is a target — ${c.where}: ` +
        `${key}${c.said ? ` "${c.said}"` : ""} carries ${c.on}` +
        (c.n > 1 ? ` (×${c.n})` : "") +
        ` — ${c.at}. Give it a role and a focus stop (a <button>, or role + tabindex), ` +
        `or write it into BARE_PRESSES in this file with what it costs.`,
    );
  }
  for (const marker of Object.keys(BARE_PRESSES)) {
    if ([...carriers.values()].some((c) => c.marker === marker)) continue;
    fail(
      `census stale: ${marker} is written into BARE_PRESSES but carries no bare press handler ` +
        `on any stop — either the debt was paid (delete its line) or the walk stopped ` +
        `reaching the surface it lives on (fix the walk).`,
    );
  }

  // ── 3b) the breadcrumb spends the floor only on the finger ────────
  // Section 3 above now asks its question of a hand. This asks the other
  // half of the same question, and it exists because section 3 **cannot
  // fail on it**: a control that is 44 wide for everybody clears the
  // finger's floor forever, and the screen it ruins is the one section 3
  // has stopped visiting.
  //
  // What ruins it: a step in a path is as wide as its name, and a floor
  // applied to all of them makes the gap after a step **inversely
  // proportional to the length of its name** — 40px of nothing after
  // `/`, 26 after `nfs`, 4 after `workingdir`. That is not spacing, it
  // reads as damage, and it is what a person actually said out loud
  // (「目录显示很丑,有很多奇怪的空格」, 2026-08-29).
  //
  // **So the assertion is on the spread, not on the width.** Any fixed
  // padding is fine and the number is the stylesheet's business; what
  // may not happen is one step being padded more than another *because
  // its name is shorter*. Written as「every step is its own natural
  // width」the check would be a second copy of `px-0.5` living in this
  // file, red the day somebody changes the padding for a good reason.
  //
  // The two halves are one section on purpose: the finger half is what
  // keeps「no floor at all」from passing, and the mouse half is what
  // keeps「the floor for everybody」from passing. Either alone is a
  // green that argues for the other bug.
  const crumbSpend = () =>
    page.evaluate(() => {
      const crumbs = [...document.querySelectorAll("[data-crumb]")].map((el) => {
        const box = el.getBoundingClientRect();
        // The name's own width, measured on the text rather than
        // computed from a font: a Range over the button's contents is
        // what the browser actually laid out.
        const range = document.createRange();
        range.selectNodeContents(el);
        const text = range.getBoundingClientRect();
        return {
          label: (el.textContent || "").trim(),
          // The last step is not a step: it takes the blank tail so
          // that pressing the empty part of the bar opens the editor
          // (`FilesKind.tsx` says why it is one target and not two), so
          // its width is about the bar, not about its name.
          rest: el.hasAttribute("data-address-rest"),
          text: Math.round(text.width * 10) / 10,
          box: Math.round(box.width * 10) / 10,
        };
      });
      return {
        floor: parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--touch")),
        coarse: matchMedia("(pointer: coarse)").matches,
        crumbs,
      };
    });
  const atCrumbs = async () => {
    await press(rail("files"));
    // The walk left a tab open, so the new-tab page and its `[data-place]`
    // need not be there — pressing it is an attempt, not a requirement.
    await press("[data-place]");
    return (await page.locator("[data-crumb]").count()) > 0;
  };
  await page.setViewportSize({ width: 1400, height: 900 });
  const spend = {};
  for (const [posture, coarse] of [
    ["finger", true],
    ["mouse", false],
  ]) {
    await cdp.send("Emulation.setTouchEmulationEnabled", coarse ? { enabled: true, maxTouchPoints: 1 } : { enabled: false });
    await wait(300);
    if (!(await atCrumbs())) {
      fail(`probe dead: no breadcrumb on the files landing under a ${posture}`);
      continue;
    }
    const seen = await crumbSpend();
    if (seen.coarse !== coarse) {
      fail(`probe void: asked for a ${posture} and got pointer coarse=${seen.coarse} — the override did not take`);
      continue;
    }
    spend[posture] = seen;
    console.log(
      `crumbs · ${posture}: ` +
        seen.crumbs
          .map((c) => `${JSON.stringify(c.label)} ${c.text}→${c.box}${c.rest ? " (the tail)" : ""}`)
          .join(", "),
    );
  }
  // …and the pointer is back, which sections 5 and 5b are measured
  // under. Asserted rather than assumed: this is the moment smoke.mjs
  // §14 was lost at.
  if (!(await page.evaluate(() => matchMedia("(hover: hover)").matches))) {
    fail("the pointer never came back after section 3 — everything below this line is measuring a screen with no hover");
  }
  if (!spend.finger || !spend.mouse) {
    fail("probe dead: the breadcrumb was not read in both postures, so neither half below judged");
  } else {
    const steps = (s) => s.crumbs.filter((c) => !c.rest);
    // Both halves are vacuous on a path whose every name is already
    // wider than the floor — there would be nothing for a floor to
    // raise and nothing for a mouse to hand back. The fixture's path is
    // `/home/peer/work`, so this holds; it is asserted because a fixture
    // is a thing somebody edits.
    if (steps(spend.finger).length < 3) {
      fail(`probe dead: ${steps(spend.finger).length} steps in the breadcrumb — the fixture's path is too shallow to judge`);
    } else if (!steps(spend.mouse).some((c) => c.text < spend.mouse.floor)) {
      fail("probe void: every step's name is already wider than the floor, so neither half of this section is about anything — deepen the fixture's path");
    } else {
      for (const c of steps(spend.finger)) {
        if (c.box + 0.5 < spend.finger.floor) {
          fail(`with a finger, the step ${JSON.stringify(c.label)} is ${c.box} wide against a ${spend.finger.floor} floor — ${whereIs("data-crumb")}`);
        }
      }
      const slack = steps(spend.mouse).map((c) => Math.round((c.box - c.text) * 10) / 10);
      const spread = Math.round((Math.max(...slack) - Math.min(...slack)) * 10) / 10;
      console.log(`crumbs · with a mouse the slack after each step is ${JSON.stringify(slack)}, spread ${spread}`);
      // 1px of spread is sub-pixel text metrics, not a floor: a floor
      // shows up here as tens of pixels (40 after `/`, 4 after a long
      // name) because it is the *difference* from the name's width.
      if (spread > 1) {
        fail(
          `with a mouse the steps are padded unevenly — slack ${JSON.stringify(slack)}, spread ${spread}px. ` +
            `A step's gap must not depend on the length of its name; that is a touch floor being spent on a ` +
            `screen with no finger on it (src/views/FilesKind.tsx).`,
        );
      }
    }
  }

  // ── 3c) one listing, one row ──────────────────────────────────────
  // A directory listing draws two kinds of row from two different
  // branches, and **nothing above this line can see them drift apart**:
  // section 3 measures how big a target is, and both rows' targets were
  // always fine while the rows around them were not. Two ways they
  // drifted, both shipped, both found by a person looking at the screen
  // rather than by anything here:
  //
  //   - **height.** A file row holds a control wearing the touch floor,
  //     so `py-2` around it made the row 8 + 44 + 8 = 60 against a
  //     directory row's 44. A listing draws its directories first, so
  //     the list's rhythm broke at the first file and stayed broken.
  //   - **gutter.** The directory row's `pr-2` against everything
  //     else's `px-4`: a pin sat 8 from the edge while the 拿过来 in
  //     the row below it sat 16, so the right edge of the list stepped
  //     sideways at the same seam.
  //
  // **The criterion is sameness, not a number.** 16 is what this pane
  // happens to use and this file is not where that is decided — a gate
  // that asserted 16 would be a second copy of `px-4` living in
  // `scripts/`, red the day somebody retunes the pane on purpose. What
  // may not happen is one row in a listing being laid out unlike the
  // row above it.
  //
  // **Both kinds of row have to be on screen or this judges nothing** —
  // comparing directories to directories is satisfied by any listing at
  // all, which is the「a shallower fixture quietly narrowed the claim」
  // shape. Asserted, because a fixture is a thing somebody edits.
  //
  // **What it does not block**: a row that is internally ragged (the
  // size, the date and the verb drifting apart inside a row whose two
  // edges are right), and a listing whose rows agree with each other
  // and with nothing else on the face.
  await atCrumbs();
  const listing = await page.evaluate(() => {
    const px = (n) => Math.round(n * 10) / 10;
    const rows = [];
    for (const el of document.querySelectorAll("[data-up],[data-dir],[data-file]")) {
      // The directory row is the wrapper that holds the name *and* the
      // pin; the other two kinds are the row themselves.
      const row = el.hasAttribute("data-dir") ? el.parentElement : el;
      const r = row.getBoundingClientRect();
      // Left is where the row's first word starts, not where its first
      // box does: on a directory row the box is a full-width button and
      // the padding that matters is inside it.
      const leaf = [...row.querySelectorAll("*")].find(
        (e) => e.children.length === 0 && e.textContent.trim(),
      );
      let left;
      if (leaf) left = px(leaf.getBoundingClientRect().left - r.left);
      else {
        const range = document.createRange();
        range.selectNodeContents(row);
        left = px(range.getBoundingClientRect().left - r.left);
      }
      // Right is where the row's last control ends — or, on a row with
      // nothing on its right at all, the row's own padding. Both are
      // 「how much space is left over there」, which is the thing a
      // reader's eye follows down the edge of the list.
      const kids = [...row.children].filter((c) => c.getClientRects().length);
      const right = kids.length
        ? px(r.right - Math.max(...kids.map((c) => c.getBoundingClientRect().right)))
        : px(parseFloat(getComputedStyle(row).paddingRight));
      rows.push({
        what: el.getAttribute("data-dir") ?? el.getAttribute("data-file") ?? "上一级",
        kind: el.hasAttribute("data-dir") ? "dir" : el.hasAttribute("data-file") ? "file" : "up",
        h: px(r.height),
        left,
        right,
      });
    }
    return rows;
  });
  console.log(
    `listing: ${listing.map((r) => `${r.kind} ${JSON.stringify(r.what)} h${r.h} ⊢${r.left} ${r.right}⊣`).join(", ")}`,
  );
  const kinds = new Set(listing.map((r) => r.kind));
  if (!kinds.has("dir") || !kinds.has("file")) {
    fail(
      `probe void: the listing on screen has ${[...kinds].join("+") || "no"} rows — this section compares a ` +
        `directory row against a file row and cannot do it with one kind (deepen the **/ls fixture)`,
    );
  } else {
    for (const [axis, of] of [
      ["height", (r) => r.h],
      ["left gutter", (r) => r.left],
      ["right gutter", (r) => r.right],
    ]) {
      const seen = listing.map(of);
      // Sub-pixel is text metrics; this bug is whole steps of padding.
      if (Math.max(...seen) - Math.min(...seen) > 0.5) {
        fail(
          `the rows of one listing disagree on ${axis}: ` +
            listing.map((r) => `${r.kind} ${JSON.stringify(r.what)} ${of(r)}`).join(", ") +
            ` — one listing is one rhythm, whatever the number is (src/views/FilesPane.tsx)`,
        );
      }
    }
  }

  // ── 5) a dense screen has a reading order ─────────────────────────
  // **What sections 2 and 3 cannot see.** Section 2 asks whether the
  // three weights exist *somewhere on the page*, which the rail and the
  // list satisfy on their own — so the mark's place could hold 537
  // pieces of text with 535 of them at one size, and this gate stayed
  // green while a person said「一堆字表达什么不清楚,看着费劲」about the
  // exact screen it had just passed. That is the family this repo calls
  // 假绿: the assertion held, and it held somewhere else.
  //
  // So the claim is per surface and it is about size, the axis a wall of
  // text collapses on: **past a couple of dozen lines, a pane must show
  // more than one size, and its largest must be a clear step above the
  // size most of it is set in.** Under that count a pane is small enough
  // to take in whole and a single size is a legitimate choice.
  //
  // What it cannot block: a *wrong* order — a date bigger than the
  // number under it passes. It blocks the flat one, which is the shape
  // that was actually shipped.
  await page.setViewportSize({ width: 1120, height: 1000 });
  await page.locator("[data-rail-mark]").click();
  await wait(900);
  const DENSE = 24;
  const STEP = 1.25;
  const DOMINANT = 0.85;
  // **Measured per half, not on the place as a whole — and that is not
  // the first thing this section tried.** The first version measured
  // `[data-mark-place]` in one piece and stayed green on a deliberately
  // flattened ledger: thirty of thirty-one lines at one size, saved by a
  // single machine name in the *other* half reading larger. A wall does
  // not stop being a wall because something else on the screen is big.
  // The halves are found structurally rather than by name, so a pane
  // added later is measured without anyone remembering to list it — and
  // if the structure moves out from under this, the probe finds nothing
  // and says so instead of passing.
  const regions = await page.evaluate(
    ({ DENSE }) => {
      const root = document.querySelector("[data-mark-place]");
      if (!root) return null;
      const of = (el) => {
        const leaves = [...el.querySelectorAll("*")].filter(
          (e) => e.children.length === 0 && e.textContent.trim(),
        );
        const sizes = {};
        for (const e of leaves) {
          const px = parseFloat(getComputedStyle(e).fontSize);
          sizes[px] = (sizes[px] || 0) + 1;
        }
        return { leaves: leaves.length, sizes };
      };
      const halves = [...root.querySelectorAll(":scope > div > div")];
      return {
        whole: of(root),
        halves: halves.map((h, i) => ({ name: `half ${i}`, ...of(h) })),
        DENSE,
      };
    },
    { DENSE },
  );
  if (!regions) fail("probe dead: the mark's place never drew, so nothing below measured it");
  else if (regions.whole.leaves === 0) fail("probe dead: the mark's place drew no text at all");
  else if (regions.halves.length === 0) {
    fail("probe dead: the mark's place has no halves to measure — its structure moved");
  } else {
    for (const r of [{ name: "the mark's place", ...regions.whole }, ...regions.halves]) {
      const entries = Object.entries(r.sizes).map(([px, n]) => [Number(px), n]);
      const [modal, modalCount] = entries.slice().sort((a, b) => b[1] - a[1])[0];
      const largest = Math.max(...entries.map(([px]) => px));
      const share = modalCount / r.leaves;
      // **The line says whether it judged.** Until 2026-08-28 a region
      // under the threshold printed a line identical in shape to one
      // that had been judged and passed, and `continue`d — so a screen
      // that had shrunk out of this section's scope read, in the output
      // and in a reader's head, as a screen this section was holding.
      // The home screen did exactly that the day its ledger came off
      // (38 text leaves down to 15): the assertion did not break, the
      // surface stopped being the kind of surface it is about. Same
      // family as an empty `querySelectorAll` reading as「this does not
      // exist」— a silence that looks like an answer.
      const judged = r.leaves > DENSE;
      console.log(
        `density ${r.name}: ${r.leaves} leaves, ${JSON.stringify(r.sizes)}, modal ${modal} at ${Math.round(share * 100)}%, largest ${largest}` +
          (judged ? " — judged" : ` — NOT judged: ${r.leaves} leaves is under the ${DENSE} this section is about`),
      );
      if (!judged) continue;
      if (share > DOMINANT) {
        fail(
          `${r.name}: ${modalCount} of ${r.leaves} lines at ${modal} (${Math.round(share * 100)}%) — a wall, whatever else is on the screen`,
        );
      }
      if (largest < modal * STEP) {
        fail(
          `${r.name}: largest ${largest} against a modal ${modal} — under a ${STEP}× step nothing leads`,
        );
      }
    }
  }

  // 7) no machine surface draws the subscription's arcs — either card,
  //    this machine's or a peer's, on the devices landing or in the
  //    mark's place. They were on this machine's cards from 2026-08-28
  //    (「额度都是机器卡里的」) until 2026-09-02, when 「机器页面不需要
  //    有token usage的环」 took them off; the 额度 landing's foot is
  //    their one home. **Section 8 below proves the finder**: the same
  //    `[data-quota-window]` selector counts two windows there in this
  //    very run, so a zero here is an absence and not a renamed marker
  //    (the `.nav-badge` family's antidote). The `**/quota` route at
  //    the head of this file stays — it feeds the band section 8
  //    measures.
  const arcsOn = async (what) => {
    await until(5_000, async () => (await page.locator(what).count()) > 0);
    await wait(300);
    return await page.evaluate((sel) => {
      const card = document.querySelector(sel);
      if (!card) return null;
      return {
        windows: card.querySelectorAll("[data-quota-window]").length,
        via: card.querySelectorAll("[data-quota-via]").length,
      };
    }, what);
  };
  await page.setViewportSize({ width: 1120, height: 1000 });
  await open("devices");
  await press("[data-list] [data-row-open]");
  const mineCard = await arcsOn("[data-machine-card]");
  if (!mineCard) fail("probe dead: this machine's card never drew, so nothing below measured it");
  else if ((await page.locator("[data-machine-card] [data-hooks]").count()) !== 1) {
    fail("probe void: the card measured is not this machine's — the row order moved");
  } else if (mineCard.windows !== 0 || mineCard.via !== 0) {
    fail(`this machine's card draws ${mineCard.windows} quota windows — they left this surface on 2026-09-02`);
  }
  await open("devices");
  await pressNth("[data-list] [data-row-open]", 1);
  const peerCard = await arcsOn("[data-machine-card]");
  if (!peerCard) fail("probe dead: the peer's card never drew");
  else if ((await page.locator("[data-machine-card] [data-vitals]").count()) !== 1) {
    fail("probe void: the peer's card drew no readings, so its empty quota proves nothing");
  } else if (peerCard.windows !== 0) {
    fail(`a peer's card draws ${peerCard.windows} quota windows`);
  }
  await page.locator("[data-rail-mark]").click();
  await wait(400);
  await page.locator('[data-seat][data-seat-me="true"]').click();
  const mineNode = await arcsOn("[data-node-card]");
  if (!mineNode) fail("probe dead: this machine's node card never opened");
  else if (mineNode.windows !== 0) {
    fail(`the home node card draws ${mineNode.windows} quota windows`);
  }
  await page.keyboard.press("Escape");
  await wait(300);
  await page.locator('[data-seat][data-seat-me="false"]').first().click();
  const peerNode = await arcsOn("[data-node-card]");
  if (!peerNode) fail("probe dead: the peer's node card never opened");
  else if (peerNode.windows !== 0) {
    fail(`a peer's node card draws ${peerNode.windows} quota windows`);
  }
  await page.keyboard.press("Escape");

  // ── 8) 额度: the three shapes of a reading, and the band ──────────
  // The fixture routes three accounts (`board` above): a fraction, a
  // spend with no denominator, a refusal carried from a peer. Every
  // claim is tied to that fixture — an implementation that draws
  // nothing goes red on each, and a changed fixture must change them
  // (the repo's falsifiability rule).
  await page.setViewportSize({ width: 1400, height: 900 });
  await press(rail("quota"));
  await until(5_000, async () => (await page.locator("[data-quota-row]").count()) > 0);
  const quotaRows = await page.locator("[data-quota-row]").count();
  if (quotaRows !== board.length) {
    fail(`\u989d\u5ea6 draws ${quotaRows} rows for ${board.length} routed accounts`);
  }
  const fillRow = await page.locator('[data-quota-row="acc-fill"]').textContent();
  if (!fillRow?.includes("45%")) {
    fail(`the fraction row does not say its share — got ${JSON.stringify(fillRow)}`);
  }
  // The refused account: the trouble on the glass is a sentence, not
  // the wire key. Clicking the row is also the finder proving itself —
  // the detail draws the trouble only for the picked account.
  await press('[data-quota-row="acc-trouble"]');
  const troubleSaid = await page
    .locator('[data-quota-detail="acc-trouble"] [data-quota-trouble="key-refused"]')
    .textContent()
    .catch(() => null);
  if (!troubleSaid || !troubleSaid.trim() || troubleSaid.includes("key-refused")) {
    fail(`the refused account's trouble is not a person's sentence — got ${JSON.stringify(troubleSaid)}`);
  }
  // …and a peer's reading names its reader with an age.
  const readBy = await page
    .locator('[data-quota-detail="acc-trouble"] [data-quota-age]')
    .textContent();
  if (!readBy?.includes("peer")) {
    fail(`a peer's reading does not name its reader — got ${JSON.stringify(readBy)}`);
  }
  // The subscription band at the landing's foot — the one surface that
  // draws these two windows since 2026-09-02, and the positive half
  // section 7's absences lean on: same selector, same run.
  const bandWindows = await page.locator("[data-quota-sub] [data-quota-window]").count();
  if (bandWindows !== 2) {
    fail(`the \u989d\u5ea6 foot draws ${bandWindows} subscription windows for a routed two-window quota`);
  }

  // ── 8b) narrow: the open sessions are a count and a switcher ──────
  // The wide strip's other half (`TabStrip`'s head): a phone shows one
  // screen, so the strip cannot be there, and the count is the way to
  // every other tab. Mechanics only — tabs are pure front-end (state
  // and localStorage) — so the mock is not asked anything it can lie
  // about.
  await page.evaluate(() => window.localStorage.removeItem("khor.sessions.tabs"));
  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload({ waitUntil: "load" });
  await wait(900);
  await open("sessions");
  await press("[data-list] [data-row-open]");
  if ((await page.locator("[data-back]").count()) === 0) {
    fail("probe dead: the narrow session detail never drew, so the tab count was never on a screen");
  } else {
    await press("[data-back]");
    await pressNth("[data-list] [data-row-open]", 1);
    const narrowCount = page.locator("[data-detail-header] [data-tab-count]");
    if ((await narrowCount.count()) !== 1) {
      fail("no tab count on the narrow session detail — the only way to the other open tab is missing");
    } else {
      const said = Number((await narrowCount.innerText()).trim());
      if (said !== 2) fail(`two rows were opened and the count says ${said}`);
      await narrowCount.click();
      await wait(400);
      const listed = await page.locator("[data-tab-switcher] [data-switch-tab]").count();
      if (listed !== 2) fail(`the switcher lists ${listed} of 2 open tabs`);
      // Switching really switches: pick the tab that is not in front
      // and a session detail comes back.
      const other = await page
        .locator('[data-tab-switcher] [data-switch-tab][data-on="false"] [data-switch-open]')
        .first()
        .getAttribute("data-switch-open");
      await press(`[data-switch-open="${other}"]`);
      if ((await page.locator("[data-tab-switcher]").count()) !== 0) {
        fail("picking a tab did not put the switcher away");
      }
      if ((await page.locator("[data-detail-header] [data-halo]").count()) !== 1) {
        fail("the switcher's pick did not land on a session detail");
      }
      // Closing from inside: the switcher stays up (it is the screen
      // you are on — `TabStrip`'s own judgment) and the survivor stays.
      await narrowCount.click();
      await wait(400);
      await press(`[data-switch-close="${other}"]`);
      const left = await page.locator("[data-tab-switcher] [data-switch-tab]").count();
      if (left !== 1) fail(`closing one of two tabs left ${left} rows in the switcher`);
    }
  }

  // ── 9) readings per volume and per card, with a whole fallback ────
  // Tied to the fixture: the peer surveys (two cards, two volumes, one
  // minted strain), elder does not (empty lists, aggregate only). An
  // implementation that ignores the lists reds the first half; one
  // that forgets the fallback — the un-upgraded reporter every mesh
  // has — reds the second. Both cards are opened by row order, the
  // walk's own convention.
  await page.setViewportSize({ width: 1400, height: 900 });
  await open("devices");
  await pressNth("[data-list] [data-row-open]", 1);
  await until(5_000, async () => (await page.locator("[data-machine-card] [data-vitals]").count()) === 1);
  const surveyed = await page.evaluate(() => ({
    vols: [...document.querySelectorAll('[data-machine-card] [data-vitals-unit="vol"]')].length,
    gpus: [...document.querySelectorAll('[data-machine-card] [data-vitals-unit="gpu"]')].length,
    vram: document.querySelectorAll('[data-machine-card] [data-vitals-unit="vram"]').length,
    disk: document.querySelectorAll('[data-machine-card] [data-vitals-unit="disk"]').length,
    strained: document.querySelectorAll('[data-machine-card] [data-vitals-detail][data-strain="tight"]').length,
  }));
  if (surveyed.vols !== 2) fail(`the surveyed card draws ${surveyed.vols} volume rings for 2 routed volumes`);
  if (surveyed.gpus !== 2) fail(`the surveyed card draws ${surveyed.gpus} GPU rings for 2 routed cards`);
  if (surveyed.vram !== 0) fail("a per-card row plus an aggregate VRAM ring is the same bytes twice");
  if (surveyed.disk !== 0) fail("the single-disk ring is still drawn beside the volumes it became");
  if (surveyed.strained !== 1) fail(`the minted volume strain did not land (${surveyed.strained} tight details)`);
  await open("devices");
  await pressNth("[data-list] [data-row-open]", 2);
  await until(5_000, async () => (await page.locator("[data-machine-card] [data-vitals]").count()) === 1);
  const elder = await page.evaluate(() => ({
    vols: document.querySelectorAll('[data-machine-card] [data-vitals-unit="vol"]').length,
    gpus: document.querySelectorAll('[data-machine-card] [data-vitals-unit="gpu"]').length,
    vram: document.querySelectorAll('[data-machine-card] [data-vitals-unit="vram"]').length,
    disk: document.querySelectorAll('[data-machine-card] [data-vitals-unit="disk"]').length,
  }));
  if (elder.disk !== 1 || elder.gpus !== 1 || elder.vram !== 1 || elder.vols !== 0) {
    fail(
      `the un-surveyed card lost its old rings: disk ${elder.disk}, gpu ${elder.gpus}, ` +
        `vram ${elder.vram}, vols ${elder.vols} — the fallback is the un-upgraded mesh's face`,
    );
  }

  // ── 10) the top and the CPU day under the rings, and the refusal ──
  // Tied to the fixture the other way round from 9: the peer answers
  // (three rows, a day of 288 with holes where nobody measured), elder
  // refuses with a sentence. An implementation that draws nothing reds
  // the first half; one that bridges or zeroes the holes reds the gap
  // count; one that swallows the refusal into an empty table — the
  // idle-machine lie — reds the second half, where the table must be
  // absent and the sentence present.
  await open("devices");
  await pressNth("[data-list] [data-row-open]", 1);
  await until(5_000, async () => (await page.locator("[data-machine-card] [data-load-top]").count()) === 1);
  const busy = await page.evaluate(() => ({
    rows: document.querySelectorAll("[data-machine-card] [data-top-row]").length,
    dayRows: [...document.querySelectorAll("[data-machine-card] [data-day-row]")].map((r) => ({
      name: r.getAttribute("data-day-row"),
      cells: r.querySelectorAll("[data-day-cell]").length,
      gaps: r.querySelectorAll("[data-day-cell][data-gap]").length,
      barsInGaps: r.querySelectorAll("[data-day-cell][data-gap] > *").length,
      bars: r.querySelectorAll("[data-day-cell]:not([data-gap]) > *").length,
      peak: r.querySelector("[data-day-peak]")?.textContent?.trim() ?? "",
    })),
    why: document.querySelectorAll("[data-machine-card] [data-load-why]").length,
    //   The age column, cell by cell. **The row the fixture gave no age
    //   must be empty** — a `0 秒` there would be this table naming the
    //   oldest kind of process on a machine as its newest.
    ages: [...document.querySelectorAll("[data-machine-card] [data-top-time]")].map((td) =>
      td.textContent.trim(),
    ),
  }));
  if (busy.rows !== load.top.length) fail(`the peer's top draws ${busy.rows} rows for ${load.top.length} routed`);
  //     Written against the fixture rather than as literals, so a
  //     changed fixture cannot leave this asserting yesterday's numbers.
  const withAge = load.top.filter((r) => r.run_secs !== null).length;
  const drawn = busy.ages.filter((t) => t !== "").length;
  if (busy.ages.length !== load.top.length) {
    fail(`the age column has ${busy.ages.length} cells for ${load.top.length} rows`);
  }
  if (drawn !== withAge) {
    fail(`${drawn} ages drawn for ${withAge} rows that carry one — an absent age must draw nothing, not 0`);
  }
  if (busy.ages.some((t) => /^0\s/.test(t))) {
    fail(`an age drew as a zero (${JSON.stringify(busy.ages)}) — 「没说」 became 「刚起的」`);
  }
  //     **The identifying word survives the narrow card**, and this is
  //     the one posture where it was lost. The account is dropped
  //     narrow because every row of one person's own machine says the
  //     same word; the word off the command line is dropped by the same
  //     reflex and must not be, because it is different on every row —
  //     which is the entire reason the column exists. Measured
  //     2026-09-04: a 390 card showed three rows all reading `python3`.
  const withArg = load.top.filter((r) => r.arg !== null).length;
  //     Counted on the cell that carries it. It rode behind the
  //     process name once and is a column of its own now; what is
  //     asserted did not change — **the word that tells two identical
  //     rows apart is on screen at 390 too**, which is where the rows
  //     are hardest to read and where it was dropped before.
  const argsAt = async () =>
    page.evaluate(
      () =>
        [...document.querySelectorAll('[data-machine-card] [data-top-row] [data-c="cmd"]')].filter(
          (c) => c.textContent.trim().length > 0,
        ).length,
    );
  const wideArgs = await argsAt();
  await page.setViewportSize({ width: 390, height: 844 });
  await new Promise((r) => setTimeout(r, 400));
  const narrowArgs = await argsAt();
  await page.setViewportSize({ width: 1120, height: 1000 });
  await new Promise((r) => setTimeout(r, 400));
  if (wideArgs !== withArg) fail(`the wide card draws ${wideArgs} identifying words for ${withArg} rows that carry one`);
  if (narrowArgs !== withArg) {
    fail(
      `the narrow card draws ${narrowArgs} identifying words for ${withArg} rows that carry one — ` +
        `on 390 the rows collapse to their short names, which is the case this column exists for`,
    );
  }

  //     ── while an answer is out, nothing on the table moves ────────
  //
  //     **The bug this replaces was a re-sort wearing a ranking's
  //     clothes** (measured 2026-09-04 on turing): pressing a column
  //     re-ordered the rows already on screen while the true answer was
  //     still crossing, so the table showed the busiest 200 sorted by
  //     pid — `[10, 11, 25, 31, 43, 386, 410, 422]` against the
  //     machine's `[1, 2, 3, 4, 6, 9, …]` — and became the real thing a
  //     poll later. Worse than a blank: a blank says 「还没好」, this
  //     says 「好了」 and is wrong, looking exactly like a true answer.
  //
  //     So: the rows keep the order of the answer that produced them,
  //     `aria-sort` does not promise an order that has not arrived, and
  //     the pressed head is the only thing that moves.
  const pidsNow = () =>
    page.locator("[data-machine-card] [data-top-row]").evaluateAll((els) => els.map((e) => e.dataset.topRow));
  const sortStateNow = () =>
    page.evaluate(() => ({
      sorted: [...document.querySelectorAll("[data-machine-card] [data-top-head]")]
        .filter((th) => th.getAttribute("aria-sort") !== "none")
        .map((th) => th.dataset.topHead + ":" + th.getAttribute("aria-sort")),
      asking: [...document.querySelectorAll("[data-machine-card] [data-top-head][data-asking]")].map(
        (th) => th.dataset.topHead,
      ),
    }));
  const before = { pids: await pidsNow(), state: await sortStateNow() };
  held.on = true;
  await page.locator('[data-machine-card] [data-top-head="mem"] button').click();
  await until(5_000, async () => held.seen > 0);
  await new Promise((r) => setTimeout(r, 500));
  const during = { pids: await pidsNow(), state: await sortStateNow() };
  if (during.pids.join(",") !== before.pids.join(",")) {
    fail(
      `the rows re-sorted while the answer was still out: ${before.pids.join(",")} became ${during.pids.join(",")} ` +
        `— that is a re-ranking of the old set wearing the new ranking's clothes`,
    );
  }
  if (during.state.sorted.join(",") !== before.state.sorted.join(",")) {
    fail(
      `aria-sort promised ${during.state.sorted.join(",")} while the rows were still ${before.state.sorted.join(",")} ` +
        `— to a reader who cannot see the table that promise is the whole answer`,
    );
  }
  if (during.state.asking.join(",") !== "mem") {
    fail(`the pressed head does not say it is waiting (asking=[${during.state.asking.join(",")}])`);
  }
  held.release?.();
  held.on = false;
  const landed = await until(8_000, async () => (await pidsNow()).join(",") !== before.pids.join(","));
  if (!landed) fail("the held answer never replaced the rows, so the assertion above was never tested against a change");
  const after = await sortStateNow();
  if (!after.sorted.includes("mem:descending")) fail(`the answer landed but aria-sort still says ${after.sorted.join(",")}`);
  if (after.asking.length !== 0) fail(`a head is still marked waiting after the answer landed: ${after.asking.join(",")}`);
  const peerGpus = devs.find((d) => d.name === "peer").vitals.vitals.gpus;
  //     ── nothing overflows sideways, at any width ─────────────────
  //
  //     **Worth more than any single assertion here**, because what it
  //     guards is not today's columns but every column anyone adds
  //     later. Stepped at 10px and not at 20: a 19px-wide overflow band
  //     was measured on the earlier single-panel draft (a breakpoint
  //     written at 560 while the one-line layout stopped fitting at
  //     580), and 20px steps walked straight over it — two-point
  //     checking at 1100 and 390 never had a chance.
  //
  //     One page, only the viewport changes; a context per step costs
  //     four times as long and measures the same thing.
  //
  //     **The whole card, including the process table.** The table
  //     used to be excused here — at 390 its seven columns wanted 456
  //     in a 332 box — and the band was printed rather than asserted so
  //     it could not be forgotten. The eight-column table re-lays
  //     itself as stacked rows when its own box is too narrow, so the
  //     excuse is gone and this is now the guarantee the whole card
  //     makes at every width it is drawn for.
  {
    const wide = { width: 1120, height: 1000 };
    const bad = [];
    // **From 390, the narrowest width this app is drawn for** — the
    // phone the narrow face targets, and the width every other stop in
    // this file uses. At 380 the shell itself is 3px over (measured:
    // the boxes involved are the screen, the card and its header, each
    // exactly 380 wide sitting one pixel in — no drawer or part row
    // among them), which is a shell question and not this batch's.
    // Printed below rather than dropped.
    for (let w = 390; w <= 1200; w += 10) {
      await page.setViewportSize({ width: w, height: 800 });
      // **Settled, and settled on purpose.** The drawer decides its own
      // layout from its own width through a ResizeObserver, so the
      // frame right after a resize is measured against the previous
      // width — a real state, lasting one frame, that no human resizing
      // a window can see. Two animation frames is the browser's own
      // definition of "the layout after this change"; measuring before
      // it reports the layout before the change.
      await page.evaluate(
        () => new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(() => ok(null)))),
      );
      await wait(45);
      const read = () =>
        page.evaluate(() => {
          const de = document.documentElement;
          const out = { doc: de.scrollWidth - de.clientWidth, drawer: 0, table: 0 };
          // A box carrying `overflow-x: hidden` reports nothing and
          // turns this into a check that cannot fail — none of these do.
          for (const n of document.querySelectorAll("[data-drawer],[data-drawer-scroll],[data-load-top-box]")) {
            out.drawer = Math.max(out.drawer, n.scrollWidth - n.clientWidth);
          }
          return out;
        });
      // **Read twice, and count only what survives both.** A resize
      // leaves one frame measured against the old width, and the
      // readings at the narrow end moved run to run (3, then 1, then 5)
      // — a quantity that flaps makes a gate that flaps, which is worse
      // than no gate. A structural band is there in both reads; a
      // settle transient is not. This is a discriminating test rather
      // than a threshold picked to be quiet.
      const first = await read();
      await wait(90);
      const over = await read();
      over.doc = Math.min(first.doc, over.doc);
      over.drawer = Math.min(first.drawer, over.drawer);
      over.table = Math.min(first.table, over.table);
      if (over.doc > 0) bad.push(`${w}:page+${over.doc}`);
      if (over.drawer > 0) bad.push(`${w}:drawer+${over.drawer}`);

    }
    await page.setViewportSize(wide);
    await wait(200);
    if (bad.length) fail(`sideways overflow at ${bad.length} widths: ${bad.slice(0, 8).join(" ")}`);
    await page.setViewportSize({ width: 380, height: 800 });
    await wait(120);
    const at380 = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    if (at380 > 0) console.log(`note: at 380 the shell is ${at380}px over — below the 390 this face is drawn for`);
    console.log("bandwidth 390→1200 by 10: page, drawer and process table all clean");
  }

  //     ── the dial is a 210° face, and the rim carries the parts ───
  //
  //     **This is the assertion D lives or dies by.** The dial exists
  //     because a full ring cannot tell 「one card pinned, three idle」
  //     from 「four cards half busy」 — the summary arc is identical in
  //     both, and the person deciding where to run a job needs the idle
  //     card. Without something holding the shape, a later hand puts a
  //     plain circle back and nothing anywhere goes red.
  const dial = await page.evaluate(() => {
    const card = document.querySelector("[data-machine-card]");
    const svgs = [...card.querySelectorAll("[data-dial-face]")];
    // A gauge drawn as a full circle would be `<circle>` with a radius
    // the size of the dial; the milestone dots are r=0.9.
    const bigCircles = [...card.querySelectorAll("circle")].filter(
      (c) => Number(c.getAttribute("r")) > 20,
    ).length;
    const pipsOn = (unit) => {
      // **By `data-dial`, not by `data-vitals-unit`.** That one is
      // also on the per-card rows under the dials, and it is absent
      // entirely on two of the four dials — a selector built on it
      // finds the wrong element, or none, and then answers "no marks"
      // about a dial covered in them (which is what it did first).
      const box = card.querySelector(`[data-metric][data-dial="${unit}"]`);
      const face = box?.querySelector("[data-dial-face]");
      if (!face) return null;
      return {
        pips: [...face.querySelectorAll("[data-dial-pip]")].map((p) => Number(p.dataset.dialPipAt)),
        span: face.querySelectorAll("[data-dial-span]").length,
        arc: face.querySelectorAll("[data-ring-fill]").length,
      };
    };
    return { faces: svgs.length, bigCircles, gpu: pipsOn("gpu"), cpu: pipsOn("cpu"), mem: pipsOn("mem") };
  });
  if (dial.faces === 0) fail("probe dead: no dial face on this card, so nothing below measured a dial");
  else {
    //   A full circle anywhere on this card is the shape coming back.
    if (dial.bigCircles !== 0) {
      fail(`${dial.bigCircles} full circles on the machine card — the 210° dial was replaced by a ring again`);
    }
    //   **Marks belong to parts, not to occupants.** The processes under
    //   CPU and memory do not partition those readings, and a mark for
    //   one would claim a meaning the drawing does not have.
    if (dial.cpu && dial.cpu.pips.length !== 0) fail(`the CPU dial carries ${dial.cpu.pips.length} rim marks — CPU has no parts`);
    if (dial.mem && dial.mem.pips.length !== 0) fail(`the memory dial carries ${dial.mem.pips.length} rim marks`);
    //   The GPU dial's marks are the cards' own readings, one each.
    const want = peerGpus.map((g) => Math.round(g.util_pct)).sort((a, b) => a - b);
    const got = (dial.gpu?.pips ?? []).slice().sort((a, b) => a - b);
    if (got.join(",") !== want.join(",")) {
      fail(`the GPU dial marks sit at [${got.join(",")}] for cards reading [${want.join(",")}]`);
    }
    if (!dial.gpu) fail("probe dead: no GPU dial found, so the mark assertions above measured nothing");
    else if (want.length > 1 && dial.gpu.span !== 1) {
      fail("no hairline spanning the marks — 「spread out or not」 has to have a length that can be measured");
    }
    else if (dial.gpu.arc !== 1) fail(`the GPU dial draws ${dial.gpu.arc} summary arcs`);
  }

  //     ── three readings of one day, and one cursor across them ────
  //
  //     **The moment survives the switch.** Three readings of the same
  //     numbers is the claim; three components each remembering their
  //     own state is what it would be instead, and the difference shows
  //     exactly here — put the cursor somewhere, change the reading,
  //     and the readout must say the same thing word for word.
  const readings = await page.evaluate(async () => {
    const pick = (k) => document.querySelector(`[data-machine-card] [data-day-view-pick="${k}"]`);
    const band = () => document.querySelector("[data-machine-card] [data-day-band]");
    const readout = () => document.querySelector("[data-machine-card] [data-day-at]")?.innerText.trim() ?? null;
    const words = [...document.querySelectorAll("[data-machine-card] [data-day-view-pick]")].map(
      (b) => b.dataset.dayViewPick,
    );
    if (words.length !== 3) return { words };
    // Park the cursor a third of the way along the first band.
    const b = band().getBoundingClientRect();
    const put = () =>
      band().dispatchEvent(
        new PointerEvent("pointermove", {
          bubbles: true,
          clientX: b.left + b.width / 3,
          clientY: b.top + b.height / 2,
        }),
      );
    put();
    await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    const onHeat = readout();
    pick("ridge").click();
    await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    const onRidge = readout();
    const ridgeViews = [...document.querySelectorAll("[data-machine-card] [data-day-view]")].map(
      (n) => n.dataset.dayView,
    );
    pick("heat").click();
    await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    return { words, onHeat, onRidge, ridgeViews };
  });
  if (readings.words.length !== 3) fail(`the day offers ${readings.words.length} readings, not three`);
  else if (!readings.onHeat) fail("the cursor drew no readout — 「which moment」 is what the switch has to carry");
  else {
    if (readings.onHeat !== readings.onRidge) {
      fail(`the moment did not survive the switch: "${readings.onHeat}" became "${readings.onRidge}"`);
    }
    if (readings.ridgeViews.some((v) => v !== "ridge")) {
      fail(`picking 山脊 left bands reading ${[...new Set(readings.ridgeViews)].join(",")}`);
    }
  }

  //     ── the scatter: one pair of axes, and three states kept apart
  //
  //     **Card memory is an area, and the two biggest must be told
  //     apart.** Radius by square root means area follows bytes; the
  //     foot prints both readings and both radii so the claim is
  //     checkable on the page rather than taken on faith. And the three
  //     states are three appearances on purpose: a halo (there is card
  //     memory), a dashed ring (looked, it is zero), nothing at all
  //     (could not look) — merging the last two would turn 「did not
  //     look」 into 「is idle」, which is the neighbouring-value mistake
  //     this whole surface is shaped to avoid.
  const plot = await page.evaluate(() => {
    const box = document.querySelector("[data-machine-card] [data-top-plot]");
    if (!box) return null;
    const halos = [...box.querySelectorAll("[data-plot-halo]")].map((n) => Number(n.dataset.plotHalo));
    return {
      points: box.querySelectorAll("[data-plot-point]").length,
      halos,
      zeros: box.querySelectorAll("[data-plot-zero]").length,
      foot: box.querySelector("[data-plot-foot]")?.textContent?.trim() ?? "",
      axes: box.textContent.includes("CPU") && box.textContent.includes("内存"),
    };
  });
  if (!plot) fail("no scatter on the card — every process was to be on the picture");
  else {
    if (plot.points !== load.top.length) fail(`the scatter draws ${plot.points} points for ${load.top.length} rows`);
    if (!plot.axes) fail(`the scatter does not name its axes: "${plot.foot}"`);
    //   The fixture has one row with card memory, one with none — so
    //   the halo count is a fact about the data, not about the chart.
    const withVram = load.top.filter((r) => r.gpu_mem !== null && r.gpu_mem > 0).length;
    if (plot.halos.length !== withVram) {
      fail(`${plot.halos.length} halos for ${withVram} rows holding card memory`);
    }
    //   **Area, not radius.** Two readings a factor of four apart must
    //   come out a factor of two apart in radius; a chart that scaled
    //   the radius directly would show four.
    //   **The points are actually separated.** A linear axis on these
    //   readings puts almost everything in one corner — measured on the
    //   live fleet, the median process is at 0.0% / 0.2% / 1.3% of a
    //   core against maxima of 18 / 43 / 89 — so 「how many pairs sit
    //   on top of each other」 is the thing the scale has to move.
    //
    //   ⚠️ **A picture with a handful of points passes this no matter
    //   what**, which is the assertion-that-cannot-fail again. Under
    //   eight points it is not evaluated and says so out loud rather
    //   than reporting a pass it did not earn.
    //
    //   **The threshold is half the points, measured here rather than
    //   carried over.** On this fixture at this width the root scale
    //   gives 8 crowded pairs and a linear one gives **57** — that 7×
    //   is the signal, and any line drawn between them catches the
    //   regression. The spec's own figure (a quarter of the points)
    //   came from a different plot width and a different set of rows,
    //   and on these it fails a scale that is working; **a number
    //   measured on one tree is not a number about another** — the
    //   same lesson the drawer's track width taught.
    const near = await page.evaluate(() => {
      const pts = [...document.querySelectorAll("[data-machine-card] [data-plot-point]")].map((g) => ({
        x: Number(g.dataset.plotX),
        y: Number(g.dataset.plotY),
      }));
      let close = 0;
      for (let i = 0; i < pts.length; i++) {
        for (let j = i + 1; j < pts.length; j++) {
          if (Math.hypot(pts[i].x - pts[j].x, pts[i].y - pts[j].y) < 6) close += 1;
        }
      }
      return { n: pts.length, close };
    });
    if (near.n < 8) {
      console.log(`scatter: ${near.n} points — too few to judge crowding, that check was not evaluated`);
    } else if (near.close > near.n / 2) {
      fail(`${near.close} point pairs sit within 6px of each other on ${near.n} points — the axes are not spreading them`);
    } else {
      console.log(`scatter: ${near.close} crowded pairs on ${near.n} points (allowed ${Math.floor(near.n / 2)}; linear measures 57)`);
    }
    if (plot.halos.length > 1) {
      const sorted = [...plot.halos].sort((a, b) => b - a);
      const ratio = sorted[0] / sorted[sorted.length - 1];
      if (!(ratio > 1.05)) fail(`the halos are ${sorted.join(",")} — the biggest and smallest are the same size`);
    }
  }

  //     ── the process table: PID, and marks that were measured ─────
  //
  //     **PID is on every row at every width, and its ink starts at the
  //     panel's edge.** This is not a preference: on turing the two
  //     vLLM workers carry the same name *and* the same command line,
  //     so the only things that tell them apart are this number and
  //     their card memory. A layout that drops the column to save room
  //     drops the answer.
  //
  //     ⚠️ **Measured as ink, not as the cell.** A cell's box starts
  //     where its track starts, whatever the text inside is doing — the
  //     real incident this guards against (the pid pushed 229px into
  //     the panel) was `text-right` on a wide track moving the *glyphs*
  //     while the box never moved. A box-based version of this
  //     assertion reads 0 at the scene of the accident.
  const pid = await page.evaluate(() => {
    const panel = document.querySelector("[data-machine-card] [data-panel]");
    if (!panel) return null;
    const rows = [...panel.querySelectorAll("[data-top-row]")];
    const cells = rows.map((r) => r.querySelector('[data-c="pid"]')).filter(Boolean);
    if (cells.length === 0) return { rows: rows.length, drawn: 0, left: null };
    const r = document.createRange();
    r.selectNodeContents(cells[0]);
    return {
      rows: rows.length,
      drawn: cells.length,
      left: +(r.getBoundingClientRect().left - panel.getBoundingClientRect().left).toFixed(1),
    };
  });
  if (!pid || pid.rows === 0) fail("probe dead: no process rows, so nothing below measured the table");
  else {
    if (pid.drawn !== pid.rows) fail(`${pid.drawn} of ${pid.rows} rows draw a PID`);
    if (pid.left === null || pid.left > 4) fail(`the PID's ink sits ${pid.left}px into the panel`);
  }
  //     **Every truncation mark was measured, and every overflow got
  //     one.** Both directions, because each is its own kind of lie: a
  //     mark on a cell that fits is decoration wearing a signal's
  //     clothes, and an overflow without one is a value that ended
  //     early with nothing saying so.
  const marks = await page.evaluate(() => {
    let decorative = 0;
    let silent = 0;
    let marked = 0;
    let cells = 0;
    for (const c of document.querySelectorAll('[data-machine-card] [data-top-row] [data-c="cmd"]')) {
      if (!c.firstElementChild || c.dataset.open !== undefined) continue;
      cells += 1;
      const inner = c.firstElementChild;
      const over = inner.scrollWidth - inner.clientWidth > 1;
      const has = c.dataset.cut !== undefined;
      if (has) marked += 1;
      if (has && !over) decorative += 1;
      if (!has && over) silent += 1;
    }
    return { cells, marked, decorative, silent };
  });
  //     **And at least one cell really did not fit.** With none, both
  //     assertions above are satisfied by a table that never marks
  //     anything — the fixture carries a command line long enough that
  //     the column cannot hold it.
  if (marks.marked === 0) fail(`no cell overflowed (${marks.cells} checked) — the two assertions below tested nothing`);
  if (marks.decorative !== 0) fail(`${marks.decorative} cells that fit carry a truncation mark — the mark is decoration`);
  if (marks.silent !== 0) fail(`${marks.silent} cells overflow with no mark — a value ended early and nothing said so`);
  console.log(`command line: ${marks.marked}/${marks.cells} cells marked, ${marks.decorative} decorative, ${marks.silent} silent`);
  //     **A mark can be opened, by pointer and by keyboard.** The
  //     narrow card has no hover at all, and narrow is where cells run
  //     out of room first.
  if (marks.marked > 0) {
    const grew = await page.evaluate(async () => {
      const c = document.querySelector('[data-machine-card] [data-c="cmd"][data-cut]');
      const h0 = c.getBoundingClientRect().height;
      c.click();
      await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
      const h1 = c.getBoundingClientRect().height;
      c.click();
      await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
      return { h0, h1, h2: c.getBoundingClientRect().height, role: c.getAttribute("role"), tab: c.getAttribute("tabindex") };
    });
    if (!(grew.h1 > grew.h0 && Math.abs(grew.h2 - grew.h0) < 1)) {
      fail(`pressing a cut cell went ${grew.h0} → ${grew.h1} → ${grew.h2} — it did not open and close`);
    }
    if (grew.role !== "button" || grew.tab !== "0") {
      fail(`a cut cell is role=${grew.role} tabindex=${grew.tab} — the keyboard cannot reach it`);
    }
    //   **And the table is still a table while it is open.** The first
    //   version made the open cell span every column, which pushed the
    //   rest of that row onto the next grid line and took every row
    //   below it with them — the whole table came apart, while the
    //   assertion above (「it grew, then it shrank」) stayed true. So:
    //   open one, and check the columns still line up.
    const whileOpen = await page.evaluate(async () => {
      const c = document.querySelector('[data-machine-card] [data-c="cmd"][data-cut]');
      c.click();
      await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
      const lefts = [
        ...new Set(
          [...document.querySelectorAll('[data-machine-card] [data-top-row] [data-c="pid"]')].map(
            (n) => +n.getBoundingClientRect().left.toFixed(1),
          ),
        ),
      ];
      c.click();
      await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
      return lefts;
    });
    if (whileOpen.length !== 1) {
      fail(`with a cell open the PID column has ${whileOpen.length} left edges (${whileOpen.slice(0, 4).join(",")}) — the grid came apart`);
    }
  }

  //     ── the drawer: one height, and three straight edges ─────────
  //
  //     **Pressing a different dial must not move the card.** A drawer
  //     that grows to its content makes everything below it jump while
  //     somebody is reading it.
  //
  //     ⚠️ Opened **by state, not by counting presses** — the press is a
  //     toggle, so pressing a dial that is already open closes it. The
  //     spec's author wrote this assertion wrong twice and got a red on
  //     a page that was correct; a red from a broken setup and a red
  //     from a real bug read exactly the same.
  const drawerHeights = [];
  for (const key of ["cpu", "gpu", "mem", "disk"]) {
    const dial = page.locator(`[data-machine-card] [data-dial="${key}"]`);
    if ((await dial.count()) === 0) continue;
    if ((await dial.getAttribute("data-open")) !== "true") await dial.click();
    await wait(120);
    const seen = await page.evaluate(() => {
      const drawer = document.querySelector("[data-machine-card] [data-drawer]");
      const open = drawer.querySelector("[data-parts]:not([hidden])");
      const head = drawer.querySelector("[data-drawer-head] [data-drawer-count]");
      return {
        h: Math.round(drawer.getBoundingClientRect().height),
        says: Number(head?.dataset.drawerCount ?? -1),
        rows: open ? open.querySelectorAll("[data-part]").length : -1,
      };
    });
    drawerHeights.push(`${key}:${seen.h}`);
    //   **Every group, not just whichever is open at the end.** The
    //   first version read only the last one the loop left showing, so
    //   a heading lying on one of the other three was never looked at —
    //   an assertion pointed at the wrong element passes for reasons
    //   that have nothing to do with what it claims.
    if (seen.says !== seen.rows) {
      fail(`the ${key} drawer's heading says ${seen.says} but draws ${seen.rows} rows — the count is about something else`);
    }
  }
  {
    const hs = new Set(drawerHeights.map((x) => x.split(":")[1]));
    if (drawerHeights.length < 2) fail(`probe dead: only ${drawerHeights.length} drawers could be opened`);
    else if (hs.size !== 1) fail(`the drawers are ${drawerHeights.join(" ")} — pressing around makes the card jump`);
  }
  //     **The heading names the reading and counts what is in the
  //     drawer.** Without it a block of six rows under four dials does
  //     not say which dial it belongs to, and the count is what tells a
  //     drawer of parts from a drawer of occupants at a glance.
  const head = await page.evaluate(() => {
    const h = document.querySelector("[data-machine-card] [data-drawer-head]");
    if (!h) return null;
    const open = document.querySelector("[data-machine-card] [data-parts]:not([hidden])");
    return {
      text: h.innerText.replace(/\s+/g, " ").trim(),
      glyphs: h.querySelectorAll("[data-ring-glyph]").length,
      // **What it counts, against what is under it.** A heading saying
      // 「4 卡」 over three rows is a lie about the thing directly
      // below — and the node can report a card count while sending no
      // per-card list, so the two really can disagree.
      says: Number(h.querySelector("[data-drawer-count]")?.dataset.drawerCount ?? -1),
      rows: open ? open.querySelectorAll("[data-part]").length : -1,
    };
  });
  if (!head) fail("the open drawer has no heading — six rows under four dials do not say which dial they are");
  else {
    if (head.glyphs !== 1) fail(`the drawer heading carries ${head.glyphs} glyphs`);
    if (!/\d/.test(head.text)) fail(`the drawer heading counts nothing: "${head.text}"`);
  }
  //     **The bar's two edges are one value each, down the whole list.**
  //     That is what the shared grid buys, and a row that sized its own
  //     bar would read as a ragged ladder.
  //
  //     ⚠️ **Measured at a width where the grid is actually in play.**
  //     At the gate's usual viewport the card is narrow enough that the
  //     drawer is in its stacked layout, where there is no shared grid
  //     to check — and the first version of this ran there and passed
  //     by being skipped, which reads exactly like passing on merit.
  //     So: widen first, then refuse to draw a conclusion if it is
  //     still tight.
  await page.setViewportSize({ width: 1600, height: 1000 });
  await wait(250);
  const edges = await page.evaluate(() => {
    const drawer = document.querySelector("[data-machine-card] [data-drawer]");
    const open = drawer.querySelector("[data-parts]:not([hidden])");
    const side = (sel, s) => [
      ...new Set([...open.querySelectorAll(sel)].map((n) => +n.getBoundingClientRect()[s].toFixed(1))),
    ];
    return {
      rows: open.querySelectorAll("[data-part]").length,
      barL: side("[data-part-bar]", "left"),
      barR: side("[data-part-bar]", "right"),
      nameL: side("[data-part-name]", "left"),
      numR: side("[data-part-num]", "right"),
      tight: drawer.hasAttribute("data-tight"),
    };
  });
  if (edges.rows < 2) fail(`probe dead: the open drawer has ${edges.rows} rows, so "one value each" is trivially true`);
  else if (edges.tight) fail("probe dead: the drawer stayed in its stacked layout, so the shared grid was never tested");
  else {
    for (const [what, vals] of [
      ["the bar's left edge", edges.barL],
      ["the bar's right edge", edges.barR],
      ["the name column's left edge", edges.nameL],
      ["the numbers' right edge", edges.numR],
    ]) {
      if (vals.length !== 1) fail(`${what} has ${vals.length} values across ${edges.rows} rows: ${vals.join(",")}`);
    }
  }
  await page.setViewportSize({ width: 1120, height: 1000 });
  await wait(200);

  //     ── one busy card among idle ones is still visible ───────────
  //
  //     **The aggregate flattens it, and the aggregate is what a
  //     careless implementation would draw.** This fixture's two cards
  //     are 90 and 4 while `gpu.util_pct` says 67; an implementation
  //     that painted the machine's mean per card would put 67 on both
  //     and the spread would collapse to 0. `khor_core::Gpu::util_pct`
  //     carries the ruling: the mean is a true sentence about the
  //     machine and a false lead about the job.
  const cardsNow = await page.evaluate(() => ({
    named: [...document.querySelectorAll("[data-machine-card] [data-gpu-card]")].map((e) => e.dataset.gpuCard),
    utils: [...document.querySelectorAll("[data-machine-card] [data-gpu-util]")].map((e) =>
      Number(e.dataset.gpuUtil),
    ),
    //   And the day block must not have grown per-card bands: the
    //   recorder keeps one aggregate series, so a card's own 24 hours
    //   is a promise the backend cannot pay (`history.rs`). Copying the
    //   aggregate four times would look exactly like the real thing.
    inDay: document.querySelectorAll("[data-machine-card] [data-day-band-axis] [data-gpu-card]").length,
  }));
  //     The finder proves itself before the negative claim below is
  //     worth anything.
  const wantCards = peerGpus.length;
  if (cardsNow.named.length !== wantCards || cardsNow.utils.length !== wantCards) {
    fail(
      `probe dead: ${cardsNow.named.length} card rows and ${cardsNow.utils.length} readings for ${wantCards} routed cards`,
    );
  } else {
    const spread = Math.max(...cardsNow.utils) - Math.min(...cardsNow.utils);
    const want = peerGpus.map((g) => Math.round(g.util_pct));
    const spreadWanted = Math.max(...want) - Math.min(...want);
    if (spread < spreadWanted - 1) {
      fail(
        `the cards read [${cardsNow.utils.join(",")}] where the fixture is [${want.join(",")}] — ` +
          `a spread of ${spread} against ${spreadWanted} is the machine's mean drawn once per card`,
      );
    }
    if (cardsNow.inDay !== 0) {
      fail(`${cardsNow.inDay} per-card rows inside the 24-hour block — the recorder keeps no per-card history to draw`);
    }
  }

  //     ── the band is a heat band: the value is the ink ────────────
  //
  //     **A · the shade really encodes the number.** Pinned to two
  //     named buckets rather than 「at least two shades exist」, which
  //     a implementation that coloured by bucket index would also
  //     satisfy.
  const shades = await page.evaluate(() => {
    const row = document.querySelector('[data-machine-card] [data-day-row="cpu"]');
    const cells = [...row.querySelectorAll("[data-day-cell]")];
    const at = (i) => cells[i]?.querySelector("[data-day-shade]")?.dataset.dayShade ?? null;
    const rects = [...row.querySelectorAll("[data-day-shade]")];
    return {
      // the fixture's cpu series is 5 + (i % 60), so bucket 0 is 5 and
      // bucket 58 is 63 — a low one and a high one, by construction.
      low: at(0),
      high: at(58),
      heights: [...new Set(rects.map((r) => r.getAttribute("height")))],
      views: [...new Set([...row.querySelectorAll("[data-day-view]")].map((e) => e.dataset.dayView))],
      hours: row.querySelectorAll("[data-day-hour]").length,
    };
  });
  if (shades.low === null || shades.high === null) {
    fail(`probe dead: the cpu row has no shaded cells to read (low=${shades.low} high=${shades.high})`);
  } else if (shades.low === shades.high) {
    fail(`bucket 0 (5%) and bucket 58 (63%) drew the same shade "${shades.low}" — the ink is not carrying the value`);
  }
  //     **B · no bars.** Every drawn cell is the full height of the
  //     row; a height proportional to the value is the old chart coming
  //     back under a new name.
  if (shades.heights.length !== 1 || shades.heights[0] !== "100") {
    fail(`the cells draw ${shades.heights.length} different heights (${shades.heights.join(",")}) — a heat band has one`);
  }
  if (shades.views.join(",") !== "heat") fail(`the band does not say which reading it is (${shades.views.join(",")})`);
  if (shades.hours !== 23) fail(`${shades.hours} hour marks on a 288-bucket day — one an hour is 23 between the ends`);
  //     **F · strain is judged per bucket, and only where being full is
  //     a fault.** 内存 wears it; CPU does not, because a busy machine
  //     is a machine working (see `shadeOf`). The fixture puts one
  //     bucket at exactly 90.0 in the vram series to pin `>=`.
  const strain = await page.evaluate(() => {
    const of = (name) => {
      const row = document.querySelector(`[data-machine-card] [data-day-row="${name}"]`);
      if (!row) return null;
      return [...row.querySelectorAll("[data-day-cell]")].map((c) => {
        const r = c.querySelector("[data-day-shade]");
        return r ? r.dataset.dayShade : null;
      });
    };
    return { mem: of("mem"), cpu: of("cpu"), vram: of("vram") };
  });
  const wantStrain = memBuckets.map((v, i) => (v !== null && v >= 90 ? i : -1)).filter((i) => i >= 0);
  const gotStrain = (strain.mem ?? []).map((v, i) => (v === "strain" ? i : -1)).filter((i) => i >= 0);
  if (gotStrain.join(",") !== wantStrain.join(",")) {
    fail(`内存 wears strain on buckets [${gotStrain.join(",")}] where the fixture is ≥90 on [${wantStrain.join(",")}]`);
  }
  if ((strain.cpu ?? []).some((v) => v === "strain")) {
    fail("a CPU bucket drew in the strain colour — a machine at 95% CPU is a machine doing its job, not an alarm");
  }
  if ((strain.vram ?? []).some((v) => v === "strain")) {
    fail("a 显存 bucket drew in the strain colour — a full card is the normal state of a box being used");
  }
  //     And the boundary itself: the fixture's bucket 11 is exactly
  //     90.0, so a `>` would leave it a step below.
  if ((strain.vram ?? [])[11] !== "4") {
    fail(`the bucket at exactly 90.0 drew "${(strain.vram ?? [])[11]}" — the top step is >= 90, not > 90`);
  }

  //     **One row per series that has a reading, and the all-hole one
  //     is not a row.** A named row of pure absence would read as a
  //     reading this machine failed to take, rather than as one it does
  //     not take at all — and the fixture's disk series is exactly that
  //     case, so an implementation that draws a row per *field* reds
  //     here.
  if (busy.dayRows.length !== dayRows) {
    fail(
      `the day draws ${busy.dayRows.length} rows for ${dayRows} recorded series ` +
        `(${busy.dayRows.map((r) => r.name).join(",")}) — the fixture's disk series is all holes`,
    );
  }
  if (busy.dayRows.some((r) => r.name === "disk")) fail("a series with no reading in it still got a row");
  for (const r of busy.dayRows) {
    if (r.cells !== 288) fail(`the ${r.name} row draws ${r.cells} cells for a day of 288 buckets`);
    if (r.gaps !== dayGaps) fail(`${r.name}: ${r.gaps} gaps where the fixture left ${dayGaps} unmeasured`);
    if (r.barsInGaps !== 0) fail(`${r.name}: ${r.barsInGaps} gap cells draw a bar — absence painted as a reading`);
    if (r.bars !== 288 - dayGaps) fail(`${r.name}: ${r.bars} bars for ${288 - dayGaps} readings`);
    //   And each row says how high it got: a shape alone answers 什么
    //   时候 and never 多高.
    if (!/\d+%/.test(r.peak)) fail(`${r.name}: the row draws a shape but never says its peak ("${r.peak}")`);
  }
  if (busy.why !== 0) fail("the peer answered and the card still prints a refusal");
  await open("devices");
  await pressNth("[data-list] [data-row-open]", 2);
  await until(5_000, async () => (await page.locator("[data-machine-card] [data-load-why]").count()) === 1);
  const refused = await page.evaluate(() => ({
    why: document.querySelector("[data-machine-card] [data-load-why]")?.textContent ?? "",
    band: document.querySelectorAll("[data-machine-card] [data-day-cell]").length,
    //   **What a refusal must keep, and what it must not.** Rows and
    //   band are the machine's readings: drawing either of them for an
    //   answer that never arrived invents an idle machine, which is the
    //   whole reason the sentence exists. The column heads are not
    //   readings — they are the controls — and a refusal that takes
    //   them away is a dead end: ordering a remote by PID is refused by
    //   a khor that predates that column, and with the heads gone there
    //   is no second column left to press (2026-09-04). So: heads yes,
    //   rows no, and the sentence in the body where the rows would be.
    rows: document.querySelectorAll("[data-machine-card] [data-top-row]").length,
    heads: document.querySelectorAll("[data-machine-card] [data-top-head]").length,
    pressable: document.querySelectorAll("[data-machine-card] [data-top-head] button").length,
    whyInBody: document.querySelectorAll("[data-machine-card] [data-load-top] > [data-load-why]").length,
  }));
  if (!refused.why.includes(ELDER_REFUSAL)) fail(`elder's refusal is not the sentence it sent: "${refused.why}"`);
  if (refused.rows !== 0 || refused.band !== 0) {
    fail(`elder refused and the card still draws ${refused.rows} rows / ${refused.band} band cells — an idle machine invented`);
  }
  if (refused.heads === 0 || refused.pressable !== refused.heads) {
    fail(
      `elder refused and the way out went with it: ${refused.heads} column heads, ${refused.pressable} of them pressable ` +
        `— a refusal with no head left to press can only be escaped by opening another machine`,
    );
  }
  if (refused.whyInBody !== 1) {
    fail(`the refusal is not where the rows would be (${refused.whyInBody} in the table body) — it has to read as what replaced them`);
  }

  if (thrown.length) fail(`page errors while measuring: ${thrown.join(" | ")}`);
  if (!process.exitCode) console.log(
      "face gate: three pressures, numbers in columns, every stop's targets a finger's size, a dense pane that leads, the machine surfaces bare of subscription arcs, the \u989d\u5ea6 rows in all three shapes with the arcs at their foot, and narrow's session tabs switching for real",
    );
} finally {
  await browser?.close();
  vite?.kill();
}
