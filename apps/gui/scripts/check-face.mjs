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
    // The row's own directory, raw (`khor_node::live::place_of` stopped
    // folding it in 工作区批③). `null` is a session nobody placed.
    place: null,
    mux: false,
  });
  // **Four rows for the workspace column** (工作区批③), which belongs to
  // **peer**'s `/home/peer/work`. Two belong; the other two are each
  // wrong on exactly one axis, which is what makes either rule
  // falsifiable on its own:
  //
  //   - `/home/peer/workshop` on peer — a sibling, not a child. A
  //     `startsWith` with no separator files it under the project.
  //   - `/home/peer/work` on **elder** — the same path on another
  //     machine. A workspace is the pair (machine, directory), and
  //     these rows deliberately do not repeat their machine
  //     (`showVia`), so nothing on screen would say where this one is
  //     really running.
  // **`via` is set on these**, which is what lets the column's own
  // claim be falsifiable: a row with nothing to say about a machine
  // draws no `[data-via]` whether or not the column asked for one.
  const wsRow = (n, word, place, kind, last, home) => ({
    ...row(n, word),
    id: `${kind}/ws-${n}`,
    kind,
    title: `ws-session-${n}`,
    place,
    last,
    // **The channel name**, which is what `SessionRow.home` carries and
    // what the column compares against — never the name on the glass
    // (`views/WorkspacePane`'s `channel`).
    home,
    via: "peer",
    via_face: null,
  });
  const rows = [
    row(0, "idle"),
    row(1, "blocked"),
    wsRow(2, "idle", "/home/peer/work", "tui", "最近说的一句", "peer"),
    // A shell session is one of the five-question rows like any other,
    // drawn by the same component and not by a branch of its own.
    wsRow(3, "busy", "/home/peer/work/src", "shell", "在跑", "peer"),
    wsRow(4, "idle", "/home/peer/workshop", "tui", "隔壁", "peer"),
    wsRow(5, "idle", "/home/peer/work", "tui", "别的机器上的同一个路径", "elder"),
  ];
  const fill = (used, total) => ({ used, total });
  const devs = [
    {
      id: "e".repeat(64),
      name: "gate",
      // ⚠️ **`shown` was missing here until 2026-09-12** and five
      // assertions in the 額度 section had been red for it. It is the
      // name a person reads; `name` is the channel the network files
      // this machine under, and every screen that shows a machine to
      // somebody reads `shown` (`DeviceRow`'s own head says the two
      // must not be confused). With it absent the 額度 landing could
      // not say which machine its own subscription belongs to, so
      // grouping by 来源 collapsed to one nameless group — and the
      // failure read as「the default grouping is wrong」, which is a
      // sentence about the product for a fault in this file.
      shown: "gate",
      me: true,
      face: null,
      pinned: false,
      vitals: null,
      hop: null,
      sponsor: null,
      watch: [],
      watched: null,
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
      shown: "peer",
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
      // **Three watched directories, one per shape** (2026-09-06 关注目录):
      // on a listed volume, missing, and on a filesystem khor does not
      // list. All three, because the reader paints three sentences and
      // a fixture with only the first would let 「missing」 and
      // 「unlisted」 quietly become each other.
      watch: ["/data/models", "/nope", "/proc/1"],
      watched: {
        rows: [
          { path: "/data/models", missing: false, volume: { mount: "/data", fill: fill(46_000_000_000, 50_000_000_000) } },
          { path: "/nope", missing: true, volume: null },
          { path: "/proc/1", missing: false, volume: null },
        ],
        age_ms: 42_000,
      },
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
      shown: "elder",
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
      watch: [],
      watched: null,
      // ring(2)'s other seat: opposite the peer, up top.
      seat: { left: 50, top: 16 },
    },
  ];
  await page.route("**/sessions", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(rows) }),
  );
  //   The watch list as last saved through `set_watch`, per machine —
  //   on a real node the table is written before the call answers and
  //   the next poll carries it, and a fixture that kept answering the
  //   old list would reset the editor's draft under the section that
  //   measures it (2026-09-06).
  const watchKept = {};
  await page.route("**/devices", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify(devs.map((d) => (watchKept[d.name] ? { ...d, watch: watchKept[d.name] } : d))),
    }),
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
      { pid: 101, name: "khor", user: "zgh", cpu_pct: 240.5, mem: 512_000_000, gpu_pct: 44, gpu_mem: 8_000_000_000, run_secs: 90_061, arg: "serve", status: "run", cpu_ms: 7200000, io_bytes: 4294967296, ppid: 1 },
      // prettier-ignore
      { pid: 202, name: "node", user: "root", cpu_pct: 12, mem: 128_000_000, gpu_pct: 5, gpu_mem: 2_000_000_000, run_secs: 45, arg: "scripts/check-face.mjs --with-a-very-long-tail-so-this-cell-cannot-possibly-fit-in-its-column-and-must-say-so", status: "sleep", cpu_ms: 61000, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 303, name: "kernel_task", user: "root", cpu_pct: 3.2, mem: 2_000_000_000, gpu_pct: null, gpu_mem: null, run_secs: null, arg: null, status: "diskwait", cpu_ms: 512000, io_bytes: 1048576, ppid: 1 },
      // **Twenty rows lifted from a real machine's distribution**
      // (turing, 2026-09-04, `dist-turing.json`): the median process
      // uses a fifth of one percent of a core while the busiest uses
      // forty-three. Three hand-made rows cannot say whether an axis
      // spreads a table out, and the crowding check simply declines to
      // run below eight points — a fixture that makes an assertion
      // unevaluable is a fixture that makes it useless.
      // prettier-ignore
      { pid: 900000, name: "python3", user: "gate", cpu_pct: 42.9910, mem: 8708096, gpu_pct: null, gpu_mem: null, run_secs: 11, arg: "tail/0.py", status: "zombie", cpu_ms: 3600000, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900001, name: "khor-2077930-6", user: "zgh", cpu_pct: 41.9717, mem: 155160576, gpu_pct: null, gpu_mem: null, run_secs: 148, arg: "tail/1.py", status: "stopped", cpu_ms: 900, io_bytes: 33554432, ppid: 1 },
      // prettier-ignore
      { pid: 900002, name: "netdata", user: "_windowserver", cpu_pct: 32.0176, mem: 4024270848, gpu_pct: null, gpu_mem: null, run_secs: 285, arg: "tail/2.py", status: "sleep", cpu_ms: 45000, io_bytes: 8388608, ppid: 1 },
      // prettier-ignore
      { pid: 900003, name: "apps.plugin", user: "gate", cpu_pct: 13.5588, mem: 109076480, gpu_pct: null, gpu_mem: null, run_secs: 422, arg: "tail/3.py", status: "idle", cpu_ms: 128000, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900004, name: "fetch", user: "root", cpu_pct: 7.9261, mem: 46661632, gpu_pct: null, gpu_mem: null, run_secs: 559, arg: "tail/4.py", status: "unknown-13", cpu_ms: 9600000, io_bytes: 2147483648, ppid: 1 },
      // prettier-ignore
      { pid: 900005, name: "tokio-rt-worke", user: "zgh", cpu_pct: 5.4203, mem: 155160576, gpu_pct: null, gpu_mem: null, run_secs: 696, arg: "tail/5.py", status: "sleep", cpu_ms: 33000, io_bytes: 524288, ppid: 1 },
      // prettier-ignore
      { pid: 900006, name: "tokio-rt-worke", user: "gate", cpu_pct: 3.8330, mem: 155160576, gpu_pct: null, gpu_mem: null, run_secs: 833, arg: "tail/6.py", status: "run", cpu_ms: 7100, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900007, name: "spyder", user: "root", cpu_pct: 2.2191, mem: 472055808, gpu_pct: null, gpu_mem: null, run_secs: 970, arg: "tail/7.py", status: "sleep", cpu_ms: 480000, io_bytes: 65536, ppid: 1 },
      // prettier-ignore
      { pid: 900008, name: "top", user: "zgh", cpu_pct: 1.6829, mem: 8065024, gpu_pct: null, gpu_mem: null, run_secs: 1107, arg: "tail/8.py", status: "tracing", cpu_ms: 2400000, io_bytes: 134217728, ppid: 1 },
      // prettier-ignore
      { pid: 900009, name: "tokio-runtime-", user: "gate", cpu_pct: 0.5521, mem: 46661632, gpu_pct: null, gpu_mem: null, run_secs: 1244, arg: "tail/9.py", status: "sleep", cpu_ms: 61500, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900010, name: "prometheus", user: "root", cpu_pct: 0.3185, mem: 166809600, gpu_pct: null, gpu_mem: null, run_secs: 1381, arg: "tail/10.py", status: "parked", cpu_ms: 800, io_bytes: 16777216, ppid: 1 },
      // prettier-ignore
      { pid: 900011, name: "containerd", user: "zgh", cpu_pct: 0.2230, mem: 24326144, gpu_pct: null, gpu_mem: null, run_secs: 1518, arg: "tail/11.py", status: "sleep", cpu_ms: 155000, io_bytes: 268435456, ppid: 1 },
      // prettier-ignore
      { pid: 900012, name: "UV_WORKER[201]", user: "gate", cpu_pct: 0.1911, mem: 4024270848, gpu_pct: null, gpu_mem: null, run_secs: 1655, arg: "tail/12.py", status: "run", cpu_ms: 12000, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900013, name: "VLLM::EngineCo", user: "root", cpu_pct: 0.1274, mem: 1630076928, gpu_pct: null, gpu_mem: null, run_secs: 1792, arg: "tail/13.py", status: "sleep", cpu_ms: 3300, io_bytes: 4096, ppid: 1 },
      // prettier-ignore
      { pid: 900014, name: "python", user: "zgh", cpu_pct: 0.1062, mem: 3850240, gpu_pct: null, gpu_mem: null, run_secs: 1929, arg: "tail/14.py", status: "dead", cpu_ms: 640000, io_bytes: 1073741824, ppid: 1 },
      // prettier-ignore
      { pid: 900015, name: "go.d.plugin", user: "gate", cpu_pct: 0.0903, mem: 73875456, gpu_pct: null, gpu_mem: null, run_secs: 2066, arg: "tail/15.py", status: "sleep", cpu_ms: 91000, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900016, name: "kworker/5:2-ev", user: "root", cpu_pct: 0.0796, mem: 0, gpu_pct: null, gpu_mem: null, run_secs: 2203, arg: "tail/16.py", status: "sleep", cpu_ms: 1900, io_bytes: 786432, ppid: 1 },
      // prettier-ignore
      { pid: 900017, name: "kworker/4:1-ev", user: "zgh", cpu_pct: 0.0743, mem: 0, gpu_pct: null, gpu_mem: null, run_secs: 2340, arg: "tail/17.py", status: "run", cpu_ms: 27000, io_bytes: 209715200, ppid: 1 },
      // prettier-ignore
      { pid: 900018, name: "kworker/u257:0", user: "gate", cpu_pct: 0.0690, mem: 0, gpu_pct: null, gpu_mem: null, run_secs: 2477, arg: "tail/18.py", status: "sleep", cpu_ms: 5400, io_bytes: null, ppid: 1 },
      // prettier-ignore
      { pid: 900019, name: "go.d.plugin", user: "root", cpu_pct: 0.0637, mem: 73875456, gpu_pct: null, gpu_mem: null, run_secs: 2614, arg: "tail/19.py", status: "unclear", cpu_ms: 71000, io_bytes: 51200, ppid: 1 },
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
    // **A page whose CPU readings crowd the bottom of the axis** — what
    // this Mac's own table looks like (most rows at 0, the rest spread
    // over three decades below one percent). Served for the 读写 order,
    // which nothing else here asks for, so the scatter's tick labels
    // can be measured on readings that put several ticks inside one
    // hundredth of a percent (2026-09-06: `0.00% 0.01% 0.01% 0.02%`).
    if (r.request().postDataJSON().by === "io") {
      const fresh = dayNow();
      const crowd = [0, 0, 0.000001, 0.000002, 0.0031, 0.0062, 0.0093, 0.0124, 0.0186, 0.031, 0.062, 0.12, 0.31, 1.2, 3.1, 12.4, 45, 240.5];
      const low = { ...fresh, top: fresh.top.map((t, i) => ({ ...t, cpu_pct: crowd[i % crowd.length] })) };
      return r.fulfill({ contentType: "application/json", body: JSON.stringify(low) });
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
  // The watch editor's save (`set_watch`): answered the way the node
  // would — the list as kept for a tidy one, and the node's own
  // sentence (a relative path) as a refusal. What was sent is kept so
  // the section can read it back: a save that answered without being
  // asked would be a control with one arm.
  const watchSaves = [];
  await page.route("**/set_watch", (r) => {
    const body = r.request().postDataJSON();
    watchSaves.push(body);
    const bad = (body.paths ?? []).find((p) => !String(p).startsWith("/"));
    if (bad) {
      return r.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify(`关注的目录得是绝对路径: ${bad}`) });
    }
    const kept = body.paths.map((p) => String(p).trim());
    watchKept[body.machine] = kept;
    return r.fulfill({ contentType: "application/json", body: JSON.stringify(kept) });
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
  const ring = (account, name, provider, { unit = null, kind = "bill", ...patch } = {}) => ({
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
      // The tail fields, spelled out so a fixture that leaves one off
      // is a `undefined` the page can be caught mishandling — rather
      // than a key React never sees.
      remaining: null,
      expires_at: null,
      models: [],
      days: [],
      spent_this_month: null,
      spent_all_time: null,
      model_days: [],
      log_capped: null,
      // ⚠️ **Added 2026-09-08, and its absence was a real hole.**
      // `AccountReading::model_tokens` has been on the wire since
      // `5aa036b`; this fixture never grew it, so every routed account
      // reached the page as `undefined` — and `undefined` is not `[]`,
      // it walks straight through a `.length` the way the block above
      // says `undefined` walks through `!== null`. Nothing was reading
      // it until the 额度 card's lower half did, and then it would have
      // thrown on every account here.
      model_tokens: [],
      // The three a **subscription's** reading carries (2026-09-07).
      // Spelled out for the reason the block above is: an omitted key
      // reaches the page as `undefined`, and `undefined` is not `null`
      // — it walks straight through a `!== null` guard. Measured that
      // day: a missing `credits` throws inside `creditsPct` (whose
      // first line is `c === null`), and a missing `fill` draws
      // `A57.6 57.6 0 0 0 NaN NaN`. **Neither state can come off the
      // wire**: every `Option` here is a tail field with `serde
      // default` and no `skip_serializing_if`, so it crosses as
      // `null`. They are states only this file can invent.
      windows: [],
      plan: null,
      credits: null,
      ...patch,
    },
    age_ms: 0,
    home_name: "gate",
    mine: true,
    strain: null,
    base_url: null,
    budget: null,
    pinned: false,
    // **The unit the person typed, and it sits beside the reading, not
    // inside it.** The node relabels `reading.currency` before the
    // board leaves it, so the override is spelled once here and its
    // *effect* is spelled as the row's `currency` — a fixture that set
    // only one of the two would be describing a wire state that cannot
    // happen. (Lifted out of `patch` for that reason: `patch` fills the
    // reading.)
    unit,
    // ⚠️ **A field the wire has always carried and this fixture never
    // did** (`AccountRing.kind`, non-optional). It cost a whole gate:
    // 「类型」 is not a hard-coded pair of ticks, it is **derived from
    // the rows on screen** (`QuotaLanding`'s `seen(axis)`), so every
    // row answering `undefined` meant the `kind:bill` tick was never
    // drawn and its probe waited 30 seconds for a node that could not
    // exist.
    //
    // **Neither「the gate went stale」nor「the product broke」**, which
    // were the two candidates — it is the third thing this repo has a
    // name for: a hand-written mock drifting from the Rust it stands
    // in for. The product reads the real field; only this file was
    // still living in the world before it. Judged by reading
    // `seen(axis)` and `AccountRing.ts` rather than by pointing the
    // probe somewhere it would go green.
    kind,
  });
  const board = [
    ring("acc-fill", "gateway", "newapi", {
      fill: 0.45,
      spent: 45,
      limit: 100,
      currency: "USD",
      resets_at: Math.floor(Date.now() / 1000) + 86_400,
    }),
    // **A vendor with a day chart and no ceiling on its log** — the
    // control arm for the account below, which has both.
    // **A figure past four characters, so the k step is walked by
    // something.** 2026-09-07:「超过了 4 位,能不能用别的方式,比如
    // 1.1k」 — and the real readings top out at 123.75, so **without
    // this row that branch has never run**. A rule nobody has walked is
    // not 「probably fine」, it is unknown (this repo's own judgment,
    // written the same day).
    // **…and it walks the k step with no unit set**, so the default
    // mark rides the shortened form too (`1235$` is five characters,
    // `1k$` is three).
    ring("acc-thousands", "big spender", "newapi", {
      spent: 1234.5,
      currency: "USD",
      unlimited: true,
    }),
    // **A mark that is not on the shortcut row and is not one
    // character**, because 2026-09-07 made the field free text
    // (「直接输入符号」) and free text is what people will put in it.
    // Two rules run through this one row and neither has another
    // walker:
    //
    // - the ring cannot afford `124USDT`, and **the digits are what it
    //   keeps** — before this row existed the code would have stepped
    //   the magnitude down instead and drawn a reading of 124 as `0k`;
    // - the column gives a mark that **starts with a letter** a word
    //   gap (`124 USDT`), where a symbol gets none (`12.34¥` below).
    //   One rule, two outcomes, and a fixture with only symbols in it
    //   would leave half of it unwalked.
    ring("acc-usdt", "wallet", "newapi", {
      spent: 123.75,
      currency: "USDT",
      unit: "USDT",
    }),
    // **The widest string a ring's middle can hold, and until
    // 2026-09-08 nothing drew it.**
    //
    // The middle is capped at four characters (`RING_CHARS`) and
    // checked twice: the count, and — because four *wide* characters
    // can still overflow — the string's width against the rim's inner
    // diameter. **That second check had never seen its worst input.**
    // Measured the night this row was added: the widest middle on the
    // glass was `45%` at **27** of a 32 inner, while `124$` measures
    // **30.3** — three digits *and* a mark, which is the most four
    // characters can weigh. The margin the check actually has is
    // **1.7**, and it was reading one of **5.0**.
    //
    // It came to light by accident: `124$` had to be conjured for one
    // throwaway run (by flipping the row above to `$`) to measure a
    // figure for `Face`'s notes — **and then reverted, which put the
    // worst case back out of reach.** A sample that exists only inside
    // somebody's temporary edit is a sample the gate does not have.
    //
    // **What it buys, measured rather than argued.** Teardown: the
    // middle's type from `19 * k` to `22 * k`, +15.8%, a change any
    // hand tuning the dial could make. The spill check named exactly
    // one offender — `[["124$", 2.5]]` — while `45%` grew to **30.8**
    // and stayed inside its 32. **Without this row that fault ships
    // green**, and it ships as a number drawn over its own ring.
    //
    // ⚠️ **A new row rather than a change to the row above.** That one
    // is a rung of the unit ladder below, whose whole claim is that
    // its three rows differ in `unit` and nothing else; editing it to
    // borrow a `$` quietly costs the ladder a rung — and the letter/
    // symbol spacing rule would lose its only lettered sample with it.
    ring("acc-widest", "big figure", "newapi", {
      spent: 123.75,
      currency: "$",
      unit: "$",
    }),
    // **The unit ladder's three rungs are these next three rows, and
    // every field but `unit` is identical across them** — same figure,
    // same provider, same days. Anything else that differed would be a
    // second explanation for a difference on screen.
    //
    //   ① nobody set one  → the default mark is drawn (`12$`)
    //   ② somebody set `$` → **the same thing**, and that is the claim
    //   ③ somebody set `¥` → the person's mark
    //
    // ⚠️ **① and ② cannot tell a live default from a hard-coded one**:
    // an implementation that ignored `unit` entirely and always drew
    // `$` passes both. **③ is the only rung that fails it**, which is
    // why it is here and why the assertion order below reads it last.
    ring("acc-number", "vendor", "claude", {
      spent: 12.34,
      currency: "USD",
      days: [
        { at: 1_800_000_000, spent: 4.0 },
        { at: 1_800_086_400, spent: 8.34 },
      ],
      // **The provider road, with the cache itemised** — the arm that
      // says the four cache quantities appear when the source states
      // them. `acc-capped` below is the same road with
      // `cache_split: false`, and `acc-widest` reports no tokens at
      // all: three arms, because 「the cache columns are drawn」 and
      // 「they are not」 and 「there is no chart」 are three different
      // screens and a fixture with one of them tests nothing.
      model_tokens: [
        { at: 1_800_000_000, model: "gpt-5", tokens: { input: 400, cached_input: 900, cache_write: 100, output: 1_600 }, cache_split: true },
        { at: 1_800_086_400, model: "gpt-5", tokens: { input: 200, cached_input: 500, cache_write: 50, output: 800 }, cache_split: true },
        { at: 1_800_086_400, model: null, tokens: { input: 10, cached_input: 0, cache_write: 0, output: 40 }, cache_split: true },
      ],
    }),
    ring("acc-dollar", "vendor2", "claude", {
      spent: 12.34,
      currency: "$",
      unit: "$",
      days: [
        { at: 1_800_000_000, spent: 4.0 },
        { at: 1_800_086_400, spent: 8.34 },
      ],
    }),
    ring("acc-yuan", "vendor3", "claude", {
      spent: 12.34,
      currency: "¥",
      unit: "¥",
      days: [
        { at: 1_800_000_000, spent: 4.0 },
        { at: 1_800_086_400, spent: 8.34 },
      ],
    }),
    // **A gateway whose log the provider cut.** Three days here, and
    // the fixture's own count is what the title has to say — 「7 天」
    // hard-coded would pass on any chart at all.
    ring("acc-capped", "gateway2", "newapi", {
      //   ⚠️ **These three instants are real local midnights, and that
      //   is the whole point of them** (2026-09-09). Every other `at`
      //   in this fixture is a round number like `1_800_000_000` —
      //   08:00Z, **not a day boundary in any zone** — so reading it as
      //   UTC and reading it in the node's zone give the same date, and
      //   a gate comparing the two readings could not tell them apart.
      //   That is 「两条胳膊被造得必然一致」 in its purest form: the
      //   date bug measured on real accounts was invisible here.
      //
      //   `1_788_537_600` is 2026-09-05 00:00 in Asia/Shanghai, which is
      //   2026-09-04 16:00Z. **A face reading these as UTC prints the
      //   day before, all three.**
      zone: "Asia/Shanghai",
      spent: 9.0,
      currency: "USD",
      days: [
        { at: 1_788_537_600, spent: 1.0 },
        { at: 1_788_624_000, spent: 3.0 },
        { at: 1_788_710_400, spent: 5.0 },
      ],
      log_capped: 1000,
      // **`cache_split: false` is 「those two were not reported」, not
      // 「they were nought」** (`ModelTokenDay`). new-api's log carries
      // `prompt_tokens` and `completion_tokens` and nothing else, so
      // this is what that road really answers — and the four
      // cache-derived columns must leave the table rather than be
      // drawn as zeroes.
      model_tokens: [
        { at: 1_788_537_600, model: "deepseek-chat", tokens: { input: 300, cached_input: 0, cache_write: 0, output: 700 }, cache_split: false },
        { at: 1_788_624_000, model: "deepseek-chat", tokens: { input: 150, cached_input: 0, cache_write: 0, output: 350 }, cache_split: false },
      ],
    }),
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
  // ⚠️ **The `**/usage` route is back, 2026-09-08.** The tombstone that
  // stood here said its absence was not an oversight — nothing had
  // asked for it since 2026-08-29, so a fixture would have been
  // scaffolding for a call nobody makes. **That is no longer true**:
  // the 额度 detail's lower half asks for it when a subscription's card
  // opens (`views/UsageHalf`), and the reason the old caller was
  // deleted (a tally of every transcript, on a two-second heartbeat,
  // for one line) is answered by *when* it is asked rather than by not
  // asking.
  //
  // **What this fixture is built to tell apart**, each one a way the
  // half could be wrong while looking right:
  //
  //   - `claude-opus-6` **is a model khor has never heard of**, and it
  //     is deliberately one of the *smallest* here. It must take the
  //     opus hue at full strength — 「the next free colour」 fails it,
  //     and so does 「the newest is whichever spent most」, which is
  //     what the design this was built from actually did.
  //   - `claude-fable-5` **outspends `claude-fable-5-1`** by an order
  //     of magnitude, so ordering generations by volume and ordering
  //     them by name give different answers. Without that gap the two
  //     rules cannot be told apart.
  //   - **08-29 is exactly twice 08-28** (14,000 against 7,000), which
  //     is what makes 「the stack's axis is linear」 measurable as a
  //     ratio of drawn heights rather than read off an attribute the
  //     product writes about itself.
  //   - **09-02 is missing.** A day nobody spent on has no row
  //     (`UsageDay`), so an axis built from the rows would skip it and
  //     「7 天」 would quietly mean 「the last seven days that had
  //     spend」.
  //   - a `null` model and a one-day `haiku` — **the row a 「top five」
  //     fold would swallow**, and a folded band is the same screen as a
  //     model that never ran.
  //   - ⚠️ **`claude-opus-4-8` runs on 08-30 and on no other day**, so
  //     it is inside the whole span and outside the seven-day one. That
  //     is what makes 「switching the window does not recolour」
  //     falsifiable: the opus family has three generations over the
  //     whole span and two over the week, so a colour worked out from
  //     the window's models — the bug the design this was built from
  //     actually had — moves `claude-opus-5` from one fade to another.
  //     **The first draft of this fixture put it on 09-06 as well, and
  //     that made every model present in both windows**: the teardown
  //     was run and the gate stayed green, because the two arms of the
  //     comparison could not come out different whatever the code did.
  // **How much of each model's input came out of a cache**, one
  // multiple per model so a model's rate is the same on every one of
  // its days and the aggregate means something.
  //
  // ⚠️ **These vary on purpose, and the first draft's did not.** With
  // one multiple for everybody the cache rate came out 0.833 for every
  // model — an axis with a single reading, which the ruler answers
  // `flat` for. **Every axis on the page then chose the same ruler**,
  // and a fixture like that cannot tell 「the ruler is chosen by
  // measurement」 from 「it is always this one」 — the machine page's
  // gate has the same control arm for the same reason. Spread this way
  // the rate axis lands on a linear ruler (readings 0.60–0.86) while
  // the token counts land on a log one, so both are reachable in one
  // run.
  const cacheK = {
    "claude-opus-5": 12,
    "claude-fable-5": 8,
    "claude-fable-5-1": 10,
    "claude-opus-4-8": 5,
    "claude-opus-6": 6,
    "claude-haiku-4-5-20251001": 4,
    "": 3,
  };
  const usageCell = (day, model, input, output) => ({
    day,
    category: "claude",
    model,
    tokens: { input, cached_input: input * cacheK[model ?? ""], cache_write: input, output },
  });
  const usageModels = [
    usageCell("2026-08-28", "claude-opus-5", 1_000, 4_000),
    usageCell("2026-08-28", "claude-fable-5", 500, 1_500),
    usageCell("2026-08-29", "claude-opus-5", 2_000, 8_000),
    usageCell("2026-08-29", "claude-fable-5", 1_000, 3_000),
    usageCell("2026-08-30", "claude-opus-5", 1_000, 3_000),
    usageCell("2026-08-30", "claude-opus-4-8", 200, 800),
    usageCell("2026-08-31", "claude-opus-5", 1_500, 2_500),
    usageCell("2026-09-01", "claude-fable-5", 800, 3_200),
    usageCell("2026-09-01", null, 100, 400),
    usageCell("2026-09-03", "claude-opus-5", 600, 1_400),
    usageCell("2026-09-03", "claude-haiku-4-5-20251001", 10, 40),
    usageCell("2026-09-04", "claude-opus-5", 900, 2_100),
    usageCell("2026-09-04", "claude-fable-5-1", 50, 150),
    usageCell("2026-09-05", "claude-opus-5", 700, 1_300),
    // **Not equal to `claude-fable-5-1`'s 200.** They tied in the first
    // draft, which put two dots on one spot — and two points that
    // cannot be told apart cannot be a control for anything.
    usageCell("2026-09-05", "claude-opus-6", 30, 120),
    usageCell("2026-09-06", "claude-opus-5", 400, 600),
    usageCell("2026-09-06", "claude-opus-6", 40, 90),
  ];
  // **A second vendor's rows, which this card must not draw.** The
  // subscription's half asks for one category; a half that drew
  // everything would pass every other assertion here.
  const usageOther = [{ ...usageCell("2026-09-06", "gpt-5", 9_999, 9_999), category: "codex" }];
  // `days` is the coarser list and it is **not** what the half reads —
  // it is here because 「models empty beside a non-empty days」 is one
  // of the three faces, and that state cannot be built without it.
  const usageAnswer = {
    days: [...new Set(usageModels.map((m) => m.day))].map((day) => ({
      day,
      category: "claude",
      tokens: { input: 0, cached_input: 0, cache_write: 0, output: 0 },
    })),
    unreadable: 0,
    models: [...usageModels, ...usageOther],
    today: "2026-09-06",
  };
  // **Held, so that 「it has not answered yet」 is a state a probe can
  // stand in** rather than a race it has to win. The first pass over a
  // real machine's transcripts is 18.5 s, so this is the ordinary
  // first sight of the card, not a corner.
  //
  // **Every held request is remembered, not just the last one.** An
  // earlier section can open the subscription's card in passing, and
  // with a single slot that first request would still be waiting when
  // the release fires — pending forever, on a route this gate later
  // asserts about.
  const usageHold = { on: true, waiting: [], body: usageAnswer };
  usageHold.release = () => {
    for (const res of usageHold.waiting.splice(0)) res();
  };
  await page.route("**/usage", async (r) => {
    if (usageHold.on) await new Promise((res) => usageHold.waiting.push(res));
    await r.fulfill({ contentType: "application/json", body: JSON.stringify(usageHold.body) });
  });
  // **Three windows, two lengths.** The third is a week limited to one
  // model — the shape the endpoint grew on 2026-09-06 and the parser
  // was dropping — and it is here because the gap between「three in the
  // detail」and「two on the ring」is the rule this page draws by: the
  // ring's layers are lengths, and one model's share of a week is not a
  // third length. A fixture with two windows cannot tell the two
  // numbers apart, so it could not have caught the drop.
  //
  // `scope` is the model's name **as the endpoint spells it**; nothing
  // in khor may supply it, which is why the fixture's word is one the
  // catalog has never heard of.
  const quotaWindows = [
    { span: "five_hour", scope: null, used_pct: 4, resets_at: Math.floor(Date.now() / 1000) + 3600, active: false, severity: "normal" },
    { span: "seven_day", scope: null, used_pct: 39, resets_at: Math.floor(Date.now() / 1000) + 86_400, active: true, severity: "normal" },
    { span: "seven_day", scope: "Fable", used_pct: 29, resets_at: Math.floor(Date.now() / 1000) + 86_400, active: false, severity: "normal" },
  ];
  // The plan is a product name the node de-slugged out of the stored
  // credential (`default_claude_max_20x`), not a catalog word — so the
  // fixture states it the way the node hands it over.
  const quotaPlan = "Max 20x";
  // **The credits, and the two `null`s are the point.** The capture the
  // parser was written against reports `balance` and `monthly_limit` as
  // null while reporting a spend — so「有才画」has something to be
  // wrong about here. A fixture with all three filled cannot tell a
  // page that checks from one that draws whatever it is handed.
  const quotaCredits = {
    enabled: false,
    disabled_reason: "out_of_credits",
    used: 9.21,
    balance: null,
    monthly_limit: null,
    currency: "USD",
    ever_enabled: true,
  };
  // **The credits' other arm.** The capture above reports a spend and
  // no ceiling, so it exercises exactly one half of 「Credits 只有在有
  // 分母时才是一个环」 — and a control group with one arm proves
  // nothing: an implementation that never rings credits passes it, and
  // so does one that rings them always-except-here. This second fixture
  // is the same money **with** a monthly ceiling, and everything about
  // it has to change together: a pip appears, a part appears, and the
  // percentage is `used / monthly_limit`.
  const quotaCreditsCapped = {
    ...quotaCredits,
    enabled: true,
    disabled_reason: null,
    monthly_limit: 20,
    // 9.21 of 20 — **not equal to any window's percent in this
    // fixture** (4 / 39 / 29), so a pip at 46 cannot be one of theirs
    // and the mark being counted is the one being asserted.
    balance: null,
  };
  // ⚠️ **The middle arm, and it is the one that makes the other two
  // mean anything.** The routed fixture is `enabled: false` *and* has
  // no ceiling; the capped one is `enabled: true` *and* has one. Two
  // fields moving together is a control group that cannot say which
  // field did it — measured 2026-09-07 by removing the
  // 「monthly_limit != null」 test from the product: **the gate stayed
  // green**, because `enabled: false` was short-circuiting the whole
  // question. This one is `enabled: true` with **no** ceiling, one
  // field apart from `quotaCreditsCapped`, and it must draw no pip.
  const quotaCreditsUncapped = {
    ...quotaCredits,
    enabled: true,
    disabled_reason: null,
    monthly_limit: null,
    balance: null,
  };
  // ⚠️ **`fill` is stated by each caller, never worked out here.**
  // The node decides which window speaks for the whole plan
  // (`khor_node::quota::whole_plan_fill`) and sends the share; this
  // replica's job is to *supply* it, not to reproduce the rule. A
  // fixture that computed it from `windows` would be the replica
  // running the very rule the arms below are checking against, and the
  // two would agree no matter what either of them did.
  //
  // The default is the standing fixture's whole-plan week (39 of
  // `quotaWindows`), so a caller that says nothing gets a consistent
  // reading.
  const routeQuota = async (credits, windows = quotaWindows, fill = 0.39) => {
    await page.unroute("**/quota");
    await page.route("**/quota", (r) =>
      r.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          quota: {
            windows,
            as_of: Math.floor(Date.now() / 1000) - 120,
            plan: quotaPlan,
            credits,
            fill,
          },
          trouble: null,
        }),
      }),
    );
  };
  await routeQuota(quotaCredits);
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
  // The workspace table (工作区批①). **One read, no machine named** —
  // the list is replicated, so this route takes no arguments and the
  // landing has no per-machine states left to draw.
  //
  // Four rows, one for each thing a row can be: `peer`'s leads because
  // it is pinned (and it is the door the walk below opens, so the
  // controls being measured are the same ones as before); `gate`'s is
  // not a repository, so its git corner must come out empty; `elder`
  // answers neither question, and its row must still be here.
  //
  // **The fourth is the one with no machine name at all** — its machine
  // has left the device table, which `khor_node::Node::workspaces`
  // answers with an empty `machine` on purpose so the row stays
  // removable. An empty machine means「this one」to every verb the node
  // has, so a face that asked about it would paint *this* machine's
  // branch and mtime under another machine's path. Section 4f below
  // asserts that nothing was asked.
  const wsRows = [
    { device: "e".repeat(64), machine: "gate", path: "/home/gate/notes", name: "notes", pinned: false },
    { device: pin.device, machine: "peer", path: pin.path, name: "work", pinned: true },
    { device: "d".repeat(64), machine: "elder", path: "/srv/elder/app", name: "app", pinned: false },
    { device: "c".repeat(64), machine: "", path: "/home/ghost/left", name: "left", pinned: false },
  ];
  // **Stateful, because 4g adds a row and then looks for it.** The node
  // answers this out of a replicated table, so a fixture that answered
  // the same four rows after a successful `ws_add` would be asserting
  // that the call returned — not that anything was declared.
  await page.route("**/workspaces", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(wsRows) }),
  );
  const addedWs = [];
  await page.route("**/ws_add", (r) => {
    const body = r.request().postDataJSON() ?? {};
    const machine = body.machine ?? "";
    const path = body.path ?? "";
    addedWs.push({ machine, path });
    const shown = machine === "" ? "gate" : machine;
    wsRows.push({
      device: devs.find((d) => d.name === shown)?.id ?? "",
      machine: shown,
      path,
      // The node derives this from the path (`khor_node::workspace::
      // basename`); the fixture stands in for the node, so it derives
      // it the same way rather than being handed a name.
      name: path.split("/").filter(Boolean).pop() ?? path,
      pinned: false,
    });
    return r.fulfill({ contentType: "application/json", body: "null" });
  });
  // Every machine the two per-row questions were asked about, so 4f can
  // assert about a question that was **never sent** — an assertion no
  // reading of the screen can make, because a nameless row's corners
  // would be blank whether the ask was skipped or merely refused.
  const askedAbout = { git_status: [], touched: [] };
  // When each row last changed. **`elder` refuses**, the way a machine
  // that is asleep does, and that row must keep its place with an empty
  // corner — a row that vanished because its machine did would be the
  // landing hiding work rather than reporting it.
  await page.route("**/touched", (r) => {
    const body = r.request().postDataJSON() ?? {};
    const machine = body.machine ?? "";
    askedAbout.touched.push({ machine, path: body.path ?? "" });
    if (machine === "elder") {
      return r.fulfill({ status: 400, contentType: "application/json", body: JSON.stringify("联系不上 elder") });
    }
    return r.fulfill({
      contentType: "application/json",
      body: JSON.stringify(machine === "gate" ? Date.now() - 3_600_000 : Date.now() - 60_000),
    });
  });
  // **The digest is what makes the fixture editable** (工作区批4): the
  // view offers a save exactly when it was handed one, so a fixture
  // without it would quietly turn the editor off and 4c below would
  // report a dead probe rather than a drifting caret.
  await page.route("**/read_text", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        total: 42,
        at_ms: Date.now(),
        text: "# notes\n\nthe fixture's file\n",
        cut: false,
        binary: false,
        digest: "0".repeat(64),
      }),
    }),
  );
  // One marked file and one marked directory, so the tree wears both
  // kinds of mark (工作区批3): `src` folds up a `·` from what is under
  // it, `notes.md` wears its own letter.
  // Counted, because 工作区批7 turns on the marks being asked again
  // when somebody comes back to the window rather than on a timer.
  let gitAsked = 0;
  await page.route("**/git_status", (r) => {
    gitAsked += 1;
    // **Only `peer` is a repository.** `gate` refuses the way a
    // directory outside every checkout does and `elder` the way a
    // machine that cannot be reached does — two different sentences,
    // one blank corner, which is the whole point of the corner being
    // blank rather than saying「干净」.
    const body = r.request().postDataJSON() ?? {};
    const machine = body.machine ?? "";
    askedAbout.git_status.push({ machine, path: body.dir ?? "" });
    if (machine === "gate" || machine === "elder") {
      return r.fulfill({
        status: 400,
        contentType: "application/json",
        body: JSON.stringify(machine === "gate" ? "这个路径不在 git 仓库里" : "联系不上 elder"),
      });
    }
    return r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        branch: "main",
        rows: [
          { path: "notes.md", mark: "M" },
          { path: "src/one.rs", mark: "U" },
        ],
        more: false,
      }),
    });
  });
  // The three shapes a gutter draws, one of each: a change that has
  // both halves, an insertion with nothing removed, and a deletion with
  // no current line to stand on. A fixture with only the first would
  // leave the other two's arithmetic to be found on somebody's screen.
  await page.route("**/git_diff", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        hunks: [
          { at: 1, lines: 1, was: ["# old notes"] },
          { at: 3, lines: 2, was: [] },
          { at: 4, lines: 0, was: ["a line that went away"] },
        ],
        more: false,
      }),
    }),
  );
  // A machine with an editor, so the workspace head draws the switch
  // (工作区批5). **The empty answer is the other half of that contract**
  // and it is asserted in 4d below by turning this fixture off, so both
  // branches are measured rather than only the interesting one.
  // A document and a picture, answered by the one call the view makes
  // (`khor_gui_core::files::preview`). The png is a real one — the
  // smallest valid image there is — because a made-up base64 would draw
  // a broken-image glyph and section 4e below would pass on it.
  const ONE_PX_PNG =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
  await page.route("**/preview", (r) => {
    const png = (r.request().postData() ?? "").includes(".png");
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify(
        png
          ? {
              kind: "image",
              total: 9000,
              data: ONE_PX_PNG,
              media: "image/png",
              html: "",
              cut: false,
              empty: false,
            }
          : {
              kind: "doc",
              total: 1234,
              data: "",
              media: "",
              html: "<h1>notes</h1><p>the fixture file</p><table><tr><th>a</th></tr></table>",
              cut: false,
              empty: false,
            },
      ),
    });
  });
  await page.route("**/code_here", (r) =>
    r.fulfill({
      contentType: "application/json",
      // **`ready: false` is the fixture's whole point here**: it is the
      // machine that has not fetched the server, which is the only one
      // the consent card is for (`khor_gui_core::code::Editor`).
      body: JSON.stringify({ version: "1.137.0", ready: false }),
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
  // 删掉 (工作区批④). **Stateful on purpose**: `removeRefuses` makes the
  // next call come back as a refusal, which is how 4i gets at the one
  // branch a person meets when something is read-only or already gone —
  // without a second file in the listing, which sections 3c and 4b
  // measure by shape.
  const removed = [];
  let removeRefuses = null;
  await page.route("**/remove", (r) => {
    const body = r.request().postDataJSON() ?? {};
    if (removeRefuses !== null) {
      const why = removeRefuses;
      removeRefuses = null;
      return r.fulfill({
        status: 400,
        contentType: "application/json",
        body: JSON.stringify(why),
      });
    }
    removed.push({ machine: body.machine ?? "", path: body.path ?? "" });
    return r.fulfill({ contentType: "application/json", body: "null" });
  });
  // This machine has a recycle bin, so the local sentence is the
  // reversible one and the tick is offered beside it. A face that read
  // the platform off the browser instead would never consult this.
  await page.route("**/trash_here", (r) =>
    r.fulfill({ contentType: "application/json", body: "true" }),
  );
  // Every listing asked for, so 4g can assert that stepping into a
  // directory really asked that machine about **that** path — the
  // breadcrumb changing is what a person sees, and a breadcrumb can
  // change over a listing nobody fetched.
  const askedLs = [];
  // **Echoes the path it was asked for**, empty meaning home (the `Ls`
  // contract, `khor_node::files`). It answered `pin.path` for every
  // path until 工作区批②, which no probe had noticed because none of
  // them walked into a directory — and a picker that walks is a picker
  // whose breadcrumb would have bounced straight back.
  await page.route("**/ls", (r) => {
    const body = r.request().postDataJSON() ?? {};
    askedLs.push({ machine: body.machine ?? "", path: body.path ?? "" });
    return r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        path: body.path === "" || body.path === undefined ? pin.path : body.path,
        entries: [
          { name: "src", dir: true, size: 0, at_ms: Date.now() },
          { name: "notes.md", dir: false, size: 1234, at_ms: Date.now() },
          // One of each road (工作区批6): a document and a picture.
          // **`notes.md` became a document**, which is why the code
          // view's sections now open `one.rs` — a pane that paints
          // markdown as prose has no caret and no gutter to measure.
          { name: "one.rs", dir: false, size: 400, at_ms: Date.now() },
          { name: "shot.png", dir: false, size: 9000, at_ms: Date.now() },
        ],
        truncated: false,
      }),
    });
  });
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
  // `components/PathCrumbs.tsx`; 「目录显示很丑,有很多奇怪的空格」 was the same
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
  //     ── 「还没答」那张脸,趁路由还挂着 ──────────────────────
  //
  //     **它必须在这里,不能在下面那一节。** 走查(第 3 节)有一站要打开
  //     这张卡并等下半张画出来,那一站会把这个挂起放掉;等到下面再测,
  //     它早就答过了。而这张脸只在**第一次**取数时存在——答案一旦落进
  //     state 就不会再回到「还没答」。
  //
  //     An account whose answer has not arrived draws **nothing**: an
  //     empty frame is not 「no reading」, it is the picture of an
  //     account that spent nothing, and that is a fact nobody
  //     established. The route is held, so this is a state to stand in
  //     rather than a race to win — on a real machine it is 18.5 s.
  await press(rail("quota"));
  await until(8000, async () => (await page.locator('[data-quota-row="sub:local"]').count()) > 0);
  await press('[data-quota-row="sub:local"]');
  await wait(500);
  if ((await page.locator("[data-quota-detail] [data-usage-half]").count()) !== 0) {
    fail("the usage half drew before its answer arrived — an empty chart is the picture of an account that spent nothing");
  }
  usageHold.on = false;
  usageHold.release();
  if (!(await until(8000, async () => (await page.locator("[data-usage-half] [data-usage-chart]").count()) > 0))) {
    fail("probe dead: the usage half never drew after the answer landed, so 「it draws nothing while asking」 measured nothing");
  }

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
    // **The walk changed on 2026-09-11** (工作区替代文件): the disk is
    // not a browsing landing any more, so the door is the workspace
    // list and the house is one workspace open. The controls being
    // measured — the breadcrumb, the directory rows — are the same
    // ones, under a different owner.
    { name: "workspaces", anchor: "[data-ws-row]", go: () => press(rail("workspaces")) },
    {
      // …and a workspace actually open, which is where those controls
      // are. Stopping at the list would be a walk that visits the door
      // and reports on the house.
      name: "workspaces · one open",
      anchor: "[data-crumb]",
      go: async () => (await press(rail("workspaces"))) && press("[data-ws-row]"),
    },
    {
      // …and a file open inside it, which is where the change gutter is
      // (工作区批3). Its own stop rather than more work inside the one
      // above: the gutter is a column of targets and the reach and
      // says-so sections judge what is on screen at a stop, so a gutter
      // that only existed halfway through another stop's `go` would be
      // measured by nothing.
      name: "workspaces · one file",
      anchor: "[data-gutter]",
      go: async () =>
        (await press(rail("workspaces"))) &&
        (await press("[data-ws-row]")) &&
        press('[data-file="one.rs"] button'),
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
    {
      // ⚠️ **The stop this walk did not have, added 2026-09-08.**
      // 額度's own stop presses the rail and anchors on a *row*, so the
      // walk stopped at the list — **nothing in it ever opened an
      // account's detail**, and the whole lower half (its column heads,
      // its axis pickers) lived outside every finger-floor assertion in
      // section 3. Measured that day, before this stop existed: the
      // column heads were 23.3px and the axis pickers 23px, against a
      // floor of 44 — and the gate was green, because it could not see
      // them.
      //
      // **A floor nothing walks to is not a floor.** The half is only
      // drawn once the usage answer has landed, which is why the hold
      // above is released before this walk starts.
      name: "\u989d\u5ea6 \u00b7 \u4e00\u5f20\u5361\u7684\u4e0b\u534a\u5f20",
      anchor: "[data-usage-half] [data-usage-chart]",
      go: async () => {
        await press(rail("quota"));
        await until(5_000, async () => (await page.locator('[data-quota-row="sub:local"]').count()) > 0);
        return press('[data-quota-row="sub:local"]');
      },
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
    "data-plot-point": [
      "src/views/MachineCard.tsx",
      "a point on the scatter, and the floor is the wrong criterion because a " +
        "44px point is not a small point — it is the plot. Two hundred of them " +
        "at that size would tile the whole drawing and there would be nothing " +
        "left to read; the *drawn* dot is 1.8px for exactly that reason, and " +
        "the target is already a transparent circle four times its radius. " +
        "This is WCAG 2.5.5's Essential carve-out in its literal form: the " +
        "size is determined by the information being presented. **What it " +
        "does not buy:** a keyboard, and a finger. A point cannot be tabbed " +
        "to and cannot be reliably hit on a touch screen — and what a point " +
        "*does* is jump to a row of the table directly under it, which is the " +
        "canonical list, is fully reachable by keyboard, and is where that " +
        "reader was going anyway. So the affordance is additive: nothing is " +
        "only available through it. **It is not「no problem」** — a touch user " +
        "on a dense plot will miss, and the answer to that is a bigger idea " +
        "(brush, or a nearest-point hit test), not a bigger dot.",
    ],
    "data-usage-point": [
      "src/views/UsageHalf.tsx",
      "a dot on the 額度 card's scatter. **The same waiver the machine " +
        "card's points carry, and it has to be argued again rather than " +
        "borrowed, because that card's reason — two hundred of them would " +
        "tile the drawing — is not this one's: there are seven here.** " +
        "The reason here is that **the dot's size is a reading**: its area " +
        "is the third quantity, so a dot grown to the floor is a dot whose " +
        "size has stopped meaning anything, which is WCAG 2.5.5's Essential " +
        "carve-out in its literal form. The target is already a transparent " +
        "circle at 20 viewBox units — about 22px across the wide face and " +
        "9px on a 390px phone, which is as large as the drawing can afford " +
        "at each width, and the phone is where these measured 15–34px. " +
        "**What it does not buy:** a finger on the narrow face. A touch " +
        "user there will miss, and the honest answer to that is a nearest- " +
        "point hit test, not a bigger dot. What a dot *does* is mark the " +
        "row under it in the table — the canonical list, reachable by " +
        "keyboard, and where that reader was going anyway — so nothing is " +
        "available only through a dot. **It is not 「no problem」.**",
    ],
    "data-hunk": [
      "src/views/FileView.tsx",
      "one change bar in a workspace file's gutter. **The size is the " +
        "reading**: a bar stands beside the lines it is about, so its " +
        "height *is* how many lines changed — a one-line change measures " +
        "21.6 tall because a line of code is 21.6 tall, and grown to 44 it " +
        "would be a bar claiming two lines changed. That is WCAG 2.5.5's " +
        "Essential carve-out in its literal form, and the width is spent " +
        "the same way: the column is 11 across because every pixel of it " +
        "is taken off the code beside it, and a 44-wide gutter would push " +
        "the file two characters right on a phone to make room for a " +
        "decoration. A deletion is smaller still (a quarter of a line, " +
        "measured 5) because it marks a boundary between two lines rather " +
        "than covering either, and a notch tall enough to press would sit " +
        "on top of one of them and claim it changed. **What it does not " +
        "buy:** a finger. On a touch screen these are read, not pressed — " +
        "and what a bar *does* is unfold what HEAD had there, which is the " +
        "one thing in this view available nowhere else. So unlike the " +
        "scatter's points, this is **not** additive: a phone can see that " +
        "a file differs and cannot see how. **It is not「no problem」** — " +
        "the answer is a way into the diff that is not the bar (a hunk " +
        "list, or a press anywhere on the changed lines), which is a " +
        "bigger idea than a wider gutter.",
    ],
    "data-ws-seam": [
      "src/views/WorkspacePane.tsx",
      "a workspace's own seam — the one between the file tree and the file, and the one between " +
        "the file and the sessions. **The same shape as `data-list-seam` below and the argument has " +
        "to be made again rather than borrowed, because the two spend their width on different " +
        "things.** That one takes from a list and a detail; these take from a *file* — every pixel " +
        "the seam grows is a column of code that stops being on screen, on the one face in this app " +
        "whose whole job is showing code. WCAG 2.5.5's Essential carve-out in the same literal form: " +
        "the size is determined by the information being presented. **What it does not buy:** a " +
        "finger on a wide touch screen. It is also the one place where that costs less than it " +
        "looks — unlike the list seam, **both of these columns can be put away and brought back " +
        "from the head above them** (`gui.workspace_tree`, `gui.workspace_sessions`), and those two " +
        "controls do wear the floor. So a touch user loses the *width*, not the column. **It is " +
        "still not「no problem」**: they cannot choose a width at all, and the answer to that is a " +
        "width control that is not a two-pixel strip, not a wider strip.",
    ],
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
          // (`components/PathCrumbs.tsx` says why it is one target and not two), so
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
    await press(rail("workspaces"));
    // The walk may have left a workspace open, so the list row need not
    // be there — pressing it is an attempt, not a requirement.
    await press("[data-ws-row]");
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
            `screen with no finger on it (src/components/PathCrumbs.tsx).`,
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

  // ── 4b) the change gutter can be walked, and the count is the
  //        gutter's own ─────────────────────────────────────────────
  //
  // **What「点开是那一处」needs that pressing a bar cannot give.** A
  // hunk eight hundred lines down is not reachable by pressing
  // something nobody can see, so the two steps are the half of the
  // affordance that works off screen — and the count beside them has to
  // be a count of *the bars*, not of anything else, or the number and
  // the drawing are two different claims about one file.
  //
  // Measured rather than asserted from the fixture's own array: the
  // number on screen is compared to the bars on screen, so a fixture
  // edited later moves both together and this still judges something.
  await atCrumbs();
  await press('[data-file="one.rs"] button');
  const walk = await page.evaluate(async () => {
    const wait = () => new Promise((r) => setTimeout(r, 120));
    const bars = () => document.querySelectorAll("[data-hunk]").length;
    const said = () => {
      const el = document.querySelector("[data-hunk-count]");
      return el ? Number(el.dataset.hunkCount) : null;
    };
    const at = () => {
      const el = document.querySelector("[data-hunk-at]");
      return el ? Number(el.dataset.hunkAt) : null;
    };
    const next = document.querySelector('[data-hunk-step="1"]');
    const prev = document.querySelector('[data-hunk-step="-1"]');
    if (!next || !prev) return { bars: bars(), said: said(), steps: null };
    const seen = [at()];
    for (let i = 0; i < bars() + 1; i++) {
      next.click();
      await wait();
      seen.push(at());
    }
    prev.click();
    await wait();
    const back = at();
    // Which hunk the open panel belongs to, read off the panel rather
    // than off the state: they are two different claims and only one of
    // them is on screen.
    const panel = document.querySelector("[data-was]");
    const bar = document.querySelectorAll("[data-hunk]")[back - 1];
    return {
      bars: bars(),
      said: said(),
      steps: seen,
      back,
      panelAt: panel ? Number(panel.dataset.was) : null,
      barAt: bar ? Number(bar.dataset.hunk) : null,
    };
  });
  console.log(
    `hunk walk: ${walk.bars} bars, the bar says ${walk.said}, steps ${JSON.stringify(walk.steps)}, ` +
      `back to ${walk.back} whose panel is at line ${walk.panelAt} and whose bar is at ${walk.barAt}`,
  );
  if (walk.bars === 0) {
    fail(
      "probe void: no change bars on the open file — this section walks them and cannot do it " +
        "with none (deepen the **/git_diff fixture)",
    );
  } else if (walk.steps === null) {
    fail("probe dead: the file view drew change bars but no way to step through them (src/views/FileView.tsx)");
  } else {
    if (walk.said !== walk.bars) {
      fail(`the file says ${walk.said} changes and draws ${walk.bars} bars — one file, one count (src/views/FileView.tsx)`);
    }
    // Nothing open, then 1, 2, … and round to 1 again: the wrap is the
    // half that keeps the control alive on the last hunk.
    const want = [null, ...Array.from({ length: walk.bars }, (_, i) => i + 1), 1];
    if (JSON.stringify(walk.steps) !== JSON.stringify(want)) {
      fail(
        `stepping through ${walk.bars} changes counted ${JSON.stringify(walk.steps)}, wanted ` +
          `${JSON.stringify(want)} (src/views/FileView.tsx)`,
      );
    }
    if (walk.panelAt === null || walk.panelAt !== walk.barAt) {
      fail(
        `after stepping, the open panel is at line ${walk.panelAt} and the hunk it names is at ` +
          `${walk.barAt} — 「点开是那一处」 is the whole claim (src/views/FileView.tsx)`,
      );
    }
  }

  // …and the panel covers no code, which is the whole of what changed
  // on 2026-09-13. It used to be an overlay laid at the hunk's own
  // line, which put the removed lines where they had been and hid
  // however many current lines it was tall — a cost carried as a known
  // one for a day. **Measured as two boxes that do not intersect**,
  // because「it looks fine」is exactly what an overlay looks like until
  // the file is long enough.
  // **Taken on the first hunk, and that is the whole of whether this
  // check is worth anything.** The walk above ends on the last one,
  // which in this fixture sits on the file's last line — an overlay
  // there covers nothing, because there is nothing under it. Measured
  // where the walk happened to stop, the check reported no overlap
  // against the old overlay too and passed on the behaviour it exists
  // to forbid (2026-09-13). The first hunk has three lines below it.
  const lap = await page.evaluate(async () => {
    const bar = document.querySelector("[data-hunk]");
    if (!bar) return null;
    bar.click();
    await new Promise((r) => setTimeout(r, 60));
    const panel = document.querySelector("[data-was]");
    // **`[data-sheet]` and no fallback.** The first spelling fell back
    // to `document.querySelector("pre")`, which is the line-number
    // column. A probe that cannot find its subject must go void, never
    // green.
    const code = document.querySelector("[data-sheet]");
    if (!panel || !code) return null;
    const p = panel.getBoundingClientRect();
    const c = code.getBoundingClientRect();
    const over = Math.min(p.bottom, c.bottom) - Math.max(p.top, c.top);
    return {
      over: Math.round(Math.max(0, over)),
      panel: Math.round(p.height),
      code: Math.round(c.height),
    };
  });
  if (lap === null) {
    fail("probe dead: no expanded panel to measure against the code (src/views/FileView.tsx)");
  } else {
    console.log(
      `hunk panel: ${lap.panel}px tall over a ${lap.code}px file, overlapping it by ${lap.over}px`,
    );
    if (lap.over > 1) {
      fail(
        `the expanded panel lies over ${lap.over}px of the file — it names the lines HEAD had and ` +
          "hiding the ones that replaced them is the comparison losing its other half " +
          "(src/views/FileView.tsx)",
      );
    }
  }

  // ── 4b-bis) saving, going back, and the two ways out ──────────────
  //
  // **Three things no gate could see until today**, and each one is a
  // dead end if it is wrong rather than a wrong picture. A save that
  // refuses used to leave a sentence standing with nothing to press:
  // the only move left was copying the text out by hand. And a save
  // that lands used to be the end of the previous content — khor wrote
  // it and had no way back, which is exactly the minute somebody
  // notices they saved the wrong thing (2026-09-13).
  //
  // Driven through the real buttons, never through state: what is
  // being checked is that a person has a way out, and a way out is a
  // thing you can press.
  const editing = await page.$("textarea");
  if (editing === null) {
    fail("probe void: nothing editable on the open file, so nothing to save (src/views/FileView.tsx)");
  } else {
    // A save khor takes. The digest it answers is the one the next save
    // pins to.
    await page.route("**/write_text", (r) =>
      r.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ at_ms: Date.now(), digest: "1".repeat(64) }),
      }),
    );
    await editing.click();
    await page.keyboard.type("X");
    await press("[data-save]");
    await new Promise((r) => setTimeout(r, 120));
    const afterSave = await page.evaluate(() => ({
      state: document.querySelector("[data-save-state]")?.dataset.saveState ?? null,
      undo: !!document.querySelector("[data-undo]"),
      text: document.querySelector("textarea")?.value ?? "",
    }));
    console.log(
      `save: state ${afterSave.state}, a way back is ${afterSave.undo ? "offered" : "absent"}`,
    );
    if (afterSave.state !== "saved") {
      fail(`a save the far machine took left the file saying ${afterSave.state} (src/views/FileView.tsx)`);
    }
    if (!afterSave.undo) {
      fail(
        "a save landed and offered no way back to what was there — khor replaced the file and kept " +
          "nothing, which is the one minute somebody needs it (src/views/FileView.tsx)",
      );
    } else {
      await press("[data-undo]");
      await new Promise((r) => setTimeout(r, 80));
      const back = await page.evaluate(() => document.querySelector("textarea")?.value ?? "");
      console.log(`back: the box holds ${JSON.stringify(back.slice(0, 24))}`);
      if (back === afterSave.text) {
        fail(
          "「回到存之前」 left the box exactly as the save did — it is supposed to put the replaced " +
            "text back (src/views/FileView.tsx)",
        );
      }
      // **Into the box, not onto the disk.** khor writes when somebody
      // presses 存 and at no other moment, so the file must now read as
      // unsaved rather than as written.
      const nowState = await page.evaluate(
        () => document.querySelector("[data-save-state]")?.dataset.saveState ?? null,
      );
      if (nowState !== "unsaved") {
        fail(
          `going back left the file saying ${nowState} — putting the old text in the box is not ` +
            "writing it, and saying 存好了 about bytes nobody wrote is the worst of the three " +
            "(src/views/FileView.tsx)",
        );
      }
    }

    // …and a save the far machine refuses, which is the case that had
    // no exit at all.
    await page.unroute("**/write_text");
    await page.route("**/write_text", (r) =>
      r.fulfill({ status: 500, contentType: "application/json", body: JSON.stringify("这个文件在你编辑期间变了") }),
    );
    await press("[data-save]");
    await new Promise((r) => setTimeout(r, 120));
    const refused = await page.evaluate(() => ({
      state: document.querySelector("[data-save-state]")?.dataset.saveState ?? null,
      theirs: !!document.querySelector("[data-take-theirs]"),
      mine: !!document.querySelector("[data-keep-mine]"),
      merge: !!document.querySelector("[data-merge]"),
    }));
    console.log(
      `refused: state ${refused.state}, exits ${[refused.theirs && "theirs", refused.merge && "merge", refused.mine && "mine"]
        .filter(Boolean)
        .join(" + ") || "none"}`,
    );
    if (refused.state !== "refused") {
      fail(`a save the far machine refused left the file saying ${refused.state} (src/views/FileView.tsx)`);
    }
    if (!refused.theirs || !refused.mine || !refused.merge) {
      fail(
        "a refused save offers no way out — the sentence says the file moved and the only move left " +
          "is copying the text out by hand (src/views/FileView.tsx)",
      );
    }

    // **The third exit, driven, and what it is asked with.** The merge
    // itself is Rust and tested there (`khor_node::merge`); what only
    // this side can get wrong is which text goes in which hole — and
    // handing the box in as 「base」 is a merge that silently throws
    // away exactly the change the person pressed the button to keep.
    // So the request is read, not just the answer.
    let asked = null;
    await page.route("**/merge_text", (r) => {
      asked = JSON.parse(r.request().postData() ?? "{}");
      return r.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ text: "merged-by-the-fixture\n", clashes: 2 }),
      });
    });
    // 「theirs」 comes off a fresh read, so the file has to have moved
    // over there for the merge to be about anything.
    await page.unroute("**/read_text");
    await page.route("**/read_text", (r) =>
      r.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          total: 42,
          at_ms: Date.now(),
          text: "# notes\n\nwhat the other person wrote\n",
          cut: false,
          binary: false,
          digest: "2".repeat(64),
        }),
      }),
    );
    await press("[data-merge]");
    await new Promise((r) => setTimeout(r, 160));
    const merged = await page.evaluate(() => ({
      box: document.querySelector("textarea")?.value ?? "",
      said: document.querySelector("[data-merged]")?.dataset.merged ?? null,
      state: document.querySelector("[data-save-state]")?.dataset.saveState ?? null,
    }));
    console.log(
      `merge: asked with base ${JSON.stringify((asked?.base ?? "").slice(0, 18))}, ` +
        `box now ${JSON.stringify(merged.box.slice(0, 22))}, said ${merged.said}`,
    );
    if (asked === null) {
      fail("「合起来」 was pressed and khor was never asked to merge anything (src/views/FileView.tsx)");
    } else {
      if (asked.base === asked.mine) {
        fail(
          "the merge was asked with the box as its own base — that is a two-way merge wearing a " +
            "three-way name, and it drops the other side's change every time (src/views/FileView.tsx)",
        );
      }
      if (!asked.theirs?.includes("what the other person wrote")) {
        fail(
          "the merge's 「theirs」 is not what the machine answered just now — it is merging against " +
            "a stale read (src/views/FileView.tsx)",
        );
      }
    }
    if (!merged.box.includes("merged-by-the-fixture")) {
      fail("the merged text never reached the box (src/views/FileView.tsx)");
    }
    if (merged.said !== "2") {
      fail(
        `the merge left clashes in the text and the screen said ${merged.said} — a number nobody ` +
          "sees is a number nobody settles (src/views/FileView.tsx)",
      );
    }
    // **Merged is not saved.** khor writes when somebody presses 存;
    // a merge that painted 存好了 would be claiming bytes nobody wrote.
    if (merged.state === "saved") {
      fail(
        "the file read as saved straight after a merge — nothing was written, and saying otherwise " +
          "is the worst answer khor can give (src/views/FileView.tsx)",
      );
    }
    await page.unroute("**/merge_text");
    await page.unroute("**/write_text");
  }

  // ── 4b-ter) the marks, when somebody comes back ───────────────────
  //
  // **A pane that was in the background is a pane holding an answer
  // from before.** The marks are asked when the workspace opens, when
  // the tree walks and after khor's own save; none of those fire while
  // the window sits behind something else and somebody else commits.
  // So returning to the window asks again — and nothing asks on a
  // timer, because every beat is a dial plus a `git status` walk on
  // another machine for an answer that rarely moved.
  const hide = (yes) =>
    page.evaluate((v) => {
      Object.defineProperty(document, "hidden", { configurable: true, get: () => v });
      document.dispatchEvent(new Event("visibilitychange"));
    }, yes);
  const quiet = gitAsked;
  await new Promise((r) => setTimeout(r, 700));
  const stillQuiet = gitAsked;
  await hide(true);
  await new Promise((r) => setTimeout(r, 80));
  await hide(false);
  await new Promise((r) => setTimeout(r, 200));
  console.log(
    `marks: ${quiet} asked so far, ${stillQuiet - quiet} while nobody touched it, ` +
      `${gitAsked - stillQuiet} on coming back`,
  );
  if (stillQuiet !== quiet) {
    fail(
      `the marks were asked ${stillQuiet - quiet} time(s) with nobody touching the pane — that is a ` +
        "poll, and every beat is a dial and a git walk on somebody else's machine " +
        "(src/views/WorkspacePane.tsx)",
    );
  }
  if (gitAsked === stillQuiet) {
    fail(
      "coming back to the window left the marks as they were — a person who was away while somebody " +
        "else committed is reading an answer from before and cannot tell (src/views/WorkspacePane.tsx)",
    );
  }
  await hide(false);

  // ── 4f) the workspace row's four corners ──────────────────────────
  //
  // **The list is a table now** (工作区批①, user ruling): a workspace is
  // `(machine, directory)` that somebody declared, not a checkout khor
  // inferred. What a row says is four facts in four corners — its name
  // beside its machine's face, where it is, what git says, when it last
  // changed — and two of those four come from the machine holding the
  // files, which can refuse.
  //
  // Claims, in the order a wrong one costs most:
  //   i. a machine that cannot answer loses the **corner**, never the
  //      row — the failure this landing's whole old design was: a
  //      refusal painted as an absence sends somebody to look somewhere
  //      else instead of at that machine;
  //  ii. a directory that is no repository has an **empty** git corner
  //      — not「无 git」, which would be a word for a non-fact, and not
  //      「干净」, which would be a lie;
  // iii. the pinned row leads, whatever the times say;
  //  iv. all four corners are actually drawn;
  //   v. the 「+」 is on the bar and cannot be pressed — 批② is the
  //      picker it opens, and a pane whose only way to add is invisible
  //      reads as a pane that cannot;
  //  vi. a row whose machine has **left the device table** is not asked
  //      anything at all. Its `machine` is empty, and an empty machine
  //      means「this one」to every verb the node has — so an ask would
  //      come back with *this* Mac's branch and mtime for a path on
  //      another one. **Both halves are needed**: blank corners alone
  //      cannot tell「never asked」from「asked and refused」, and only
  //      the first is safe, so the question that was never sent is what
  //      this asserts.
  await press(rail("workspaces"));
  // **The gate is「any row at all」and not「all three」**, because a
  // row missing is exactly what claim i is about: asserting the count
  // here would report it as a dead probe — a sentence about this file —
  // instead of as the product hiding a machine that went quiet.
  const corners = await until(5_000, async () =>
    (await page.locator("[data-ws-row]").count()) > 0,
  );
  if (!corners) {
    fail("probe dead: the workspace landing drew no rows (src/views/WorkspacesList.tsx)");
  } else {
    // Give the two per-row questions a moment to land; the assertions
    // below are about what arrives, not about how fast.
    await until(5_000, async () =>
      (await page.locator("[data-ws-touched]").first().innerText()).trim() !== "",
    );
    const seen = await page.$$eval("[data-ws-row]", (rows) =>
      rows.map((el) => ({
        pinned: el.getAttribute("data-ws-pinned"),
        name: el.querySelector("[data-ws-name]")?.textContent?.trim() ?? null,
        where: el.querySelector("[data-ws-where]")?.textContent?.trim() ?? null,
        git: el.querySelector("[data-ws-git]")?.textContent?.trim() ?? null,
        touched: el.querySelector("[data-ws-touched]")?.textContent?.trim() ?? null,
      })),
    );
    console.log(`workspaces: ${JSON.stringify(seen)}`);
    for (const corner of ["name", "where", "git", "touched"]) {
      if (seen.some((r) => r[corner] === null)) {
        fail(
          `a workspace row has no ${corner} corner — the anchor is the fact, and a corner that is not ` +
            "there cannot be read as empty either (src/views/WorkspacesList.tsx)",
        );
      }
    }
    if (seen[0]?.name !== "work" || seen[0]?.pinned !== "true") {
      fail(
        `the workspace list leads with ${JSON.stringify(seen[0])} — a pinned row leads, whatever the ` +
          "times say (src/views/WorkspacesList.tsx)",
      );
    }
    const gate = seen.find((r) => r.name === "notes");
    if (gate?.git !== "") {
      fail(
        `a directory that is no repository drew 「${gate?.git}」 in its git corner — nothing is the ` +
          "only honest answer there, since 干净 and 读不了 look alike (src/views/WorkspacesList.tsx)",
      );
    }
    const peer = seen.find((r) => r.name === "work");
    if (!peer?.git?.startsWith("main")) {
      fail(
        `the repository row drew 「${peer?.git}」 where its branch goes — without the positive half ` +
          "the assertion above passes on a corner that never draws anything (src/views/WorkspacesList.tsx)",
      );
    }
    const elder = seen.find((r) => r.name === "app");
    if (elder === undefined) {
      fail(
        "the row whose machine could not answer is gone from the list — a refusal painted as an " +
          "absence sends somebody to look anywhere but at that machine (src/views/WorkspacesList.tsx)",
      );
    } else if (elder.touched !== "" || elder.git !== "") {
      fail(
        `the unreachable machine's row drew 「${elder.git}」/「${elder.touched}」 — a machine that ` +
          "said nothing must leave the corners blank (src/views/WorkspacesList.tsx)",
      );
    } else if (elder.where !== "elder:/srv/elder/app") {
      fail(
        `…and it must still say where it is, not 「${elder.where}」 — the row is openable and the ` +
          "path is what makes it identifiable (src/views/WorkspacesList.tsx)",
      );
    }
    const ghost = wsRows.find((w) => w.machine === "");
    const ghostRow = seen.find((r) => r.name === "left");
    if (ghostRow === undefined) {
      fail(
        "the row whose machine has left the device table is gone from the list — it is the one row " +
          "that can only be taken off from here, and hiding it makes it permanent " +
          "(src/views/WorkspacesList.tsx)",
      );
    } else if (ghostRow.git !== "" || ghostRow.touched !== "") {
      fail(
        `the nameless row drew 「${ghostRow.git}」/「${ghostRow.touched}」 — there is no machine to ` +
          "have answered, so whatever is in those corners came from the wrong disk " +
          "(src/views/WorkspacesList.tsx)",
      );
    } else if (ghostRow.where !== ghost.path) {
      fail(
        `…and with no machine to name it must say just the path, not 「${ghostRow.where}」 ` +
          "(src/views/WorkspacesList.tsx)",
      );
    }
    console.log(
      `workspaces asked: git ${JSON.stringify(askedAbout.git_status.map((a) => a.machine))}, ` +
        `touched ${JSON.stringify(askedAbout.touched.map((a) => a.machine))}`,
    );
    for (const [op, asks] of Object.entries(askedAbout)) {
      const nameless = asks.filter((a) => a.machine === "");
      if (nameless.length) {
        fail(
          `${op} was asked ${nameless.length} time(s) with no machine named — the node reads that ` +
            "as 「this one」, so the answer is this machine's and the row it lands on is another " +
            "machine's (src/views/WorkspacesList.tsx)",
        );
      }
      const atGhost = asks.filter((a) => a.path === ghost.path);
      if (atGhost.length) {
        fail(
          `${op} was asked about ${ghost.path}, whose machine is not in the device table — ` +
            `as ${JSON.stringify(atGhost)} (src/views/WorkspacesList.tsx)`,
        );
      }
      // The positive half: without it both assertions above would pass
      // on a landing that asked nothing at all.
      if (!asks.some((a) => a.machine === "peer")) {
        fail(`probe dead: ${op} was never asked about peer either, so nothing above means anything`);
      }
    }
    const plus = page.locator("[data-ws-add]");
    if ((await plus.count()) !== 1) {
      fail(
        `the workspace bar has ${await plus.count()} 「+」 — a pane whose only way to add something ` +
          "is invisible reads as a pane that cannot (src/App.tsx)",
      );
    } else if (!(await plus.isEnabled())) {
      fail(
        "the workspace 「+」 cannot be pressed — 批② built the picker behind it, and the empty " +
          "state now points at it (src/App.tsx)",
      );
    }
  }

  // ── 4g) the directory picker ──────────────────────────────────────
  //
  // **One dialog answers 哪台机器上的哪个目录, and two surfaces open
  // it** (工作区批②): the workspace pane's 「+」 and the session
  // wizard's 目录 box. It is the same act at both ends, so it is the
  // same screen — and the two ends differ only in what they do with the
  // answer.
  //
  // Claims:
  //   i. the 「+」 opens it;
  //  ii. it lists **directories only** — a picker offering files is
  //      offering something it cannot answer with;
  // iii. stepping into one really asks that machine about that path.
  //      The breadcrumb alone would not prove it: a breadcrumb is drawn
  //      from what this face believes, and what this face believes is
  //      exactly what a picker that never asked would also show;
  //  iv. confirming declares **what was picked**, and the row turns up;
  //   v. the wizard's way in writes into its 目录 box — the box stays
  //      the answer, the picker is a second road to it;
  //  vi. and it is usable on a phone.
  const dirPicker = async () => {
    await press(rail("workspaces"));
    await press("[data-ws-add]");
    return until(5_000, async () => (await page.locator("[data-dir-picker]").count()) === 1);
  };
  if (!(await dirPicker())) {
    fail("probe dead: the workspace 「+」 opened no picker (src/components/DirPicker.tsx)");
  } else {
    const rows = await page.$$eval("[data-dir-picker-row]", (els) =>
      els.map((e) => e.getAttribute("data-dir-picker-row")),
    );
    // The fixture's listing is one directory among several files.
    if (rows.length !== 1 || rows[0] !== "src") {
      fail(
        `the picker drew ${JSON.stringify(rows)} — only directories belong in a dialog whose whole ` +
          "question is which directory (src/components/DirPicker.tsx)",
      );
    }
    // A machine that is not the one it opened on, because the picker's
    // reason to exist is that the directory may be somewhere else.
    await press('[data-dir-picker-machine="peer"]');
    await until(3_000, async () => askedLs.some((a) => a.machine === "peer"));
    if (!askedLs.some((a) => a.machine === "peer" && a.path === "")) {
      fail(
        `picking a machine did not walk back to its home — asked ${JSON.stringify(askedLs.slice(-3))}, ` +
          "and a path typed for one disk means nothing on another (src/components/DirPicker.tsx)",
      );
    }
    const into = `${pin.path}/src`;
    await press('[data-dir-picker-row="src"]');
    const walked = await until(5_000, async () =>
      askedLs.some((a) => a.machine === "peer" && a.path === into),
    );
    if (!walked) {
      fail(
        `stepping into a directory asked for ${JSON.stringify(askedLs.slice(-3))} and never for ` +
          `${into} — the breadcrumb is drawn from what this face believes, so it moves either way ` +
          "(src/components/DirPicker.tsx)",
      );
    }
    const crumb = await page.$$eval("[data-dir-picker] [data-crumb]", (els) =>
      els.map((e) => e.getAttribute("data-crumb")).pop(),
    );
    if (crumb !== into) {
      fail(`…and the breadcrumb stayed at 「${crumb}」 rather than following (src/components/DirPicker.tsx)`);
    }
    const box = await page.locator("[data-dir-picker-path]").inputValue();
    if (box !== into) {
      fail(
        `the box says 「${box}」 where the walk is ${into} — the box is what gets confirmed, so a ` +
          "walk it did not follow is a click that picks somewhere else (src/components/DirPicker.tsx)",
      );
    }

    await press("[data-dir-picker-take]");
    const declared = await until(5_000, async () => addedWs.length > 0);
    if (!declared) {
      fail("probe dead: 「选这个目录」 declared nothing (src/App.tsx)");
    } else if (addedWs[0].machine !== "peer" || addedWs[0].path !== into) {
      fail(
        `the 「+」 declared ${JSON.stringify(addedWs[0])} — it must be the pair the picker was ` +
          "standing on, machine and path (src/App.tsx)",
      );
    }
    console.log(`dir picker: rows ${JSON.stringify(rows)}, declared ${JSON.stringify(addedWs[0])}`);
    // …and the list is re-read, which is the half a successful call
    // does not prove: the pane bar lives above the list and the list
    // owns every read it makes (`App`'s `wsAdded`).
    const landed = await until(5_000, async () =>
      (await page.locator("[data-ws-row] [data-ws-name]").allInnerTexts()).includes("src"),
    );
    if (!landed) {
      fail(
        "the declared workspace never turned up in the list — the pane bar declared it and nothing " +
          "told the list to look again (src/App.tsx, src/views/WorkspacesList.tsx)",
      );
    }
    if ((await page.locator("[data-dir-picker]").count()) !== 0) {
      fail("the picker stayed open after it was answered (src/components/DirPicker.tsx)");
    }
  }

  // The wizard's way in. **The box is still the answer** — every probe
  // in `smoke` types into it — so this asserts the picker writes there
  // rather than replacing it.
  await press(rail("sessions"));
  await press("[data-pane-new]");
  await press('[data-new-item="new"]');
  const wizard = await until(5_000, async () =>
    (await page.locator("[data-new-session-pick]").count()) === 1,
  );
  if (!wizard) {
    fail("probe dead: the wizard drew no way into the picker (src/views/NewSessionDialog.tsx)");
  } else {
    await press("[data-new-session-pick]");
    if (!(await until(5_000, async () => (await page.locator("[data-dir-picker]").count()) === 1))) {
      fail("the wizard's 「挑…」 opened no picker (src/views/NewSessionDialog.tsx)");
    } else {
      await press('[data-dir-picker-row="src"]');
      await press("[data-dir-picker-take]");
      const wrote = await until(
        5_000,
        async () => (await page.locator("[data-new-session-dir]").inputValue()) !== "~",
      );
      const said = await page.locator("[data-new-session-dir]").inputValue();
      console.log(`wizard picked: ${said}`);
      if (!wrote || !said.endsWith("/src")) {
        fail(
          `the picker answered into 「${said}」 — the 目录 box is where the answer lands and every ` +
            "other road already writes there (src/views/NewSessionDialog.tsx)",
        );
      }
    }
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");
  }

  // …and on a phone. The dialog's own shell handles the width
  // (`ui/dialog` is `w-full max-w-[calc(100%-2rem)]`), so what is
  // measured here is that the body still fits inside it: a picker whose
  // list or footer hangs out of the sheet is one a finger cannot finish.
  await page.setViewportSize({ width: 390, height: 844 });
  await wait(200);
  if (!(await dirPicker())) {
    fail("probe dead: the 「+」 opened no picker at 390 (src/components/DirPicker.tsx)");
  } else {
    const fit = await page.evaluate(() => {
      const sheet = document.querySelector("[data-dir-picker]");
      const box = sheet.getBoundingClientRect();
      const take = sheet.querySelector("[data-dir-picker-take]").getBoundingClientRect();
      const row = sheet.querySelector("[data-dir-picker-row]")?.getBoundingClientRect() ?? null;
      const px = (n) => Math.round(n * 10) / 10;
      return {
        sheet: { w: px(box.width), h: px(box.height) },
        over: px(Math.max(0, box.right - window.innerWidth) + Math.max(0, -box.left)),
        takeIn: take.right <= box.right + 0.5 && take.bottom <= box.bottom + 0.5,
        rowIn: row === null || (row.right <= box.right + 0.5 && row.left >= box.left - 0.5),
        tall: px(Math.max(0, box.bottom - window.innerHeight)),
      };
    });
    console.log(`dir picker at 390: ${JSON.stringify(fit)}`);
    if (fit.over > 0 || fit.tall > 0) {
      fail(
        `the picker hangs ${fit.over} past the side and ${fit.tall} past the foot at 390 — a sheet ` +
          "that does not fit the screen is one a finger cannot finish (src/components/DirPicker.tsx)",
      );
    }
    if (!fit.takeIn || !fit.rowIn) {
      fail(
        `at 390 the picker's ${!fit.takeIn ? "confirm" : "listing"} sits outside its own sheet ` +
          "(src/components/DirPicker.tsx)",
      );
    }
    await page.keyboard.press("Escape");
  }
  await page.setViewportSize({ width: 1400, height: 900 });
  await wait(200);

  // ── 4h) the workspace's own sessions ──────────────────────────────
  //
  // **One project's column, drawn by the sessions list itself**
  // (工作区批③). It used to hand-draw a title and a state word, which
  // is three of the five questions missing and a second answer to
  // 「what does a session row look like」.
  //
  // Claims:
  //   i. a session belongs here when it is running on **this machine**
  //      and **under** this directory. Both halves have their own
  //      counter-example in the fixture, each wrong on one axis only:
  //      `/home/peer/workshop` on peer (a sibling, not a child — the
  //      prefix collision) and `/home/peer/work` on elder (the same
  //      path, another machine — a workspace is the pair). Neither is
  //      visible to a person reading the column, because these rows do
  //      not repeat their machine;
  //  ii. the rows are the list's rows — `data-title` and `[data-last]`,
  //      not two spans this pane invented;
  // iii. and they do **not** repeat the machine (`[data-via]`), which
  //      the header above them already said;
  //  iv. a shell session is a row like any other, not a branch;
  //   v. pressing one opens it here, and there is a way back.
  await press(rail("workspaces"));
  await press("[data-ws-row]");
  // Wide with the list pane open, the column starts collapsed — the
  // fifth column's existence is what decides that (`WorkspacePane`).
  if ((await page.locator("[data-ws-sessions]").count()) === 0) {
    await press("[data-ws-sessions-switch]");
  }
  const column = await until(5_000, async () =>
    (await page.locator("[data-ws-sessions] [data-row]").count()) > 0,
  );
  if (!column) {
    fail("probe dead: the workspace drew no session column (src/views/WorkspacePane.tsx)");
  } else {
    const seen = await page.$$eval("[data-ws-sessions] [data-row]", (els) =>
      els.map((e) => ({
        title: e.getAttribute("data-title"),
        last: e.querySelector("[data-last]")?.textContent?.trim() ?? null,
        via: e.querySelector("[data-via]") !== null,
      })),
    );
    console.log(`workspace sessions: ${JSON.stringify(seen)}`);
    const titles = seen.map((r) => r.title).sort();
    if (JSON.stringify(titles) !== JSON.stringify(["ws-session-2", "ws-session-3"])) {
      fail(
        `the workspace column holds ${JSON.stringify(titles)} — this workspace is ` +
          "peer:/home/peer/work, so it is every session on **peer** under that directory and " +
          "nothing else. ws-session-4 is peer:/home/peer/workshop, a sibling rather than a child; " +
          "ws-session-5 is elder:/home/peer/work, the same path on another machine — and neither " +
          "row says so on screen (src/views/WorkspacePane.tsx)",
      );
    }
    if (seen.some((r) => r.last === null || r.last === "")) {
      fail(
        `a row in the column said nothing of its last line (${JSON.stringify(seen)}) — these are the ` +
          "sessions list's rows, and 最近一条 is one of the five questions (src/views/SessionsList.tsx)",
      );
    }
    if (seen.some((r) => r.via)) {
      fail(
        "a row in the column repeats its machine — the header directly above it already said which " +
          "machine this directory is on (src/views/SessionsList.tsx `showVia`)",
      );
    }
    // The shell row is `shell/ws-3`; it must be one of the two, drawn
    // by the same component rather than skipped or special-cased.
    if ((await page.locator('[data-ws-sessions] [data-row="shell/ws-3"]').count()) !== 1) {
      fail(
        "the shell session is not in the column — a shell answers the same five questions as a " +
          "conversation and is one row like any other (src/views/WorkspacePane.tsx)",
      );
    }
    // …and pressing one opens it beside its own files, with a way back.
    await press('[data-ws-sessions] [data-row="shell/ws-3"] [data-row-open]');
    const opened = await until(5_000, async () =>
      (await page.locator("[data-ws-sessions] [data-detail-header]").count()) === 1,
    );
    if (!opened) {
      fail(
        "pressing a row in the column opened nothing — the column is a place to work, not a list of " +
          "what exists (src/views/WorkspacePane.tsx)",
      );
    } else {
      await press("[data-ws-sessions] [data-back]");
      const back = await until(5_000, async () =>
        (await page.locator("[data-ws-sessions] [data-row]").count()) > 0,
      );
      if (!back) {
        fail("…and there was no way back to the column (src/views/WorkspacePane.tsx)");
      }
    }
  }

  // ── 4i) deleting from the tree ────────────────────────────────────
  //
  // 「khor 可以删文件啊,用户仍然右键某个文件可以删除。但提示,跟 VS Code
  // 一样,可以不再提醒」 — the user, 2026-09-14 (工作区批④).
  //
  // Claims:
  //   i. the row's menu offers it, and pressing raises a sheet that
  //      names the thing;
  //  ii. confirming really asks the node, with the machine and the
  //      path — and the directory is read again afterwards, because
  //      the path did not change and nothing else would notice;
  // iii. **the tick is offered only where the act can be undone.** A
  //      far machine has no recycle bin to put anything in, so its
  //      sheet carries no 「不再问」 — an irreversible act must not be
  //      silenceable by a tick made once about something else;
  //  iv. …and where it is offered, ticking it really does skip the
  //      sheet next time;
  //   v. deleting the file somebody is reading closes the reading — a
  //      view onto a file that is not there is the one state this pane
  //      must not leave behind;
  //  vi. a refusal keeps the sheet open, in the node's own sentence.
  const rowAt = async (name) => {
    await press(rail("workspaces"));
    const n = await page.$$eval(
      "[data-ws-row]",
      (els, want) =>
        els.findIndex((e) => e.querySelector("[data-ws-name]")?.textContent?.trim() === want),
      name,
    );
    if (n < 0) return false;
    await page.locator("[data-ws-row]").nth(n).click();
    return until(5_000, async () => (await page.locator("[data-file]").count()) > 0);
  };
  // The menu hangs on the row's **button**, never on the row around it
  // (`FilesPane` — section 4 above is why), so the press lands there.
  const menuOn = async (file) => {
    // `.first()`: the name is the row's first button and 拿过来 is its
    // second, and only the name carries the menu.
    await page.locator(`[data-file="${file}"] > button`).first().click({ button: "right" });
    return until(3_000, async () => (await page.locator('[data-menu-item="remove"]').count()) === 1);
  };

  // **peer's workspace first** — another machine, so the sheet is the
  // one that cannot be taken back.
  if (!(await rowAt("work"))) {
    fail("probe dead: peer's workspace drew no tree (src/views/FilesPane.tsx)");
  } else if (!(await menuOn("notes.md"))) {
    fail("probe dead: a file row offers no way to delete it (src/views/FilesPane.tsx)");
  } else {
    await page.locator('[data-menu-item="remove"]').click();
    const sheet = await until(5_000, async () =>
      (await page.locator("[data-remove-dialog]").count()) === 1,
    );
    if (!sheet) {
      fail("pressing 删掉 raised no sheet (src/components/RemoveDialog.tsx)");
    } else {
      const says = (await page.locator("[data-remove-says]").innerText()).trim();
      console.log(`remove · far: ${JSON.stringify(says)}`);
      if (!says.includes("notes.md")) {
        fail(
          `the sheet says 「${says}」 without naming what it is about — a confirmation that does ` +
            "not name the thing is one people learn to press through (src/components/RemoveDialog.tsx)",
        );
      }
      if ((await page.locator("[data-remove-never-ask]").count()) !== 0) {
        fail(
          "a file on another machine offered 「不再问」 — there is no recycle bin over there, so " +
            "this is the act that cannot be taken back and the one that must keep asking " +
            "(src/components/RemoveDialog.tsx)",
        );
      }
      // vi) the refusal, before the success — the sheet has to survive
      // it, and the next press is what proves it still works.
      removeRefuses = "这个文件是只读的";
      await page.locator("[data-remove-go]").click();
      const kept = await until(5_000, async () =>
        (await page.locator("[data-remove-refused]").count()) === 1,
      );
      const said = kept ? (await page.locator("[data-remove-refused]").innerText()).trim() : "";
      if (!kept || said !== "这个文件是只读的") {
        fail(
          `a refused delete left 「${said}」 on a sheet that ${kept ? "stayed" : "closed"} — the ` +
            "node's own sentence belongs where the press was made (src/components/RemoveDialog.tsx)",
        );
      }
      const lsBefore = askedLs.length;
      await page.locator("[data-remove-go]").click();
      const went = await until(5_000, async () => removed.length > 0);
      if (!went) {
        fail("probe dead: confirming asked the node nothing (src/views/FilesPane.tsx)");
      } else {
        console.log(`remove · asked ${JSON.stringify(removed[0])}`);
        if (removed[0].machine !== "peer" || !removed[0].path.endsWith("/notes.md")) {
          fail(
            `删掉 asked for ${JSON.stringify(removed[0])} — it must be the machine holding the ` +
              "file and the path on it (src/views/FilesPane.tsx)",
          );
        }
        const reread = await until(5_000, async () => askedLs.length > lsBefore);
        if (!reread) {
          fail(
            "the directory was not read again after a delete — the path did not change, so nothing " +
              "else would notice, and the row stays on screen over a file that is gone " +
              "(src/views/FilesPane.tsx)",
          );
        }
      }
    }
  }

  // v) the open file closes with it.
  if (await rowAt("work")) {
    await press('[data-file="one.rs"] button');
    const reading = await until(5_000, async () =>
      (await page.locator("[data-edit]").count()) === 1,
    );
    if (!reading) {
      fail("probe dead: the file never opened, so closing it proves nothing");
    } else if (await menuOn("one.rs")) {
      await page.locator('[data-menu-item="remove"]').click();
      await until(5_000, async () => (await page.locator("[data-remove-dialog]").count()) === 1);
      await page.locator("[data-remove-go]").click();
      const shut = await until(5_000, async () =>
        (await page.locator("[data-edit]").count()) === 0,
      );
      if (!shut) {
        fail(
          "the file somebody was reading was deleted and the reading stayed open — a view onto a " +
            "file that is not there is the one state this pane must not leave (src/views/FilesPane.tsx)",
        );
      }
    }
  }

  // iii/iv) **this machine's workspace**, where the bin exists: the
  // tick is offered, and ticking it means the next one does not ask.
  if (!(await rowAt("notes"))) {
    fail("probe dead: gate's workspace drew no tree (src/views/FilesPane.tsx)");
  } else if (await menuOn("notes.md")) {
    await page.locator('[data-menu-item="remove"]').click();
    await until(5_000, async () => (await page.locator("[data-remove-dialog]").count()) === 1);
    const says = (await page.locator("[data-remove-says]").innerText()).trim();
    console.log(`remove · here: ${JSON.stringify(says)}`);
    if ((await page.locator("[data-remove-never-ask]").count()) !== 1) {
      fail(
        "a file on this machine offered no 「不再问」 — it goes to the recycle bin, which is the " +
          "one road where a tick costs nothing (src/components/RemoveDialog.tsx)",
      );
    } else {
      await page.locator("[data-remove-never-ask]").check();
      const was = removed.length;
      await page.locator("[data-remove-go]").click();
      await until(5_000, async () => removed.length > was);
      // …and now the next one goes straight through.
      if (await menuOn("one.rs")) {
        const before = removed.length;
        await page.locator('[data-menu-item="remove"]').click();
        const straight = await until(5_000, async () => removed.length > before);
        if (!straight) {
          fail(
            "「不再问」 was ticked and the next delete still asked — the tick is the whole of what " +
              "it promises (src/components/RemoveDialog.tsx)",
          );
        }
        if ((await page.locator("[data-remove-dialog]").count()) !== 0) {
          fail("…and the sheet came up anyway (src/views/FilesPane.tsx)");
        }
      }
    }
  }
  // The tick is this browser's, and every later section shares the
  // browser — put it back so nothing downstream inherits it.
  await page.evaluate(() => {
    try {
      window.localStorage.removeItem("khor.remove-never-ask");
    } catch {
      // Nothing to put back if it could not be written in the first
      // place; the sections below ask either way.
    }
  });

  // ── 4c) the caret sits on the ink ─────────────────────────────────
  //
  // **The editor is a transparent textarea laid over the painted
  // `<pre>`** (`views/FileView`), and the whole of why that works is
  // that the browser lays the two out identically — same font, size,
  // line height, padding, and `white-space: pre` on both. Nothing in
  // the language enforces that. One `className` edited on one of them
  // and the caret drifts a character further from its letter on every
  // column, which nobody notices in a screenshot and everybody notices
  // while typing.
  //
  // So: the two boxes, corner for corner, and the typographic
  // properties that decide where a glyph lands, read off both.
  await atCrumbs();
  await press('[data-file="one.rs"] button');
  const sheet = await page.evaluate(() => {
    const ta = document.querySelector("[data-edit]");
    if (!ta) return null;
    const pre = ta.parentElement.querySelector("pre");
    const px = (n) => Math.round(n * 10) / 10;
    const t = ta.getBoundingClientRect();
    const p = pre.getBoundingClientRect();
    const of = (el) => {
      const s = getComputedStyle(el);
      return {
        font: `${s.fontFamily}|${s.fontSize}|${s.lineHeight}|${s.letterSpacing}|${s.tabSize}`,
        pad: `${s.paddingTop}|${s.paddingRight}|${s.paddingBottom}|${s.paddingLeft}`,
        ws: s.whiteSpace,
      };
    };
    return {
      dx: px(t.left - p.left),
      dy: px(t.top - p.top),
      dw: px(t.width - p.width),
      dh: px(t.height - p.height),
      ta: of(ta),
      pre: of(pre),
    };
  });
  if (sheet === null) {
    fail(
      "probe dead: the open file grew no editor — either the fixture stopped being savable " +
        "(**/read_text must answer a digest) or the surface is read-only again (src/views/FileView.tsx)",
    );
  } else {
    console.log(
      `caret on ink: box off by ${sheet.dx}/${sheet.dy} at the corner, ${sheet.dw}/${sheet.dh} in size; ` +
        `font ${sheet.ta.font === sheet.pre.font ? "same" : "DIFFERENT"}, ` +
        `padding ${sheet.ta.pad === sheet.pre.pad ? "same" : "DIFFERENT"}`,
    );
    for (const [what, a, b] of [
      ["the type", sheet.ta.font, sheet.pre.font],
      ["the padding", sheet.ta.pad, sheet.pre.pad],
      ["white-space", sheet.ta.ws, sheet.pre.ws],
    ]) {
      if (a !== b) {
        fail(
          `the editor and the painted copy disagree on ${what}: ${a} against ${b} — the caret is ` +
            "laid out by one and the letters by the other (src/views/FileView.tsx)",
        );
      }
    }
    // Sub-pixel is text metrics; a drift worth catching is a whole
    // border or a padding step.
    for (const [axis, n] of [["x", sheet.dx], ["y", sheet.dy], ["width", sheet.dw], ["height", sheet.dh]]) {
      if (Math.abs(n) > 0.5) {
        fail(`the editor's box is ${n} off the painted copy's on ${axis} (src/views/FileView.tsx)`);
      }
    }
  }

  // ── 4d) the editor switch, and the card in front of it ────────────
  //
  // **Two things, and the second is why the first is a gate at all.**
  // The switch appears only where there is an editor to open — a
  // machine without one, and a machine that could not be asked, both
  // answer empty and both must get nothing, because a button that
  // fails is worse than no button. And pressing it does not launch:
  // the first press on a machine raises a card, because what happens
  // next is a 640 MB download onto somebody else's disk and their
  // acceptance of Microsoft's licence (「第一个,第一次是允许的.联网也要
  // 说一下」— the user, 2026-09-10).
  await atCrumbs();
  const withCode = await page.evaluate(() => {
    const b = document.querySelector("[data-code]");
    return b ? { on: b.dataset.code, word: b.textContent.trim() } : null;
  });
  if (withCode === null) {
    fail(
      "a machine that answered `code --version` drew no editor switch — the workspace head is where " +
        "it belongs (src/views/WorkspacePane.tsx)",
    );
  } else {
    await press("[data-code]");
    const card = await page.evaluate(() => {
      const c = document.querySelector("[data-code-consent]");
      return c
        ? {
            lines: c.innerText.trim().split("\n").filter(Boolean).length,
            go: !!document.querySelector("[data-code-go]"),
            no: !!document.querySelector("[data-code-no]"),
          }
        : null;
    });
    console.log(
      `editor switch: says ${JSON.stringify(withCode.word)} while ${withCode.on}; ` +
        `pressing it raises ${card ? `a card of ${card.lines} lines with both answers: ${card.go && card.no}` : "nothing"}`,
    );
    if (card === null) {
      fail(
        "pressing the editor switch started something instead of asking — the first press on a machine " +
          "downloads 640 MB onto it and accepts a licence on its behalf (src/views/WorkspacePane.tsx)",
      );
    } else if (!card.go || !card.no) {
      fail("the consent card does not offer both answers — a card with one way out is not a question");
    }
    if (card !== null) await press("[data-code-no]");
  }

  // …and the branch where the card would be a lie. A machine that
  // already has `serve-web`'s server has nothing to download and
  // accepted the licence when those bytes landed, so the card's own
  // sentence (「第一次要…约 640 MB」) is false there. It was shown
  // anyway until 2026-09-13, because the face consulted only its own
  // storage — a fresh profile asked in front of a server that had been
  // on disk for two days.
  await page.unroute("**/code_here");
  await page.route("**/code_here", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ version: "1.137.0", ready: true }),
    }),
  );
  await press(rail("sessions"));
  await atCrumbs();
  const readySwitch = await page.locator("[data-code]").count();
  if (readySwitch !== 1) {
    fail(
      `a machine with the server already unpacked drew ${readySwitch} editor switch(es) — having the ` +
        "server is a reason to skip the card, never a reason to hide the switch " +
        "(src/views/WorkspacePane.tsx)",
    );
  } else {
    await press("[data-code]");
    const asked = await page.locator("[data-code-consent]").count();
    console.log(`editor switch: a machine that already has the server raises ${asked} card(s)`);
    if (asked !== 0) {
      fail(
        "the consent card came up for a machine that already has the server — its sentence says 640 MB " +
          "is about to come down, and nothing is (src/views/WorkspacePane.tsx)",
      );
    }
  }
  // …and the other branch: a machine with no editor draws no switch.
  // **Measured rather than reasoned**, because「draws nothing」is what
  // a broken selector also looks like — which is why the positive half
  // above runs first and on the same element.
  await page.unroute("**/code_here");
  await page.route("**/code_here", (r) =>
    r.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ version: "", ready: false }),
    }),
  );
  await press(rail("sessions"));
  await atCrumbs();
  const withoutCode = await page.locator("[data-code]").count();
  console.log(`editor switch: a machine with no \`code\` draws ${withoutCode}`);
  if (withoutCode !== 0) {
    fail(
      `a machine that answered no editor still drew ${withoutCode} switch(es) — pressing one would ` +
        "fail, and offering nothing is the whole point (src/views/WorkspacePane.tsx)",
    );
  }
  await page.unroute("**/code_here");
  await page.route("**/code_here", (r) =>
    r.fulfill({
      contentType: "application/json",
      // **`ready: false` is the fixture's whole point here**: it is the
      // machine that has not fetched the server, which is the only one
      // the consent card is for (`khor_gui_core::code::Editor`).
      body: JSON.stringify({ version: "1.137.0", ready: false }),
    }),
  );

  // ── 4e) the two screens that are not a code view ──────────────────
  //
  // **A picture is drawn, a document is prose, and neither is text in a
  // `<pre>`.** The failure this catches is the quiet one: a `.png` sent
  // down the text road comes back as「这不是文本」or as a pane of
  // mojibake, and both look like a decision rather than a bug. The
  // tables that decide the road are compared directly by
  // `apps/gui/core/tests/kind_tables.rs`; this measures that the road
  // arrives somewhere.
  await atCrumbs();
  await press('[data-file="shot.png"] button');
  const pic = await page.evaluate(async () => {
    const img = document.querySelector('[data-look="image"]');
    if (!img) return null;
    // **Waited for, not assumed.** A `data:` URL still decodes
    // asynchronously, and `naturalWidth` is 0 both before it has and
    // forever after a broken one — so a test that read it straight
    // away would pass on a broken image half the time.
    if (!img.complete) await img.decode().catch(() => {});
    return { w: img.naturalWidth, h: img.naturalHeight, src: img.src.slice(0, 24) };
  });
  console.log(`picture: ${pic ? `${pic.w}x${pic.h} from ${pic.src}…` : "nothing drawn"}`);
  if (pic === null) {
    fail(
      "a .png opened as something other than a picture — the file view's kind table and the node's " +
        "have drifted, or the image branch is gone (src/views/FileView.tsx)",
    );
  } else if (pic.w === 0 || pic.h === 0) {
    fail(
      `the picture drew ${pic.w}x${pic.h} — the bytes reached an <img> and it could not decode them, ` +
        "which is a broken-image glyph wearing the right attribute",
    );
  }

  await atCrumbs();
  await press('[data-file="notes.md"] button');
  const doc = await page.evaluate(() => {
    const el = document.querySelector('[data-look="doc"]');
    if (!el) return null;
    const h = el.querySelector("h1");
    const box = (n) => (n ? getComputedStyle(n) : null);
    return {
      headings: el.querySelectorAll("h1,h2,h3").length,
      tables: el.querySelectorAll("table").length,
      pre: el.closest("section,div")?.querySelectorAll("pre").length ?? 0,
      h1Size: h ? parseFloat(box(h).fontSize) : 0,
      bodySize: parseFloat(box(el).fontSize),
    };
  });
  console.log(
    `document: ${doc ? `${doc.headings} heading(s), ${doc.tables} table(s), h1 ${doc.h1Size} over body ${doc.bodySize}` : "nothing drawn"}`,
  );
  if (doc === null) {
    fail("a .md opened as something other than a document (src/views/FileView.tsx)");
  } else {
    if (doc.headings === 0 || doc.tables === 0) {
      fail(
        `the document drew ${doc.headings} headings and ${doc.tables} tables — the node's HTML ` +
          "arrived and nothing rendered it as structure (src/styles/app.css `.prose-khor`)",
      );
    }
    // **Prose, not code.** The whole point of this road is that a
    // document is read, and a heading the same size as its body is a
    // wall of text with markup in it.
    if (doc.h1Size <= doc.bodySize) {
      fail(
        `a heading measured ${doc.h1Size} against body text at ${doc.bodySize} — the document is ` +
          "rendered but not styled, which reads as markdown that failed to render",
      );
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
  //    有token usage的环」 took them off; the 额度 landing is their one
  //    home. **Section 8 below proves the finder**: the same
  //    `[data-quota-window]` selector counts this fixture's windows
  //    there in this very run, so a zero here is an absence and not a
  //    renamed marker
  //    (the `.nav-badge` family's antidote). The `**/quota` route at
  //    the head of this file stays — it feeds the subscription row
  //    section 8 measures.
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

  //   ── the dials: the box follows the ink, and the air is a stated
  //      number ────────────────────────────────────────────────────
  //
  //   2026-09-08, the user: 「机器页的环和下方的标题艰巨太远了.环的间距
  //   也太远了.明明可以把环绘制的更大,页面更紧凑,没有充分利用空间.」
  //
  //   **Both distances are measured off the arc's own ink, never off a
  //   box.** A dial's box is a claim about the drawing; the `path` is
  //   the drawing. Measured that day before the fix: the box was 88
  //   tall while the arc's ink stopped at 60.6 (it sweeps to 15°, not
  //   to the bottom of its circle), so 27.4px of empty box sat between
  //   the arc and the word under it — the reader saw 35.4 where 8 was
  //   intended.
  {
    const dials = await page.evaluate(() => {
      const vit = document.querySelector("[data-machine-card] [data-vitals]");
      if (!vit) return null;
      const cols = [...vit.children];
      const read = (col) => {
        const btn = col.querySelector("[data-metric]");
        const paths = [...col.querySelectorAll("[data-metric] path")];
        if (!btn || !paths.length) return null;
        // The union of the arcs' ink — the track and the fill are two
        // paths on one circle, and a pip is a path too.
        const ink = paths.map((p) => p.getBoundingClientRect());
        const box = btn.getBoundingClientRect();
        const word = [...col.children].filter((n) => !n.hasAttribute("data-metric")).pop();
        return {
          w: +(Math.max(...ink.map((b) => b.right)) - Math.min(...ink.map((b) => b.left))).toFixed(1),
          boxL: +box.left.toFixed(1),
          boxR: +box.right.toFixed(1),
          // The empty box above the ink and below it. `crop` cuts to
          // `reach`, so the two are related by the sweep's own angle —
          // see the assertion.
          // The box's own height and the arc's own radius — neither
          // moves when a pip is drawn, which is why the expectation
          // below is built out of these two and not out of「how much
          // box sits above the ink」(a pip reaches higher than the arc,
          // so that distance is smaller on the dial that has one).
          boxH: +box.height.toFixed(2),
          // **The rim's own radius, off the fill arc** — not the max
          // over every path: a pip and the milestone span are arcs too,
          // at radii of their own, and `Math.max` picked one of those
          // (56 where the rim is 45) on the two dials that draw them.
          r: (() => {
            const fill = col.querySelector("[data-metric] [data-ring-fill]");
            const m = fill && /A\s*([\d.]+)\s+([\d.]+)/.exec(fill.getAttribute("d") || "");
            return m ? Number(m[1]) : null;
          })(),
          stroke: parseFloat(getComputedStyle(col.querySelector("[data-metric] path")).strokeWidth),
          gapIn: parseFloat(getComputedStyle(col).rowGap),
          toWord: word ? +(word.getBoundingClientRect().top - Math.max(...ink.map((b) => b.bottom))).toFixed(1) : null,
        };
      };
      const got = cols.map(read).filter(Boolean);
      const gaps = [];
      for (let i = 1; i < got.length; i += 1) gaps.push(+(got[i].boxL - got[i - 1].boxR).toFixed(1));
      const stated = parseFloat(getComputedStyle(vit).columnGap);
      return { n: got.length, arcW: got.map((g) => g.w), toWord: got.map((g) => g.toWord), gaps, stated, boxH: got.map((g) => g.boxH), r: got.map((g) => g.r), stroke: got[0].stroke, gapIn: got[0].gapIn };
    });
    if (!dials || dials.n < 2) {
      fail(`probe dead: ${dials ? dials.n : "no"} dials with ink, so neither distance was measured`);
    } else if (!dials.arcW.every((w) => w > 10)) {
      // **The arm this repo keeps meeting**: two gaps between two arcs
      // that were never drawn agree perfectly. The ink has to be there
      // before a distance between two of them means anything.
      fail(`the dials' arcs measure ${JSON.stringify(dials.arcW)} across — a gap between drawings that are not there is not a gap`);
    } else {
      // **The word sits under the *drawing's box*, and that box ends a
      // little below the arc on purpose.**
      //
      // ⚠️ This assertion was written at 12 and the product measured
      // 13.1 — and the product was right. `crop` cuts to `reachOf`,
      // which is the rim **plus a pip's whole length plus half its cap**:
      // a pip may sit outside the arc, so a box cut to the arc would
      // clip one. Measured 2026-09-08: 5.0px of that allowance sits
      // under the ink, and `--gap-in` (8) is the rest. **Before the
      // crop the same distance was 35.4** — that is what was fixed.
      // ⚠️ **Derived, not a number.** This assertion read `<= 14` until
      // 2026-09-08 — measured at size 88, and it went red the moment the
      // dial grew (14.3 at 132) while nothing was wrong. **A threshold
      // that only holds at one size is a constant only its author can
      // check.**
      //
      // What holds at every size is the crop's own geometry: it cuts to
      // `reach`, so the empty box **above** the ink is `reach − R` and
      // the empty box **below** it is that same slack seen at the arc's
      // closing angle, plus half a stroke. Both are measured here, so
      // the expectation is computed from the drawing rather than
      // recalled from a run.
      // `crop` makes the box `reach·(1+sin15°) + stroke/2` tall and
      // starts it `reach` above the centre, so the empty box under the
      // ink is `sin15°·(reach − R) + stroke/2`. **`reach` comes back
      // out of the box's own height** and `R` off the track's `A r r`
      // — two readings a pip cannot move.
      const CLOSE = Math.sin((15 * Math.PI) / 180);
      if (dials.r.some((v) => v === null)) fail(`probe dead: a dial has no [data-ring-fill] to read its rim radius from (${JSON.stringify(dials.r)})`);
      const wants = dials.boxH.map((h, i) => {
        const reach = (h - dials.stroke / 2) / (1 + CLOSE);
        return dials.gapIn + CLOSE * (reach - dials.r[i]) + dials.stroke / 2;
      });
      const wrong = dials.toWord.map((v, i) => v === null || Math.abs(v - wants[i]) > 1);
      if (wrong.some(Boolean)) {
        fail(
          `a dial's arc ends ${JSON.stringify(dials.toWord)} above its own word, and the crop's own geometry says ` +
            `${JSON.stringify(wants.map((w) => +w.toFixed(2)))} (--gap-in ${dials.gapIn} + the pip room the crop keeps, ` +
            `derived from boxes ${JSON.stringify(dials.boxH)} tall over arcs of radius ${JSON.stringify(dials.r)}) — anything more is empty box, which is what ` +
            `「环和下方的标题太远」 was.`,
        );
      }
      // ⚠️ **Tombstone: 「the air between two dials is a stated number」
      // was asserted here, and the user overruled it on 2026-09-09.**
      // It is not a claim that failed — it is a claim that was decided
      // against, and the two rulings read as contradictions unless both
      // are written down:
      //
      //   2026-09-08 「环的间距也太远了」 → the gap stopped being a
      //   remainder (`justify-around` spread 426.8 of a 990 row as air)
      //   and became `--gap-group`, a number the stylesheet states.
      //
      //   2026-09-09 「环之间的间距,排布,平铺屏幕宽度」 → the dials
      //   are the 額度 card's size, the outer two sit flush with the
      //   row's edges, and **what is left over goes between them** —
      //   which makes the gap a remainder again, on purpose. **The user
      //   was shown the cost first** (the gap reaching 151 at the wide
      //   posture, against the 18 he had just asked for) and chose this
      //   knowing it. So 「the gap grew」 is not a regression here, and
      //   nobody should quote the 09-08 sentence to undo it.
      //
      // What survives is the half that was never about the number:
      // **the gaps are equal to each other**. A row that let one gap
      // differ from the next is the one thing neither ruling wanted.
      //
      // ⚠️ **Measured between the boxes, not between the ink.** The
      // arcs come out 115.2 / 142.9 / 115.2 / 129.2 wide because a dial
      // with pips draws further out than one without; ink-to-ink varies
      // by design, and asserting it equal would be asserting that no
      // dial has pips.
      const spread = Math.max(...dials.gaps) - Math.min(...dials.gaps);
      if (dials.gaps.length > 0 && spread > 0.6) {
        fail(
          `the dials sit ${JSON.stringify(dials.gaps)} apart — the spare width is divided between them, so the gaps are ` +
            `equal to each other however wide the row is; ${spread.toFixed(1)} of difference means something else is ` +
            `placing them`,
        );
      }
      if (dials.gaps.some((g) => g + 0.6 < dials.stated)) {
        fail(
          `the dials sit ${JSON.stringify(dials.gaps)} apart while ${dials.stated} is the floor — the spare width is added ` +
            `to that floor, never taken out of it`,
        );
      }
      //   **The reading in the middle fits inside its own ring.**
      //
      //   The widest a dial can say is four characters (`100%`, and a
      //   busy box says `240%`) while this fixture only ever shows
      //   three — so the string is put into the product's own `<text>`,
      //   measured there, and put back. **Measured in the element that
      //   draws it**, never in a copy: `getComputedStyle(node).font` is
      //   empty on an SVG text, and a copy measured with the fallback
      //   font answered 29.73 for a string the real element drew at
      //   44.72. That first attempt was caught by its own restore check.
      //
      //   ⚠️ **The inner diameter comes off the rim's `A r r`, never off
      //   a box.** Measured 2026-09-08: at the old size 88 the inner
      //   diameter was 56 and `100%` drew 57.25 — **it did not fit**,
      //   silently, on any machine that reached three digits.
      const fit = await page.evaluate(() => {
        const btn = document.querySelector("[data-machine-card] [data-metric]");
        const node = btn && [...btn.querySelectorAll("*")].filter((n) => n.children.length === 0 && /^\d+%$/.test((n.textContent || "").trim()))[0];
        const fill = btn && btn.querySelector("[data-ring-fill]");
        const m = fill && /A\s*([\d.]+)\s+([\d.]+)/.exec(fill.getAttribute("d") || "");
        if (!node || !m) return null;
        const stroke = parseFloat(getComputedStyle(fill).strokeWidth);
        const was = node.textContent;
        const before = +node.getBoundingClientRect().width.toFixed(2);
        node.textContent = "100%";
        const widest = +node.getBoundingClientRect().width.toFixed(2);
        node.textContent = was;
        return { inner: +(2 * Number(m[1]) - stroke).toFixed(2), widest, restored: +node.getBoundingClientRect().width.toFixed(2) === before };
      });
      if (!fit) {
        fail("probe dead: no dial reading or no rim arc, so 「the number fits its ring」 measured nothing");
      } else if (!fit.restored) {
        // The arm: if putting the string back did not put the width
        // back, the two readings came from two different states.
        fail("probe void: the dial's reading did not return to its own width after the measurement — the two readings are not one state");
      } else if (fit.widest > fit.inner) {
        fail(`100%@card = ${fit.widest} vs inner@card = ${fit.inner} — the widest reading a dial can say does not fit inside its own ring`);
      }
      console.log(`dials: arcs ${JSON.stringify(dials.arcW)} across, ${JSON.stringify(dials.toWord)} above their words, gaps ${JSON.stringify(dials.gaps)}, 100%@card ${fit ? fit.widest : "?"} vs inner ${fit ? fit.inner : "?"}`);
    }
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
  // **`board.length + 1`, and the `+ 1` is the subscription.** The
  // `**/quota` route above answers two windows, so this fixture has a
  // subscription to list, and 2026-09-06 folded it out of the foot's
  // band and into the list (「不要有现在下方单独一个订阅的横条」). The
  // number is spelled as the fixture plus that one row rather than as
  // `4`, so a fixture that stops answering a quota changes this count
  // by itself instead of leaving a constant behind that used to mean
  // something.
  const listed = board.length + 1;
  const quotaRows = await page.locator("[data-quota-row]").count();
  if (quotaRows !== listed) {
    fail(`\u989d\u5ea6 draws ${quotaRows} rows for ${board.length} routed accounts and one subscription`);
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
  // **The subscription's windows, now in its row's detail** — the
  // positive half section 7's absences lean on: same selector, same
  // run, moved with the band that carried it.
  //
  // ⚠️ **Both halves, because either alone is satisfied by the wrong
  // thing.** 2026-09-06 folded the foot's band into the list
  // (「不要有现在下方单独一个订阅的横条」): a gate that only counted
  // windows would pass with the band still there, and one that only
  // checked the band was gone would pass on a page that lost the
  // subscription altogether. So: the band is nowhere, **and** the
  // subscription is a row, **and** opening it draws its two windows.
  const band = await page.locator("[data-quota-sub]").count();
  if (band !== 0) {
    fail(`the \u989d\u5ea6 foot still draws a subscription band (${band}) — it is a row in the list now`);
  }
  const subRow = page.locator('[data-quota-row][data-quota-kind="sub"]').first();
  const subRows = await page.locator('[data-quota-row][data-quota-kind="sub"]').count();
  if (subRows !== 1) {
    fail(`the \u989d\u5ea6 list has ${subRows} subscription rows for a routed quota, not one`);
  } else {
    await subRow.click();
    await wait(250);
    const shownWindows = await page.locator("[data-quota-detail] [data-quota-window]").evaluateAll((els) =>
      els.map((e) => e.getAttribute("data-quota-window")),
    );
    if (shownWindows.length !== quotaWindows.length) {
      fail(
        `the subscription's detail draws ${JSON.stringify(shownWindows)} for the fixture's ${quotaWindows.length} windows`,
      );
    }
    //   **One pip per part, one for one** — the counting rule that
    //   replaced the concentric ring on 2026-09-07
    //   (「轮缘卡点数 = 展开后的表盘数」). The rim is read off the
    //   drawing (`data-dial-pip`) and the list off the panel
    //   (`data-quota-part`), so a build that draws a mark for something
    //   it does not list, or lists something it does not mark, is the
    //   one thing this cannot pass.
    //
    //   ⚠️ **Rewritten from「two layers, and a scoped window is not a
    //   layer」** — the rule `727d030` brought back and 2026-09-07
    //   overturned:「你绘制的只有两个外环线段。但它有四个环。如果是两两
    //   重叠就没关系」. **The product changed, not the assertion.** The
    //   old shape stacked arcs at different radii, where a week's Fable
    //   share nested inside the week it is a share of made one radius
    //   mean two things; on a rim there are no radii, only angles, and
    //   two windows at the same percent are two marks on one spot.
    //
    //   **The scoped window is what makes this falsifiable at all**: it
    //   is exactly the mark the previous rule dropped, so a build still
    //   filtering by `scope` draws two pips here and goes red — which
    //   is the difference this gate is built around rather than a count
    //   that any implementation with the right total would satisfy.
    //
    //   ⚠️ **And the third term is not padding.** The pips and the
    //   parts come off one list in the product, so `pips === parts`
    //   holds however wrong that list is — a filter reinstated upstream
    //   drops a mark *and* a line together and the two still agree.
    //   What catches it is comparing both against the **fixture**.
    //   Measured 2026-09-07 by putting the old `scope === null` filter
    //   back: the red reads「2 pips for 2 parts and 3 routed windows」,
    //   which is the third term firing and the first two staying quiet.
    const rim = await page.evaluate(() => {
      const row = document.querySelector('[data-quota-row][data-quota-kind="sub"]');
      const detail = document.querySelector("[data-quota-detail]");
      return {
        pips: [...(row?.querySelectorAll("[data-dial-pip]") ?? [])].map((e) => ({
          word: e.getAttribute("data-dial-pip"),
          at: Number(e.getAttribute("data-dial-pip-at")),
        })),
        parts: [...(detail?.querySelectorAll("[data-quota-part]") ?? [])].map((e) =>
          e.getAttribute("data-quota-part"),
        ),
        // The hairline from the lowest mark to the highest: 「spread out
        // or not」 with a length, drawn only where there are two marks
        // to span between.
        span: row?.querySelectorAll("[data-dial-span]").length ?? 0,
      };
    });
    const scopedWindow = quotaWindows.find((w) => w.scope !== null);
    if (rim.pips.length !== rim.parts.length || rim.pips.length !== quotaWindows.length) {
      fail(
        `the subscription's rim carries ${rim.pips.length} pips for ${rim.parts.length} parts and ${quotaWindows.length} routed windows — one pip per part, one for one`,
      );
    } else if (!rim.pips.some((p) => p.word.includes(scopedWindow.scope))) {
      fail(
        `the rim's pips are ${JSON.stringify(rim.pips.map((p) => p.word))} — the window limited to ${JSON.stringify(scopedWindow.scope)} has no mark, which is the filter 2026-09-07 removed`,
      );
    } else if (rim.span !== 1) {
      fail(`three marks and ${rim.span} spread arcs — the hairline says how far apart they are`);
    }
    //   …and every mark sits at its own window's number, so a rim that
    //   drew the right *count* at the wrong angles is not a pass.
    for (const w of quotaWindows) {
      const word = w.scope ?? String(w.span);
      const mark = rim.pips.find((p) => (w.scope ? p.word.includes(w.scope) : p.at === w.used_pct));
      if (!mark || mark.at !== w.used_pct) {
        fail(
          `the window ${JSON.stringify(word)} is ${w.used_pct}% and its mark reads ${JSON.stringify(mark?.at ?? null)}`,
        );
      }
    }
    //   **And the model's name is the payload's, not the catalog's.**
    //   The fixture names a model `zh.toml` has never heard of, so a
    //   build that spelled window names itself draws something else
    //   here — which is the whole bug of 2026-09-06, in one string.
    const scoped = quotaWindows.find((w) => w.scope !== null);
    const scopedSaid = await page
      .locator(`[data-quota-detail] [data-quota-window="seven_day:${scoped.scope}"] [data-quota-window-name]`)
      .textContent()
      .catch(() => null);
    //   **Both halves.** 「名字从 payload 来」 forbids khor spelling the
    //   model; it does not license dropping the length, and 「Fable」
    //   with no period beside it leaves a reader 35% through nothing in
    //   particular. So: the endpoint's word **and** the catalog's.
    //   The unscoped week's own *name* — read off the label rather
    //   than the block, which also holds the ring's percentage and the
    //   reset line (measuring the block and splitting on whitespace
    //   reads 「39%7」, which is how this was first written).
    const spanWord = (
      await page.locator('[data-quota-window="seven_day"] [data-quota-window-name]').textContent()
    )?.trim() ?? "";
    if (!scopedSaid?.includes(scoped.scope)) {
      fail(
        `the per-model window does not wear the endpoint's own name ${JSON.stringify(scoped.scope)} — got ${JSON.stringify(scopedSaid)}`,
      );
    } else if (!spanWord || !scopedSaid.includes(spanWord)) {
      fail(
        `the per-model window says ${JSON.stringify(scopedSaid)} without the length ${JSON.stringify(spanWord)} the unscoped week wears`,
      );
    }

    //   **The plan says itself once per surface.** On the row it is
    //   the sub-line; on the card's head it is a chip beside the title.
    //
    //   ⚠️ **Rewritten 2026-09-07, and the product is what changed.**
    //   This used to require a chip **on the row** as well
    //   (`f9e2062`, one day old) while the sub-line said 类型 · 提供商 ·
    //   来源. The ruling cut that line to one word — and for a
    //   subscription that word *is* the plan, so the chip and the line
    //   had become two spellings of one fact on a row with room for
    //   one. The chip stayed where the name is a title.
    //
    //   **Equality, not `includes`**: the whole point of the ruling is
    //   what is *not* on the line, and a substring check passes on the
    //   three-segment line it replaced.
    const rowPlan = await page.evaluate(() => {
      const row = document.querySelector('[data-quota-row][data-quota-kind="sub"]');
      const line = row?.querySelector("[data-quota-sub-line]");
      return {
        // **An absence, with the live control beside it**: the same
        // selector finds the head's chip in this same run, so a zero
        // here is「the row has none」and not「the marker was renamed」.
        chipOnRow: row?.querySelectorAll("[data-quota-plan]").length ?? null,
        line: line?.textContent?.trim() ?? null,
        // The mark of a truncated line: more text than there is room
        // for. One pixel of slack for sub-pixel layout. `textContent`
        // cannot see an ellipsis — the browser truncates in layout.
        cut: line ? line.scrollWidth > line.clientWidth + 1 : null,
      };
    });
    const headChips = await page.locator("[data-quota-detail] [data-quota-plan]").count();
    const headChip = (await page.locator("[data-quota-detail] [data-quota-plan]").textContent().catch(() => null))?.trim();
    const planOnDetail = await page.locator("[data-quota-detail] [data-quota-credits]").count();
    if (headChips !== 1 || headChip !== quotaPlan) {
      fail(
        `the card head draws ${headChips} plan chips reading ${JSON.stringify(headChip)}, not one saying ${JSON.stringify(quotaPlan)}`,
      );
    } else if (rowPlan.chipOnRow !== 0) {
      fail(
        `the row still draws ${rowPlan.chipOnRow} plan chip(s) — the sub-line is the plan now, and two spellings of one fact is what 2026-09-07 removed`,
      );
    }
    //   …and the sub-line is **类型 · 提供商, the same pair on every
    //   row** (2026-09-07:「标题是用户可以自己起的。我们要有个统一的类型
    //   模型,订阅/计费、厂商。写在副标题」).
    //
    //   ⚠️ **Rewritten from 「one word: the plan, or the provider」 —
    //   the product changed.** That version had a subscription wearing
    //   its plan and an account wearing its provider: two kinds of row
    //   speaking two different languages, which is what a unified model
    //   rules out. The name above is free text somebody typed, so it
    //   classifies nothing and this line carries the whole of what a
    //   row *is*.
    //
    //   **Both rows, the same shape on each** — a check on the
    //   subscription alone would pass a build that still special-cased
    //   accounts.
    const billLine = (await page.locator('[data-quota-row="acc-fill"] [data-quota-sub-line]').textContent())?.trim();
    const twoParts = (line) => (line ?? "").split("·").map((x) => x.trim()).filter(Boolean);
    if (twoParts(rowPlan.line).length !== 2 || twoParts(billLine).length !== 2) {
      fail(
        `the sub-lines are ${JSON.stringify([rowPlan.line, billLine])} — 类型 · 提供商 on every row, whichever kind it is`,
      );
    } else if (rowPlan.line?.includes(quotaPlan)) {
      fail(
        `the subscription's sub-line still carries its plan (${JSON.stringify(rowPlan.line)}) — the plan is a product name, and this line says what kind of thing the row is`,
      );
    } else if (rowPlan.cut !== false) {
      fail(
        `at this column's width the sub-line is still cut (${JSON.stringify(rowPlan.line)}) — one word was supposed to end that`,
      );
    }

    //   **The credits: shown, and not shown as a window.** Three
    //   claims, and the third is the one a careless build fails:
    //   `balance` and `monthly_limit` are null in this fixture and must
    //   draw *no row at all* — the wrong implementation draws an empty
    //   one, which reads as「your balance is nothing」.
    if (planOnDetail !== 1) {
      fail(`probe dead: the subscription detail draws ${planOnDetail} 信用额 blocks, so nothing below is measuring one`);
    } else {
      const credits = await page.locator("[data-quota-detail] [data-quota-credits]").evaluate((dl) => ({
        text: dl.textContent ?? "",
        pairs: dl.querySelectorAll("dt").length,
        used: dl.querySelector("[data-quota-credits-used]")?.textContent ?? null,
        balance: dl.querySelector("[data-quota-credits-balance]") !== null,
        on: dl.querySelector("[data-quota-credits-on]")?.getAttribute("data-quota-credits-on") ?? null,
      }));
      if (!credits.used?.includes("9.21") || credits.on !== "false") {
        fail(`the 信用额 block says used=${JSON.stringify(credits.used)} on=${JSON.stringify(credits.on)} for a spent-out fixture`);
      }
      if (credits.balance || credits.pairs !== 2) {
        fail(
          `the 信用额 block drew ${credits.pairs} rows and balance=${credits.balance} — the fixture reports no balance and no monthly limit, and a row for a figure nobody stated reads as a zero`,
        );
      }
      //   …and it is not on the ring: the layers were already checked
      //   above against the *windows*, so a credits figure that leaked
      //   into `ringWindows` would have shown up there. The one thing
      //   left to say is that the row's summary is a window, not money.
      const summary = await page.locator('[data-quota-row][data-quota-kind="sub"]').textContent();
      if (summary?.includes("9.21")) {
        fail(`the subscription row's summary names the credits (${JSON.stringify(summary)}) — money is not a window`);
      }
    }

    //   **A ring with no ceiling is a full ring** (2026-09-06 user
    //   ruling). Still read off the arc itself rather than a class that
    //   might mean it — but the arc is a 210° `path` now, not a dashed
    //   circle, so the reading is **the angle its last point sits at**:
    //   the dial opens at −195° and sweeps 210°, so where the arc stops
    //   *is* the fraction. `stroke-dasharray` reads `null` on a path
    //   and this gate would have gone quietly green-on-null for every
    //   row — **the assertion did not change, the geometry under it
    //   did** (2026-09-07:「环全部和机器卡对齐」).
    //
    //   Three rows, three answers, and **all three are needed**: an
    //   implementation that draws everything full passes on `acc-number`
    //   alone, and one that draws nothing full passes on `acc-trouble`
    //   alone. The fixture has a fraction (0.45), a reading with no
    //   denominator (a spend, no limit) and a refusal, so each wrong
    //   rule is caught by one of the other two.
    const arcs = await page.locator("[data-quota-row]").evaluateAll((rows) =>
      rows.map((row) => {
        const id = row.getAttribute("data-quota-row");
        const svg = row.querySelector("[data-dial-face]");
        const arc = svg?.querySelector("[data-ring-fill]");
        // The track is always drawn, so its absence means the dial
        // itself never rendered — a different failure from「no arc」and
        // it must not read as one.
        if (!svg || svg.querySelectorAll("path").length === 0) return { id, drawn: "no dial" };
        if (!arc) return { id, drawn: null };
        // **The centre is read off the track that was drawn, not off
        // the box.**
        //
        // ⚠️ It used to be `[vb[2] / 2, vb[3] * 0.6]`, and the comment
        // above it said the centre was 「never assumed」 — **while
        // assuming `cy = 0.6 × the box's height`, which held only
        // while the box's height was `size`.** Cropping the box
        // vertically (2026-09-07) made that false and this gate read
        // 1.13 and 0.37 for arcs that were perfectly correct: a
        // plausible wrong number, which is the failure the old comment
        // was written about, arriving through the half it did not
        // cover.
        //
        // The track is one arc of known radius starting at `ARC0`
        // (−195°), so its own `d` gives the centre with nothing
        // assumed about the box at all: `M x0 y0 A r r …`, and
        // `centre = start − r·(cos ARC0, sin ARC0)`.
        const track = svg.querySelector("path");
        const td = /M([-\d.]+)\s+([-\d.]+)\s*A([-\d.]+)/.exec(track?.getAttribute("d") ?? "");
        if (!td) return { id, drawn: "no track" };
        const [x0, y0, r] = [Number(td[1]), Number(td[2]), Number(td[3])];
        const A0 = (-195 * Math.PI) / 180;
        const [cx, cy] = [x0 - r * Math.cos(A0), y0 - r * Math.sin(A0)];
        const n = (arc.getAttribute("d") ?? "").replace(/,/g, " ").trim().split(/\s+/).map(Number);
        const [x, y] = [n[n.length - 2], n[n.length - 1]];
        let deg = (Math.atan2(y - cy, x - cx) * 180) / Math.PI;
        while (deg < -195) deg += 360;
        while (deg > 165) deg -= 360;
        return { id, drawn: Number(((deg + 195) / 210).toFixed(2)) };
      }),
    );
    const arcOf = (id) => arcs.find((a) => a.id === id)?.drawn ?? null;
    if (arcOf("acc-fill") !== 0.45 || arcOf("acc-number") !== 1 || arcOf("acc-trouble") !== null) {
      fail(
        `the arcs read ${JSON.stringify(arcs)} — a fraction draws itself (0.45), a reading with no ceiling draws a whole ring (1), and only 问不出来 leaves bare track (null)`,
      );
    }

    //   ── 8·a truncated log says so, beside the chart it truncates ──
    //   ⚠️ **Tombstone: this section used to measure the by-day *money*
    //   chart** — its bar count, its title, the zone its dates were cut
    //   in, its tick seat against the token chart's. That chart left
    //   the card on 2026-09-10 (「取消现在那个多出的最近1天那的bar图.
    //   就绘制花掉的 token」) and `accounts-trend-days` went with it.
    //
    //   **What could not leave is the cut-log line.** `days` and
    //   `model_tokens` come out of one `/api/log/token` answer
    //   (`crates/node/src/accounts.rs`), so a provider that truncated
    //   the log truncated the token chart too — and a reader who is not
    //   told sees one busy day and reads it as the window
    //   (「看图的人不能以为自己看的是 7 天」, 2026-09-07).
    const capOf = async (row) => {
      await page.locator(`[data-quota-row="${row}"]`).click();
      await wait(300);
      return await page.evaluate(() => ({
        line: document.querySelector("[data-quota-days-capped]")?.textContent?.trim() ?? null,
        //   The probe before the negative claim: with no token chart on
        //   this card, 「no cap line」 and 「nothing drew」 are one
        //   reading, and the assertion below would pass on an empty
        //   screen.
        chart: !!document.querySelector('[data-usage-zone="trend"] [data-usage-chart]'),
      }));
    };
    const uncut = await capOf("acc-number");
    const trimmed = await capOf("acc-capped");
    if (!uncut.chart || !trimmed.chart) {
      fail(
        `probe dead: the token chart is absent (uncut=${uncut.chart} trimmed=${trimmed.chart}) — ` +
          `「no cap line」 below would be saying 「no card」`,
      );
    } else if (uncut.line !== null) {
      fail(
        `an uncut log says ${JSON.stringify(uncut.line)} — **a disclaimer that is always on screen is read as ` +
          `decoration**, and this one would be false on almost every account`,
      );
    } else if (!trimmed.line?.includes("1000")) {
      fail(
        `a cut log says ${JSON.stringify(trimmed.line)} — it carries the count the provider actually answered, and ` +
          `it sits beside the chart that count truncates`,
      );
    }

    //   ── 8·what a ring may hold ─────────────────────────────────
    //   2026-09-07:「保持在 4 位字符以内吧。比如,百分比最长是 100%。
    //   然后,124 这里就是 124$。」
    //
    //   **Two claims, and they are not the same claim.** The character
    //   count is the *rule* — it does not drift when a face, a size or
    //   a stroke changes. The fit is the *outcome*, and four wide
    //   characters can still overflow: `124$` measures 24.4 against an
    //   inner 24.7, which is 0.3 of room. **Either alone lets the other
    //   one's failure through.**
    //
    //   Every row, because the band that broke was 10–99.99 (`12.34`,
    //   five characters) and it is one account among several — a check
    //   on 「the first row」 would have passed while that one overflowed.
    const middles = await page.evaluate(() => {
      return [...document.querySelectorAll("[data-quota-row]")].map((r) => {
        const svg = r.querySelector("[data-dial-face]");
        const t = svg?.querySelector("text");
        // **The rim's own radius and stroke, off the track that was
        // drawn.**
        //
        // ⚠️ Was `2 * 30 * (vbH / 88) - 2.6` — the reference size and
        // the row's stroke written out again here, with the box's
        // height standing in for `size`. Both halves of that broke on
        // 2026-09-07: the box stopped being `size` tall (it is cropped
        // to the drawing now), and a hard-coded `2.6` cannot describe
        // two dials of different strokes. It reported five middles
        // spilling that were not.
        const track = svg?.querySelector("path");
        const td = /A([-\d.]+)/.exec(track?.getAttribute("d") ?? "");
        const inner = td ? 2 * Number(td[1]) - Number(track.getAttribute("stroke-width") ?? 0) : 0;
        const b = t?.getBBox();
        return {
          id: r.getAttribute("data-quota-row"),
          text: t?.textContent ?? "",
          chars: (t?.textContent ?? "").length,
          over: b ? +(b.width - inner).toFixed(1) : -1,
          w: b ? +b.width.toFixed(1) : -1,
          inner: +inner.toFixed(1),
          box: svg ? `${Math.round(svg.getBoundingClientRect().width)}x${Math.round(svg.getBoundingClientRect().height)}` : "?",
        };
      });
    });
    console.log(
      `ring middles: ${middles.map((m) => `${m.id} "${m.text}" ${m.w}/${m.inner} box ${m.box}`).join(" · ")}`,
    );
    const tooLong = middles.filter((m) => m.chars > 4);
    const spilling = middles.filter((m) => m.over > 0);
    if (middles.length === 0) {
      fail("probe dead: no row drew a dial, so nothing below measures a middle");
    } else if (tooLong.length > 0) {
      fail(
        `a ring's middle reads ${JSON.stringify(tooLong.map((m) => m.text))} — four characters is the rule (「保持在 4 位字符以内」)`,
      );
    } else if (spilling.length > 0) {
      fail(
        `a ring's middle spills out of its ring: ${JSON.stringify(spilling.map((m) => [m.text, m.over]))} — four characters is the rule, fitting is the outcome, and both are owed`,
      );
    }

    //   ── 8·the dial's box holds the drawing, not a margin ───────
    //   2026-09-07:「额度的环…周围都有很多空白,浪费了很多空间,环明明可以
    //   更大一些。这些似乎是没有好的设计原则导致的。」 The principle he
    //   was missing, written down and now checked: **the box holds the
    //   drawing; the room around it belongs to the layout.**
    //
    //   Measured before the change: a 141 × 88 box holding 71 × 48 of
    //   ink — **half of it empty**, and the emptiness was inherited.
    //   `Face`'s default box is 1.6 × its size because the *machine*
    //   card needs room for the sentence on the dial's other face; this
    //   card has no such sentence and took the box anyway.
    //
    //   **Two assertions, and the first does not depend on the
    //   fixture.** How wide the ink runs depends on where the pips
    //   fall, which is data; how wide the *box* is, is a decision. So
    //   the box's own proportions are checked first, and the ink's
    //   share second as the outcome.
    //
    //   ⚠️ **What this is really guarding is the next widening.** The
    //   box grew wide for a reason that was true somewhere else; the
    //   way it happens again is somebody needing to fit one more thing
    //   and reaching for `ratio`. Then the white space is back and no
    //   test says so.
    //   **On the subscription's dial, named rather than whichever card
    //   happened to be open.** Its rim carries pips, so its ink is the
    //   widest a dial here gets — measuring a pip-less account dial
    //   instead reads 0.68 for a box that is perfectly tight, and a
    //   threshold loosened to let that pass would stop catching the
    //   thing this is for. (Found by writing it the other way first.)
    await page.locator('[data-quota-row][data-quota-kind="sub"]').first().click();
    await wait(300);
    const dialBox = await page.evaluate(() => {
      const svg = document.querySelector("[data-quota-lead] [data-dial-face]");
      if (!svg) return null;
      const S = svg.getBoundingClientRect();
      let x0 = 1e9, x1 = -1e9, y0 = 1e9, y1 = -1e9;
      // **The track alone, beside the whole ink.** The track is one
      // arc of fixed radius drawn at fixed angles, so its bounds are
      // pure geometry; every other child sits at an angle that comes
      // from *data* (a pip is drawn at its own reading's angle, and a
      // pip near 50% reaches the very top where one near 4% does not).
      // Reporting both is what tells a later reader whether a shift in
      // these numbers was a change to the drawing or a change to the
      // fixture.
      const t = svg.querySelector("path")?.getBBox();
      for (const c of svg.children) {
        const b = c.getBBox();
        if (b.width === 0 && b.height === 0) continue;
        x0 = Math.min(x0, b.x);
        x1 = Math.max(x1, b.x + b.width);
        y0 = Math.min(y0, b.y);
        y1 = Math.max(y1, b.y + b.height);
      }
      // **Everything below is in the drawing's own units**, not CSS
      // pixels: the box is the `viewBox`, the ink is the children's
      // bounds, and the two are directly comparable there. Mixing the
      // rendered rect in was how the height half stayed invisible —
      // the rect and the `viewBox` height agreed by construction, so
      // dividing one by the other could only ever answer 1.
      const [bx, by, bw, bh] = svg.getAttribute("viewBox").split(/\s+/).map(Number);
      return {
        box: `${+bw.toFixed(1)}x${+bh.toFixed(1)}`,
        ink: `${+(x1 - x0).toFixed(1)}x${+(y1 - y0).toFixed(1)}`,
        wideShare: +((x1 - x0) / bw).toFixed(2),
        tallShare: +((y1 - y0) / bh).toFixed(2),
        boxAspect: +(bw / bh).toFixed(2),
        inkAspect: +((x1 - x0) / (y1 - y0)).toFixed(2),
        // Clipping is silent — an over-tight box draws a dial with a
        // pip sliced off and reports nothing. Positive numbers are
        // room to spare on each side.
        spare: [+(x0 - bx).toFixed(1), +(bx + bw - x1).toFixed(1), +(y0 - by).toFixed(1), +(by + bh - y1).toFixed(1)],
        track: t ? `${+t.width.toFixed(1)}x${+t.height.toFixed(1)}` : null,
        trackAspect: t && t.height ? +(t.width / t.height).toFixed(2) : null,
        trackWide: t ? +(t.width / bw).toFixed(2) : null,
        trackTall: t ? +(t.height / bh).toFixed(2) : null,
      };
    });
    if (!dialBox) {
      fail("probe dead: the card's dial never drew, so nothing below measures a box");
    } else if (dialBox.spare.some((v) => v < 0)) {
      fail(
        `the dial's ink hangs outside its box (spare ${JSON.stringify(dialBox.spare)} on ⊣⊢⊤⊥) — a box tighter than the drawing clips it, and clipping does not raise anything`,
      );
    } else if (dialBox.trackWide < 0.6 || dialBox.trackTall < 0.6) {
      //   ⚠️ **Measured against the track, and the reason is a fault
      //   this section had for four hours on 2026-09-07.**
      //
      //   It used to compare the box's proportions against **all** the
      //   ink's, and against a 0.25 tolerance. Both numbers were real
      //   and both came out of one run, so it read like a sound
      //   comparison — but they are not the same kind of quantity:
      //
      //   | | reading | what it is |
      //   |---|---|---|
      //   | box | 1.66 | a **decision** (`ratio`, and the crop) |
      //   | track | 1.59 | **pure geometry** — `2R : 1.2588R`, a 210° constant, and `getBBox` excludes the stroke so size and stroke fall out of it too |
      //   | all ink | 1.48 | the track **plus the pips**, and a pip is drawn at the angle of *its own reading* |
      //
      //   So 0.11 of that spread was **data**: a window near 50% puts a
      //   pip at the very top and a window near 4% does not. The
      //   tolerance had 0.07 left. **A different set of percentages in
      //   the fixture turns this red, and the red says 「the box has
      //   the wrong proportions」 — pointing at something that is not
      //   broken.** It was passing on the fixture's numbers, not on the
      //   product's.
      //
      //   The track carries the same claim with none of the data:
      //   cropped it fills **0.71** of the box's height, uncropped
      //   **0.43**, and neither number moves when a percentage does.
      //
      //   ⚠️ **The ink's own shares are still printed below** — they
      //   are worth reading, they are just not fit to assert on.
      fail(
        `the dial's track fills ${dialBox.trackWide} across and ${dialBox.trackTall} down of its box (track ${dialBox.track} in ${dialBox.box}) — the box is cropped to the drawing, and a box the arc itself does not fill is dead band`,
      );
    }
    console.log(
      `dial box: track ${dialBox?.track} fills ${dialBox?.trackWide}/${dialBox?.trackTall} of ${dialBox?.box} · ` +
        `ink ${dialBox?.ink} fills ${dialBox?.wideShare}/${dialBox?.tallShare} (read, not asserted) · ` +
        `aspect box ${dialBox?.boxAspect} track ${dialBox?.trackAspect} ink ${dialBox?.inkAspect}`,
    );

    //   ── 8·what an account's dial says in its middle ────────────
    //   2026-09-07:「说过的没有额度的环,显示金额」. A reading with no
    //   denominator shows its money there; one with a denominator shows
    //   the share.
    //
    //   **Two rows, and neither alone is enough** — `acc-fill` has a
    //   fraction (0.45) and `acc-number` has a spend and no ceiling. An
    //   implementation that always draws money passes on the second and
    //   fails the first; one that always draws a percent fails the
    //   second; **and one that draws nothing when there is no fraction
    //   — which is what this replaced — passes neither.**
    const middleOf = async (row) => {
      await page.locator(`[data-quota-row="${row}"]`).click();
      await wait(300);
      return (
        await page
          .locator(`[data-quota-detail="${row}"] [data-quota-lead] text`)
          .textContent()
          .catch(() => null)
      )?.trim() ?? null;
    };
    const withCeiling = await middleOf("acc-fill");
    const without = await middleOf("acc-number");
    if (withCeiling !== "45%") {
      fail(`a reading with a ceiling reads ${JSON.stringify(withCeiling)} in the middle, not its share`);
    } else if (!without) {
      fail(
        `a reading with no ceiling reads ${JSON.stringify(without)} in the middle — 「没有额度的环,显示金额」`,
      );
    } else if (without !== "12$") {
      //   ⚠️ **Was `includes("12.34")`, and the product changed**: a
      //   ring's middle is at most four characters now (「保持在 4 位
      //   字符以内」), so 12.34 rounds to whole units and wears its
      //   sign. **Equality, not `includes`** — the ruling is about the
      //   shortening, and a substring check passes on the long form.
      fail(
        `a reading with no ceiling reads ${JSON.stringify(without)} where the fixture spent 12.34 — four characters means 「12$」`,
      );
    }

    //   ── 8·the unit ladder, on the glass ───────────────────────
    //   2026-09-07:「用户设置余量单位的时候,直接输入符号。比如 $,不要
    //   去识别,不然各种货币符号我们识别不完,用户也可能说的不对。给几个
    //   推荐选项,默认是 $。」
    //
    //   Three rows one field apart (see the fixture), read in the two
    //   places a unit appears — **the ring and the column, in one
    //   breath**, because 「一个账户在这一屏上只有一个单位」 is a claim
    //   about the pair and neither half can carry it alone. The code
    //   this replaced could say `124$` in one and `124 USDT` in the
    //   other and pass both.
    //
    //   ⚠️ **The order matters.** ① and ② are asserted equal, and that
    //   pair is deliberately weak: an implementation with `$` hard
    //   coded — ignoring `unit` altogether — satisfies it. ③ is the arm
    //   that fails such an implementation, and it is the reason the
    //   fixture carries a third row at all.
    const unitOf = async (row) => {
      await page.locator(`[data-quota-row="${row}"]`).click();
      await wait(300);
      return await page.evaluate((row) => {
        const at = (sel) =>
          document.querySelector(`[data-quota-detail="${row}"] ${sel}`)?.textContent?.trim() ?? null;
        return { mid: at("[data-quota-lead] text"), col: at("[data-quota-spent]") };
      }, row);
    };
    const unset = await unitOf("acc-number");
    const asDollar = await unitOf("acc-dollar");
    const asYuan = await unitOf("acc-yuan");
    const asFree = await unitOf("acc-usdt");
    const shown = JSON.stringify({ unset, asDollar, asYuan, asFree });
    if ([unset, asDollar, asYuan, asFree].some((t) => t.mid === null || t.col === null)) {
      fail(`probe dead: the unit rows drew ${shown}, so nothing below is measuring a unit`);
    } else if (unset.mid !== "12$" || unset.col !== "12.34$") {
      fail(
        `an account nobody set a unit on reads ${JSON.stringify(unset)} — 「默认是 $」, applied when drawing`,
      );
    } else if (asDollar.mid !== unset.mid || asDollar.col !== unset.col) {
      fail(
        `setting the unit to 「$」 reads ${JSON.stringify(asDollar)} where leaving it unset reads ${JSON.stringify(unset)} — the default and the thing it defaults to are one mark`,
      );
    } else if (asYuan.mid !== "12¥" || asYuan.col !== "12.34¥") {
      fail(
        `an account set to 「¥」 reads ${JSON.stringify(asYuan)} — the mark is the person's, and a default that cannot be overridden is a guess`,
      );
    } else if (asFree.mid !== "124") {
      //   Not `0k`: see the fixture's note on this row.
      fail(
        `a mark too long for the ring leaves ${JSON.stringify(asFree.mid)} in it — the magnitude is what a ring keeps, and 124 may never be drawn as a step down`,
      );
    } else if (asFree.col !== "124 USDT") {
      fail(
        `the column of a lettered mark reads ${JSON.stringify(asFree.col)} — a word takes a word gap where a symbol takes none (「右栏要读起来像钱」)`,
      );
    }

    //   ── 8·the row's menu ───────────────────────────────────────
    //   2026-09-07:「卡上不放按钮(撤掉、编辑都不要):右键列表里的条目弹
    //   置顶 / 编辑 / 删除」. Three claims, and the first two are an
    //   absence and its live control in one run — the shape section 7
    //   leans on: **the button is nowhere on the card, and the three
    //   items are there in the menu**. Counting only the absence passes
    //   on a build that lost the actions altogether.
    const buttonOnCard = await page.locator("[data-quota-detail] [data-quota-remove]").count();
    if (buttonOnCard !== 0) {
      fail(`the card still carries ${buttonOnCard} 撤掉 button(s) — the acts moved to the row's menu`);
    }
    await page.locator('[data-quota-row="acc-fill"]').click({ button: "right" });
    await wait(500);
    const menu = await page.locator("[data-row-menu] [data-menu-item]").evaluateAll((els) =>
      els.map((e) => ({ key: e.getAttribute("data-menu-item"), word: e.textContent?.trim() ?? "" })),
    );
    const keys = menu.map((m) => m.key);
    if (keys.length !== 3 || !keys.includes("pin") || !keys.includes("edit") || !keys.includes("remove")) {
      fail(`an account's menu offers ${JSON.stringify(keys)} — 置顶 / 编辑 / 删除 were expected`);
    } else if (menu.some((m) => m.word === "")) {
      fail(`probe void: an item has no word (${JSON.stringify(menu)}), so the words below measure nothing`);
    } else {
      //   **删除 takes two presses, and the first one is the control
      //   arm**: it puts the sentence on screen and moves nothing. A
      //   menu that removed on one press would empty the row and every
      //   assertion after it would still pass.
      await page.locator('[data-menu-item="remove"]').click();
      await wait(300);
      const armed = await page.evaluate(() => ({
        said: document.querySelector('[data-menu-confirm="remove"]')?.textContent?.trim() ?? null,
        rowStillThere: document.querySelectorAll('[data-quota-row="acc-fill"]').length,
        // The sentence is the account's own, never the machine's: two
        // different acts cannot share one clause about their cost.
        machineWord: document.querySelector('[data-menu-confirm="forget"]') !== null,
      }));
      if (!armed.said || armed.rowStillThere !== 1 || armed.machineWord) {
        fail(
          `the first 删除 press said ${JSON.stringify(armed.said)} and left ${armed.rowStillThere} row(s) — one press must arm and move nothing`,
        );
      }
    }
    await page.keyboard.press("Escape");
    await wait(300);

    //   **编辑's key box: shut where the key is not, open where it is.**
    //   Two rows, one field, opposite answers — the fixture has
    //   `acc-fill` read here (`mine: true`) and `acc-trouble` carried
    //   from a peer (`mine: false`), so an implementation that always
    //   shuts it fails on the first and one that never shuts it fails
    //   on the second. **Neither row alone can tell those apart.**
    //
    //   And on both: the box is **empty**, and nothing on the dialog
    //   says whether a key exists. A secret echoed to a screen has left
    //   the machine it was promised to, and from a peer there is
    //   nothing truthful to echo — the key is not on this disk.
    const keyBox = async (row) => {
      await page.locator(`[data-quota-row="${row}"]`).click({ button: "right" });
      await wait(400);
      await page.locator('[data-menu-item="edit"]').click();
      await wait(400);
      const seen = await page.evaluate(() => {
        const box = document.querySelector("[data-quota-edit-key]");
        const dialog = document.querySelector("[data-quota-edit]");
        return {
          drew: box !== null,
          // ⚠️ **`box.disabled`, not the `data-` marker beside it.**
          // Measured 2026-09-07: with the assertion reading the marker,
          // setting `disabled={false}` and leaving the marker alone
          // **passed** — the gate was checking that the component
          // describes itself consistently, which it does whether or not
          // the field is actually shut. The property is the thing; the
          // marker is a label about the thing.
          shut: box === null ? null : box.disabled,
          value: box?.value ?? null,
          // Anything on this dialog that answers「is there a key」 —
          // a tick, a mask, the word 已设置. The note under the box is
          // about what happens if you leave it blank, and says nothing
          // about what is stored.
          text: dialog?.textContent ?? "",
        };
      });
      await page.keyboard.press("Escape");
      await wait(300);
      return seen;
    };
    const here = await keyBox("acc-fill");
    const peer = await keyBox("acc-trouble");
    if (!here.drew || !peer.drew) {
      fail(`probe dead: the edit dialog drew no key box (here=${here.drew} peer=${peer.drew})`);
    } else if (here.shut !== false || peer.shut !== true) {
      fail(
        `the key box is shut=${here.shut} on a key held here and shut=${peer.shut} on one held by a peer — it opens only where the key is`,
      );
    } else if (here.value !== "" || peer.value !== "" || /••|已设置|\*\*\*/.test(here.text + peer.text)) {
      fail(
        `the edit dialog says something about the stored key (here=${JSON.stringify(here.value)} peer=${JSON.stringify(peer.value)}) — it never echoes one and never says whether one exists`,
      );
    }

    //   **编辑's unit box: what it holds, what it hints, and which of
    //   the shortcuts is lit.** 2026-09-07:「给几个推荐选项,默认是
    //   $」.
    //
    //   ① an account with a mark set → the box **holds** it, and that
    //   mark's chip is the lit one; ② an account with none → the box is
    //   **empty**, the hint is `$`, and **the `$` chip is still lit**,
    //   because what this account reads as today is `$` and a person
    //   opening 编辑 has to be able to see that.
    //
    //   ⚠️ **The hint is a hint and not a value**, and that is the
    //   whole of ②: prefilled, somebody who came to fix a *name* and
    //   pressed 保存 would leave with an override they never chose.
    //
    //   ⚠️ **Which chip is lit is read as ink, not as a marker.** The
    //   `data-quota-unit-pick` attribute is only how the probe finds
    //   them; whether one is lit is a computed `background-color`, and
    //   exactly one of the four may differ from the rest. A gate
    //   reading the component's own `data-on` would pass an
    //   implementation whose lighting had stopped working, because the
    //   label and the paint come out of the same expression.
    const unitBox = async (row) => {
      await page.locator(`[data-quota-row="${row}"]`).click({ button: "right" });
      await wait(400);
      await page.locator('[data-menu-item="edit"]').click();
      await wait(400);
      return await page.evaluate(() => {
        const box = document.querySelector("[data-quota-edit-unit]");
        const chips = [...document.querySelectorAll("[data-quota-unit-pick]")];
        const paint = (e) => getComputedStyle(e).backgroundColor;
        const grounds = chips.map(paint);
        // The odd one out by paint — no appeal to what the component
        // says about itself.
        const lit = chips.filter((e) => grounds.filter((g) => g === paint(e)).length === 1);
        const tops = chips.map((e) => Math.round(e.getBoundingClientRect().top));
        return {
          drew: box !== null,
          value: box?.value ?? null,
          hint: box?.placeholder ?? null,
          shut: box === null ? null : box.disabled,
          marks: chips.map((e) => e.getAttribute("data-quota-unit-pick")),
          lit: lit.map((e) => e.getAttribute("data-quota-unit-pick")),
          // 「一行放得下」, read as one line: every chip on the same
          // top edge. A row that wrapped would put one of them lower.
          lines: [...new Set(tops)].length,
        };
      });
    };
    const setTo = await unitBox("acc-yuan");
    await page.keyboard.press("Escape");
    await wait(300);
    const notSet = await unitBox("acc-number");
    //   **The chips are a shortcut, so pressing one has to reach the
    //   field** — and the field has to stay open afterwards, which is
    //   what separates 「推荐选项」 from a picker that owns the answer.
    await page.locator('[data-quota-unit-pick="€"]').click();
    await wait(250);
    const afterPress = await page.evaluate(() => {
      const box = document.querySelector("[data-quota-edit-unit]");
      return { value: box?.value ?? null, shut: box === null ? null : box.disabled };
    });
    await page.keyboard.press("Escape");
    await wait(300);
    if (!setTo.drew || !notSet.drew) {
      fail(`probe dead: the edit dialog drew no unit box (${setTo.drew}/${notSet.drew})`);
    } else if (setTo.marks.length !== 4 || setTo.marks[0] !== "$") {
      fail(
        `编辑 offers ${JSON.stringify(setTo.marks)} as shortcuts — four of them with 「$」 first (「给几个推荐选项,默认是 $」)`,
      );
    } else if (setTo.lines !== 1) {
      fail(`the shortcut row wrapped onto ${setTo.lines} lines — 「一行放得下」 is the count's whole rule`);
    } else if (setTo.value !== "¥" || setTo.hint !== "$") {
      fail(
        `an account marked 「¥」 opens holding ${JSON.stringify(setTo.value)} / hinting ${JSON.stringify(setTo.hint)} — the person's mark is the value, the default is the hint`,
      );
    } else if (notSet.value !== "" || notSet.hint !== "$") {
      fail(
        `an account with no mark opens holding ${JSON.stringify(notSet.value)} / hinting ${JSON.stringify(notSet.hint)} — a saved form must not invent a choice nobody made, and the hint is what an empty box draws`,
      );
    } else if (JSON.stringify(setTo.lit) !== '["¥"]' || JSON.stringify(notSet.lit) !== '["$"]') {
      fail(
        `the lit shortcut is ${JSON.stringify(setTo.lit)} on a 「¥」 account and ${JSON.stringify(notSet.lit)} on one with no mark — exactly one is lit, and on an empty box it is the default the figures are actually wearing`,
      );
    } else if (afterPress.value !== "€" || afterPress.shut !== false) {
      fail(
        `pressing a shortcut left the box at ${JSON.stringify(afterPress.value)} (shut=${afterPress.shut}) — a shortcut fills the field and leaves it open, it does not replace it`,
      );
    }

    //   …and the subscription is not a registration: nothing to edit,
    //   nothing to remove, so it is offered no menu at all.
    await page.locator('[data-quota-row][data-quota-kind="sub"]').first().click({ button: "right" });
    await wait(500);
    const onSub = await page.locator("[data-row-menu] [data-menu-item]").count();
    if (onSub !== 0) {
      fail(`the subscription row offers ${onSub} menu item(s) — it is not a registration`);
    }
    await page.keyboard.press("Escape");
    await wait(300);
    await page.locator('[data-quota-row][data-quota-kind="sub"]').first().click();
    await wait(300);

    //   **One way back to a freshly-routed card**, shared by every arm
    //   below: the route is swapped, the page reloaded so the shell
    //   asks again, and the subscription re-opened.
    const goToQuota = async () => {
      await page.reload({ waitUntil: "load" });
      await wait(400);
      await press(rail("quota"));
      await until(5_000, async () => (await page.locator("[data-quota-row]").count()) > 0);
      await page.locator('[data-quota-row][data-quota-kind="sub"]').first().click();
      await wait(300);
    };

    //   ── 8·which window speaks for the whole plan ───────────────
    //   2026-09-07:「list 页、打开后的多环,claude 显示 week_all。现在,
    //   是 fable。」 The headline is the **longest unscoped** window, not
    //   the fullest one — a window scoped to a model says how full that
    //   model's slice is, and a line with room for one number owes it
    //   to the subscription, not to a slice of it.
    //
    //   **The fixture is built so 「fullest」 and 「whole plan」 give
    //   different answers**: the week is 39% and Fable's slice is 29%,
    //   so a build that still picked the fullest would happen to agree
    //   — which is why the arms below add one where the *scoped* window
    //   is the fullest. Without that, this check passes on the code it
    //   was written to replace.
    const headlineOf = async () => await page.evaluate(() => {
      const row = document.querySelector('[data-quota-row][data-quota-kind="sub"]');
      const detail = document.querySelector("[data-quota-detail]");
      const arc = detail?.querySelector("[data-quota-lead] [data-ring-fill]");
      return {
        summary: row?.lastElementChild?.textContent?.trim() ?? "",
        centre: detail?.querySelector("[data-quota-lead] text")?.textContent ?? "",
        // Bare track is「no arc drawn at all」— the same picture 问不出来
        // wears. Read off the arc itself, never off a class.
        arc: arc === null || arc === undefined ? null : arc.getAttribute("d"),
        // What is still listed beside it: nothing may be lost when the
        // headline is.
        parts: [...(detail?.querySelectorAll("[data-quota-part]") ?? [])].map((e) =>
          e.getAttribute("data-quota-part"),
        ),
      };
    });
    //   ⚠️ **Tombstone: these three arms used to test the *rule*, and
    //   they do not any more.** Until 2026-09-07 the face picked the
    //   window that speaks for the whole plan (`headline()`), and these
    //   walked its three steps: the longest unscoped window, the
    //   five-hour when there is no week, and **nothing** when no window
    //   covers the plan.
    //
    //   That rule lives in `khor_node::quota::whole_plan_fill` now, so
    //   that a probed subscription and a **registered** one share one
    //   answer instead of each working out its own. It is tested there,
    //   three steps and a teardown each
    //   (`the_share_is_the_longest_window_that_covers_the_whole_plan`).
    //
    //   **What is left here is one step smaller, and worth saying
    //   because these arms read as if they were more:** that the face
    //   draws the share it was handed, and that a `null` share draws
    //   bare track rather than a full ring or a zero. **They say
    //   nothing about which window that share came from** — somebody
    //   who breaks that rule and comes here looking for the red will
    //   not find it here. It is in the Rust.
    //
    //   (a) **A scoped window that is fuller than the unscoped one.**
    //   The headline must still be the week's 30, not the slice's 88 —
    //   and the share routed with it says 30, so what this arm now
    //   proves is that the face prints what it was given rather than
    //   reaching for the fullest number on screen.
    const slicedWindows = [
      { span: "five_hour", scope: null, used_pct: 4, resets_at: null, active: null, severity: null },
      { span: "seven_day", scope: null, used_pct: 30, resets_at: null, active: null, severity: null },
      { span: "seven_day", scope: "Fable", used_pct: 88, resets_at: null, active: null, severity: null },
    ];
    await routeQuota(quotaCredits, slicedWindows, 0.3);
    await goToQuota();
    const whole = await headlineOf();
    //   ⚠️ **The card's middle only.** This read the row's right-hand
    //   cell too, and that cell stopped being the reading on 2026-09-07
    //   (「右边是更新时间」) — the row's figure is in its ring now, which
    //   `8·what a ring may hold` measures. **The product changed; what
    //   this section is about did not.**
    if (!whole.centre.includes("30")) {
      fail(
        `with a 30% week beside an 88% model slice the card reads ${JSON.stringify(whole.centre)} — the headline is the whole plan, not the fullest window`,
      );
    }
    //   (b) **Only a five-hour window.** Still the whole plan, and the
    //   row says which window it is — a reader has to see that this is
    //   not a week.
    const hourOnly = [slicedWindows[0]];
    await routeQuota(quotaCredits, hourOnly, 0.04);
    await goToQuota();
    const hour = await headlineOf();
    const fiveHourWord = (
      await page.locator('[data-quota-part="five_hour"] [data-quota-window-name]').textContent()
    )?.trim() ?? "";
    if (!fiveHourWord) {
      fail("probe dead: the five-hour window has no word, so what follows measures nothing");
    } else if (!hour.centre.includes("4")) {
      fail(
        `with only a five-hour window the card reads ${JSON.stringify(hour.centre)} — it steps down to the whole plan's shorter window`,
      );
    } else if (!hour.parts.includes("five_hour")) {
      //   …and it is nameable: the row no longer prints the window's
      //   word (its right end is the age), so the panel is where a
      //   reader learns *which* window that 4% belongs to.
      fail(`the panel lists ${JSON.stringify(hour.parts)} — the window it stepped down to has to be nameable`);
    }
    //   (c) **No unscoped window at all**: nothing may be invented.
    //   Bare track — **not a full arc** (that is 不限量) and **not 0%**
    //   (that is a reading). And the panel still lists every slice: the
    //   third claim is what tells 「the headline is gone」 from 「the
    //   card fell over」.
    const slicesOnly = [
      { span: "seven_day", scope: "Fable", used_pct: 88, resets_at: null, active: null, severity: null },
      { span: "seven_day", scope: "Opus", used_pct: 12, resets_at: null, active: null, severity: null },
    ];
    // `null`, which is what the node sends when no window covers the
    // whole plan — and the assertion below is that a `null` share
    // draws bare track. **Not `undefined`**: that is the replica's own
    // failure mode and it draws a NaN arc, which would fail this arm
    // for a reason that has nothing to do with the product.
    await routeQuota(quotaCredits, slicesOnly, null);
    await goToQuota();
    const none = await headlineOf();
    if (none.arc !== null) {
      fail(
        `with no whole-plan window the dial still draws an arc (${JSON.stringify(none.arc)}) — bare track is the picture for「there is no such number」`,
      );
    } else if (none.centre.trim() !== "") {
      fail(
        `with no whole-plan window the card says ${JSON.stringify(none.centre)} — it may not invent one`,
      );
    } else if (none.parts.length !== slicesOnly.length) {
      fail(
        `the panel lists ${JSON.stringify(none.parts)} for ${slicesOnly.length} routed windows — losing the headline may not cost the slices`,
      );
    }

    //   ── 8b) 登记过的订阅:一行,一张卡,一个数 ────────────────
    //   2026-09-07 用户:「现在的登记里面,还应该能够登记订阅。」 A
    //   subscription is something you register now; the probed row is
    //   what a khor that has been told nothing still shows. Both at
    //   once would be one login drawn twice.
    const routeBoard = async (rows) => {
      await page.unroute("**/quota_board");
      await page.route("**/quota_board", (r) =>
        r.fulfill({ contentType: "application/json", body: JSON.stringify(rows) }),
      );
    };
    //   `windows` is the standing fixture's, so the registered row is
    //   made of exactly what the probed row is made of — one replacing
    //   the other is the claim, and a different set of windows would
    //   blur it. `credits` is stated because a subscription's reading
    //   carries it (see `ring`'s note on why an omitted key is not a
    //   null).
    const registered = (mine) => ({
      ...ring("acc-sub", "max", "claude", {
        kind: "sub",
        fill: 0.39,
        windows: quotaWindows,
        plan: "Max 20x",
        credits: null,
      }),
      mine,
      home_name: mine ? "gate" : "peer",
      age_ms: mine ? 0 : 42_000,
    });
    const subRowIds = async () =>
      await page
        .locator('[data-quota-row][data-quota-kind="sub"]')
        .evaluateAll((els) => els.map((e) => e.getAttribute("data-quota-row")));

    //   **Read as ids, not as a count**: 「one row」 and 「the *right*
    //   one」 are two claims, and a build that suppressed the
    //   registration instead of the probe satisfies the count.
    await routeBoard([...board, registered(true)]);
    await goToQuota();
    const homedHere = await subRowIds();
    if (JSON.stringify(homedHere) !== JSON.stringify(["acc-sub"])) {
      fail(
        `with this machine's subscription registered the list draws ${JSON.stringify(homedHere)} — the registration stands and the probe (sub:local) stands down`,
      );
    }

    //   ⚠️ **This arm is the test.** The one above passes just as well
    //   against a build that hides the probe whenever *any*
    //   subscription exists anywhere in the mesh. This one registers a
    //   subscription homed on **another** machine and demands the probe
    //   **stay** — two logins, two rows. One arm cannot tell those
    //   builds apart, and it would read exactly like a successful
    //   verification.
    //
    //   The two fixtures differ in **one field** (`mine`): two that
    //   differed in two are not a control group, they are two
    //   unrelated samples, and whichever difference the code happened
    //   to read explains the result.
    await routeBoard([...board, registered(false)]);
    await goToQuota();
    const homedElsewhere = (await subRowIds()).sort();
    if (JSON.stringify(homedElsewhere) !== JSON.stringify(["acc-sub", "sub:local"])) {
      fail(
        `a subscription registered on another machine left ${JSON.stringify(homedElsewhere)} — it must not stand down this machine's own probed row: two logins, two rows`,
      );
    }

    //   **One row is one card.** 「Has an account」 used to mean 「is not
    //   a subscription」; a registered one has both, so a detail chosen
    //   the old way draws the windows card **and** the money card, one
    //   under the other, each answering as if the other were not there.
    await routeBoard([...board, registered(true)]);
    await goToQuota();
    await page.locator('[data-quota-row="acc-sub"]').click();
    await wait(300);
    const cards = await page.locator("[data-quota-detail]").count();
    if (cards !== 1) {
      fail(`a registered subscription opens ${cards} cards — one row is one card`);
    } else if ((await page.locator("[data-quota-detail] [data-quota-parts]").count()) !== 1) {
      //   …and it is the **windows** card, told from the money card by
      //   what only one of them has rather than by an attribute both
      //   would carry.
      fail("a registered subscription's card has no window list — it opened the money card");
    }

    //   ⚠️⚠️ **THIS FIXTURE IS DELIBERATELY IMPOSSIBLE. MAKING IT
    //   CONSISTENT SILENTLY RETIRES THIS ARM.** The share (0.5) matches
    //   none of its own windows (4 / 39 / 29), which cannot happen on
    //   the wire — the node derives `fill` *from* those windows. It
    //   reads like bad test data. It is not.
    //
    //   **What happens if you tidy it:** set it to 0.39 and this arm
    //   goes on passing, for ever, against every implementation — it
    //   would then be asserting that a face which re-derives the number
    //   from the windows draws the same thing as one that prints what
    //   it was handed, **which is true, which is why the assertion
    //   would be worthless**. Nothing would go red, and the axis this
    //   arm exists for would stop being tested with no sign of it.
    //
    //   That axis is 「the face draws the node's number」 — the property
    //   the rule's move out of `QuotaLanding.tsx` created. **Every
    //   other arm here is consistent, and consistency is exactly what
    //   blinds them to it**: a face that quietly grew its own copy of
    //   the rule back would pass all of them.
    const impossible = registered(true);
    await routeBoard([...board, { ...impossible, reading: { ...impossible.reading, fill: 0.5 } }]);
    await goToQuota();
    await page.locator('[data-quota-row="acc-sub"]').click();
    await wait(300);
    const middle = (await page.locator("[data-quota-detail] [data-quota-lead] text").first().textContent())?.trim() ?? "";
    if (middle !== "50%") {
      fail(
        `the dial's middle reads ${JSON.stringify(middle)} for a reading whose share is 0.5 — the face must draw the node's number, and any of the routed windows appearing here means the rule that moved to khor_node::quota::whole_plan_fill grew back`,
      );
    }

    //   Put the board back: everything after this is written against
    //   the standing fixture, and a gate that left its own state behind
    //   would make the next section's failure somebody else's puzzle.
    await routeBoard(board);
    //   …and back to the fixture the rest of this section is written
    //   against.
    await routeQuota(quotaCredits);
    await goToQuota();

    //   ── 8·the bars, and that they are simply there ─────────────
    //   2026-09-07:「左多环右展开 bar 来铺宽度空间」,「直接就展开,不用
    //   用户点击」. Each window is a row beside the dial — name, a bar,
    //   a number — and **nothing is pressed to make that happen**.
    //
    //   ⚠️ **Rewritten twice in one day, and the product changed both
    //   times, not the assertion.** It pressed a line of text
    //   (`data-quota-spread-toggle`), then pressed the dial
    //   (`data-quota-dial-toggle`, `02066e9`); now there is nothing to
    //   press. **The absence is asserted with the bars as the live
    //   control in the same run** — a zero from a renamed marker and a
    //   zero from a control that is gone read the same otherwise.
    const bars = await page.evaluate(() => {
      const detail = document.querySelector("[data-quota-detail]");
      const rows = [...(detail?.querySelectorAll("[data-quota-part]") ?? [])];
      return {
        // Nothing on this card is a handle for opening anything.
        handles: document.querySelectorAll(
          "[data-quota-dial-toggle],[data-quota-spread-toggle],[data-quota-spread]",
        ).length,
        parts: rows.map((r) => r.getAttribute("data-quota-part")),
        // **Real widths, off the glass.** Not a class, not the `style`
        // string, not a `data-` number: a marker saying 39 while the
        // ink is 0 wide is the trap section 8's key box fell into.
        // **`data-part-*`, the machine card's names**, because it is
        // the machine card's component now (`components/Bar`). Scoped
        // to this card's rows, so the two surfaces cannot read each
        // other's bars.
        track: rows.map((r) =>
          Math.round(r.querySelector("[data-part-bar]")?.getBoundingClientRect().width ?? -1),
        ),
        ink: rows.map((r) =>
          Math.round(r.querySelector("[data-part-ink]")?.getBoundingClientRect().width ?? -1),
        ),
        pct: rows.map((r) => r.querySelector("[data-quota-window-pct]")?.textContent?.trim() ?? ""),
      };
    });
    if (bars.handles !== 0) {
      fail(`the card still carries ${bars.handles} thing(s) to press before the bars appear — they are simply there now`);
    } else if (bars.parts.length !== quotaWindows.length) {
      fail(`the card draws ${bars.parts.length} bar rows for the fixture's ${quotaWindows.length} windows`);
    } else if (bars.ink.some((w) => w <= 0)) {
      //   ⚠️ **A bar of zero width, and why this cheap line is here.**
      //   The first build of this layout drew every bar 0px wide — the
      //   column had no `flex-1`, so the grid's `1fr` track resolved to
      //   nothing. **On a screenshot that reads as 「the bars have not
      //   been added yet」, not as 「the layout is broken」** — the
      //   broken shape and the not-yet-built shape are identical, and
      //   it is looked at in a moment when 「not yet」 is the likelier
      //   story. Only a measurement tells them apart.
      fail(
        `a bar's ink is ${JSON.stringify(bars.ink)} on tracks ${JSON.stringify(bars.track)} — a zero-width bar looks exactly like a bar nobody has drawn yet`,
      );
    } else {
      //   **The ink is the reading.** Longest bar is the fullest
      //   window, and three different percents are three different
      //   lengths — a row of equal bars passes every count above and
      //   says nothing (the same hole the day chart's heights guard).
      const want = quotaWindows.map((w) => w.used_pct);
      const order = (xs) => xs.map((_, i) => i).sort((a, b) => xs[a] - xs[b]).join(",");
      if (order(bars.ink) !== order(want)) {
        fail(
          `the bars' ink is ${JSON.stringify(bars.ink)} for windows at ${JSON.stringify(want)} — longer bar, fuller window`,
        );
      } else if (new Set(bars.ink).size !== bars.ink.length) {
        fail(`three windows at three percents drew ${JSON.stringify(bars.ink)} — equal bars say nothing`);
      }
      //   …and the number beside each bar is that window's own.
      for (const w of quotaWindows) {
        const id = w.scope ? `${w.span}:${w.scope}` : String(w.span);
        const at = bars.parts.indexOf(id);
        if (at < 0 || bars.pct[at] !== `${Math.round(w.used_pct)}%`) {
          fail(`the row for ${JSON.stringify(id)} reads ${JSON.stringify(bars.pct[at] ?? null)} for ${w.used_pct}%`);
        }
      }
    }

    //   ── 8·credits, the other arm ────────────────────────────────
    //   Everything above measured credits **without** a ceiling: no
    //   pip, a figure below. That is one arm, and one arm of a control
    //   group is not a control group — 「rings credits never」 passes it
    //   just as well as the rule being asserted. So the same money is
    //   served again **with** a monthly ceiling, and three things have
    //   to move together: a pip, a part, and the pip's angle.
    //
    //   **The count is compared against the fixture, not against the
    //   pips.** The pips and the parts come off one list in the
    //   product (see the note further up), so `pips === parts` is true
    //   of any wrong list; `windows + 1` is the number an outside
    //   thing says it should be.
    await routeQuota(quotaCreditsCapped);
    await goToQuota();
    const capped = await page.evaluate(() => {
      const row = document.querySelector('[data-quota-row][data-quota-kind="sub"]');
      const detail = document.querySelector("[data-quota-detail]");
      return {
        pips: [...(row?.querySelectorAll("[data-dial-pip]") ?? [])].map((e) => ({
          word: e.getAttribute("data-dial-pip"),
          at: Number(e.getAttribute("data-dial-pip-at")),
        })),
        parts: [...(detail?.querySelectorAll("[data-quota-part]") ?? [])].map((e) =>
          e.getAttribute("data-quota-part"),
        ),
      };
    });
    const want = quotaWindows.length + 1;
    if (capped.pips.length !== want || capped.parts.length !== want) {
      fail(
        `with a monthly ceiling on the credits the rim carries ${capped.pips.length} pips and the panel ${capped.parts.length} parts — the fixture has ${quotaWindows.length} windows and one credits with a denominator, so both are ${want}`,
      );
    } else if (!capped.parts.includes("credits")) {
      fail(`the parts are ${JSON.stringify(capped.parts)} — credits with a ceiling is one of them`);
    } else {
      //   9.21 of 20 is 46%, and no window in this fixture is at 46 —
      //   so this mark cannot be somebody else's.
      const pct = Math.round((quotaCreditsCapped.used / quotaCreditsCapped.monthly_limit) * 100);
      if (!capped.pips.some((p) => p.at === pct)) {
        fail(
          `the credits' share is ${pct}% and the rim's marks read ${JSON.stringify(capped.pips.map((p) => p.at))}`,
        );
      }
    }
    //   **The middle arm**: the same money, switched on, with no
    //   ceiling. One field apart from the fixture above, and the answer
    //   has to flip back — no pip, no part, and the figures still on
    //   the glass because the endpoint did report a spend.
    await routeQuota(quotaCreditsUncapped);
    await goToQuota();
    const uncapped = await page.evaluate(() => {
      const row = document.querySelector('[data-quota-row][data-quota-kind="sub"]');
      const detail = document.querySelector("[data-quota-detail]");
      return {
        pips: (row?.querySelectorAll("[data-dial-pip]") ?? []).length,
        parts: (detail?.querySelectorAll("[data-quota-part]") ?? []).length,
        used: detail?.querySelector("[data-quota-credits-used]")?.textContent ?? null,
      };
    });
    if (uncapped.pips !== quotaWindows.length || uncapped.parts !== quotaWindows.length) {
      fail(
        `credits switched on with no ceiling draws ${uncapped.pips} pips and ${uncapped.parts} parts — a mark needs an angle and an angle needs a denominator, so both are ${quotaWindows.length}`,
      );
    } else if (!uncapped.used?.includes("9.21")) {
      fail(
        `probe dead: the uncapped credits' spend reads ${JSON.stringify(uncapped.used)} — if the figures are not on the glass, 「no pip」 is measuring a block that never drew`,
      );
    }

    //   ── 8·a pip may not be drawn outside its box ───────────────
    //   **The thing the crop could break, and the only assertion that
    //   would notice.** `Face` crops the box to `reach` — the rim plus
    //   a pip's whole length plus its round cap — and a crop computed
    //   without that allowance would slice pips off. **Clipping raises
    //   nothing**: the mark is simply not there, on a dial that
    //   otherwise looks perfect.
    //
    //   ⚠️ **It needs a pip near 50%, and the standing fixture has
    //   none** (4 / 39 / 29) — so this arm routes its own windows and
    //   puts one there. A pip's angle is its own reading's, and only
    //   one near half-way reaches the top of the dial, which is the
    //   edge the crop is tight against.
    //
    //   **How much that sample is worth, measured rather than
    //   argued.** Three teardowns of `reachOf`, each run whole:
    //
    //   | what was broken | who caught it |
    //   |---|---|
    //   | the crop turned off | the track check (0.71 → **0.43**) and the size-slot gate |
    //   | `reach` forgets the pips entirely | this (50% is **18.9** out) — and `spare`, which crosses on the standing fixture too |
    //   | `reach` forgets only the round **cap** | **this alone** (50% is **1.63** out) |
    //
    //   The third row is why the 50 is here. At 4/39/29 the topmost
    //   pip still had **4.57** of room under that same break, so
    //   `spare` stayed green, the track check went *greener* (0.73),
    //   and a dial was quietly painting outside its box. **A pip at 50
    //   sits on the boundary by construction** — `reach` is defined as
    //   exactly how far it goes — so it is the one sample that turns
    //   an error of any size into a red.
    //
    //   ⚠️ **The stroke is added back by hand.** `getBBox` is the
    //   geometry, not the paint; a pip is a round-capped line, so the
    //   mark on screen reaches half a stroke further on every side
    //   than its box says. Measuring the geometry alone would call a
    //   crop 「inside」 while the cap hung over the edge.
    const topWindows = [
      { span: "five_hour", scope: null, used_pct: 50, resets_at: Math.floor(Date.now() / 1000) + 3600, active: false, severity: "normal" },
      { span: "seven_day", scope: null, used_pct: 39, resets_at: Math.floor(Date.now() / 1000) + 86_400, active: true, severity: "normal" },
    ];
    await routeQuota(quotaCredits, topWindows, 0.5);
    await goToQuota();
    const pipRoom = await page.evaluate(() => {
      const svg = document.querySelector("[data-quota-lead] [data-dial-face]");
      if (!svg) return null;
      const [bx, by, bw, bh] = svg.getAttribute("viewBox").split(/\s+/).map(Number);
      const pips = [...svg.querySelectorAll("[data-dial-pip]")].map((e) => {
        const b = e.getBBox();
        const half = (parseFloat(getComputedStyle(e).strokeWidth) || 0) / 2;
        return {
          at: e.getAttribute("data-dial-pip-at"),
          //   The smallest of the four margins: negative means the
          //   painted mark crosses that edge of the box.
          room: +Math.min(
            b.x - half - bx,
            bx + bw - (b.x + b.width + half),
            b.y - half - by,
            by + bh - (b.y + b.height + half),
          ).toFixed(2),
        };
      });
      return { pips, tightest: pips.length ? pips.reduce((m, p) => (p.room < m.room ? p : m)) : null };
    });
    if (!pipRoom || pipRoom.pips.length !== topWindows.length) {
      fail(
        `probe dead: a dial routed ${topWindows.length} windows drew ${pipRoom ? pipRoom.pips.length : "no"} pips — nothing below is measuring a mark`,
      );
    } else if (!pipRoom.pips.some((p) => p.at === "50")) {
      fail(
        `probe void: the pips read ${JSON.stringify(pipRoom.pips.map((p) => p.at))} and none of them is the 50 this arm routed — without it the check below never reaches the top of the box, which is the only edge it is about`,
      );
    } else if (pipRoom.tightest.room < -0.5) {
      //   ⚠️ **The expected reading here is exactly `0`, and that is
      //   the design rather than a near miss.** The crop's bound *is*
      //   the furthest a pip can reach (`Face`'s `reach`), so the
      //   worst-placed pip — the one at 50%, straight up — lands on
      //   the boundary to the pixel. Measured 2026-09-07: `50` has
      //   `0.00` of room and `39` has `6.21`.
      //
      //   So the half-unit is **slack for float noise, not slack in
      //   the claim**: a crop that actually forgot the pips misses by
      //   a pip's whole length (`9k`, about 17 units at this size),
      //   which is thirty times this tolerance. **Widening it to make
      //   something pass would be a different act** — this is here so
      //   that a `-0.004` from sub-pixel rounding does not read as a
      //   clipped mark.
      fail(
        `a pip at ${pipRoom.tightest.at}% is drawn ${-pipRoom.tightest.room} outside its box — the crop must leave room for a pip's whole length and its cap (Face reach), and a clipped mark raises nothing at all`,
      );
    }
    console.log(
      `pip room: ${JSON.stringify(pipRoom?.pips)} — tightest ${pipRoom?.tightest?.room} at ${pipRoom?.tightest?.at}%`,
    );

    //   …and back to the fixture everything after this was written
    //   against, so a later section is not silently measuring this one.
    //
    //   ⚠️ **Three arms above lean on these two lines and none of them
    //   puts the route back itself** — the capped credits, the uncapped
    //   credits, and the pip-room arm, which routes a **50% five-hour
    //   window** to push a mark to the very top. **Anything inserted
    //   between the last of them and this restore inherits that 50%
    //   window**, and 8a below is written against the standing fixture
    //   (`board.length + 1` rows — 10 since a widest-middle account
    //   joined the fixture on 2026-09-08, and spelled as the sum
    //   rather than as a count for exactly that reason): it would go
    //   red saying
    //   something about *8a*, which is a sentence about a section that
    //   is not broken.
    //
    //   So: an arm added down here either restores what it routed, or
    //   goes **above** this line and says in its own note that it
    //   borrows this one. (`8b` a few screens up does the first: it
    //   only touches `**/quota_board` and puts that back itself.)
    await routeQuota(quotaCredits);
    await goToQuota();
  }

  // ── 8a) 额度's bar: search, the three axes, the grouping ──────
  // 2026-09-06:「额度这侧边栏分组,和 chat 的 sessionlist 类似,有 filter
  // 和搜索」. Every claim below is tied to the fixture: three accounts
  // (来源 =「khor 登记」, providers newapi/claude/openai) and one
  // subscription (来源 = this machine, `gate`).
  // The axis this page opens on (`QuotaLanding`'s `GROUP_DEFAULT`).
  // Spelled once here so the two places below that press it cannot
  // drift apart from each other.
  const GROUP_DEFAULT = "from:";
  const quotaBar = async () => (await page.locator("[data-quota-row]").count());
  const quotaKinds = async () =>
    (await page.locator("[data-quota-row]").evaluateAll((els) =>
      els.map((e) => e.getAttribute("data-quota-kind")),
    ));

  // (0) **The two 类型 words, read off the menu rather than copied
  // here.** Everything below compares against these, so a rename in
  // `zh.toml` moves the gate with the product instead of turning it
  // red — and if the ticks ever stop being there, the probe says so
  // rather than measuring a filter that is not on the screen.
  await press("[data-pane-filter]");
  const tickLabel = async (key) =>
    (await page.locator(`[data-filter-option="${key}"]`).textContent())?.trim() ?? "";
  const subTickWord = await tickLabel("kind:sub");
  const billTickWord = await tickLabel("kind:bill");
  if (!subTickWord || !billTickWord || subTickWord === billTickWord) {
    fail(
      `probe dead: the 类型 axis offers ${JSON.stringify([subTickWord, billTickWord])} — nothing below is measuring an axis`,
    );
  }
  await page.keyboard.press("Escape");
  await wait(300);

  // (i) **Search reads the words on the row, not only its name.** The
  // 计费 word is on the three accounts' second line and is in no row's
  // *name* — so a box matching names alone comes back with nothing
  // here, and one ignoring the query comes back with all four. Both
  // wrong answers are on the other side of this number.
  await page.locator("[data-pane-search]").fill(billTickWord);
  await wait(300);
  const billOnly = await quotaKinds();
  if (billOnly.length !== board.length || billOnly.some((k) => k !== "bill")) {
    fail(
      `searching ${JSON.stringify(billTickWord)} leaves ${JSON.stringify(billOnly)} — the ${board.length} billed rows were expected, and the row's own words are what the box reads`,
    );
  }
  // …and a word nothing carries empties the list and says so. Without
  // this arm the one above is satisfied by a box that narrows to
  // whatever it likes as long as the subscription is not in it.
  await page.locator("[data-pane-search]").fill("zzqqxx");
  await wait(300);
  const noneLeft = await quotaBar();
  const saidNone = await page.locator("[data-quota-none]").count();
  if (noneLeft !== 0 || saidNone !== 1) {
    fail(`a search matching nothing leaves ${noneLeft} rows and ${saidNone} 「没有匹配的」`);
  }
  await page.locator("[data-pane-search]").fill("");
  await page.keyboard.press("Escape");
  await wait(300);
  if ((await quotaBar()) !== listed) {
    fail(`clearing the box leaves ${await quotaBar()} rows, not the ${listed} the list had`);
  }

  // (ii) **The 来源 axis, both arms.** One tick is not a control: a
  // filter that always leaves the accounts standing would pass
  // 「tick khor 登记, get the accounts」 while being blind to the axis.
  // So both values are ticked in turn and the two answers have to be
  // *different* ones — which is the difference the fixture was built to
  // have (three accounts registered here, one subscription read on
  // `gate`).
  await press("[data-pane-filter]");
  await press('[data-filter-option="from:"]');
  const khorOnly = await page.locator("[data-quota-row]").evaluateAll((els) =>
    els.map((e) => e.getAttribute("data-quota-from")),
  );
  if (khorOnly.length !== board.length || khorOnly.some((f) => f !== "khor")) {
    fail(`ticking 来源 =「khor 登记」 leaves ${JSON.stringify(khorOnly)}`);
  }
  await press('[data-filter-option="from:"]');
  await press('[data-filter-option="from:gate"]');
  const gateOnly = await page.locator("[data-quota-row]").evaluateAll((els) =>
    els.map((e) => e.getAttribute("data-quota-from")),
  );
  if (gateOnly.length !== 1 || gateOnly[0] !== "gate") {
    fail(`ticking 来源 =「gate」 leaves ${JSON.stringify(gateOnly)}, not this machine's subscription alone`);
  }
  await press('[data-filter-option="from:gate"]');

  // (iii) **Grouping cuts the same list three ways, it opens on 来源,
  // and 不分组 draws no heading at all.** The heading and the tick print
  // one string — the page's own rule — so the 订阅 heading is compared
  // against the word read off the tick above rather than against a copy
  // written here.
  const groupKeys = async () =>
    (await page.locator("[data-quota-group]").evaluateAll((els) =>
      els.map((e) => e.getAttribute("data-quota-group")),
    ));
  //   ⚠️ **This used to open by asserting 不分组 draws nothing, because
  //   不分组 was the default. It is 来源 now** (2026-09-07), and the
  //   change is the product's, not the assertion's: the sub-line was cut
  //   to one word and 来源 was one of the segments that left, so the
  //   heading is the only place that axis is readable. A default of
  //   不分组 would leave it something you can sieve with and cannot see.
  //
  //   **The default is asserted before anything is pressed**, which is
  //   the only moment it is observable — every later reading in this
  //   section is of a grouping somebody chose.
  //
  //   **And「default」has to mean default.** The choice is remembered in
  //   `localStorage`, so a browser carrying a remembered value would let
  //   this assertion pass on a build whose default is anything at all —
  //   the reading would be of a memory. The context is fresh per run
  //   today; this says so out loud instead of relying on it.
  const remembered = await page.evaluate(() => {
    try {
      return window.localStorage.getItem("khor.quota.group.v2");
    } catch {
      return "unreadable";
    }
  });
  if (remembered !== null) {
    fail(
      `probe void: this browser remembers 分组 = ${JSON.stringify(remembered)}, so what follows is not a reading of the default`,
    );
  }
  const atFirst = await groupKeys();
  if (atFirst.length !== 2 || !atFirst.includes("from:gate") || !atFirst.includes("from:")) {
    fail(
      `额度 opens grouped by ${JSON.stringify(atFirst)} — 来源 is the default now, so this machine and 「khor 登记」 were expected`,
    );
  }
  //   …and 不分组 is still one press away and still draws nothing: a
  //   default nobody can undo is a different bug from a wrong default.
  await press('[data-arrange-option="none"]');
  if ((await groupKeys()).length !== 0) {
    fail(`不分组 draws ${JSON.stringify(await groupKeys())} headings`);
  }
  await press('[data-arrange-option="kind:"]');
  const byKind = await groupKeys();
  if (byKind.length !== 2 || !byKind.includes("kind:sub") || !byKind.includes("kind:bill")) {
    fail(`分组 by 类型 draws ${JSON.stringify(byKind)} — one 订阅 heading and one 计费 heading were expected`);
  }
  const kindHead = (await page.locator('[data-quota-group="kind:sub"]').textContent())?.trim();
  if (kindHead !== subTickWord) {
    fail(`the 订阅 heading says ${JSON.stringify(kindHead)} and its tick says ${JSON.stringify(subTickWord)} — one value, two spellings`);
  }
  // The second axis, because a grouper that ignores which axis it was
  // given would draw the same two headings again.
  await press('[data-arrange-option="from:"]');
  const byFrom = await groupKeys();
  if (byFrom.length !== 2 || !byFrom.includes("from:gate") || !byFrom.includes("from:")) {
    fail(`分组 by 来源 draws ${JSON.stringify(byFrom)} — this machine and 「khor 登记」 were expected`);
  }
  // **Put the pane back — to the default, not to flat.** The ticks and
  // the grouping are remembered in localStorage, so anything left on
  // here follows the run into every later section and into the
  // screenshots; and since 2026-09-07 「back」 is 来源, because that is
  // what a person who has touched nothing sees.
  await press(`[data-arrange-option="${GROUP_DEFAULT}"]`);
  await page.keyboard.press("Escape");
  await wait(300);
  if ((await quotaBar()) !== listed || (await groupKeys()).length !== 2) {
    fail(
      `the 额度 pane was left off its default: ${await quotaBar()} rows and ${JSON.stringify(await groupKeys())} headings`,
    );
  }
  // What this section actually read, printed like every other one: a
  // section that only fails is a section nobody can tell ran.
  console.log(
    `额度 bar: ${listed} rows (${board.length} + 1 ${subTickWord}), ` +
      `search ${JSON.stringify(billTickWord)} → ${billOnly.length}, nothing → 0, ` +
      `来源 khor → ${khorOnly.length} / gate → ${gateOnly.length}, ` +
      `groups ${JSON.stringify(byKind)} then ${JSON.stringify(byFrom)}`,
  );

  // ── 8a) the detail's lower half: what was spent, in tokens ────────
  //
  // Six things, and each is one of the ways this could be drawn wrong
  // while looking finished.
  {
    const half = "[data-quota-detail] [data-usage-half]";
    await press('[data-quota-row="sub:local"]');
    await wait(400);

    if ((await page.locator(`${half} [data-usage-chart]`).count()) === 0) {
      fail("probe dead: the usage half never drew, so nothing below was measured");
    }
    //   ① 「it draws nothing while asking」 is tested **before the
    //   walk**, at the top of this file — that face only exists on the
    //   first fetch, and the walk's 額度 stop needs the answer landed.
    //
    //   **The controls are the machine card's, so they are pressed the
    //   machine card's way**: one settings button per figure, opening a
    //   panel with a row per thing it sets. There are no chips and no
    //   `<select>`s here any more, and this helper being the same shape
    //   as section 10's `pick` is the point — one control, one way of
    //   pressing it.
    const pickUsage = async (which, key) => {
      // **Open the settings, then pick** — the machine card's helper
      // says why this is two presses since 2026-09-08.
      //
      // ⚠️ **Scoped to the zone that owns the row.** This half draws two
      // figures and each has its own settings button, so an unscoped
      // `[data-plot-settings]` matches both — and playwright refuses an
      // ambiguous click rather than picking one, which is the right
      // behaviour and the reason this is spelled out.
      const zone = ["view", "window"].includes(which) ? "trend" : "stats";
      const at = `${half} [data-usage-zone="${zone}"]`;
      const panel = `${at} [data-plot-settings-open]`;
      if ((await page.locator(panel).count()) === 0) {
        await page.locator(`${at} [data-plot-settings]`).click();
        await wait(150);
      }
      await page.locator(`${panel} [data-plot-set="${which}"] [data-plot-axis="${key}"]`).click();
      // Shut it again: the panel stays open across picks (three rows,
      // one visit), so leaving it open would put an overlay over every
      // measurement below.
      await page.locator(`${at} [data-plot-settings]`).click();
      await wait(300);
    };

    const read = () =>
      page.evaluate(() => {
        const el = document.querySelector("[data-quota-detail] [data-usage-half]");
        if (!el) return null;
        const day = el.querySelector('[data-usage-chart="day"]');
        const cols = [...el.querySelectorAll("[data-usage-day]")].map((g) => ({
          day: g.dataset.usageDay,
          total: Number(g.dataset.usageDayTotal),
          // **The drawn heights, not the numbers the product printed
          // beside them.** What is being checked is that the bands add
          // up to the column, and only the geometry can say that.
          heights: [...g.querySelectorAll("[data-usage-band]")].map((r) => Number(r.getAttribute("height"))),
          bands: [...g.querySelectorAll("[data-usage-band]")].map((r) => r.dataset.usageBand),
        }));
        return {
          scale: day ? day.dataset.usageScale : null,
          cols,
          keys: [...el.querySelectorAll("[data-usage-key-model]")].map((k) => k.dataset.usageKeyModel),
          rows: [...el.querySelectorAll("[data-usage-row]")].map((r) => r.dataset.usageRow),
          points: [...el.querySelectorAll("[data-usage-point]")].map((g) => ({
            model: g.dataset.usagePoint,
            r: Number(g.dataset.usagePointR),
          })),
          rulerX: el.querySelector("[data-usage-scatter]")?.dataset.usageRulerX ?? null,
          rulerY: el.querySelector("[data-usage-scatter]")?.dataset.usageRulerY ?? null,
          // The band colours **as the browser resolved them**, beside
          // the stylesheet's own value for the opus hue — see ④.
          paint: Object.fromEntries(
            [...el.querySelectorAll("[data-usage-key-model]")].map((k) => [
              k.dataset.usageKeyModel,
              getComputedStyle(k.querySelector("i")).backgroundColor,
            ]),
          ),
          opusHue: (() => {
            const probe = document.createElement("span");
            probe.style.background = "var(--model-opus)";
            document.body.append(probe);
            const c = getComputedStyle(probe).backgroundColor;
            probe.remove();
            return c;
          })(),
        };
      });

    const all = await read();
    if (all === null) fail("probe dead: no usage half to read");

    //   ② **Every model gets its own band, and a band never rounds to
    //   nothing.** A model folded into 「其余」 and a model that never
    //   ran are the same screen, so the count on the legend, in the
    //   table and in the drawing all have to be the fixture's own —
    //   which includes the unnamed row and the model that ran once.
    const want = [...new Set(usageModels.map((m) => m.model ?? ""))];
    if (all.keys.length !== want.length || all.rows.length !== want.length) {
      fail(
        `the fixture has ${want.length} models (${JSON.stringify(want)}) and the half draws ${all.keys.length} in its legend, ` +
          `${all.rows.length} in its table — a model folded away and a model that never ran are the same screen`,
      );
    }
    const flat = all.cols.flatMap((c) => c.heights);
    if (flat.length === 0 || flat.some((h) => !(h > 0))) {
      fail(`a band was drawn ${Math.min(...flat)} tall — a segment of no height is a model that did not run`);
    }

    //   ③ **The stack's axis is linear, measured rather than read off
    //   the product's own label.** The bands are summed to make the
    //   column, so a bent ruler would draw segments that do not add up
    //   to the shape they sit inside. 08-29 is exactly twice 08-28 in
    //   the fixture, so the drawn heights have to be twice as well —
    //   a square root would put them at √2, which this tolerance
    //   cannot absorb.
    const col = (d) => all.cols.find((c) => c.day === d);
    const tall = col("2026-08-29");
    const half2 = col("2026-08-28");
    if (!tall || !half2) {
      fail(`probe dead: the two days the ratio is taken between are not on the chart (${JSON.stringify(all.cols.map((c) => c.day))})`);
    } else {
      const sum = (c) => c.heights.reduce((a, b) => a + b, 0);
      const drawn = sum(tall) / sum(half2);
      const stated = tall.total / half2.total;
      if (Math.abs(drawn - stated) > 0.02) {
        fail(
          `08-29 is ${stated.toFixed(2)}× 08-28 in the fixture and ${drawn.toFixed(2)}× on the glass ` +
            `(${sum(tall).toFixed(1)}px against ${sum(half2).toFixed(1)}px) — the bands no longer add up to the column they are in`,
        );
      }
      if (all.scale !== "linear") fail(`the stacked chart says it is on a ${all.scale} ruler, and a stack has no meaning on one`);
    }
    //   **The calendar, not the rows.** 09-02 has no spend and
    //   therefore no row; an axis built from the rows would skip it,
    //   and 「7 天」 would silently mean 「the last seven days that had
    //   spend」.
    if (!all.cols.some((c) => c.day === "2026-09-02" && c.heights.length === 0)) {
      fail("09-02 — a day with no spend — is not on the axis as an empty column, so the axis is the rows rather than the calendar");
    }

    //   ④ **A model khor has never seen lands in the right hue the
    //   first time it appears.** `claude-opus-6` is not in any list in
    //   this repo and is one of the smallest readings here, so 「the
    //   next free colour」 and 「the biggest is the newest」 both fail
    //   it. Compared against the **stylesheet's** own value for the
    //   hue, which is a source that does not run through the code
    //   being tested.
    if (all.paint["claude-opus-6"] !== all.opusHue) {
      fail(
        `claude-opus-6 is painted ${all.paint["claude-opus-6"]} while --model-opus resolves to ${all.opusHue} — ` +
          `a model this face has never seen did not land in its family's hue`,
      );
    }
    //   …and the arm that says the check above can fail: an older
    //   generation of the same family must **not** be that colour, or
    //   「everything opus is the opus hue」 would pass it too.
    if (all.paint["claude-opus-5"] === all.opusHue) {
      fail("claude-opus-5 wears the family hue at full strength beside claude-opus-6 — two generations drawn as one band");
    }

    //   ⑤ **Switching the range decides which days are counted and
    //   nothing else.** The days two windows share must report the same
    //   totals to the digit, and a model must not change colour: both
    //   are recalculations dressed as a filter.
    await pickUsage("window", "7");
    const week = await read();
    // **7 columns, and the 7 is only safe because of how this fixture
    // is cut.** The axis never starts before the source's first day —
    // 「no row」 means 「spent nothing」 inside the record and 「nobody
    // has a reading」 before it — so a window longer than the data
    // draws fewer columns than it names. This fixture starts on 08-28
    // and the seven-day window starts on 08-31, so the clamp does not
    // bite. Move the fixture's first day and this number moves with it.
    if (week.cols.length !== 7) {
      fail(`「7 天」 drew ${week.cols.length} columns (${JSON.stringify(week.cols.map((c) => c.day))}) — the window is a number of days`);
    }
    for (const c of week.cols) {
      const was = col(c.day);
      if (was && was.total !== c.total) {
        fail(`${c.day} totals ${c.total} in the 7-day window and ${was.total} in the whole — switching the range recalculated something`);
      }
    }
    for (const [m, paint] of Object.entries(week.paint)) {
      if (all.paint[m] && all.paint[m] !== paint) {
        fail(`${m} is ${all.paint[m]} over the whole span and ${paint} over seven days — a colour is a fact about a model, not about a window`);
      }
    }
    await pickUsage("window", "0");

    const back = await read();
    //   ⑥ **The area carries the value: `r²` is an affine function of
    //   the reading.**
    //
    //   ⚠️ **Stated as the property, because every version of this that
    //   named a number was wrong within the day.** It compared ratios
    //   against an absolute tolerance of 0.02 while the ratios were
    //   0.0016 (an instrument with no resolution); then against a
    //   written-down floor of 5, which stopped being the floor the hour
    //   `dotSpan` made the range depend on the point count; then against
    //   the smallest drawn radius, which is **not** the floor — the
    //   smallest reading is not zero, so that subtracts too much and
    //   the error lands entirely on the small dots.
    //
    //   `r² = floor² + (ceiling² − floor²) · reading/max` is affine in
    //   the reading whatever the two ends are, so fitting a line
    //   through (reading, r²) and checking every dot sits on it tests
    //   the encoding **without knowing either end**.
    //
    //   ⚠️ **This is the only place `dotRadius` is guarded, and the
    //   machine card draws with the same function.** Measured on
    //   2026-09-08 by making the radius linear in the reading: this fit
    //   went red (r² 14.5% off a line, r 0.0% off) and the machine
    //   card's own dot block did not move at all — its endpoints are
    //   1.8 and 9.24 under either encoding, so a min/max reading cannot
    //   tell the two apart. That card is covered **only while both
    //   figures call the one function**; the day one grows its own
    //   sizing, this needs a twin over there.
    const fit = (xs, ys) => {
      const n = xs.length;
      const sx = xs.reduce((a, b) => a + b, 0);
      const sy = ys.reduce((a, b) => a + b, 0);
      const sxx = xs.reduce((a, b) => a + b * b, 0);
      const sxy = xs.reduce((a, b, k) => a + b * ys[k], 0);
      const b1 = (n * sxy - sx * sy) / (n * sxx - sx * sx);
      const b0 = (sy - b1 * sx) / n;
      const span = Math.max(...ys) - Math.min(...ys) || 1;
      return Math.max(...ys.map((y, k) => Math.abs(y - (b0 + b1 * xs[k])))) / span;
    };
    const rowOf = (m) => usageModels.filter((x) => (x.model ?? "") === m).reduce((a, x) => a + x.tokens.input + x.tokens.output, 0);
    const reads = back.points.map((p) => rowOf(p.model));
    if (back.points.length < 3 || new Set(reads).size < 3) {
      fail(`probe dead: ${back.points.length} dots over ${new Set(reads).size} distinct readings — a line needs points that differ`);
    } else {
      const asArea = fit(reads, back.points.map((p) => p.r * p.r));
      const asRadius = fit(reads, back.points.map((p) => p.r));
      if (asArea > 0.02) {
        fail(
          `the dots' r² is ${(100 * asArea).toFixed(1)}% off a straight line against their readings — the **area** is meant to ` +
            `carry the value, and a radius read straight off it makes a 4× value look 16× big`,
        );
      }
      // The arm: if `r` itself were affine in the reading — the bug —
      // this would also be small, and the check above could not tell
      // the two encodings apart.
      if (asRadius <= 0.02) {
        fail(`the dots' radius is itself a straight line against their readings (${(100 * asRadius).toFixed(1)}% off) — that is the encoding this is meant to rule out`);
      }
      console.log(`dot areas: r² ${(100 * asArea).toFixed(2)}% off a line, r ${(100 * asRadius).toFixed(1)}% off`);
    }

    //   **The ruler is printed, and it is chosen rather than fixed.**
    //   A bent axis that does not say so is a chart that lies quietly.
    //   But saying so proves nothing on its own: **two different
    //   rulers have to be reachable in one run**, or what is being
    //   tested is 「it always answers this one」. The machine page's
    //   gate carries the same arm, and this fixture is cut to make it
    //   possible — token counts want a log, the cache rate (0.60–0.86)
    //   wants a linear.
    if (!back.rulerX || !back.rulerY) {
      fail(`the scatter does not say which ruler each axis is on (${back.rulerX} / ${back.rulerY})`);
    }
    await pickUsage("y", "rate");
    const bent = await read();
    if (bent.rulerY === back.rulerY) {
      fail(
        `every axis in this fixture came out on the same ruler (${back.rulerY}) — ` +
          `「it is chosen by measurement」 was not tested, only 「it is always this one」`,
      );
    }
    await pickUsage("y", "read");

    //   ⑦ **The provider road, and its three faces.** Which columns
    //   exist is a property of the *source*, and `cache_split: false`
    //   means those two figures were not reported — **not that they
    //   were nought**, which a column of zeroes would claim.
    const colsOf = async (row) => {
      await press('[data-quota-row="' + row + '"]');
      await wait(350);
      return page.evaluate(() => {
        const el = document.querySelector("[data-quota-detail] [data-usage-half]");
        if (!el) return { none: true };
        return {
          // The sentence, not the whole block: the mute face wears the
          // half's title too, and a log line reading 「花掉的 token这家不
          // 报 token」 is two sentences run together.
          mute: el.hasAttribute("data-usage-mute") ? (el.querySelector("p:not([data-usage-title])")?.textContent ?? "").trim() : null,
          cols: [...el.querySelectorAll("[data-usage-col]")].map((c) => c.dataset.usageCol),
          //   **Whether the missing columns are explained**, added
          //   2026-09-09. Four columns vanishing on one account and not
          //   another is visible; *why* is not, and 「this provider does
          //   not report it」 and 「khor could not read it」 are a fact
          //   and a fault.
          noCache: (el.querySelector("[data-usage-no-cache]")?.textContent ?? "").trim() || null,
        };
      });
    };
    const itemised = await colsOf("acc-number");
    const plain = await colsOf("acc-capped");
    const silent = await colsOf("acc-widest");
    for (const k of ["read", "write", "rate", "reuse"]) {
      if (!itemised.cols?.includes(k)) fail(`acc-number states its cache and the table has no ${k} column (${JSON.stringify(itemised.cols)})`);
      if (plain.cols?.includes(k)) {
        fail(`acc-capped does not report its cache and the table draws a ${k} column anyway — a figure nobody reported is not a zero`);
      }
    }
    //   The two roads' cache answers, and the sentence that tells them
    //   apart. **Both arms**: the account that states its cache must not
    //   wear the sentence either, or it says nothing about `cacheKnown`
    //   and would pass on a page that always printed it.
    if (plain.noCache === null) {
      fail(`acc-capped does not report its cache and the figure's foot says nothing about it — a reader sees four columns gone and cannot tell 「这家不报」 from 「khor 没读到」`);
    }
    if (itemised.noCache !== null) {
      fail(`acc-number states its cache and the foot still says ${JSON.stringify(itemised.noCache)} — that sentence is about a source that does not report it`);
    }
    if (!plain.cols?.includes("total")) {
      fail(`probe dead: acc-capped drew no table at all (${JSON.stringify(plain)}), so 「the cache columns are gone」 measured nothing`);
    }
    if (!silent.mute) {
      fail(`acc-widest reports no tokens at all and the half drew ${JSON.stringify(silent)} — a provider that publishes no token report is not an account that spent nothing`);
    }
    //   ⚠️ **Back to the subscription's card first.** ⑦ above walks
    //   three API accounts and leaves the pane on the one that reports
    //   no tokens at all — a card with a sentence and no table on it,
    //   where every selector below would find nothing and every
    //   assertion would pass by finding zero of what it was looking
    //   for. That is the `querySelectorAll`-returns-`[]` shape, and it
    //   reads as 「the row is not a control」 rather than 「there is no
    //   row」.
    await press('[data-quota-row="sub:local"]');
    await wait(400);
    if ((await page.locator(`${half} [data-usage-row]`).count()) === 0) {
      fail("probe dead: no rows on the subscription's card, so nothing about heads or pressing was measured");
    }

    //   ⑧ **Every head says it can be sorted, not just the sorted
    //   one.** `aria-sort="none"` is what makes a row of heads read as
    //   a row of controls; marking only the active column leaves 「these
    //   can be sorted」 unsaid. The machine card's heads all carry it.
    const sorts = await page.evaluate((sel) =>
      [...document.querySelectorAll(`${sel} th[data-usage-col]`)].map((h) => h.getAttribute("aria-sort")), half);
    if (sorts.length === 0) fail("probe dead: no column heads, so nothing about their sort state was read");
    else if (sorts.some((v) => v === null)) {
      fail(`${sorts.filter((v) => v === null).length} of ${sorts.length} column heads carry no aria-sort — 「these columns can be sorted」 is left unsaid`);
    } else if (sorts.filter((v) => v !== "none").length !== 1) {
      fail(`${sorts.filter((v) => v !== "none").length} heads claim to be the sorted one (${JSON.stringify(sorts)}) — exactly one column is`);
    }

    //   ⑨ **Pressing a point locates a row; it does not select one.**
    //   The machine card's gesture, and this half had the other one:
    //   point *or* row toggled a tint that stayed. Three parts, and the
    //   arms that say each can fail:
    //
    //     - the row is marked when a point is pressed;
    //     - **the mark takes itself off** — a mark that stays is a
    //       selection, and this checks it is gone a second later;
    //     - **the table is not a control** — pressing a row marks
    //       nothing, which is the direction that was removed.
    const hotNow = () => page.evaluate((sel) => ({
      hot: [...document.querySelectorAll(`${sel} [data-usage-row][data-hot]`)].map((r) => r.dataset.usageRow),
      rowCursor: getComputedStyle(document.querySelector(`${sel} [data-usage-row]`)).cursor,
    }), half);
    await page.locator(`${half} [data-usage-point="claude-opus-5"]`).click();
    await wait(200);
    const lit = await hotNow();
    if (lit.hot.length !== 1 || lit.hot[0] !== "claude-opus-5") {
      fail(`pressing claude-opus-5's dot marked ${JSON.stringify(lit.hot)} — a point that cannot name its row is a dot`);
    }
    await wait(1600);
    const faded = await hotNow();
    if (faded.hot.length !== 0) {
      fail(`the mark on ${JSON.stringify(faded.hot)} is still there 1.8s later — a highlight that stays is a selection, and a selection has to be undone`);
    }
    if (lit.rowCursor === "pointer") {
      fail(`the table's rows still answer the finger (cursor: ${lit.rowCursor}) — the row is where an answer lands, not a control`);
    }
    await page.locator(`${half} [data-usage-row="claude-fable-5"]`).click();
    await wait(300);
    if ((await hotNow()).hot.length !== 0) {
      fail("pressing a row marked it — this half is 「a point locates a row」, one way, like the machine card");
    }

    //   ⑩ **The two surfaces are one product, measured in one run.**
    //
    //   2026-09-08, from the person using it: 「和机器页的交互、元素统
    //   一。现在看起来是两个产品。」 Measured that day, this half's
    //   blocks were flat sections — square corners, no border, no card
    //   ground, 16px at the sides — against the machine card's panels
    //   at 8px radius, a hairline border, `--card`, and 12 all round.
    //   **They shared no value.**
    //
    //   So this reads **the card each figure is actually drawn on** —
    //   walking up from the drawing to the first bordered, non
    //   transparent ancestor, rather than looking for a marker the
    //   component would have to put there. A marker would be the
    //   product describing itself; this is the screen.
    //
    //   ⚠️ **It must not be able to pass by finding nothing.** Two
    //   surfaces that both draw no card agree perfectly, and so do two
    //   heads that are both zero tall — the `querySelectorAll`-returns
    //   -`[]` family. So the readings are checked for being real
    //   before they are checked for being equal.
    const cardOf = (sel) =>
      page.evaluate((s) => {
        const start = document.querySelector(s);
        if (!start) return null;
        let n = start;
        while (n && n !== document.body) {
          const cs = getComputedStyle(n);
          if (cs.borderTopWidth !== "0px" && cs.backgroundColor !== "rgba(0, 0, 0, 0)") {
            return {
              radius: cs.borderTopLeftRadius,
              border: cs.borderTopWidth,
              pad: [cs.paddingTop, cs.paddingRight, cs.paddingBottom, cs.paddingLeft].join(" "),
              bg: cs.backgroundColor,
            };
          }
          n = n.parentElement;
        }
        return null;
      }, sel);
    //   **A column head, as the reader sees it.** Height was the only
    //   thing compared until 2026-09-09, and the two heads differed in
    //   three more: the ink, the padding, and **which element carries
    //   the rule under them** (one draws it on the cell, one on the row
    //   above the cells).
    //
    //   ⚠️ These are read on two screens, one after the other, and that
    //   is only allowed because none of them is a layout reading: a
    //   colour, a weight and a padding are properties of the element
    //   wherever it is drawn. **The height stays a same-state reading**
    //   — it is the one that moves with the box.
    const headOf = (sel) =>
      page.evaluate((s) => {
        const b = document.querySelector(s);
        if (!b) return null;
        const cs = getComputedStyle(b);
        const cell = b.closest("th") ?? b.parentElement;
        const cc = cell ? getComputedStyle(cell) : null;
        const n = (v) => Math.round(parseFloat(v) * 10) / 10;
        return {
          h: +b.getBoundingClientRect().height.toFixed(1),
          ink: cs.color.replace(/\s/g, ""),
          weight: cs.fontWeight,
          pad: `${n(cs.paddingTop)},${n(cs.paddingRight)},${n(cs.paddingBottom)},${n(cs.paddingLeft)}`,
          cellPad: cc ? `${n(cc.paddingTop)},${n(cc.paddingRight)},${n(cc.paddingBottom)},${n(cc.paddingLeft)}` : null,
          cellRule: cc ? n(cc.borderBottomWidth) : null,
        };
      }, sel);
    //   ⚠️ **The ink, not the box.** The first draft compared the two
    //   svg widths and reported 169 against 158 — **a difference that
    //   is not the one the user is talking about**: the 額度 dial's box
    //   is the plain square (`DIAL_CARD_RATIO` 1) and the machine
    //   card's is cropped sideways to its ink (`hugRatio`, 2026-09-08,
    //   so that the gap a reader sees is the gap the row states). Same
    //   drawing, two boxes around it. 「就是现在额度页的大小」 is about
    //   the ring, so what is compared is the arc the reader sees and
    //   the weight it is drawn at.
    const inkOf = (sel) =>
      page.evaluate((s) => {
        const el = document.querySelector(s);
        if (!el) return null;
        const track = el.querySelector("[data-ring-track]") ?? el.querySelector("path");
        if (!track || !track.getBBox) return null;
        const b = track.getBBox();
        return {
          arc: +b.width.toFixed(1),
          stroke: +parseFloat(getComputedStyle(track).strokeWidth || track.getAttribute("stroke-width") || "0").toFixed(2),
        };
      }, sel);
    //   **The two pages' dials are one dial** (2026-09-09, the user:
    //   「机器卡片环的大小,就是现在额度页的大小」). Read off the glass
    //   on both surfaces rather than by comparing two constants: the
    //   pair that decides it is `DIAL_CARD_SIZE` / `DIAL_CARD_STROKE`,
    //   and a machine card that took the size from there while keeping
    //   a stroke of its own would draw the same circle at a different
    //   weight — which is what 「一样大」 does not mean.
    //
    //   ⚠️ **Each read on its own screen**, and this one first: the
    //   walk leaves this pane in a moment, and a reading taken after
    //   that would be a reading of nothing.
    const mineDial = await inkOf("[data-quota-detail] [data-quota-lead] svg");
    const mine = await cardOf(`${half} [data-usage-chart]`);
    const mineHead = await headOf(`${half} th[data-usage-col] button`);
    //   **How a figure is drawn, measured off the glass on both pages.**
    //
    //   2026-09-08, the user: 「x和y的label和机器对不上啊,位置、样式.
    //   …二者统一出一个绘图方法,包括样式、字号等.」 Both pages already
    //   used the same *control* (`components/PlotControls`, then still
    //   named `AxisPick`); what disagreed was the coordinate system it
    //   was placed into — one figure was
    //   drawn at one unit per pixel, the other into a scaled `viewBox`,
    //   so the second had to approximate the first's pixel offsets as
    //   percentages (`6.59%` is `58 / 880`).
    //
    //   ⚠️ **What this can and cannot prove.** It compares the two
    //   figures with each other, so **editing `lib/plot` moves both and
    //   stays green** — which is right, that is an intended change, not
    //   a regression. What it catches is the thing that actually went
    //   wrong: **one page computing its own**. The teardown for 「one
    //   place decides」 is therefore to give one page its own number and
    //   watch this go red, not to edit the shared one.
    const drawnAt = (svg, tickGroup, tickText, pickSel, seatSel, xPickSel) =>
      page.evaluate(
        ([svgSel, group, word, pick, seat, xpick]) => {
          const el = document.querySelector(svgSel);
          if (!el) return null;
          const frame = el.getBoundingClientRect();
          const ticks = [...document.querySelectorAll(group)];
          const first = document.querySelector(word);
          const label = document.querySelector(pick);
          //   **Where the settings button sits, in the figure's own
          //   frame.** This is the one the user pointed at
          //   (2026-09-09:「为什么机器和余量页三个点的位置还不一样」),
          //   and until that day the 額度 trend chart's button was not
          //   in a figure at all — it was in a row underneath.
          const btn = seat ? document.querySelector(seat) : null;
          //   ⚠️ **Both ends, because the seats are not all measured
          //   from the same one.** `labelSeat` puts the y label at a
          //   `top` and the x label and the settings button at a
          //   `bottom` — so comparing two figures of different heights
          //   by distance-from-the-top says they disagree when they are
          //   obeying the same rule. Measured 2026-09-09: 148 / 228 /
          //   180 from the top for three figures whose seats are
          //   identical. **`up` is the reading that matches the rule.**
          const at = (n) => {
            const r = n.getBoundingClientRect();
            return {
              x: +(r.left - frame.left).toFixed(1),
              y: +(r.top - frame.top).toFixed(1),
              up: +(frame.bottom - r.bottom).toFixed(1),
              back: +(frame.right - r.right).toFixed(1),
            };
          };
          const xLab = xpick ? document.querySelector(xpick) : null;
          return {
            settings: btn ? at(btn) : null,
            xLabel: xLab ? at(xLab) : null,
            // The tick word's ink size, and where its gutter ends —
            // both are what the reader sees, not what was imported.
            font: first ? getComputedStyle(first).fontSize : null,
            gutter: first ? +(first.getBoundingClientRect().right - frame.left).toFixed(1) : null,
            ticks: ticks.length,
            // The axis label's seat, relative to the drawing's own box.
            label: label
              ? {
                  x: +(label.getBoundingClientRect().left - frame.left).toFixed(1),
                  y: +(label.getBoundingClientRect().top - frame.top).toFixed(1),
                }
              : null,
          };
        },
        [svg, tickGroup, tickText, pickSel, seatSel ?? null, xPickSel ?? null],
      );
    //   ⚠️ **Scoped to the zone, not to the half.** This half draws two
    //   figures and both carry `data-usage-tick-x` since 2026-09-09 —
    //   an unscoped selector would count one figure's ticks while
    //   measuring the other's frame, which is the two-readings-one-state
    //   mistake with the state left out of the reading.
    const stats = `${half} [data-usage-zone="stats"]`;
    const trend = `${half} [data-usage-zone="trend"]`;
    const drawnMine = await drawnAt(
      `${stats} [data-usage-scatter]`,
      `${stats} [data-usage-tick-x]`,
      `${stats} [data-usage-tick-y] text`,
      `${stats} [data-plot-pick="y"]`,
      `${stats} [data-plot-settings]`,
      `${stats} [data-plot-pick="x"]`,
    );
    //   **The bar chart is the third figure, and it is the one the user
    //   was looking at.** It has no y label — its vertical axis is a
    //   count — so what is compared here is the tick ink, the gutter
    //   and **where the settings button sits**.
    const drawnBars = await drawnAt(
      `${trend} [data-usage-chart]`,
      `${trend} [data-usage-tick-x]`,
      `${trend} [data-usage-tick-y] text`,
      `${trend} [data-plot-pick="view"]`,
      `${trend} [data-plot-settings]`,
      `${trend} [data-plot-pick="view"]`,
    );
    const mineFoot = (await page.locator(`${half} [data-plot-foot]`).count()) > 0;
    await open("devices");
    await press("[data-list] [data-row-open]");
    await until(8_000, async () => (await page.locator("[data-machine-card] [data-top-plot]").count()) > 0);
    await wait(400);
    const theirs = await cardOf("[data-machine-card] [data-top-plot]");
    const theirsHead = await headOf("[data-machine-card] [data-top-head] button");
    const drawnTheirs = await drawnAt(
      "[data-machine-card] [data-plot-svg]",
      "[data-machine-card] [data-plot-tick-x]",
      "[data-machine-card] [data-plot-tick-y] text",
      '[data-machine-card] [data-plot-pick="y"]',
      "[data-machine-card] [data-plot-settings]",
      '[data-machine-card] [data-plot-pick="x"]',
    );
    if (!mine || !theirs || !mineHead || !theirsHead) {
      fail(
        `probe dead: cards ${JSON.stringify([mine, theirs])} heads ${JSON.stringify([mineHead, theirsHead])} — ` +
          `two surfaces were not both measured, so 「they are one product」 compared nothing`,
      );
    } else if (parseFloat(theirs.radius) <= 0 || parseFloat(theirs.border) <= 0 || !(theirsHead.h > 0)) {
      fail(
        `the machine card's own block reads radius ${theirs.radius}, border ${theirs.border}, head ${theirsHead} — ` +
          `a reference with nothing in it makes every comparison below pass by agreeing about nothing`,
      );
    } else {
      for (const k of ["radius", "border", "pad", "bg"]) {
        if (mine[k] !== theirs[k]) {
          fail(
            `the 額度 card's usage block has ${k} ${mine[k]} and the machine card's block has ${theirs[k]} — ` +
              `these are one product's two surfaces, and 「看起来是两个产品」 is what a reader calls this. ` +
              `**Both come from \`components/Panel\`; if that is meant to change, change the component** — ` +
              `a value edited on one side is the drift this measures.`,
          );
        }
      }
      if (Math.abs(mineHead.h - theirsHead.h) > 0.5) {
        fail(`a column head is ${mineHead.h} tall on the 額度 card and ${theirsHead.h} on the machine card — one table, two rows of controls`);
      }
      for (const k of ["ink", "weight", "pad", "cellPad", "cellRule"]) {
        if (String(mineHead[k]) !== String(theirsHead[k])) {
          fail(
            `a column head's ${k} is ${JSON.stringify(mineHead[k])} on the 額度 card and ${JSON.stringify(theirsHead[k])} on the ` +
              `machine card — one row of controls, drawn twice. 「看起来是两个产品」 is what a reader calls a difference this size.`,
          );
        }
      }
      console.log(`one product: card ${JSON.stringify(theirs)}, head ${JSON.stringify(theirsHead)} on both`);
    }
    //   …and the drawing itself, on the same run.
    if (!drawnMine || !drawnTheirs) {
      fail(`probe dead: one of the two figures was not found (${JSON.stringify([drawnMine, drawnTheirs])}), so 「one drawing method」 compared nothing`);
    } else if (!(drawnTheirs.ticks >= 2) || !drawnTheirs.font || !drawnTheirs.label) {
      // **The arm this repo has already been bitten by**: two figures
      // that both drew nothing agree perfectly. The reference has to be
      // real before the comparison means anything.
      fail(
        `the machine card's figure reads ${JSON.stringify(drawnTheirs)} — a reference with no ticks, no ink and no label ` +
          `makes every comparison below pass by agreeing about nothing`,
      );
    } else {
      for (const [k, mineV, theirsV] of [
        ["tick ink", drawnMine.font, drawnTheirs.font],
        ["tick count", drawnMine.ticks, drawnTheirs.ticks],
        ["y-tick gutter", drawnMine.gutter, drawnTheirs.gutter],
        ["y label x", drawnMine.label?.x, drawnTheirs.label?.x],
        ["y label y", drawnMine.label?.y, drawnTheirs.label?.y],
        //   ⚠️ **Read from the corner the seat is anchored to.** The
        //   button moved twice on 2026-09-10 — bottom-left, top-right,
        //   then top-left — and each move changes which two numbers are
        //   the rule. Anchored to a far corner, distance-from-the-left
        //   is the figure's own width and distance-from-the-bottom is
        //   its own height: while it sat top-right these two read
        //   940 vs 932 and 234 vs 154, and this gate said two figures
        //   disagreed about a rule they were both keeping. **The claim
        //   never changed (one seat, one module); the axes have to
        //   match where the seat is pinned.**
        ["settings seat x", drawnMine.settings?.x, drawnTheirs.settings?.x],
        ["settings seat y", drawnMine.settings?.y, drawnTheirs.settings?.y],
        ["x label up", drawnMine.xLabel?.up, drawnTheirs.xLabel?.up],
        //   **The third figure.** The bar chart's controls sat in a row
        //   under it until 2026-09-09 — the same control in the same
        //   product with two ideas of where it lives, which is what
        //   「为什么机器和余量页三个点的位置还不一样」 names. Its tick
        //   ink and gutter are here for the same reason: one drawing
        //   method has to reach the figure the reader opens first.
        ["bar tick ink", drawnBars?.font, drawnTheirs.font],
        ["bar y-tick gutter", drawnBars?.gutter, drawnTheirs.gutter],
        ["bar settings seat x", drawnBars?.settings?.x, drawnTheirs.settings?.x],
        ["bar settings seat y", drawnBars?.settings?.y, drawnTheirs.settings?.y],
        ["bar x label up", drawnBars?.xLabel?.up, drawnTheirs.xLabel?.up],
      ]) {
        const same = typeof mineV === "number" && typeof theirsV === "number" ? Math.abs(mineV - theirsV) <= 0.6 : mineV === theirsV;
        if (!same) {
          fail(
            `the two figures disagree about ${k}: 額度 ${JSON.stringify(mineV)}, machine ${JSON.stringify(theirsV)} — ` +
              `**both draw through \`lib/plot\`; a number that differs is a page computing its own.** ` +
              `If the drawing method is meant to change, change the module and both move together.`,
          );
        }
      }
      //   **The foot, by one name on both.** It said the same sentence
      //   from the same key on both pages since 2026-09-08 and still
      //   wore two different marks and sat on opposite sides — which is
      //   how a gate reading 「one drawing method」 could be green while
      //   a reader saw two feet.
      //   ⚠️ **Each foot is read on its own screen.** The first draft
      //   asked for both in one `evaluate` at this point — and by here
      //   the walk has left the 額度 pane, so it read `mine: false` on a
      //   card that was not on the glass. Two readings, two states, one
      //   comparison: the mistake this file keeps catching.
      //   ⚠️ **Reversed 2026-09-10.** This asserted both figures carry
      //   the 「面积 = X」 sentence, from one key, findable by one name.
      //   The sentence is gone from both (「所有散点,无论机器还是api,
      //   都不要有下面的面积注释」) — and the claim that mattered was
      //   never 「it is there」, it was **「whatever one figure does,
      //   the other does」**. So it stands, pointing the other way: one
      //   of them growing a foot back alone is the drift this catches.
      const feet = { mine: mineFoot, theirs: await page.locator("[data-machine-card] [data-plot-foot]").count() > 0 };
      if (feet.mine || feet.theirs) {
        fail(`a figure still carries the area sentence ${JSON.stringify(feet)} — it left both on 2026-09-10, and one figure keeping it is the drift 「统一出一个绘图方法」 is about`);
      }
      if (!drawnBars || !(drawnBars.ticks >= 2)) {
        fail(`probe dead: the 額度 bar chart reads ${JSON.stringify(drawnBars)} — a figure with no ticks agrees with anything`);
      }
      console.log(`one drawing: ${JSON.stringify(drawnTheirs)} · bars ${JSON.stringify(drawnBars)}`);
    }
    //   **The two pages' dials are one dial** (2026-09-09, the user:
    //   「机器卡片环的大小,就是现在额度页的大小」).
    //
    //   ⚠️ **On the second machine, because the first one refuses.**
    //   This section opens `[data-row-open]` at index 0 — the machine
    //   whose answer is a sentence — and that card has no dial row at
    //   all. The first draft read it there and got `null`, which the
    //   probe-dead arm caught: a comparison against nothing.
    await pressNth("[data-list] [data-row-open]", 1);
    await until(5_000, async () => (await page.locator("[data-machine-card] [data-vitals]").count()) > 0);
    await wait(300);
    const theirsDial = await inkOf("[data-machine-card] [data-vitals] svg");
    if (!mineDial || !theirsDial || !(mineDial.arc > 0) || !(theirsDial.arc > 0)) {
      fail(`probe dead: dials read ${JSON.stringify([mineDial, theirsDial])} — 「the two pages draw one dial」 compared nothing`);
    } else {
      for (const k of ["arc", "stroke"]) {
        if (Math.abs((mineDial[k] ?? -1) - (theirsDial[k] ?? -2)) > 0.1) {
          fail(
            `a dial is ${JSON.stringify(mineDial[k])} on the 額度 card and ${JSON.stringify(theirsDial[k])} on the machine ` +
              `card (${k}) — the user's word for these two is 「就是现在额度页的大小」, and a stroke that differs makes it ` +
              `the same circle at a different weight`,
          );
        }
      }
      console.log(`one dial: ${JSON.stringify(theirsDial)} on both pages`);
    }
    await open("quota");
    await until(5_000, async () => (await page.locator('[data-quota-row="sub:local"]').count()) > 0);
    await press('[data-quota-row="sub:local"]');
    await wait(400);
    //   ⑪ **Nothing on this card is spaced 16 or 32 any more.**
    //
    //   2026-09-08, the user: 額度's detail is to be on the machine
    //   card's ladder. That ladder is 8 / 12 / 18 / 24 (`--gap-in`,
    //   `-row`, `-group`, `-pane`); **16 and 32 are not rungs on it**,
    //   and they were what this page was built out of — `p-4`, `gap-4`,
    //   `gap-x-8`.
    //
    //   ⚠️ **This is a ratchet, not a ladder check.** Measured the same
    //   day, matched scope (`[data-quota-detail]` against
    //   `[data-machine-card]`): the machine card has **no** off-ladder
    //   value inside it at all, while this detail still has 4px and 6px
    //   ones — the table's `px-1 py-1` cells, mostly, because that card
    //   has no padded cells to begin with (its rows are a grid whose
    //   columns are separated by `--gap-in`). **Asserting the whole
    //   ladder would demand that table be rebuilt**, which is a
    //   different piece of work; this holds the line that was actually
    //   crossed and drawn back.
    {
      const off = await page.evaluate(() => {
        const out = [];
        // ⚠️ **The container itself is in the list, not just its
        // descendants.** `[data-quota-detail] *` reads only children —
        // and the padding the user's ruling was most about sits on
        // that very element (`p-4`). Measured 2026-09-08 by putting
        // `p-4` back: **the gate stayed green.** An assertion blind to
        // its own subject passes for the same reason an empty
        // `querySelectorAll` does.
        for (const el of document.querySelectorAll("[data-quota-detail], [data-quota-detail] *")) {
          const cs = getComputedStyle(el);
          const hit = [
            ["padding-top", cs.paddingTop], ["padding-right", cs.paddingRight],
            ["padding-bottom", cs.paddingBottom], ["padding-left", cs.paddingLeft],
            ["row-gap", cs.rowGap], ["column-gap", cs.columnGap],
          ].filter(([, v]) => v === "16px" || v === "32px");
          if (hit.length) {
            out.push(`${[...el.attributes].map((a) => a.name).find((n) => n.startsWith("data-")) ?? el.tagName}: ${hit.map(([k, v]) => `${k} ${v}`).join(", ")}`);
          }
        }
        return out;
      });
      // The arm that says this can find anything at all: an assertion
      // that only ever counts zero is one nobody can tell from a
      // selector that stopped matching.
      const seen = await page.evaluate(() => document.querySelectorAll("[data-quota-detail], [data-quota-detail] *").length);
      if (seen < 20) fail(`probe dead: only ${seen} elements under the 額度 detail, so 「nothing is spaced 16 or 32」 measured almost nothing`);
      else if (off.length) {
        fail(`${off.length} places on the 額度 detail are still spaced 16 or 32 — the ladder is 8/12/18/24: ${JSON.stringify(off)}`);
      } else {
        console.log(`ladder: 0 of ${seen} elements under the 額度 detail are spaced 16 or 32`);
      }
    }


    console.log(
      `usage half: ${all.keys.length} models over ${all.cols.length} days (7-day window ${week.cols.length}), ` +
        `stack ${all.scale}, scatter ${back.rulerX}×${back.rulerY} (rate axis ${bent.rulerY}), ` +
        `cache columns ${itemised.cols.length} vs ${plain.cols.length}, no-report says ${JSON.stringify(silent.mute)}, ` +
        `heads ${JSON.stringify(sorts)}`,
    );
    await press('[data-quota-row="sub:local"]');
    await wait(300);
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

  // ── 8c) narrow 额度: one screen at a time, with a way back ──────
  // 2026-09-06: the landing became an inbox, so on a phone it takes the
  // inbox posture — list screen, detail screen, `data-back`. It used to
  // be both stacked in one scroller, and **that is what the middle
  // number below is about**: with the list still on the detail screen a
  // 「there is a back button」 check passes on the old layout too. The
  // rows going to zero is the claim; the back button is only the exit
  // it needs.
  // (Still at 390 from 8b, and reached as a switch so the list is not
  // collapsed.)
  await open("quota");
  await until(5_000, async () => (await page.locator("[data-quota-row]").count()) > 0);
  const narrowList = await page.locator("[data-quota-row]").count();
  const backOnList = await page.locator("[data-quota-detail]").count();
  if (narrowList !== listed || backOnList !== 0) {
    fail(`probe dead: narrow 额度 opened on ${narrowList} rows and ${backOnList} details — the list screen is not what it opened on`);
  } else {
    await press('[data-quota-row][data-quota-kind="sub"]');
    const onDetail = {
      rows: await page.locator("[data-quota-row]").count(),
      detail: await page.locator("[data-quota-detail]").count(),
      back: await page.locator("[data-back]").count(),
      windows: await page.locator("[data-quota-detail] [data-quota-window]").count(),
    };
    if (onDetail.detail !== 1 || onDetail.windows !== quotaWindows.length) {
      fail(`probe dead: pressing a narrow 额度 row drew ${onDetail.detail} details / ${onDetail.windows} windows — nothing below is measuring a detail screen`);
    } else if (onDetail.rows !== 0) {
      fail(`the narrow 额度 detail is on the same screen as ${onDetail.rows} list rows — a phone shows one screen at a time`);
    } else if (onDetail.back !== 1) {
      fail(`the narrow 额度 detail draws ${onDetail.back} ways back — it is the only exit from that screen`);
    } else {
      await press("[data-back]");
      const backHome = {
        rows: await page.locator("[data-quota-row]").count(),
        detail: await page.locator("[data-quota-detail]").count(),
      };
      if (backHome.rows !== listed || backHome.detail !== 0) {
        fail(`pressing back left ${backHome.rows} rows and ${backHome.detail} details, not the list it came from`);
      }
      console.log(
        `narrow 额度: list ${narrowList} rows → detail ${onDetail.rows} rows + ${onDetail.windows} windows + ${onDetail.back} back → list ${backHome.rows} rows`,
      );
    }
  }

  // ── 8d) narrow 额度: the table says when it has cut a column off ──
  // 用户 2026-09-10 chose the wordless answer (「丁」) out of four: no
  // sentence, a veil at the edge that is cut. The whole risk of a
  // wordless mark is that it says the wrong thing silently — a veil
  // stuck on after the last column has scrolled into view is a lie
  // nothing corrects — so this measures the marks rather than the paint.
  //
  // **Three arms, and the third is the one that matters.** Cut on the
  // right, cut on the left after scrolling to the end, and **nothing
  // cut when the table fits**: without that last one a mark that is
  // simply always on passes the first two, which is this repo's ninth
  // false green (对照组的两条胳膊被造得必然一致).
  {
    await press('[data-quota-row][data-quota-kind="sub"]');
    await wait(400);
    const cut = '[data-quota-detail] [data-usage-zone="stats"] [data-side-cut]';
    if ((await page.locator(cut).count()) !== 1) {
      fail("probe dead: the stats table is not in a side-cut frame, so nothing below measured a cut edge");
    } else {
      const marks = () =>
        page.evaluate((sel) => {
          const el = document.querySelector(sel);
          const box = el.firstElementChild;
          return {
            left: el.hasAttribute("data-cut-left"),
            right: el.hasAttribute("data-cut-right"),
            over: box.scrollWidth - box.clientWidth,
          };
        }, cut);
      const shove = (to) =>
        page.evaluate(
          ([sel, x]) => {
            const box = document.querySelector(sel).firstElementChild;
            box.scrollLeft = x === "end" ? box.scrollWidth : 0;
          },
          [cut, to],
        );

      const atRest = await marks();
      if (atRest.over <= 1) {
        fail(`probe dead: the stats table fits in 390px (${atRest.over}px over), so there is no cut edge to measure`);
      } else if (!atRest.right || atRest.left) {
        fail(`a table cut off on the right marks left=${atRest.left} right=${atRest.right} — the veil is on the wrong edge, and it has no words to correct itself with`);
      } else {
        await shove("end");
        await wait(200);
        const atEnd = await marks();
        if (atEnd.right || !atEnd.left) {
          fail(`scrolled to the last column the table still marks right=${atEnd.right} left=${atEnd.left} — a veil that stays says there is more when there is not`);
        } else {
          // The control arm: wide enough that nothing is cut, and
          // **still inside the narrow face**. A mark that cannot go
          // away passes both arms above, so this is the one that has
          // to be reachable — and it was not, the first four ways it
          // was torn down. 719 is the breakpoint (`use-narrow`); going
          // out to 1400 crosses it, rebuilds the whole subtree, and
          // remounts the very thing under test, so any stuck mark is
          // wiped by the remount rather than caught here. 700 keeps the
          // same mounted table and merely stops cutting it.
          await page.setViewportSize({ width: 700, height: 900 });
          await wait(400);
          const wide = await marks();
          if (wide.over > 1) {
            fail(`probe dead: the stats table is still ${wide.over}px too wide at 700 — the control arm never got a table that fits`);
          } else if (wide.left || wide.right) {
            fail(`a table with nothing cut off still marks left=${wide.left} right=${wide.right} — the veil is painted on, not measured`);
          } else {
            console.log(
              `side cut: 390px over by ${atRest.over} → right only; scrolled to end → left only; 700px fits → neither`,
            );
          }
          await page.setViewportSize({ width: 390, height: 844 });
          await wait(300);
        }
      }
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
              bent: lineEl.dataset.bent === "1",
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
            if (on) {
              line.style.width = "3px";
              line.dataset.bent = "1";
            } else {
              line.style.removeProperty("width");
              delete line.dataset.bent;
            }
            return true;
          }, on);
        //   **Read off the cell that was squeezed, and squeeze again if
        //   it is gone.** The card re-asks for its table every few
        //   seconds, and an answer landing between the squeeze and the
        //   reading can hand the row a fresh line element — one without
        //   the 3px on it. Measured 2026-09-06: one run in six read
        //   「3 characters」 here, which is the *unsqueezed* tightest
        //   cell, and the probe called itself dead. The mark says which
        //   cell wore the squeeze; a reading with no marked cell is
        //   retried rather than judged.
        let bent = null;
        for (let go = 0; go < 4 && bent === null; go += 1) {
          if (!(await bend(true))) break;
          const squeezed = (await cmdCells()).filter((c) => c.bent);
          await bend(false);
          if (squeezed.length) bent = squeezed[0];
        }
        if (bent === null) {
          fail("probe dead: no 「命令」 line to squeeze (or none stayed squeezed long enough to read), so 「the name is always there」 was never shown to be checkable");
        } else if (!(bent.chars < 1)) {
          fail(`probe dead: a cell squeezed to three pixels still reported ${bent.chars} characters of its name — the check below cannot fail`);
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
  //     ── a card column, once filled, is not taken back ────────────
  //
  //     2026-09-05: 「A column the machine cannot fill does not exist」
  //     used to be asked of the *machine* — 「does it report video
  //     memory」 — and that is the wrong level: turing reports two
  //     RTX A5000s and not one of its two hundred processes carries a
  //     per-process figure, so the table drew two empty columns and
  //     spent 136px on them. The question is 「can any row fill me」.
  //
  //     **Asked of the rows, the answer changes with the page** — the
  //     pid ordering returns boot daemons, none of them on a card — so
  //     it is latched: seen once on this machine, kept until the
  //     machine changes. The section above already refuses a column set
  //     that moves under a press; this one refuses the other half, the
  //     one a latch is *for*: a page with no readings in it must not
  //     retract what an earlier page showed.
  //
  //     The fixture's pid route is exactly that page — it nulls
  //     `gpu_pct`, `gpu_mem` and `run_secs` on every row — so the walk
  //     is: land on the busiest ordering (readings present), press pid
  //     (none present), and require the columns to be the same set.
  {
    const cols = () =>
      page.evaluate(() =>
        [...document.querySelectorAll("[data-machine-card] [data-top-heads] > *")]
          .map((n) => n.dataset.topHead ?? n.dataset.c ?? "?")
          .join(","),
      );
    const readings = () =>
      page.evaluate(() => ({
        gpu: [...document.querySelectorAll("[data-machine-card] [data-top-row] [data-c='gpu']")].filter(
          (n) => n.textContent.trim(),
        ).length,
        vram: [...document.querySelectorAll("[data-machine-card] [data-top-row] [data-c='vram']")].filter(
          (n) => n.textContent.trim(),
        ).length,
      }));
    await open("devices");
    await pressNth("[data-list] [data-row-open]", 1);
    await until(8_000, async () => (await page.locator("[data-machine-card] [data-top-heads] > *").count()) > 3);
    await wait(500);
    const withReadings = { cols: await cols(), n: await readings() };
    //   **Every column the table draws has ink in it somewhere**, and
    //   this is the one stop where the claim is fair: the rows routed
    //   here are the full fixture, so a blank column is the table's
    //   fault rather than the page's. (Below, the same section
    //   deliberately routes rows with nothing in the card columns —
    //   which is what the latch is for, and what makes 「no ink」 the
    //   expected answer there.)
    //
    //   The column set is latched off 「has any row ever filled this」,
    //   and that question is asked of the **wire fields** — so a
    //   fixture that has drifted from the Rust struct answers 「yes」
    //   with `undefined`, which is neither `null` nor a reading.
    //   Measured 2026-09-05: 状态 / USER / CPU时间 / 读写 appeared, were
    //   sized, took width from the table, and drew **nothing**, and
    //   every other assertion about them passed.
    const ink = await page.evaluate(() => {
      const out = {};
      for (const h of document.querySelectorAll("[data-machine-card] [data-top-heads] > *")) {
        const key = h.dataset.topHead ?? h.dataset.c;
        const cells = [...document.querySelectorAll(`[data-machine-card] [data-top-row] [data-c="${key}"]`)];
        out[key] = {
          filled: cells.filter((c) => c.textContent.trim() !== "").length,
          of: cells.length,
          first: cells.slice(0, 3).map((c) => c.textContent),
        };
      }
      return out;
    });
    const blank = Object.entries(ink).filter(([, v]) => v.of > 0 && v.filled === 0);
    if (!Object.keys(ink).length) {
      fail("probe dead: no column heads, so 「every column has ink」 measured nothing");
    } else if (blank.length) {
      fail(
        `the columns ${blank.map(([k]) => k).join(", ")} are drawn and no row fills them ` +
          `(${blank.map(([k, v]) => `${k}: ${v.of} cells, first three ${JSON.stringify(v.first)}`).join("; ")}) — ` +
          `a column the table shows and never fills takes width from the ones that speak, ` +
          `and it usually means the stand-in rows have drifted from the wire struct`,
      );
    } else {
      console.log(`column ink: ${Object.entries(ink).map(([k, v]) => `${k} ${v.filled}/${v.of}`).join(" ")}`);
    }
    if (!/\bvram\b/.test(withReadings.cols) && !/\bgpu\b/.test(withReadings.cols)) {
      fail(
        `probe dead: the card opened with columns [${withReadings.cols}] and no card column among them, ` +
          `so 「a filled column is not taken back」 has nothing to take back`,
      );
    } else if (withReadings.n.gpu + withReadings.n.vram === 0) {
      fail(
        `probe dead: the card columns are drawn but ${withReadings.n.gpu}+${withReadings.n.vram} rows fill them — ` +
          `the latch was never set by a real reading, so what follows tests nothing`,
      );
    } else {
      await page.locator('[data-machine-card] [data-top-head="pid"] button').click();
      await wait(900);
      const after = { cols: await cols(), n: await readings() };
      if (after.n.gpu + after.n.vram !== 0) {
        fail(
          `probe dead: the pid ordering still fills ${after.n.gpu}+${after.n.vram} card cells, ` +
            `so 「a page with no readings」 never happened and the latch was not tested`,
        );
      }
      if (after.cols !== withReadings.cols) {
        fail(
          `a page with no card readings took the columns away: [${withReadings.cols}] → [${after.cols}] — ` +
            `the latch is meant to stop exactly this (「一点就没有 GPU 这列了」)`,
        );
      }
      console.log(
        `card columns: [${withReadings.cols}] with ${withReadings.n.gpu}+${withReadings.n.vram} filled cells, ` +
          `still [${after.cols}] on a page with ${after.n.gpu}+${after.n.vram}`,
      );
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
      //   said of the day's readings; the drawers share that slot and
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
      const still = [];
      for (const h of heads) {
        //   **Pressed twice, and the two orders compared** — not 「did
        //   the rows change from whatever they were」. A head the table
        //   already happens to be sorted by moves nothing on the first
        //   press, and reads as a head that does not work: measured
        //   2026-09-05 on PID, which the section above leaves the table
        //   ordered by. Ascending against descending is a question
        //   about the column itself and does not depend on where the
        //   table started.
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
        await page.locator(`[data-machine-card] [data-top-head="${h}"] button`).click();
        await wait(900);
        const flipped = await page.evaluate(() =>
          [...document.querySelectorAll("[data-machine-card] [data-top-row]")].map((r) => r.dataset.topRow).join(","),
        );
        if (flipped !== after.rows) moved += 1;
        else still.push(h);
      }
      //   **Every head, not 「at least one」.** The old form asked only
      //   that some press moved the rows, and it stayed green through
      //   2026-09-05's four new columns while three of the eleven heads
      //   did nothing — the fixture gave every row the same account and
      //   no status at all, so 状态 and USER had nothing to order by and
      //   the presses were no-ops that looked like ties.
      //
      //   The fixture now gives each column an order of its own (see
      //   the rows), so a head that does not move them is a head that
      //   does not sort. **Which head** is named, because 「8 of 11」
      //   sends somebody to look at all eleven.
      if (moved !== heads.length) {
        fail(
          `${heads.length - moved} of ${heads.length} column heads read the same both ways ` +
            `(${still.join(", ")}) — a head that cannot re-order is a control that does nothing`,
        );
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
      //   ⚠️ **Every cell of a row, not its first.** The first child is
      //   the pid, and the pid is the one cell drawn in the muted ink —
      //   so comparing the head with it reported 「0.62 → 0.94」 and
      //   passed while the head and **every other cell on the table**
      //   were the same 0.94. An assertion aimed at the one atypical
      //   member of a set is an assertion about nothing.
      const row = document.querySelector("[data-machine-card] [data-top-row]");
      const cells = row ? [...row.children] : [];
      const cell = cells[0];
      const pick = document.querySelector("[data-machine-card] [data-plot-pick]");
      const seam = document.querySelector("[data-machine-card] [data-top-seam]");
      const table = document.querySelector("[data-machine-card] [data-load-top-box]");
      const plot = document.querySelector("[data-machine-card] [data-top-plot]");
      const svg = document.querySelector("[data-machine-card] [data-plot-svg]");
      if (!head || !cell) return null;
      const cs = (n) => getComputedStyle(n);
      const lum = (c) => {
        const m = /rgba?\(([^)]+)\)/.exec(c);
        if (!m) return 0;
        const [r, g, b, a = "1"] = m[1].split(",").map((x) => parseFloat(x));
        //   How much ink actually lands, against the paper: the alpha
        //   is the channel this palette uses for its ink steps.
        return +(parseFloat(a) * (1 - (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255)).toFixed(3);
      };
      return {
        headWeight: +cs(head).fontWeight,
        rowWeight: +cs(cell).fontWeight,
        headColor: cs(head).color,
        rowColor: cs(cell).color,
        //   The whole row, so the claim is about the table and not
        //   about whichever cell happened to come first.
        cellWeights: [...new Set(cells.map((c) => +cs(c).fontWeight))],
        cellInks: [...new Set(cells.map((c) => lum(cs(c).color)))],
        headInk: lum(cs(head).color),
        //   The rule under the heads, and the one above the table.
        headRule: cs(head).boxShadow,
        seamRule: seam ? +parseFloat(cs(seam).borderTopWidth || "0") : null,
        //   How far the table's head sits from the figure's foot,
        //   against how far two rows sit from each other. A break has
        //   to be bigger than the rhythm it is breaking.
        //   **Which comes first on the glass** (2026-09-05:「先把散点
        //   和表再交换下位置」 — the second swap of this pair; the
        //   figure is on top again). Read as coordinates rather than as
        //   DOM order on purpose: a stylesheet can reorder a grid, and
        //   what a reader meets first is a fact about the glass.
        plotToTable:
          table && plot
            ? +(table.getBoundingClientRect().top - plot.getBoundingClientRect().bottom).toFixed(1)
            : null,
        //   **And how far the table's head sits below the figure's axis
        //   words** — the distance the first swap was made to open.
        //   Adjacency is safe again *only because the two rows of eight
        //   axis words are gone*: one word per axis now, opening in
        //   place. What is left directly above the head is the figure's
        //   foot, a sentence.
        pickToHead:
          pick && head
            ? +(head.getBoundingClientRect().top - pick.getBoundingClientRect().bottom).toFixed(1)
            : null,
        //   **Are the axis labels inside the drawing?** (2026-09-06:
        //   「以合适的方式显示x和y label,比如贴在坐标轴内侧。不要占用
        //   与下方的间隔空间」.) Asked as containment rather than as a
        //   gap, because containment is what was asked for *and*
        //   implies the rest: a label inside the drawing occupies no
        //   row in the flow, so it cannot be the space between the
        //   figure and the table.
        //   ⚠️ **The ink, not the element.** The label's hit area is
        //   44px tall on purpose and the strip it sits in is a third of
        //   that, so the *box* hangs out of the drawing by design —
        //   asked that way this reported both labels outside on a build
        //   where both words were plainly inside. What has to be in the
        //   drawing is what a reader can see.
        labels: svg
          ? [...document.querySelectorAll("[data-machine-card] [data-plot-pick]")].map((n) => {
              //   Over the **text nodes**: a range that contains an
              //   element takes that element's border box, and the
              //   button here is 44px tall — which is the same wrong
              //   answer one level down.
              let top = Infinity;
              let bottom = -Infinity;
              let left = Infinity;
              let right = -Infinity;
              const walk = document.createTreeWalker(n, NodeFilter.SHOW_TEXT);
              for (let t = walk.nextNode(); t; t = walk.nextNode()) {
                if (!t.textContent.trim()) continue;
                const g = document.createRange();
                g.selectNodeContents(t);
                const b = g.getBoundingClientRect();
                if (!b.width && !b.height) continue;
                top = Math.min(top, b.top);
                bottom = Math.max(bottom, b.bottom);
                left = Math.min(left, b.left);
                right = Math.max(right, b.right);
              }
              const r = { top, bottom, left, right };
              const v = svg.getBoundingClientRect();
              return {
                which: n.dataset.plotPick,
                inside:
                  Number.isFinite(r.top) &&
                  r.top >= v.top - 1 &&
                  r.bottom <= v.bottom + 1 &&
                  r.left >= v.left - 1 &&
                  r.right <= v.right + 1,
              };
            })
          : [],
        //   And what is left between the last of the drawing and the
        //   first of the table, for the record.
        figureToHead:
          svg && head ? +(head.getBoundingClientRect().top - svg.getBoundingClientRect().bottom).toFixed(1) : null,
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
      //   **Heavier than every cell, and no lighter than any of them.**
      //
      //   Not 「darker」: the table's numbers are already at full ink
      //   (measured, every cell but the pid: alpha .94), so a head
      //   cannot be darker than its body without dimming the data,
      //   which is the wrong thing to trade. Weight is the channel
      //   that is left, and the rule under the heads is the other
      //   half — see the stylesheet, where it is also made to run
      //   unbroken across the gaps.
      if (!look.cellWeights.length) {
        fail("probe dead: the row has no cells, so 「the head is set apart」 measured nothing");
      } else {
        const heaviestCell = Math.max(...look.cellWeights);
        if (look.headWeight <= heaviestCell) {
          fail(
            `the column heads weigh ${look.headWeight} against ${look.cellWeights.join("/")} in the row — ` +
              `nothing on screen says which line is the head`,
          );
        }
        const darkestCell = Math.max(...look.cellInks);
        if (look.headInk < darkestCell - 0.02) {
          fail(
            `the column heads are lighter than the rows (ink ${look.headInk} against ${look.cellInks.join("/")}) — ` +
              `a head has to be at least as present as what is under it`,
          );
        }
      }
      if (!look.headRule || look.headRule === "none") {
        fail("the column heads carry no rule under them — the line that says 「rows start here」 is missing");
      }
      if (!(look.seamRule > 0)) {
        fail(
          `the table is not separated from the figure above it (seam rule ${look.seamRule}) — ` +
            `they are two blocks about the same two hundred processes and nothing says where one ends`,
        );
      }
      //   **The figure is the first thing in the panel** (2026-09-05:
      //   「先把散点和表再交换下位置」).
      //
      //   ⚠️ This assertion has now pointed both ways, and it is not a
      //   preference either time: what it pins is that the order on the
      //   glass is the one that was asked for, so a stylesheet or a
      //   refactor cannot quietly turn it over.
      if (look.plotToTable !== null && !(look.plotToTable > 0)) {
        fail(`the table starts ${look.plotToTable}px after the figure ends — the two are back in the other order`);
      }
      //   **Both axis labels sit inside the drawing.** Red arm: put
      //   one back in the flow and its box leaves the svg's.
      if (look.labels.length !== 2) {
        fail(`probe dead: ${look.labels.length} axis labels found, not two — 「tucked inside」 measured nothing`);
      } else {
        const out = look.labels.filter((l) => !l.inside).map((l) => l.which);
        if (out.length) {
          fail(
            `the ${out.join(" and ")} axis label sits outside the drawing — a label in the flow spends a row of ` +
              `the card on one word, and the lower one spends it between the figure and the table`,
          );
        }
      }
      //   And they are still two blocks rather than one — the reason
      //   the first swap happened at all. If anything above the head
      //   ever grows back into a row of headings, this distance is what
      //   stops it being read as the table's own.
      if (look.pickToHead !== null && look.rowPitch !== null && look.pickToHead < look.rowPitch * 1.5) {
        fail(
          `the table's head sits ${look.pickToHead}px below the figure's axis words where a row is ${look.rowPitch}px tall — ` +
            `a break has to be bigger than the rhythm it breaks`,
        );
      }
      console.log(
        `heads: weight ${look.cellWeights.join("/")}→${look.headWeight}, ink ${look.cellInks.join("/")}→${look.headInk}, ` +
          `rule under them ${look.headRule !== "none"}, seam ${look.seamRule}px, ` +
          `figure ends ${look.plotToTable}px before the table, drawing to head ${look.figureToHead}px, ` +
          `labels inside ${look.labels.map((l) => `${l.which}:${l.inside}`).join(" ")}, ` +
          `head ${look.pickToHead}px below the axis words against a ${look.rowPitch}px row`,
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
      //   ⚠️ **Tombstone: an assertion that the heads' ground reached
      //   12px upward, 2026-09-11 — written and withdrawn the same
      //   night.** It guarded a fix that turned out to paint nothing:
      //   the head sits at 0 from the scroll port's top, so a shadow
      //   above it is outside the clip. Painted red and photographed:
      //   no red anywhere.
      //
      //   **A gate over a fix that does not work is worse than no gate**
      //   — it is the sentence 「something is holding this」 with nothing
      //   behind it, and the next person reads it as coverage. The band
      //   on the user's screen is unexplained and is an account in
      //   docs/handoff, not an assertion here.
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
      //   ⚠️ **The container's right edge, not the grid's own.** This
      //   read `t.getBoundingClientRect()` — the grid itself — and
      //   compared it with the grid's own last track. A grid is exactly
      //   as wide as its tracks unless something stretches it, so that
      //   difference is **0 whatever the table does**: a table sized to
      //   its content passes identically. The number it printed was
      //   right and could not have been wrong, which is the same fault
      //   as reading a floor off the element it is meant to constrain.
      const holder = t.parentElement;
      const box = holder.getBoundingClientRect();
      //   ⚠️ **Track geometry off the row cells, not the heads.** Each
      //   head but the last is widened by one gap and padded back by it
      //   so the rule under them runs unbroken; measured on the heads,
      //   every track then reads a gap wider than it is and every gap
      //   reads zero. The row cells are the tracks without that
      //   compensation.
      const firstRow = document.querySelector("[data-machine-card] [data-top-row]");
      const trackEls = firstRow ? [...firstRow.children] : heads;
      const rects = trackEls.map((h) => h.getBoundingClientRect());
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
      const headRoom = {};
      //   Cells whose ink is wider than their track **and which nothing
      //   is clipping** — see the loop below for why that pair is the
      //   test and why `slackIn` on its own is not.
      const spill = {};
      for (let i = 0; i < trackEls.length && i < heads.length; i += 1) {
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
        //   ⚠️ **A cell wider than its track is allowed; a cell wider
        //   than its track *and not clipped* is not.** The line above
        //   deliberately does not assert on `slackIn` — 用户 truncates
        //   by design, so 「the ink is wider than the track」 is the
        //   ordinary state of that column and an assertion on it would
        //   be wrong, not strict.
        //
        //   What is never allowed is the third case: wide, and nothing
        //   cutting it, so the text simply draws past its column into
        //   the next one. That is what 「列表值超出表头」 looks like
        //   from the reader's side.
        //
        //   **`scrollWidth > clientWidth` is the whole test, and it
        //   works *because* of a trap this file has been bitten by**:
        //   under `overflow: visible` those two are equal no matter how
        //   much text there is (the ink escapes instead of scrolling),
        //   while a clipped box reports the overflow honestly. So the
        //   pair splits exactly the way this assertion needs:
        //     · scrollWidth > clientWidth  → something is clipping it ✓
        //     · equal, and ink > track     → it is drawing outside ✗
        //   Range-measuring alone cannot tell those apart: it reports
        //   the text's own width either way, truncated or not.
        for (const c of cells) {
          if (ink(c) <= rects[i].width + 0.5) continue;
          const clipped = c.scrollWidth > c.clientWidth
            || [...c.querySelectorAll("*")].some((k) => k.scrollWidth > k.clientWidth);
          if (!clipped) {
            (spill[key] ??= []).push(+(ink(c) - rects[i].width).toFixed(1));
          }
        }
        //   **The head on its own**, kept apart from the cells because
        //   the two have different rights. A *cell* may overrun its
        //   track — 用户 truncates by design, and so does 状态 when it
        //   is printing a token this build has no word for. A **head**
        //   may not: it is the column's name, it is the same three
        //   characters on every machine, and a track too narrow for it
        //   silently amputates the word rather than eliding it.
        //   Measured 2026-09-05 on all three machines: 「CPU」 plus its
        //   sort chevron in a 44px track rendered 「CPL」, and every
        //   assertion in this section passed — they were all about
        //   tracks and gaps, and none about whether a word survived.
        //   ⚠️ **Measured as「does anything stick out of the track」,
        //   not as「how wide is the text」.** A `Range` over the
        //   button's contents returns the *text*, and the chevron is an
        //   `<svg>` the range does not account for — written that way
        //   first, and it reported 21px of room in the very track that
        //   was rendering 「CPL」. So every child is asked for its own
        //   box and the union is compared with the track's edges.
        let l = Infinity;
        let r = -Infinity;
        for (const c of btn.childNodes) {
          let box2 = null;
          if (c.nodeType === 3) {
            const rg = document.createRange();
            rg.selectNodeContents(c);
            box2 = rg.getBoundingClientRect();
          } else if (c.getBoundingClientRect) {
            box2 = c.getBoundingClientRect();
          }
          if (!box2 || (box2.width === 0 && box2.height === 0)) continue;
          l = Math.min(l, box2.left);
          r = Math.max(r, box2.right);
        }
        //   ⚠️ **How far it sticks out, not how much room is left on
        //   the nearer side.** Written as `min(rightRoom, leftRoom)`
        //   first, and the head's content is packed against one edge —
        //   so the other room was always exactly 0 and the minimum was
        //   0 for all eleven columns, on a build that fitted and on a
        //   build that did not. A number that is 0 whatever happens is
        //   a check that cannot fail.
        headRoom[key] =
          r === -Infinity ? null : +Math.max(0, rects[i].left - l, r - rects[i].right).toFixed(1);
      }
      //   Whether anything at all is written in each column, anywhere
      //   on the page. Not 「every row」: absence is legitimate per row
      //   (a process with no age, a card figure khor may not read), and
      //   what is not legitimate is a column that is never filled by
      //   anybody.
      //   Each track's width divided by the share it was given. With
      //   every floor satisfied these are one number; a column that
      //   swallowed the leftover shows up as its own.
      const frs = Object.fromEntries(
        (t.dataset.topFrs ?? "").split(" ").filter(Boolean).map((p) => {
          const [k, v] = p.split(":");
          return [k === "name" ? "cmd" : k, Number(v)];
        }),
      );
      const share = {};
      for (let i = 0; i < trackEls.length && i < heads.length; i += 1) {
        const key = heads[i].dataset.topHead ?? heads[i].dataset.c;
        if (frs[key]) share[key] = rects[i].width / frs[key];
      }
      return {
        heads: heads.length,
        share,
        //   The used value, so this counts what the browser resolved
        //   rather than what anybody wrote.
        tracks: getComputedStyle(t).gridTemplateColumns.trim().split(/\s+/).length,
        trackList: getComputedStyle(t).gridTemplateColumns.trim(),
        headRoom,
        boxW: +box.width.toFixed(1),
        gridW: +t.getBoundingClientRect().width.toFixed(1),
        slack: +(box.right - rects[rects.length - 1].right).toFixed(1),
        gaps,
        gap: +gap.toFixed(1),
        slackIn,
        spill,
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
      //   **As many tracks as there are heads.** This is the invariant
      //   the four hand-written stylesheet variants could not keep: on
      //   2026-09-05 the two card columns became two independent
      //   latches, no variant covered 「显存 but no GPU」 (turing), and
      //   six cells met five declared tracks — the sixth fell into an
      //   implicit column and the 「命令」 track collapsed to 55px.
      //   Every other assertion in this section passed, because the
      //   fixture only ever has the one combination.
      if (spread.tracks !== spread.heads) {
        fail(
          `the table declares ${spread.tracks} tracks for ${spread.heads} columns (${spread.trackList}) — ` +
            `the extra cells fall into implicit tracks and the widths stop meaning anything`,
        );
      }
      const canyon = spread.gaps.filter((g) => g > spread.gap + 1);
      if (canyon.length) {
        fail(
          `at 1500 ${canyon.length} of ${spread.gaps.length} column gaps are wider than the table's own gap ` +
            `(${spread.gaps.join("/")} against ${spread.gap}) — the slack went between two columns`,
        );
      }
      //   **⚠️ Tombstone: 「no track but 「命令」 may hold room it has no
      //   ink for」 is gone, and its replacement asserts the opposite
      //   arrangement.**
      //
      //   That rule came from the 799px hole — slack parked inside a
      //   quantity column opens a gap to the *left* of its numbers, and
      //   the gaps between columns stay 8 the whole time, so it is
      //   invisible to a gap-based probe. It was true, and it drove the
      //   design to 「every column at its floor, the whole remainder to
      //   the last column」 — which is the shape the person using this
      //   then rejected on a wide card (2026-09-05:「我们要不按页面宽度
      //   均分列宽。或稍微微调一些,大致是均分」). One column holding
      //   hundreds of pixels of blank is the same hole, just moved to
      //   the end of the row.
      //
      //   So the assertion is now **that the division happened**: every
      //   track is `minmax(floor, share fr)`, and where the floors are
      //   all satisfied the grid's own arithmetic makes `width / share`
      //   identical across the table. Any build that hands the leftover
      //   to one column breaks that equality by hundreds.
      //
      //   The shares are read off the table (`data-top-frs`) rather
      //   than repeated here: a gate carrying its own copy of the
      //   weights is the two-lists shape this file has been bitten by.
      //
      //   **Bounded to the wide stop on purpose.** A narrow table has
      //   tracks pinned at their floors, and a floor above a column's
      //   share is legitimate — there the equality does not hold and
      //   must not be asserted.
      const per = Object.entries(spread.share);
      if (per.length < 4) {
        fail(`probe dead: only ${per.length} tracks carried a share, so 「the width was divided」 measured nothing`);
      } else {
        const lo = Math.min(...per.map(([, v]) => v));
        const hi = Math.max(...per.map(([, v]) => v));
        if (hi - lo > 2) {
          fail(
            `at 1500 the width is not divided: per-share widths run ${lo.toFixed(1)}→${hi.toFixed(1)}px ` +
              `(${per.map(([k, v]) => `${k}:${v.toFixed(1)}`).join(" ")}) — ` +
              `one column is taking the leftover instead of every column taking its share`,
          );
        }
      }
      //   **No column's name is cut.** See the probe for what this is
      //   kept apart from and why.
      const cut = Object.entries(spread.headRoom).filter(([, v]) => v !== null && v > 0.5);
      if (cut.length) {
        fail(
          `the heads ${cut.map(([k, v]) => `${k} (${v}px over)`).join(", ")} do not fit their own tracks — ` +
            `a column head that does not fit is not elided, it is amputated: 「CPU」 becomes 「CPL」 and reads ` +
            `like a word nobody chose`,
        );
      }
      //   **No cell draws outside its own column.** A cell may be
      //   wider than its track — that is what truncation is for — but
      //   then something must be cutting it. See the probe.
      //
      //   ⚠️ **This one has not been shown to fire, and two attempts
      //   failed in the same way.** Taking `truncate` off the command
      //   cell, and then taking only its clipping off, both turned the
      //   whole run red — but as a *timeout* several steps earlier, not
      //   as this message: the same truncation is already load-bearing
      //   for 「cut command: ellipsis」 and for the expand-on-click
      //   behaviour, so there is no build in reach where this is the
      //   first thing to notice.
      //
      //   That is not the same as a gate with no failing world (the
      //   world exists — text drawing into the next column — it is just
      //   not reachable by removing something). What it means is that
      //   **today this column is guarded twice and this is the second
      //   guard**; its value is the next column somebody adds, which
      //   will have no ellipsis assertion of its own. Reported here
      //   rather than in the green list, per this repo's rule about
      //   assertions that have never been seen to fail.
      const over = Object.entries(spread.spill ?? {});
      if (over.length) {
        fail(
          `cells draw past their tracks with nothing clipping them: ` +
            over.map(([k, v]) => `${k} (${v.length} row(s), worst ${Math.max(...v)}px over)`).join(", ") +
            ` — text that overruns its column lands on the next one, which is what 「列表值超出表头」 is`,
        );
      }
      console.log(
        `table at 1500: holder ${spread.boxW}, grid ${spread.gridW}, last track ${spread.slack} short of it, ` +
          `gaps ${spread.gaps.join("/")} (gap ${spread.gap}), ` +
          `head over ${Object.entries(spread.headRoom).map(([k, v]) => `${k}:${v}`).join(" ")}, ` +
          `per-share ${Object.entries(spread.share).map(([k, v]) => `${k}:${v.toFixed(1)}`).join(" ")}, ` +
          `room beyond ink ${Object.entries(spread.slackIn).map(([k, v]) => `${k}:${v}`).join(" ")}, ` +
          `unclipped spill ${Object.keys(spread.spill ?? {}).length === 0 ? "none" : Object.entries(spread.spill).map(([k, v]) => `${k}:${v.length}`).join(" ")}`,
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

  //     ── the disk drawer: the watched directories, and their editor
  //
  //     2026-09-06:「磁盘是有多个指标的,但用户可能有自己想关注的磁盘。写在
  //     磁盘点击后,可以有一个编辑,用户可以在这增删目录,而后更新关注磁盘」.
  //     Measured on the peer's card, whose fixture watches three
  //     directories, one per shape:
  //
  //       - three rows under the volumes, each a unit (`khor devices`
  //         prints one sentence per row — the smoke holds the count),
  //         and the three sentences differ: a directory on a listed
  //         volume, a missing one, one on a filesystem khor does not
  //         list. The third must not wear the first's numbers — that is
  //         the path on `/proc` reporting the root — so its number
  //         column is a word, not a size;
  //       - **one column, one row per path** (the ruling): adding a
  //         fourth path draws a fourth row marked 未保存 with **no
  //         unit** — it is not a reading, and neither face may count it
  //         — so the unit count stays three until the machine answers;
  //         saving sends **the whole list** to `set_watch` for the peer,
  //         the row turns into a saved-but-unanswered one (还没问到,
  //         still no unit) and the save comes to rest;
  //       - a relative path is refused **by the node's sentence**, on
  //         screen under the list, with the failure word on the button
  //         that was pressed — one voice with `khor watch` — and the
  //         refused row stays, marked, for fixing.
  {
    await page.locator('[data-machine-card] [data-dial="disk"]').click();
    await wait(400);
    const drawer = await page.evaluate(() => {
      const box = document.querySelector("[data-machine-card] [data-drawer-scroll]");
      const parts = document.querySelector('[data-machine-card] [data-parts="disk"]');
      if (!box || !parts || parts.hidden) return null;
      const rows = [...box.querySelectorAll('[data-parts="watch"] [data-watch-row]')].map((r) => ({
        path: r.dataset.watchRow,
        shape: r.dataset.watchShape,
        pending: r.dataset.pending ?? null,
        unit: r.querySelector('[data-vitals-unit="watch"]') !== null,
        sentence: r.querySelector('[data-vitals-detail="watch"]')?.textContent.trim() ?? "",
        num: r.querySelector("[data-part-num]")?.textContent.trim() ?? "",
        bar: r.querySelector("[data-part-ink]") !== null,
      }));
      return {
        rows,
        vols: parts.querySelectorAll('[data-vitals-unit="vol"]').length,
        editor: !!box.querySelector("[data-watch-editor]"),
        units: box.querySelectorAll('[data-parts="watch"] [data-vitals-unit="watch"]').length,
        saveDisabled: box.querySelector("[data-watch-save]")?.disabled ?? null,
        count: Number(document.querySelector("[data-machine-card] [data-drawer-count]")?.dataset.drawerCount),
      };
    });
    if (!drawer) {
      fail("probe dead: the disk drawer did not open, so nothing about the watched directories was measured");
    } else {
      const shapes = drawer.rows.map((r) => r.shape).join(",");
      if (shapes !== "on,missing,unlisted") {
        fail(`the disk drawer draws watched rows as [${shapes}] for a fixture watching one of each shape`);
      }
      if (drawer.rows.some((r) => !r.unit)) fail("a watched row is not a unit — `khor devices` prints one sentence per row and the counts would disagree");
      const sentences = new Set(drawer.rows.map((r) => r.sentence));
      if (sentences.size !== drawer.rows.length || drawer.rows.some((r) => !r.sentence)) {
        fail(`the three watched rows do not say three different things: ${drawer.rows.map((r) => `「${r.sentence}」`).join(" ")}`);
      }
      const on = drawer.rows.find((r) => r.shape === "on");
      const unlisted = drawer.rows.find((r) => r.shape === "unlisted");
      if (on && !/\d/.test(on.num)) fail(`the row on a listed volume shows no size: 「${on.num}」`);
      if (unlisted && (/\d/.test(unlisted.num) || unlisted.bar)) {
        fail(`the row on an unlisted filesystem wears a size or a bar (「${unlisted.num}」, bar ${unlisted.bar}) — the root's numbers under /proc`);
      }
      //   The heading counts volumes and only volumes: a directory is
      //   a place on one, not one more of them.
      if (drawer.count !== drawer.vols) {
        fail(`the drawer's heading counts ${drawer.count} for ${drawer.vols} volumes — the ${drawer.rows.length} watched rows are not volumes`);
      }
      if (drawer.rows.some((r) => r.pending !== null)) {
        fail(`a row is marked unsaved before anything was typed: ${JSON.stringify(drawer.rows)}`);
      }
      if (!drawer.editor) fail("the disk drawer has no editor for the watched directories");
      else if (drawer.saveDisabled !== true) {
        fail("the save is live before anything was changed — a control that always offers to save is a control that cannot say whether anything changed");
      } else {
        //   Add one: a fourth row, marked, and **still three units**.
        //   Then save: the whole list crosses, the row turns into a
        //   saved-but-unanswered one, and the unit count is still three
        //   — the machine has not answered for it, and neither has the
        //   terminal.
        const rowsOf = () =>
          page.evaluate(() => {
            const box = document.querySelector("[data-machine-card] [data-drawer-scroll]");
            return {
              rows: [...box.querySelectorAll('[data-parts="watch"] [data-watch-row]')].map((r) => `${r.dataset.watchRow}:${r.dataset.watchShape}${r.dataset.pending ? "+" + r.dataset.pending : ""}`),
              units: box.querySelectorAll('[data-parts="watch"] [data-vitals-unit="watch"]').length,
              saveDisabled: box.querySelector("[data-watch-save]")?.disabled ?? null,
              error: box.querySelector("[data-watch-error]")?.textContent ?? null,
            };
          });
        const sent0 = watchSaves.length;
        await page.locator("[data-machine-card] [data-watch-input]").fill("/scratch/run7");
        await page.locator("[data-machine-card] [data-watch-add]").click();
        await wait(150);
        const added = await rowsOf();
        if (added.rows.join(",") !== "/data/models:on,/nope:missing,/proc/1:unlisted,/scratch/run7:pending+add") {
          fail(`adding a path drew ${JSON.stringify(added.rows)} — one new row, marked unsaved, under the three was owed`);
        }
        if (added.units !== 3) {
          fail(`adding an unsaved path changed the unit count to ${added.units} — a path nobody answered for is not a reading, and \`khor devices\` prints no sentence for it`);
        }
        await page.locator("[data-machine-card] [data-watch-save]").click();
        const saved = await until(5_000, async () => watchSaves.length > sent0);
        if (!saved) fail("pressing 保存 sent nothing to set_watch");
        else {
          const call = watchSaves[watchSaves.length - 1];
          if (call.machine !== "peer" || JSON.stringify(call.paths) !== JSON.stringify(["/data/models", "/nope", "/proc/1", "/scratch/run7"])) {
            fail(`the save sent ${JSON.stringify(call)} — the whole list, for the peer, was owed`);
          }
          const rested = await until(6_000, async () => {
            const now = await rowsOf();
            return now.saveDisabled === true && now.rows.join(",") === "/data/models:on,/nope:missing,/proc/1:unlisted,/scratch/run7:asked";
          });
          const after = await rowsOf();
          if (!rested) fail(`after a tidy save the rows read ${JSON.stringify(after.rows)} with the save ${after.saveDisabled ? "at rest" : "live"} — the new path was to become a saved, unanswered row and the save to rest`);
          if (after.units !== 3) fail(`a saved but unanswered path counts as a unit (${after.units}) — the terminal prints nothing for it yet`);
          if (after.error) fail(`a tidy save printed a refusal: ${after.error}`);
        }
        //   A relative path: the node's sentence, on the button and under
        //   the list, and the draft kept for fixing.
        await page.locator("[data-machine-card] [data-watch-input]").fill("models");
        await page.locator("[data-machine-card] [data-watch-add]").click();
        await wait(150);
        await page.locator("[data-machine-card] [data-watch-save]").click();
        const refused = await until(5_000, async () => (await page.locator("[data-machine-card] [data-watch-error]").count()) === 1);
        const bad = await page.evaluate(() => ({
          error: document.querySelector("[data-machine-card] [data-watch-error]")?.textContent ?? "",
          word: document.querySelector("[data-machine-card] [data-watch-save]")?.textContent.trim() ?? "",
          failed: document.querySelector("[data-machine-card] [data-watch-save]")?.dataset.failed ?? null,
          rows: [...document.querySelectorAll('[data-machine-card] [data-parts="watch"] [data-watch-row]')].map((r) => `${r.dataset.watchRow}${r.dataset.pending ? "+" + r.dataset.pending : ""}`),
        }));
        if (!refused || !bad.error.includes("models")) fail(`a relative path was not refused in the node's sentence (${JSON.stringify(bad)})`);
        if (bad.failed !== "true" || bad.word === (await page.evaluate(() => document.querySelector("[data-machine-card] [data-watch-add]")?.textContent.trim()))) {
          fail(`the refused save does not wear the failure word on the button pressed (${JSON.stringify(bad)})`);
        }
        if (!bad.rows.includes("models+add")) fail(`the refused entry vanished from the list (${JSON.stringify(bad.rows)}) — the person cannot fix what they cannot see`);
        console.log(`watch editor: ${drawer.rows.length} rows [${shapes}], save sent ${JSON.stringify(watchSaves[watchSaves.length - 2]?.paths)}, refusal 「${bad.error}」`);
      }
    }
    //   Back to the day: the sections below read the band, which a
    //   drawer hides.
    await page.locator('[data-machine-card] [data-dial="disk"]').click();
    await wait(300);
  }

  //     ── two readings of one day, and one cursor across them ──────
  //
  //     **The moment survives the switch.** Two readings of the same
  //     numbers is the claim; two components each remembering their
  //     own state is what it would be instead, and the difference shows
  //     exactly here — put the cursor somewhere, change the reading and
  //     come back, and the readout must say the same thing word for
  //     word. (It was measured across three readings until 2026-09-06;
  //     the one in the middle left, and the parallel figure has no
  //     time cursor to read out, so the round trip is what is left to
  //     measure — a panel that reset its cursor on the way out and back
  //     would come home saying nothing.)
  const readings = await page.evaluate(async () => {
    const pick = (k) => document.querySelector(`[data-machine-card] [data-day-view-pick="${k}"]`);
    const band = () => document.querySelector("[data-machine-card] [data-day-band]");
    const readout = () => document.querySelector("[data-machine-card] [data-day-at]")?.innerText.trim() ?? null;
    const words = [...document.querySelectorAll("[data-machine-card] [data-day-view-pick]")].map(
      (b) => b.dataset.dayViewPick,
    );
    if (words.length !== 2) return { words };
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
    pick("para").click();
    await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    const paraViews = [...document.querySelectorAll("[data-machine-card] [data-day-view]")].map(
      (n) => n.dataset.dayView,
    );
    pick("heat").click();
    await new Promise((ok) => requestAnimationFrame(() => requestAnimationFrame(ok)));
    const backOnHeat = readout();
    return { words, onHeat, paraViews, backOnHeat };
  });
  if (readings.words.length !== 2) fail(`the day offers ${readings.words.length} readings, not two`);
  else if (!readings.onHeat) fail("the cursor drew no readout — 「which moment」 is what the switch has to carry");
  else {
    if (readings.onHeat !== readings.backOnHeat) {
      fail(`the moment did not survive the round trip: "${readings.onHeat}" came back as "${readings.backOnHeat}"`);
    }
    if (readings.paraViews.some((v) => v !== "para")) {
      fail(`picking 24h 统计 left bands reading ${[...new Set(readings.paraViews)].join(",")}`);
    }
  }

  //     ── both readings, and the words each one draws ──────────────
  //
  //     **The section above only passes through 叠放.** Before
  //     2026-09-06 it was never opened by anything here, and on 2026-09-04 that is
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
  //     says anything. So this section asks two things, on both
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
  //         crash was in. 热力带 prints the bare number there on
  //         purpose (`day-peak-at` is only the parallel figure's,
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
    for (const view of ["heat", "para"]) {
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
  for (const view of ["heat", "para"]) {
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
    //   text element. 热力带 must not have one; 叠放 must.
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
    `day readings: ${["heat", "para"]
      .map((v) => {
        const t = spoken[v]?.texts ?? [];
        return `${v} ${t.length} words / ${t.filter((s) => !String(s ?? "").trim()).length} empty`;
      })
      .join(", ")}`,
  );

  //     ── 24h 统计: each axis wears its quartiles, in the right ink order
  //
  //     2026-09-06:「叠放图对每类指标新增一个趋势:这一列有一个类似科研
  //     绘图的区间,中间是均值或中位数,上下是标准差或四分位数」. What is
  //     drawn is a box from P25 to P75 with the median as a bar across
  //     it, per axis. Four claims, and the first is the one that keeps
  //     the other three honest:
  //
  //       - **the numbers are the fixture's.** P25, the median and P75
  //         on the mark are recomputed here from the same buckets, with
  //         the module's own definition (the median the brush readout
  //         already uses), so a box drawn from a constant, or from the
  //         wrong series, goes red on a value rather than on a shape;
  //       - **the median is inside the box** and **the peak is at or
  //         above its top** — geometry, read off the elements;
  //       - **the box's fill is under the hairlines, and its outline
  //         and the median bar over them**, by document order — a fill
  //         painted on top would cover the very hairs the figure is
  //         made of, and a bar painted underneath is under a slab of
  //         black exactly where the middle half of the readings is;
  //       - **the box wears `--massLo`**, computed, not a new colour —
  //         the mark is a quantity and wears what the areas wear.
  {
    const quartilesOf = (vs) => {
      const s = vs.filter((v) => v !== null).sort((a, b) => a - b);
      if (s.length < 4) return null;
      const at = (q) => s[Math.min(s.length - 1, Math.floor(q * (s.length - 1)))];
      return { q1: at(0.25), med: s[Math.floor(s.length / 2)], q3: at(0.75) };
    };
    const expect = {
      cpu: quartilesOf(dayBuckets),
      gpu: quartilesOf(gpuBuckets),
      mem: quartilesOf(memBuckets),
      disk: quartilesOf(diskBuckets),
      vram: quartilesOf(vramBuckets),
    };
    await page.locator('[data-machine-card] [data-day-view-pick="para"]').click();
    await wait(400);
    const marks = await page.evaluate(() => {
      const band = document.querySelector("[data-machine-card] [data-day-band]");
      if (!band) return null;
      const blk = band.closest(".dayblk");
      const massLo = blk ? getComputedStyle(blk).getPropertyValue("--massLo").trim() : null;
      const all = [...band.querySelectorAll("*")];
      const order = (n) => all.indexOf(n);
      //   The hairlines' group is the one holding the `.draw` paths.
      const hairs = band.querySelector("path.draw")?.parentElement ?? null;
      const out = {};
      for (const box of band.querySelectorAll("[data-day-trend-of]")) {
        const key = box.dataset.dayTrendOf;
        const med = band.querySelector(`[data-day-median="${key}"]`);
        const frame = band.querySelector(`[data-day-trend-frame="${key}"]`);
        const label = band.querySelector(`[data-day-peak-label="${key}"]`);
        //   The bar's halo: the element painted just before the bar, a
        //   line on the same y in the paper's ink and wider than the bar
        //   — what keeps the bar readable where the bundle is one solid
        //   line (磁盘 at 84.2 all day). Read as structure because the
        //   bar and the bundle share one colour there, and no pixel
        //   probe can tell a bar on a line from the line.
        const under = med?.previousElementSibling ?? null;
        const haloed = !!under && under.tagName === "line" && under.classList.contains("halo") &&
          under.getAttribute("y1") === med.getAttribute("y1") &&
          Number(under.getAttribute("stroke-width")) > Number(med.getAttribute("stroke-width"));
        //   Boxes in the drawing's own units (the viewBox is 1:1 with
        //   pixels); the bar is a line, so its box is grown by half
        //   its halo stroke, and the outline by half of its.
        const grow = (b, by) => (b ? { l: b.x - by, r: b.x + b.width + by, t: b.y - by, b: b.y + b.height + by } : null);
        const hit = (a, c) => !!a && !!c && !(a.r < c.l || c.r < a.l || a.b < c.t || c.b < a.t);
        const labelBox = label ? grow(label.getBBox(), 0) : null;
        const barBox = med ? grow(med.getBBox(), 1.8) : null;
        const frameBox = frame ? grow(frame.getBBox(), 1.2) : null;
        const peak = [...band.querySelectorAll("circle")].find((c) => c.getAttribute("r") === "2.8" && Math.abs(Number(c.getAttribute("cx")) - Number(box.getAttribute("x")) - Number(box.getAttribute("width")) / 2) < 0.6);
        out[key] = {
          q1: Number(box.dataset.dayQ1),
          q3: Number(box.dataset.dayQ3),
          med: med ? Number(med.dataset.dayMed) : null,
          top: Number(box.getAttribute("y")),
          bottom: Number(box.getAttribute("y")) + Number(box.getAttribute("height")),
          medY: med ? Number(med.getAttribute("y1")) : null,
          peakY: peak ? Number(peak.getAttribute("cy")) : null,
          boxBeforeHairs: hairs ? order(box) < order(hairs) : null,
          medAfterHairs: hairs && med ? order(med) > order(hairs) : null,
          frameAfterHairs: hairs && frame ? order(frame) > order(hairs) : null,
          fill: getComputedStyle(box).fill,
          framed: !!frame,
          haloed,
          boxH: Number(box.getAttribute("height")),
          labelOnBar: hit(labelBox, barBox),
          labelOnFrame: hit(labelBox, frameBox),
          //   How far the label's bottom clears the top of whatever
          //   mark is drawn — the framed box, or the bar alone.
          //   Positive is clear. **Measured vertically alone**, because
          //   the intersection above is blind on this fixture: every
          //   label sits beside its axis, so the two boxes only ever
          //   meet by the bar's last three tenths of a pixel, and no
          //   series here peaks where it sits. The rule the drawing
          //   follows is one-sided anyway — the label rises until it
          //   is above the mark — so that is what is read.
          labelClear: labelBox ? +(((frameBox ?? barBox)?.t ?? Infinity) - labelBox.b).toFixed(2) : null,
          labelBox,
        };
      }
      return { massLo, marks: out, view: band.dataset.dayView };
    });
    if (!marks || marks.view !== "para") {
      fail(`probe dead: 24h 统计 is not showing (${JSON.stringify(marks?.view)}), so its quartile marks were not measured`);
    } else {
      const keys = Object.keys(expect).filter((k) => expect[k] !== null);
      for (const k of keys) {
        const m = marks.marks[k];
        if (!m) {
          fail(`the ${k} axis draws no quartile box — every axis with a day behind it wears one`);
          continue;
        }
        const e = expect[k];
        if (Math.abs(m.q1 - e.q1) > 0.06 || Math.abs(m.med - e.med) > 0.06 || Math.abs(m.q3 - e.q3) > 0.06) {
          fail(`the ${k} axis's box says ${m.q1}/${m.med}/${m.q3} where the fixture's buckets give ${e.q1.toFixed(1)}/${e.med.toFixed(1)}/${e.q3.toFixed(1)}`);
        }
        if (!m.haloed) {
          fail(`the ${k} axis's median bar has no paper halo under it — on a bundle that is one line all day the bar is invisible without one`);
        }
        if (m.medY === null || m.medY < m.top - 0.6 || m.medY > m.bottom + 0.6) {
          fail(`the ${k} axis's median bar (y=${m.medY}) is outside its box (${m.top}–${m.bottom})`);
        }
        if (m.peakY !== null && m.peakY > m.top + 0.6) {
          fail(`the ${k} axis's peak (y=${m.peakY}) is below the box's top (${m.top}) — a maximum under the 75th percentile`);
        }
        //   A box under four pixels high has no inside for an outline
        //   to enclose, so it is drawn as its bar alone; at four and
        //   above the outline is owed. Read off the fill's height, the
        //   same number the drawing decides by.
        if (m.framed !== m.boxH >= 4) {
          fail(`the ${k} axis's box is ${m.boxH}px high and ${m.framed ? "wears" : "lacks"} an outline — under 4px only the median bar is drawn, at 4px and above the box is`);
        }
        if (m.boxBeforeHairs !== true || m.medAfterHairs !== true || (m.framed && m.frameAfterHairs !== true)) {
          fail(`the ${k} axis's box/median are painted in the wrong order (fill under hairs: ${m.boxBeforeHairs}, outline over hairs: ${m.frameAfterHairs}, median over hairs: ${m.medAfterHairs})`);
        }
        //   **The peak label yields to the mark** (2026-09-06, seen on
        //   内存 and 磁盘: a day that peaks where it usually sits put
        //   「峰值 86% · 17:20」 across its own box). Both are owed, and
        //   neither may cover the other; the label is the one that moves.
        if (!m.labelBox) {
          fail(`the ${k} axis has no peak label to keep off its box — the probe cannot tell whether one covers the other`);
        } else if (m.labelOnBar || m.labelOnFrame) {
          fail(`the ${k} axis's peak label sits on its quartile mark (on the bar: ${m.labelOnBar}, on the box: ${m.labelOnFrame}; label ${JSON.stringify(m.labelBox)})`);
        } else if (!(m.labelClear >= -0.6)) {
          fail(`the ${k} axis's peak label ends ${(-m.labelClear).toFixed(1)}px below the top of its quartile mark — it is meant to rise clear of the mark, never to hang over it`);
        }
      }
      const fills = new Set(keys.map((k) => marks.marks[k]?.fill).filter(Boolean));
      //   Compared as colours, not strings: the token is written as
      //   `rgba(26, 26, 20, 0.16)` and `getComputedStyle` reports the
      //   fill in the same spelling — but a browser is free to
      //   normalise either, so both go through one canvas to be sure.
      const same = await page.evaluate(([a, b]) => {
        const c = document.createElement("canvas").getContext("2d");
        const norm = (s) => { c.fillStyle = "#000"; c.fillStyle = s; return c.fillStyle; };
        return norm(a) === norm(b);
      }, [marks.massLo, [...fills][0] ?? ""]);
      if (fills.size !== 1 || !same) {
        fail(`the quartile boxes wear ${[...fills].join(" / ")} — they are to wear --massLo (${marks.massLo}), the areas' own light ink`);
      }
      console.log(`day quartiles: ${keys.map((k) => `${k} ${marks.marks[k]?.q1}/${marks.marks[k]?.med}/${marks.marks[k]?.q3}`).join(", ")}`);
    }
    await page.locator('[data-machine-card] [data-day-view-pick="heat"]').click();
    await wait(300);
  }

  //     ── opening a card draws the day whole, with no entrance ─────
  //
  //     2026-09-06:「机器界面选中机器后,不要有图标的动效,现在是热力带的
  //     动效。直接显示全」. Measured on a fresh mount — the peer's card
  //     opened again — and on the glass: the block wears `nofx` from
  //     the first paint, and the computed `animation-name` on every
  //     element that would wipe, fade or draw is `none`. The class
  //     alone would pass a stylesheet that stopped honouring it, and
  //     the computed style alone would pass a card whose figure never
  //     drew; the band being there is the probe-alive half.
  {
    await open("devices");
    await pressNth("[data-list] [data-row-open]", 1);
    const drawn = await until(5_000, () =>
      page.evaluate(() => !!document.querySelector("[data-machine-card] [data-day-band] [data-day-row]")),
    );
    const arrival = await page.evaluate(() => {
      const blk = document.querySelector("[data-machine-card] [data-load-day] .dayblk");
      if (!blk) return null;
      const moving = [...blk.querySelectorAll(".wipe, .fade, .draw")];
      const named = moving.map((n) => getComputedStyle(n).animationName).filter((a) => a && a !== "none");
      return { nofx: blk.classList.contains("nofx"), moving: moving.length, animated: named.length, names: [...new Set(named)] };
    });
    if (!drawn || !arrival || arrival.moving === 0) {
      fail(`probe dead: the reopened card drew no figure (${JSON.stringify(arrival)}), so 「no entrance」 measured nothing`);
    } else {
      if (!arrival.nofx) fail("the day block opens without `nofx` — the figure plays its entrance on every card opened");
      if (arrival.animated > 0) {
        fail(`${arrival.animated} of ${arrival.moving} figure elements are animating on arrival (${arrival.names.join(",")}) — the day is to be drawn whole when a machine is picked`);
      }
    }
  }
  await page.setViewportSize({ width: 380, height: 800 });
  await wait(200);

  //     ── the scatter: three quantities, and nought kept apart
  //
  //     **The third quantity is the dot's own area** (2026-09-08; the
  //     halo it replaces has a tombstone below and another in
  //     `MachineCard`). Two appearances, not three, and the second is
  //     the one that matters: a reading of **zero** wears a dashed
  //     ring, because the area's floor is a visible dot and so nought
  //     and the smallest reading come out nearly the same size. Merging
  //     those two would turn 「read zero」 into 「the smallest」, which is
  //     the neighbouring-value mistake this whole surface is shaped to
  //     avoid. **A quantity some rows do not report cannot be an area
  //     at all** — that is the picker's job, asserted in section 10.
  const plot = await page.evaluate(() => {
    const box = document.querySelector("[data-machine-card] [data-top-plot]");
    if (!box) return null;
    // **The dots' radii, since 2026-09-08.** These used to be the
    // halos' — a ring around a fixed dot, encoding 显存. The ring is
    // gone (`MachineCard`'s tombstone says why), and the third quantity
    // is the dot's own area.
    const areas = [...box.querySelectorAll("[data-plot-area]")].map((n) => Number(n.dataset.plotArea));
    return {
      points: box.querySelectorAll("[data-plot-point]").length,
      areas,
      zeros: box.querySelectorAll("[data-plot-zero]").length,
      //   The area sentence is gone (2026-09-10); this reads whatever
      //   the figure's own text is, for the message when the axes are
      //   not named.
      foot: box.textContent?.trim().slice(0, 60) ?? "",
      axes: box.textContent.includes("CPU") && box.textContent.includes("内存"),
    };
  });
  if (!plot) fail("no scatter on the card — every process was to be on the picture");
  else {
    if (plot.points !== load.top.length) fail(`the scatter draws ${plot.points} points for ${load.top.length} rows`);
    if (!plot.axes) fail(`the scatter does not name its axes: "${plot.foot}"`);
    //   ⚠️ **Tombstone: 「one halo per row holding card memory」 was
    //   asserted here.** The halo is gone, so the claim is not failing,
    //   it stopped existing — see `MachineCard`. What replaces it is
    //   the property the halo also had and the dot now carries:
    //
    //   **Area, not radius** — and ⚠️ **that half is not asserted
    //   here.** The count and the spread below are; the *shape* of the
    //   encoding is checked on the 額度 half (section 8a, the line fit
    //   through (reading, r²)), and it covers this card only because
    //   both figures call `lib/plot`'s one `dotRadius`. Torn on
    //   2026-09-08 to be sure: a linear radius reddens that fit and
    //   leaves everything below untouched, because the endpoints are
    //   the same either way. What is measured here is `data-plot-area`,
    //   the radius the circle was actually drawn with.
    //
    //   **Every point has one**, which is the other half of the ruling:
    //   the area quantity is filtered to what every row reports, so a
    //   dot without a radius would mean that filter let a null through.
    if (plot.areas.length !== plot.points) {
      fail(`${plot.areas.length} dots carry an area for ${plot.points} points — the area quantity is meant to be one every row has`);
    } else if (!plot.areas.every((r) => r > 0)) {
      fail(`a dot was drawn at radius ${Math.min(...plot.areas)} — the smallest reading is still a dot`);
    } else {
      const lo = Math.min(...plot.areas);
      const hi = Math.max(...plot.areas);
      // The floor is `dotSpan`'s `min`, so what varies with the reading
      // is the area above it. Four times the area is twice the radius —
      // stated as the property rather than as a number, because the
      // range itself now moves with how many points there are.
      //
      // **A whole pixel, not merely a difference.** The halo assertion
      // this replaces asked for a 5% spread; `hi > lo` on its own would
      // pass on a thousandth of a pixel, which is an encoding nobody
      // can read. One device pixel is the floor with a reason of its
      // own, and the shape of the scale says any real spread clears it:
      // the smallest dot is 1.8 across, and a reading four times bigger
      // doubles the radius — so the moment two readings differ enough
      // to matter, they differ by more than a pixel.
      if (!(hi - lo >= 1)) {
        fail(`the dots run ${lo}–${hi} across — under a pixel apart, and with readings this far apart that is an encoding that is not there`);
      }
      console.log(`dot areas: ${plot.areas.length} radii, ${lo}–${hi}`);
    }
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
    //   （「the biggest and smallest halo are not the same size」 used to
    //   be asserted here. It did not stop existing — it moved up into
    //   the block above, onto the dots that carry the quantity now.）
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
          //   The two ends the drawing says its x axis runs between,
          //   and the first tick — its value and where it is drawn.
          //   2026-09-06 the axis stopped starting at zero (「根据数据
          //   的最大最小值自适应调整 x 和 y 的范围」), so the claim is no
          //   longer 「zero is at zero」 but 「the smallest reading is the
          //   first tick, drawn where the smallest point is」.
          xLo: Number(plot.dataset.plotXLo),
          xHi: Number(plot.dataset.plotXHi),
          yLo: Number(plot.dataset.plotYLo),
          yHi: Number(plot.dataset.plotYHi),
          firstTick: (() => {
            const gs = [...plot.querySelectorAll("[data-plot-tick-x]")];
            if (!gs.length) return null;
            const g = gs.reduce((a, b) => (Number(b.dataset.plotTickX) < Number(a.dataset.plotTickX) ? b : a));
            return { v: Number(g.dataset.plotTickX), x: +(+g.querySelector("line").getAttribute("x1")).toFixed(2) };
          })(),
          minPointX: pts.length ? Math.min(...pts.map((p) => p.x)) : null,
          maxPointX: pts.length ? Math.max(...pts.map((p) => p.x)) : null,
          //   Where the axis itself starts — the vertical rule at the
          //   plot's left edge — and where the drawing ends on the
          //   right. Comparing the first tick with the *points* alone
          //   would not show a ruler shifted off its origin: both come
          //   from the same transform and move together.
          axisX: (() => {
            const lines = [...plot.querySelectorAll("svg > line")];
            const v = lines.find((l) => l.getAttribute("x1") === l.getAttribute("x2"));
            return v ? +(+v.getAttribute("x1")).toFixed(2) : null;
          })(),
          axisRight: (() => {
            const lines = [...plot.querySelectorAll("svg > line")];
            const h = lines.find((l) => l.getAttribute("y1") === l.getAttribute("y2"));
            return h ? +(+h.getAttribute("x2")).toFixed(2) : null;
          })(),
        };
      });
    //   **The label has to be opened before a word can be pressed**
    //   (2026-09-05:「通过点击label,去切换X或y的值」). Collapsed, the
    //   axis carries one word — the quantity it is drawn with — and the
    //   four candidates are one press away. Opening it when it is
    //   already open would close it again, so the count decides.
    // **Open the settings, then pick.** Until 2026-09-08 the axis's
    // own word was the control and this pressed it; the user moved
    // switching into one button (「都做成一个小的设置按钮」), so the act
    // is two presses now and the label is a word again. Opening a panel
    // that is already open would close it, so the count still decides.
    const pick = async (which, key) => {
      if ((await page.locator("[data-machine-card] [data-plot-settings-open]").count()) === 0) {
        await page.locator("[data-machine-card] [data-plot-settings]").click();
        await wait(120);
      }
      await page
        .locator(`[data-machine-card] [data-plot-settings-open] [data-plot-set="${which}"] [data-plot-axis="${key}"]`)
        .click();
      await page.locator("[data-machine-card] [data-plot-settings]").click();
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
      //   **The axis is the data's own span, and the first tick is the
      //   smallest reading, drawn where the smallest point is.**
      //   2026-09-06:「散点图 x 轴 0% 右侧空了一大截,应该根据数据的最大
      //   最小值自适应调整 x 和 y 的范围」. Three parts, each one of the
      //   ways this is done wrong:
      //
      //     - the two ends the drawing claims are the fixture's own
      //       minimum and maximum on that axis — computed here from
      //       the rows, so an axis pinned back to zero (the old
      //       picture) reads a different `lo` and goes red;
      //     - the lowest tick's *label* is that minimum, and its *line*
      //       is at the leftmost point's x — a `log(v+1)`-style ruler
      //       that cannot put its origin where its own label says
      //       lands the tick and the point apart;
      //     - the leftmost point sits a few pixels inside the axis
      //       line, not on it, and the rightmost the same distance
      //       inside the frame's right end: a dot on the frame is half
      //       a dot, and the room is the same on both sides.
      //
      //   `now` is CPU × 内存, and `x` is CPU on every fixture row, so
      //   the expected ends are read straight off `load.top`.
      const cpuVals = load.top.map((r) => r.cpu_pct);
      const expectLo = Math.min(...cpuVals);
      const expectHi = Math.max(...cpuVals);
      if (now.firstTick === null || now.axisX === null || now.axisRight === null || now.minPointX === null) {
        fail(`probe dead: no first tick / axis / points (${JSON.stringify([now.firstTick, now.axisX, now.axisRight, now.minPointX])}), so the axis range measured nothing`);
      } else {
        if (Math.abs(now.xLo - expectLo) > 1e-6 || Math.abs(now.xHi - expectHi) > 1e-6) {
          fail(
            `the x axis says it runs ${now.xLo}–${now.xHi} while the rows run ${expectLo}–${expectHi} — ` +
              `an axis that starts anywhere but the smallest reading spends its width on a stretch no point can be in`,
          );
        }
        if (Math.abs(now.firstTick.v - Math.round(expectLo * 10) / 10) > 0.051) {
          fail(`the lowest x tick reads ${now.firstTick.v}, not the smallest reading ${expectLo}`);
        }
        if (Math.abs(now.firstTick.x - now.minPointX) > 0.6) {
          fail(
            `the lowest tick is drawn at x=${now.firstTick.x} while the leftmost point is at ${now.minPointX} — ` +
              `a ruler that cannot put its origin where its own label says has labels that lie about where they sit`,
          );
        }
        const left = now.minPointX - now.axisX;
        const right = now.axisRight - now.maxPointX;
        if (!(left >= 4 && left <= 12)) {
          fail(`the leftmost point sits ${left.toFixed(1)}px from the axis line — it is meant to be just inside it, not on it and not adrift`);
        }
        if (Math.abs(left - right) > 0.6) {
          fail(`the room inside the frame is ${left.toFixed(1)}px on the left and ${right.toFixed(1)}px on the right — the two ends are the same reading's margin`);
        }
      }
      console.log(`plot rulers: ${Object.entries(crowd).map(([k, v]) => `${k} ${v} crowded`).join(", ")}, said 「${now.said}」`);
      await pick("y", "mem");
      await wait(200);
    }
  }

  //     ── every tick wears a label of its own ────────────────────
  //
  //     2026-09-06, seen on this Mac's own table once the axis began
  //     at the data's minimum: `0.00% 0.01% 0.01% 0.02% 0.12% 0.94%
  //     7.6%`. Two labels reading the same over two positions break
  //     the one promise a tick makes (`ruler`'s: the label is a real
  //     reading, drawn where that reading belongs), and a `0.00%` on
  //     an axis whose lowest reading is above zero is a reading that
  //     was never taken. So, on both axes and on two pages:
  //
  //       - no two ticks on an axis print the same label;
  //       - a label reads as zero only for the reading zero — at most
  //         one such label, and only when the axis really starts at 0.
  //
  //     **Measured on a page built to crowd the bottom of the axis**
  //     — the 读写 order answers with CPU readings three decades under
  //     one percent, two of them a millionth apart, which is the shape
  //     that made the old two-decimal rule print the same label thrice.
  //     The default page is measured too, as the arm that says the
  //     crowding page is the one doing the work.
  {
    const labels = () =>
      page.evaluate(() => {
        const plot = document.querySelector("[data-machine-card] [data-top-plot]");
        if (!plot) return null;
        return {
          pair: plot.dataset.plotAxes,
          xLo: Number(plot.dataset.plotXLo),
          x: [...plot.querySelectorAll("[data-plot-tick-x] text")].map((t) => t.textContent.trim()),
          y: [...plot.querySelectorAll("[data-plot-tick-y] text")].map((t) => t.textContent.trim()),
        };
      });
    const judge = (got, page) => {
      if (!got || got.x.length < 3 || got.y.length < 3) {
        fail(`probe dead: the ${page} page draws ${got?.x.length ?? 0}×${got?.y.length ?? 0} tick labels, so 「each tick has its own label」 measured nothing`);
        return;
      }
      const [xk, yk] = (got.pair ?? "x").split("x");
      for (const [axis, ls, key] of [["x", got.x, xk], ["y", got.y, yk]]) {
        const dup = ls.filter((l, i) => ls.indexOf(l) !== i);
        if (dup.length) fail(`the ${page} page's ${axis} axis prints the same label twice: ${dup.map((d) => `「${d}」`).join(" ")} (all: ${ls.join(", ")})`);
        //   **All seven, on an axis whose labels take decimals.** A
        //   tick quietly dropped is how a broken formatter would pass
        //   the line above — measured: with the fold applied to every
        //   axis, one decimal for all reduced the crowded page to five
        //   ticks and the duplicate arm never fired. 时长 alone may
        //   fold (its unit ladder has no decimals to add).
        if (key !== "time" && ls.length !== 7) {
          fail(`the ${page} page's ${axis} axis (${key}) draws ${ls.length} ticks, not seven — a tick was dropped rather than labelled apart (${ls.join(", ")})`);
        }
      }
      const zeros = got.x.filter((l) => parseFloat(l) === 0);
      const allowed = got.xLo === 0 ? 1 : 0;
      if (zeros.length > allowed) {
        fail(`the ${page} page's x axis prints ${zeros.length} labels that read as zero (${zeros.join(", ")}) for an axis starting at ${got.xLo} — a reading above zero written as 0`);
      }
    };
    judge(await labels(), "default");
    //   Wide, because the 读写 head is folded away at the narrow stop
    //   the scatter sections run at — a press on a head that is not on
    //   screen waits thirty seconds and then throws. Put back after.
    await page.setViewportSize({ width: 1400, height: 900 });
    await wait(300);
    await page.locator('[data-machine-card] [data-top-head="io"] button').click();
    const arrived = await until(8_000, async () => (await labels())?.xLo === 0);
    if (!arrived) fail("probe dead: ordering by 读写 never drew the crowded page (x lo stayed above 0)");
    else {
      const got = await labels();
      judge(got, "crowded");
      console.log(`plot ticks (crowded page): x ${got.x.join(" ")} · y ${got.y.join(" ")}`);
    }
    await page.locator('[data-machine-card] [data-top-head="cpu"] button').click();
    await until(8_000, async () => (await labels())?.xLo > 0);
    await page.setViewportSize({ width: 380, height: 800 });
    await wait(300);
    //   Back where the sections below expect the figure: on screen.
    //   Their first reading is a resting position, and a plot left
    //   below the fold by the resize here would be scrolled into view
    //   by their first press — read as 「the picker moved the figure」.
    await page.locator("[data-machine-card] [data-top-plot]").scrollIntoViewIfNeeded();
    await wait(300);
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
          //   **Which pair the figure is drawn with, read off the
          //   figure** — not off the picker's buttons, which are not
          //   there when the label is closed (2026-09-05: the picker
          //   collapses after a choice, and reading the buttons made
          //   this report an empty pair for a plot that had just
          //   switched correctly). `data-plot-axes` is written by the
          //   thing that does the drawing, which is the honest place
          //   to ask.
          chosen: (plot.dataset.plotAxes ?? "").split("x").join(","),
          off: plot.querySelector("[data-plot-off]")?.dataset.plotOff ?? null,
          //   **The candidates live in the settings panel since
          //   2026-09-08**, not inside the label. Counting them where
          //   they used to be would answer 0 forever — a selector that
          //   still matches something and never matches the thing.
          words: [...plot.querySelectorAll("[data-plot-settings-open] [data-plot-axis]")].length,
          rows: [...plot.querySelectorAll("[data-plot-settings-open] [data-plot-set]")].map((n) => n.dataset.plotSet).join(","),
          //   Collapsed, each axis carries one word — and it has to be
          //   the quantity that axis is *drawn with*, not a fixed
          //   caption. A label reading 「横」 would pass a check that
          //   only counted two of them.
          labels: [...plot.querySelectorAll("[data-plot-label]")]
            .map((b) => `${b.dataset.plotLabel}:${b.textContent.trim()}`)
            .join(","),
          //   Where the drawing starts. **Opening a picker must not
          //   move it** — that is the whole 「同一位置、同一高度」 rule,
          //   and a build that pushed the figure down on every press
          //   would still switch axes correctly.
          //   ⚠️ **`[data-plot-svg]`, not `svg`.** The axis labels carry
          //   chevrons, so 「the first svg in the block」 is a 12px icon —
          //   measured, and it reported the drawing as 12 pixels tall
          //   and 「the picker pushed the figure down by 36」 on a build
          //   where nothing moved at all.
          svgTop: (() => {
            const el = plot.querySelector("[data-plot-svg]");
            return el ? Math.round(el.getBoundingClientRect().top) : null;
          })(),
          svgHeight: (() => {
            const el = plot.querySelector("[data-plot-svg]");
            return el ? Math.round(el.getBoundingClientRect().height) : null;
          })(),
          //   The whole block, because the two labels sit on opposite
          //   sides of the drawing: opening the one *below* it cannot
          //   move the drawing's top however tall it grows, so a check
          //   that watched only that would pass a build where the
          //   horizontal picker pushed everything under it down.
          plotHeight: Math.round(plot.getBoundingClientRect().height),
        };
      });
    //   **The label has to be opened before a word can be pressed**
    //   (2026-09-05:「通过点击label,去切换X或y的值」). Collapsed, the
    //   axis carries one word — the quantity it is drawn with — and the
    //   four candidates are one press away. Opening it when it is
    //   already open would close it again, so the count decides.
    // **Open the settings, then pick.** Until 2026-09-08 the axis's
    // own word was the control and this pressed it; the user moved
    // switching into one button (「都做成一个小的设置按钮」), so the act
    // is two presses now and the label is a word again. Opening a panel
    // that is already open would close it, so the count still decides.
    const pick = async (which, key) => {
      if ((await page.locator("[data-machine-card] [data-plot-settings-open]").count()) === 0) {
        await page.locator("[data-machine-card] [data-plot-settings]").click();
        await wait(120);
      }
      await page
        .locator(`[data-machine-card] [data-plot-settings-open] [data-plot-set="${which}"] [data-plot-axis="${key}"]`)
        .click();
      await page.locator("[data-machine-card] [data-plot-settings]").click();
      await wait(250);
    };
    const start = await read();
    //   **Two labels, closed, each naming its own axis's quantity**
    //   (2026-09-05:「散点图搞成带X和Y label的」). Before the swap the
    //   two axes had no name at all — only ticks and a ruler mark — so
    //   「这根轴是什么」 was answered by counting which of eight words in
    //   the foot was bold.
    if (!start) {
      fail("probe dead: no plot, so nothing about its axis labels was measured");
    } else if (start.labels !== "y:内存,x:CPU") {
      fail(`the axes are labelled ${JSON.stringify(start.labels)} — each axis has to say the quantity it is drawn with`);
    } else if (start.words !== 0) {
      fail(`${start.words} axis words are showing before anything was pressed — the label is supposed to be one word`);
    }
    //   **Opening one must not move the drawing.** Measured rather than
    //   reasoned: four buttons where one was is a real layout change,
    //   and the reason it does not push is that both states are a row
    //   of `min-h-touch` buttons — which is a fact about the
    //   stylesheet, not about the markup, so only the glass can say it.
    // How many quantities a row offers — read off the page rather than
    // written down here, so adding one to `AXES` does not make this
    // gate red about a thing that is not wrong.
    const AXIS_COUNT = await page.evaluate(() => {
      const b = document.querySelector("[data-machine-card] [data-plot-settings]");
      return b ? (b.getAttribute("aria-label") || "").split(" / ").length : 0;
    }).then(async () => {
      await page.locator("[data-machine-card] [data-plot-settings]").click();
      await wait(200);
      const n = await page.locator('[data-machine-card] [data-plot-settings-open] [data-plot-set="x"] [data-plot-axis]').count();
      await page.locator("[data-machine-card] [data-plot-settings]").click();
      await wait(200);
      return n;
    });
    if (AXIS_COUNT < 2) fail(`probe dead: the settings offer ${AXIS_COUNT} quantities on the x row, so nothing below counted anything`);
    for (const which of start ? ["y", "x"] : []) {
      //   **Read the closed state again here, not once at the top.**
      //   The figure sizes itself from its box through a
      //   `ResizeObserver`, so a reading taken in the frame after a
      //   layout change catches an svg 12px tall — measured, and it
      //   reported a 36px 「the picker pushed the figure down」 that was
      //   the drawing arriving rather than anything moving.
      await wait(400);
      const shut = await read();
      //   ⚠️ **The claim on this line changed on 2026-09-08, it was not
      //   deleted.** It used to be 「pressing the label opens the four
      //   candidates」 — the 2026-09-05 gesture. The user moved
      //   switching into one button, so that sentence is false **by
      //   design** now, and the two halves it becomes are:
      //
      //     - pressing the **label** opens nothing (it is a word again,
      //       and a word that still opened a list would be the old
      //       gesture surviving its own removal);
      //     - pressing the **settings button** opens every row.
      await page.locator(`[data-machine-card] [data-plot-label="${which}"]`).click();
      await wait(300);
      const pressedLabel = await read();
      if (pressedLabel.words !== 0) {
        fail(`pressing the ${which} axis label opened ${pressedLabel.words} words — the label is a word now, not the control`);
      }
      await page.locator("[data-machine-card] [data-plot-settings]").click();
      await wait(400);
      const open = await read();
      if (open.rows !== "x,y,z") {
        fail(`the settings opened rows ${JSON.stringify(open.rows)}, and this figure sets x, y and the dot's area`);
      }
      //   **The area row offers fewer quantities than an axis row, and
      //   that is the whole ruling of 2026-09-08.** An axis may be
      //   drawn against a quantity some rows do not report — the row is
      //   simply not placed, and the figure says how many. **An area
      //   cannot**: the point has to exist because it carries X and Y,
      //   so a missing reading has nowhere to go but the floor, and a
      //   floor-sized dot reads as 「the smallest」. So the picker only
      //   offers what **every** row has (`lib/plot`'s `everyRowHas`).
      //
      //   On this fixture 显存 is on 2 of 23 rows and 时长 on 22 of 23,
      //   so both are offerable as an axis and neither as an area —
      //   which is the difference this counts. **Nobody maintains a
      //   list**: the day a machine reports video memory for every
      //   process, that count rises on its own.
      const rowWords = async (row) =>
        page.locator(`[data-machine-card] [data-plot-settings-open] [data-plot-set="${row}"] [data-plot-axis]`).count();
      //   ⚠️ **Tombstone: 「the panel opened 2 × AXIS_COUNT words」 was
      //   asserted here.** With three rows of which one is deliberately
      //   shorter, a single product is the wrong shape for the claim —
      //   and it would have been satisfied by three rows offering
      //   everything. The two counts below are what it becomes.
      const [xWords, zWords] = [await rowWords("x"), await rowWords("z")];
      if (!(zWords >= 1)) {
        fail(`the area row offers ${zWords} quantities — an empty picker is not a filter, it is a broken one`);
      } else if (!(zWords < xWords)) {
        fail(
          `the area row offers ${zWords} quantities and an axis row offers ${xWords} — on this fixture 显存 has a reading ` +
            `on 2 of 23 rows, and a quantity nobody reported must not be offerable as a dot's size`,
        );
      }
      //   **Both measurements, because the labels sit on opposite
      //   sides.** The vertical one is above the drawing, so growing
      //   it moves the drawing's top; the horizontal one is below, so
      //   growing it moves nothing above it and only the block's own
      //   height shows it.
      if (open.svgTop !== shut.svgTop || open.svgHeight !== shut.svgHeight || open.plotHeight !== shut.plotHeight) {
        fail(
          `opening the ${which} axis label moved the figure (top ${shut.svgTop}→${open.svgTop}, ` +
            `svg ${shut.svgHeight}→${open.svgHeight}, block ${shut.plotHeight}→${open.plotHeight}) — ` +
            `the picker has to open in place`,
        );
      }
      //   And the other axis stays shut, so 「one place, contents
      //   swapped」 is true of each axis separately.
      //   ⚠️ **Tombstone: 「opening one label did not open the other」
      //   was asserted here, and it cannot be any more.** The two axes
      //   had a picker each, so 「the pickers are independent」 was a
      //   real property; since 2026-09-08 there is **one** panel with a
      //   row per axis, and both rows are supposed to be there at once
      //   — which is what `open.rows === "x,y,z"` above now says. The
      //   claim did not fail, it stopped existing.
      //
      //   Both labels still have to keep naming their own quantity
      //   while the panel is open, and that is what this checks.
      const other = which === "x" ? "y" : "x";
      if (!new RegExp(`${other}:`).test(open.labels)) {
        fail(`with the settings open the ${other} axis stopped naming its quantity (labels ${JSON.stringify(open.labels)})`);
      }
      //   Put it back the way it was, by choosing what was already
      //   chosen — the rest of this section starts from cpu×mem.
      await pick(which, which === "x" ? "cpu" : "mem");
    }
    if (!start) {
      // already failed above
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

  //     ── a point says what it is, and takes you to its row ────────
  //
  //     2026-09-06:「悬浮散点要显示命令名。点击散点要定位到表格的行,
  //     并高亮一瞬」. Two claims, and the expensive half of each is not
  //     「did something happen」 but 「did the right thing happen」.
  {
    //   **Same string, character for character, as the table's cell.**
    //   Two places assembling their own version of 「what this process
    //   is」 drift, and the day one row reads `python3` on the dot and
    //   `python3 train.py` in the table, no gate here would say a word
    //   — a reader would spot it at once. So this compares the two
    //   *rendered* strings rather than checking that both call the
    //   same helper: the helper is the fix, this is the evidence.
    const say = await page.evaluate(() => {
      const pts = [...document.querySelectorAll("[data-machine-card] [data-plot-point]")];
      const out = [];
      for (const g of pts.slice(0, 40)) {
        const pid = g.dataset.plotPoint;
        const t = g.querySelector("title");
        const cell = document.querySelector(`[data-machine-card] [data-top-row="${pid}"] [data-cmd-line]`);
        if (!t || !cell) continue;
        out.push({ pid, said: t.textContent.split("\n")[0].trim(), cell: cell.textContent.trim() });
      }
      return out;
    });
    if (say.length < 5) {
      fail(`probe dead: ${say.length} points could be paired with a row, so 「the dot says what the table says」 measured nothing`);
    } else {
      const off = say.filter((x) => x.said !== x.cell);
      if (off.length) {
        fail(
          `${off.length} of ${say.length} points name their process differently from the table ` +
            `(pid ${off[0].pid}: dot ${JSON.stringify(off[0].said)} vs table ${JSON.stringify(off[0].cell)}) — ` +
            `two places assembling the same string is two places to drift`,
        );
      } else {
        console.log(`plot hover: ${say.length} points say exactly what their row says`);
      }
    }

    //   **A press on a point lands on that row, below the head.**
    //
    //   ⚠️ The row is chosen as **the last one that also has a point**,
    //   so the table has to actually scroll to reach it. On a table
    //   short enough to fit, every arrangement passes — including the
    //   one that scrolls the row under the sticky head, which is the
    //   bug this is here for.
    //   Wide enough that the table is a grid with sticky heads — the
    //   stacked layout has no heads at all, and 「below the head」 there
    //   is a question about an element that is `display: none`.
    await page.setViewportSize({ width: 1400, height: 1000 });
    await wait(400);
    const want = await page.evaluate(() => {
      const pts = new Set([...document.querySelectorAll("[data-machine-card] [data-plot-point]")].map((g) => g.dataset.plotPoint));
      const rows = [...document.querySelectorAll("[data-machine-card] [data-top-row]")];
      const box = document.querySelector("[data-machine-card] [data-load-top-box]");
      const floor = box.getBoundingClientRect().bottom;
      //   ⚠️ **The first row that is out of sight below the window —
      //   not the last row.** Scrolling to the last one runs the box to
      //   its maximum, and a maximum is the same wherever you asked to
      //   go: measured, the arm that removed the sticky head's height
      //   from the scroll **stayed green** against the last row,
      //   because the clamp put it in the right place anyway. A row
      //   just past the fold needs an exact scroll, and an exact scroll
      //   is the only kind that can be wrong by the height of a head.
      const hit = rows.find(
        (r) => pts.has(r.dataset.topRow) && r.firstElementChild.getBoundingClientRect().top > floor,
      );
      return {
        pid: hit ? hit.dataset.topRow : null,
        rows: rows.length,
        hot: document.querySelectorAll("[data-machine-card] [data-top-row][data-hot]").length,
        scrollTop: box ? Math.round(box.scrollTop) : null,
        scrollable: box ? box.scrollHeight - box.clientHeight : null,
      };
    });
    if (!want.pid) {
      fail("probe dead: no point shares a pid with a row, so 「a press goes to its row」 had nothing to press");
    } else if (want.hot !== 0) {
      //   The aliveness half: if a row were already marked, 「it is
      //   marked after the press」 would pass without a press.
      fail(`${want.hot} rows were already highlighted before anything was pressed — the probe cannot tell`);
    } else {
      await page.locator(`[data-machine-card] [data-plot-point="${want.pid}"]`).click();
      await wait(400);
      const got = await page.evaluate(() => {
        const hot = [...document.querySelectorAll("[data-machine-card] [data-top-row][data-hot]")];
        const head = document.querySelector("[data-machine-card] [data-top-heads] > *");
        const box = document.querySelector("[data-machine-card] [data-load-top-box]");
        const cell = hot[0]?.firstElementChild;
        const c = cell?.getBoundingClientRect();
        const h = head?.getBoundingClientRect();
        const b = box?.getBoundingClientRect();
        return {
          n: hot.length,
          pid: hot[0]?.dataset.topRow ?? null,
          //   ⚠️ **Two edges, not one.** 「below the head」 alone is
          //   satisfied by a row that never came into view at all —
          //   measured: the first version of this reported a row
          //   **767px** below the head and passed. Visible means below
          //   the head *and* above the box's own bottom.
          under: c && h ? +(c.top - h.bottom).toFixed(1) : null,
          within: c && b ? +(b.bottom - c.bottom).toFixed(1) : null,
          scrollTop: box ? Math.round(box.scrollTop) : null,
          tint: cell ? getComputedStyle(cell).backgroundColor : null,
        };
      });
      if (got.n !== 1) fail(`pressing a point highlighted ${got.n} rows, not one`);
      else if (got.pid !== want.pid) fail(`pressing the point for pid ${want.pid} highlighted pid ${got.pid} instead`);
      else if (!(got.under >= -1)) {
        fail(
          `the row a point asked for sits ${got.under}px above the sticky head's bottom — it scrolled *behind* the ` +
            `heads, which is the way this lands on a table long enough to scroll`,
        );
      } else if (!(got.within >= -1)) {
        fail(
          `the row a point asked for is ${got.within}px past the bottom of the table's window — it was marked ` +
            `but never brought into view, and 「below the head」 alone cannot tell those apart`,
        );
      } else if (want.scrollable > 0 && got.scrollTop === want.scrollTop) {
        fail(
          `the table did not scroll (${got.scrollTop} either side, ${want.scrollable}px of scroll available) — ` +
            `the row was reachable only because it already happened to be on screen`,
        );
      } else if (!got.tint || /rgba?\(0, 0, 0, 0\)|transparent/.test(got.tint)) {
        fail(`the highlighted row's cells have no tint (${got.tint}) — the row is marked in the DOM and not on the glass`);
      } else {
        console.log(
          `plot press: pid ${got.pid} of ${want.rows} rows, scroll ${want.scrollTop}→${got.scrollTop} of ${want.scrollable}, ` +
            `${got.under}px below the head and ${got.within}px above the window's floor, tint ${got.tint}`,
        );
      }
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
  //     **⚠️ The mark is an ellipsis, and there is no square anywhere.**
  //     A 10px bordered box used to sit at the end of a cut cell;
  //     2026-09-05 the person using this said what it actually looked
  //     like — 「关键是它看着就是字体缺字呀」 — and it went. The
  //     replacement needed no code: the line already wears `truncate`,
  //     so the browser was drawing the ellipsis behind the square the
  //     whole time.
  //
  //     Two claims, because either alone lets it back: **the square is
  //     nowhere on the page** (a stylesheet or a component can bring it
  //     back without touching the words that argue against it), and
  //     **the line is still set to elide** — `text-overflow: ellipsis`
  //     with `overflow: hidden`, which is what makes the 「…」 appear at
  //     all. The second cannot be read off `textContent`: the DOM holds
  //     the whole string either way, and the ellipsis is painted, not
  //     stored.
  const elide = await page.evaluate(() => {
    const cut = document.querySelector('[data-machine-card] [data-c="cmd"][data-cut] [data-cmd-line]');
    const fits = [...document.querySelectorAll('[data-machine-card] [data-c="cmd"]:not([data-cut]) [data-cmd-line]')][0];
    const of = (n) => (n ? (({ textOverflow, overflow, whiteSpace }) => ({ textOverflow, overflow, whiteSpace }))(getComputedStyle(n)) : null);
    return { squares: document.querySelectorAll("[data-cut-mark]").length, cut: of(cut), fits: of(fits) };
  });
  if (elide.squares > 0) {
    fail(`${elide.squares} truncation squares are still drawn — the mark that reads as a missing glyph is back`);
  }
  if (!elide.cut) {
    fail("probe dead: no cut cell to read, so 「the ellipsis is what marks it」 measured nothing");
  } else if (elide.cut.textOverflow !== "ellipsis" || elide.cut.overflow !== "hidden") {
    fail(
      `a cut line is ${JSON.stringify(elide.cut)} — with no ellipsis and no clip the value simply stops, ` +
        `and nothing on screen says it was cut`,
    );
  }

  //     **The hover says the part that is missing, and it is not the
  //     only way to it.** 2026-09-05:「命令这种显示不全的,鼠标悬浮后
  //     显示」. Three things, and the first is what makes the other two
  //     mean anything.
  if (marks.marked > 0) {
    const hoverText = async (how) => {
      const before = await page.locator("[data-cmd-full]").count();
      const cell = page.locator('[data-machine-card] [data-c="cmd"][data-cut]').first();
      if (how === "hover") await cell.hover();
      else await cell.evaluate((n) => n.focus());
      await wait(300);
      const got = await page.locator("[data-cmd-full]").allTextContents();
      //   Put the pointer somewhere with nothing under it, so the next
      //   arm starts from closed.
      await page.mouse.move(2, 2);
      await page.evaluate(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());
      await wait(250);
      return { before, got };
    };
    //   ⚠️ **The probe has to be dead before it is alive.** A tooltip
    //   already on the page would let 「hovering shows the command」
    //   pass without a hover happening — the shape of false green this
    //   file keeps finding. So the count before the pointer moves is
    //   asserted to be zero.
    const byHover = await hoverText("hover");
    const line = await page
      .locator('[data-machine-card] [data-c="cmd"][data-cut] [data-cmd-line]')
      .first()
      .textContent();
    if (byHover.before !== 0) {
      fail(`${byHover.before} tooltips were already open before anything was hovered — the probe cannot tell`);
    } else if (byHover.got.length !== 1) {
      fail(`hovering a cut command opened ${byHover.got.length} tooltips, not one`);
    } else if (byHover.got[0].trim() !== line.trim()) {
      //   Verbatim, because 「it showed something」 is satisfied by a
      //   tooltip repeating the visible half.
      fail(
        `the hover says ${JSON.stringify(byHover.got[0].trim().slice(0, 60))}… while the row holds ` +
          `${JSON.stringify(line.trim().slice(0, 60))}… — it is not the same command`,
      );
    }
    //   **Focus opens it too**, because the narrow card has no pointer
    //   and a keyboard has no hover. The press stays the way in there —
    //   asserted below — and this is the second one, not a replacement.
    const byFocus = await hoverText("focus");
    if (byFocus.got.length !== 1) {
      fail(`focusing a cut command opened ${byFocus.got.length} tooltips — a keyboard cannot reach the hover`);
    }
    console.log(`cut command: ellipsis, no squares, hover and focus both say ${line.trim().length} chars`);
  }

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
      const bottom = scroller.getBoundingClientRect().bottom;
      let below = 0;
      //   Every visible grid, not the first: the disk drawer holds the
      //   volumes' grid and, under it, the watched directories' — the
      //   hook counts rows across both, and a probe that read one grid
      //   called the hook a liar on the other (2026-09-06).
      for (const row of scroller.querySelectorAll("[data-parts]:not([hidden]) [data-part]")) {
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
    //   ── both readings end where the slot ends ──────────────────
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
      for (const view of ["heat", "para"]) {
        await page.locator(`[data-machine-card] [data-day-view-pick="${view}"]`).click();
        await wait(500);
        floors[view] = await floorOf();
      }
      const got = Object.entries(floors).filter(([, v]) => v !== null);
      if (got.length !== 2) {
        fail(`probe dead: only ${got.length} of the two readings could be measured (${JSON.stringify(floors)})`);
      } else {
        const gaps = got.map(([, v]) => v.gap);
        const spread = Math.max(...gaps) - Math.min(...gaps);
        if (spread > 1) {
          fail(
            `the two readings stop at ${got.map(([k, v]) => `${k} ${v.gap}`).join(", ")} px above the slot's floor ` +
              `— a spread of ${spread.toFixed(1)}, so switching reading opens a strip of nothing under one of them`,
          );
        }
        //   ⚠️ **Equal is not enough, and this is the half that was
        //   missing.** Written with only the spread, the section passed
        //   on a build where all three (then) under-filled by the same 5px —
        //   two drawings agreeing on where to stop short is still the
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

    //   **And it fits the slot it shares, in both readings.** The
    //   slot is one height for the drawings and the drawers both, so
    //   the tallest reading is what sets it — and a reading that
    //   outgrows it is silently clipped at the bottom, where its
    //   readout line lives. Measured this way because the number that
    //   sets the height is arrived at by adding up parts, and one of
    //   them (the readout's `margin-top`) is not in any element's
    //   height: 叠放 overflowed by exactly that 6px on the first try.
    for (const view of ["heat", "para"]) {
      await page.locator(`[data-machine-card] [data-day-view-pick="${view}"]`).click();
      await wait(400);
      const over = await page.evaluate(() => {
        const blk = document.querySelector("[data-machine-card] [data-load-day] .dayblk");
        if (!blk) return null;
        return { over: blk.scrollHeight - blk.clientHeight, parts: [...blk.children].map((c) => `${c.className}:${Math.round(c.getBoundingClientRect().height)}`) };
      });
      if (!over) fail(`probe dead: no day block while showing ${view}`);
      else if (over.over > 0) {
        fail(`叠放/热力带 — ${view} is ${over.over}px taller than the slot it shares (${over.parts.join(" ")}) — its foot is clipped`);
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

  //   The stops above leave the shell at whatever width they last
  //   needed; both readings below have to come from one posture, so it
  //   is stated here rather than inherited.
  await page.setViewportSize({ width: 1400, height: 900 });
  //   ── one size slot, two kinds of mark ───────────────────────
  //   2026-09-07:「把这些环和机器头像,也就是 sessionlist 里的头像、
  //   机器 list 或机器页面的头像一视同仁,他们都是一种类型/实体。」
  //
  //   **Read on two screens, because that is the only place the claim
  //   lives.** `Face`'s `AVATAR_BOX` mirrors `--avatar` in `tokens.css`
  //   — a number copied from CSS into TS, which nothing in either
  //   language can keep in step. **This is what keeps the copy safe**,
  //   and it is deliberately not a check that two symbols point at one
  //   constant: those two `32`s are different quantities (`--avatar` is
  //   a box, `DIAL_ROW_SIZE` is a `size` parameter, and `Face` divides
  //   one to get the other), so an assertion about references would be
  //   blind to exactly the drift this is for. **The claim is about the
  //   screen, so the reading is off the screen.**
  //
  //   Heights only. An avatar is square and a 210° dial is 1.46 times
  //   as wide as it is tall — **they share a slot, not a footprint**,
  //   and squaring the dial up would mean squashing the arc.
  const markSlot = async () => {
    await open("sessions");
    await until(5_000, async () => (await page.locator("[data-list] [data-row]").count()) > 0);
    const mark = await page.evaluate(() => {
      const e = document.querySelector(
        "[data-list] [data-row] [data-face], [data-list] [data-row] [data-face-blank]",
      );
      return e ? +e.getBoundingClientRect().height.toFixed(1) : null;
    });
    await open("quota");
    await until(5_000, async () => (await page.locator("[data-quota-row]").count()) > 0);
    const dial = await page.evaluate(() => {
      const e = document.querySelector("[data-quota-row] [data-dial-face]");
      return e ? +e.getBoundingClientRect().height.toFixed(1) : null;
    });
    return { mark, dial };
  };
  const slotRead = await markSlot();
  if (slotRead.mark === null || slotRead.dial === null) {
    //   The avatars in this fixture draw as `AvatarBlank` (no face on
    //   the machines it routes), which is the same box sized by the
    //   same utility — but if **neither** selector matches, the
    //   comparison below would be two nulls agreeing.
    fail(
      `probe dead: the session row's mark measured ${slotRead.mark} and the 額度 row's dial ${slotRead.dial} — nothing below is comparing two marks`,
    );
  } else if (Math.abs(slotRead.mark - slotRead.dial) > 0.5) {
    fail(
      `a session row's mark is ${slotRead.mark} tall and a 額度 row's dial is ${slotRead.dial} — they are one size slot ` +
        `(「一视同仁」), and the slot is 「--avatar」 in tokens.css. **If the slot is meant to change, change that token ` +
        `and bring Face's AVATAR_BOX with it**; changing either one alone is the drift this measures, so do not ` +
        `settle it by editing whichever of the two you happened to open first.`,
    );
  }
  console.log(`size slot: session mark ${slotRead.mark} tall, 額度 dial ${slotRead.dial} tall`);

  //   ── the settings panel opens where it can be read whole ───────
  //
  //   2026-09-08, the user, on his first press after this batch went
  //   in: 「三个点点击了弹出的窗口都显示不全」.
  //
  //   ⚠️ **This is the hole that list called the second kind, and it
  //   came true the same night.** Section 10 already asserted how many
  //   rows the panel opens, how many quantities each row offers, that
  //   the label opens nothing, and that opening it moves no part of the
  //   figure — **all green, all torn down, and not one of them asked
  //   whether the thing was on the screen.** The assertions were true
  //   and would really go red; they were on a different axis from the
  //   risk.
  //
  //   Measured that night, before any fix (three widths, real
  //   `getBoundingClientRect`):
  //
  //     trend  @1400/1120/390 — 172.6px past the right edge at **every
  //                             width, the same number**, because the
  //                             button sits at the right end of its row
  //                             and the panel grew rightwards from it;
  //     stats  @390           — 155px past, because it is 474 across
  //                             for eight quantities and the screen is
  //                             390: no side to anchor to would fit it;
  //     machine@any           — fitted, and only because four
  //                             quantities make a narrower panel.
  //                             **Not spared, just not hit**: at 218.5
  //                             across it sits at left=46 with room to
  //                             spare, and the day that card offers a
  //                             few more quantities it walks the stats
  //                             panel's road.
  //
  //   So this reads **geometry off the glass**, at every width and on
  //   every figure that has one of these — not `data-*`, which is the
  //   product's own account of itself, and not one surface, because the
  //   machine card passed while two panels next door did not.
  //
  //   ⚠️ **This was torn down by putting the regression back, not by
  //   inventing a bad state** (2026-09-09 00:07:36). `PlotControls.tsx`
  //   was replaced with the file the user actually had — checked with
  //   `diff` against `70335e6` and byte-identical, `absolute
  //   bottom-full left-0` and `min-w-max`, not one character moved —
  //   and this section went red **four times, at the four places and
  //   with the four numbers measured off his screen's shape**:
  //   trend 172.6 at each of the three widths, stats 155.0 at 390.
  //
  //   The difference matters and is the reason it was done this way:
  //   a hand-made bad position proves the section **responds**; the
  //   shipped file proves it is on the axis a whole batch of green
  //   assertions next door had missed. Section 10 asserted the panel's
  //   rows, each row's offer, that the label opens nothing, and that
  //   opening it moves no part of the figure — five teardowns, all
  //   red, all true, **and every one of them blind to whether the
  //   thing was on the screen.**
  {
    const seen = [];
    const panelBox = (at) =>
      page.evaluate((s) => {
        const el = document.querySelector(s);
        if (!el) return null;
        const r = el.getBoundingClientRect();
        //   The box it has to stay inside is **whichever is tighter**:
        //   the nearest ancestor that clips, or the window. The 額度
        //   detail pane is `overflow:auto`, so on that page the pane is
        //   the real edge and the window would have said 「fits」.
        let n = el.parentElement;
        let left = 0;
        let right = window.innerWidth;
        let top = 0;
        let bottom = window.innerHeight;
        let by = "window";
        while (n) {
          const cs = getComputedStyle(n);
          if (cs.overflowX !== "visible" || cs.overflowY !== "visible") {
            const b = n.getBoundingClientRect();
            left = Math.max(left, b.left);
            right = Math.min(right, b.right);
            top = Math.max(top, b.top);
            bottom = Math.min(bottom, b.bottom);
            by = `${n.tagName.toLowerCase()}(${cs.overflowX}/${cs.overflowY})`;
            break;
          }
          n = n.parentElement;
        }
        return {
          x: +r.x.toFixed(1), y: +r.y.toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1),
          right: +r.right.toFixed(1), bottom: +r.bottom.toFixed(1),
          box: { left: +left.toFixed(1), right: +right.toFixed(1), top: +top.toFixed(1), bottom: +bottom.toFixed(1), by },
          //   **Proof the panel is a panel.** A `<div>` that renders
          //   nothing has a zero box that sits inside every edge —
          //   「一致地什么都没有」 passing a containment test is exactly
          //   this gate's way of being blind, so what it holds is
          //   counted here and asserted below.
          picks: el.querySelectorAll("[data-plot-axis]").length,
          rows: el.querySelectorAll("[data-plot-set]").length,
        };
      }, at);
    const fits = async (name, atBtn, atPanel) => {
      const buttons = await page.locator(atBtn).count();
      if (buttons !== 1) {
        fail(`probe dead: ${name} has ${buttons} settings buttons, so nothing about its panel was measured`);
        return;
      }
      await page.locator(atBtn).click();
      await wait(300);
      const p = await panelBox(atPanel);
      if (!p) {
        //   ⚠️ **This arm and the empty-panel one below cannot be made
        //   to fire today, and that is written here rather than left to
        //   look like coverage.** A panel that is not drawn satisfies
        //   「nothing sticks out」 perfectly, so an edge-comparing gate
        //   would go green on a component that renders nothing — the
        //   reason both arms exist. But **every earlier section presses
        //   into one of these panels**, so taking the panel away kills
        //   the run before it reaches here. Measured 2026-09-08:
        //   removing it outright dies in 8a's `pickUsage` (a 30s
        //   `locator.click` timeout, raw throw, whole run terminated);
        //   removing it only under 500px dies in section 10's `pick`,
        //   because that section measures the card at 380.
        //
        //   So the risk **is** covered — the gate goes red, loudly —
        //   just not by this line's words. These two arms are what
        //   keeps *this* section from going vacuously green if those
        //   earlier sections are ever re-ordered or dropped.
        fail(`probe dead: ${name}'s settings were pressed and no panel was drawn — 「it fits」 would have been true of nothing`);
        return;
      }
      if (p.rows === 0 || p.picks === 0 || p.w <= 0 || p.h <= 0) {
        fail(`probe dead: ${name}'s panel measures ${p.w}x${p.h} with ${p.rows} rows and ${p.picks} quantities — an empty box fits inside anything`);
      } else {
        const out = [
          p.x < p.box.left ? `left ${(p.box.left - p.x).toFixed(1)}` : "",
          p.y < p.box.top ? `top ${(p.box.top - p.y).toFixed(1)}` : "",
          p.right > p.box.right ? `right ${(p.right - p.box.right).toFixed(1)}` : "",
          p.bottom > p.box.bottom ? `bottom ${(p.bottom - p.box.bottom).toFixed(1)}` : "",
        ].filter(Boolean);
        if (out.length) {
          fail(
            `${name}: the settings panel (${p.w}x${p.h} at ${p.x},${p.y}) hangs ${out.join(" and ")} outside ` +
              `${p.box.by} ${p.box.left},${p.box.top}-${p.box.right},${p.box.bottom} — whatever is past that edge is ` +
              `cut off with no sign of it, which is 「弹出的窗口显示不全」. **The panel measures its own room** ` +
              `(components/PlotControls): if this is red, either that measurement stopped running or something above ` +
              `it started clipping — do not answer it by moving the button.`,
          );
        }
        seen.push(`${name} ${p.w}x${p.h}`);
      }
      await page.locator(atBtn).click();
      await wait(150);
    };
    //   **Three widths, because the two failures had different shapes**:
    //   one was the same at every width (an anchor on the wrong side)
    //   and the other only appeared at the narrow stop (a panel wider
    //   than the screen). A single width would have caught one of them.
    for (const w of [1400, 1120, 390]) {
      await page.setViewportSize({ width: w, height: 900 });
      await wait(300);
      await open("devices");
      await pressNth("[data-list] [data-row-open]", 1);
      await until(5_000, async () => (await page.locator("[data-machine-card] [data-top-plot]").count()) === 1);
      await fits(`machine@${w}`, "[data-machine-card] [data-plot-settings]", "[data-machine-card] [data-plot-settings-open]");
      await open("quota");
      await until(5_000, async () => (await page.locator("[data-quota-row]").count()) > 0);
      await press('[data-quota-row="sub:local"]');
      await wait(500);
      for (const zone of ["trend", "stats"]) {
        await fits(
          `${zone}@${w}`,
          `[data-quota-detail] [data-usage-zone="${zone}"] [data-plot-settings]`,
          `[data-quota-detail] [data-usage-zone="${zone}"] [data-plot-settings-open]`,
        );
      }
    }
    if (seen.length !== 9) {
      fail(`probe dead: ${seen.length} of 9 settings panels were measured (${seen.join(", ")}) — 「they all fit」 is a claim about all of them`);
    }
    console.log(`settings panels: ${seen.join(" · ")}`);
    await page.setViewportSize({ width: 1400, height: 900 });
    await wait(300);
  }


  //     ── the four dials divide the row, and none of them is alone ──
  //
  //     2026-09-09, the user:「机器的环,要铺平宽度,现在左右空太多。
  //     窄屏不允许一行只有一个环。我们一共四个,窄屏就一行两个」.
  //
  //     ⚠️ **Six widths, and the two that mattered are in the middle.**
  //     Measured that day before the change: 1400 and 1120 and 390 were
  //     all fine — four, four, and two-plus-two — while **900 and 560
  //     put the disk dial on a line by itself**. The user reported the
  //     narrow face and the narrow face was the one that worked; a gate
  //     at his three widths would have gone green on the bug he was
  //     describing. The row's width is what decides this and it is not
  //     monotonic in the viewport (642 wide at 700, **490 at 900**,
  //     because the list pane appears in between), so the widths here
  //     are chosen to straddle the row widths, not the screen sizes.
  //
  //     Before / after, at the six (row width → what was drawn):
  //
  //       1400 → 990: 4 dials of 123.5 in a 990 row, 221 of air on each
  //                   side   →  4 of 234, no air
  //       1120 → 710: 4 of 123.5, 81 each side  →  4 of 164, no air
  //        900 → 490: **3 + 1**                 →  2 + 2, each 236
  //        700 → 642: 4 of 123.5, 47 each side  →  4 of 147, no air
  //        560 → 502: **3 + 1**                 →  2 + 2, each 242
  //        390 → 332: 2 + 2 of 123.5            →  2 + 2, each 157
  {
    const seenDials = [];
    const seenFits = [];
    //   ⚠️ **360 is here because it is the phone the shrink is for.**
    //   The five wider stops draw the full size and would let a broken
    //   degradation pass by never reaching it: 390 gives this row 332,
    //   which is 2px inside the last size that fits, so it shrinks by
    //   one pixel and proves almost nothing. A 12/13 mini is 360 and
    //   its row is 302 — **30px short**, which is where the fit reading
    //   below has something to say.
    for (const w of [1400, 1120, 900, 700, 560, 390, 360]) {
      await page.setViewportSize({ width: w, height: 900 });
      await wait(400);
      await open("devices");
      await pressNth("[data-list] [data-row-open]", 1);
      await until(5_000, async () => (await page.locator("[data-machine-card] [data-vitals]").count()) === 1);
      await wait(200);
      const d = await page.evaluate(() => {
        const row = document.querySelector("[data-machine-card] [data-vitals]");
        if (!row) return null;
        const rr = row.getBoundingClientRect();
        const kids = [...row.children].map((k) => {
          const b = k.getBoundingClientRect();
          const face = k.querySelector("svg");
          return {
            unit: k.querySelector("[data-dial]")?.getAttribute("data-dial") ?? "?",
            x: +b.x.toFixed(1), y: +b.y.toFixed(1), w: +b.width.toFixed(1),
            svgW: face ? +face.getBoundingClientRect().width.toFixed(1) : null,
          };
        });
        const lines = {};
        for (const k of kids) (lines[Math.round(k.y)] = lines[Math.round(k.y)] || []).push(k.unit);
        return {
          row: { x: +rr.x.toFixed(1), w: +rr.width.toFixed(1) },
          cols: Number(row.getAttribute("data-vitals-cols")),
          kids,
          lines: Object.values(lines),
        };
      });
      if (!d || d.kids.length < 4) {
        //   Fewer than four dials is a different card (a machine with no
        //   GPU draws three), and every claim below is about how four
        //   are laid out — so this says so instead of passing on three.
        fail(`probe dead: the dial row at ${w} has ${d ? d.kids.length : "no"} dials, so 「four divide the row」 measured nothing`);
        continue;
      }
      //   ① **No line holds exactly one.** The user's rule, and the one
      //   the old `flex-wrap` broke: it packs as many as fit per line,
      //   so a row between three and four dials wide lands on 3 + 1.
      const alone = d.lines.filter((l) => l.length === 1);
      if (alone.length > 0) {
        fail(
          `at ${w} the dials sit ${JSON.stringify(d.lines)} — a line with one dial on it is what 「窄屏不允许一行只有一个环」 ` +
            `forbids, and it is what wrapping does on its own: this has to be a declared column count, not a leftover`,
        );
      }
      //   ② **Four or two, never three.** Three columns with four dials
      //   *is* 3 + 1, so this and ① are the same rule read at two ends —
      //   ① off the glass, ② off the decision.
      if (d.cols !== 4 && d.cols !== 2) {
        fail(`at ${w} the row declares ${d.cols} columns — four dials divide into four or two, and three of them is 3 + 1`);
      }
      //   ③ **They divide the row: equal shares, and no air left over.**
      //   「要铺平宽度,现在左右空太多」 — before this, 1400 left 221 on
      //   each side.
      const widths = [...new Set(d.kids.map((k) => k.w))];
      if (widths.length !== 1) {
        fail(`at ${w} the dials are ${JSON.stringify(widths)} wide — they divide one row and an unequal share is not a division`);
      }
      const leftAir = d.kids[0].x - d.row.x;
      const rightAir = d.row.x + d.row.w - (d.kids[d.kids.length - 1].x + d.kids[d.kids.length - 1].w);
      //   ⚠️ **Both signs.** This read `> 1` only, and a teardown that
      //   switched the shrink off went **green**: the dials then need
      //   334 in a row of 302, `space-between` leaves them flush left
      //   and lets them hang off the right, and a negative reading is
      //   not 「more than 1 of air」. **Air and overflow are the same
      //   axis measured from opposite ends** — a row that is exactly
      //   filled reads zero at both edges, and anything else is one of
      //   the two failures.
      if (Math.abs(leftAir) > 1 || Math.abs(rightAir) > 1) {
        fail(
          `at ${w} the row's edges read ${leftAir.toFixed(1)} on the left and ${rightAir.toFixed(1)} on the right — ` +
            `positive is air the dials were to fill, negative is dials hanging off the edge; the row is to be exactly filled`,
        );
      }
      //   ④ ⚠️ **Never bigger than the 額度 card's dial, and smaller
      //   only where the row cannot hold it.** For one day the size was
      //   a share of the row — it grew to fill its column — and the
      //   user ruled that out:「机器卡片环的大小,就是现在额度页的大小」.
      //   So the size is a number now, with **one** way down: two boxes
      //   at that number plus the floor gap need 334.1, and a 390-wide
      //   phone gives this row 332; a 12/13 mini is 360 and gives 302.
      //   The dial yields there rather than the gap, and 「一行只剩一个
      //   环」 stays forbidden.
      //
      //   The full size is not written here — it is read from the
      //   widest posture in this same loop, where nothing is shrinking.
      const box = d.kids[0].svgW;
      seenDials.push(`${w}:${d.cols}×${box}`);
      //   **The centre reading still fits, at the size actually drawn.**
      //   This is the arm that replaces 「find the size below which
      //   `100%` overflows」: that number does not exist while the
      //   stroke stays in proportion (measured on the three sizes this
      //   repo has recorded), and a threshold written down here would
      //   only go red when somebody remembered to come back and change
      //   it. Reading the product's own `<text>` at the smallest size
      //   it draws asks the question that has an answer.
      const fit = await page.evaluate(() => {
        const btn = document.querySelector("[data-machine-card] [data-metric]");
        const node = btn && [...btn.querySelectorAll("*")].filter((n) => n.children.length === 0 && /^\d+%$/.test((n.textContent || "").trim())).pop();
        const fill = btn && btn.querySelector("[data-ring-fill]");
        const m = fill && /A\s*([\d.]+)\s+([\d.]+)/.exec(fill.getAttribute("d") || "");
        if (!node || !m) return null;
        const stroke = parseFloat(getComputedStyle(fill).strokeWidth);
        const was = node.textContent;
        const before = +node.getBoundingClientRect().width.toFixed(2);
        node.textContent = "100%";
        const widest = +node.getBoundingClientRect().width.toFixed(2);
        node.textContent = was;
        return { inner: +(2 * Number(m[1]) - stroke).toFixed(2), widest, restored: +node.getBoundingClientRect().width.toFixed(2) === before };
      });
      if (!fit || !fit.restored) {
        fail(`probe dead: the dial's centre reading could not be measured at ${w} (${JSON.stringify(fit)})`);
      } else if (fit.widest > fit.inner) {
        fail(
          `at ${w} the dial draws ${box} across and 100%@${box} = ${fit.widest} against inner@${box} = ${fit.inner} — ` +
            `the widest reading a dial can say does not fit inside its own ring. **The stroke has to shrink with the size**: ` +
            `at a fixed stroke the inner diameter falls faster than the type does, which is how this overflowed at 88 (57.25 ` +
            `against 56) before 2026-09-08.`,
        );
      } else {
        seenFits.push(`${w}:100%@${box}=${fit.widest}<${fit.inner}`);
      }
      //   ⚠️ **Where this arm can be torn, and where it cannot.**
      //   Torn 2026-09-09, one thing at a time:
      //
      //     type 15% bigger than `19k`  → red at every posture
      //                                   (123.17 against 110.23)
      //     the shrink switched off     → red on the edges arm above,
      //                                   not on this one
      //     **the stroke held fixed while the size shrinks** → ⚠️ **no
      //     red at any width this walk visits.** With a stroke of 5 the
      //     inner falls to the type's width only below size ~109 —
      //     about a 280-wide window, narrower than any device — so the
      //     sentence in the failure above is *true* and this arm cannot
      //     prove it here. It is written as the reason rather than
      //     claimed as tested.
    }
    if (seenDials.length !== 7) {
      fail(`probe dead: ${seenDials.length} of 7 postures measured (${seenDials.join(" ")}) — 「at every width」 is a claim about all of them`);
    } else {
      //   **The widest posture is the full size, and nothing exceeds
      //   it.** 1400 is the first entry and has room to spare, so what
      //   it draws is the number; every other posture is that or less.
      const boxes = seenDials.map((r) => Number(r.split("×")[1]));
      const full = boxes[0];
      if (boxes.some((b) => b > full + 0.5)) {
        fail(`the dials come out ${boxes.join(" / ")} across — none may be bigger than the ${full} the widest posture draws`);
      }
      //   ⚠️ **And the shrink is not free-running**: only the narrow
      //   postures may be under it, and never by more than the room
      //   asks. A quarter under would mean the size had gone back to
      //   being a share of the row.
      if (boxes.some((b) => b < full * 0.7)) {
        fail(`the dials come out ${boxes.join(" / ")} across against a full ${full} — a dial that gives up a third of itself is a share again, not a number`);
      }
    }
    console.log(`dial row: ${seenDials.join(" · ")}`);
    console.log(`dial fit: ${seenFits.join(" · ")}`);
    await page.setViewportSize({ width: 1400, height: 900 });
    await wait(300);
  }

  if (thrown.length) fail(`page errors while measuring: ${thrown.join(" | ")}`);
  if (!process.exitCode) console.log(
      "face gate: three pressures, numbers in columns, every stop's targets a finger's size, a dense pane that leads, the machine surfaces bare of subscription arcs, the \u989d\u5ea6 rows in all three shapes with the subscription among them and its bar narrowing them, and narrow's session tabs switching for real",
    );
} finally {
  await browser?.close();
  vite?.kill();
}
