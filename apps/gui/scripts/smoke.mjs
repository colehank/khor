// Real-connection acceptance for the GUI. No mocks anywhere: three homes
// on real UDP, the dev bridge is the real backend behind an HTTP skin,
// the browser is the system Chrome. What is asserted:
//   1. joining the mesh from the GUI: beta starts alone, takes alpha's
//      ticket through the devices pane, and alpha arrives wearing a face
//      — the control being that alpha is provably absent beforehand;
//   2. no pane wears a title: the name is readable off the region's
//      aria-label (which proves the probe found the pane) and appears
//      nowhere in what the pane paints;
//   3. the GUI issues a real ticket: a third machine joins with it from
//      the CLI and shows up in the list;
//   4. search filters the devices pane for real, both ways, says so when
//      nothing matched, and clearing brings everything back;
//   5. leaving a line reaches the other machine: beta tells alpha from
//      the GUI, alpha reads it back with `khor log`;
//   6. a session living on alpha shows up in beta's GUI, with source,
//      and the word in the GUI equals the word the CLI prints;
//   7. the same search on the sessions pane;
//   8. the word filter filters by the key the backend sent, each word
//      keeping its own rows and nothing else;
//   9. clicking the row is the seen semantics: the unread badge clears
//      AND alpha's own list turns idle — the loop closes cross-device;
//  10. faces: rows paint a real derived SVG (not a placeholder), one
//      machine is the same picture in two places on one screen, two
//      machines are two pictures, and flipping the theme moves nothing
//      inside the SVG;
//  11. the rail's name shows on hover with no delay worth measuring, on
//      keyboard focus too, and goes away when the pointer leaves;
//  12. all four landings open, and the two that have no content of
//      their own (files, browser) list the same machines the devices
//      pane does — the same set, not merely a list of the same length;
//  13. each pane's bar carries exactly what that pane can do, and the
//      three machine panes name their search box the same thing (a
//      files pane offering to search files would be naming something
//      it cannot find);
//  14. the app's mark is at the head of the rail, its artwork actually
//      loaded, and it is not a control: no button around it and no
//      hover response — with a rail glyph measured the same way as the
//      control, since "nothing changed" is also what a broken probe
//      says;
//  15. the back button exists only on the narrow face (after proving
//      the detail header renders at all — negative assertions must
//      first prove the probe is alive), and the mark is not in the
//      narrow rail;
//  16. zero pageerror throughout.
// Every wait has a deadline; cleanup runs in finally and kills by pid.
//
// No Chinese literal appears below. Words are read off the running app
// (a rail item's aria-label, a row's data-word) and compared against
// what the other face prints — the catalog owns the text, this script
// owns the comparison.
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, rmSync, existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
const KHOR = join(repo, "target/debug/khor");
const BRIDGE = join(repo, "target/debug/bridge");
const SCRATCH = process.env.SMOKE_DIR ?? join(repo, "target/gui-smoke");
const A = join(SCRATCH, "alpha");
const B = join(SCRATCH, "beta");
const G = join(SCRATCH, "gamma");
// Overridable because this machine also carries the developer's own app:
// a bridge already sitting on the default port belongs to somebody else
// and points at somebody else's home. Borrowing vite is fine (same source
// tree); borrowing a *backend* would mean the whole run measured the
// wrong node while reporting green, so the bridge below is checked to be
// ours before anything is asserted.
// 1442/1443, not 1430/1431: those two are the developer's own pair on
// this machine (`vite.config.ts` pins the dev server to 1430, and the
// preview bridge sits on 1431). Defaulting here to the pair next door
// means a smoke run and a session of hand-driving the app can be up at
// the same time without either noticing the other.
const BRIDGE_PORT = Number(process.env.BRIDGE_PORT ?? 1443);
const VITE_PORT = Number(process.env.VITE_PORT ?? 1442);
// The line beta leaves in alpha's window. ASCII on purpose: the message
// is test data, not the thing under test (docs/handoff 中文 rule).
const NEEDLE = "khor-smoke-hello";

