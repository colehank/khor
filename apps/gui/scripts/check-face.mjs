// The type gate, rendered half: what the letterforms actually weigh on
// the glass. The static half (`check-type.mjs`) proves nobody wrote a
// stray literal; this half proves the tokens land — a stylesheet layer,
// a purge, or a font stack missing a weight would leave every rule in
// place and every screen at 400, and no static check can see that.
//
// Three claims:
//   1. the token classes resolve on a live page: a probe element wearing
//      font-title computes 600, font-label 500, tabular-nums carries
//      font-variant-numeric — the CSS chain end to end;
//   2. a real face uses them: with one session row and one device routed
//      in, the list row's title weighs title, a section label weighs
//      label, and every element that opted into tabular-nums got it —
//      the weight distribution is {400,500,600}, not the flat {400}
//      this gate was born from;
//   3. zero page errors while measuring — a half-drawn page answers
//      probes too (the browser-check false-green family).
//
// The routed rows are shaped by hand from the bindings. If the shape
// drifts, the list draws nothing, the title probe finds no element, and
// this gate goes red saying so — loud drift, the survivable kind.
//
// Run: `npm run face` (boots its own vite, needs system Chrome).
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.FACE_PORT ?? 1451);
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
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

  // One session row and one device, so a title and a reading exist to
  // measure. Hand-shaped from the bindings (see the file head for why
  // drift here fails loud instead of quietly).
  const row = {
    id: "tui/face-gate",
    kind: "tui",
    title: "face-gate-title",
    word: "idle",
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
  };
  const dev = {
    id: "e".repeat(64),
    name: "gate",
    me: true,
    face: null,
    pinned: false,
    vitals: null,
    hop: null,
    seat: { x: 0, y: 0 },
  };
  await page.route("**/sessions", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify([row]) }),
  );
  await page.route("**/devices", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify([dev]) }),
  );
  // The shell polls sessions, devices and hooks in one Promise.all —
  // one rejection drops the whole batch, so the third leg answers too.
  await page.route("**/hooks_state", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify({ installed: false }) }),
  );
  // The mark's place asks for these two, and section 4 measures it. A
  // fortnight of spending, because the wall this gate was written about
  // only exists at that length.
  const days = [];
  for (let i = 13; i >= 0; i--) {
    const d = new Date(Date.now() - i * 86_400_000);
    days.push({
      day: `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`,
      category: "claude",
      tokens: { input: 1500, cached_input: 246_000_000, cache_write: 7_800_000, output: 280_000 - i * 12_000 },
    });
  }
  await page.route("**/usage", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify({ days, unreadable: 0 }) }),
  );
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

  // ── 3) touch targets: every rail item is pressable by a finger ────
  // The token is the criterion (`--touch`, tokens.css: HIG / WCAG floor)
  // and is read off the page rather than repeated here — a second copy
  // of the number is how a gate and a stylesheet drift apart. Measured
  // on the narrow face because that is the posture with fingers, and on
  // the wide face too: a desktop is where a trackpad's imprecision and
  // the web face on an iPad both live.
  for (const [name, vp] of [["narrow", { width: 390, height: 844 }], ["wide", { width: 1400, height: 900 }]]) {
    await page.setViewportSize(vp);
    await wait(400);
    const touch = await page.evaluate(() => {
      const floor = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--touch"));
      const items = [...document.querySelectorAll("[data-rail-item]")].map((el) => {
        const r = el.getBoundingClientRect();
        return { w: Math.round(r.width), h: Math.round(r.height) };
      });
      return { floor, items };
    });
    if (!(touch.floor > 0)) fail(`${name}: --touch does not resolve on the page`);
    if (touch.items.length === 0) fail(`${name}: probe dead — no rail items to measure`);
    const under = touch.items.filter((i) => i.h < touch.floor || i.w < touch.floor);
    if (under.length > 0) {
      fail(
        `${name}: ${under.length} rail item(s) under the ${touch.floor} touch floor — ` +
          under.map((i) => `${i.w}×${i.h}`).join(", "),
      );
    } else {
      console.log(`${name}: ${touch.items.length} rail items all ≥ ${touch.floor} touch floor`);
    }
  }

  // ── 4) a dense screen has a reading order ─────────────────────────
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
      console.log(
        `density ${r.name}: ${r.leaves} leaves, ${JSON.stringify(r.sizes)}, modal ${modal} at ${Math.round(share * 100)}%, largest ${largest}`,
      );
      if (r.leaves <= DENSE) continue;
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

  if (thrown.length) fail(`page errors while measuring: ${thrown.join(" | ")}`);
  if (!process.exitCode) console.log("face gate: three pressures, numbers in columns, and a dense pane that leads");
} finally {
  await browser?.close();
  vite?.kill();
}
