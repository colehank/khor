// The day strip, measured on the glass — the one thing about it that
// nothing else in this tree can see.
//
// # Why this gate exists, and it is not a hypothetical
//
// The strip shipped once without ever appearing on screen. `cargo test`
// green (259 assertions across three crates), `tsc --noEmit` clean,
// `check-px` clean, `check-icons` clean, `pageerror` empty. Six gates,
// none of them red, and **the feature was not on the page at all**.
//
// Nothing had regressed. Every one of those checks was doing exactly
// its job — none of them can see whether a band made it onto a row.
// The cause was a React effect keyed on a ref: the list renders its
// empty state until the first poll answers, so the observed node did
// not exist on the first render, and a ref object never changes, so
// nothing ever re-ran the effect when the node arrived. From inside
// that effect **"no node yet" and "no node ever" are the same
// observation** — it cannot throw, cannot warn, and cannot be red. It
// can only be absent.
//
// So: adding a thing is three steps too, and the third is *who proves
// it reached the screen*.
//
// # The claims
//
//   1. every word cell is painted in its own word's colour — asserted
//      as a **derivation**, this row's cell against this row's word,
//      never against a colour written in this file. A palette pinned
//      here would keep passing after the six moved;
//   2. the two nothings (`khor_core::DayCell`) are told apart from the
//      words and from each other **by substance, not by tint**: a word
//      is a full-height block, 不知道 is a hairline, 那时没有这一行 is
//      no element at all. Tint alone is how they would quietly fold
//      back into one on screen after the node worked to keep them apart;
//   3. the strips share one axis — each spans its whole row, and starts
//      left of where the row's content begins. A strip is a timeline
//      and timelines only compare if they line up; nested into the
//      row's padded button it would still look fine alone and be wrong
//      the moment two rows are read against each other.
//
// # The anti-vacuum probes are the load-bearing part
//
// Claims 1 and 2 are mostly negative — *no cell wears the wrong
// colour*, *no nothing wears a state colour* — and **every negative
// assertion passes against a blank screen**. That is not a worry, it is
// what happened: on the run that caught the bug above, claims 1 and 2
// had nothing to disagree with and would have reported green.
//
// So before any of them: at least four distinct words must be on
// screen (one word cannot tell "coloured by its word" from "all one
// colour"), and both nothings must appear at least once. A fixture that
// stops producing one of them fails here as a dead probe rather than
// passing as a clean sheet.
//
// # It builds nothing
//
// It needs a bridge and a fixture and it makes neither: cargo's
// artifact lock is shared with whoever else is on this machine, and a
// check that queues behind a build is a check nobody runs
// (`check-rail`'s head). Missing artifacts are a refusal that names the
// command, the way `smoke.mjs` refuses.
//
// Run: `npm run day` (needs system Chrome, and:
// `cargo build -p khor-gui-core --bin bridge -p khor-node --example day_fixture`)
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "..", "..");

// Its own pair, not the dev server's and not another gate's. Borrowing
// a running vite would measure whatever source *that* one serves, and a
// measurement of somebody else's tree reported as a measurement of this
// one is worse than no measurement (`check-rail`). `--strictPort` makes
// a collision an error instead of a silent hop to the next port.
const PORT = Number(process.env.DAY_PORT ?? 1453);
const BRIDGE_PORT = Number(process.env.DAY_BRIDGE_PORT ?? 1454);

const BRIDGE = join(repo, "target/debug/bridge");
const FIXTURE = join(repo, "target/debug/examples/day_fixture");

const problems = [];
const fail = (m) => problems.push(m);

for (const [what, path] of [["the bridge", BRIDGE], ["the fixture", FIXTURE]]) {
  if (!existsSync(path)) {
    console.error(
      `${what} is not built: ${path}\n` +
        "  cargo build -p khor-gui-core --bin bridge -p khor-node --example day_fixture\n" +
        "This gate does not build it for you — cargo's lock is shared, and a check " +
        "that queues behind a build is a check nobody runs.",
    );
    process.exit(1);
  }
}