const homeEnv = (dir, name) => {
  const e = { ...process.env, KHOR_HOME: dir, KHOR_NAME: name };
  delete e.KHOR_SESSION;
  return e;
};
const envA = homeEnv(A, "alpha");
const envB = homeEnv(B, "beta");
const envG = homeEnv(G, "gamma");

const children = [];
function run(cmd, args, env, name) {
  // detached = own process group, so cleanup can kill the whole tree —
  // killing an npx wrapper alone orphans the real server underneath.
  const c = spawn(cmd, args, { env, stdio: ["ignore", "pipe", "pipe"], detached: true });
  c.stderr.on("data", (d) => process.stderr.write(`[${name}] ${d}`));
  children.push(c);
  return c;
}
function cli(env, ...args) {
  return execFileSync(KHOR, args, { env, encoding: "utf8", timeout: 30_000 });
}
function feedHook(env, event, extra = "") {
  const payload = `{"session_id":"cafe1","cwd":"/tmp/proj","hook_event_name":"${event}"${extra}}`;
  execFileSync(KHOR, ["state", "--hook"], { env, input: payload, timeout: 15_000 });
}
async function until(what, ms, f, step = 400) {
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

let browser;
try {
  rmSync(SCRATCH, { recursive: true, force: true });
  for (const d of [A, B, G]) mkdirSync(d, { recursive: true });

  // alpha: serve + a hooked agent session (register → busy), and a
  // ticket from its CLI — the one thing the GUI is not being asked to
  // do in this scenario, so that what it *is* asked to do (joining) is
  // the only untested step in the pairing.
  run(KHOR, ["serve"], envA, "serve-a");
  await until("alpha endpoint.json", 15_000, () => existsSync(join(A, ".khor/endpoint.json")));
  const ticketA = cli(envA, "invite").trim().split("\n").pop().trim();
  feedHook(envA, "SessionStart");
  feedHook(envA, "UserPromptSubmit");
  if (!cli(envA, "sessions").includes("tui/cafe1")) throw new Error("alpha row missing");

  // beta: the bridge is the app backend — embedded serve pumps sync and
  // holds this key's endpoint, which is what lets the GUI mint and
  // accept tickets without a terminal.
  const bridge = run(BRIDGE, [], { ...envB, BRIDGE_PORT: String(BRIDGE_PORT) }, "bridge");
  let bridgeGone = null;
  bridge.on("exit", (code) => (bridgeGone = `bridge exited with ${code}`));
  // It says so on stdout once it has the port. If it died instead — the
  // usual reason being a bridge already bound there — say that here,
  // where it is one line, instead of discovering it as a wrong answer
  // twenty assertions later.
  let bridgeReady = false;
  bridge.stdout.on("data", (d) => (bridgeReady ||= String(d).includes(`:${BRIDGE_PORT}`)));
  await until(`our own bridge on ${BRIDGE_PORT}`, 15_000, () => {
    if (bridgeGone) throw new Error(`${bridgeGone} — is the port already taken?`);
    return bridgeReady;
  });
  await until("beta endpoint.json", 15_000, () => existsSync(join(B, ".khor/endpoint.json")));
  const vite = run("npx", ["vite", "--port", String(VITE_PORT), "--strictPort"], { ...process.env, PATH: process.env.PATH }, "vite");
  vite.stdout.on("data", () => {});
  await until("vite up", 30_000, async () => {
    const r = await fetch(`http://localhost:${VITE_PORT}/`).catch(() => null);
    return r?.ok;
  });

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const page = await browser.newPage({ viewport: { width: 1080, height: 720 } });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(String(e)));
  await page.goto(`http://localhost:${VITE_PORT}/?bridge=${BRIDGE_PORT}`);

  const listPane = page.locator("[data-list]");
  const railItem = (tab) => page.locator(`[data-rail-item][data-landing="${tab}"]`);
  const openLanding = async (tab) => {
    await railItem(tab).click();
    await until(`the ${tab} pane`, 10_000, async () =>
      (await listPane.getAttribute("aria-label")) === (await railItem(tab).getAttribute("aria-label")),
    );
  };

  // 1) joining from the GUI. The control first: alpha is not in beta's
  //    table, so whatever makes it appear below can only be the join.
  await openLanding("devices");
  await until("beta's own row", 10_000, async () => (await page.locator("[data-device]").count()) === 1);
  if ((await page.locator('[data-device="alpha"]').count()) !== 0) {
    throw new Error("alpha is in the table before pairing — the control is void");
  }
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="join"]').click();
  await page.locator("[data-ticket-input]").fill(ticketA);
  await page.locator("[data-join]").click();
  await until("alpha's row, wearing its own face", 30_000, async () => {
    const err = await page.locator("[data-dialog-error]").count();
    if (err) throw new Error(await page.locator("[data-dialog-error]").innerText());
    return (await page.locator('[data-device="alpha"] [data-face] svg').count()) === 1;
  });

  // 2) no pane wears a title. The probe is proven alive first: the name
  //    has to be readable, and it has to be the same fact the rail
  //    states — then the claim is that it is nowhere in the paint.
  //    (A placeholder is an attribute, not text, so the search box
  //    naming its pane does not put a title back on the screen.)
  const assertUntitled = async (tab) => {
    await openLanding(tab);
    const name = await listPane.getAttribute("aria-label");
    if (!name) throw new Error(`probe dead: the ${tab} pane has no accessible name`);
    if ((await page.locator("[data-pane-bar]").count()) !== 1) {
      throw new Error(`probe dead: no pane bar on ${tab}`);
    }
    if ((await listPane.innerText()).includes(name)) {
      throw new Error(`the ${tab} pane paints its own name: ${name}`);
    }
  };
  await assertUntitled("devices");

  // 3) the GUI mints a real ticket — proven by a third machine joining
  //    with it from the CLI, not by the string looking ticket-shaped.
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="invite"]').click();
  const guiTicket = await until("a ticket from the GUI", 20_000, async () => {
    const err = await page.locator("[data-dialog-error]").count();
    if (err) throw new Error(await page.locator("[data-dialog-error]").innerText());
    const t = await page.locator("[data-ticket]").inputValue();
    return t.length > 0 && t;
  });
  await page.keyboard.press("Escape");
  cli(envG, "pair", guiTicket);
  await until("gamma in beta's device list", 30_000, async () =>
    (await page.locator('[data-device="gamma"]').count()) === 1,
  );

  // 4) search filters for real, and both ways: a box that always kept
  //    the first row would pass one direction and fail the other.
  const devices = page.locator("[data-device]");
  const search = page.locator("[data-pane-search]");
  const before = await devices.count();
  if (before !== 3) throw new Error(`expected alpha, beta and gamma; saw ${before}`);
  await search.fill("gamma");
  await until("only gamma", 5_000, async () => (await devices.count()) === 1);
  if ((await page.locator('[data-device="gamma"]').count()) !== 1) {
    throw new Error("search kept the wrong row");
  }
  await search.fill("alpha");
  await until("only alpha", 5_000, async () => (await devices.count()) === 1);
  if ((await page.locator('[data-device="alpha"]').count()) !== 1) {
    throw new Error("search kept the wrong row");
  }
  await search.fill("no-machine-is-called-this");
  await until("an empty result that says so", 5_000, async () => (await devices.count()) === 0);
  if ((await page.locator("[data-empty]").count()) !== 1) {
    throw new Error("filtered to nothing without saying nothing matched");
  }
  await search.fill("");
  await until("every machine back", 5_000, async () => (await devices.count()) === before);

  // 5) leaving a line: from beta's GUI into alpha's window, read back by
  //    alpha's own CLI. Two machines, two faces, one message.
  await assertUntitled("sessions");
  await page.locator("[data-pane-new]").click();
  await page.locator('[data-new-item="tell"]').click();
  await page.locator('[data-machine="alpha"]').click();
  await page.locator("[data-tell-text]").fill(NEEDLE);
  await page.locator("[data-tell-send]").click();
  await until("the tell dialog to close on success", 20_000, async () => {
    const err = await page.locator("[data-dialog-error]").count();
    if (err) throw new Error(await page.locator("[data-dialog-error]").innerText());
    return (await page.locator("[data-tell-dialog]").count()) === 0;
  });
  // Polled at a walking pace: each turn is a whole CLI process opening
  // the node, and the sync pump is on a five-second beat anyway.
  await until(
    "alpha to read the line in its own window",
    40_000,
    () => cli(envA, "log", "alpha").includes(NEEDLE),
    1_000,
  );

  // 6) the reported row reaches beta's GUI, busy, with its source; and
  //    the CLI line for the same row carries the same display word.
  const row = page.locator("[data-word]", { hasText: "proj" });
  await until("the alpha row in beta's GUI", 30_000, async () => (await row.count()) === 1);
  if ((await row.getAttribute("data-word")) !== "busy") throw new Error("row should be busy");
  const guiWord = (await row.locator("[data-word-text]").innerText()).trim();
  if (!(await row.innerText()).includes("alpha")) throw new Error("reported row must show its source");
  const cliLine = cli(envB, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
  if (!cliLine || !cliLine.includes(guiWord)) {
    throw new Error(`CLI and GUI disagree on the word: ${guiWord} vs ${cliLine}`);
  }

  // 7) searching the session pane, same shape as the devices one: a term
  //    only one row answers to, then the control that clearing brings the
  //    rest back.
  const sessionRows = page.locator("[data-word]");
  const sessionSearch = page.locator("[data-pane-search]");
  const allRows = await sessionRows.count();
  if (allRows < 2) throw new Error(`need more than one row to tell a filter from a no-op; saw ${allRows}`);
  await sessionSearch.fill("proj");
  await until("only the row titled proj", 5_000, async () => (await sessionRows.count()) === 1);
  if ((await sessionRows.getAttribute("data-word")) !== "busy") {
    throw new Error("session search kept the wrong row");
  }
  await sessionSearch.fill("");
  await until("every session back", 5_000, async () => (await sessionRows.count()) === allRows);

  // 8) the word filter. The keys come off the rows themselves — the ones
  //    the node sent — so this never spells a state out. Reading all of
  //    them, not a sample: the rows are ordered local-first, and a sample
  //    off the top says more about that ordering than about the words.
  const wordsOnScreen = [
    ...new Set(await sessionRows.evaluateAll((els) => els.map((e) => e.dataset.word))),
  ];
  if (wordsOnScreen.length < 2) {
    throw new Error(`every row wears ${wordsOnScreen[0]} — the filter cannot be told from a no-op`);
  }
  // Opening is conditional and closing is waited for, so that a menu
  // which failed to open — or failed to close, leaving the next click to
  // toggle it shut — reports itself instead of surfacing as a missing
  // option several lines later.
  const menu = page.locator("[data-slot=dropdown-menu-content]");

  // Before using the menu: its opening motion is real and is on our
  // tokens. Measured off the element, because a missing @import leaves
  // the classes sitting in the markup with nothing behind them — the
  // markup would read exactly the same either way, and no other
  // assertion here would notice.
  const ms = (v) => (v.trim().endsWith("ms") ? parseFloat(v) : parseFloat(v) * 1000);
  await page.locator("[data-pane-filter]").click();
  await until("the filter menu", 5_000, async () => (await menu.count()) === 1);
  const tokens = await page.evaluate(() => {
    const s = getComputedStyle(document.documentElement);
    return { dur: s.getPropertyValue("--dur-120"), ease: s.getPropertyValue("--ease-out").trim() };
  });
  const motion = await menu.evaluate((el) => {
    const s = getComputedStyle(el);
    return { name: s.animationName, dur: s.animationDuration, ease: s.animationTimingFunction };
  });
  if (motion.name === "none") throw new Error("the menu opens with no animation at all");
  if (ms(motion.dur) !== ms(tokens.dur)) {
    throw new Error(`the menu's duration is not our token: ${motion.dur} vs ${tokens.dur}`);
  }
  if (motion.ease !== tokens.ease) {
    throw new Error(`the menu's easing is not our token: ${motion.ease} vs ${tokens.ease}`);
  }
  await page.keyboard.press("Escape");
  await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);

  const tickWord = async (w) => {
    if ((await menu.count()) === 0) await page.locator("[data-pane-filter]").click();
    await until(`the ${w} option in the filter menu`, 5_000, async () =>
      (await page.locator(`[data-filter-option="${w}"]`).count()) === 1,
    );
    await page.locator(`[data-filter-option="${w}"]`).click();
    await page.keyboard.press("Escape");
    await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  };
  for (const w of wordsOnScreen) {
    const mine = await page.locator(`[data-word="${w}"]`).count();
    await tickWord(w);
    await until(`only the ${w} rows`, 5_000, async () => (await sessionRows.count()) === mine);
    if ((await page.locator(`[data-word="${w}"]`).count()) !== mine) {
      throw new Error(`filtering on ${w} dropped ${w} rows`);
    }
    await tickWord(w);
    await until("every row back", 5_000, async () => (await sessionRows.count()) === allRows);
  }

  // 9) turn ends on alpha → done + unread on beta; clicking the row is
  //    seen; the loop closes: badge clears here, alpha turns idle there.
  feedHook(envA, "Stop");
  await until("done + unread badge", 30_000, async () =>
    (await row.getAttribute("data-word")) === "done" && (await row.locator("[data-unread]").count()) === 1,
  );
  // The row is a strip holding two controls now — what opens it, and
  // the pin. Clicking the strip's centre would land on whichever of the
  // two happens to sit there; name the one that means "open".
  await row.locator("[data-row-open]").click();
  await until("beta's row to settle idle, badge gone", 30_000, async () =>
    (await row.getAttribute("data-word")) === "idle" && (await row.locator("[data-unread]").count()) === 0,
  );
  // The loop's far end: alpha's own CLI now prints the same idle word
  // the GUI shows — the watermark travelled, and the wording matches.
  const idleGuiWord = (await row.locator("[data-word-text]").innerText()).trim();
  await until("alpha to read the seen watermark", 30_000, () => {
    const line = cli(envA, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
    return Boolean(line && line.includes(idleGuiWord));
  });

  // 10) faces. Everything below is still on the wide viewport.
  //
  //    a. every session row paints a real derived face. Stated
  //    positively on purpose: the blank branch renders no <svg> at all,
  //    so "there is an svg in every row" *is* "no row fell back to a
  //    placeholder", with no negative selector to spell wrong.
  const rowCount = await page.locator("[data-word]").count();
  const facedRows = await page.locator("[data-word] [data-face] svg").count();
  if (rowCount === 0 || facedRows !== rowCount) {
    throw new Error(`rows with a painted face: ${facedRows} of ${rowCount}`);
  }
  //    …and what it painted is the derivation, not an empty canvas: the
  //    canvas side is one of the two the core ships, and the ground rect
  //    carries a hex color from the palette.
  const rowFace = page.locator("[data-word] [data-face] svg").first();
  const viewBox = await rowFace.getAttribute("viewBox");
  if (viewBox !== "0 0 80 80" && viewBox !== "0 0 36 36") {
    throw new Error(`a face's canvas is neither 80 nor 36: ${viewBox}`);
  }
  if (!/<rect[^>]*fill="#[0-9a-f]{6}"/.test(await rowFace.innerHTML())) {
    throw new Error("a face has no ground rect in a palette color");
  }

  //    b. one machine, two places on one screen, one picture. The blur
  //    filter's id is per-instance (a document-uniqueness device, not
  //    part of the face), so it is normalized away before comparing.
  const faceOf = async (locator) =>
    (await locator.innerHTML()).replace(/av-blur-[^"')\s]+/g, "BLUR");
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) >= 2);
  const railFace = page.locator("nav > [data-face]");
  const betaRow = page.locator('[data-device="beta"] [data-face]');
  if ((await railFace.count()) !== 1) throw new Error("probe dead: no face at the foot of the rail");
  if ((await betaRow.count()) !== 1) throw new Error("probe dead: no face on beta's own row");
  if ((await faceOf(railFace)) !== (await faceOf(betaRow))) {
    throw new Error("this machine has two different faces on one screen");
  }

  //    c. the control: two machines are not one picture. Without this,
  //    a painter that drew the same thing for everyone would pass (b).
  const alphaRow = page.locator('[data-device="alpha"] [data-face]');
  if ((await alphaRow.count()) !== 1) throw new Error("probe dead: no face on alpha's row");
  if ((await faceOf(alphaRow)) === (await faceOf(betaRow))) {
    throw new Error("two machines were painted the same face");
  }

  //    d. a theme flip moves nothing inside the SVG.
  //
  //    Two measurements, because they fail differently. The markup
  //    catches a painter that emitted different numbers or colors. The
  //    *computed* paint catches the subtler one: markup that is
  //    byte-identical because it says `var(--something)`, which then
  //    resolves per theme — a face that reads the theme without any
  //    string ever changing.
  //
  //    The probe has to be proven alive first, or "nothing changed" is
  //    also exactly what a theme switch that never took effect looks
  //    like. So two things that *should* follow the theme are measured
  //    in the same breath: the page's ground, and the face's hairline
  //    edge — the one documented theme-aware part, which lives outside
  //    the SVG.
  const themeProbe = async () =>
    page.evaluate(() => {
      const face = document.querySelector('[data-device="beta"] [data-face]');
      const edge = face && face.querySelector("span");
      const svg = face && face.querySelector("svg");
      return {
        body: getComputedStyle(document.body).backgroundColor,
        edge: edge && getComputedStyle(edge).borderTopColor,
        // Every painted element's resolved paint, in document order
        paint:
          svg &&
          [...svg.querySelectorAll("*")]
            .map((el) => {
              const s = getComputedStyle(el);
              return [el.tagName, s.fill, s.stroke, s.mixBlendMode, s.filter].join("|");
            })
            .join("\n"),
      };
    });
  await page.emulateMedia({ colorScheme: "light" });
  const lightFace = await faceOf(betaRow);
  const light = await themeProbe();
  await page.emulateMedia({ colorScheme: "dark" });
  const darkFace = await faceOf(betaRow);
  const dark = await themeProbe();
  if (light.body === dark.body) throw new Error("probe dead: the theme flip changed no ground color");
  if (!light.edge || light.edge === dark.edge) {
    throw new Error(`probe dead: the avatar edge did not follow the theme (${light.edge} / ${dark.edge})`);
  }
  if (!light.paint) throw new Error("probe dead: read no computed paint off the face");
  if (lightFace !== darkFace) throw new Error("the face's markup changed when the theme did");
  if (light.paint !== dark.paint) throw new Error("the face's computed paint changed when the theme did");
  await page.emulateMedia({ colorScheme: null });

  // 11) the rail's names. The deadline is the assertion: Radix's stock
  //     delay is 700ms, so a window well under that is what separates
  //     "shows at once" from "shows eventually" — drop the delayDuration
  //     override and this goes red rather than slow.
  const tip = page.locator("[data-rail-tip]");
  // Walked, not teleported: a tooltip whose content is hoverable keeps
  // itself open across a grace area, and it takes real pointer moves
  // through that area to leave it.
  const pointerAway = () => page.mouse.move(900, 600, { steps: 12 });
  await pointerAway();
  await until("no tip while the pointer is away", 5_000, async () => (await tip.count()) === 0);
  const sessionsName = await railItem("sessions").getAttribute("aria-label");
  await railItem("sessions").hover();
  await until("the rail name on hover, at once", 400, async () => (await tip.count()) === 1, 50);
  if ((await tip.innerText()).trim() !== sessionsName) {
    throw new Error(`the rail tip names the wrong thing: ${await tip.innerText()}`);
  }
  await pointerAway();
  await until("the tip to leave with the pointer", 5_000, async () => {
    const n = await tip.count();
    // Say which of the two it is: still open, or stuck mid-exit.
    if (n > 0) throw new Error(`still up, data-state=${await tip.getAttribute("data-state")}`);
    return true;
  });
  // Keyboard reaches it too — the tip is the only thing that says where
  // an icon-only glyph goes, so it cannot be a mouse-only affordance.
  //
  // Focus has to genuinely move for there to be a focus event at all:
  // this glyph is already focused from the click that opened its pane,
  // and re-focusing an already-focused element fires nothing, so the
  // assertion would be measuring an event that never happened. Park
  // focus elsewhere first and prove it left.
  const devicesName = await railItem("devices").getAttribute("aria-label");
  const focusedName = () =>
    page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? "");
  await page.locator("[data-pane-search]").focus();
  if ((await focusedName()) === devicesName) {
    throw new Error("probe dead: focus never left the rail glyph, so nothing was tested");
  }
  await railItem("devices").focus();
  await until(
    "the rail name on focus",
    2_000,
    async () => {
      if ((await tip.count()) === 1) return true;
      const who = await page.evaluate(() => {
        const el = document.activeElement;
        return el ? `${el.tagName}[${el.getAttribute("aria-label")}]` : "nothing";
      });
      throw new Error(`no tip; focus sits on ${who}`);
    },
    50,
  );
  if ((await tip.innerText()).trim() !== devicesName) {
    throw new Error(`focus showed the wrong name: ${await tip.innerText()}`);
  }
  await railItem("devices").blur();

  // 12) four landings, and what the two borrowed ones show.
  //
  //     Machine *names* are compared as sets, not counts: three panes
  //     each showing three rows proves nothing if one of them is
  //     showing three of something else.
  const LANDING_TABS = ["sessions", "devices", "files", "browser"];
  const machinesOn = async (tab) => {
    await openLanding(tab);
    await until(`machines on the ${tab} pane`, 10_000, async () =>
      (await page.locator("[data-device]").count()) > 0,
    );
    return (
      await page.locator("[data-device]").evaluateAll((els) => els.map((e) => e.dataset.device))
    ).sort();
  };
  for (const tab of LANDING_TABS) {
    if ((await railItem(tab).count()) !== 1) throw new Error(`no rail glyph for ${tab}`);
  }
  const onDevices = await machinesOn("devices");
  if (onDevices.length !== 3) throw new Error(`expected three machines, saw ${onDevices.length}`);
  for (const tab of ["files", "browser"]) {
    const here = await machinesOn(tab);
    if (here.join(",") !== onDevices.join(",")) {
      throw new Error(`the ${tab} pane lists ${here} where devices lists ${onDevices}`);
    }
  }

  // 13) each pane's bar holds what that pane can do — and nothing it
  //     cannot. The positive half runs first and is what proves the
  //     selectors still name real things: without it, every "is absent"
  //     below would also pass on a renamed attribute.
  const barOf = async (tab) => {
    await openLanding(tab);
    return {
      search: await page.locator("[data-pane-search]").count(),
      filter: await page.locator("[data-pane-filter]").count(),
      plus: await page.locator("[data-pane-new]").count(),
      label: await page.locator("[data-pane-search]").getAttribute("aria-label"),
    };
  };
  const bars = {};
  for (const tab of LANDING_TABS) bars[tab] = await barOf(tab);
  const want = {
    sessions: { search: 1, filter: 1, plus: 1 },
    devices: { search: 1, filter: 0, plus: 1 },
    files: { search: 1, filter: 0, plus: 0 },
    browser: { search: 1, filter: 0, plus: 0 },
  };
  for (const [tab, w] of Object.entries(want)) {
    for (const [what, n] of Object.entries(w)) {
      if (bars[tab][what] !== n) {
        throw new Error(`the ${tab} bar has ${bars[tab][what]} ${what}, expected ${n}`);
      }
    }
    if (!bars[tab].label) throw new Error(`probe dead: the ${tab} search box has no name`);
  }
  // The three machine panes search machines, so they say the same
  // thing; the sessions pane searches something else and says something
  // else. Read off the app, never spelled here — the catalog owns the
  // words, this owns the comparison.
  for (const tab of ["files", "browser"]) {
    if (bars[tab].label !== bars.devices.label) {
      throw new Error(`the ${tab} search box is named ${bars[tab].label}, not ${bars.devices.label}`);
    }
  }
  if (bars.sessions.label === bars.devices.label) {
    throw new Error("the sessions pane borrowed the machine pane's search label");
  }

  // 14) the mark at the head of the rail.
  const mark = page.locator("[data-rail-mark]");
  if ((await mark.count()) !== 1) throw new Error("no mark at the head of the rail");
  // Its artwork really loaded. A broken import renders an <img> all the
  // same, and every other assertion here would still pass.
  const drew = await mark.locator("img").evaluate((el) => el.complete && el.naturalWidth > 0);
  if (!drew) throw new Error("the mark's artwork did not load");
  // It is not a control. Two independent readings, because they fail
  // differently: no button wraps it, and pointing at it changes nothing.
  if (await mark.evaluate((el) => Boolean(el.closest("button")))) {
    throw new Error("the mark sits inside a button");
  }
  const bg = (loc) => loc.evaluate((el) => getComputedStyle(el).backgroundColor);
  const settle = () => new Promise((r) => setTimeout(r, 300));
  await pointerAway();
  await settle();
  const markResting = await bg(mark);
  await mark.hover();
  await settle();
  if ((await bg(mark)) !== markResting) throw new Error("the mark answers the pointer");
  // …and the control that says the measurement can see a hover at all.
  const glyph = railItem("files");
  await pointerAway();
  await settle();
  const glyphResting = await bg(glyph);
  await glyph.hover();
  await settle();
  if ((await bg(glyph)) === glyphResting) {
    throw new Error("probe dead: a rail glyph shows no hover either, so the mark's silence proves nothing");
  }
  await pointerAway();

  // 15) faces of the shell: wide has a detail header but no back;
  //     narrow, after entering a detail, has the back button.
  await openLanding("sessions");
  await until("rows on the session list", 10_000, async () => (await page.locator("[data-word]").count()) > 0);
  await row.locator("[data-row-open]").click();
  if ((await page.locator("[data-detail-header]").count()) !== 1) throw new Error("probe dead: no detail header");
  if ((await page.locator("[data-back]").count()) !== 0) throw new Error("back button on the wide face");
  // Shrinking mid-detail keeps the detail up (Telegram's behavior) —
  // and there, with the list genuinely off-screen, back exists.
  await page.setViewportSize({ width: 390, height: 720 });
  await until("narrow detail with back", 10_000, async () => (await page.locator("[data-back]").count()) === 1);
  await page.locator("[data-back]").click();
  await until("back to the narrow list", 10_000, async () => (await page.locator("[data-list]").count()) === 1);
  await until("rows on the narrow list", 10_000, async () => (await page.locator("[data-word]").count()) > 0);
  // All four landings are reachable down here too — the narrow rail is
  // the only way to any of them — and the mark is not among them: it
  // answers no tap, and this is the row where everything else does.
  for (const tab of LANDING_TABS) {
    if ((await railItem(tab).count()) !== 1) throw new Error(`no ${tab} glyph on the narrow rail`);
  }
  await openLanding("browser");
  await until("machines on the narrow browser pane", 10_000, async () =>
    (await page.locator("[data-device]").count()) === onDevices.length,
  );
  if ((await page.locator("[data-rail-mark]").count()) !== 0) {
    throw new Error("the mark is in the narrow rail");
  }

  // 16) the page never threw.
  if (pageErrors.length) throw new Error(`pageerror: ${pageErrors.join(" | ")}`);

  if (process.env.SMOKE_SHOT) {
    await page.setViewportSize({ width: 1080, height: 720 });
    // The rail clicks above leave the pointer on a glyph, and the rail
    // names appear on hover — park it off the rail so the shot shows
    // the resting screen rather than one mid-hover.
    await page.mouse.move(900, 600);
    await new Promise((r) => setTimeout(r, 600));
    await page.screenshot({ path: process.env.SMOKE_SHOT });
  }

  console.log("gui smoke: all green");
} finally {
  if (browser) await browser.close().catch(() => {});
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGTERM");
    } catch {}
  }
  await new Promise((r) => setTimeout(r, 500));
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGKILL");
    } catch {}
  }
}
