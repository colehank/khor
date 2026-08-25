// The file landing's tabs, against a real mesh. No mocks: two isolated
// homes on real UDP, the dev bridge is the real backend behind an HTTP
// skin, the browser is the system Chrome.
//
// What it asserts, in the order a person would do it:
//   1. two tabs on two machines at two paths do not cross: switching
//      shows the other one's listing and neither has the other's rows;
//      closing one leaves the survivor open; a reload brings them all
//      back (localStorage), pointed where they were;
//   2. the address bar's two segments — a machine completed and
//      committed to a chip, then a path completed a layer at a time,
//      and Enter really opening what was typed. A whole path pasted in
//      goes straight there. A path that does not exist comes back as a
//      sentence, **not as a blank pane**;
//   3. the fresh tab offers the places this person keeps: pins on top,
//      and pressing one points the tab at it;
//   4. narrow (390): one screen, the switcher opens, switches and
//      closes for real, and the body does not scroll sideways;
//   5. the machine column is gone, and the width it spent went to the
//      content — measured against the landing that still has one;
//   6. zero pageerror throughout, and a 600-tall window does not
//      overflow.
//
// **No Chinese literal appears below.** Words are read off the running
// app and compared with each other; the catalog owns the text, this
// owns the comparison.
//
// Run: `node scripts/check-tabs.mjs` (needs `cargo build -p khor-cli
// -p khor-gui-core --bins` first, and system Chrome).
import { execFileSync, spawn } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
// One fixed directory, wiped on the way in rather than minted per run.
// It holds a copy of two ~115MB binaries, so a name carrying the pid
// left 223MB behind every trip — nine runs of writing this file came to
// 1.8GB on a disk other people are working on, and nothing announced
// it. A fixed name cannot accumulate; two runs at once were never
// possible anyway (they would fight over the ports).
const SCRATCH = process.env.TABS_DIR ?? join(tmpdir(), "khor-tabs");
/**
 * When this run started, printed beside every file it writes.
 *
 * **The fixed directory above fixed one thing and left another.** Every
 * run overwrites the same names, so a screenshot path quoted in a report
 * still goes stale the moment somebody re-runs — it just points at the
 * right *kind* of picture now instead of the wrong one. A stamp on the
 * same line means the quote carries which run it came from, and a reader
 * can tell at a glance whether the file in front of them is that one.
 * (tabs-node's catch, after a path in a report of mine went stale
 * exactly this way.)
 */
const STAMP = new Date().toTimeString().slice(0, 8);
const shot = (name) => `${join(SCRATCH, name)} (run ${STAMP})`;
const RUN = join(SCRATCH, "run");
const KHOR = join(RUN, "khor");
const BRIDGE = join(RUN, "bridge");
// Away from 1430/1431 (the developer's own pair) and 1442/1443 (the
// smoke run's), so all three can be up at once without any of them
// measuring somebody else's node.
const BRIDGE_PORT = Number(process.env.BRIDGE_PORT ?? 1447);
const VITE_PORT = Number(process.env.VITE_PORT ?? 1446);
const A = join(SCRATCH, "alpha");
const B = join(SCRATCH, "beta");

const homeEnv = (dir, name) => {
  const e = { ...process.env, KHOR_HOME: dir, KHOR_NAME: name };
  delete e.KHOR_SESSION;
  return e;
};
const envA = homeEnv(A, "alpha");
const envB = homeEnv(B, "beta");

const children = [];
function run(cmd, args, env, name, cwd) {
  // Own process group, so cleanup kills the whole tree — killing an npx
  // wrapper alone orphans the server underneath it.
  const c = spawn(cmd, args, { env, cwd, stdio: ["ignore", "pipe", "pipe"], detached: true });
  c.stderr.on("data", (d) => process.stderr.write(`[${name}] ${d}`));
  children.push(c);
  return c;
}
function cli(env, ...args) {
  let last;
  for (let i = 0; i < 5; i++) {
    try {
      return execFileSync(KHOR, args, { env, encoding: "utf8", timeout: 30_000 });
    } catch (e) {
      last = e;
      execFileSync("/bin/sleep", ["0.4"]);
    }
  }
  throw new Error(`khor ${args.join(" ")} failed 5x: ${last?.stderr || last}`);
}
async function until(what, ms, f, step = 300) {
  const deadline = Date.now() + ms;
  let last;
  for (;;) {
    try {
      last = await f();
      if (last) return last;
    } catch (e) {
      last = e;
    }
    if (Date.now() >= deadline) break;
    await new Promise((r) => setTimeout(r, step));
  }
  throw new Error(`timed out waiting for: ${what} (last: ${last})`);
}
const fails = [];
const fail = (m) => {
  fails.push(m);
  console.error(`  FAIL ${m}`);
};
const ok = (m) => console.log(`  ok   ${m}`);