// A home of its own, always: the fixture writes a whole day of record
// and a handful of registry entries, and doing that to somebody's real
// `~/.khor` would be this gate damaging the thing it measures.
const home = mkdtempSync(join(tmpdir(), "khor-day-gate-"));
execFileSync(FIXTURE, [home], { stdio: "inherit" });

let bridge, vite, browser;
const listening = async (port, what, ms) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/`, { method: "POST", body: "{}" });
      if (r) return true;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`${what} never came up on ${port}`);
};

try {
  bridge = spawn(BRIDGE, [], {
    env: { ...process.env, KHOR_HOME: home, BRIDGE_PORT: String(BRIDGE_PORT) },
    stdio: ["ignore", "pipe", "pipe"],
  });
  bridge.on("exit", (code) => {
    if (code !== null && code !== 0) fail(`the bridge exited with ${code} — probably a taken port`);
  });
  await listening(BRIDGE_PORT, "the bridge", 60_000);

  vite = spawn("node_modules/.bin/vite", ["--port", String(PORT), "--strictPort"], {
    cwd: gui,
    stdio: ["ignore", "pipe", "pipe"],
  });
  vite.stderr.on("data", (b) => process.stderr.write(`[vite] ${b}`));
  await new Promise((ok, no) => {
    const t = setTimeout(() => no(new Error("vite never said it was ready")), 60_000);
    vite.stdout.on("data", (b) => {
      if (String(b).includes("ready in")) {
        clearTimeout(t);
        ok();
      }
    });
  });

  browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    // This machine has an http_proxy set, and a proxy in front of
    // localhost turns every call into a timeout that reads as a broken
    // backend.
    args: ["--no-proxy-server"],
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  // A page that throws paints half a screen, and every negative claim
  // below would pass against the wreckage (docs: 页面在抛异常,脚本照样
  // 报绿).
  const thrown = [];
  page.on("pageerror", (e) => thrown.push(String(e)));
  await page.goto(`http://localhost:${PORT}/?bridge=${BRIDGE_PORT}`, { waitUntil: "load" });

  const until = async (what, ms, f) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      if (await f()) return true;
      await new Promise((r) => setTimeout(r, 200));
    }
    fail(`probe void: ${what}`);
    return false;
  };
  await until("rows on the session list", 20_000, async () => (await page.locator("[data-row]").count()) > 0);
  await until("a day strip painted", 25_000, async () => (await page.locator("[data-day-strip]").count()) > 0);

  const seen = await page.evaluate(() => {
    const WORDS = ["busy", "blocked", "done", "errored", "failed", "idle"];
    // What each word's token *resolves to*, asked of the browser rather
    // than read as text: `getComputedStyle` substitutes `var()`, so a
    // literal comparison would be comparing "var(--state-idle)" against
    // a colour and never match.
    const swatch = document.createElement("span");
    document.body.appendChild(swatch);
    const resolve = (v) => {
      swatch.style.backgroundColor = v;
      return getComputedStyle(swatch).backgroundColor;
    };
    const wants = Object.fromEntries(WORDS.map((w) => [w, resolve(`var(--state-${w})`)]));
    swatch.remove();

    const marks = [...document.querySelectorAll("[data-day-mark]")].map((el) => {
      const kid = el.firstElementChild;
      return {
        mark: el.dataset.dayMark,
        kidBg: kid ? getComputedStyle(kid).backgroundColor : null,
        kidH: kid ? Math.round(kid.getBoundingClientRect().height * 100) / 100 : null,
        kids: el.childElementCount,
      };
    });

    const px = (n) => Math.round(n * 100) / 100;
    const boxes = [...document.querySelectorAll("[data-row]")].map((row) => {
      const strip = row.querySelector("[data-day-strip]");
      const button = row.querySelector("[data-row-open]");
      const r = row.getBoundingClientRect();
      const s = strip && strip.getBoundingClientRect();
      const b = button && button.getBoundingClientRect();
      // Where the row's **content** begins. Not the button's box: its
      // inset is padding, so `button.left === row.left` by construction
      // and comparing those two would be a comparison that cannot fail.
      const inset = b ? parseFloat(getComputedStyle(button).paddingLeft) : 0;
      return {
        id: row.dataset.row,
        rowL: px(r.left),
        rowR: px(r.right),
        stripL: s ? px(s.left) : null,
        stripR: s ? px(s.right) : null,
        stripH: s ? px(s.height) : null,
        contentL: b ? px(b.left + inset) : null,
        runs: strip ? strip.childElementCount : 0,
      };
    });
    return { wants, marks, boxes };
  });

  if (thrown.length) fail(`the page threw: ${thrown.join(" | ")}`);

  const words = seen.marks.filter((m) => !["unwatched", "absent"].includes(m.mark));
  const unwatched = seen.marks.filter((m) => m.mark === "unwatched");
  const absent = seen.marks.filter((m) => m.mark === "absent");
  const kinds = new Set(words.map((m) => m.mark));

  // ── the probes, before the claims that need them ──────────────────
  if (words.length === 0) fail("probe dead: no word cells on screen — claims 1 and 2 would pass on a blank strip");
  if (kinds.size < 4) {
    fail(
      `probe dead: only ${[...kinds].join("+") || "nothing"} on screen — with fewer than four ` +
        "words this cannot tell 「coloured by its word」 from 「all one colour」 (deepen day_fixture)",
    );
  }
  if (unwatched.length === 0) fail("probe dead: no 不知道 cells — claim 2's first half would pass by vacuum");
  if (absent.length === 0) fail("probe dead: no 那时没有这一行 cells — claim 2's second half would pass by vacuum");

  // ── 1) each word is its own word's colour ─────────────────────────
  for (const m of words) {
    if (m.kidBg !== seen.wants[m.mark]) {
      fail(`① a ${m.mark} cell painted ${m.kidBg}, while --state-${m.mark} resolves to ${seen.wants[m.mark]}`);
    }
  }

  // ── 2) the two nothings, told apart by substance ──────────────────
  const sixColours = new Set(Object.values(seen.wants));
  for (const m of unwatched) {
    if (sixColours.has(m.kidBg)) fail(`② 不知道 wears a state colour: ${m.kidBg} — it is not one of the six words`);
    if (m.kids !== 1) fail(`② 不知道 should be one hairline, has ${m.kids} children`);
  }
  for (const m of absent) {
    if (m.kids !== 0) fail(`② 那时没有这一行 should paint nothing at all, has ${m.kids} children`);
  }
  const hair = unwatched[0]?.kidH ?? null;
  const block = words[0]?.kidH ?? null;
  if (hair !== null && block !== null && !(hair < block)) {
    fail(`② the two must differ by substance: hairline is ${hair} and a word block is ${block}`);
  }

  // ── 3) one axis ───────────────────────────────────────────────────
  for (const b of seen.boxes) {
    if (b.stripL === null) {
      fail(`③ ${b.id} has no strip`);
      continue;
    }
    if (b.stripL !== b.rowL || b.stripR !== b.rowR) {
      fail(`③ ${b.id} strip spans ${b.stripL}..${b.stripR}, its row spans ${b.rowL}..${b.rowR}`);
    }
    if (!(b.stripL < b.contentL)) {
      fail(
        `③ ${b.id} strip starts at ${b.stripL}, at or past where content begins (${b.contentL}) — ` +
          "it is inside the padded flow rather than full-bleed",
      );
    }
  }
  const lefts = new Set(seen.boxes.map((b) => b.stripL));
  const rights = new Set(seen.boxes.map((b) => b.stripR));
  if (lefts.size > 1 || rights.size > 1) {
    fail(`③ the strips do not share an axis: lefts ${[...lefts]}, rights ${[...rights]}`);
  }

  console.log(`palette: ${JSON.stringify(seen.wants)}`);
  console.log(
    `cells: ${words.length} word (${[...kinds].sort().join("+")}), ` +
      `${unwatched.length} 不知道, ${absent.length} 那时没有这一行`,
  );
  console.log(`substance: hairline ${hair}, word block ${block}`);
  console.log(
    `axis: ${seen.boxes.map((b) => `${b.id} ${b.stripL}..${b.stripR} h${b.stripH} ⊢${b.contentL} ${b.runs} runs`).join(", ")}`,
  );

  // ── 4) a browser that has been here before ────────────────────────
  //
  // **Everything above ran in a blank profile, and that is the one
  // state a returning user is never in.** Their browser carries a
  // dragged pane width, a collapsed pane, an arrangement, filter words
  // — and the strip's width comes from measuring that pane. Measured
  // 2026-08-30: with the pane stored collapsed the box is zero wide,
  // and the strip asked for nothing and painted nothing while fifteen
  // rows sat in the DOM. Every claim above passed that page, because
  // every one of them ran against a profile that had never been used.
  //
  // So: the same page, with the state a returning browser can hold.
  // The claim is deliberately weaker than the three above — a
  // zero-width pane has no geometry and no colours worth asserting —
  // and it is the one that matters: **stored state must not be able to
  // switch the feature off.**
  const returning = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  await returning.addInitScript(() => {
    try {
      localStorage.setItem("khor.list-width", '{"w":462,"collapsed":true}');
    } catch {
      /* a profile that refuses storage is simply the blank case again */
    }
  });
  const back = await returning.newPage();
  const backThrew = [];
  back.on("pageerror", (e) => backThrew.push(String(e)));
  await back.goto(`http://localhost:${PORT}/?bridge=${BRIDGE_PORT}`, { waitUntil: "load" });
  const backSeen = await (async () => {
    const end = Date.now() + 25_000;
    let last = { rows: 0, strips: 0, boxW: null };
    while (Date.now() < end) {
      last = await back.evaluate(() => {
        const box = document.querySelector("[data-row]")?.parentElement;
        return {
          rows: document.querySelectorAll("[data-row]").length,
          strips: document.querySelectorAll("[data-day-strip]").length,
          boxW: box ? Math.round(box.getBoundingClientRect().width) : null,
        };
      });
      if (last.rows > 0 && last.strips > 0) return last;
      await new Promise((r) => setTimeout(r, 300));
    }
    return last;
  })();
  if (backThrew.length) fail(`④ the returning page threw: ${backThrew.join(" | ")}`);
  if (backSeen.rows === 0) {
    fail("probe dead: the returning page listed no rows — ④ would pass with nothing to switch off");
  } else if (backSeen.strips !== backSeen.rows) {
    fail(
      `④ a returning browser lost the strip: ${backSeen.rows} rows, ${backSeen.strips} strips, ` +
        `pane ${backSeen.boxW}px. Stored state must not be able to switch the feature off.`,
    );
  }
  console.log(`returning: rows=${backSeen.rows} strips=${backSeen.strips} pane=${backSeen.boxW}px (collapsed)`);
  await returning.close();
} finally {
  if (browser) await browser.close();
  for (const p of [vite, bridge]) if (p && !p.killed) p.kill("SIGTERM");
  await new Promise((r) => setTimeout(r, 800));
  for (const p of [vite, bridge]) if (p && p.exitCode === null) p.kill("SIGKILL");
  rmSync(home, { recursive: true, force: true });
  // Leaving a listener behind is its own bug, and one this tree has paid
  // for twice: an orphaned dev server holds a port and memory while
  // nobody knows it is there. Said out loud rather than assumed.
  for (const port of [PORT, BRIDGE_PORT]) {
    try {
      execFileSync("lsof", ["-i", `:${port}`, "-sTCP:LISTEN"], { stdio: "pipe" });
      console.error(`left behind: something is still listening on ${port}`);
      problems.push(`a listener survived teardown on ${port}`);
    } catch {
      /* lsof exits non-zero when nothing listens, which is the good case */
    }
  }
}

if (problems.length) {
  console.error("\nRED:\n" + problems.join("\n"));
  process.exit(1);
}
console.log("\nday strip: painted, coloured by its own words, and on one axis");
