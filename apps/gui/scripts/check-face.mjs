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
//      least `--touch` on both sides, in both postures;
//   4. a dense pane has a reading order;
//   5. zero page errors while measuring — a half-drawn page answers
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
      seat: { x: 0, y: 0 },
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
          gpu: { util_pct: 67, cards: 1, mem: fill(2_000_000_000, 8_000_000_000) },
          version: "0.1.0",
        },
        age_ms: 42_000,
        mem_strain: null,
        disk_strain: null,
        vram_strain: null,
      },
      hop: "direct",
      sponsor: null,
      seat: { x: 1, y: 0 },
    },
  ];
  await page.route("**/sessions", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(rows) }),
  );
  await page.route("**/devices", (r) =>
    r.fulfill({ contentType: "application/json", body: JSON.stringify(devs) }),
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
  //     DOM says it is a target, so this cannot see it (neither can a
  //     keyboard; that is a11y's gate, not this one). The `cursor:
  //     pointer` survey printed beside each stop is the tell-tale for
  //     it, and it is **printed, never asserted** — a hover style on a
  //     row would make it a red that teaches people to widen the
  //     excuses;
  //   - a target that is big enough and sits under something else;
  //   - the settings dialog's body: it draws from `face_choices`, an
  //     answer this fixture does not fake, so only the dialog's frame is
  //     judged there. The stop says so on its own line rather than
  //     counting zero and moving on.
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
      return { floor, targets, excused, pointing: pointing.length };
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
  const STOPS = [
    { name: "sessions list", anchor: "[data-list]", go: () => press(rail("sessions")) },
    {
      name: "session detail",
      anchor: "[data-detail-header]",
      go: async () => (await press(rail("sessions"))) && press("[data-list] [data-row-open]"),
    },
    // The map rides along on the wide face, where it is what the pane
    // opens into; narrow has the list alone, so the anchor is the row —
    // an anchor that only exists in one posture would report the other
    // one as a dead probe forever.
    { name: "devices", anchor: "[data-device]", go: () => press(rail("devices")) },
    {
      // This machine's card: the only one carrying the hooks control,
      // and the only one with no readings (it never asks itself).
      name: "machine card · this machine",
      anchor: "[data-hooks]",
      go: async () => (await press(rail("devices"))) && press("[data-list] [data-row-open]"),
    },
    {
      // …and a machine that has answered, which is the only way the
      // card's rings are on screen to be measured at all. The anchor is
      // the readings themselves, so a drifted `VitalsReading` shape
      // says *probe dead* here rather than passing a card that drew a
      // 「never reached」line instead.
      name: "machine card · a machine that answered",
      anchor: "[data-vitals]",
      go: async () => (await press(rail("devices"))) && pressNth("[data-list] [data-row-open]", 1),
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
    { name: "browser", anchor: '[data-browse-landing="page"]', go: () => press(rail("browser")) },
    { name: "the mark's place", anchor: "[data-mark-place]", go: () => press("[data-rail-mark]") },
    {
      name: "settings (frame only — its body needs a face_choices answer)",
      anchor: "[data-face-settings]",
      // The one rail item that opens no landing and is not the mark, so
      // it is named by what it is not rather than by a key it does not
      // have. (The mark's own marker sits on the drawing inside its
      // button, which is why this excludes by `:has` rather than by
      // the attribute.)
      go: () => press("[data-rail-item]:not([data-landing]):not(:has([data-rail-mark]))"),
      after: async () => {
        await page.keyboard.press("Escape");
        await wait(300);
      },
    },
  ];

  // Same control, same posture, same size, on four surfaces is one
  // thing to fix — collapsed so the list reads as a work queue.
  const under = new Map();
  for (const [posture, vp] of [
    ["narrow", { width: 390, height: 844 }],
    ["wide", { width: 1400, height: 900 }],
  ]) {
    await page.setViewportSize(vp);
    await page.reload({ waitUntil: "load" });
    await wait(900);
    for (const stop of STOPS) {
      await stop.go();
      const here = `${posture} · ${stop.name}`;
      if ((await page.locator(stop.anchor).count()) === 0) {
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
      for (const t of seen.targets) {
        if (t.w >= seen.floor && t.h >= seen.floor) continue;
        const key = `${posture}|${t.own.join(" ") || t.cls}|${t.w}×${t.h}`;
        const was = under.get(key);
        if (was) {
          was.n += 1;
          continue;
        }
        // Three ways to name where it is written, in falling order of
        // certainty, and **the line says which one it used** — an
        // approximate location presented as an exact one is the kind of
        // fact that sends somebody to edit a file that is fine.
        const own = t.own.map((a) => [a, whereIs(a)]).find(([, at]) => at);
        const byClass = t.cls && whereIs(t.cls.split(" ").slice(0, 3).join(" "));
        const via = t.chain.map((a) => [a, whereIs(a)]).find(([, at]) => at);
        under.set(key, {
          n: 1,
          where: here,
          what:
            `${t.own.join(" ") || `<${t.tag}> .${t.cls}`}` + (t.said ? ` "${t.said}"` : ""),
          size: `${t.w}×${t.h}`,
          floor: seen.floor,
          at: own
            ? own[1]
            : byClass
              ? `${byClass} (by class)`
              : via
                ? `${via[1]} (the surface it is on, not where it is written)`
                : "no marker and no class — grep for the words on it",
        });
      }
      await stop.after?.();
    }
  }
  for (const u of [...under.values()]) {
    fail(
      `under the ${u.floor} touch floor — ${u.where}: ${u.what} ${u.size}` +
        (u.n > 1 ? ` (×${u.n})` : "") +
        ` — ${u.at}`,
    );
  }
  if (under.size === 0) console.log("touch: every pressable thing on every stop clears the floor");

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
  if (!process.exitCode) console.log(
      "face gate: three pressures, numbers in columns, every stop's targets a finger's size, and a dense pane that leads",
    );
} finally {
  await browser?.close();
  vite?.kill();
}