async function cleanup() {
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGKILL");
    } catch {
      /* already gone */
    }
  }
}
for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  process.on(sig, () => void cleanup().finally(() => process.exit(130)));
}

// ── 0) the shell does not know what a disk is ──────────────────────
//
// Static, and first, because it needs nothing standing up — and because
// it is the one claim the rest of this file cannot make. Every assertion
// below would pass just as well on a shell with `fetchLs` wired straight
// into it; what would be gone is the reason the browser landing can be a
// descriptor instead of a second copy (docs/DESIGN.md 浏览型外壳).
//
// **The probe is proved alive first**: the same finder is pointed at the
// instance, which must name those very things. Without that, "the shell
// mentions no disk" and "this grep is looking in the wrong place" read
// exactly the same (the `.nav-badge` lesson).
{
  const SHELL = join(gui, "src/browse");
  // What only a disk has. Not a vocabulary list — these are the four
  // calls and types that would mean the shell had learned about one.
  const DISK = /\b(fetchLs|fetchDirPins|DirListing|DirRow)\b/;
  const INSTANCE = /from "@\/views\//;
  const shellFiles = readdirSync(SHELL).filter((f) => /\.tsx?$/.test(f));
  if (shellFiles.length < 5) fail(`probe dead: only ${shellFiles.length} files in src/browse`);
  // **Counted, not just reported.** The first spelling printed its "ok"
  // line whenever the probe was alive, so a run that had just failed on
  // a real violation printed the failure and then a sentence saying the
  // shell was clean — a reassurance standing right beside the thing it
  // contradicts, which is worse than saying nothing. Caught by running
  // the gate against an injected `fetchLs`, which is also how it was
  // proved to go red at all.
  const before = fails.length;
  for (const f of shellFiles) {
    const text = readFileSync(join(SHELL, f), "utf8");
    if (DISK.test(text)) fail(`the shell knows about a disk: src/browse/${f}`);
    if (INSTANCE.test(text)) fail(`the shell imports an instance: src/browse/${f}`);
  }
  const kindFile = readFileSync(join(gui, "src/views/FilesKind.tsx"), "utf8");
  if (!DISK.test(kindFile)) fail("probe dead: the finder does not even match the files instance");
  else if (fails.length === before) {
    ok(`the shell (${shellFiles.length} files) names no disk; the instance does`);
  }
}

