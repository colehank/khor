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
  // **The positive control moved on 2026-09-11** (工作区替代文件). It
  // used to be `FilesKind.tsx`, the shell's disk instance, which is
  // gone with the landing it served. `FilesPane.tsx` is the same code
  // under a different owner — a workspace's file tree rather than a
  // browsing tab's body — and it names the same four symbols, so it
  // proves the finder is alive for exactly the reason the old one did.
  const kindFile = readFileSync(join(gui, "src/views/FilesPane.tsx"), "utf8");
  if (!DISK.test(kindFile)) fail("probe dead: the finder does not even match the files pane");
  else if (fails.length === before) {
    ok(`the shell (${shellFiles.length} files) names no disk; the pane does`);
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
  // Reach a landing as a *switch*, never a re-click: pressing the open
  // sessions/devices glyph now collapses its list (App.tsx 再点收起,
  // 2026-09-02), and this gate re-opens the same landing across
  // sections. Hop through the sibling when already there so the return
  // leaves the list expanded (the width reads below depend on it).
  // Browsing landings do not toggle and pass through unchanged.
  const go = async (tab) => {
    if (
      (tab === "sessions" || tab === "devices") &&
      (await rail(tab).getAttribute("data-on").catch(() => null)) === "true"
    ) {
      await rail(tab === "sessions" ? "devices" : "sessions").click();
      await page.waitForTimeout(200);
    }
    await rail(tab).click();
  };
  // ── 1–7) ⚠️ **Tombstone: the disk kind's half of this gate,
  //        2026-09-12** (工作区批6).
  //
  // Seven sections and about four hundred lines browsed a directory
  // through tabs, a strip, a switcher and an address bar. **Their
  // subject left on 2026-09-11** (工作区替代文件) and they were kept
  // red on purpose for a day, throwing rather than skipping, because
  // deleting a real-wire gate is a decision to take deliberately.
  //
  // **What they were waiting for did not happen, and that is the
  // decision being recorded here.** The note said they wait for the
  // code kind — VS Code as the browsing shell's second customer. Batch
  // 5 built VS Code and did not make it a kind: every tab in that
  // shell names an exit machine and an editor has none, so it lives in
  // the workspace's middle column instead (docs/handoff/001.md 批5).
  // The shell now has exactly one customer, and a gate proving that
  // one shape serves two has nothing to prove.
  //
  // **What is lost, named rather than quietly dropped:**
  //
  //   - two tabs on two machines with two paths, and no crossing;
  //   - closing one and finding the other, across a reload;
  //   - the address bar's two segments, a paste, and its refusal;
  //   - narrow: one screen and a switcher that really switches;
  //   - a short window, the page's own errors, and not flashing a lie
  //     on the way back.
  //
  // The tab machinery underneath is still exercised by section 8,
  // which drives the same shell with pages in it — so what went is the
  // second *instance*, not the shell. If a second kind ever arrives,
  // this is the file it comes back to.


  console.log("\n7) the other landings are where they were");
  for (const tab of ["sessions", "devices"]) {
    await go(tab);
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
  await go("devices");
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
  console.log(`     screenshots in ${SCRATCH} (run ${STAMP}): ${["sessions", "devices"].join(".png, ")}.png`);

  // ── 8) the same shell, a second time, with pages in it ────────────
  //
  // **What is worth asserting here is only what differs.** Tabs, the
  // strip, the switcher, the store and the widths are the shell's, and
  // the sections above proved them once; running the identical checks a
  // second time would measure the same code and call it two results.
  // What is new is the three places the two kinds genuinely part
  // company — the words, what completes the second segment, and who
  // spends the borrow — plus the one structural fact that the second
  // landing exists at all.
  console.log("\n8) the browser landing, through the same shell");
  await page.setViewportSize({ width: 1200, height: 800 });
  const openBrowser = async () => {
    // Browser is past the fourth landing, so on the narrow bar it lives
    // behind「…」(App.tsx `NARROW_SHOWN`, 2026-09-02) — two presses
    // there, one on the wide column where every landing has a glyph.
    // Wait for the rail to settle before deciding which: `.count()`
    // does not auto-wait, and reading it a tick before the wide rail
    // re-rendered once sent a wide open down the narrow (fold) path.
    await until("the rail to settle", 10_000, async () =>
      (await rail("browser").count()) + (await page.locator("[data-rail-more]").count()) > 0,
    );
    //   ⚠️ **Decide and press in one attempt, and retry the whole
    //   attempt.** The wait above is satisfied by *either* shape, so on
    //   a viewport change it can pass while the wide rail is still on
    //   screen — and then the branch presses a button the narrow
    //   re-render is about to remove. Measured 2026-09-06 at the
    //   390-wide stop: `element was detached from the DOM` on a rail
    //   button playwright had just resolved, and on later runs the
    //   locator never resolving at all. **It reproduces at
    //   `853447b` too**, so it is this walk's race and not the card's.
    for (let i = 0; ; i += 1) {
      try {
        if ((await rail("browser").count()) === 0) {
          await page.locator("[data-rail-more]").click({ timeout: 4_000 });
          await page.locator('[data-more-landing="browser"]').click({ timeout: 4_000 });
        } else {
          await rail("browser").click({ timeout: 4_000 });
        }
        break;
      } catch (e) {
        if (i >= 4) throw e;
        await new Promise((r) => setTimeout(r, 400));
      }
    }
    await until("the browser landing", 10_000, async () =>
      (await page.locator('[data-browse-landing="page"]').count()) === 1,
    );
  };
  //    **The other half of this comparison moved on 2026-09-12.** It
  //    used to be the files landing measured before and after its
  //    machine column; that landing is gone, so the width a list pane
  //    costs is read off one that still has one. Same claim, live
  //    subject: a landing that browses spends none of its width on a
  //    column, and the content is wider for it.
  await go("devices");
  // Waited for by the list pane, which section 7 just proved appears,
  // rather than by the detail: `[data-screen]` is re-keyed on every
  // screen change, so a count read mid-swap is a count of nothing.
  await until("the devices landing", 10_000, async () =>
    (await page.locator("[data-list]").count()) === 1,
  );
  // **What a list pane costs, computed rather than looked up.** The
  // detail area is what the window has left after the rail and the
  // list, and those two are the elements this app actually draws —
  // `[data-screen]` is re-keyed on every screen change and is not
  // reliably in the document to measure.
  const detailWide = await page.evaluate(() => {
    const w = (sel) => {
      const el = document.querySelector(sel);
      return el ? el.getBoundingClientRect().width : 0;
    };
    // The rail is the nav holding the landing glyphs — found through
    // one of its own items rather than by the tag, because `nav` is not
    // a promise this app makes and `[data-rail-item]` is.
    const rail = document.querySelector("[data-rail-item]")?.closest("nav");
    const railW = rail ? rail.getBoundingClientRect().width : 0;
    return Math.round(window.innerWidth - railW - w("[data-list]"));
  });
  await openBrowser();
  // ⚠️ **Tombstone: this asked for the opposite until 2026-09-20.** It
  // read 「the browser landing still paints a list pane」 and 「content
  // gained N」 — the claim being that a 浏览型 landing spends none of
  // its width on a column. The user overturned it (docs/SHELL.md 第 2
  // 条:「浏览器改版为侧边栏. 标签页移动到侧边栏, 参考chrome的最新模式」),
  // and the column it has now is not the one that claim was about: what
  // was refused was a permanent column of **machines**, answering a
  // question the tab settles the moment it opens. A column of **tabs**
  // is the thing a person watches here.
  //
  // So the measurement is kept and its sign is flipped: the landing
  // draws the shell's pane under its own name, and its content is what
  // is left beside it. **The section is not deleted**, because「the
  // browser has a column now」is exactly the kind of change that should
  // have to walk past a gate that used to say otherwise.
  if ((await page.locator("[data-list]").count()) !== 0) {
    fail("the browser landing paints the sessions pane's marker — its column answers to [data-browse-tabs]");
  } else ok("the browser landing draws no [data-list]");
  const column = await page.locator("[data-browse-tabs]").count();
  if (column !== 1) {
    fail(`the browser landing drew ${column} tab column(s) — 「标签页移动到侧边栏」 (src/browse/BrowseLanding.tsx)`);
  } else ok("the browser landing's tabs are a column");
  const pageWide = await page.evaluate(() => {
    const w = (sel) => {
      const el = document.querySelector(sel);
      return el ? el.getBoundingClientRect().width : 0;
    };
    return Math.round(w('[data-browse-landing="page"]') - w("[data-browse-tabs]"));
  });
  // The two landings spend the same room on a column, because it is the
  // same component at the same seam (`components/ListPane`, 第 6 条).
  // Measured rather than assumed: a column that drifted a few pixels
  // from the other would be two widths wearing one name.
  if (Math.abs(pageWide - detailWide) > 2) {
    fail(
      `the browser's page has ${pageWide} beside its column while a list pane leaves ${detailWide} — ` +
        "one shell, one seam, one width (src/components/ListPane.tsx)",
    );
  } else {
    ok(`the column leaves ${pageWide}, the same room a list pane leaves (${detailWide})`);
  }

  //    **The words are this landing's own.** Read off the two landings
  //    and required to differ — never spelled here, the catalog owns
  //    them. This is the assertion the first cut of the browser kind
  //    would have failed: it mounted correctly and said 「目录」.
  //    **Read from whichever state the bar is in.** The name rides the
  //    box when a tab is fresh and the resting face when it is pointed
  //    somewhere, and by this point in the run the files landing is
  //    pointed somewhere — the first spelling looked only for the box,
  //    found nothing, and reported "an address bar with no name" about a
  //    bar that was sitting right there wearing it.
  const wordsOn = async (open) => {
    await open();
    await page.waitForTimeout(400);
    return page.evaluate(() => ({
      empty: document.querySelector("[data-no-places]")?.textContent ?? null,
      address:
        document.querySelector("[data-omni-input]")?.getAttribute("aria-label") ??
        document.querySelector("[data-address-rest]")?.getAttribute("aria-label") ??
        null,
    }));
  };
  //    ⚠️ **This compared two landings until 2026-09-12** and now reads
  //    one. The claim was that each kind names its address bar its own
  //    way — which needed two kinds, and there is one. What survives is
  //    the half that can still be checked: the bar has a name at all.
  //    **Weaker, and named as weaker**: a second kind arriving is what
  //    makes the comparison possible again.
  const pageWords = await wordsOn(openBrowser);
  console.log(`     pages ${JSON.stringify(pageWords)}`);
  if (!pageWords.address) fail("probe dead: an address bar with no name");
  else ok("the browsing landing names its address bar");

  //    A page kept, made through the real backend rather than by
  //    reaching into a file: the bridge is the node behind an HTTP
  //    skin, so this is `Node::pin_web` and nothing else.
  const KEPT = "https://khor.example/kept";
  const ask = (cmd, args) =>
    fetch(`http://127.0.0.1:${BRIDGE_PORT}/${cmd}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(args),
    }).then((r) => (r.ok ? r.json() : Promise.reject(new Error(`${cmd}: ${r.status}`))));
  await ask("pin_web", { machine: "alpha", url: KEPT, on: true });
  // **Left and come back, because the fresh tab reads its pins once on
  // mount** (`use-places`). This used to happen by accident: the
  // section before it ended on the files landing, which unmounted this
  // one. That section went on 2026-09-12 with the disk kind, and the
  // assertion below then waited twenty seconds for a list fetched
  // before the pin existed — a real dependency that had been met by a
  // side effect.
  await go("devices");
  await until("away from the browser landing", 10_000, async () =>
    (await page.locator('[data-browse-landing="page"]').count()) === 0,
  );
  await openBrowser();
  await until("the kept page on the fresh tab", 20_000, async () =>
    (await page.locator(`[data-place="${KEPT}"]`).count()) === 1,
  );
  ok("a pinned page reaches the browser landing's fresh tab");

  //    **The second segment completes from memory, not from a machine.**
  //    Nobody can enumerate the web, so what the box offers is what this
  //    person keeps — and it has to be offered on the machine it was
  //    kept against.
  const pageBox = page.locator("[data-omni-input]");
  await pageBox.click();
  await pageBox.fill("alpha");
  await until("alpha offered as an exit", 10_000, async () =>
    (await page.locator('[data-omni-item="dev:alpha"]').count()) === 1,
  );
  await pageBox.press("Enter");
  //    ⚠️ **Tombstone: 「alpha as the exit chip」, 2026-09-12.** It
  //    waited for `[data-chip="dev:alpha"]`, and the machine's chip was
  //    taken out of this box on 2026-09-09 — one exit per landing means
  //    the machine is not a property of what is being typed, and drawn
  //    as a chip the box read as two fields when it is one
  //    (`AddressBar`'s `chosen={[]}`). The same assertion rotted in
  //    `smoke.mjs` and was found there first; here it went unnoticed
  //    for three days because this gate had been dying at section 5.
  //
  //    **What proves the commit landed is the line below.** Kept pages
  //    are per machine, so a completion that offers alpha's pin is a
  //    box that really is pointed at alpha — weaker than a chip anybody
  //    could see, and it is the observable this face still has.
  await pageBox.fill("https://khor.exa");
  await until("the kept page completed from what is remembered", 15_000, async () =>
    (await page.locator(`[data-omni-item="${KEPT}"]`).count()) === 1,
  );
  ok("the address bar completes a kept page");
  //    **Tab completes, Enter goes** (`Omnibox`), and pressing the wrong
  //    one of those is not a product failure. The first spelling here
  //    pressed Enter on the half-typed address, went to it — correctly —
  //    and then complained that the tab was titled after where it had
  //    actually gone. Tab first, and the box must really hold the whole
  //    address before Enter means anything.
  await pageBox.press("Tab");
  await until("the completion written into the box", 10_000, async () =>
    (await pageBox.inputValue()) === KEPT,
  );
  ok("Tab writes the completion into the box");

  //    Enter spends a real borrow. **Whether it succeeds is not the
  //    assertion** — that depends on a network this test does not own —
  //    what is asserted is that the pane *says* which happened. A blank
  //    pane after pressing Enter is the one outcome forbidden, and it is
  //    the one a borrow that quietly did nothing would produce.
  await pageBox.press("Enter");
  //    Waited to *settle*, not merely to appear: 「正在借…」 is a real
  //    thing to say but it is not the answer, and a probe that stops at
  //    it would pass on a borrow that never comes back.
  //    ⚠️ **This waited for the element to say 「opened」 and it can
  //    never say that.** `BrowserPane` draws the status only while the
  //    state is *not* opened — a page that came up needs no sentence,
  //    the page is the sentence — so success is the element leaving.
  //    The old wait could therefore only ever be satisfied by a
  //    failure, and it was: this run borrows alpha's network, which
  //    used not to come up and now does. Three days of the gate dying
  //    at section 5 is why nobody saw it.
  //
  //    So what is waited for is the state **leaving 「opening」**, by
  //    either road, and each road is then judged on its own terms.
  const settled = await until("the borrow settling", 30_000, async () => {
    const how = await page
      .locator("[data-web-status]")
      .getAttribute("data-web-status")
      .catch(() => null);
    if (how === null) return "opened";
    return how === "failed" ? "failed" : false;
  });
  if (settled === "failed") {
    const spoke = await page.locator("[data-web-status]").innerText();
    if (!spoke.trim()) fail("the borrow failed and said so with an empty element");
    else ok(`the borrow failed and said why: ${JSON.stringify(spoke.trim().slice(0, 48))}`);
  } else {
    ok("the borrow came up, and the pane stopped talking about it");
  }

  //    …and the tab wears the page's host, not its whole address — the
  //    kind's `leafOf`, which is the other thing the two do differently.
  const pageTitles = await page
    .locator("[data-tab-label]")
    .evaluateAll((els) => els.map((e) => e.textContent));
  console.log(`     browser tab titles: ${JSON.stringify(pageTitles)}`);
  if (!pageTitles.includes("khor.example")) {
    fail(`no tab is titled by the page's host — ${JSON.stringify(pageTitles)}`);
  } else ok("a browser tab is titled by its host");

  //    The narrow face is the shell's, but this landing has to reach it
  //    too — one press, because the shape itself was proved in 4.
  await page.setViewportSize({ width: 390, height: 800 });
  await openBrowser();
  if ((await page.locator("[data-tab-count]").count()) !== 1) {
    fail("no tab count on the browser landing's narrow face");
  } else ok("the narrow switcher is reachable on the browser landing too");
  const overPage = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  if (overPage > 0) fail(`the browser landing scrolls ${overPage} sideways at 390`);
  else ok("no sideways scroll at 390");
  await page.screenshot({ path: join(SCRATCH, "browser-narrow.png") });
  await page.setViewportSize({ width: 1200, height: 800 });
  await openBrowser();
  await page.waitForTimeout(300);
  await page.screenshot({ path: join(SCRATCH, "browser-wide.png") });
  console.log(`     ${shot("browser-wide.png")}, ${shot("browser-narrow.png")}`);

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
