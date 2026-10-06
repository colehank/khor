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
//   0. (2026-09-29) the day is **hidden until the front tab is pressed**:
//      no row and no room kept for one, a hover that says what a press
//      does, a press (and Enter / Space) that does it, a double-click on
//      the name that renames without the band so much as blinking, a
//      choice that outlives a reload, no row at all for a day with no
//      record, a skeleton rather than a blank while the day is on its
//      way, and on the narrow face the front tab in the session bar.
//      Shown, it **floats over the session and takes none of its room**
//      (「悬浮内容悬浮在session内容上即可」): the session does not move or
//      shrink by a pixel, and a press on the row off a stretch is the
//      session's. Everything from 1 on is read with the band opened this
//      way;
//   1. every word cell is painted in its own word's colour — asserted
//      as a **derivation**, this row's cell against this row's word,
//      never against a colour written in this file. A palette pinned
//      here would keep passing after the six moved;
//   2. one axis, three tiers (`khor_core::DayCell`): every tier that has
//      ink fills the same height, so the band reads as one line rather
//      than a line beside a bar — 不知道 was a hairline until a user
//      called it 「一条细线」 and the coloured part 「bar」, two nouns for
//      one axis. What separates the tiers instead is **ink**: a word has
//      a hue, 不知道 has a quiet tint of the page's own ink, and
//      那时没有这一行 has no element at all. Measured as a floor *and* a
//      factor below the faintest word, because the pair that can now
//      collapse is 不知道 against 空闲 — the one word left without a hue;
//   3. the strips share one axis — each spans its whole row, and starts
//      left of where the row's content begins. A strip is a timeline
//      and timelines only compare if they line up; nested into the
//      row's padded button it would still look fine alone and be wrong
//      the moment two rows are read against each other;
//   6. (2026-09-29) the band is a control: pills 2 apart and never
//      thinner than a dot, a readout and a popover that say the same
//      stretch, a finger that gets the popover, one Tab stop, and a hit
//      area that covers nothing but its own row. Its own section says
//      what each half caught on the code before it.
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
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

import { fakeHome, fakeUsage } from "./fake-home.mjs";

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
/**
 * Flips the desktop's scheme **and waits for the face to wear it**.
 *
 * ⚠️ 批 D2 put the dark palette under `:root[data-theme="dark"]` rather
 * than a media query (`tokens.css` says why: the alternative was a
 * second copy of the whole palette). So the stamp is one JS hop behind
 * the emulation — `src/theme.ts`'s matchMedia listener re-stamping the
 * root — and a colour read straight after `emulateMedia` is the theme
 * being *left*.
 *
 * **This waits for the stamp and nothing else.** The settle each caller
 * keeps afterwards is a different wait for a different thing: these
 * glyphs have colour transitions, and a reading taken the instant the
 * palette changes is bit-for-bit the colour it is leaving (the same
 * lesson `smoke` §26 wrote down about an animating menu item). Folding
 * the two into one wait is how this gate went red measuring ink at 1.08
 * against a ground that had already turned.
 */
async function flipTheme(page, scheme) {
  await page.emulateMedia({ colorScheme: scheme });
  const deadline = Date.now() + 5_000;
  for (;;) {
    const worn = await page.evaluate(() =>
      document.documentElement.getAttribute("data-theme"),
    );
    if (worn === scheme) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `the face never wore the ${scheme} scheme (data-theme=${JSON.stringify(worn)}) — ` +
          "批 D2's stamp is what repaints, and without it every colour below is the other theme's",
      );
    }
    await new Promise((r) => setTimeout(r, 50));
  }
}

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
// Not the operator's `HOME` either (`fake-home.mjs`).
const usage = await fakeUsage();
const fakeHomeDir = join(home, "home");
mkdirSync(fakeHomeDir, { recursive: true });
const faked = fakeHome(fakeHomeDir, usage.url);
// The fixture names the rows it put on the list (`row <id>`), so a row
// that never reaches the screen is reported as that, by name. Without
// it the only symptom was a strip with one word, and the probes below
// blamed the fixture's window: from SHELL.md 第八轮 until 2026-09-28 the
// fixture's three rows were never imported, stayed off the list, and
// this gate said 「day_fixture's window moved past it」.
const fixtureOut = String(execFileSync(FIXTURE, [home], { stdio: ["ignore", "pipe", "inherit"] }));
process.stdout.write(fixtureOut);
const fixtureRows = fixtureOut
  .split("\n")
  .filter((l) => l.startsWith("row "))
  .map((l) => l.slice(4).trim());