let browser;
try {
  rmSync(SCRATCH, { recursive: true, force: true });
  mkdirSync(RUN, { recursive: true });
  for (const [built, here] of [
    [join(repo, "target/debug/khor"), KHOR],
    [join(repo, "target/debug/bridge"), BRIDGE],
  ]) {
    if (!existsSync(built)) {
      throw new Error(`${built} is not built — cargo build -p khor-cli -p khor-gui-core --bins`);
    }
    copyFileSync(built, here);
    chmodSync(here, 0o755);
  }
  for (const d of [A, B]) mkdirSync(d, { recursive: true });

  // Two trees to browse, one on each machine, each with a directory
  // whose name exists nowhere else — so "this listing is that machine's"
  // is provable from what is on screen rather than from a count.
  const treeA = join(A, "tree-alpha");
  const treeB = join(B, "tree-beta");
  mkdirSync(join(treeA, "inner-alpha"), { recursive: true });
  mkdirSync(join(treeB, "inner-beta"), { recursive: true });
  writeFileSync(join(treeA, "inner-alpha", "only-on-alpha.txt"), "a");
  writeFileSync(join(treeB, "inner-beta", "only-on-beta.txt"), "b");

  console.log("standing up two homes…");
  run(KHOR, ["serve"], envA, "serve-a");
  await until("alpha endpoint", 20_000, () => existsSync(join(A, ".khor/endpoint.json")));
  const ticket = cli(envA, "invite").trim().split("\n").pop().trim();

  const bridge = run(BRIDGE, [], { ...envB, BRIDGE_PORT: String(BRIDGE_PORT) }, "bridge");
  let bridgeGone = null;
  bridge.on("exit", (code) => (bridgeGone = `bridge exited with ${code}`));
  let bridgeReady = false;
  bridge.stdout.on("data", (d) => (bridgeReady ||= String(d).includes(`:${BRIDGE_PORT}`)));
  await until(`our own bridge on ${BRIDGE_PORT}`, 20_000, () => {
    if (bridgeGone) throw new Error(`${bridgeGone} — is the port already taken?`);
    return bridgeReady;
  });
  await until("beta endpoint", 20_000, () => existsSync(join(B, ".khor/endpoint.json")));
  cli(envB, "pair", ticket);
  await until("alpha in beta's table", 30_000, () => cli(envB, "devices").includes("alpha"));

  const vite = run(
    "npx",
    ["vite", "--port", String(VITE_PORT), "--strictPort"],
    process.env,
    "vite",
    gui,
  );
  vite.stdout.on("data", () => {});
  await until("vite up", 60_000, async () => {
    const r = await fetch(`http://localhost:${VITE_PORT}/`).catch(() => null);
    return r?.ok;
  });

  browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    args: ["--no-proxy-server"],
  });
  const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
  const thrown = [];
  page.on("pageerror", (e) => thrown.push(String(e)));
  const url = `http://localhost:${VITE_PORT}/?bridge=${BRIDGE_PORT}`;
  await page.goto(url);

  const rail = (tab) => page.locator(`[data-rail-item][data-landing="${tab}"]`);
  const openFiles = async () => {
    await rail("files").click();
    await until("the files landing", 10_000, async () =>
      (await page.locator('[data-browse-landing="dir"]').count()) === 1,
    );
  };
  const box = page.locator("[data-omni-input]");
  /** Point the open tab at a machine and a path, through the real box. */
  const goTo = async (machine, path) => {
    if ((await page.locator("[data-omni-input]").count()) === 0) {
      // Resting: the step you are standing on is the way into the box.
      await page.locator("[data-crumb]").last().click();
    }
    if ((await page.locator(`[data-chip="dev:${machine}"]`).count()) === 0) {
      await box.click();
      await box.fill(machine);
      await until(`${machine} offered`, 10_000, async () =>
        (await page.locator(`[data-omni-item="dev:${machine}"]`).count()) === 1,
      );
      await box.press("Enter");
      await until(`${machine} as a chip`, 15_000, async () =>
        (await page.locator(`[data-chip="dev:${machine}"]`).count()) === 1,
      );
    }
    await box.fill(path);
    await box.press("Enter");
  };
  const listedNames = () =>
    page.evaluate(() =>
      [...document.querySelectorAll("[data-files-list] [data-dir], [data-files-list] [data-file]")]
        .map((e) => e.dataset.dir ?? e.dataset.file)
        .sort(),
    );

  console.log("\n5) the machine column is gone, and the width went to the content");
  await openFiles();
  if ((await page.locator("[data-list]").count()) !== 0) {
    fail("the files landing still paints a list pane");
  } else {
    ok("no list pane on the files landing");
  }
  // The probe that keeps the line above from being about a renamed
  // attribute: the same locator finds one on the landing that still has
  // one, in the same window.
  await rail("browser").click();
  await until("the browser pane", 10_000, async () => (await page.locator("[data-list]").count()) === 1);
  const listWide = await page.locator("[data-list]").evaluate((el) => el.getBoundingClientRect().width);
  const detailWide = await page
    .locator("[data-list]")
    .evaluate((el) => el.parentElement.lastElementChild.getBoundingClientRect().width);
  await openFiles();
  const filesWide = await page
    .locator('[data-browse-landing="dir"]')
    .evaluate((el) => el.getBoundingClientRect().width);
  console.log(`     list ${listWide} | browser content ${detailWide} | files content ${filesWide}`);
  if (filesWide <= detailWide) {
    fail(`the files content is ${filesWide} wide, no wider than the ${detailWide} a pane with a column gets`);
  } else {
    ok(`content gained ${Math.round(filesWide - detailWide)} (a ${Math.round(listWide)} column, less the seam)`);
  }

  console.log("\n3) a fresh tab offers the places this person keeps");
  // A pin, made through the real path: open a directory, press its pin.
  await goTo("beta", treeB);
  await until("beta's tree listed", 20_000, async () =>
    (await page.locator('[data-dir="inner-beta"]').count()) === 1,
  );
  await page.locator('[data-pin-dir="inner-beta"]').click();
  await until("the pin took", 15_000, async () =>
    (await page.locator('[data-pin-dir="inner-beta"][data-on="true"]').count()) === 1,
  );
  await page.locator("[data-tab-new]").click();
  await until("a fresh tab", 10_000, async () => (await page.locator("[data-omni-input]").count()) === 1);
  const pinnedPlace = `${treeB}/inner-beta`;
  await until("the pinned place on the fresh tab", 20_000, async () =>
    (await page.locator(`[data-place="${pinnedPlace}"]`).count()) === 1,
  );
  ok("the pin is on the fresh tab");
  const sections = await page
    .locator("[data-places-section]")
    .evaluateAll((els) => els.map((e) => e.dataset.placesSection));
  if (sections[0] !== "pinned") fail(`the fresh tab leads with ${sections[0]}, not the pins`);
  else ok(`sections in order: ${sections.join(" → ")}`);
  // **Photographed here, where the claim is true.** The two shots at the
  // end of this file are of an *open* tab, which is the right picture of
  // a strip and the wrong picture of this — and a report that pointed at
  // one of those for evidence about the fresh tab sent a reader to a
  // screenshot showing something else (tabs-node caught exactly that).
  // The scratch directory is fixed and every run overwrites it, so the
  // only durable evidence is a file this run writes for this state.
  await page.screenshot({ path: join(SCRATCH, "new-tab.png") });
  console.log(`     the fresh tab, photographed: ${shot("new-tab.png")}`);
  await page.locator(`[data-place="${pinnedPlace}"]`).click();
  await until("pressing it points the tab there", 20_000, async () =>
    (await page.locator('[data-file="only-on-beta.txt"]').count()) === 1,
  );
  ok("pressing a place opens it in that tab");

  console.log("\n1) two tabs, two machines, two paths — and no crossing");
  // Tab 1 is on beta already. A second tab, on alpha.
  await page.locator("[data-tab-new]").click();
  await until("a second tab", 10_000, async () => (await page.locator("[data-tab]").count()) === 3);
  await goTo("alpha", `${treeA}/inner-alpha`);
  await until("alpha's own file", 25_000, async () =>
    (await page.locator('[data-file="only-on-alpha.txt"]').count()) === 1,
  );
  const onAlpha = await listedNames();
  if (onAlpha.includes("only-on-beta.txt")) fail(`alpha's tab shows beta's rows: ${onAlpha}`);
  else ok(`alpha's tab: ${onAlpha.join(",")}`);
  const tabIds = await page.locator("[data-tab]").evaluateAll((els) => els.map((e) => e.dataset.tab));
  // Back to the beta tab — by pressing it, the way a person would.
  await page.locator(`[data-tab-open="${tabIds[1]}"]`).click();
  await until("beta's own file back", 20_000, async () =>
    (await page.locator('[data-file="only-on-beta.txt"]').count()) === 1,
  );
  const onBeta = await listedNames();
  if (onBeta.includes("only-on-alpha.txt")) fail(`beta's tab shows alpha's rows: ${onBeta}`);
  else ok(`beta's tab: ${onBeta.join(",")}`);
  const labels = await page
    .locator("[data-tab-label]")
    .evaluateAll((els) => els.map((e) => e.textContent));
  console.log(`     tab titles: ${JSON.stringify(labels)}`);
  if (labels[1] !== "inner-beta" || labels[2] !== "inner-alpha") {
    fail(`the tabs are titled ${JSON.stringify(labels)}, not by the directory each is in`);
  } else {
    ok("each tab is titled by the directory it is in");
  }
  // Every tab wears a face, and it is the face of the machine it is on
  // — read off the devices pane, never spelled here.
  //
  //   **The whole markup, not its length.** Length was the first
  //   spelling and it is a false green waiting to happen: two different
  //   faces are the same handful of transforms with different numbers
  //   in them, so they come out within a byte or two of each other and
  //   land on the same count often enough to be seen — it did, on the
  //   third run of this file.
  const faceOf = (sel) =>
    page.locator(sel).evaluate((el) => el.querySelector("svg")?.outerHTML ?? "");
  const alphaTabFace = await faceOf(`[data-tab-open="${tabIds[2]}"]`);
  const betaTabFace = await faceOf(`[data-tab-open="${tabIds[1]}"]`);
  if (!alphaTabFace || !betaTabFace) fail("a tab has no face drawn");
  else if (alphaTabFace === betaTabFace) fail("both tabs wear the same picture");
  else ok(`two machines, two pictures (${betaTabFace.length} vs ${alphaTabFace.length} bytes of svg)`);

  console.log("\n1b) closing one leaves the other, and a reload brings them back");
  const before = await page.locator("[data-tab]").count();
  await page.locator(`[data-tab-close="${tabIds[0]}"]`).click();
  await until("one fewer tab", 10_000, async () => (await page.locator("[data-tab]").count()) === before - 1);
  ok(`closed one: ${before} → ${before - 1}`);
  const kept = await page.locator("[data-tab]").evaluateAll((els) => els.map((e) => e.dataset.tab));
  await page.reload();
  await openFiles();
  await until("the tabs back after a reload", 15_000, async () =>
    (await page.locator("[data-tab]").count()) === kept.length,
  );
  const after = await page.locator("[data-tab]").evaluateAll((els) => els.map((e) => e.dataset.tab));
  if (after.join(",") !== kept.join(",")) fail(`after a reload the tabs are ${after}, were ${kept}`);
  else ok(`the same ${after.length} tabs came back`);
  await until("and one of them is pointed where it was", 20_000, async () =>
    (await page.locator('[data-file="only-on-beta.txt"]').count()) === 1 ||
    (await page.locator('[data-file="only-on-alpha.txt"]').count()) === 1,
  );
  ok("a restored tab is still pointed at its own directory");

  console.log("\n2) the address bar: two segments, a paste, and a refusal");
  await page.locator("[data-tab-new]").click();
  await until("a fresh tab", 10_000, async () => (await page.locator("[data-omni-input]").count()) === 1);
  await box.click();
  await box.fill("al");
  await until("a machine completed from two letters", 10_000, async () =>
    (await page.locator('[data-omni-item="dev:alpha"]').count()) === 1,
  );
  ok("typing two letters completes the machine");
  await box.press("Enter");
  await until("the chip, and the caret still in the box", 15_000, async () =>
    (await page.locator('[data-chip="dev:alpha"]').count()) === 1 &&
    (await page.locator("[data-omni-input]").count()) === 1,
  );
  ok("the machine became a chip and the box stayed open for the path");
  await box.fill(`${treeA}/`);
  await until("the next layer completed", 20_000, async () =>
    (await page.locator(`[data-omni-item="${treeA}/inner-alpha/"]`).count()) === 1,
  );
  ok("the path completes a layer at a time");
  //   …and the layer is cached: coming back to it asks nothing.
  await box.fill(`${treeA}/inner-alpha/`);
  await until("the deeper layer", 20_000, async () => {
    await page.waitForTimeout(300);
    return true;
  });
  await page.evaluate(() => performance.clearResourceTimings());
  await box.fill(`${treeA}/`);
  await page.waitForTimeout(900);
  const asked = await page.evaluate(
    () => performance.getEntriesByType("resource").filter((e) => e.name.endsWith("/ls")).length,
  );
  console.log(`     going back a layer asked the machine ${asked} time(s)`);
  //   A cached layer answers at once; it is still re-asked, so this is
  //   about the candidate list never being empty in between, not about
  //   the call count being zero.
  await until("the cached layer offered with no wait", 3_000, async () =>
    (await page.locator(`[data-omni-item="${treeA}/inner-alpha/"]`).count()) === 1,
  );
  ok("a layer already walked comes back immediately");
  //   A whole path pasted in goes straight there.
  await box.fill(`${treeA}/inner-alpha`);
  await box.press("Enter");
  await until("the pasted path really opened", 25_000, async () =>
    (await page.locator('[data-file="only-on-alpha.txt"]').count()) === 1,
  );
  ok("a whole path typed in opens on Enter");
  //   And a path that is not there says so.
  await page.locator("[data-crumb]").last().click();
  await box.fill(`${treeA}/no-such-place`);
  await box.press("Enter");
  await until("a refusal on screen", 25_000, async () =>
    (await page.locator("[data-cannot-reach]").count()) === 1,
  );
  const said = (await page.locator("[data-cannot-reach]").innerText()).trim();
  if (!said) fail("the refusal element is empty — a blank pane with an attribute on it");
  else ok(`refused out loud: ${JSON.stringify(said.slice(0, 60))}`);
  if ((await page.locator("[data-files-list] [data-file]").count()) !== 0) {
    fail("a listing is still on screen under the refusal — it looks like it arrived");
  } else {
    ok("nothing is painted that would look like having arrived");
  }

  console.log("\n4) narrow: one screen, and a switcher that really switches");
  await page.setViewportSize({ width: 390, height: 800 });
  await openFiles();
  if ((await page.locator("[data-tab-strip]").count()) !== 0) fail("the tab strip is on the narrow face");
  else ok("no tab strip at 390");
  const count = page.locator("[data-tab-count]");
  if ((await count.count()) !== 1) fail("no tab count on the narrow face");
  const narrowTabs = Number((await count.innerText()).trim());
  await count.click();
  await until("the switcher", 10_000, async () => (await page.locator("[data-tab-switcher]").count()) === 1);
  const rows = await page.locator("[data-switch-tab]").count();
  if (rows !== narrowTabs) fail(`the button counts ${narrowTabs} tabs, the switcher lists ${rows}`);
  else ok(`the count and the switcher agree: ${rows}`);
  const switchIds = await page
    .locator("[data-switch-tab]")
    .evaluateAll((els) => els.map((e) => e.dataset.switchTab));
  await page.locator(`[data-switch-open="${switchIds[0]}"]`).click();
  await until("the switcher put itself away", 10_000, async () =>
    (await page.locator("[data-tab-switcher]").count()) === 0,
  );
  ok("picking a tab closes the switcher and shows that tab");
  await count.click();
  await until("the switcher again", 10_000, async () => (await page.locator("[data-tab-switcher]").count()) === 1);
  await page.locator(`[data-switch-close="${switchIds[0]}"]`).click();
  await until("one fewer row", 10_000, async () =>
    (await page.locator("[data-switch-tab]").count()) === rows - 1,
  );
  ok(`closed one from the switcher: ${rows} → ${rows - 1}`);
  await page.locator("[data-tab-switcher] [data-tab-new]").click();
  await until("a fresh tab from the switcher", 10_000, async () =>
    (await page.locator("[data-tab-switcher]").count()) === 0 &&
    (await page.locator("[data-omni-input]").count()) === 1,
  );
  ok("the switcher opens a fresh tab and gets out of the way");
  //   The one measurement a screenshot cannot fake.
  const over390 = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  if (over390 > 0) fail(`the body scrolls ${over390} sideways at 390`);
  else ok("no sideways scroll at 390");

  console.log("\n6) a short window, and the page's own errors");
  await page.setViewportSize({ width: 1200, height: 600 });
  await openFiles();
  await page.waitForTimeout(400);
  const over600 = await page.evaluate(() => ({
    x: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    y: document.documentElement.scrollHeight - document.documentElement.clientHeight,
  }));
  if (over600.x > 0 || over600.y > 0) fail(`a 600-tall window overflows by ${JSON.stringify(over600)}`);
  else ok("a 600-tall window does not overflow");

  console.log("\n6b) coming back does not flash a lie");
  //   Leaving the landing unmounts it, so the places are asked for
  //   again on the way in — and until that answers, "nothing is pinned"
  //   is a statement about a request rather than about this person.
  //   The pin made in 3 is still real, so the sentence would be false.
  await page.setViewportSize({ width: 1200, height: 800 });
  await rail("sessions").click();
  await until("the sessions pane", 10_000, async () => (await page.locator("[data-list]").count()) === 1);
  await rail("files").click();
  await until("the files landing", 10_000, async () =>
    (await page.locator('[data-browse-landing="dir"]').count()) === 1,
  );
  const flashed = await page.locator("[data-no-places]").count();
  if (flashed !== 0) fail("the fresh tab claims nothing is pinned before anything has answered");
  else ok("no empty-state claim while the answer is still coming");
  //   …and the probe is alive: the places do turn up.
  await until("the places, once asked", 20_000, async () =>
    (await page.locator("[data-place]").count()) > 0,
  );
  ok("the places arrive a moment later");
  //   The rail says where you are. A landing whose glyph is not lit is
  //   a person who cannot tell which of four places they are in.
  const lit = await page
    .locator("[data-rail-item][data-on='true']")
    .evaluateAll((els) => els.map((e) => e.dataset.landing ?? "mark"));
  if (lit.join(",") !== "files") fail(`on the files landing the rail lights ${JSON.stringify(lit)}`);
  else ok("the rail lights files, and only files");
  //   …and the attribute really reaches the paint. `data-on` is a fact
  //   in the DOM; the block behind the glyph is what a person sees, and
  //   a stylesheet that stopped applying would leave the first true and
  //   the second false. Pointer parked away first — hover paints the
  //   same kind of block, and it would answer this question for the
  //   wrong reason.
  await page.mouse.move(2, 400);
  await page.waitForTimeout(300);
  const blocks = await page
    .locator("[data-rail-item][data-landing]")
    .evaluateAll((els) =>
      els.map((e) => [e.dataset.landing, getComputedStyle(e).backgroundColor]),
    );
  console.log(`     rail backgrounds: ${JSON.stringify(blocks)}`);
  const painted = blocks.filter(([, bg]) => !/rgba\(0, 0, 0, 0\)|transparent/.test(bg));
  if (painted.length !== 1 || painted[0][0] !== "files") {
    fail(`the rail paints a block on ${JSON.stringify(painted)}, with files the open landing`);
  } else {
    ok(`one block, on files: ${painted[0][1]}`);
  }

  console.log("\n7) the other landings are where they were");
  for (const tab of ["sessions", "devices"]) {
    await rail(tab).click();
    await until(`the ${tab} pane`, 10_000, async () =>
      (await page.locator("[data-list]").getAttribute("aria-label")) ===
      (await rail(tab).getAttribute("aria-label")),
    );
    const bar = await page.locator("[data-pane-bar]").count();
    if (bar !== 1) fail(`the ${tab} landing lost its pane bar`);
    else ok(`${tab}: list pane + pane bar, unchanged`);
    await page.screenshot({ path: join(SCRATCH, `${tab}.png`) });
  }
  //   …and a machine row still opens that machine's card. **Not
  //   belt-and-braces**: the files landing used to be a third consumer
  //   of `DevicesList.onOpen`, and taking it away rewrote that prop's
  //   branch — so "the card still opens" is exactly the thing that edit
  //   could have broken, and a pane bar being present says nothing
  //   about it. (`smoke.mjs` §23 asserts the same; this is the cheap
  //   copy that can be run on its own.)
  await rail("devices").click();
  await until("machines on the devices pane", 10_000, async () =>
    (await page.locator('[data-device="alpha"] [data-row-open]').count()) === 1,
  );
  await page.locator('[data-device="alpha"] [data-row-open]').click();
  //   `[data-machine-card]`, not `[data-machine-id]`: the id stepped
  //   behind a copy button in 2ef4750 and the attribute went with it.
  //   Written down because this probe was first spelled the old way and
  //   went red — reading, for a few minutes, exactly like "the tabs work
  //   broke the machine card". An empty match says "I did not find it",
  //   never "it is not there".
  try {
    await until("alpha's card", 10_000, async () =>
      (await page.locator("[data-machine-card]").count()) === 1,
    );
    ok("a machine row still opens that machine's card");
  } catch {
    fail("a machine row no longer opens its card");
  }
  await openFiles();
  // With something actually listed under it: a strip photographed over
  // an empty tab cannot show whether the open tab joins its own page.
  await goTo("beta", treeB);
  await until("a listing under the strip", 20_000, async () =>
    (await page.locator('[data-dir="inner-beta"]').count()) === 1,
  );
  await page.mouse.move(600, 400);
  await page.waitForTimeout(300);
  await page.screenshot({ path: join(SCRATCH, "files-wide.png") });
  await page.setViewportSize({ width: 390, height: 800 });
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(SCRATCH, "files-narrow.png") });
  console.log(`     screenshots in ${SCRATCH} (run ${STAMP}): ${["sessions", "devices", "files-wide", "files-narrow"].join(".png, ")}.png`);

  if (thrown.length) fail(`page errors: ${thrown.join(" | ")}`);
  else ok("zero pageerror throughout");
} finally {
  await browser?.close();
  await cleanup();
}

if (fails.length) {
  console.error(`\ntabs gate: ${fails.length} failure(s)\n- ${fails.join("\n- ")}`);
  process.exit(1);
}
console.log("\ntabs gate: the strip, the address bar and the switcher all do what they say");
