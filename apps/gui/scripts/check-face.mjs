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
  // **Two buckets are pinned: 95, and 90 exactly.** The second is the
  // strain boundary, and it lives on a row that wears strain because
  // that is where `>= 90` is nameable — a `>` written instead reds on
  // that bucket alone.
  const memBuckets = dayBuckets.map((b, i) => (b === null ? null : i === 7 ? 95 : i === 11 ? 90 : 40 + (i % 20)));
  // **An idle machine's day, which is what the bands were widened
  // for.** Everything here lives under 20 — turing really does draw
  // 564 buckets in that range across four rows — and with the old
  // four bands the whole row lands in one shade, which is the
  // regression the flat-slab check exists to catch. A fixture whose
  // every series happens to span the full range cannot see it.
  const gpuBuckets = dayBuckets.map((b, i) => (b === null ? null : 2 + (i % 14)));
  const diskBuckets = dayBuckets.map(() => null);
  // **A day spent wholly inside the 70–90 band, which is what the
  // eighth cut was made for.** turing's video memory really does live
  // between 74.6 and 89.6 for a whole day: fifteen points of range,
  // every one of them in one band, so it drew as a slab no matter how
  // many cuts went in at the bottom. This series is that shape — under
  // the seven-band ramp it is one shade across 15 points of a 20-point
  // band and the flat-slab check reds; under eight it draws two.
  //
  // It carries no outlier on purpose: one bucket in a second band is
  // all it takes for a row to stop being flat, and then the arm proves
  // nothing.
  const vramBuckets = dayBuckets.map((b, i) => (b === null ? null : 72 + (i % 16)));
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
  //   **The window moves between answers, because on a real node it
  //   does.** `load.cpu_day.now_ms` is stamped once when this object is
  //   built, and a fixture that hands out the same instant forever
  //   cannot show a chart being redrawn for a newer one — which is the
  //   whole of the 「a press must not move the day」 section below.
  //   Written without this, that section passed with its fix removed:
  //   an assertion resting on a constant (`index.html`'s
  //   `group_totals`, the first of the false greens in this repo's
  //   ledger).
  //
  //   A second per answer: enough to be a different instant, small
  //   enough that the window's hour marks and its 「yesterday」 label
  //   are the ones every other section here was written against.
  let served = 0;
  const dayNow = () => ({ ...load, cpu_day: { ...load.cpu_day, now_ms: load.cpu_day.now_ms + served++ * 1000 } });
  await page.route("**/load", async (r) => {
    if (r.request().postDataJSON().machine === "elder") {
      return r.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify(ELDER_REFUSAL) });
    }
    // **A page with nothing on a card in it, and no ages either.** This
    // is what ordering turing by PID really returns: the lowest pids
    // are the boot daemons, none of them on a card and several of them
    // started before the kernel could say when. It is here so that
    // 「the columns do not depend on the page」 has two distinguishable
    // states — without it every ordering answers with the same rows and
    // an implementation that reads the column set off the rows passes.
    if (r.request().postDataJSON().by === "pid") {
      const fresh = dayNow();
      const bare = { ...fresh, top: fresh.top.map((t) => ({ ...t, gpu_pct: null, gpu_mem: null, run_secs: null })) };
      return r.fulfill({ contentType: "application/json", body: JSON.stringify(bare) });
    }
    if (held.on) {
      held.seen += 1;
      await new Promise((ok) => (held.release = ok));
      const fresh = dayNow();
      const flipped = { ...fresh, top: [...fresh.top].reverse() };
      return r.fulfill({ contentType: "application/json", body: JSON.stringify(flipped) });
    }
    return r.fulfill({ contentType: "application/json", body: JSON.stringify(dayNow()) });
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
    dayRows: [...document.querySelectorAll("[data-machine-card] [data-day-row]")].map((r) => {
      const segs = [...r.querySelectorAll("[data-day-cell]")];
      return {
        name: r.getAttribute("data-day-row"),
        cells: segs.length,
        //   **What the cells cover, not how many there are.** Buckets of
        //   the same step are drawn as one rect, so a count is a fact
        //   about the data's shape; what has to hold is that the ink
        //   covers every measured bucket and no unmeasured one.
        covered: segs.reduce((n, c) => n + (Number(c.dataset.dayTo) - Number(c.dataset.dayFrom)), 0),
        //   **The mark is not inside the row's group** — the drawing
        //   hangs it on the figure, past the plot's right edge — so it
        //   is found by the row it names rather than by descent.
        peak:
          document
            .querySelector(`[data-machine-card] [data-day-peak="${r.getAttribute("data-day-row")}"]`)
            ?.textContent?.trim() ?? "",
      };
    }),
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
  //     which is the entire reason it is carried at all. Measured
  //     2026-09-04: a 390 card showed three rows all reading `python3`.
  //
  //     ⚠️ **Expected count, not 「every row that has an `arg`」.** Since
  //     the two columns became one cell (2026-09-04) a word identical
  //     to the name it sits beside is not drawn twice — that is the two
  //     vLLM workers on turing, whose `argv[0]` *is* their name. So the
  //     rows this asks about are the ones where the word says something
  //     the name did not.
  const withArg = load.top.filter((r) => r.arg !== null && r.arg !== r.name).length;
  //     ⚠️ **Counted on ink, not on text.** The obvious probe is
  //     `textContent.trim().length > 0`, and it is blind to the way
  //     this cell actually fails: the word lives in a flex item beside
  //     the name, and a name with no ceiling squeezes it to **zero
  //     width** — present in the DOM, its text intact, and nothing on
  //     screen. Same shape as the 「undefined」 sweep that could not see
  //     an empty `<text>`: measure the thing the reader has, which is
  //     width.
  const argsAt = async () =>
    page.evaluate(
      () =>
        [...document.querySelectorAll("[data-machine-card] [data-top-row] [data-cmd-arg]")].filter(
          (c) => c.textContent.trim().length > 0 && c.getBoundingClientRect().width >= 1,
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
        `on 390 the rows collapse to their short names, which is the case this word exists for`,
    );
  }
  //     ── 「命令」 is one line, and the name is always the start of it
  //
  //     2026-09-04:「这俩就是俩字符串,放一列是有原因的,干嘛还要单独显示
  //     宽度呢?他俩加一块共享列宽」 — said after seeing a row whose
  //     process name had been squeezed out of existence and replaced by
  //     a single `▪`.
  //
  //     **The section this replaces measured the wrong thing, correctly.**
  //     It held a floor under the word and asked that neither half be
  //     squeezed below it — and every one of its assertions passed on
  //     the build that produced the `▪`, because the name giving way
  //     was that design working as written. A pair of widths cannot be
  //     made safe by bounding each of them; it had to stop being a
  //     pair.
  //
  //     So what is asked now is the thing that actually matters and the
  //     thing a reader would check: **whatever else that cell drops,
  //     the first characters of the process's name are on screen.**
  //     Measured at the tightest width where the table is still a grid,
  //     which is where the cell is under the most pressure.
  {
    const tightAt = async (w) => {
      await page.setViewportSize({ width: w, height: 1000 });
      await new Promise((r) => setTimeout(r, 120));
      return page.evaluate(() => document.querySelector("[data-machine-card] [data-load-top-box]")?.dataset.tight !== undefined);
    };
    let edge = null;
    for (let w = 1120; w >= 400; w -= 20) {
      if (await tightAt(w)) break;
      edge = w;
    }
    if (edge === null) {
      fail("probe dead: the table is stacked at every width down to 400, so the grid's tightest stop was never reached");
    } else {
      await page.setViewportSize({ width: edge, height: 1000 });
      await new Promise((r) => setTimeout(r, 200));
      //   **One reading of the cells, used by the check and by the
      //   proof that the check works.** How much of the name is
      //   actually on screen, in characters: the line clips with
      //   `overflow: hidden`, so a name pushed past the right edge has
      //   a box that starts beyond the visible width — measured against
      //   the line's own box, not against the name's.
      const cmdCells = () =>
        page.evaluate(() => {
          const out = [];
          for (const cell of document.querySelectorAll("[data-machine-card] [data-top-row] [data-c='cmd']")) {
            const name = cell.querySelector("[data-cmd-name]");
            const lineEl = cell.querySelector("[data-cmd-line]");
            if (!name || !lineEl) continue;
            const lb = lineEl.getBoundingClientRect();
            const nb = name.getBoundingClientRect();
            const shown = Math.max(0, Math.min(nb.right, lb.right) - Math.max(nb.left, lb.left));
            const per = nb.width / Math.max(1, name.textContent.length);
            out.push({
              name: name.textContent.slice(0, 20),
              chars: +(shown / Math.max(0.001, per)).toFixed(2),
              cellW: +cell.getBoundingClientRect().width.toFixed(1),
              text: cell.textContent.trim().slice(0, 24),
            });
          }
          return out;
        });
      //   **Before asserting that the name is never gone, prove the
      //   probe can see it go.** This is a negative claim, and today
      //   nothing in the fixture violates it — the shape that produced
      //   the `▪` needed a long identifying word in a narrow cell on
      //   the real fleet, which this data does not have. So: squeeze
      //   one cell until the name really is off screen, check it is
      //   reported, put it back.
      //
      //   ⚠️ It has to be **the same** reading the check uses. Written
      //   first with its own copy of the arithmetic, this proof would
      //   have stayed green with the check below blinded — two arms
      //   built to agree, which is a control with no power. That
      //   mistake has already been made twice tonight, once on the wrap
      //   check and once here.
      {
        const bend = (on) =>
          page.evaluate((on) => {
            const line = document.querySelector("[data-machine-card] [data-top-row] [data-c='cmd'] [data-cmd-line]");
            if (!line) return false;
            if (on) line.style.width = "3px";
            else line.style.removeProperty("width");
            return true;
          }, on);
        if (!(await bend(true))) {
          fail("probe dead: no 「命令」 line to squeeze, so 「the name is always there」 was never shown to be checkable");
        } else {
          const squeezed = await cmdCells();
          await bend(false);
          const least = squeezed.length ? Math.min(...squeezed.map((c) => c.chars)) : null;
          if (!(least !== null && least < 1)) {
            fail(`probe dead: a cell squeezed to three pixels still reported ${least} characters of its name — the check below cannot fail`);
          }
        }
      }
      const cells = await cmdCells();
      if (cells.length === 0) {
        fail(`probe dead: no 「命令」 cell at ${edge}, so 「the name is always there」 measured nothing`);
      } else {
        //   One whole character. Less than that is not a truncated
        //   name, it is a placeholder standing where the answer was.
        const gone = cells.filter((c) => c.chars < 1);
        if (gone.length) {
          const w = gone[0];
          fail(
            `at ${edge} — the grid's tightest stop — ${gone.length} of ${cells.length} 「命令」 cells show less than one ` +
              `character of the process's name (「${w.name}」 → ${w.chars} chars in a ${w.cellW}px cell, cell reads 「${w.text}」) ` +
              `— a cell that cannot say what the process is has not truncated, it has deleted the answer`,
          );
        }
        //   **And nothing stands in for the name.** A single glyph in
        //   place of a word is the shape of the bug, and it passes a
        //   「the name is present」 test that only looks at the DOM.
        const stand = cells.filter((c) => /^[▪■□◦•·]\s*$/.test(c.text));
        if (stand.length) {
          fail(`${stand.length} 「命令」 cells read as a placeholder glyph (「${stand[0].text}」) instead of a name`);
        }
        console.log(
          `命令 at ${edge} (the grid's tightest): ${cells.length} cells, ` +
            `least of a name on screen ${Math.min(...cells.map((c) => c.chars)).toFixed(1)} chars, ` +
            `narrowest cell ${Math.min(...cells.map((c) => c.cellW)).toFixed(0)}px`,
        );
      }
    }
    await page.setViewportSize({ width: 1120, height: 1000 });
    await new Promise((r) => setTimeout(r, 200));
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

  //     ── the column set is a fact about the machine ───────────────
  //
  //     **Press every head in turn; not one column may appear or
  //     disappear.** 「这台机器有没有卡」 is a property of the machine,
  //     not of the two hundred rows the current ordering cut — and when
  //     it was read off the rows, pressing 「GPU」 on turing deleted the
  //     GPU column, taking with it the head you would press to undo it
  //     (user, 2026-09-04:「一点整个表就变了，比如GPU，一点就没有GPU这
  //     列了」).
  //
  //     The route above answers a PID ordering with a page that has no
  //     card readings and no ages in it, which is what the real one
  //     does; so this goes red on the implementation it was written
  //     against, and green only on one that asks the machine.
  const colsNow = () =>
    page.evaluate(() =>
      [...document.querySelectorAll("[data-machine-card] [data-top-heads] > *")]
        .map((n) => n.dataset.topHead ?? n.dataset.c ?? "?")
        .join(","),
    );
  {
    const first = await colsNow();
    if (first.split(",").length < 5) fail(`probe dead: the table shows only [${first}], so pressing its heads proves nothing`);
    //   **Every head, `cmd` included.** It used to be excused here
    //   because the command line was a column that did not sort; since
    //   the merge (2026-09-04) 「命令」 is a head like the others and
    //   orders by the name, so the one head that was never pressed is
    //   now pressed too.
    for (const col of first.split(",")) {
      await page.locator(`[data-machine-card] [data-top-head="${col}"] button`).click();
      // Let the ask land — the rows are redrawn from the answer, and it
      // is the redraw that used to take the column with it.
      await new Promise((r) => setTimeout(r, 700));
      const now = await colsNow();
      if (now !== first) {
        fail(`pressing 「${col}」 changed the table's columns from [${first}] to [${now}] — an ordering may not decide which columns exist`);
        break;
      }
    }
  }
  //     ── one slot: the rings, the day, and no jump ────────────────
  //
  //     2026-09-04:「上环下趋势,合成一个模块」、「点一个环,下面换成那个
  //     环的抽屉;再点同一个环,回到趋势」、「那个槽定高,两种内容共用同一个
  //     高度」、「开着抽屉时那个环要看得出是选中的」.
  //
  //     Five things, and each of them is the whole point of one of
  //     those sentences:
  //
  //       1. **it opens on the day**, with no ring pressed;
  //       2. **a press trades the day for that ring's drawer**, and the
  //          same ring pressed again trades it back — a control whose
  //          press does nothing when it is already on reads as stuck;
  //       3. **the slot is one height**, whichever of the two is in it
  //          and whichever ring is open — this is 「不许一切一跳」 and it
  //          is measured on the card, not on the slot alone, because a
  //          slot that held its height while the card below it moved
  //          would satisfy a smaller claim;
  //       4. **the first line is one line**: the reading words and the
  //          drawer's heading start at the same y and are the same
  //          height, so the words are *replaced* rather than vanishing
  //          and coming back;
  //       5. **what is closed is hidden, never unmounted** — `smoke.mjs`
  //          counts this card's readings against `khor devices`, and a
  //          slot that dropped its rows would make the card disagree
  //          with the terminal, which reads as a broken backend.
  {
    await page.setViewportSize({ width: 1120, height: 1000 });
    await wait(300);
    const look = () =>
      page.evaluate(() => {
        const slot = document.querySelector("[data-machine-card] [data-ring-slot]");
        //   **What is under the slot**, which is the thing that
        //   actually moves when the slot changes size. The card's own
        //   box was measured here first and it is a scroll container —
        //   1000 tall in every state, including the one where the slot
        //   went 169 → 219. An assertion on it cannot fail.
        const below = document.querySelector("[data-machine-card] [data-top-plot]");
        const day = slot?.querySelector("[data-load-day]");
        const drawer = slot?.querySelector("[data-drawer]");
        const head = slot?.querySelector("[data-drawer-head]");
        const words = slot?.querySelector("[data-day-views]");
        const line = (n) => (n && !n.hidden ? { y: Math.round(n.getBoundingClientRect().top), h: Math.round(n.getBoundingClientRect().height) } : null);
        return {
          slotH: slot ? Math.round(slot.getBoundingClientRect().height) : null,
          belowY: below ? Math.round(below.getBoundingClientRect().top) : null,
          showing: day && !day.hidden ? "day" : (drawer && !drawer.hidden ? drawer.dataset.drawer : null),
          //   Present in the document either way — that is the claim.
          dayInDom: !!day,
          rowsInDom: document.querySelectorAll("[data-machine-card] [data-parts] [data-part]").length,
          bandInDom: !!slot?.querySelector("[data-day-band]"),
          first: line(head) ?? line(words),
          on: [...document.querySelectorAll("[data-machine-card] [data-dial-word]")].filter((w) => w.dataset.on === "true").length,
        };
      });
    const press = async (k) => {
      await page.locator(`[data-machine-card] [data-dial="${k}"]`).click();
      await wait(300);
      return look();
    };
    const rest = await look();
    if (rest.slotH === null) {
      fail("probe dead: the machine card has no ring slot, so nothing about it was measured");
    } else {
      if (rest.showing !== "day") fail(`the card opens showing ${rest.showing} — with no ring pressed the slot is the day's`);
      if (rest.on !== 0) fail(`${rest.on} ring names are marked as open on a card where nothing is open`);
      const seen = [rest];
      const onCpu = await press("cpu");
      seen.push(onCpu);
      if (onCpu.showing !== "cpu") fail(`pressing CPU left the slot showing ${onCpu.showing}`);
      if (onCpu.on !== 1) fail(`${onCpu.on} ring names are marked with the CPU drawer open — exactly the open one says so`);
      const onMem = await press("mem");
      seen.push(onMem);
      if (onMem.showing !== "mem") fail(`pressing 内存 while CPU was open left the slot showing ${onMem.showing}`);
      const back = await press("mem");
      seen.push(back);
      if (back.showing !== "day") fail(`pressing the open ring again left the slot showing ${back.showing} — it has to go back to the day`);
      if (back.on !== 0) fail(`${back.on} ring names still marked after closing the drawer`);
      //   3 — one height, everywhere.
      const heights = [...new Set(seen.map((x) => x.slotH))];
      if (heights.length !== 1) fail(`the slot is ${heights.join("/")} tall depending on what is in it — pressing a ring moves the card`);
      const belows = [...new Set(seen.map((x) => x.belowY))];
      if (belows.length !== 1 || belows[0] === null) {
        fail(`what sits under the slot starts at ${belows.join("/")} depending on what the slot shows — pressing a ring moves the rest of the card`);
      }
      //   4 — one first line.
      const firsts = seen.map((x) => x.first).filter(Boolean);
      if (firsts.length < 2) {
        fail(`probe dead: only ${firsts.length} of the slot's states drew a first line, so 「same line」 compared nothing`);
      } else {
        const ys = [...new Set(firsts.map((f) => f.y))];
        const hs = [...new Set(firsts.map((f) => f.h))];
        if (ys.length !== 1) fail(`the slot's first line starts at ${ys.join("/")} — the reading words and the drawer's heading are not the same line`);
        if (hs.length !== 1) fail(`the slot's first line is ${hs.join("/")} tall — one of them pushes what is under it`);
      }
      //   5 — closed is hidden, not gone.
      for (const x of seen) {
        if (!x.dayInDom || !x.bandInDom) {
          fail(`the 24-hour figure left the document while the slot showed ${x.showing} — smoke counts this card against 「khor devices」`);
          break;
        }
      }
      const rows = [...new Set(seen.map((x) => x.rowsInDom))];
      if (rows.length !== 1 || rows[0] === 0) {
        fail(`the drawers hold ${rows.join("/")} rows depending on which is open — closed groups are hidden, never unmounted`);
      }
      //   **Every drawer's air, and it has to be shared not parked.**
      //   2026-09-04:「不要满足最高高度,而导致部分图下面空掉了」 was
      //   said of the three readings; the drawers share that slot and
      //   had it worse. Measured on the fleet before the fix: the GPU
      //   drawer's last row sat **152px** above the floor on a machine
      //   with one card, 磁盘 121px with two volumes, against 13px for
      //   the six-row ones.
      //
      //   ⚠️ **This section used to print those numbers and assert
      //   nothing**, on the argument that a list's bottom edge is where
      //   its items ran out and is a different fact from a drawing's.
      //   That argument was overruled by the person reading the screen,
      //   and he was right about what it produced: ink at the top and a
      //   void under it is the same picture whatever is drawing it.
      //
      //   **The check is symmetry, not zero, and that is the decision
      //   this replaces the argument with.** Six rows fill the box and
      //   touch both its edges; one card cannot — there is nothing left
      //   to stretch but the air. What was complained about is air at
      //   one end, so what is asked is that there is as much above as
      //   below. Measured after: 0/0 for the six-row drawers, 34.8/34.8
      //   for two cards, 78.5/78.5 for one.
      //
      //   **All four, not whichever one the presses above left open** —
      //   the short ones are the only ones that can fail this, and a
      //   six-row drawer passes it for free.
      const air = [];
      for (const k of ["cpu", "gpu", "mem", "disk"]) {
        const dial = page.locator(`[data-machine-card] [data-dial="${k}"]`).first();
        if (!(await dial.count())) continue;
        if ((await dial.getAttribute("data-open")) !== "true") await dial.click();
        await wait(350);
        const one = await page.evaluate(() => {
          const slot = document.querySelector("[data-machine-card] [data-ring-slot]");
          const drawer = slot?.querySelector("[data-drawer]");
          const open = drawer?.querySelector("[data-parts]:not([hidden])");
          if (!open || drawer.hidden) return null;
          const rows = [...open.querySelectorAll("[data-part]")];
          if (!rows.length) return { which: drawer.dataset.drawer, rows: 0 };
          const cells = rows.flatMap((r) => [...r.children]).map((c) => c.getBoundingClientRect());
          const body = open.closest("[data-drawer-scroll]") ?? open;
          return {
            which: drawer.dataset.drawer,
            rows: rows.length,
            gap: +(slot.getBoundingClientRect().bottom - Math.max(...cells.map((r) => r.bottom))).toFixed(1),
            headGap: +(Math.min(...cells.map((r) => r.top)) - body.getBoundingClientRect().top).toFixed(1),
          };
        });
        if (one && one.rows > 0) air.push(one);
      }
      // Leave the slot back on the day, which is where this section found it.
      {
        const openDial = page.locator('[data-machine-card] [data-dial][data-open="true"]').first();
        if (await openDial.count()) await openDial.click();
        await wait(300);
      }
      //   ⚠️ **Prove the detector can see lopsided air before trusting
      //   it.** In this fixture every drawer fills its box outright
      //   (0/0 four times), so the assertion below has nothing to catch
      //   here — the shape it is for needs a machine with one card,
      //   which the fleet has and this data does not. So: make one
      //   drawer huddle its rows at the top, check it is reported, put
      //   it back. Same reading as the check, deliberately.
      {
        const dial = page.locator('[data-machine-card] [data-dial="gpu"]').first();
        const has = (await dial.count()) > 0;
        if (has && (await dial.getAttribute("data-open")) !== "true") await dial.click();
        await wait(350);
        const bent = await page.evaluate(() => {
          const open = document.querySelector("[data-machine-card] [data-parts]:not([hidden])");
          if (!open) return null;
          open.style.alignContent = "start";
          open.style.gridAutoRows = "min-content";
          const slot = document.querySelector("[data-machine-card] [data-ring-slot]");
          const rows = [...open.querySelectorAll("[data-part]")];
          const cells = rows.flatMap((r) => [...r.children]).map((c) => c.getBoundingClientRect());
          const body = open.closest("[data-drawer-scroll]") ?? open;
          const out = {
            gap: +(slot.getBoundingClientRect().bottom - Math.max(...cells.map((r) => r.bottom))).toFixed(1),
            headGap: +(Math.min(...cells.map((r) => r.top)) - body.getBoundingClientRect().top).toFixed(1),
          };
          open.style.removeProperty("align-content");
          open.style.removeProperty("grid-auto-rows");
          return out;
        });
        const openDial = page.locator('[data-machine-card] [data-dial][data-open="true"]').first();
        if (await openDial.count()) await openDial.click();
        await wait(300);
        if (!bent) fail("probe dead: no drawer to bend, so 「the air is shared」 was never shown to be checkable");
        else if (Math.abs(bent.headGap - bent.gap) <= 1) {
          fail(
            `probe dead: a drawer forced to huddle its rows at the top reported ${bent.headGap} above and ${bent.gap} below — ` +
              `the check below cannot fail`,
          );
        }
      }
      if (air.length < 2) {
        fail(`probe dead: only ${air.length} drawers had rows to measure, so 「the air is shared」 was not tested`);
      } else {
        const parked = air.filter((a) => Math.abs(a.headGap - a.gap) > 1);
        if (parked.length) {
          const w = parked[0];
          fail(
            `the ${w.which} drawer leaves ${w.headGap}px above its ${w.rows} rows and ${w.gap}px below — ` +
              `the slack is parked at one end, which is the strip of nothing the fixed slot was for`,
          );
        }
        //   Probe alive: at least one of them is short enough that it
        //   *could* have been lopsided. A run where every drawer
        //   overflowed would pass this without testing it.
        if (!air.some((a) => a.gap > 1)) {
          console.log(`drawer air: ${air.map((a) => `${a.which} ${a.rows}r ${a.headGap}/${a.gap}`).join(", ")} — every drawer filled its box outright`);
        } else {
          console.log(`drawer air: ${air.map((a) => `${a.which} ${a.rows}r ${a.headGap}/${a.gap}`).join(", ")}`);
        }
      }
      console.log(
        `ring slot: opens on ${rest.showing}, slot ${heights.join("/")} tall and what is under it at ${belows.join("/")} across ` +
          `[${seen.map((x) => x.showing).join(" → ")}], first line ${JSON.stringify(firsts[0])}, ${rows[0]} rows in the document throughout`,
      );
    }
  }

  //     ── a press orders the rows and moves nothing else ───────────
  //
  //     2026-09-04:「表点一些 sorting 方式,趋势和 CPU、GPU、内存都在变。
  //     不应该这样」. One ask carries the rows *and* the day, so the
  //     answer to 「order by memory」 came back with a newer `now_ms`
  //     and the 24-hour figure was rebuilt under a reader who had asked
  //     for nothing of the sort.
  //
  //     **Node identity, not content.** The fixture's day is the same
  //     numbers every time, so a redrawn chart draws the same picture —
  //     an assertion on what it *says* passes straight through the bug.
  //     What changes is that every node is a new one, so the probe
  //     marks a cell and looks for that mark afterwards. (The mark
  //     survives a React re-render; it does not survive the module
  //     rebuilding the figure, which is the thing being caught.)
  //
  //     ⚠️ **And the rows must actually move**, or this is a section
  //     about a table that ignored every press.
  {
    await page.setViewportSize({ width: 1120, height: 1000 });
    await wait(300);
    const heads = await page.evaluate(() =>
      [...document.querySelectorAll("[data-machine-card] [data-top-heads] > *")]
        .map((h) => h.dataset.topHead)
        .filter(Boolean),
    );
    if (heads.length < 4) {
      fail(`probe dead: ${heads.length} column heads, so 「a press moves nothing else」 was not measured`);
    } else {
      let moved = 0;
      let kept = true;
      for (const h of heads) {
        const before = await page.evaluate(() => {
          const cell = document.querySelector("[data-machine-card] [data-day-cell]");
          if (cell) cell.setAttribute("data-probe-tag", "1");
          return {
            marked: !!cell,
            dials: [...document.querySelectorAll("[data-machine-card] [data-dial]")]
              .map((d) => `${d.dataset.dial}=${d.innerText.replace(/\s+/g, "")}`)
              .join("|"),
            rows: [...document.querySelectorAll("[data-machine-card] [data-top-row]")].map((r) => r.dataset.topRow).join(","),
          };
        });
        if (!before.marked) {
          fail("probe dead: no day cell to mark, so nothing was watched across the press");
          break;
        }
        await page.locator(`[data-machine-card] [data-top-head="${h}"] button`).click();
        await wait(900);
        const after = await page.evaluate(() => ({
          kept: !!document.querySelector('[data-machine-card] [data-day-cell][data-probe-tag="1"]'),
          dials: [...document.querySelectorAll("[data-machine-card] [data-dial]")]
            .map((d) => `${d.dataset.dial}=${d.innerText.replace(/\s+/g, "")}`)
            .join("|"),
          rows: [...document.querySelectorAll("[data-machine-card] [data-top-row]")].map((r) => r.dataset.topRow).join(","),
        }));
        if (!after.kept) {
          kept = false;
          fail(`pressing 「${h}」 rebuilt the 24-hour figure — the reader asked for the rows to move, not the chart`);
          break;
        }
        if (after.dials !== before.dials) {
          fail(`pressing 「${h}」 changed the dials — they are drawn from the machine's vitals, which an ordering does not touch`);
          break;
        }
        if (after.rows !== before.rows) moved += 1;
      }
      if (moved === 0) {
        fail(`probe dead: ${heads.length} presses and the rows never changed, so 「and nothing else moved」 measured a table that ignores its heads`);
      }
      //   Reports what it saw; it does not state the finding. The
      //   first draft ended 「untouched by all of them」 and printed
      //   that verbatim on the run where the very first press had just
      //   failed.
      console.log(`sorting: ${heads.length} heads, rows re-ordered on ${moved}, day rebuilt by ${kept ? 0 : "a press"}`);
    }
  }

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
    //   **One detector, used by the check and by the proof that the
    //   check works.** Reports every cell or head whose ink is taller
    //   than one line of it, skipping the cell that is open — unfolding
    //   is what its mark promised. Measured as ink with a `Range` and
    //   not as a box: a cell's box is stretched by the tallest thing in
    //   its row, so comparing boxes says nothing about which cell
    //   wrapped.
    const wrapsNow = () =>
      page.evaluate(() => {
        const one = (n) => {
          const st = getComputedStyle(n);
          const v = parseFloat(st.lineHeight);
          return Number.isFinite(v) ? v : parseFloat(st.fontSize) * 1.6;
        };
        const out = [];
        const nodes = [
          ...document.querySelectorAll("[data-machine-card] [data-top-heads] > *"),
          ...document.querySelectorAll("[data-machine-card] [data-top-row] > *"),
        ];
        for (const n of nodes) {
          if (n.dataset.open !== undefined) continue;
          const target = n.querySelector("button") ?? n;
          const r = document.createRange();
          r.selectNodeContents(target);
          const ink = r.getBoundingClientRect().height;
          if (ink > one(target) + 2) {
            out.push(`${n.dataset.topHead ?? n.dataset.c ?? "?"}:${ink.toFixed(0)}/${one(target).toFixed(0)}`);
            if (out.length > 2) break;
          }
        }
        return out;
      });
    //   **Before asserting that nothing wraps, prove that detector can
    //   see a wrap.** This is a negative claim about every cell at
    //   every width, and today nothing violates it: the cells wear
    //   `whitespace-nowrap` and `truncate` of their own, and the
    //   table-wide rule added for it (`app.css`, 「Nothing in this table
    //   wraps」) is a guarantee for the *next* cell somebody writes
    //   rather than a fix for a current one. **Deleting that rule reds
    //   nothing** — measured — so the assertion below would pass just
    //   as happily on a detector that had stopped working.
    //
    //   ⚠️ It has to be **this** detector. The first version of this
    //   proof carried its own copy of the ink-against-a-line test, so
    //   blinding the one in the loop left the proof green — two arms
    //   built to agree, which is the shape of a control with no
    //   discriminating power.
    {
      const bend = (on) =>
        page.evaluate((on) => {
          const cell = document.querySelector('[data-machine-card] [data-top-row] [data-c="time"]');
          if (!cell) return false;
          if (on) {
            cell.dataset.wrapProbe = cell.textContent;
            cell.style.whiteSpace = "normal";
            cell.textContent = "一 二 三 四 五 六 七 八 九 十 十一 十二 十三 十四";
          } else {
            cell.textContent = cell.dataset.wrapProbe ?? "";
            cell.style.removeProperty("white-space");
            delete cell.dataset.wrapProbe;
          }
          return true;
        }, on);
      if (!(await bend(true))) {
        fail("probe dead: no 时长 cell to bend, so 「nothing wraps」 was never shown to be checkable");
      } else {
        const caught = await wrapsNow();
        await bend(false);
        if (caught.length === 0) {
          fail("probe dead: a cell forced onto several lines was not reported — the wrap check below cannot fail");
        } else if ((await wrapsNow()).length !== 0) {
          fail("the wrap probe did not put the cell back, so every width below is measuring the probe's own damage");
        }
      }
    }
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
      //   **And nothing in the table became two lines.** 2026-09-04:
      //   「表的内容,比如"内存"是不允许换行的」. A wrapped cell moves
      //   every value in its row off the line its neighbours are on,
      //   and a table is read by scanning down a column.
      const wrapped = await wrapsNow();
      if (wrapped.length) bad.push(`${w}:wrapped[${wrapped.join(",")}]`);
    }
    await page.setViewportSize(wide);
    await wait(200);
    //   One message for both kinds of finding: they are found on the
    //   same walk, and a width that does one often does the other.
    if (bad.length) fail(`${bad.length} widths are not clean: ${bad.slice(0, 8).join(" ")}`);
    await page.setViewportSize({ width: 380, height: 800 });
    await wait(120);
    const at380 = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    if (at380 > 0) console.log(`note: at 380 the shell is ${at380}px over — below the 390 this face is drawn for`);
    console.log(`bandwidth 390→1200 by 10: ${bad.length} widths flagged (sideways overflow or a cell on two lines)`);
  }

  //     ── the heads read as heads, and not as the figure's foot ────
  //
  //     2026-09-04:「表头要明显,现在看起来就是上面的散点图的 X 和 y 设置
  //     是下面表的表头」. Two rows of small muted words, one under the
  //     other, with nothing between them: the reader joins them, and
  //     「横 CPU 内存 显存 时长」 is read as the columns of the table.
  //
  //     Two things, and either alone leaves the complaint standing: the
  //     head has to **look different from the rows under it**, and the
  //     table has to be **visibly separated from the figure above it**.
  {
    await page.setViewportSize({ width: 1120, height: 1000 });
    await wait(300);
    const look = await page.evaluate(() => {
      const head = document.querySelector("[data-machine-card] [data-top-heads] > *");
      const cell = document.querySelector("[data-machine-card] [data-top-row] > *");
      const pick = document.querySelector("[data-machine-card] [data-plot-pick]");
      const seam = document.querySelector("[data-machine-card] [data-top-seam]");
      if (!head || !cell) return null;
      const cs = (n) => getComputedStyle(n);
      return {
        headWeight: +cs(head).fontWeight,
        rowWeight: +cs(cell).fontWeight,
        headColor: cs(head).color,
        rowColor: cs(cell).color,
        //   The rule under the heads, and the one above the table.
        headRule: cs(head).boxShadow,
        seamRule: seam ? +parseFloat(cs(seam).borderTopWidth || "0") : null,
        //   How far the table's head sits from the figure's foot,
        //   against how far two rows sit from each other. A break has
        //   to be bigger than the rhythm it is breaking.
        pickToHead:
          pick && head
            ? +(head.getBoundingClientRect().top - pick.getBoundingClientRect().bottom).toFixed(1)
            : null,
        rowPitch: (() => {
          const rows = [...document.querySelectorAll("[data-machine-card] [data-top-row]")].slice(0, 2);
          if (rows.length < 2) return null;
          return +(
            rows[1].firstElementChild.getBoundingClientRect().top - rows[0].firstElementChild.getBoundingClientRect().top
          ).toFixed(1);
        })(),
      };
    });
    if (!look) {
      fail("probe dead: no column head or no row, so 「the heads read as heads」 measured nothing");
    } else {
      //   **Heavier, darker, or both — and something must differ.** A
      //   head set exactly like the rows is the thing complained about.
      if (look.headWeight <= look.rowWeight && look.headColor === look.rowColor) {
        fail(
          `the column heads are set like the rows (weight ${look.headWeight} vs ${look.rowWeight}, ` +
            `ink ${look.headColor}) — nothing on screen says which line is the head`,
        );
      }
      if (!look.headRule || look.headRule === "none") {
        fail("the column heads carry no rule under them — the line that says 「rows start here」 is missing");
      }
      if (!(look.seamRule > 0)) {
        fail(
          `the table is not separated from the figure above it (seam rule ${look.seamRule}) — ` +
            `the scatter's axis words and the table's head are two rows of small text with nothing between them, ` +
            `and they get read as one`,
        );
      }
      if (look.pickToHead !== null && look.rowPitch !== null && look.pickToHead < look.rowPitch * 1.5) {
        fail(
          `the figure's axis words sit ${look.pickToHead}px above the table's head where a row is ${look.rowPitch}px tall — ` +
            `a break has to be bigger than the rhythm it breaks`,
        );
      }
      console.log(
        `heads: weight ${look.rowWeight}→${look.headWeight}, ink ${look.rowColor}→${look.headColor}, ` +
          `rule under them ${look.headRule !== "none"}, seam ${look.seamRule}px, ` +
          `axis words ${look.pickToHead}px above the head against a ${look.rowPitch}px row`,
      );
    }
  }

  //     ── the column heads do not go with the rows ─────────────────
  //
  //     2026-09-04:「拖动进程列表的时候,表头要悬浮,不要被拖走,这种基础
  //     的都搞错」. The process window is a fixed box that scrolls, and
  //     a head that leaves with the rows takes with it the only thing
  //     saying which column a number is in — **and the control that
  //     re-orders them**.
  //
  //     ⚠️ **Measured on the head's own box, and the reason is the bug
  //     it replaced.** `[data-top-heads]` wore `sticky top-0 bg-card`
  //     for as long as this table has existed and it was inert:
  //     `display: contents` gives that element **no box at all**, so it
  //     cannot be positioned and cannot paint a ground. Reading the
  //     class off the markup would have said 「sticky」; the head went
  //     from 737 to −2675.
  //
  //     Three things after the scroll, because the shallow version of
  //     this passes on a head that is pinned and useless: it is still
  //     **where it was**, it still **covers** the rows going under it
  //     (a transparent head is a head with rows written through it),
  //     and it still **sorts**.
  {
    await page.setViewportSize({ width: 1120, height: 1000 });
    await wait(300);
    const before = await page.evaluate(() => {
      const box = document.querySelector("[data-machine-card] [data-load-top-box]");
      const head = document.querySelector("[data-machine-card] [data-top-heads] > *");
      if (!box || !head) return null;
      return {
        scrollable: box.scrollHeight - box.clientHeight,
        top: +head.getBoundingClientRect().top.toFixed(1),
        firstRow: +document.querySelector("[data-machine-card] [data-top-row] > *").getBoundingClientRect().top.toFixed(1),
        opaque: getComputedStyle(head).backgroundColor,
      };
    });
    if (!before) {
      fail("probe dead: no process window or no column heads, so 「the heads stay」 measured nothing");
    } else if (before.scrollable < 40) {
      fail(`probe dead: the process window scrolls ${before.scrollable}px, so scrolling it proves nothing about the heads`);
    } else {
      const after = await page.evaluate(async () => {
        const box = document.querySelector("[data-machine-card] [data-load-top-box]");
        box.scrollTop = box.scrollHeight;
        await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
        const head = document.querySelector("[data-machine-card] [data-top-heads] > *");
        return {
          top: +head.getBoundingClientRect().top.toFixed(1),
          firstRow: +document.querySelector("[data-machine-card] [data-top-row] > *").getBoundingClientRect().top.toFixed(1),
          moved: box.scrollTop,
        };
      });
      if (Math.abs(after.top - before.top) > 1) {
        fail(`the column heads went from ${before.top} to ${after.top} while the rows were scrolled — they were dragged away with them`);
      }
      //   Probe alive: the rows really did move.
      if (Math.abs(after.firstRow - before.firstRow) < 40) {
        fail(`probe dead: the first row moved ${(after.firstRow - before.firstRow).toFixed(1)}px, so 「the heads stayed」 was true of a table nothing scrolled`);
      }
      const bg = before.opaque;
      if (!bg || bg === "rgba(0, 0, 0, 0)" || bg === "transparent") {
        fail(`the column heads have no ground of their own (${bg}) — the rows sliding under them are written through the words`);
      }
      //   And it still sorts, which is the half a pinned-but-dead head
      //   would pass without.
      const col = await page.evaluate(() => document.querySelector("[data-machine-card] [data-top-heads] > [data-top-head]")?.dataset.topHead);
      const rowsBefore = await page.evaluate(() =>
        [...document.querySelectorAll("[data-machine-card] [data-top-row]")].map((r) => r.dataset.topRow).join(","),
      );
      await page.locator(`[data-machine-card] [data-top-head="${col}"] button`).click();
      await wait(900);
      const rowsAfter = await page.evaluate(() =>
        [...document.querySelectorAll("[data-machine-card] [data-top-row]")].map((r) => r.dataset.topRow).join(","),
      );
      if (rowsAfter === rowsBefore) {
        fail(`pressing 「${col}」 after scrolling changed nothing — the head is pinned and no longer a control`);
      }
      console.log(
        `sticky heads: window scrolls ${before.scrollable}, heads ${before.top}→${after.top}, ` +
          `first row ${before.firstRow}→${after.firstRow}, ground ${bg}, still sorts`,
      );
    }
  }

  //     ── the table reaches the edge its neighbours reach ──────────
  //
  //     2026-09-04:「表那应该表根据屏幕铺开。现在全挤在左边了」— the
  //     table's columns summed to 692 in a 1378 box while the day
  //     chart, the drawer's bars and the scatter above it all ran the
  //     full width. A card whose four blocks stop at four different
  //     places reads as one of them having failed to load.
  //
  //     **Two assertions, and they are the two halves of the history
  //     here.** Width has to go somewhere, and every previous answer
  //     put it in the wrong place:
  //
  //       - inside the *name* track → 799px between a process's name
  //         and its CPU;
  //       - inside the first *quantity* track → the same hole one
  //         column along (时长 holding 123 for content wanting 78);
  //       - to the right of the last column → this complaint.
  //
  //     So: the last column must reach the box's edge (**it spreads**),
  //     and every gap *between* tracks must stay the one gap
  //     (**and not by opening a canyon**). The slack goes to 「命令」,
  //     the one column whose content is unbounded.
  //
  //     ⚠️ **What this does not buy, measured and not waved past:** on a
  //     row whose command is short the width still ends up as blank —
  //     it is simply blank *inside* one column rather than between two.
  //     Measured on turing at 1500: the widest row's ink uses 789 of
  //     the column's 959, and the median row's uses 124. The claim
  //     these two assertions support is 「the table ends where the card
  //     ends and no two columns drifted apart」, which is what was
  //     asked for — **not** 「there is no white space」.
  {
    const wide = { width: 1500, height: 1000 };
    await page.setViewportSize(wide);
    await wait(300);
    const spread = await page.evaluate(() => {
      const t = document.querySelector("[data-machine-card] [data-load-top]");
      if (!t) return null;
      const heads = [...document.querySelectorAll("[data-machine-card] [data-top-heads] > *")];
      if (heads.length < 4) return { heads: heads.length };
      const box = t.getBoundingClientRect();
      const rects = heads.map((h) => h.getBoundingClientRect());
      const gaps = rects.slice(1).map((r, i) => +(r.left - rects[i].right).toFixed(1));
      const probe = document.createElement("span");
      probe.style.cssText = "position:absolute;visibility:hidden;width:var(--gap-in)";
      t.appendChild(probe);
      const gap = probe.getBoundingClientRect().width;
      probe.remove();
      //   **How much room each track holds beyond the ink in it.** A
      //   `1fr` on a quantity column does not widen the gap *between*
      //   columns — the numbers are right-aligned, so the space opens
      //   **inside** the track, on its left. That is precisely the
      //   123-for-78 hole this section is about, and a gap-based probe
      //   is blind to it: measured, with `1fr` moved onto 时长, the
      //   gaps all still read 8 while that track held 591px for
      //   content wanting 78.
      const ink = (n) => {
        const r = document.createRange();
        r.selectNodeContents(n);
        const w = r.getBoundingClientRect().width;
        return Number.isFinite(w) ? w : 0;
      };
      const slackIn = {};
      for (let i = 0; i < heads.length; i += 1) {
        const key = heads[i].dataset.topHead ?? heads[i].dataset.c;
        const cells = [...document.querySelectorAll(`[data-machine-card] [data-top-row] [data-c="${key}"]`)];
        //   ⚠️ **The head's ink is inside its button, not its box.** A
        //   head carries a `w-full` button so the whole column head is
        //   pressable, so ranging over the `<th>` returns the *track's*
        //   width — and the difference below is then always ~0. Written
        //   that way first: with `1fr` moved onto 时长 the probe
        //   reported 24px of room in a track holding 592 for content
        //   wanting 102, and the assertion stayed green.
        const btn = heads[i].querySelector("button") ?? heads[i];
        const widest = Math.max(ink(btn), ...cells.map((c) => ink(c)), 0);
        slackIn[key] = +(rects[i].width - widest).toFixed(1);
      }
      return {
        heads: heads.length,
        boxW: +box.width.toFixed(1),
        slack: +(box.right - rects[rects.length - 1].right).toFixed(1),
        gaps,
        gap: +gap.toFixed(1),
        slackIn,
        cols: heads.map((h, i) => `${h.dataset.topHead ?? h.dataset.c}:${rects[i].width.toFixed(0)}`).join(" "),
      };
    });
    if (!spread || spread.heads < 4) {
      fail(`probe dead: the table shows ${spread ? spread.heads : 0} heads at 1500, so 「it spreads」 measured nothing`);
    } else {
      //   One character of tolerance, and no more: the point of the
      //   assertion is that the leftover is not parked here.
      if (spread.slack > 8) {
        fail(
          `at 1500 the table stops ${spread.slack}px short of its box (${spread.cols}) — ` +
            `the card's other blocks reach the edge and this one does not, which reads as a block that failed`,
        );
      }
      const canyon = spread.gaps.filter((g) => g > spread.gap + 1);
      if (canyon.length) {
        fail(
          `at 1500 ${canyon.length} of ${spread.gaps.length} column gaps are wider than the table's own gap ` +
            `(${spread.gaps.join("/")} against ${spread.gap}) — the slack went between two columns`,
        );
      }
      //   **And no track but 「命令」 holds room it has no ink for.**
      //   That is where the slack is allowed to be: the one column
      //   whose content has no bound. A quantity track wider than its
      //   widest number is the 799px hole, and it does not show up as a
      //   gap — see the probe.
      //
      //   60px of tolerance for the heads' own padding (`px-row` on the
      //   right-aligned ones) and the chevron beside the sorted word;
      //   the hole this catches is measured in hundreds.
      const fat = Object.entries(spread.slackIn).filter(([k, v]) => k !== "cmd" && v > 60);
      if (fat.length) {
        fail(
          `at 1500 ${fat.map(([k, v]) => `${k} holds ${v}px it has no ink for`).join(", ")} — ` +
            `the slack went inside a quantity column, which opens a hole to the left of its numbers ` +
            `(the gaps stay ${spread.gap} the whole time, so this is not visible as a gap)`,
        );
      }
      console.log(
        `table at 1500: box ${spread.boxW}, slack ${spread.slack}, gaps ${spread.gaps.join("/")} (gap ${spread.gap}), ` +
          `room beyond ink ${Object.entries(spread.slackIn).map(([k, v]) => `${k}:${v}`).join(" ")}`,
      );
    }
    await page.setViewportSize({ width: 1120, height: 1000 });
    await wait(200);
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

  //     ── no row paints a whole day as one flat slab ───────────────
  //
  //     **The failure this catches is not faintness, it is sameness.**
  //     Measured on the live fleet: turing drew 564 buckets across four
  //     rows and every one of them landed in the bottom band, so each
  //     row was a single shade from end to end. The block read as 「no
  //     data here」 — and a check comparing 「missing draws no ink」
  //     against 「low draws some ink」 is **green while that ships**
  //     (the gap is a real ΔL* 7.35). What is broken is along the row,
  //     so that is where it has to be measured.
  //
  //     ⚠️ Grouped by the row element, not by the rectangles' `y`. A
  //     drawing that puts two rects in one bucket would read as two
  //     rows to a `y`-based grouping and report each half flat — two
  //     false reds, on a chart that is fine.
  const flat = await page.evaluate(() => {
    return [...document.querySelectorAll("[data-machine-card] [data-day-row]")].map((row) => {
      const segs = [...row.querySelectorAll("[data-day-cell]")];
      return {
        name: row.dataset.dayRow,
        shades: [...new Set(segs.map((n) => n.dataset.dayShade))],
        drawn: segs.reduce((n, c) => n + (Number(c.dataset.dayTo) - Number(c.dataset.dayFrom)), 0),
      };
    });
  });
  {
    const series = { cpu: dayBuckets, gpu: gpuBuckets, mem: memBuckets, disk: diskBuckets, vram: vramBuckets };
    for (const r of flat) {
      const seen = (series[r.name] ?? []).filter((v) => v !== null);
      if (seen.length === 0 || r.drawn === 0) continue;
      const spread = Math.max(...seen) - Math.min(...seen);
      if (r.shades.length !== 1) continue;
      //   **The excuse is relative to the band the row sits in, not an
      //   absolute span.** An absolute threshold is wrong at both ends:
      //   low, it reds a machine that genuinely did not move; high, it
      //   waves through turing's video memory, which crosses **fifteen
      //   points** and still lands wholly inside one band — the exact
      //   defect the eighth band was cut for. So the question is what
      //   fraction of its own band the row's range fills: measured on
      //   the live fleet, 显存 is 14.9 of a 20-wide band (75%, a
      //   defect) and 磁盘 is 2.4 of a 12-wide one (21%, not).
      //
      //   turing's disk row is a flat slab and **that is correct**: a
      //   day that moves 2.4 points cannot be drawn by any quantised
      //   ramp — a hundred points cut into N bands puts a boundary
      //   inside 2.4 only past N=41, by which point neighbouring steps
      //   are below the eye's threshold. It is arithmetic, and a disk
      //   that gains 2.4 points in a day *should* look flat.
      //
      //   ⚠️ The boundaries are written here rather than read from the
      //   page on purpose — a gate that asks the implementation what
      //   its bands are agrees with it whatever they become. They must
      //   be changed here deliberately when `BANDS` changes, and the
      //   top-step assertion above is what notices if they have not.
      const edges = [3, 8, 20, 45, 70, 80, 90];
      const k = Number(r.shades[0]);
      if (!Number.isFinite(k)) continue;
      const lo = k === 0 ? 0 : edges[k - 1];
      const hi = k >= edges.length ? 100 : edges[k];
      const share = spread / (hi - lo);
      if (share > 0.5) {
        fail(
          `the ${r.name} row paints all ${r.drawn} of its buckets in one shade (${r.shades[0]}) ` +
            `while the readings span ${spread.toFixed(1)} points — ${Math.round(share * 100)}% of that band's own ` +
            `${hi - lo}-point width, so there was something to read and the ramp did not read it`,
        );
      }
    }
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
    //   **The drawing's own hit area, and the event it listens for.**
    //   The panel puts a transparent rect over the plot and reads
    //   `mousemove` on it; a synthetic `pointermove` at the `<svg>`
    //   reaches no listener and the readout stays empty — which looks
    //   exactly like a cursor that does not work.
    const put = () => {
      const hit = document.querySelector("[data-machine-card] [data-day-hit]");
      (hit ?? band()).dispatchEvent(
        new MouseEvent("mousemove", {
          bubbles: true,
          clientX: b.left + b.width / 3,
          clientY: b.top + b.height / 2,
        }),
      );
    };
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

  //     ── all three readings, and the words each one draws ─────────
  //
  //     **The section above switches between two of the three.** 叠放
  //     was never opened by anything here, and on 2026-09-04 that is
  //     where a `TypeError` lived: the drawing's three functions take
  //     the figure's pixel width in a parameter named `W`, and the
  //     module's words were in a global of the same name, so inside
  //     them `W.peakAt(…)` was a call on a number. It killed the whole
  //     card's subtree. The gap label, the shade legend and the strain
  //     legend were on the same fault and did not throw — they wrote
  //     the string 「undefined」 into the figure and everything stayed
  //     green.
  //
  //     Nothing above would have caught either half, and not because
  //     an assertion was wrong: **every one of them measures a number
  //     or a position** — cells, coverage, spacing, heights, the
  //     viewBox, the cursor's moment. None asks whether the figure
  //     says anything. So this section asks two things, on all three
  //     readings:
  //
  //       - **no word came out missing**, which on this drawing means
  //         **empty**, not the string 「undefined」. `txt()` assigns
  //         `textContent`, and that attribute is nullable, so `undefined`
  //         converts to `null` and `null` to `""` — the three labels on
  //         the fault rendered as **empty `<text>` elements**, invisible
  //         and silent. A sweep for the word 「undefined」 is the
  //         obvious assertion here and it is a blind one: it cannot
  //         fire on the bug it is named after. (It is kept alongside,
  //         for the ways a word can arrive wrong rather than absent.)
  //       - **叠放 says its peak in words**, which is the one call the
  //         crash was in. 热力带 and 山脊 print the bare number there
  //         on purpose (`day-peak-at` is only the parallel figure's,
  //         where a hair is a moment and 「什么时候」 has to be
  //         written), so the two arms of this are opposite: bare there,
  //         a moment as well here. A sentence that lost its call comes
  //         back a bare number, and a bare number that grew one is the
  //         other mistake — one assertion, and it fails both ways.
  //
  //     ⚠️ **At a width where the figure is not in short mode.** The
  //     block above leaves the viewport at 380, and at 380 the drawing
  //     skips `peakAt` by design — the assertion would have measured a
  //     branch that was never the broken one.
  await page.setViewportSize({ width: 1400, height: 900 });
  await wait(300);
  const beforeSwitching = thrown.length;
  const spoken = await page.evaluate(async () => {
    const pick = (k) => document.querySelector(`[data-machine-card] [data-day-view-pick="${k}"]`);
    const frame = () => new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    const out = {};
    for (const view of ["heat", "ridge", "para"]) {
      pick(view)?.click();
      await frame();
      await new Promise((ok) => setTimeout(ok, 120));
      const band = document.querySelector("[data-machine-card] [data-day-band]");
      out[view] = {
        drawn: !!band,
        view: band?.dataset.dayView ?? null,
        //   Each text element on its own, so a failure can name the one
        //   that is empty rather than a wall of the whole figure.
        texts: band ? [...band.querySelectorAll("text")].map((t) => t.textContent) : [],
      };
    }
    pick("heat")?.click();
    await frame();
    return out;
  });
  for (const view of ["heat", "ridge", "para"]) {
    const seen = spoken[view];
    if (!seen?.drawn) {
      fail(`picking ${view} left no figure at all — nothing below this was measured for it`);
      continue;
    }
    if (seen.view !== view) fail(`picking ${view} left the band saying it is ${seen.view}`);
    //   **Probe alive.** A figure with no words in it passes an
    //   「no word is undefined」 sweep for the wrong reason, and that is
    //   the exact shape the bug had in 叠放 (the subtree was gone).
    if (seen.texts.length < 4) {
      fail(`probe dead: ${view} draws ${seen.texts.length} text elements, so 「its words are there」 measured nothing`);
      continue;
    }
    const wrong = seen.texts.filter((t) => /undefined|\[object Object\]|\bNaN\b/.test(t ?? ""));
    if (wrong.length) {
      fail(
        `${view} writes ${wrong.length} of its ${seen.texts.length} words as a missing value ` +
          `(${wrong.slice(0, 3).map((t) => JSON.stringify(t)).join(", ")}) — a word the drawing never got`,
      );
    }
    const empty = seen.texts.filter((t) => !String(t ?? "").trim()).length;
    if (empty > 0) {
      fail(
        `${view} draws ${empty} of its ${seen.texts.length} <text> elements with nothing in them — ` +
          `a label positioned, styled and animated around a word that never arrived`,
      );
    }
    //   A peak said with its moment: the value and an `hh:mm`, in one
    //   text element. 热力带/山脊 must not have one; 叠放 must.
    const withMoment = seen.texts.filter((t) => /\d+\s*%.*\d{2}:\d{2}/.test(t ?? ""));
    if (view === "para" && withMoment.length === 0) {
      fail(
        `叠放 prints no peak with its moment (${seen.texts.length} words, none of them 「value · hh:mm」) — ` +
          `on that figure a hair is a moment and the time is compressed out, so a bare number answers 多高 and never 什么时候`,
      );
    }
    if (view !== "para" && withMoment.length > 0) {
      fail(
        `${view} prints ${withMoment.length} peaks with a clock on them (${JSON.stringify(withMoment[0])}) — ` +
          `on a band whose x *is* the clock, the moment is where the mark already is`,
      );
    }
  }
  if (thrown.length > beforeSwitching) {
    fail(`changing the reading threw: ${thrown.slice(beforeSwitching).join(" | ")}`);
  }
  //   **The line reports, it does not conclude.** It used to end 「none
  //   missing」 and printed that verbatim on a run where three of these
  //   assertions had just failed — a summary that states the finding
  //   rather than the reading will eventually state the wrong one.
  console.log(
    `day readings: ${["heat", "ridge", "para"]
      .map((v) => {
        const t = spoken[v]?.texts ?? [];
        return `${v} ${t.length} words / ${t.filter((s) => !String(s ?? "").trim()).length} empty`;
      })
      .join(", ")}`,
  );
  await page.setViewportSize({ width: 380, height: 800 });
  await wait(200);

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

  //     ── the ruler is chosen, said, and lands zero at zero ────────
  //
  //     2026-09-04:「因 X 或 y scale 不一无法绘制出有意义的图的时候,用
  //     log 或什么方法非线性的绘制呀」.
  //
  //     Four things, and each is one of the ways this could be done
  //     wrong:
  //
  //       - **it is decided by measurement, not always on.** A log axis
  //         on data that did not need one is its own distortion, so a
  //         quantity whose spread the square root handles must stay on
  //         the square root;
  //       - **zero is at zero.** `log 0` is undefined and 0 is the
  //         commonest value in this table; a ruler that fudged it with
  //         `v + 1` would put every reading slightly off the place its
  //         own label claims;
  //       - **the ruler is written on the axis.** Whether it is needed
  //         depends on that machine's numbers, so two machines can be
  //         two rulers — and then a position means two things unless it
  //         is said;
  //       - **the log actually spreads them.** The whole point is the
  //         crowding, and 「we changed the maths」 is not evidence that
  //         it helped.
  {
    const read = () =>
      page.evaluate(() => {
        const plot = document.querySelector("[data-machine-card] [data-top-plot]");
        if (!plot) return null;
        const pts = [...plot.querySelectorAll("[data-plot-point]")].map((g) => ({
          x: Number(g.dataset.plotX),
          y: Number(g.dataset.plotY),
        }));
        let close = 0;
        for (let i = 0; i < pts.length; i += 1) {
          for (let j = i + 1; j < pts.length; j += 1) {
            if (Math.hypot(pts[i].x - pts[j].x, pts[i].y - pts[j].y) < 6) close += 1;
          }
        }
        return {
          pair: plot.dataset.plotAxes,
          sx: plot.dataset.plotScaleX,
          sy: plot.dataset.plotScaleY,
          said: [...plot.querySelectorAll("[data-plot-ruler]")].map((t) => `${t.dataset.plotRuler}:${t.textContent}`).join(" "),
          n: pts.length,
          close,
          //   Where the x axis's zero tick sits, and where the leftmost
          //   point sits. On a ruler that put zero anywhere else the
          //   two would not meet.
          zeroTickX: (() => {
            const g = [...plot.querySelectorAll("[data-plot-tick-x]")].find((n) => Number(n.dataset.plotTickX) === 0);
            return g ? +(+g.querySelector("line").getAttribute("x1")).toFixed(2) : null;
          })(),
          minPointX: pts.length ? Math.min(...pts.map((p) => p.x)) : null,
          //   Where the axis itself starts — the vertical rule at the
          //   plot's left edge. `at(0)` has to land exactly there, and
          //   comparing the zero tick with the *points* would not show
          //   it: both come from the same transform, so a ruler shifted
          //   off its origin moves them together.
          axisX: (() => {
            const lines = [...plot.querySelectorAll("svg > line")];
            const v = lines.find((l) => l.getAttribute("x1") === l.getAttribute("x2"));
            return v ? +(+v.getAttribute("x1")).toFixed(2) : null;
          })(),
        };
      });
    const pick = async (which, key) => {
      await page.locator(`[data-machine-card] [data-plot-pick="${which}"] [data-plot-axis="${key}"]`).click();
      await wait(300);
    };
    await pick("x", "cpu");
    await pick("y", "mem");
    const now = await read();
    if (!now) {
      fail("probe dead: no plot, so nothing about its rulers was measured");
    } else {
      if (!/x:/.test(now.said) || !/y:/.test(now.said)) {
        fail(`the plot does not say which ruler each axis is on (${JSON.stringify(now.said)}) — two machines can be two rulers`);
      }
      //   **Both rulers have to be reachable in one run**, or the
      //   choosing is not being tested — only whichever one this
      //   fixture happens to want.
      const seen = new Set([now.sx, now.sy]);
      const crowd = { [`${now.pair} ${now.sx}×${now.sy}`]: `${now.close}/${now.n}` };
      await pick("y", "time");
      const onTime = await read();
      seen.add(onTime.sx).add(onTime.sy);
      crowd[`${onTime.pair} ${onTime.sx}×${onTime.sy}`] = `${onTime.close}/${onTime.n}`;
      if (seen.size < 2) {
        fail(
          `every axis in this fixture came out on the same ruler (${[...seen].join(",")}) — ` +
            `「it is chosen by measurement」 was not tested, only 「it is always this one」`,
        );
      }
      //   **Zero at zero.** The `0` tick and the leftmost point have to
      //   agree; a `log(v+1)` ruler puts the tick where `log 1` is and
      //   the point somewhere else.
      if (now.zeroTickX === null || now.axisX === null) {
        fail(`probe dead: no zero tick (${now.zeroTickX}) or no axis line (${now.axisX}), so 「zero is at zero」 measured nothing`);
      } else {
        if (Math.abs(now.zeroTickX - now.axisX) > 0.6) {
          fail(
            `the reading 0 is drawn at x=${now.zeroTickX} while the axis begins at ${now.axisX} — ` +
              `a ruler that cannot put zero at its own origin (log(v+1) is the usual way in) has labels that lie about where they sit`,
          );
        }
        if (now.minPointX !== null && now.minPointX < now.axisX - 0.6) {
          fail(`a point sits at x=${now.minPointX}, left of the axis at ${now.axisX}`);
        }
      }
      console.log(`plot rulers: ${Object.entries(crowd).map(([k, v]) => `${k} ${v} crowded`).join(", ")}, said 「${now.said}」`);
      await pick("y", "mem");
      await wait(200);
    }
  }

  //     ── the two axes are the reader's, and changing one changes ──
  //        the picture, the ruler and the sentence together
  //
  //     2026-09-04:「表要可以设定 x 和 y 轴」. Four candidates on each
  //     axis, the four quantities the table has columns for.
  //
  //     **Three things have to move together, and each of them alone
  //     would pass on a broken build:**
  //
  //       - the *ticks*, because a picture redrawn against a new
  //         quantity under the old ruler's labels is the worst thing
  //         this chart could do;
  //       - the *points*, because ticks alone can be relabelled while
  //         every dot stays where it was;
  //       - the *word*, because the foot is the only place the picture
  //         says what it is of.
  //
  //     **And the rows that have no reading on an axis must be absent,
  //     not at zero.** 显存 is the case that matters: measured on the
  //     live fleet, 200 of 200 rows carry no card memory on two of the
  //     three machines. Drawn at zero they would pile on the origin and
  //     read as 「two hundred processes using none of the card」, which
  //     is the neighbouring-value mistake. So: fewer points, and a foot
  //     that says how many fewer.
  {
    const read = () =>
      page.evaluate(() => {
        const plot = document.querySelector("[data-machine-card] [data-top-plot]");
        if (!plot) return null;
        return {
          pair: plot.dataset.plotAxes,
          xTicks: [...plot.querySelectorAll("[data-plot-tick-x] text")].map((t) => t.textContent).join(","),
          yTicks: [...plot.querySelectorAll("[data-plot-tick-y] text")].map((t) => t.textContent).join(","),
          points: plot.querySelectorAll("[data-plot-point]").length,
          xs: [...plot.querySelectorAll("[data-plot-point]")].map((g) => g.dataset.plotX).join(","),
          chosen: [...plot.querySelectorAll('[data-plot-axis][data-on="true"]')].map((b) => b.dataset.plotAxis).join(","),
          off: plot.querySelector("[data-plot-off]")?.dataset.plotOff ?? null,
          words: [...plot.querySelectorAll("[data-plot-pick] [data-plot-axis]")].length,
        };
      });
    const pick = async (which, key) => {
      await page.locator(`[data-machine-card] [data-plot-pick="${which}"] [data-plot-axis="${key}"]`).click();
      await wait(250);
    };
    const start = await read();
    if (!start || start.words !== 8) {
      fail(`probe dead: the plot offers ${start ? start.words : 0} axis words, not two sets of four`);
    } else {
      if (start.pair !== "cpuxmem") fail(`the plot opens on ${start.pair} — the pair every machine can be read against is cpu×mem`);
      await pick("y", "time");
      const onTime = await read();
      if (onTime.yTicks === start.yTicks) {
        fail(`picking 时长 left the vertical ruler reading ${start.yTicks} — the labels are the old quantity's`);
      }
      if (onTime.xs === start.xs) fail("picking 时长 moved no point — the ruler changed and the picture did not");
      if (onTime.chosen !== "cpu,time") fail(`the foot says the axes are ${onTime.chosen} after picking 时长`);
      //   **A quantity most rows do not have.** The fixture's rows carry
      //   `gpu_mem` on some and `null` on the rest, which is the shape
      //   the fleet has.
      await pick("x", "vram");
      const onVram = await read();
      if (onVram.points >= start.points) {
        fail(
          `on 显存 the plot still draws ${onVram.points} of ${start.points} points — ` +
            `a row with no card reading is being placed, and the only place to put it is zero`,
        );
      }
      const off = Number(onVram.off ?? -1);
      if (off !== start.points - onVram.points) {
        fail(
          `${start.points - onVram.points} rows were left off the picture and the foot says ${onVram.off} — ` +
            `a chart quietly missing its rows draws the same as an idle machine`,
        );
      }
      //   **The pair survives the card going away and coming back.**
      //   Kept in the module rather than in the card for exactly this:
      //   three cards read one after another must be three pictures of
      //   the same thing.
      //
      //   ⚠️ **Leaving the pane, not just opening another machine.**
      //   React keeps the card mounted when the machine changes under
      //   it — same component, same position — so `useState` inside it
      //   survives that too, and the first version of this assertion
      //   passed with the shared store swapped out for local state.
      //   Going to the sessions landing and back is what actually
      //   unmounts it.
      await open("sessions");
      await wait(200);
      await open("devices");
      //   Row 1, not row 2: row 2 is the machine that refuses to answer
      //   `load`, so it draws no plot at all and the read below comes
      //   back `null` — which is how this assertion first came to be
      //   skipped rather than run.
      await pressNth("[data-list] [data-row-open]", 1);
      await until(8_000, async () => (await page.locator("[data-machine-card] [data-top-plot]").count()) === 1);
      const elsewhere = await read();
      //   `null` is not 「fine」 — it is 「there was no plot to ask」,
      //   and a guard that skips on it is a section that stops being
      //   evaluated the day the navigation drifts.
      if (!elsewhere) {
        fail("probe dead: no plot after leaving the pane and coming back, so 「the pair survives」 measured nothing");
      } else if (elsewhere.chosen !== "vram,time") {
        fail(`the axes came back as ${elsewhere.chosen} after leaving the pane — the pair belongs to the reader, not to one card`);
      }
      await open("devices");
      await pressNth("[data-list] [data-row-open]", 1);
      await until(5_000, async () => (await page.locator("[data-machine-card] [data-top-plot]").count()) === 1);
      await pick("x", "cpu");
      await pick("y", "mem");
      const back = await read();
      if (back.chosen !== "cpu,mem") fail(`the axes could not be put back (${back.chosen})`);
      //   Reports; does not conclude. (Third time this batch.)
      console.log(
        `plot axes: cpu×mem ${start.points} points → cpu×时长 ruler ${onTime.yTicks.slice(0, 24)}… → ` +
          `显存×时长 ${onVram.points} points with ${onVram.off} said to be off; after leaving the pane ${elsewhere ? elsewhere.chosen : "no plot"}`,
      );
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
      //   ⚠️ **The line, not the cell and not the two inks.** The cell
      //   is a flex box that never overflows, and the name and the word
      //   are inline spans inside one clipping line — neither of them
      //   overflows on its own either. `[data-cmd-line]` is the box
      //   that clips, and it is the only one whose `scrollWidth` means
      //   「there is more of this」.
      //
      //   This assertion has now been pointed at the wrong element
      //   twice, once per shape of the cell, and each time it reported
      //   a correctly-marked row as decoration. **The element that
      //   clips is the element to ask** — not its parent, not its
      //   children.
      const clip = c.querySelector("[data-cmd-line]");
      const over = !!clip && clip.scrollWidth - clip.clientWidth > 1;
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
  //   Each drawer's heading, and how it counts what is under it. Read
  //   in the loop below, judged after it — the claim is about the two
  //   kinds being told apart, and one drawer alone cannot carry it.
  const grammar = [];
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
        //   What the rows under this heading are, and how the heading
        //   counts them — see the section below.
        kind: drawer.querySelector("[data-drawer-head]")?.dataset.drawerKind ?? null,
        counts: head?.textContent?.trim() ?? "",
      };
    });
    drawerHeights.push(`${key}:${seen.h}`);
    grammar.push({ key, kind: seen.kind, counts: seen.counts });
    //   **Every group, not just whichever is open at the end.** The
    //   first version read only the last one the loop left showing, so
    //   a heading lying on one of the other three was never looked at —
    //   an assertion pointed at the wrong element passes for reasons
    //   that have nothing to do with what it claims.
    //   **The 「more below」 note is a state, not decoration.** It has
    //   to be absent when everything fits, present when it does not,
    //   and its number has to be what is actually out of sight —
    //   counted on the cells, because a row here has `display:
    //   contents` and no box of its own to measure.
    const more = await page.evaluate(() => {
      const drawer = document.querySelector("[data-machine-card] [data-drawer]");
      const scroller = drawer.querySelector("[data-drawer-scroll]");
      const open = drawer.querySelector("[data-parts]:not([hidden])");
      const bottom = scroller.getBoundingClientRect().bottom;
      let below = 0;
      for (const row of open.querySelectorAll("[data-part]")) {
        const cell = row.firstElementChild;
        // Same rule as the hook: a row clipped by the edge is below.
        // (Writing the assertion with the *other* rule would have
        // agreed with the bug — which is what happened first time, and
        // why a screenshot caught it instead of this.)
        if (cell && cell.getBoundingClientRect().bottom > bottom + 1) below += 1;
      }
      const note = drawer.querySelector("[data-drawer-more]");
      return {
        below,
        says: note ? Number(note.dataset.drawerMore) : 0,
        //   **Whether it is on screen at all**, separately from what it
        //   says — see below.
        shown: !!note,
        words: note?.textContent?.trim() ?? "",
        overflows: scroller.scrollHeight - scroller.clientHeight > 1,
      };
    });
    if (more.says !== more.below) {
      fail(`the ${key} drawer says ${more.says} more below and ${more.below} rows are out of sight`);
    }
    if (!more.overflows && more.says !== 0) {
      fail(`the ${key} drawer fits entirely and still offers to be dragged`);
    }
    //   **The hint is there or it is not, and that is now the whole
    //   signal.** It used to read out the number — 「还有 3 个在下面,
    //   拖一下」 — and 2026-09-04 the words became 「下滑查看更多」
    //   (「你不觉得这个画很蠢吗,不要这个」). The number stayed, on
    //   `data-drawer-more`, which is what the two assertions above
    //   read.
    //
    //   ⚠️ **Those two do not cover this one, and did not cover it
    //   before either.** Both are about the number: a hint rendered
    //   unconditionally with `data-drawer-more={0}` satisfies each of
    //   them — `says === below === 0`, and `says !== 0` is false — and
    //   sits on screen under every drawer that fits. While the number
    //   was in the sentence that was at least visibly wrong (「还有 0
    //   个在下面」); with the words no longer counting anything, a
    //   permanent hint is indistinguishable from a correct one to
    //   everything above, and 「一句常驻的提示等于没有提示」 is exactly
    //   what it would become. So: presence against the fact.
    if (more.shown !== more.below > 0) {
      fail(
        more.shown
          ? `the ${key} drawer offers 「${more.words}」 with nothing out of sight — a hint that is always there is part of the heading, not a state`
          : `the ${key} drawer hides ${more.below} rows and says nothing about them`,
      );
    }
    if (seen.says !== seen.rows) {
      fail(`the ${key} drawer's heading says ${seen.says} but draws ${seen.rows} rows — the count is about something else`);
    }
  }
  {
    const hs = new Set(drawerHeights.map((x) => x.split(":")[1]));
    if (drawerHeights.length < 2) fail(`probe dead: only ${drawerHeights.length} drawers could be opened`);
    else if (hs.size !== 1) fail(`the drawers are ${drawerHeights.join(" ")} — pressing around makes the card jump`);
  }
  //     ── a drawer of occupants says so in its heading ─────────────
  //
  //     **Six occupants and six parts draw identically.** A card and a
  //     volume are parts of the ring above and their readings add up to
  //     it; a process is only an occupant, and the handful listed do
  //     not add up to anything — there are several thousand more. Two
  //     blocks that say different things must not look the same.
  //
  //     ⚠️ **This replaced a footnote**, not a hole. The sentence
  //     「占用最多的几个,加起来不等于上面那个环」 used to sit under
  //     these two drawers (`top-share-note`); 2026-09-04 it was struck
  //     (「也不要,很蠢」) and the fact moved into the heading. Nothing
  //     guarded the footnote — deleting it would have reddened nothing,
  //     which is exactly why the replacement needs a gate.
  //
  //     **What is measured is the count's grammar, not its words.** A
  //     parts heading counts bare — 「3 卷」, the number first, and when
  //     you have counted them the ring is accounted for. An occupants
  //     heading carries a qualifier ahead of the number — 「占用最多的
  //     6 个进程」 — which says on its face that these are the top few
  //     out of more. So: does the count begin with its number.
  //
  //     **Both arms, because one arm proves nothing.** Asking only that
  //     the occupants' heading be qualified passes for a build that
  //     qualified all four, and that build has erased the very
  //     distinction this section is about while reading as a pass. The
  //     two arms have to come out *different*, and the gate says which
  //     way round.
  {
    const digit = (s) => /^[\d０-９]/.test(s);
    const occ = grammar.filter((g) => g.kind === "occupants");
    const part = grammar.filter((g) => g.kind === "parts");
    if (occ.length === 0 || part.length === 0) {
      fail(
        `probe dead: the drawers report kinds ${JSON.stringify(grammar.map((g) => `${g.key}=${g.kind}`))} — ` +
          `with only one kind on screen, 「the two are told apart」 measured nothing`,
      );
    } else {
      for (const g of occ) {
        if (digit(g.counts)) {
          fail(
            `the ${g.key} drawer holds occupants and counts them bare ("${g.counts}") — ` +
              `six processes then read as the six things the ring is made of, and they are not: ` +
              `there are thousands more and these few do not add up to it`,
          );
        }
      }
      for (const g of part) {
        if (!digit(g.counts)) {
          fail(
            `the ${g.key} drawer holds parts and qualifies its count ("${g.counts}") — ` +
              `if both kinds hedge, neither says anything, and the one distinction this heading carries is gone`,
          );
        }
      }
    }
  }
  //     **Open one first.** Since the 24-hour figure moved into this
  //     slot (2026-09-04) the card arrives with **no ring pressed** —
  //     the slot shows the day — so the sections below, which are about
  //     the drawer, have to ask for one. They used to inherit whatever
  //     the loop above left open, and the first run after the change
  //     crashed here on a `null` drawer: state that arrives by accident
  //     is state a section is not really testing.
  {
    const dial = page.locator('[data-machine-card] [data-dial="cpu"]').first();
    if ((await dial.getAttribute("data-open")) !== "true") await dial.click();
    await wait(250);
    if ((await page.locator("[data-machine-card] [data-parts]:not([hidden])").count()) !== 1) {
      fail("probe dead: pressing the CPU ring opened no drawer, so every drawer section below measured nothing");
    }
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
  //   And ask for a drawer again after the resize — see the note above
  //   the previous press. Nothing about a width should close one, but
  //   this section's whole subject is inside an open drawer and it is
  //   cheaper to ask than to inherit.
  {
    const dial = page.locator('[data-machine-card] [data-dial="cpu"]').first();
    if ((await dial.getAttribute("data-open")) !== "true") await dial.click();
    await wait(250);
  }
  const edges = await page.evaluate(() => {
    const drawer = document.querySelector("[data-machine-card] [data-drawer]");
    const open = drawer && drawer.querySelector("[data-parts]:not([hidden])");
    if (!open) return { dead: true, drawer: drawer ? drawer.dataset.drawer : null };
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
  if (edges.dead) {
    fail(`probe dead: no drawer is open at 1600 (data-drawer=${JSON.stringify(edges.drawer)}), so the shared grid was never measured`);
  } else if (edges.rows < 2) fail(`probe dead: the open drawer has ${edges.rows} rows, so "one value each" is trivially true`);
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

  //     ── the 24-hour figure, which shares a slot with the drawers ──
  //
  //     **Every section below needs the figure showing.** Since the
  //     merge (2026-09-04) it lives in the slot under the four rings and
  //     a pressed ring hides it — hidden, so it is still in the DOM and
  //     every selector still finds it, while **`getBBox` returns 0×0 for
  //     all of it and the layout numbers are the ones from before it was
  //     hidden**. That is the worst kind of state to measure in: the
  //     probes do not come back empty, they come back stale.
  //
  //     So: put the slot back on the day, and refuse to continue if it
  //     did not go back.
  {
    const openDial = page.locator('[data-machine-card] [data-dial][data-open="true"]').first();
    if (await openDial.count()) await openDial.click();
    await wait(300);
    const showing = await page.evaluate(() => {
      const d = document.querySelector("[data-machine-card] [data-load-day]");
      return d ? { hidden: d.hidden, w: Math.round(d.getBoundingClientRect().width) } : null;
    });
    if (!showing || showing.hidden || showing.w < 60) {
      fail(`probe dead: the 24-hour figure is not showing (${JSON.stringify(showing)}), so every reading of it below is stale`);
    }
    //   ── all three readings end where the slot ends ─────────────
    //
    //   2026-09-04:「24小时趋势图,不要满足最高高度,而导致部分图下面空掉
    //   了。可以全部图画到等高。不然视觉一致性很差」. The block has one
    //   height, and the three drawings are not naturally the same size
    //   (92 / 108 / 153 for four rows), so two of them used to stop in
    //   mid-air with a strip of nothing under them.
    //
    //   **Measured on the ink, not on the box.** The box is `height:
    //   100%` in every case — it was already the same in every reading
    //   while the strip was plainly there, so an assertion on it passes
    //   through the bug. What has to match is where the drawing's own
    //   bounding box stops: **the distance from the last ink to the
    //   slot's floor, the same in all three, to the pixel.**
    //
    //   One pixel of tolerance and no more. The three fill by different
    //   arithmetic — a row pitch, a stagger, an axis length — and a
    //   looser bound would let one of them drift a visible amount while
    //   the assertion still called it 「等高」.
    {
      const floorOf = () =>
        page.evaluate(() => {
          const slot = document.querySelector("[data-machine-card] [data-ring-slot]");
          const svg = document.querySelector("[data-machine-card] [data-load-day] [data-day-band]");
          if (!slot || !svg) return null;
          //   Only what is painted, and in page coordinates: the
          //   cursor's parked dots are at `opacity:0` and are not ink.
          let bottom = -Infinity;
          for (const n of svg.querySelectorAll("*")) {
            if (n.getAttribute("opacity") === "0") continue;
            const r = n.getBoundingClientRect();
            if (r.width === 0 && r.height === 0) continue;
            bottom = Math.max(bottom, r.bottom);
          }
          if (!Number.isFinite(bottom)) return null;
          const box = slot.getBoundingClientRect();
          //   **What is allowed to be between the last ink and the
          //   floor**: the cursor's readout line and the room above it,
          //   and nothing else. Read off the element rather than
          //   written here, so the number cannot drift from the
          //   stylesheet.
          const ro = document.querySelector("[data-machine-card] [data-load-day] .ro");
          const roH = ro ? ro.getBoundingClientRect().height + (parseFloat(getComputedStyle(ro).marginTop) || 0) : 0;
          return {
            gap: +(box.bottom - bottom).toFixed(1),
            allowed: +roH.toFixed(1),
            svgH: Math.round(svg.getBoundingClientRect().height),
          };
        });
      const floors = {};
      for (const view of ["heat", "ridge", "para"]) {
        await page.locator(`[data-machine-card] [data-day-view-pick="${view}"]`).click();
        await wait(500);
        floors[view] = await floorOf();
      }
      const got = Object.entries(floors).filter(([, v]) => v !== null);
      if (got.length !== 3) {
        fail(`probe dead: only ${got.length} of the three readings could be measured (${JSON.stringify(floors)})`);
      } else {
        const gaps = got.map(([, v]) => v.gap);
        const spread = Math.max(...gaps) - Math.min(...gaps);
        if (spread > 1) {
          fail(
            `the three readings stop at ${got.map(([k, v]) => `${k} ${v.gap}`).join(", ")} px above the slot's floor ` +
              `— a spread of ${spread.toFixed(1)}, so switching reading opens a strip of nothing under two of them`,
          );
        }
        //   ⚠️ **Equal is not enough, and this is the half that was
        //   missing.** Written with only the spread, the section passed
        //   on a build where all three under-filled by the same 5px —
        //   three drawings agreeing on where to stop short is still the
        //   strip of nothing the reader complained about. So the gap
        //   itself is bounded by the one thing that belongs there.
        const tall = got.filter(([, v]) => v.gap > v.allowed + 1);
        if (tall.length) {
          fail(
            `${tall.map(([k, v]) => `${k} stops ${v.gap}px above the floor where only the readout's ${v.allowed}px belongs`).join(", ")} ` +
              `— the drawing is not filling the box it was given`,
          );
        }
        console.log(
          `day fill: ${got.map(([k, v]) => `${k} svg ${v.svgH} ink ${v.gap}/${v.allowed} off the floor`).join(", ")}`,
        );
      }
    }

    //   **And it fits the slot it shares, in all three readings.** The
    //   slot is one height for the drawings and the drawers both, so
    //   the tallest reading is what sets it — and a reading that
    //   outgrows it is silently clipped at the bottom, where its
    //   readout line lives. Measured this way because the number that
    //   sets the height is arrived at by adding up parts, and one of
    //   them (the readout's `margin-top`) is not in any element's
    //   height: 叠放 overflowed by exactly that 6px on the first try.
    for (const view of ["heat", "ridge", "para"]) {
      await page.locator(`[data-machine-card] [data-day-view-pick="${view}"]`).click();
      await wait(400);
      const over = await page.evaluate(() => {
        const blk = document.querySelector("[data-machine-card] [data-load-day] .dayblk");
        if (!blk) return null;
        return { over: blk.scrollHeight - blk.clientHeight, parts: [...blk.children].map((c) => `${c.className}:${Math.round(c.getBoundingClientRect().height)}`) };
      });
      if (!over) fail(`probe dead: no day block while showing ${view}`);
      else if (over.over > 0) {
        fail(`叠放/山脊/热力带 — ${view} is ${over.over}px taller than the slot it shares (${over.parts.join(" ")}) — its foot is clipped`);
      }
    }
    await page.locator('[data-machine-card] [data-day-view-pick="heat"]').click();
    await wait(300);
  }
  //     ── the band is a heat band: the value is the ink ────────────
  //
  //     **A · the shade really encodes the number.** Pinned to two
  //     named buckets rather than 「at least two shades exist」, which
  //     a implementation that coloured by bucket index would also
  //     satisfy.
  const shades = await page.evaluate(() => {
    const row = document.querySelector('[data-machine-card] [data-day-row="cpu"]');
    if (!row) return null;
    //   **A cell is a run of buckets, not a bucket.** The drawing merges
    //   neighbours that land on the same step into one rect — the
    //   prototype's own arrangement, kept because it is the drawing that
    //   was moved across. So a cell carries the half-open range it
    //   covers, and 「what shade is bucket 58」 is a lookup, not an index.
    const segs = [...row.querySelectorAll("[data-day-cell]")].map((r) => ({
      from: Number(r.dataset.dayFrom),
      to: Number(r.dataset.dayTo),
      x: Number(r.getAttribute("x")),
      w: Number(r.getAttribute("width")),
      h: Number(r.getAttribute("height")),
      key: r.dataset.dayShade,
    }));
    const at = (i) => segs.find((sg) => i >= sg.from && i < sg.to)?.key ?? null;
    //   **The band is one drawing**, so the reading's name and the hour
    //   marks belong to the figure and are read from it. Asked of a row
    //   they come back empty, and an empty answer reads like 「it is not
    //   there」 — the same trap as a renamed selector.
    const band = document.querySelector("[data-machine-card] [data-day-band]");
    return {
      // the fixture's cpu series is 5 + (i % 60), so bucket 0 is 5 and
      // bucket 58 is 63 — a low one and a high one, by construction.
      low: at(0),
      high: at(58),
      segs,
      covered: segs.reduce((n, sg) => n + (sg.to - sg.from), 0),
      heights: [...new Set(segs.map((sg) => sg.h))],
      views: [band.dataset.dayView],
      //   The hour marks' own x, so the claim can be about spacing
      //   rather than about a count that only holds for one clock.
      //   Sorted: the drawing lists its hours in the order the clock
      //   reaches them, which is not left-to-right for a window that
      //   crosses midnight.
      hourX: [...band.querySelectorAll("[data-day-hour]")]
        .map((l) => Number(l.getAttribute("x1")))
        .sort((a, b) => a - b),
      x0: segs.length ? segs[0].x : 0,
      x1: segs.length ? segs[segs.length - 1].x + segs[segs.length - 1].w : 0,
    };
  });
  if (!shades || shades.segs.length === 0) {
    fail("probe dead: the cpu row has no cells to read, so nothing below was measured");
  } else {
  if (shades.low === null || shades.high === null) {
    fail(`probe dead: the cpu row has no shaded cells to read (low=${shades.low} high=${shades.high})`);
  } else if (shades.low === shades.high) {
    fail(`bucket 0 (5%) and bucket 58 (63%) drew the same shade "${shades.low}" — the ink is not carrying the value`);
  }
  //     **B · no bars.** Every drawn cell is the full height of the
  //     row; a height proportional to the value is the old chart coming
  //     back under a new name.
  if (shades.heights.length !== 1) {
    fail(`the cells draw ${shades.heights.length} different heights (${shades.heights.join(",")}) — a heat band has one`);
  }
  if (shades.views.join(",") !== "heat") fail(`the band does not say which reading it is (${shades.views.join(",")})`);
  //     ── the ruler is the day, not the pixels it got ──────────────
  //
  //     **This replaced 「288 cells to a row」**, which the drawing
  //     stopped satisfying the moment it started merging neighbours of
  //     the same step into one rect. Counting elements was only ever a
  //     proxy; what it was really guarding is that **every bucket has
  //     its own place on the ruler** — and that is checkable directly,
  //     on segments or on cells, whichever the drawing makes:
  //
  //       - the segments **tile**: each one's right edge is the next
  //         one's left edge, with nothing between and nothing over;
  //       - every edge is a **whole pixel** (off the integer grid the
  //         antialiasing eats the ink unevenly and a row of one shade
  //         comes out combed);
  //       - the ranges **cover the day**: `Σ(to-from)` plus the holes is
  //         288, so no bucket was dropped;
  //       - and the **narrowest one-bucket run is `round(slot)`** —
  //         which is what a drawing that thinned the day to fit its
  //         pixels could not produce.
  //
  //     Together those block both ways of coarsening: rounding to the
  //     width available, and drawing every second bucket.
  {
    const segs = shades.segs;
    const slot = (shades.x1 - shades.x0) / 288;
    let broke = null;
    for (let i = 1; i < segs.length && !broke; i += 1) {
      if (segs[i - 1].to === segs[i].from && Math.abs(segs[i - 1].x + segs[i - 1].w - segs[i].x) > 0.01) {
        broke = `segment ${i - 1} ends at ${(segs[i - 1].x + segs[i - 1].w).toFixed(2)} and ${i} starts at ${segs[i].x.toFixed(2)}`;
      }
    }
    if (broke) fail(`the buckets do not tile: ${broke} — a seam or an overlap, and the ink stops meaning the value`);
    const ragged = segs.filter((sg) => !Number.isInteger(sg.x) || !Number.isInteger(sg.x + sg.w));
    if (ragged.length) {
      fail(`${ragged.length} of ${segs.length} cell edges are off the pixel grid (first at x=${ragged[0].x}) — antialiasing then combs the row`);
    }
    if (shades.covered !== 288 - dayGaps) {
      fail(`the cpu row's cells cover ${shades.covered} buckets where the fixture measured ${288 - dayGaps} — a day thinned to fit its pixels, or a gap bridged`);
    }
    const ones = segs.filter((sg) => sg.to - sg.from === 1);
    if (ones.length === 0) {
      fail("probe dead: no single-bucket run in the cpu row, so 「one bucket is one slot wide」 measured nothing");
    } else {
      const minW = Math.min(...ones.map((sg) => sg.w));
      if (minW !== Math.round(slot)) {
        fail(`a one-bucket cell is ${minW}px where the day's own slot is ${slot.toFixed(4)} (round ${Math.round(slot)}) — the ruler is the pixels, not the day`);
      }
    }
  }
  //     **The hours are three apart, and that is checked as a spacing
  //     rather than as a count.**
  //
  //     ⚠️ It used to demand **23**, one an hour between the ends. That
  //     number is gone on purpose and is not a deletion to restore: the
  //     drawing marks every third hour (`TICKS_W`, eight of them; four
  //     at narrow, `TICKS_N`). The reasoning is the day chart's own,
  //     one step further — a hair per bucket was dropped because 「288
  //     five-minute buckets at three pixels cannot be counted, so that
  //     row of hairs was decoration claiming to be a scale」, and
  //     twenty-three hour marks across a card are the mild case of the
  //     same complaint. A spacing holds for every clock; a count only
  //     holds for the one the window happened to start on.
  {
    const gaps = shades.hourX.slice(1).map((x, i) => x - shades.hourX[i]);
    const want = ((shades.x1 - shades.x0) * 36) / 288;
    if (shades.hourX.length < 6) {
      fail(`${shades.hourX.length} hour marks on a 288-bucket day — every third hour is seven or eight of them`);
    } else if (gaps.some((g) => Math.abs(g - want) > 1.5)) {
      fail(`the hour marks are ${gaps.map((g) => g.toFixed(1)).join("/")} apart where three hours is ${want.toFixed(1)}`);
    }
  }
  }
  //     ── nothing in the drawing sticks out of its own box ────────
  //
  //     **The viewBox is the drawing's edge, and anything past it is
  //     simply gone** — no overflow, no error, no sign on the page that
  //     a number was ever drawn. Measured on the real face at 390: a
  //     guard meant to keep a bucket at least a pixel wide put the
  //     right gutter *outside* a 332-wide box, and all five rows' peak
  //     numbers were clipped away silently. The sideways-overflow scan
  //     cannot see this: the SVG element itself fits, it is its own
  //     contents that do not.
  //
  //     Checked at both stops, because the geometry is different at
  //     each and it was the narrow one that broke.
  {
    const wide = page.viewportSize();
    const fits = async (where) => {
      const box = await page.evaluate(() => {
        const svg = document.querySelector("[data-machine-card] [data-day-band]");
        if (!svg) return null;
        const vb = svg.viewBox.baseVal;
        //   **Only what is painted.** The cursor's dots are parked at
        //   the origin at `opacity:0` until somebody moves over the
        //   figure, and `svg.getBBox()` counts them — a probe that
        //   reported the drawing as sticking out by 2.6px on a chart
        //   nothing was wrong with.
        let x = Infinity, y = Infinity, right = -Infinity, bottom = -Infinity;
        for (const n of svg.querySelectorAll("*")) {
          if (!n.getBBox || n.getAttribute("opacity") === "0") continue;
          const b = n.getBBox();
          if (b.width === 0 && b.height === 0) continue;
          x = Math.min(x, b.x); y = Math.min(y, b.y);
          right = Math.max(right, b.x + b.width); bottom = Math.max(bottom, b.y + b.height);
        }
        if (!Number.isFinite(x)) return null;
        return { vbW: vb.width, vbH: vb.height, x, y, right, bottom };
      });
      if (!box) return fail(`probe dead: no day band at ${where}, so nothing was checked for clipping`);
      if (box.right > box.vbW + 0.5 || box.x < -0.5 || box.bottom > box.vbH + 0.5 || box.y < -0.5) {
        fail(
          `at ${where} the 24-hour drawing runs to ${box.right.toFixed(1)}×${box.bottom.toFixed(1)} in a ` +
            `${box.vbW}×${box.vbH} box (from ${box.x.toFixed(1)},${box.y.toFixed(1)}) — whatever is past the edge is clipped away with no sign of it`,
        );
      }
    };
    //   **And no bucket overlaps the next one.** The fill is
    //   semi-transparent, so an overlap paints twice and reads as a
    //   darker stripe: at 390 the plot is 244 pixels for 288 buckets,
    //   and a one-pixel floor on each bucket's width combed the whole
    //   row (seen on the real face, invisible to every count and every
    //   bounding box). Exact tiling is the only arrangement in which
    //   the ink means the value.
    const tiles = async (where) => {
      const bad = await page.evaluate(() => {
        const row = document.querySelector('[data-machine-card] [data-day-row="cpu"]');
        if (!row) return null;
        const r = [...row.querySelectorAll("[data-day-shade]")].map((n) => ({
          x: Number(n.getAttribute("x")),
          w: Number(n.getAttribute("width")),
        }));
        if (r.length < 2) return null;
        let worst = 0;
        for (let i = 1; i < r.length; i += 1) worst = Math.max(worst, r[i - 1].x + r[i - 1].w - r[i].x);
        return { worst, n: r.length };
      });
      if (!bad) return fail(`probe dead: no cpu buckets to check for overlap at ${where}`);
      if (bad.worst > 0.01) {
        fail(
          `at ${where} a bucket overlaps the next by ${bad.worst.toFixed(2)}px — a semi-transparent fill painted twice is a ` +
            `darker stripe, and the ink is supposed to be the value`,
        );
      }
    };
    await fits(`${wide.width}`);
    await tiles(`${wide.width}`);
    await page.setViewportSize({ width: 390, height: wide.height });
    await new Promise((r) => setTimeout(r, 400));
    await fits("390");
    await tiles("390");
    await page.setViewportSize(wide);
    await new Promise((r) => setTimeout(r, 400));
  }

  //     **F · strain is judged per bucket, and only where being full is
  //     a fault.** 内存 wears it; CPU does not, because a busy machine
  //     is a machine working (see `shadeOf`). The fixture puts one
  //     bucket at exactly 90.0 in the vram series to pin `>=`.
  const strain = await page.evaluate(() => {
    //   Buckets of the same step share a rect, so a row's 288 answers
    //   are read back out of the runs rather than off 288 elements.
    const of = (name) => {
      const row = document.querySelector(`[data-machine-card] [data-day-row="${name}"]`);
      if (!row) return null;
      const out = new Array(288).fill(null);
      for (const c of row.querySelectorAll("[data-day-cell]")) {
        for (let i = Number(c.dataset.dayFrom); i < Number(c.dataset.dayTo); i += 1) out[i] = c.dataset.dayShade;
      }
      return out;
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
  //     And the boundary itself: the fixture's memory bucket 11 is
  //     exactly 90.0, so a `>` written for `>=` leaves it a step below
  //     the top and drops out of the list above. Pinned on a row that
  //     *wears* strain because that is where the top step has a name of
  //     its own — no literal band number to go stale when the ramp
  //     gains a cut.
  if ((strain.mem ?? [])[11] !== "strain") {
    fail(`the memory bucket at exactly 90.0 drew "${(strain.mem ?? [])[11]}" — the top step is >= 90, not > 90`);
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
    //   **A hole is a hole**: the ink covers exactly the measured
    //   buckets, and the fixture leaves `dayGaps` of them unmeasured. A
    //   drawing that bridged a gap would cover 288 here; one that
    //   thinned the day would cover fewer than the readings it has.
    if (r.covered !== 288 - dayGaps) {
      fail(`the ${r.name} row's ink covers ${r.covered} buckets where the fixture measured ${288 - dayGaps} — a gap was bridged, or the day was thinned`);
    }
    if (r.cells === 0) fail(`the ${r.name} row drew no cells at all`);
    //   And each row says how high it got: a shape alone answers 什么
    //   时候 and never 多高.
    if (!/^\d+$/.test(r.peak)) fail(`${r.name}: the row draws a shape but never says its peak ("${r.peak}")`);
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