// …and the one it made **with no record** (`quiet <id>`): on the list,
// in none of the samples, so its day has not one stretch with a word.
// ⓪ e asks that it draw no row at all.
const fixtureQuiet = fixtureOut
  .split("\n")
  .filter((l) => l.startsWith("quiet "))
  .map((l) => l.slice(6).trim());

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
    env: { ...process.env, ...faked, KHOR_HOME: home, BRIDGE_PORT: String(BRIDGE_PORT) },
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
  // ⓪ e's rows, emptied on the wire (`day_fixture`'s quiet row says why
  // the recorder cannot leave one empty): 未查看 end to end, the shape a
  // peer that never answered arrives in (`day::unwatched`), or no entry
  // at all, a row a poll behind the list. Only those rows; every other
  // row's day is the recorder's own.
  let emptyAs = "unwatched";
  await page.route("**/day_strips", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    const res = await route.fetch();
    const body = await res.json();
    for (const id of fixtureQuiet) {
      if (!(id in body)) continue;
      const cells = body[id].reduce((n, r) => n + r.cells, 0);
      if (emptyAs === "missing") delete body[id];
      else body[id] = [{ mark: "unwatched", cells }];
    }
    await route.fulfill({ response: res, json: body });
  });
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
  // **The band lives under the open session's header now, one at a
  // time** (2026-09-02:「状态bar…在session打开后的上方显示」), so the
  // rows are opened in turn and each one's band read while it is up.
  // Nothing is asserted on the list: a strip found on a row would be
  // the old arrangement coming back.
  if ((await page.locator("[data-row] [data-day-strip]").count()) !== 0) {
    fail("a day strip is back on a list row — the band belongs under the open session's header");
  }
  const rowIds = await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  // **Every row the fixture made is on the list**, before any band is
  // read: the day under test lives on those rows, and a band is read by
  // opening its row. A tui row is on the list only once khor started it
  // or somebody imported it (docs/SHELL.md 第八轮).
  if (fixtureRows.length === 0) fail("probe dead: day_fixture named no rows — nothing says which days to look for");
  const offList = fixtureRows.filter((id) => !rowIds.includes(id));
  if (offList.length) {
    fail(
      `probe dead: day_fixture's ${offList.join(", ")} never reached the session list (saw ${rowIds.join(", ") || "nothing"}) ` +
        "— their days are in the record and nobody can open them",
    );
  }
  if (fixtureQuiet.length === 0) fail("probe dead: day_fixture named no row without a record — ⓪ e would pass on nothing");
  const quietOff = fixtureQuiet.filter((id) => !rowIds.includes(id));
  if (quietOff.length) fail(`probe dead: day_fixture's ${quietOff.join(", ")} never reached the session list — ⓪ e has no row to open`);

  // The words the strip and its switch say, read out of the generated
  // table rather than written here: a copy in this file would keep
  // passing after the catalog moved. Either language is accepted — the
  // face picks one.
  const catalogWords = (() => {
    const src = readFileSync(join(gui, "src/gen/catalog.ts"), "utf8");
    const table = (name) => {
      const m = src.match(new RegExp(`const ${name}(?:: [^=]+)? = \\{([\\s\\S]*?)\\n\\};`));
      if (!m) return {};
      return Object.fromEntries([...m[1].matchAll(/^\s*(\w+): "([^"]*)",?$/gm)].map((x) => [x[1], x[2]]));
    };
    const zh = table("zh_state");
    const en = table("en_state");
    const zhGui = table("zh_gui");
    const enGui = table("en_gui");
    zh.unwatched = zhGui.day_unwatched;
    en.unwatched = enGui.day_unwatched;
    // The front tab's hover is the column's (UX-30: 打开附栏 / 收起附栏);
    // `none` is what the expanded ruler says of a day with no record.
    const hint = {
      show: [zhGui.side_show, enGui.side_show].filter(Boolean),
      hide: [zhGui.side_hide, enGui.side_hide].filter(Boolean),
      none: [zhGui.day_none, enGui.day_none].filter(Boolean),
    };
    return { zh, en, hint };
  })();
  const HINT = catalogWords.hint;
  if (Object.values(HINT).some((w) => w.length !== 2)) {
    fail(`probe dead: ⓪ could not read side_show / side_hide / day_none out of src/gen/catalog.ts (${JSON.stringify(HINT)})`);
  }
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  // ── 0) the day is 最近's first row (UX-39 §4, 2026-10-06) ─────────
  //
  // ⚠️ 墓碑 (UX-36 → UX-39 §4): the day was a two-point line in the
  // session pill's slot and the pill's menu 「今天」 hung it in full from
  // the pill. ⚠️ 墓碑 (UX-30 §5 → UX-36): it was the 附栏's ruler.
  // ⚠️ 墓碑 (2026-09-29 → 10-06): the front tab's press brought the band
  // down under the tabs. The front tab's press opens the session's 附栏
  // (`views/SessionSide`), which lands on 最近, whose first row is
  // 过去24小时: the day's two totals over its band (`views/DayToday`,
  // `DayStrip`); the row does not expand. No pill draws a day. Open,
  // width and seat are one memory per session (`use-session-side`).
  // Read on the glass:
  //
  //   a. a fresh profile draws no column and no day, and keeps no room
  //      for either — the session starts where the pane does;
  //   b. the front tab's hover says what a press will do (打开附栏 hidden,
  //      收起附栏 shown), and a press does it — on the face at once, on
  //      the name once the double-click window has passed, and by Enter /
  //      Space; the tab says `aria-expanded` and names the column
  //      (`aria-controls`). The column's 最近 starts with the day's band;
  //      no pill anywhere draws a day line and the pill's menu has no
  //      「今天」; the session gives up exactly the column's width;
  //   c. a double-click on the name renames and never toggles the column,
  //      not for one frame — sampled on every animation frame across the
  //      gesture, closed and open;
  //   d. the column and its day outlive a reload;
  //   e. a day with no record (`day_fixture`'s quiet row): the row stands
  //      and holds no band and no word; another session's column is
  //      untouched;
  //   f. while the day is on its way, the row holds the band's skeleton
  //      in the band's own geometry, never a blank (a context whose day
  //      route is held);
  //   g. narrow — the phone's pill holds no line; its tap opens the sheet
  //      on 最近, the day its first row, and the row does not expand (in
  //      ⑥e, where the phone's page is).
  const dayRows = (p) => p.locator("[data-detail-day]").count();
  const waitRows = async (p, want, ms = 1_500) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      if (((await dayRows(p)) > 0) === want) return true;
      await sleep(40);
    }
    return false;
  };
  const columns = (p) => p.locator("[data-session-side]").count();
  const waitColumn = async (p, want, ms = 1_500) => {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      if (((await columns(p)) > 0) === want) return true;
      await sleep(40);
    }
    return false;
  };
  const frontOpen = (p, id) => p.locator(`[data-session-tabs] [data-tab="${id}"][data-on="true"] [data-tab-open]`);
  /** The front tab's hover, read fresh: off the tab and back, because a
      pointer that is already on it since the press that closed the hint
      asks for nothing new. */
  const hintOf = async (p, id) => {
    await p.mouse.move(700, 600);
    await sleep(150);
    const b = await frontOpen(p, id).boundingBox();
    if (!b) return null;
    await p.mouse.move(b.x + 6, b.y + b.height / 2);
    const end = Date.now() + 1_500;
    while (Date.now() < end) {
      const t = await p.evaluate(() => {
        const el = [...document.querySelectorAll("[data-tab-hint]")].find((e) => {
          const r = e.getBoundingClientRect();
          return r.width > 0 && r.height > 0;
        });
        return el ? (el.innerText ?? "").trim() : null;
      });
      if (t) return t;
      await sleep(60);
    }
    return null;
  };
  /** Presses the front tab on its face — the half of the tab that is not
      the name, so nothing waits on a double-click. */
  const pressFace = async (p, id) => {
    const b = await frontOpen(p, id).boundingBox();
    if (!b) return false;
    await p.mouse.click(b.x + 6, b.y + b.height / 2);
    return true;
  };
  /** Shows the day: the front tab's 附栏 open on 最近, whose first row it
      is (UX-39 §4). */
  const openToday = async (p) => {
    const t = p.locator('[data-session-tabs] [data-tab][data-on="true"] [data-tab-open]');
    if ((await t.count()) !== 1) return false;
    if ((await columns(p)) === 0) {
      const b = await t.boundingBox();
      if (!b) return false;
      await p.mouse.click(b.x + 6, b.y + b.height / 2);
      if (!(await waitColumn(p, true, 1_500))) return false;
    }
    const pick = p.locator("[data-session-side] [data-side-head] [data-seat-pick] select");
    if ((await pick.inputValue().catch(() => null)) !== "recent") await pick.selectOption("recent").catch(() => {});
    return true;
  };
  /** Whether the column stood in the pane, on every animation frame for
      `ms` — the only way to see a column that opened and shut inside one
      gesture. */
  const watchColumn = (p, ms) =>
    p.evaluate(
      (ms) =>
        new Promise((done) => {
          const seen = [];
          const end = performance.now() + ms;
          const tick = () => {
            seen.push(!!document.querySelector("[data-session-side]"));
            if (performance.now() < end) requestAnimationFrame(tick);
            else done(seen);
          };
          tick();
        }),
      ms,
    );
  const ariaOf = (p, id) =>
    frontOpen(p, id).evaluate((b) => ({
      expanded: b.getAttribute("aria-expanded"),
      controls: b.getAttribute("aria-controls"),
      column: document.querySelector("[data-session-side]")?.id ?? null,
    }));
  const openTab = async (id) => {
    await page.locator(`[data-row="${id}"] [data-row-open]`).click();
    return until(`${id} in front`, 10_000, async () => (await frontOpen(page, id).count()) === 1);
  };
  /** The column, its 最近's first row (the day), and the session
      beside it; and every day line a pill might still draw. */
  const columnGeo = (p) =>
    p.evaluate(() => {
      const col = document.querySelector("[data-session-side]");
      const session = document.querySelector("[data-detail-row] > :first-child");
      if (!col || !session) return null;
      const scroller = col.querySelector("[data-side-recent] [data-nav-clear]");
      const row = col.querySelector("[data-detail-day]");
      const r = (el) => el.getBoundingClientRect();
      const c = r(col), t = r(session);
      return {
        column: { left: c.left, right: c.right, w: c.width },
        session: { left: t.left, right: t.right, w: t.width },
        view: window.innerWidth,
        first: !!row && scroller?.firstElementChild === row,
        expands: !!row && (row.hasAttribute("aria-expanded") || row.querySelector(":scope > button[aria-expanded]") !== null),
        marks: row ? row.querySelectorAll("[data-day-strip] [data-day-mark]").length : 0,
        lines: document.querySelectorAll("[data-tab-log], [data-pill-log], [data-day-ruler-line], [data-day-pop]").length,
      };
    });

  const first = fixtureRows[0];
  if (first && (await openTab(first))) {
    // Long enough for the first answer to have landed: what is asserted
    // next is the resting state, not the loading one (f has that).
    await sleep(1_500);
    // a. nothing under the tabs, no column, and no room kept
    const rest = await page.evaluate(() => {
      const pane = document.querySelector("[data-detail-pane]");
      const kid = pane?.firstElementChild ?? null;
      const r = (el) => el.getBoundingClientRect();
      return {
        rows: document.querySelectorAll("[data-detail-day]").length,
        bands: document.querySelectorAll("[data-day-strip], [data-day-skeleton]").length,
        columns: document.querySelectorAll("[data-session-side]").length,
        first: kid ? `${kid.tagName.toLowerCase()}${[...kid.attributes].map((a) => `[${a.name}]`).join("")}` : null,
        kept: pane && kid ? +(r(kid).top - r(pane).top).toFixed(2) : null,
        sessionRight: pane ? +(r(pane).right - r(document.querySelector("[data-detail-row] > :first-child") ?? pane).right).toFixed(2) : null,
      };
    });
    console.log(`⓪a fresh profile: ${JSON.stringify(rest)}`);
    if (rest.rows !== 0 || rest.bands !== 0 || rest.columns !== 0) {
      fail(`⓪a a fresh profile draws ${rest.rows} day row(s), ${rest.bands} band(s) and ${rest.columns} column(s) — the column opens on the front tab's press, and the day lives in it`);
    }
    if (rest.first === null) fail("probe dead: ⓪a the open session's pane has nothing in it to measure");
    else if (rest.kept !== 0) fail(`⓪a the session starts ${rest.kept}px below the pane's top (first child ${rest.first}) — no room is kept for a hidden column`);
    if (rest.sessionRight !== 0) fail(`⓪a the session ends ${rest.sessionRight}px short of the pane's right edge with no column open — no room is kept for one`);

    // b. the hover says it, a press does it
    const before = await hintOf(page, first);
    if (!HINT.show.includes(before)) fail(`⓪b the front tab's hover reads ${JSON.stringify(before)} while the column is closed — want ${HINT.show.join(" / ")}`);
    const shut = await ariaOf(page, first);
    if (shut.expanded !== "false") fail(`⓪b the front tab says aria-expanded=${shut.expanded} while the column is closed`);
    const paneRight = await page.evaluate(() => document.querySelector("[data-detail-pane]")?.getBoundingClientRect().right ?? null);
    await pressFace(page, first);
    if (!(await waitColumn(page, true, 600))) fail("⓪b pressing the front tab's face opened no column within 600ms");
    else {
      let geo = null;
      await until("the day in the column's 最近", 10_000, async () => {
        const g = await columnGeo(page);
        if (g && g.marks > 0) geo = g;
        return geo !== null;
      });
      if (!geo) fail("⓪b the column opened and its 最近 never held the day's band (UX-39 §4)");
      else {
        if (!geo.first) fail("⓪b the day is not 最近's first row in the column (UX-39 §4)");
        if (geo.expands) fail("⓪b the day's row is a disclosure — it does not expand (UX-39 §4)");
        if (geo.lines !== 0) fail(`⓪b ${geo.lines} day line(s) or hanging day(s) are drawn — no pill draws a day (UX-39 §4)`);
        // The session gives up exactly the column's width, the column
        // running to the window's edge (UX-39 §6: beside the strip).
        if (Math.abs(geo.session.right - geo.column.left) > 0.5 || Math.abs(geo.column.right - geo.view) > 0.5) {
          fail(`⓪b the session ends at ${geo.session.right} and the column starts at ${geo.column.left} (ends ${geo.column.right}, window ${geo.view}) — the column takes its width from the session`);
        }
        // The pill's menu offers no 「今天」.
        const t = page.locator('[data-session-tabs] [data-tab][data-on="true"] [data-tab-open]');
        await t.click({ button: "right" });
        await sleep(300);
        const today = await page.locator('[data-menu-item="today"]').count();
        await page.keyboard.press("Escape");
        await sleep(200);
        if (today !== 0) fail("⓪b the pill's menu still offers 「今天」 (UX-39 §4)");
        console.log(`⓪b column ${JSON.stringify(geo.column)}, the day first in 最近 with ${geo.marks} marks, ${geo.lines} pill lines`);
      }
      const open = await ariaOf(page, first);
      if (open.expanded !== "true" || !open.controls || open.controls !== open.column) {
        fail(`⓪b the open front tab says ${JSON.stringify(open)} — aria-expanded=true and aria-controls naming the column`);
      }
      const after = await hintOf(page, first);
      if (!HINT.hide.includes(after)) fail(`⓪b the front tab's hover reads ${JSON.stringify(after)} while the column is open — want ${HINT.hide.join(" / ")}`);
      console.log(`⓪b hover: 「${before}」 → 「${after}」`);
    }
    // …on the name: after the double-click window, and only once
    await page.locator(`[data-tab="${first}"] [data-session-title]`).click();
    if (!(await waitColumn(page, false, 1_500))) fail("⓪b a click on the front tab's name did not close the column within 1.5s");
    // …and by keys, on the focused tab
    await frontOpen(page, first).focus();
    await page.keyboard.press("Enter");
    if (!(await waitColumn(page, true, 600))) fail("⓪b Enter on the focused front tab did not open the column");
    await page.keyboard.press(" ");
    if (!(await waitColumn(page, false, 600))) fail("⓪b Space on the focused front tab did not close the column");

    // c. a double-click renames, and the column holds still — both ways
    for (const shown of [false, true]) {
      if (shown) {
        await pressFace(page, first);
        await waitColumn(page, true, 600);
      }
      await page.mouse.move(700, 600);
      const watching = watchColumn(page, 1_000);
      await page.locator(`[data-tab="${first}"] [data-session-title]`).dblclick();
      const frames = await watching;
      const editor = await page.locator(`[data-tab="${first}"] [data-title-input]`).count();
      const flips = frames.filter((v, i) => i > 0 && v !== frames[i - 1]).length;
      if (editor !== 1) fail(`⓪c a double-click on the front tab's name opened ${editor} editor(s) — rename must still win`);
      if (frames.length < 10) fail(`probe dead: ⓪c sampled ${frames.length} frame(s) across the double-click`);
      else if (flips !== 0 || frames[0] !== shown) {
        fail(
          `⓪c a double-click on the name ${shown ? "with the column open" : "with the column closed"} moved the column: ` +
            `${frames.filter(Boolean).length} of ${frames.length} frames had one, ${flips} flip(s) — it renames, it does not toggle`,
        );
      }
      console.log(`⓪c double-click, column ${shown ? "open" : "closed"}: ${frames.length} frames, ${flips} flips, editor ${editor}`);
      await page.keyboard.press("Escape");
      await sleep(200);
    }

    // d. it outlives a reload (the column is open now, from c)
    if (!(await openToday(page)) || !(await waitRows(page, true, 1_000))) fail("probe dead: ⓪d the column did not show the day before the reload");
    await page.reload({ waitUntil: "load" });
    const kept = await until("the column and its day, back after a reload with nobody pressing", 20_000, async () =>
      (await columns(page)) === 1 && (await page.locator("[data-session-side] [data-detail-day] [data-day-strip]").count()) === 1,
    );
    if (!kept) fail("⓪d the column was open on 最近 with its day before the reload and is not after it — the choice is remembered per session");
  } else {
    fail("probe dead: ⓪ had no fixture row to open");
  }

  // e. a day with no record: the ruler is drawn as what it is, the
  //    expanded view says so — in both shapes it arrives in (the route
  //    above)
  for (const shape of ["unwatched", "missing"]) {
  if (shape !== emptyAs) {
    emptyAs = shape;
    // The next answer carries the new shape; a reload asks at once
    // rather than waiting out the poll.
    await page.reload({ waitUntil: "load" });
    await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 0);
  }
  for (const id of fixtureQuiet) {
    if (!(await openTab(id))) continue;
    if ((await columns(page)) !== 0) fail(`⓪e ${id} (${shape}) opened with a column — another session's memory leaked into it`);
    await pressFace(page, id);
    if (!(await waitColumn(page, true, 1_000))) {
      fail(`⓪e pressing ${id}'s front tab (${shape}) opened no column`);
      continue;
    }
    // The row stands; a day with no record holds no band and no word.
    let got = null;
    await until(`${id}'s day row`, 10_000, async () => {
      got = await page.evaluate(() => {
        const row = document.querySelector("[data-session-side] [data-detail-day]");
        return row ? { bands: row.querySelectorAll("[data-day-strip]").length, words: row.querySelectorAll("[data-day-mark]").length, skeleton: row.querySelectorAll("[data-day-skeleton]").length } : null;
      });
      return got !== null && got.skeleton === 0;
    });
    if (!got) fail(`⓪e ${id} (${shape}) has no record and its 附栏 draws no day row — the row stands`);
    else if (got.bands !== 0 || got.words !== 0) fail(`⓪e ${id} (${shape}) has no record and ${got.bands} band(s), ${got.words} mark(s) stand in its row — an empty band is the blank band`);
    console.log(`⓪e ${id} (${shape}): ${JSON.stringify(got)}`);
    await page.keyboard.press("Escape");
    await pressFace(page, id);
    await waitColumn(page, false, 1_000);
  }
  }
  // …and the quiet sessions' memories are their own: the first session
  // still opens with its column (its day folded when a row beside it was
  // pressed — a press outside folds it).
  if (first && (await openTab(first))) {
    const still = await until("the first session's column, still open after the quiet ones", 10_000, async () => (await columns(page)) === 1);
    if (!still) fail("⓪e opening the sessions with no record changed the first session's memory — one memory per session");
  }

  // f. on its way: a skeleton in the band's own geometry in the row
  {
    const slow = await browser.newContext({ viewport: { width: 1280, height: 900 } });
    await slow.addInitScript((id) => {
      try {
        localStorage.setItem(`khor.session-side.${id}`, JSON.stringify({ open: true, w: null, seat: "recent" }));
        localStorage.setItem("khor.sessions.tabs", JSON.stringify({ tabs: [id], active: id }));
      } catch {
        /* then nothing is asked for, and the probe below says so */
      }
    }, first);
    let release = () => {};
    const held = new Promise((r) => (release = r));
    await slow.route("**/day_strips", async (route) => {
      await held;
      await route.continue();
    });
    const sp = await slow.newPage();
    const slowThrew = [];
    sp.on("pageerror", (e) => slowThrew.push(String(e)));
    await sp.goto(`http://localhost:${PORT}/?bridge=${BRIDGE_PORT}`, { waitUntil: "load" });
    const skeleton = await (async () => {
      const end = Date.now() + 15_000;
      while (Date.now() < end) {
        const got = await sp.evaluate(() => {
          const row = document.querySelector("[data-session-side] [data-detail-day]");
          const sk = row?.querySelector("[data-day-skeleton]");
          const pill = sk?.querySelector("[data-skeleton]");
          if (!row) return null;
          const probe = document.createElement("span");
          probe.style.backgroundColor = "var(--fill)";
          document.body.append(probe);
          const fill = getComputedStyle(probe).backgroundColor;
          probe.remove();
          const r = row.getBoundingClientRect();
          const cs = getComputedStyle(sk ?? row);
          const rs = getComputedStyle(row);
          const p = pill?.getBoundingClientRect();
          const ps = pill ? getComputedStyle(pill) : null;
          return {
            band: !!row.querySelector("[data-day-strip]"),
            skeleton: !!sk,
            busy: sk?.getAttribute("aria-busy") ?? null,
            row: { left: r.left, right: r.right, top: r.top, h: r.height },
            // The row's own inset (the page's) and the skeleton's.
            pad: {
              l: (parseFloat(rs.paddingLeft) || 0) + (parseFloat(cs.paddingLeft) || 0),
              r: (parseFloat(rs.paddingRight) || 0) + (parseFloat(cs.paddingRight) || 0),
            },
            pill: p && { left: p.left, right: p.right, top: p.top, h: p.height, bg: ps.backgroundColor, corner: parseFloat(ps.borderTopLeftRadius) || 0 },
            fill,
          };
        });
        if (got) return got;
        await sleep(40);
      }
      return null;
    })();
    if (!skeleton) fail("⓪f with the column open on 最近 and the day's answer held back, no day row stood at all — the day is on its way: a skeleton, not nothing");
    else if (skeleton.band) fail("probe dead: ⓪f a band was drawn while its day route was held — the hold did not hold");
    else if (!skeleton.skeleton || !skeleton.pill) fail(`⓪f the open day while it loads holds no skeleton (${JSON.stringify(skeleton)}) — a blank row is the blank band`);
    else {
      const k = skeleton;
      if (k.busy !== "true") fail(`⓪f the skeleton says aria-busy=${k.busy}`);
      if (k.pill.bg !== k.fill) fail(`⓪f the skeleton is painted ${k.pill.bg}, not \`fill\` (${k.fill}) — the app's skeleton language`);
      const full = Math.abs(k.pill.left - (k.row.left + k.pad.l)) < 0.5 && Math.abs(k.pill.right - (k.row.right - k.pad.r)) < 0.5;
      if (!full) fail(`⓪f the skeleton spans ${k.pill.left.toFixed(1)}..${k.pill.right.toFixed(1)} in a row whose axis is ${(k.row.left + k.pad.l).toFixed(1)}..${(k.row.right - k.pad.r).toFixed(1)} — one pill the whole axis`);
      if (k.pill.corner < k.pill.h / 2) fail(`⓪f the skeleton's corner is ${k.pill.corner} on a ${k.pill.h}px pill — it is a pill like the band's`);
      release();
      const landed = await (async () => {
        const end = Date.now() + 15_000;
        while (Date.now() < end) {
          const got = await sp.evaluate(() => {
            const row = document.querySelector("[data-session-side] [data-detail-day]");
            const ink = row?.querySelector("[data-day-strip] [data-day-mark] > *");
            if (!row || !ink) return null;
            const r = row.getBoundingClientRect();
            const i = ink.getBoundingClientRect();
            return { row: { top: r.top, h: r.height }, ink: { top: i.top, h: i.height }, skeleton: !!row.querySelector("[data-day-skeleton]") };
          });
          if (got) return got;
          await sleep(60);
        }
        return null;
      })();
      if (!landed) fail("⓪f the day's answer was let through and no band replaced the skeleton");
      else {
        if (landed.skeleton) fail("⓪f the band arrived and the skeleton is still in the row");
        // The row holds still: the band lands where its skeleton stood.
        if (Math.abs(landed.row.top - k.row.top) > 0.5) {
          fail(`⓪f the skeleton's row is at ${k.row.top}, the band's at ${landed.row.top} — the row does not move when the day lands`);
        }
        if (Math.abs(landed.ink.h - k.pill.h) > 0.5 || Math.abs(landed.ink.top - k.pill.top) > 0.5) {
          fail(`⓪f the skeleton's pill is ${k.pill.h} tall at ${k.pill.top}, the band's pills ${landed.ink.h} at ${landed.ink.top} — the band's own geometry`);
        }
      }
      console.log(`⓪f skeleton: ${JSON.stringify(k.pill)} in a ${k.row.h}px row; then ${JSON.stringify(landed)}`);
    }
    release();
    if (slowThrew.length) fail(`⓪f the page threw: ${slowThrew.join(" | ")}`);
    await slow.close();
  }

  const openRow = async (id) => {
    await page.locator(`[data-row="${id}"] [data-row-open]`).click();
    if (!(await until(`${id} in front`, 10_000, async () => (await frontOpen(page, id).count()) === 1))) return false;
    // The day is 最近's first row in the 附栏 (UX-39 §4): open it the way
    // a person would, by the front tab's press.
    await sleep(400);
    await openToday(page);
    const up = await until(`the band for ${id}`, 25_000, async () => {
      const on = await page.locator("[data-detail-pane]").count();
      return on > 0 && (await page.locator("[data-session-side] [data-detail-day] [data-day-strip]").count()) === 1;
    });
    await sleep(300);
    return up;
  };
  const readBand = () =>
    page.evaluate(() => {
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

      const marks = [...document.querySelectorAll("[data-detail-day] [data-day-mark]")].map((el) => {
        const kid = el.firstElementChild;
        return {
          mark: el.dataset.dayMark,
          kidBg: kid ? getComputedStyle(kid).backgroundColor : null,
          kidH: kid ? Math.round(kid.getBoundingClientRect().height * 100) / 100 : null,
          kids: el.childElementCount,
        };
      });

      const px = (n) => Math.round(n * 100) / 100;
      // The band's pane is 最近's first row in the 附栏 (UX-39 §4; it was
      // the panel hanging from the pill, the column under its ruler, and
      // before that the detail pane): its content box, the page inset in
      // (the strip keeps its own inset inside its box); its top is under
      // the row's line of words, so the band's top is measured from that
      // line's foot.
      const pane = document.querySelector("[data-session-side] [data-detail-day]");
      const strip = pane?.querySelector("[data-day-strip]");
      const words = pane?.firstElementChild;
      const h = pane && pane.getBoundingClientRect();
      const st = strip && strip.getBoundingClientRect();
      const ps = pane ? getComputedStyle(pane) : null;
      const box = {
        paneL: h ? px(h.left + parseFloat(ps.paddingLeft)) : null,
        paneR: h ? px(h.right - parseFloat(ps.paddingRight)) : null,
        paneT: words ? px(words.getBoundingClientRect().bottom + (parseFloat(ps.rowGap) || 0)) : null,
        stripT: st ? px(st.top) : null,
        stripL: st ? px(st.left) : null,
        stripR: st ? px(st.right) : null,
        stripH: st ? px(st.height) : null,
        runs: strip ? strip.childElementCount : 0,
      };
      return { wants, marks, box };
    });
  // `wants` starts empty rather than null so that a run where no band
  // could be read falls through to the probes below as a clean red
  // ("no word cells on screen") instead of a TypeError — measured on
  // the tear-down that removed the band: a crash is red too, but it is
  // not the sentence that says what is missing.
  const seen = { wants: {}, marks: [], boxes: [] };
  let bandsRead = 0;
  let rowWithUnwatched = null;
  // The row ⑥ presses on: the one whose day has a 待批 stretch, because
  // that is the word the strip exists for.
  let rowWithBlocked = null;
  // **The fixture's rows with a record, not every row on the list**
  // (2026-09-29): a row whose day has no word draws no band now (⓪ e —
  // the quiet row, and the machine's own conversation, which no sample
  // ever named), and waiting for one under it would be waiting for the
  // blank band to come back.
  for (const id of fixtureRows) {
    if (!(await openRow(id))) continue;
    const got = await readBand();
    seen.wants = got.wants;
    seen.marks.push(...got.marks);
    seen.boxes.push({ id, ...got.box });
    bandsRead += 1;
    if (rowWithUnwatched === null && got.marks.some((m) => m.mark === "unwatched")) rowWithUnwatched = id;
    if (rowWithBlocked === null && got.marks.some((m) => m.mark === "blocked")) rowWithBlocked = id;
  }
  if (bandsRead === 0) fail("probe dead: no row's band could be read — every claim below would pass on nothing");
  // The ink claim (② b) reads the band on screen; leave up the row that
  // has a 不知道 stretch to read.
  if (rowWithUnwatched !== null) await openRow(rowWithUnwatched);

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
  // **待批 by name, and only this one.** Four distinct words is enough to
  // tell a derivation from a constant, and it stayed enough while the
  // fixture quietly stopped producing 待批 at all — a window change moved
  // that stretch into the unwatched hours, the count fell from six words
  // to five, and nothing was red. It is the word the strip exists for
  // (`khor_core::DayCell::rank` biases every merge toward it), so a
  // fixture that has stopped emitting it is a fixture no longer covering
  // the case that matters, whatever else it still shows.
  if (!kinds.has("blocked")) {
    fail(
      `probe dead: no 待批 cell on screen (saw ${[...kinds].sort().join("+")}) — the merge is biased ` +
        "toward that word and nothing here is exercising it (day_fixture's window moved past it)",
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
  // **One axis, not a line beside a bar.** 不知道 fills the same height
  // a word does. This used to assert the opposite — that it was thinner
  // — and it was green while a user read the band as two objects:
  // 「左边有一条细线,右边才是 bar」. A time axis whose tiers are
  // different *kinds of thing* stops being read as one axis, and every
  // comparison along it goes with it.
  const hair = unwatched[0]?.kidH ?? null;
  const block = words[0]?.kidH ?? null;
  if (hair !== null && block !== null && hair !== block) {
    fail(
      `② 不知道 is ${hair} tall and a word is ${block} — one axis means one height. ` +
        "A tier of a different shape reads as a different object, not as a stretch of the same line.",
    );
  }

  // ── 2b) and the ink has to be visible, which geometry cannot say
  //
  // **An ink nobody can see is not a faint tier, it is no tier.** The
  // height claim above cannot see this: it was green while the ink was
  // `--border` at 1.22, and on screen 不知道 and 那时没有这一行 were the
  // same thing — blank. With khor young most of every strip is 不知道,
  // so the blank ran most of the way across and a person read the axis
  // as running right-to-left.
  //
  // Two bounds, both themes, because either alone is satisfiable by
  // something plainly wrong.
  //
  // **The floor is 1.15 and not 3.0**, and that is a change of subject
  // rather than a relaxation: 3.0 is what a *stroke* needs to be made
  // out, and 不知道 is now a filled block. `check-rail` carries both
  // numbers for exactly this reason, and its own rail block ships at
  // 1.30 / 1.61 — measured, and judged visible.
  //
  // **The ceiling is the harder one now, and it moved.** It used to be
  // "under the faintest word". That is too weak once the tier is a block
  // the same size and shape as a word: 空闲 is the one word left with no
  // hue (忙碌 took indigo), so a 不知道 block drifting up in weight ends
  // as two grey blocks separated only by lightness — 「没有数据」 painted
  // as 「空闲」, the thing this batch exists to refuse. So the gap is
  // required to be a *factor*, not a margin.
  const INK_FLOOR = 1.15;
  // How much lighter than the faintest word 不知道 has to be. Two is
  // well inside what the shipped value clears (×2.9 light, ×2.61 dark)
  // and well outside what a nudge toward 空闲 would leave.
  const INK_GAP = 2.0;
  for (const scheme of ["light", "dark"]) {
    await flipTheme(page, scheme);
    // …and then the transitions, which is a different wait: a computed
    // style read the instant the palette turns is the colour it is
    // leaving. This measurement came back with both columns identical
    // twice before it was here.
    await new Promise((r) => setTimeout(r, 500));
    const isDark = await page.evaluate(() => matchMedia("(prefers-color-scheme: dark)").matches);
    if (isDark !== (scheme === "dark")) {
      fail(`probe dead: asked for ${scheme}, the page reports dark=${isDark} — ② b measured nothing`);
      continue;
    }
    const ink = await page.evaluate(() => {
      // `color(srgb …)` as well as rgb()/rgba(): that is what
      // `color-mix` computes to and its channels are 0..1, so reading it
      // as 0..255 gives a near-black with a plausible alpha — every
      // number after it wrong, and wrong plausibly.
      const parse = (v) => {
        const t = String(v);
        const s = t.match(/color\(srgb\s+([^)]+)\)/);
        if (s) {
          const n = s[1].split(/[\s/]+/).filter(Boolean).map(Number);
          return { r: n[0] * 255, g: n[1] * 255, b: n[2] * 255, a: n.length > 3 ? n[3] : 1 };
        }
        const m = t.match(/rgba?\(([^)]+)\)/);
        if (!m) return null;
        const n = m[1].split(/[,\s/]+/).filter(Boolean).map(Number);
        return { r: n[0], g: n[1], b: n[2], a: n.length > 3 ? n[3] : 1 };
      };
      const over = (f, b) => ({
        r: f.r * f.a + b.r * (1 - f.a),
        g: f.g * f.a + b.g * (1 - f.a),
        b: f.b * f.a + b.b * (1 - f.a),
        a: 1,
      });
      const lum = (c) => {
        const g = (v) => {
          v /= 255;
          return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
        };
        return 0.2126 * g(c.r) + 0.7152 * g(c.g) + 0.0722 * g(c.b);
      };
      const ratio = (a, b) => {
        const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
        return Math.round(((x + 0.05) / (y + 0.05)) * 100) / 100;
      };
      // The ground it actually sits on, walked up until something paints
      // — not `--card` assumed.
      const line = document.querySelector('[data-day-mark="unwatched"] > *');
      let el = line;
      let ground = null;
      while (el && !ground) {
        const bg = parse(getComputedStyle(el).backgroundColor);
        if (bg && bg.a > 0 && el !== line) ground = bg;
        el = el.parentElement;
      }
      if (!line || !ground) return null;
      const hairInk = parse(getComputedStyle(line).backgroundColor);
      const words = ["busy", "blocked", "done", "errored", "failed", "idle"];
      const sw = document.createElement("span");
      document.body.appendChild(sw);
      const wordRatios = words.map((w) => {
        sw.style.backgroundColor = "";
        sw.style.backgroundColor = `var(--state-${w})`;
        const c = parse(getComputedStyle(sw).backgroundColor);
        return c ? ratio(over(c, ground), ground) : Infinity;
      });
      sw.remove();
      return { hair: ratio(over(hairInk, ground), ground), faintestWord: Math.min(...wordRatios) };
    });
    if (ink === null) {
      fail(`probe dead: no 不知道 block to measure in ${scheme} — ② b asserted nothing`);
      continue;
    }
    if (ink.hair < INK_FLOOR) {
      fail(
        `② ${scheme}: the 不知道 block is ${ink.hair} against its ground, under the ${INK_FLOOR} ` +
          "a filled block needs. An ink nobody can see is not a faint tier, it is no tier.",
      );
    }
    const gap = Math.round((ink.faintestWord / ink.hair) * 100) / 100;
    if (gap < INK_GAP) {
      fail(
        `② ${scheme}: the 不知道 block is ${ink.hair} against the faintest word's ${ink.faintestWord} ` +
          `— only ×${gap} lighter, under the ×${INK_GAP} required. Two blocks of the same size and ` +
          "shape separated by that little are 「没有数据」 wearing 「空闲」's face.",
      );
    }
    console.log(
      `ink ${scheme}: 不知道 ${ink.hair} (floor ${INK_FLOOR}) vs faintest word ${ink.faintestWord} — ×${gap} lighter (need ×${INK_GAP})`,
    );
  }
  await flipTheme(page, "light");

  // ── 3) one axis ───────────────────────────────────────────────────
  //
  // One band at a time, so "the strips share an axis" is the row's own
  // edges: each session's band spans the row it sits in (最近's first,
  // UX-39 §4), directly under the row's line of words. And the same
  // length for every session, which is what makes two sessions' days
  // comparable when read one after the other.
  for (const b of seen.boxes) {
    if (b.stripL === null) {
      fail(`③ ${b.id} has no band in its pane`);
      continue;
    }
    if (b.stripL !== b.paneL || b.stripR !== b.paneR) {
      fail(`③ ${b.id} band spans ${b.stripL}..${b.stripR}, the row's axis spans ${b.paneL}..${b.paneR}`);
    }
    if (b.stripT !== b.paneT) {
      fail(
        `③ ${b.id} band starts ${+(b.stripT - b.paneT).toFixed(2)} below the row's words — ` +
          "the band is directly under 过去24小时 and its totals: nothing stands between them",
      );
    }
  }
  const lengths = new Set(seen.boxes.filter((b) => b.stripL !== null).map((b) => +(b.stripR - b.stripL).toFixed(1)));
  if (lengths.size > 1) {
    fail(`③ the bands are not one length: ${[...lengths]} — two sessions' days read on one scale`);
  }

  console.log(`palette: ${JSON.stringify(seen.wants)}`);
  console.log(
    `cells: ${words.length} word (${[...kinds].sort().join("+")}), ` +
      `${unwatched.length} 不知道, ${absent.length} 那时没有这一行`,
  );
  console.log(`heights: 不知道 ${hair}, word ${block} — one axis`);
  console.log(
    `axis: ${seen.boxes.map((b) => `${b.id} ${b.stripL}..${b.stripR} h${b.stripH} ⊤${b.stripT}/${b.paneT} ${b.runs} runs`).join(", ")}`,
  );

  // ── 6) the band is a control (2026-09-29) ─────────────────────────
  //
  // 「session的状态条也弄成圆角的bar…鼠标悬停在bar上要显示时间与状态。
  // 点击bar要有一个弹窗显示详情，照顾手机没有悬停。」 Every half of that
  // is a thing on the glass a static read cannot see, so each is read
  // back here, and each was red on the code before it:
  //
  //   a. every inked run is a pill — its corners at least half the band's
  //      height — with a gap between neighbours and never narrower than
  //      it is tall (a run the rounding ate would be a stretch of the day
  //      that silently is not there);
  //   b. the pointer's readout says the **run's** span and its word, is
  //      on screen (not clipped by the board it sits at the top of), and
  //      stays inside the pane at both ends of the day;
  //   c. a press opens a popover saying the same span and word — and only
  //      one float at a time; a second press on the run, or a press
  //      anywhere else, closes it;
  //   d. one Tab stop, ← → walk the runs with a visible ring, Enter opens,
  //      Esc closes and puts focus back on the run;
  //   e. a finger (a touch context, no hover at all) gets the popover;
  //   f. the band's hit area covers nothing else — the tab strip's lower
  //      pixels hit the tab, and nothing a few pixels over or under the
  //      band's row hits the band, wide and narrow.
  //
  // a–e were red on the code before them. **f was not, and why is worth
  // keeping**: the old hit boxes reached 12 px *up*, and the board they
  // sit in is `overflow: hidden`, which clips hit-testing as well as
  // paint — so the reach-up was cut off at the board's edge and bought
  // nothing (a first version of f, sampling half-pixels, went red on the
  // old code only because a point 0.4 px above an edge is hit-tested as
  // the pixel it rounds into). f was seen red instead on a hit box made
  // to reach down into the session (`before:-bottom-3`): 298 points under
  // the row on the wide face, 124 on the narrow one.
  //
  // The words are the catalog's (`catalogWords`, read before ⓪).
  const wordsOf = (mark) => [catalogWords.zh[mark], catalogWords.en[mark]].filter(Boolean);
  if (wordsOf("blocked").length !== 2 || wordsOf("unwatched").length !== 2) {
    fail(`probe dead: ⑥ could not read the words out of src/gen/catalog.ts (${JSON.stringify(catalogWords)})`);
  }
  const SPAN = /(\d{2}:\d{2})–(\d{2}:\d{2})/;
  /** Every run's box, its ink's box and corners, in document order. */
  const readRuns = (p) =>
    p.evaluate(() => {
      const marks = [...document.querySelectorAll("[data-detail-day] [data-day-mark]")];
      return marks.map((el, i) => {
        const r = el.getBoundingClientRect();
        const ink = el.firstElementChild;
        const k = ink ? ink.getBoundingClientRect() : null;
        const cs = ink ? getComputedStyle(ink) : null;
        const corners = cs
          ? ["borderTopLeftRadius", "borderTopRightRadius", "borderBottomLeftRadius", "borderBottomRightRadius"].map(
              (c) => parseFloat(cs[c]) || 0,
            )
          : [];
        return {
          i,
          mark: el.dataset.dayMark,
          cells: Number(el.dataset.dayCells ?? 0),
          x: r.left + r.width / 2,
          left: r.left,
          right: r.right,
          top: r.top,
          bottom: r.bottom,
          tabIndex: el.tabIndex,
          ink: k && { left: k.left, right: k.right, top: k.top, bottom: k.bottom, w: k.width, h: k.height, y: k.top + k.height / 2 },
          corner: corners.length ? Math.min(...corners) : null,
        };
      });
    });
  /** What floats now: the readout and the popover, each with its text,
      its box, how much of it survives every clipping ancestor, and the
      pane's own edges — the 附栏 the day's row is in (UX-39 §4: the
      column, or the phone's sheet). */
  const readFloats = (p) =>
    p.evaluate(() => {
      const pane = (document.querySelector("[data-session-side]") ?? document.querySelector("[data-place-sheet]") ?? document.querySelector("[data-detail-pane]"))?.getBoundingClientRect() ?? null;
      const shown = (sel) => {
        const el = [...document.querySelectorAll(sel)].find((e) => {
          const r = e.getBoundingClientRect();
          return r.width > 0 && r.height > 0;
        });
        if (!el) return null;
        const r = el.getBoundingClientRect();
        // Visible area after every ancestor that clips: the readout this
        // replaced was inside the board, above its top edge, and the
        // board is `overflow: hidden` — present in the DOM, 0% on screen.
        let [l, t, rt, b] = [r.left, r.top, r.right, r.bottom];
        for (let a = el.parentElement; a; a = a.parentElement) {
          const s = getComputedStyle(a);
          if (s.overflowX !== "visible" || s.overflowY !== "visible") {
            const q = a.getBoundingClientRect();
            l = Math.max(l, q.left);
            t = Math.max(t, q.top);
            rt = Math.min(rt, q.right);
            b = Math.min(b, q.bottom);
          }
        }
        const seen = Math.max(0, rt - l) * Math.max(0, b - t);
        return {
          text: (el.innerText ?? "").replace(/\s+/g, " ").trim(),
          left: r.left,
          right: r.right,
          top: r.top,
          bottom: r.bottom,
          visible: Math.round((seen / (r.width * r.height)) * 100) / 100,
        };
      };
      return { pane: pane && { left: pane.left, right: pane.right }, peek: shown("[data-day-peek]"), detail: shown("[data-day-detail]") };
    });
  const waitFloat = async (p, which, want, ms = 1_500) => {
    const end = Date.now() + ms;
    let last = null;
    while (Date.now() < end) {
      last = await readFloats(p);
      if (!!last[which] === want) return last;
      await sleep(60);
    }
    return last;
  };
  const activeIndex = (p) =>
    p.evaluate(() => [...document.querySelectorAll("[data-detail-day] [data-day-mark]")].indexOf(document.activeElement));
  /** The span and word a float says, checked against the run it is about. */
  const saysRun = (where, float, run) => {
    if (!float) return null;
    const span = float.text.match(SPAN);
    if (!span) fail(`⑥ ${where}: 「${float.text}」 carries no HH:MM–HH:MM span — the run's time, not only a moment in it`);
    if (!wordsOf(run.mark).some((w) => float.text.includes(w))) {
      fail(`⑥ ${where}: 「${float.text}」 does not name the run's word (${wordsOf(run.mark).join(" / ")})`);
    }
    return span ? span[0] : null;
  };
  /** Points in a horizontal line of pixels that land on the band. */
  const hitsBand = (p, y, from, to) =>
    p.evaluate(
      ({ y, from, to }) => {
        const out = [];
        for (let x = from + 1; x < to - 1; x += 3) {
          const el = document.elementFromPoint(x, y);
          if (el && el.closest("[data-day-strip]")) out.push(Math.round(x));
        }
        return out;
      },
      { y, from, to },
    );

  /** ⑥a on one face's band: every inked run a pill, 2 apart, no thinner
      than it is tall — and **every cell at least a dot and its gap
      wide**, which is what makes the last true for a stretch of one cell
      whether or not this fixture happens to have one. */
  const pills = (where, runs) => {
    const inked = runs.filter((r) => r.mark !== "absent");
    for (const r of inked) {
      if (!r.ink || r.corner === null) {
        fail(`⑥a ${where}: the ${r.mark} run at ${r.i} has no ink to measure`);
        continue;
      }
      if (r.corner < r.ink.h / 2) {
        fail(`⑥a ${where}: the ${r.mark} run at ${r.i} has a ${r.corner}px corner on a ${r.ink.h}px band — a pill needs ≥ ${r.ink.h / 2}`);
      }
      if (r.ink.w < r.ink.h - 0.01) {
        fail(`⑥a ${where}: the ${r.mark} run at ${r.i} is ${r.ink.w.toFixed(2)} wide on a ${r.ink.h} band — the rounding has eaten it`);
      }
    }
    for (let k = 0; k + 1 < runs.length; k += 1) {
      const [a, b] = [runs[k], runs[k + 1]];
      if (a.mark === "absent" || b.mark === "absent" || !a.ink || !b.ink) continue;
      const gap = b.ink.left - a.ink.right;
      if (gap < 1.99) fail(`⑥a ${where}: runs ${a.i} (${a.mark}) and ${b.i} (${b.mark}) are ${gap.toFixed(2)} apart — the gap between pills is 2`);
    }
    const band = inked[0]?.ink?.h ?? 0;
    const counted = runs.filter((r) => r.cells > 0);
    const cell = counted.length ? Math.min(...counted.map((r) => (r.right - r.left) / r.cells)) : NaN;
    if (!(cell >= band + 2 - 0.01)) {
      fail(
        `⑥a ${where}: a cell is ${cell.toFixed(2)} wide — a stretch of one cell would be a pill under its own ` +
          `${band}px height once its 2px gap is taken (use-day-strips CELL_WIDTH)`,
      );
    }
    console.log(
      `⑥a ${where}: ${inked.length} pills, corners ≥ ${Math.min(...inked.map((r) => r.corner ?? 0))}, ` +
        `narrowest ${Math.min(...inked.map((r) => r.ink?.w ?? 0)).toFixed(2)} wide, a cell ${cell.toFixed(2)} wide`,
    );
  };

  if (rowWithBlocked === null) {
    fail("probe dead: ⑥ found no row with a 待批 stretch to press on");
  } else {
    await openRow(rowWithBlocked);
    await page.mouse.move(700, 600);
    await sleep(300);
    const runs = await readRuns(page);
    const inked = runs.filter((r) => r.mark !== "absent");
    if (inked.length < 3) fail(`probe dead: ⑥ the band has ${inked.length} inked run(s) — too few to walk`);

    // a. pills, gaps, and nothing eaten
    pills("wide", runs);

    // b. the readout: the run's span and word, on screen, inside the pane
    const target = inked.find((r) => r.mark === "blocked") ?? inked[0];
    const ends = [inked[0], inked[inked.length - 1]];
    const spans = {};
    for (const r of [target, ...ends]) {
      await page.mouse.move(r.x, r.ink?.y ?? r.top + 2);
      const f = await waitFloat(page, "peek", true, 800);
      if (!f?.peek) {
        fail(`⑥b pointing at the ${r.mark} run at ${r.i} shows no readout within 800ms`);
        continue;
      }
      spans[r.i] = saysRun(`the readout over run ${r.i}`, f.peek, r);
      if (f.peek.visible < 0.99) {
        fail(
          `⑥b the readout over run ${r.i} is ${Math.round(f.peek.visible * 100)}% on screen — ` +
            "an ancestor clips it (the board is overflow: hidden, and the band is its top edge)",
        );
      }
      if (f.pane && (f.peek.left < f.pane.left - 0.5 || f.peek.right > f.pane.right + 0.5)) {
        fail(`⑥b the readout over run ${r.i} spans ${f.peek.left.toFixed(1)}..${f.peek.right.toFixed(1)}, past the pane's ${f.pane.left}..${f.pane.right}`);
      }
      console.log(`⑥b readout over ${r.mark}@${r.i}: 「${f.peek.text}」 ${Math.round(f.peek.visible * 100)}% on screen`);
      await page.mouse.move(700, 600);
      await waitFloat(page, "peek", false, 800);
    }

    // c. a press: the same span and word, one float, and the two ways out
    const press = async (r) => page.mouse.click(r.x, r.ink?.y ?? r.top + 2);
    await press(target);
    let f = await waitFloat(page, "detail", true);
    if (!f?.detail) {
      fail(`⑥c pressing the ${target.mark} run opened no popover`);
    } else {
      const said = saysRun("the popover", f.detail, target);
      if (said && spans[target.i] && said !== spans[target.i]) {
        fail(`⑥c the popover says ${said} and the readout said ${spans[target.i]} — one run, one span`);
      }
      // The readout is told to go at the press; give its exit (the
      // tooltip's 120 ms fade) the time it takes, then judge.
      const under = f.peek ? (await waitFloat(page, "peek", false, 600))?.peek : null;
      if (under) fail(`⑥c the readout 「${under.text}」 is still up under the open popover — one float at a time`);
      if (f.pane && (f.detail.left < f.pane.left - 0.5 || f.detail.right > f.pane.right + 0.5)) {
        fail(`⑥c the popover spans ${f.detail.left.toFixed(1)}..${f.detail.right.toFixed(1)}, past the pane's ${f.pane.left}..${f.pane.right}`);
      }
      console.log(`⑥c popover: 「${f.detail.text}」`);
      await press(target);
      if ((await waitFloat(page, "detail", false))?.detail) fail("⑥c pressing the same run again left the popover open");
      await press(target);
      await waitFloat(page, "detail", true);
      // Outside the run's popover: the session's own pane.
      await page.mouse.click(700, 600);
      if ((await waitFloat(page, "detail", false))?.detail) fail("⑥c a press outside left the popover open");
    }

    // d. the keyboard: one stop, arrows, a ring, Enter, Esc and back
    const stops = runs.filter((r) => r.tabIndex >= 0);
    if (stops.length !== 1) {
      fail(`⑥d the band has ${stops.length} Tab stop(s) — it is one stop, and ← → walk the runs inside it`);
    } else {
      await page.evaluate((i) => document.querySelectorAll("[data-detail-day] [data-day-mark]")[i].focus(), stops[0].i);
      const at = inked.findIndex((r) => r.i === stops[0].i);
      const back = at > 0 ? inked[at - 1] : null;
      await page.keyboard.press("ArrowLeft");
      const moved = await activeIndex(page);
      if (back && moved !== back.i) fail(`⑥d ← from run ${stops[0].i} landed on ${moved}, not the inked run before it (${back.i})`);
      const ring = await page.evaluate(() => {
        const el = document.activeElement;
        const probe = document.createElement("span");
        probe.style.color = "var(--ring)";
        document.body.append(probe);
        const want = getComputedStyle(probe).color;
        probe.remove();
        const on = [el, el?.firstElementChild].filter(Boolean).map((e) => {
          const s = getComputedStyle(e);
          return { style: s.outlineStyle, width: parseFloat(s.outlineWidth) || 0, color: s.outlineColor };
        });
        return { want, on };
      });
      if (!ring.on.some((o) => o.style !== "none" && o.width >= 1 && o.color === ring.want)) {
        fail(`⑥d the focused run wears no \`ring\` outline (${JSON.stringify(ring.on)}, ring is ${ring.want})`);
      }
      await page.keyboard.press("ArrowRight");
      if (back && (await activeIndex(page)) !== stops[0].i) fail("⑥d → did not walk back to where ← started");
      const keyed = inked.find((r) => r.i === stops[0].i);
      for (const key of ["Enter", " "]) {
        await page.keyboard.press(key);
        f = await waitFloat(page, "detail", true);
        if (!f?.detail) {
          fail(`⑥d ${key === " " ? "Space" : key} on a focused run opened no popover`);
          continue;
        }
        saysRun(`the popover from ${key === " " ? "Space" : key}`, f.detail, keyed);
        await page.keyboard.press("Escape");
        if ((await waitFloat(page, "detail", false))?.detail) fail("⑥d Esc left the popover open");
        const home = await activeIndex(page);
        if (home !== keyed.i) fail(`⑥d after Esc focus is on ${home}, not on the run the popover came from (${keyed.i})`);
      }
    }

    // f. wide: the tab's lower pixels are the tab's
    const geo = await page.evaluate(() => {
      const strip = document.querySelector("[data-detail-day] [data-day-strip]")?.getBoundingClientRect();
      const tab = document.querySelector('[data-tab][data-on="true"]')?.getBoundingClientRect();
      // **The tab is a capsule since the redesign** (2026-10-02): its
      // rounded ends are not its pixels, so the probe walks the straight
      // run between them — the corner's radius in from each end.
      const el = document.querySelector('[data-tab][data-on="true"]');
      const round = el ? Math.min(parseFloat(getComputedStyle(el).borderBottomLeftRadius) || 0, tab.height / 2) : 0;
      return strip && { top: strip.top, bottom: strip.bottom, left: strip.left, right: strip.right, tab: tab && { left: tab.left + round, right: tab.right - round, bottom: tab.bottom } };
    });
    if (!geo?.tab) {
      fail("probe dead: ⑥f found no front tab over the band");
    } else {
      const onTab = await page.evaluate(({ tab }) => {
        const out = [];
        for (let y = tab.bottom - 1; y >= tab.bottom - 4; y -= 1) {
          for (let x = tab.left + 2; x < tab.right - 2; x += 3) {
            const el = document.elementFromPoint(x, y);
            if (!el || !el.closest("[data-tab]")) out.push(`${Math.round(x)},${y}→${el ? el.closest("[data-day-strip]") ? "the band" : el.tagName.toLowerCase() : "nothing"}`);
          }
        }
        return out;
      }, geo);
      if (onTab.length) fail(`⑥f wide: ${onTab.length} point(s) on the front tab's lower 4px do not hit the tab: ${onTab.slice(0, 4).join("; ")}`);
      // Whole pixels away from the box's edges: a point less than one
      // pixel off an edge is hit-tested as the pixel it rounds to
      // (measured: 47.4 and 47.6 above a box starting at 48 both land in
      // it), which would be this probe measuring its own rounding.
      for (const dy of [1, 2, 3, 4]) {
        const above = await hitsBand(page, geo.top - dy, geo.left, geo.right);
        if (above.length) fail(`⑥f wide: ${above.length} point(s) ${dy}px above the band's box hit the band (x ${above.slice(0, 4).join(",")}…)`);
        const below = await hitsBand(page, geo.bottom + dy, geo.left, geo.right);
        if (below.length) fail(`⑥f wide: ${below.length} point(s) ${dy}px under the band's box hit the band (x ${below.slice(0, 4).join(",")}…)`);
      }
      const onBand = await page.evaluate(({ x, y }) => !!document.elementFromPoint(x, y)?.closest("[data-day-mark]"), {
        x: target.x,
        y: target.ink?.y ?? target.top + 2,
      });
      if (!onBand) fail("probe dead: ⑥f the band's own ink does not hit the band — the hit geometry above measured nothing");
      console.log(`⑥f wide: band box ${geo.top.toFixed(1)}..${geo.bottom.toFixed(1)}, tab bottom ${geo.tab.bottom.toFixed(1)}`);
    }

    // e + f narrow: a finger, and nothing above or below
    const touch = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true });
    const phone = await touch.newPage();
    const phoneThrew = [];
    phone.on("pageerror", (e) => phoneThrew.push(String(e)));
    await phone.goto(`http://localhost:${PORT}/?bridge=${BRIDGE_PORT}`, { waitUntil: "load" });
    const listed = await (async () => {
      const end = Date.now() + 20_000;
      while (Date.now() < end) {
        if ((await phone.locator(`[data-row="${rowWithBlocked}"] [data-row-open]`).count()) > 0) return true;
        await sleep(200);
      }
      return false;
    })();
    if (!listed) {
      fail(`probe dead: ⑥e the narrow face never listed ${rowWithBlocked}`);
    } else {
      await phone.locator(`[data-row="${rowWithBlocked}"] [data-row-open]`).tap();
      // ⓪g **The pill's tap opens the sheet, the day is 最近's first row**
      // (UX-30 §6, UX-39 §4): the narrow session page's deck pill
      // (`components/SessionBar`) holds no day line, and its tap opens the
      // session's 附栏 as a bottom sheet on 最近, whose first row is the
      // day's band with its two totals; the row does not expand; Escape
      // puts the sheet away (at the end of ⑥). ⚠️ 墓碑 (UX-36, UX-37 §5):
      // the line in the pill's slot, 「今天」 expanding the day in place.
      // ⚠️ 墓碑: the front tab's press showed the band at the foot
      // (2026-10-02); before that the name and face in the window's first
      // row stood in (`[data-detail-header] [data-day-toggle]`).
      const pill = phone.locator("[data-session-deck] button[data-deck-title]");
      const seated = await (async () => {
        const end = Date.now() + 10_000;
        while (Date.now() < end) {
          if ((await pill.count()) === 1) return true;
          await sleep(100);
        }
        return false;
      })();
      await sleep(1_000);
      const sheetUp = (want) => {
        const end = Date.now() + 2_000;
        return (async () => {
          while (Date.now() < end) {
            if (((await phone.locator("[data-place-sheet]").count()) > 0) === want) return true;
            await sleep(40);
          }
          return false;
        })();
      };
      /** A tap on the row's words, off the band. */
      const rowTap = async () => {
        const b = await phone.locator("[data-place-sheet] [data-day-today] > :first-child").boundingBox();
        if (b) await phone.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2);
      };
      if (!seated) {
        fail("⓪g the narrow session deck has no pill to tap — the pill opens the session's 附栏 on the phone");
      } else {
        if ((await dayRows(phone)) !== 0) fail("⓪g a fresh phone draws a day row before anybody asked for it");
        const lines = await phone.locator("[data-session-deck] [data-pill-log], [data-day-ruler-line]").count();
        if (lines !== 0) fail(`⓪g the phone's session pill draws ${lines} day line(s) — no pill draws a day (UX-39 §4)`);
        const b = await pill.boundingBox();
        await phone.touchscreen.tap(b.x + b.width / 2, b.y + b.height / 2);
        if (!(await sheetUp(true))) fail("⓪g a tap on the pill opened no sheet");
        await sleep(600);
        const today = await phone.evaluate(() => {
          const scroller = document.querySelector("[data-place-sheet] [data-side-recent] [data-nav-clear]");
          const row = document.querySelector("[data-place-sheet] [data-side-recent] [data-day-today-row]");
          const list = document.querySelector("[data-place-sheet] [data-side-recent] [data-side-list]");
          return {
            row: !!row,
            first: !!row && scroller?.firstElementChild === row,
            says: row?.firstElementChild?.firstElementChild?.textContent ?? null,
            band: !!row?.querySelector("[data-day-strip], [data-day-skeleton]"),
            totals: row?.querySelector("[data-day-totals]")?.textContent ?? null,
            listTop: list ? Math.round(list.getBoundingClientRect().top) : null,
          };
        });
        const totalsAt = Date.now() + 5_000;
        while (today.totals === null && Date.now() < totalsAt) {
          await sleep(200);
          today.totals = await phone.locator("[data-place-sheet] [data-day-totals]").textContent().catch(() => null);
        }
        console.log(`⓪g narrow: the day in 最近 ${JSON.stringify(today)}`);
        if (!today.row || !today.first) fail(`⓪g the day is not 最近's first row in the sheet (${JSON.stringify(today)}) — UX-39 §4`);
        if (today.says !== "过去24小时" && today.says !== "Last 24h") fail(`⓪g the day's row is called ${JSON.stringify(today.says)} — 过去24小时, not 今天 (UX-39 §4)`);
        if (!today.band) fail("⓪g the day's row holds no band (DayStrip)");
        if (!/忙碌 .+ · 空闲 .+/.test(today.totals ?? "")) fail(`⓪g the day's row says ${JSON.stringify(today.totals)} — 忙碌 · 空闲, counted at the minute`);
        await rowTap();
        await sleep(500);
        const after = await phone.evaluate(() => ({
          listTop: Math.round(document.querySelector("[data-place-sheet] [data-side-recent] [data-side-list]")?.getBoundingClientRect().top ?? 0),
          grew: document.querySelectorAll("[data-place-sheet] [data-day-list], [data-place-sheet] [data-day-line]").length,
        }));
        if (after.grew !== 0 || (today.listTop !== null && after.listTop !== today.listTop)) fail(`⓪g a tap on the day's row moved 最近's rows ${today.listTop} → ${after.listTop} (${after.grew} day lines) — the row does not expand (UX-39 §4)`);
        console.log(`⓪g narrow: pill → sheet → the day's row tapped: rows ${today.listTop} → ${after.listTop}`);
      }
      const end = Date.now() + 25_000;
      let pruns = [];
      while (Date.now() < end) {
        pruns = (await readRuns(phone)).filter((r) => r.mark !== "absent");
        if (pruns.length > 0) break;
        await sleep(200);
      }
      // Give the band the time to settle on the count measured for this
      // face — the first answer may be for a pane not measured yet — and
      // then hold the phone's band to ⑥a as well.
      await sleep(1_500);
      pills("narrow", await readRuns(phone));
      pruns = (await readRuns(phone)).filter((r) => r.mark !== "absent");
      const t = pruns.find((r) => r.mark === "blocked") ?? pruns[0];
      if (!t) {
        fail("probe dead: ⑥e the narrow face drew no band to tap");
      } else {
        await phone.touchscreen.tap(t.x, t.ink?.y ?? t.top + 2);
        const got = await waitFloat(phone, "detail", true);
        if (!got?.detail) fail(`⑥e a finger tapping the ${t.mark} run opened no popover — a phone has no hover, this is its only way in`);
        else {
          saysRun("the tapped popover", got.detail, t);
          console.log(`⑥e tap: 「${got.detail.text}」`);
          // A person's second tap, not a double-tap: two taps inside the
          // double-tap window are one gesture to the browser, not two.
          await sleep(500);
          await phone.touchscreen.tap(t.x, t.ink?.y ?? t.top + 2);
          if ((await waitFloat(phone, "detail", false))?.detail) fail("⑥e tapping the same run again left the popover open");
        }
        const pgeo = await phone.evaluate(() => {
          const s = document.querySelector("[data-detail-day] [data-day-strip]")?.getBoundingClientRect();
          return s && { top: s.top, bottom: s.bottom, left: s.left, right: s.right };
        });
        if (!pgeo) fail("probe dead: ⑥f the narrow face has no band box to measure around");
        for (const dy of pgeo ? [1, 2, 3, 4] : []) {
          const above = await hitsBand(phone, pgeo.top - dy, pgeo.left, pgeo.right);
          if (above.length) fail(`⑥f narrow: ${above.length} point(s) ${dy}px above the band's box hit the band`);
          const below = await hitsBand(phone, pgeo.bottom + dy, pgeo.left, pgeo.right);
          if (below.length) fail(`⑥f narrow: ${below.length} point(s) ${dy}px under the band's box hit the band`);
        }
      }
      if (seated) {
        // ⓪g …and a tap on the scrim puts the sheet away (UX-21; a phone
        // has no Escape).
        await sleep(500);
        await phone.touchscreen.tap(195, 80);
        if (!(await sheetUp(false))) fail("⓪g a tap on the scrim did not put the sheet away");
      }
    }
    if (phoneThrew.length) fail(`⑥ the narrow page threw: ${phoneThrew.join(" | ")}`);
    await touch.close();
  }

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
  // …and a session open in a tab from last time (`useSessionTabs`), so
  // the band has somewhere to be with the list put away.
  // A row with a record: one without draws no band at all now (⓪ e).
  const remembered = fixtureRows[0];
  await returning.addInitScript((id) => {
    try {
      localStorage.setItem("khor.list-width", '{"w":462,"collapsed":true}');
      localStorage.setItem("khor.sessions.tabs", JSON.stringify({ tabs: [id], active: id }));
      // …with the column open and its day open last time (⓪ d): a
      // returning browser that had them open expects them open.
      localStorage.setItem(`khor.session-side.${id}`, JSON.stringify({ open: true, w: null, seat: "recent", day: true }));
    } catch {
      /* a profile that refuses storage is simply the blank case again */
    }
  }, remembered);
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
        const band = document.querySelector("[data-detail-day] [data-day-strip]");
        return {
          rows: document.querySelectorAll("[data-row]").length,
          strips: document.querySelectorAll("[data-day-strip]").length,
          bandW: band ? Math.round(band.getBoundingClientRect().width) : 0,
          boxW: box ? Math.round(box.getBoundingClientRect().width) : null,
        };
      });
      if (last.rows > 0 && last.strips > 0 && last.bandW > 0) return last;
      await new Promise((r) => setTimeout(r, 300));
    }
    return last;
  })();
  if (backThrew.length) fail(`④ the returning page threw: ${backThrew.join(" | ")}`);
  if (backSeen.rows === 0) {
    fail("probe dead: the returning page listed no rows — ④ would pass with nothing to switch off");
  } else if (backSeen.strips !== 1 || !(backSeen.bandW > 0)) {
    fail(
      `④ a returning browser lost the band: ${backSeen.strips} strips, band ${backSeen.bandW}px wide, ` +
        `list ${backSeen.boxW}px. Stored state must not be able to switch the feature off.`,
    );
  }
  console.log(`returning: rows=${backSeen.rows} band=${backSeen.bandW}px list=${backSeen.boxW}px (collapsed)`);
  await returning.close();

  // ── 5) an observer whose callback never arrives ───────────────────
  //
  // **The failure that actually reached a user.** A page loaded in a
  // background tab: the browser does not run the rendering steps there,
  // and a `ResizeObserver` callback is delivered *as part of* those
  // steps — so the observer attaches, is never called, and the width it
  // was the only source of stays zero. Nothing throws. The band is
  // simply not there, and it never heals, until something forces one
  // render (measured 2026-08-30: screenshotting the tab took it from 0
  // strips to 15, with nothing else touched).
  //
  // **This simulates the condition, not the cause, and that is on
  // purpose — but the cause could not be produced here and that is
  // worth writing down.** Chrome's
  // `Emulation.setPageVisibilityOverride` is gone from the protocol
  // ("wasn't found", measured), and a headless browser has no occluded
  // window to hide behind; driving two tabs left the page reporting
  // `visibilityState: "visible"` throughout, so that attempt tested
  // nothing. What is stubbed here is a **browser API**, never the app:
  // an observer that accepts `observe` and never calls back is exactly
  // what the page had, and asserting against it is stricter and
  // deterministic where the real cause is neither.
  //
  // What it therefore does not cover: anything else a background tab
  // suppresses (timers, animation frames). If a future strip comes to
  // depend on one of those, this case will not notice.
  //
  // **And it asserts the outcome, not either mechanism.** Two
  // independent things now keep the strip alive here — the width is
  // read synchronously when the observer is attached, and an unmeasured
  // width falls back to a resolution instead of to silence — and either
  // one alone is enough. So this goes red only when *both* are gone,
  // which is the right assertion for what the user loses but no help at
  // all in saying which guard was removed. Whoever sees it red should
  // read `use-day-strips` rather than trust the message to point.
  const deaf = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  await deaf.addInitScript((id) => {
    // Accepts everything, delivers nothing.
    window.ResizeObserver = class {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
    try {
      localStorage.setItem("khor.sessions.tabs", JSON.stringify({ tabs: [id], active: id }));
      localStorage.setItem(`khor.session-side.${id}`, JSON.stringify({ open: true, w: null, seat: "recent", day: true }));
    } catch {
      /* then nothing is open, and the probe below says so */
    }
  }, remembered);
  const quiet = await deaf.newPage();
  const quietThrew = [];
  quiet.on("pageerror", (e) => quietThrew.push(String(e)));
  await quiet.goto(`http://localhost:${PORT}/?bridge=${BRIDGE_PORT}`, { waitUntil: "load" });
  const quietSeen = await (async () => {
    const end = Date.now() + 25_000;
    let last = { rows: 0, strips: 0 };
    while (Date.now() < end) {
      last = await quiet.evaluate(() => ({
        rows: document.querySelectorAll("[data-row]").length,
        strips: document.querySelectorAll("[data-detail-day] [data-day-strip]").length,
      }));
      if (last.rows > 0 && last.strips > 0) return last;
      await new Promise((r) => setTimeout(r, 300));
    }
    return last;
  })();
  if (quietThrew.length) fail(`⑤ the page threw with a silent observer: ${quietThrew.join(" | ")}`);
  if (quietSeen.rows === 0) {
    fail("probe dead: no rows with a silent observer — ⑤ would pass with nothing to lose");
  } else if (quietSeen.strips !== 1) {
    fail(
      `⑤ a ResizeObserver that never calls back lost the band: ${quietSeen.strips} bands under the ` +
        "open session. The width must not depend on a delivery ever arriving.",
    );
  }
  console.log(`silent observer: rows=${quietSeen.rows} band=${quietSeen.strips}`);
  await deaf.close();
} finally {
  if (browser) await browser.close();
  for (const p of [vite, bridge]) if (p && !p.killed) p.kill("SIGTERM");
  await new Promise((r) => setTimeout(r, 800));
  for (const p of [vite, bridge]) if (p && p.exitCode === null) p.kill("SIGKILL");
  rmSync(home, { recursive: true, force: true });
  await usage.close();
  if (usage.seen.strangers !== 0) {
    fail(`${usage.seen.strangers} usage request(s) carried a token that is not the fake login — a real credential reached a gate`);
  }
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
console.log("\nday strip: painted, coloured by its own words, on one axis, and a control a pointer, a finger and a keyboard all reach");
