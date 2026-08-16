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
//      the CLI and shows up in the list — and the dialog says how long
//      that ticket lasts in the same sentence `khor invite` prints,
//      with neither the words nor the number written in this script;
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
//  15. pinning crosses the CLI/GUI seam in both directions: a row
//      pinned in the app leads beta's own `khor sessions` and gets a
//      mark there, a row pinned from alpha's terminal shows as pinned
//      in beta's app (which can only happen through the sync layer),
//      both undo, the pinned and unpinned pin are two *shapes* rather
//      than two colors, every row carries the control, and machines
//      pin the same way — with the order always read back off the
//      node, never checked against one this script worked out;
//  16. arranging: the default is 最近 and prints no headings, each
//      other mode groups by its own thing and nothing else, the state
//      mode's group order equals the one the CLI prints (read off both
//      faces, spelled by neither), a pin leads in all four modes, and
//      the choice outlives a reload — it is this screen's posture, kept
//      on this device, unlike a pin;
//  17. the back button exists only on the narrow face (after proving
//      the detail header renders at all — negative assertions must
//      first prove the probe is alive), and the mark is not in the
//      narrow rail;
//  18. the pin is painted and works on the narrow face with the
//      pointer parked away — a hover-only row action is missing on a
//      phone and looks fine in every mouse-driven screenshot;
//  19. search reaches the machine a row came from, asserted on a row
//      whose own id and title provably do not carry that name, so the
//      source is the only thing it can have matched;
//  20. a ticked filter outlives a reload and is remembered per pane —
//      the preference rule the arrangement already followed, and the
//      one place that used to break it;
//  21. the settings screen: three axes of options, every one of them a
//      face the node derived and this app only painted; the marked
//      option on each axis paints what the machine already wears;
//      picking another palette repaints this machine in every place it
//      appears at once and leaves none showing the old one; beam's own
//      canvas really arrives; a hand-picked color belongs to no factory
//      set so none of them is marked; `khor face` agrees about what was
//      picked; a chosen face still ignores the theme; it outlives a
//      reload;
//  22. a machine restyled from its own terminal reaches this screen —
//      alpha changes itself with `khor face` and beta's list follows,
//      both before states established first, with this machine's own
//      face as the control since nobody else may move it;
//  23. a machine row opens that machine's card: the full id on the card
//      has the row's short id as its prefix (so it is *that* machine),
//      one face on both, the readings drawn, and the age line present
//      for a machine reached over the wire and absent for this one —
//      the offline axis, which a locally faked reading would get wrong.
//      **How many readings there are is read off `khor devices`, never
//      written here**, so a machine khor cannot ask a GPU about is
//      covered by the same assertion; and the GPU row itself is checked
//      to be the same sentence and the same card count on both faces;
//  24. the rows that open something are exactly the ones with somewhere
//      to go: the devices pane's do, the files and browser panes' do
//      not, with the positive half read first so the absence means
//      absence and not a renamed selector;
//  25. the hook button is a pair and the button is the report: absent
//      on another machine's card, present on this one, and each press
//      moves both faces together — `khor hooks` reading the same home
//      is the independent witness, with neither word spelled here;
//  26. a pin that does not take says so on the button that was pressed:
//      a real failure through the real path (the backend is taken away,
//      the button is clicked the way a person clicks it), the face
//      provably absent on the successful press just before, the colour
//      measured against a probe wearing the token, and exactly one
//      button wearing it;
//  27. zero pageerror throughout.
// Every wait has a deadline; cleanup runs in finally and kills by pid.
//
// No Chinese literal appears below. Words are read off the running app
// (a rail item's aria-label, a row's data-word) and compared against
// what the other face prints — the catalog owns the text, this script
// owns the comparison.
import { execFileSync, spawn, spawnSync } from "node:child_process";
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
// A CLI verb, retried on failure.
//
// **Not a way of ignoring errors**: a verb that is actually broken fails
// every attempt and still throws, carrying its own stderr. What this
// absorbs is the store race the ledger already records — `Node::open`
// flushes the device table, so even a read-only verb writes, and the
// loser of a race with the resident serve prints "改不了名" and exits 1.
// The bridge embeds a serve against this very home, so the window is
// open on every call here. Before this, the race surfaced as whichever
// assertion happened to be running, which reads like that feature
// breaking rather than like the known race it is.
//
// **So this file no longer guards that race.** At the rate measured
// before the fix (17 of 40), five attempts all failing is ~1%, so a
// regression here would come back green. What guards it is the cargo
// test `opening_one_home_from_many_places_at_once_never_fails`, which
// went 90-of-96 red the moment the shared temp name came back. Do not
// read a green smoke run as evidence about that.
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
// `sid` is a parameter because two homes need rows that are their own:
// a session id is the key the whole network agrees on, so feeding the
// same one into two machines would put one id on two rows with two
// different homes, and every assertion about "which machine is this
// row's" would then be measuring an accident.
function feedHook(env, event, extra = "", sid = "cafe1") {
  const payload = `{"session_id":"${sid}","cwd":"/tmp/proj","hook_event_name":"${event}"${extra}}`;
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
  //    …and the dialog says how long that ticket lasts, in the very
  //    sentence the terminal prints. **Neither the words nor the number
  //    are written here**: another home is asked to mint one and to say
  //    its own line, and the two are required to match — so the day the
  //    window moves, a screen still promising the old one goes red
  //    instead of being a discrepancy nobody is looking at.
  //
  //    `khor invite` puts the ticket on stdout and the window on
  //    stderr, which is why this reads stderr. The ticket that mints is
  //    left to expire unused; minting one is the only way to make it
  //    say the sentence.
  const shownWindow = (await page.locator("[data-ticket-window]").innerText()).trim();
  const saidWindow = spawnSync(KHOR, ["invite"], { env: envA, encoding: "utf8" }).stderr.trim();
  if (!shownWindow) throw new Error("the ticket dialog says nothing about how long it lasts");
  if (!saidWindow) throw new Error("probe dead: `khor invite` said nothing about the window");
  if (shownWindow !== saidWindow) {
    throw new Error(`the dialog says ${shownWindow}, the terminal says ${saidWindow}`);
  }

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

  // 15) pinning, both directions across the CLI/GUI seam.
  //
  //     The order is never checked here — it is read back off the node.
  //     What is asserted is that the row the node put first *is* first,
  //     which is a different claim from "the frontend sorted correctly"
  //     and the only one this layer is allowed to make.
  await openLanding("sessions");
  await until("rows on the session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  const sessionIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const pinButtons = page.locator("[data-row] [data-row-pin]");

  //     a. the control: nothing is pinned, and every row carries the pin
  //     — "every row" is the reachability claim, and counting it against
  //     the rows themselves is what makes it one.
  if ((await page.locator("[data-row][data-pinned=true]").count()) !== 0) {
    throw new Error("something is pinned before the test pinned anything — the control is void");
  }
  if ((await pinButtons.count()) !== (await page.locator("[data-row]").count())) {
    throw new Error(
      `${await pinButtons.count()} pins for ${await page.locator("[data-row]").count()} rows — not every row can be pinned`,
    );
  }

  //     b. GUI → CLI. Pin the second row (not the first: a row already on
  //     top proves nothing about floating), then read beta's own CLI.
  const idsBefore = await sessionIds();
  const target = idsBefore[1];
  const cliLineOf = (env, id) =>
    cli(env, "sessions").split("\n").find((l) => l.startsWith(`${id}\t`)) ?? "";
  const plainLine = cliLineOf(envB, target);
  if (!plainLine) throw new Error(`probe dead: ${target} is not in beta's CLI list`);

  await page.locator(`[data-row="${target}"] [data-row-pin]`).click();
  await until("the pinned row to float to the top", 10_000, async () => {
    const now = await sessionIds();
    if (now[0] === target) return true;
    // Say what the screen actually looks like: "it did not float" and
    // "the click never reached the node" fail the same way from here.
    const pinnedAttr = await page.locator(`[data-row="${target}"]`).getAttribute("data-pinned");
    const backend = cli(envB, "sessions").split("\n").filter(Boolean)[0] ?? "";
    throw new Error(
      `target=${target} data-pinned=${pinnedAttr} order=[${now.join(" ")}] cli-first=${backend.split("\t")[0]}`,
    );
  });
  if ((await page.locator(`[data-row="${target}"]`).getAttribute("data-pinned")) !== "true") {
    throw new Error("the row floated but does not say it is pinned");
  }
  //     …and beta's CLI agrees, both in order and in what it prints. The
  //     mark itself is never spelled here: it is whatever the pinned line
  //     grew relative to the same line before, which is also the proof
  //     that the CLI says *something* rather than only reordering.
  await until("beta's CLI to lead with the pinned row", 10_000, () => {
    // Data rows only: a grouped listing starts with a heading, and a
    // heading is the one line with no tabs in it.
    const rowsOut = cli(envB, "sessions").split("\n").filter((l) => l.includes("\t"));
    return rowsOut[0]?.startsWith(`${target}\t`);
  });
  const markedLine = cliLineOf(envB, target);
  if (markedLine === plainLine || !markedLine.startsWith(plainLine)) {
    throw new Error(`the CLI does not mark the pinned row: ${plainLine} -> ${markedLine}`);
  }

  //     c. the two states are two shapes, not one shape in two colors.
  //     Measured off the element: a painter that only swapped a color
  //     would pass every other assertion here.
  //     Both `transform` and the standalone `rotate` property are read:
  //     tailwind v4 compiles `-rotate-45` to the latter, and reading only
  //     the former returns "none" for a rotation that is plainly there.
  const pinTransform = (id) =>
    page.locator(`[data-row="${id}"] [data-row-pin] svg`).evaluate((el) => {
      const s = getComputedStyle(el);
      return `${s.transform}|${s.rotate}|${s.transformOrigin}`;
    });
  const pinnedShape = await pinTransform(target);
  const restingShape = await pinTransform(idsBefore[0] === target ? idsBefore[1] : idsBefore[0]);
  if (pinnedShape === restingShape) {
    throw new Error(`pinned and unpinned draw the same shape: ${pinnedShape}`);
  }
  if (/\|none\|/.test(pinnedShape)) {
    throw new Error(`the pinned state is not a rotation at all: ${pinnedShape}`);
  }
  //     …and the rotation happens around the middle. An SVG element's
  //     transform-origin defaults to (0, 0), which would swing the pin
  //     out of its own box — visibly wrong, and nothing else here looks.
  const [, , origin] = pinnedShape.split("|");
  if (/^0px 0px/.test(origin)) {
    throw new Error(`the pin rotates around its corner, not its centre: ${origin}`);
  }

  //     d. CLI → GUI, on the far machine. alpha pins one of its own rows
  //     from its terminal; beta's app has to show it, which can only
  //     happen through the sync layer.
  if ((await page.locator('[data-row="tui/cafe1"]').getAttribute("data-pinned")) === "true") {
    throw new Error("tui/cafe1 is already pinned in the GUI — the control is void");
  }
  cli(envA, "pin", "tui/cafe1");
  await until("alpha's pin to reach beta's app", 40_000, async () =>
    (await page.locator('[data-row="tui/cafe1"]').getAttribute("data-pinned")) === "true",
    1_000,
  );

  //     e. taking it back travels the same way, and the row goes home.
  await page.locator(`[data-row="${target}"] [data-row-pin]`).click();
  await until("the unpinned row to leave the top", 10_000, async () => {
    const now = await sessionIds();
    return now[0] !== target;
  });
  // Waited for rather than read once: the app's order and a fresh CLI
  // process are two observers of one write, and they do not have to
  // land in the same millisecond. If it never returns to what it was,
  // this still fails — with both strings in the message.
  await until("beta's CLI line to go back to what it was", 10_000, () => {
    const now = cliLineOf(envB, target);
    if (now === plainLine) return true;
    throw new Error(`before ${JSON.stringify(plainLine)} / now ${JSON.stringify(now)}`);
  });
  cli(envA, "unpin", "tui/cafe1");
  await until("alpha's unpin to reach beta's app", 40_000, async () =>
    (await page.locator('[data-row="tui/cafe1"]').getAttribute("data-pinned")) === "false",
    1_000,
  );

  //     f. machines pin too, and the same way: the table sorts, the app
  //     paints. gamma is last by name, so its arrival at the top is not
  //     something the previous order could have produced.
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) === 3);
  const deviceNames = () =>
    page.locator("[data-device]").evaluateAll((els) => els.map((e) => e.dataset.device));
  if ((await deviceNames())[0] === "gamma") throw new Error("gamma already leads — the control is void");
  await page.locator('[data-device="gamma"] [data-row-pin]').click();
  await until("gamma to lead beta's machine list", 10_000, async () => (await deviceNames())[0] === "gamma");
  await until("beta's CLI to lead with gamma", 10_000, () =>
    cli(envB, "devices").split("\n")[0]?.startsWith("gamma"),
  );
  await page.locator('[data-device="gamma"] [data-row-pin]').click();
  await until("gamma to go back", 10_000, async () => (await deviceNames())[0] !== "gamma");

  // 16) arranging the list: four modes, and the order is always the
  //     node's. The state mode is checked against the CLI's own output
  //     rather than against an order this script worked out — neither
  //     side spells a state word here, they are read off both faces and
  //     compared.
  await openLanding("sessions");
  await until("rows on the session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  if ((await page.locator("[data-row][data-pinned=true]").count()) !== 0) {
    throw new Error("a row is still pinned from the section above — the default check is void");
  }
  // Put the agent back to work first. The seen loop above settled every
  // row to one word, and one word is one group — "grouped by state"
  // would pass on a single heading while proving nothing about grouping
  // at all.
  feedHook(envA, "UserPromptSubmit");
  await until(
    "alpha's row to be busy again in beta's app",
    40_000,
    async () => (await page.locator('[data-row="tui/cafe1"]').getAttribute("data-word")) === "busy",
    1_000,
  );
  const wordsHere = [
    ...new Set(await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.word))),
  ];
  if (wordsHere.length < 2) {
    throw new Error(`every row wears ${wordsHere[0]} — grouping by state cannot be told from a no-op`);
  }
  const groupKeys = () =>
    page.locator("[data-group]").evaluateAll((els) => els.map((e) => e.dataset.group));
  const groupTexts = () =>
    page.locator("[data-group]").evaluateAll((els) => els.map((e) => e.innerText.trim()));
  const openMenu = async () => {
    if ((await menu.count()) === 0) await page.locator("[data-pane-filter]").click();
    await until("the filter menu", 5_000, async () => (await menu.count()) === 1);
  };
  const closeMenu = async () => {
    await page.keyboard.press("Escape");
    await until("the filter menu to close", 5_000, async () => (await menu.count()) === 0);
  };
  const chosenArrange = () =>
    page.locator("[data-arrange-option][data-state=checked]").getAttribute("data-arrange-option");
  const chooseArrange = async (key) => {
    await openMenu();
    await page.locator(`[data-arrange-option="${key}"]`).click();
    await closeMenu();
  };

  //     a. the default, with this run having chosen nothing: recent,
  //     and no headings at all — the one mode that does not group.
  await openMenu();
  const byDefault = await chosenArrange();
  await closeMenu();
  if (byDefault !== "recent") throw new Error(`the default arrangement is ${byDefault}`);
  if ((await groupKeys()).length !== 0) {
    throw new Error(`recent should print no headings, saw ${await groupKeys()}`);
  }

  //     b. every other mode groups by its own thing and by nothing
  //     else. The prefixes are the node's (khor_node::list), which is
  //     what lets a heading be a word or a machine name without the
  //     frontend guessing which.
  for (const [mode, prefix] of [
    ["category", "cat:"],
    ["device", "dev:"],
    ["state", "state:"],
  ]) {
    await chooseArrange(mode);
    await until(`${mode} headings`, 15_000, async () => (await groupKeys()).length >= 2);
    const keys = await groupKeys();
    if (!keys.every((k) => k.startsWith(prefix))) {
      throw new Error(`${mode} produced headings that are not its own: ${keys}`);
    }
  }

  //     c. the state mode's group order is the node's ranking — proven
  //     by reading the same order out of the CLI's own listing. The CLI
  //     order is taken from its *rows* (second column), so this does not
  //     depend on how a heading is punctuated there.
  await chooseArrange("state");
  await until("the state grouping to settle", 15_000, async () => (await groupKeys()).length >= 2);
  const guiStateOrder = await groupTexts();
  const cliStateOrder = [
    ...new Set(
      cli(envB, "sessions", "--by", "state")
        .split("\n")
        .filter((l) => l.includes("\t"))
        .map((l) => l.split("\t")[1]),
    ),
  ];
  if (guiStateOrder.join("|") !== cliStateOrder.join("|")) {
    throw new Error(
      `the two faces disagree on the state order: GUI [${guiStateOrder}] vs CLI [${cliStateOrder}]`,
    );
  }

  //     d. **a pin is not a mode**: it leads in all four. The row picked
  //     is not the one already on top, so nothing but the pin could put
  //     it there.
  const arrangeIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const pinTarget = (await arrangeIds())[2];
  await page.locator(`[data-row="${pinTarget}"] [data-row-pin]`).click();
  for (const mode of ["recent", "category", "device", "state"]) {
    await chooseArrange(mode);
    await until(`${pinTarget} leading in ${mode}`, 15_000, async () => {
      const now = await arrangeIds();
      return now[0] === pinTarget;
    });
  }
  await page.locator(`[data-row="${pinTarget}"] [data-row-pin]`).click();
  await until("the pin to come off again", 10_000, async () =>
    (await page.locator("[data-row][data-pinned=true]").count()) === 0,
  );

  //     e. the choice outlives a reload. It is this screen's posture and
  //     is kept on this device — unlike a pin, which travels the network
  //     because it belongs to the session rather than to the screen.
  await chooseArrange("device");
  await page.reload();
  await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 1);
  await openMenu();
  const afterReload = await chosenArrange();
  await closeMenu();
  if (afterReload !== "device") {
    throw new Error(`the arrangement did not survive a reload: ${afterReload}`);
  }
  // Leave it as it was found, so the sections below see the default.
  await chooseArrange("recent");

  // 17) faces of the shell: wide has a detail header but no back;
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

  // 18) the pin is reachable on the narrow face — the one where there is
  //     no pointer to reveal it with. A hover-only row action is simply
  //     absent on a phone, and it looks identical to a working one in
  //     every screenshot taken with a mouse present.
  //
  //     So: park the pointer far away, then measure that the button is
  //     really painted (visible, non-zero box, not transparent) and that
  //     pressing it does what it does on the wide face.
  await openLanding("sessions");
  await until("rows on the narrow session list", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  await pointerAway();
  await settle();
  const narrowPin = page.locator("[data-row] [data-row-pin]").first();
  const painted = await narrowPin.evaluate((el) => {
    const s = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    return { opacity: s.opacity, display: s.display, visibility: s.visibility, w: box.width, h: box.height };
  });
  if (
    painted.display === "none" ||
    painted.visibility === "hidden" ||
    Number(painted.opacity) === 0 ||
    painted.w === 0 ||
    painted.h === 0
  ) {
    throw new Error(`the pin is hover-only on the narrow face: ${JSON.stringify(painted)}`);
  }
  // …and it still works down here, which is the half a visibility check
  // cannot cover: a button can be perfectly visible and out of reach.
  const narrowIds = () =>
    page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row));
  const narrowTarget = (await narrowIds())[1];
  await page.locator(`[data-row="${narrowTarget}"] [data-row-pin]`).click();
  await until("the narrow pin to float its row", 10_000, async () => (await narrowIds())[0] === narrowTarget);
  await page.locator(`[data-row="${narrowTarget}"] [data-row-pin]`).click();
  await until("the narrow row to go back", 10_000, async () => (await narrowIds())[0] !== narrowTarget);

  // 19) search reaches the machine a row came from.
  //
  //     The premise has to be established or this measures nothing: find
  //     a row that **is** from somewhere else and whose own title and id
  //     do not contain that machine's name. Then searching the name has
  //     to keep it, and the only string it can be matching on is the
  //     source. Beta also holds a `chat/alpha` row — titled and keyed by
  //     that same name — which is exactly why the assertion is about one
  //     identified row rather than about how many rows survive.
  await page.setViewportSize({ width: 1080, height: 720 });
  await openLanding("sessions");
  await until("rows back on the wide face", 10_000, async () => (await page.locator("[data-row]").count()) > 1);
  const reported = await page.locator("[data-row][data-source]").evaluateAll((els) =>
    els.map((e) => ({ id: e.dataset.row, from: e.dataset.source, title: e.dataset.title ?? "" })),
  );
  if (!reported.length) throw new Error("probe dead: no row carries a source");
  // Title and id only — deliberately **not** the row's rendered text,
  // which paints "from <machine>" and therefore always contains the
  // name. Checking that would make this probe unsatisfiable, and the
  // failure would read like the feature was broken.
  const bySource = reported.find(
    (r) =>
      !r.id.toLowerCase().includes(r.from.toLowerCase()) &&
      !r.title.toLowerCase().includes(r.from.toLowerCase()),
  );
  if (!bySource) {
    throw new Error(
      `probe dead: every reported row already names its machine in its own id or title — ` +
        `matching on the source cannot be told apart: ${JSON.stringify(reported)}`,
    );
  }
  await sessionSearch.fill(bySource.from);
  await until(`the row from ${bySource.from} to survive its machine's name`, 5_000, async () =>
    (await page.locator(`[data-row="${bySource.id}"]`).count()) === 1,
  );
  // The control: a term nothing answers to drops it, so the line above
  // is the search working rather than the search being off.
  await sessionSearch.fill("no-machine-answers-to-this");
  await until("that row gone for a term nothing matches", 5_000, async () =>
    (await page.locator(`[data-row="${bySource.id}"]`).count()) === 0,
  );
  await sessionSearch.fill("");
  await until("every session back", 5_000, async () => (await page.locator("[data-row]").count()) > 1);

  // 20) a ticked filter outlives a reload, and is kept per pane.
  //
  //     The same rule the arrangement follows — a preference set once
  //     stays set — and it used to be the one place that broke it. The
  //     control is the state before: the tick has to be provably absent,
  //     or "still ticked after a reload" could just be a tick that was
  //     never anything else.
  const filterLit = () => page.locator("[data-pane-filter][data-on=true]").count();
  if ((await filterLit()) !== 0) throw new Error("probe dead: the filter is already on");
  const keptWord = wordsOnScreen[0];
  const keptRows = await page.locator(`[data-word="${keptWord}"]`).count();
  await tickWord(keptWord);
  await until(`only the ${keptWord} rows`, 5_000, async () => (await page.locator("[data-row]").count()) === keptRows);

  await page.reload();
  await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 0);
  if ((await filterLit()) !== 1) throw new Error("the filter control came back unlit after a reload");
  if ((await page.locator("[data-row]").count()) !== keptRows) {
    throw new Error("the ticked filter did not survive a reload — the list came back unfiltered");
  }
  // Per pane, not one setting shared by all of them: the stored key
  // carries the landing. Read off storage because that *is* the
  // mechanism, and today only one pane has a filter to tick.
  const filterKeys = await page.evaluate(() =>
    Object.keys(window.localStorage).filter((k) => k.startsWith("khor.filter")),
  );
  if (filterKeys.length !== 1 || !filterKeys[0].endsWith(".sessions")) {
    throw new Error(`the filter is not remembered per pane: ${JSON.stringify(filterKeys)}`);
  }
  await tickWord(keptWord);
  await until("every row back", 10_000, async () => (await page.locator("[data-row]").count()) > keptRows);

  // 21) the settings screen: what this machine wears, and changing it.
  //
  //     Every swatch there is a face **the node derived** and this app
  //     only painted, so the assertions are about the same picture
  //     turning up in two places — never about a picture looking a
  //     particular way, which is the one thing no test here can judge.
  //
  //     A session of beta's own first. Beta's other rows all belong to
  //     alpha (a chat channel is homed on the machine it talks to, and
  //     the agent row is alpha's report), so without this the sessions
  //     pane carries no face of beta's at all and "it moved everywhere"
  //     would be measuring the rail twice.
  feedHook(envB, "SessionStart", "", "beef2");
  feedHook(envB, "UserPromptSubmit", "", "beef2");
  await openLanding("sessions");
  await until("beta's own row", 15_000, async () =>
    (await page.locator('[data-row="tui/beef2"]').count()) === 1,
  );

  // The rail's last glyph is the one that opens no landing — and there
  // being exactly one such glyph is itself the check that this is still
  // the settings one and not something added beside it.
  const settingsGlyph = page.locator("[data-rail-item]:not([data-landing])");
  if ((await settingsGlyph.count()) !== 1) {
    throw new Error("probe dead: the rail does not have exactly one glyph that opens no landing");
  }
  await settingsGlyph.click();
  const sheet = page.locator("[data-face-settings]");
  await until("the settings sheet", 10_000, async () => (await sheet.count()) === 1);
  // The sheet's frame mounts before its answer arrives — it asks the
  // node when it opens, and paints nothing but its title until then. So
  // wait for the options, not for the box: waiting on the box read the
  // empty frame and reported "the settings screen offers []", which
  // reads like the screen being broken rather than like arriving early.
  // (It passed on three runs before it failed on the fourth, which is
  // the whole signature of waiting for the wrong thing.)
  await until("the options to arrive", 10_000, async () =>
    (await sheet.locator("[data-face-option]").count()) > 0,
  );

  //     a. three axes, and every option on all of them painted by the
  //     real brush. Stated positively for the same reason the row faces
  //     are: a face that failed to derive renders no <svg> at all, so
  //     counting them *is* "nothing fell back to a placeholder".
  const axisNames = await sheet
    .locator("[data-face-axis]")
    .evaluateAll((els) => els.map((e) => e.dataset.faceAxis));
  if (axisNames.join(",") !== "palette,variant,shape") {
    throw new Error(`the settings screen offers ${JSON.stringify(axisNames)}`);
  }
  const optionCount = await sheet.locator("[data-face-option]").count();
  const optionFaces = await sheet.locator("[data-face-option] [data-face] svg").count();
  if (optionCount < 6 || optionFaces !== optionCount) {
    throw new Error(`options with a painted face: ${optionFaces} of ${optionCount}`);
  }

  //     b. one option marked per axis, and **all three paint the face
  //     this machine is already wearing**. That follows from the
  //     design — a marked option is the current value on its axis, and
  //     every option is derived with the other two left alone — which
  //     is exactly why it is worth asserting: one line catches a swatch
  //     derived from the wrong style, a mark on the wrong option, and a
  //     second painter.
  // The same locator item 10 proved is this machine's face at the foot
  // of the rail — reused rather than re-derived, so what moves below is
  // the picture that test already tied to beta's own row.
  const markedFaces = async () =>
    sheet
      .locator("[data-face-option][data-on=true] [data-face]")
      .evaluateAll((els) => els.map((e) => e.innerHTML.replace(/av-blur-[^"')\s]+/g, "BLUR")));
  let worn = await faceOf(railFace);
  const marked = await markedFaces();
  if (marked.length !== 3) throw new Error(`${marked.length} options are marked; one per axis`);
  if (marked.some((m) => m !== worn)) {
    throw new Error("a marked option is not the face this machine is wearing");
  }

  //     c. picking another palette moves this machine's face
  //     **everywhere it appears**, in one go.
  //
  //     Counted rather than named: this script does not work out which
  //     rows are beta's, it counts how many places wore that face
  //     before and requires the same number to wear the new one after,
  //     and none to be left wearing the old. Scoped to #root so the
  //     sheet's own swatches — which change too — stay out of it.
  const wearing = async (want) =>
    (await page.locator("#root [data-face]").evaluateAll((els) => els.map((e) => e.innerHTML)))
      .filter((h) => h.replace(/av-blur-[^"')\s]+/g, "BLUR") === want).length;
  const places = await wearing(worn);
  if (places < 2) {
    throw new Error(`this machine's face is in ${places} place(s); the premise is more than one`);
  }
  const otherPalette = (
    await sheet
      .locator('[data-face-axis="palette"] [data-face-option]')
      .evaluateAll((els) => els.filter((e) => e.dataset.on !== "true").map((e) => e.dataset.faceOption))
  )[0];
  if (!otherPalette) throw new Error("probe dead: every palette is already the chosen one");
  await sheet.locator(`[data-face-option="${otherPalette}"][data-axis="palette"]`).click();
  await until("this machine repainted everywhere", 15_000, async () => {
    const now = await faceOf(railFace);
    return now !== worn && (await wearing(now)) === places;
  });
  const wasWorn = worn;
  worn = await faceOf(railFace);
  if ((await wearing(wasWorn)) !== 0) throw new Error("somewhere is still painting the old face");

  //     d. a variant reaches the canvas the core ships. beam's canvas
  //     is 36 where the other two are 80, so switching to it and back
  //     is a change no styling could fake and no cache could survive.
  const canvasOf = () => railFace.locator("svg").getAttribute("viewBox");
  const wideCanvas = await canvasOf();
  await sheet.locator('[data-face-option="beam"][data-axis="variant"]').click();
  await until("beam's own canvas", 15_000, async () => (await canvasOf()) === "0 0 36 36");
  await sheet.locator('[data-face-option="marble"][data-axis="variant"]').click();
  await until("back off beam's canvas", 15_000, async () => (await canvasOf()) === wideCanvas);

  //     e. a hand-picked color belongs to no factory set, so **no**
  //     palette is marked — two marks, not three. A nearest match would
  //     light a button nobody pressed, and pressing that lit button
  //     would then look like a control that does nothing.
  //
  //     Driven by setting the well and dispatching what a commit
  //     dispatches. **This is not the forbidden kind of shortcut.** The
  //     rule against going around the path under test is about stepping
  //     over the app's own code when it is in the way; the OS color
  //     picker is not the app's code, and it is a window Playwright
  //     cannot open at all. Everything from the element and the event
  //     onward is the real path — the listener being tested reads the
  //     element, not the event — so this is as close to a person's
  //     press as anything automated can stand.
  worn = await faceOf(railFace);
  await sheet.locator("[data-color-slot]").first().evaluate((el) => {
    el.value = "#123456";
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await until("the hand-picked color everywhere", 15_000, async () => {
    const now = await faceOf(railFace);
    return now !== worn && (await wearing(now)) === places;
  });
  const afterHand = await markedFaces();
  if (afterHand.length !== 2) {
    throw new Error(`${afterHand.length} options marked after a hand-picked color; no palette is one`);
  }

  //     f. the CLI says the same thing — so the change went through the
  //     library and not just through this screen. Read structurally:
  //     the marked lines are the ones carrying a third field, and the
  //     words themselves belong to the catalog, not to this script.
  await sheet.locator(`[data-face-option="${otherPalette}"][data-axis="palette"]`).click();
  await until("the factory palette back", 15_000, async () => (await markedFaces()).length === 3);
  //     …and the three marked options still paint what this machine
  //     wears. Re-asserted here and not only at the top, because up
  //     there the machine wore the factory style and every axis sat on
  //     its default — an option derived from the defaults instead of
  //     from the current style would have looked exactly the same.
  const nowWorn = await faceOf(railFace);
  if ((await markedFaces()).some((m) => m !== nowWorn)) {
    throw new Error("after a change, a marked option is not the face this machine is wearing");
  }
  const cliMarks = cli(envB, "face")
    .split("\n")
    .filter((l) => l.split("\t").length === 3);
  if (cliMarks.length !== 3) throw new Error(`khor face marks ${cliMarks.length} options; one per axis`);
  if (!cliMarks.some((l) => l.startsWith(`${otherPalette}\t`))) {
    throw new Error(`khor face does not agree that ${otherPalette} is what this machine wears`);
  }

  //     g. flipping the theme still moves nothing inside the face — now
  //     under a style somebody chose rather than the factory one. The
  //     ground proves the flip took, the way it does above.
  const chosen = await faceOf(railFace);
  const groundOf = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  await page.emulateMedia({ colorScheme: "light" });
  const lightGround = await groundOf();
  const lightChosen = await faceOf(railFace);
  await page.emulateMedia({ colorScheme: "dark" });
  if ((await groundOf()) === lightGround) throw new Error("probe dead: the theme flip changed no ground");
  if ((await faceOf(railFace)) !== lightChosen) throw new Error("a chosen face followed the theme");
  await page.emulateMedia({ colorScheme: null });

  //     h. and it outlives a reload — the choice lives in the node, not
  //     on this screen.
  await page.keyboard.press("Escape");
  await until("the sheet closed", 10_000, async () => (await sheet.count()) === 0);
  await page.reload();
  await until("rows after the reload", 20_000, async () => (await page.locator("[data-row]").count()) > 0);
  if ((await faceOf(railFace)) !== chosen) throw new Error("the chosen face did not survive a reload");

  // 22) a machine restyled from its own terminal reaches this screen.
  //
  //     The direction that carries "only a machine may say what it
  //     looks like": alpha changes its own style with `khor face`, and
  //     beta's screen follows — which can only happen through the
  //     device table and the sync pump. The before state is established
  //     on both sides first (beta is provably painting something else,
  //     and alpha's palette provably changed), and the control comes
  //     after: beta's own face did not move, because nobody else may
  //     move it.
  await openLanding("devices");
  const alphaFace = page.locator('[data-device="alpha"] [data-face]');
  const betaFace = page.locator('[data-device="beta"] [data-face]');
  await until("both machines on the list", 10_000, async () => (await alphaFace.count()) === 1);
  const alphaWas = await faceOf(alphaFace);
  const betaWas = await faceOf(betaFace);
  const alphaColorsBefore = cli(envA, "face").split("\n")[1];
  cli(envA, "face", "--palette", "monokai", "--variant", "bauhaus");
  const alphaColorsAfter = cli(envA, "face").split("\n")[1];
  if (alphaColorsBefore === alphaColorsAfter) {
    throw new Error("probe dead: alpha's own palette did not change");
  }
  await until("alpha's new face on beta's screen", 40_000, async () =>
    (await faceOf(alphaFace)) !== alphaWas,
  );
  if ((await faceOf(betaFace)) !== betaWas) {
    throw new Error("alpha restyling itself repainted this machine");
  }

  // 23) a machine row opens that machine's card.
  //
  //     The wide face is back on by now, so the list and the card are on
  //     screen together. The control comes first and it is a real one:
  //     the card's section exists as soon as the devices pane is open
  //     (it is the detail region), so what proves a machine was opened
  //     is its *content* — no id is printed until a row is clicked.
  await page.setViewportSize({ width: 1080, height: 720 });
  await openLanding("devices");
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) >= 2);
  if ((await page.locator("[data-machine-id]").count()) !== 0) {
    throw new Error("a machine card is filled in before any machine was opened");
  }

  //     **`khor devices` is the independent witness for the readings.**
  //     It prints the same reading from the same node, and `vitals_line`
  //     joins its pieces with two spaces — so counting them needs no word
  //     from the catalog, and this file stays free of Chinese.
  const cliReadings = (machine, env) => {
    const line = cli(env, "devices")
      .split("\n")
      .find((l) => l.startsWith(`${machine}\t`));
    if (!line) throw new Error(`probe dead: no ${machine} row printed by \`khor devices\``);
    const cells = line.split("\t");
    if (cells.length < 3) throw new Error(`probe dead: ${machine}'s row prints no readings`);
    return cells[2].split("  ").filter(Boolean);
  };
  //     The age counts on the GUI side too, because the CLI prints it as
  //     one more piece of that same run — leaving it out would make the
  //     two disagree for a reason that has nothing to do with readings.
  const sameNumberOfReadings = async (machine, env) => {
    const onScreen =
      (await page.locator("[data-vitals-unit]").count()) +
      (await page.locator("[data-vitals-age]").count());
    const printed = cliReadings(machine, env);
    if (onScreen !== printed.length) {
      throw new Error(
        `${machine}: the card draws ${onScreen} readings, \`khor devices\` prints ` +
          `${printed.length} — ${printed.join(" | ")}`,
      );
    }
  };

  //     Clicked the way a person clicks it: the row's own button.
  const shortId = await page
    .locator('[data-device="alpha"]')
    .evaluate((el) => el.dataset.row.slice(0, 12));
  await page.locator('[data-device="alpha"] [data-row-open]').click();
  await until("alpha's card", 10_000, async () => (await page.locator("[data-machine-id]").count()) === 1);

  //     …and it is alpha's card, not merely *a* card. The row prints the
  //     first twelve characters of the id and the card prints all of it,
  //     so one being a prefix of the other ties the two together without
  //     this script knowing either value.
  const cardId = await page.locator("[data-machine-id]").innerText();
  if (!cardId.startsWith(shortId) || cardId.length <= shortId.length) {
    throw new Error(`the card shows ${cardId}, which is not the full id behind ${shortId}`);
  }

  //     One machine, one picture — the same rule the rail and the row
  //     already answer to, now with a third place to be wrong in.
  const cardFace = page.locator("[data-machine-card] [data-face]");
  if ((await cardFace.count()) !== 1) throw new Error("probe dead: no face on the card");
  if ((await faceOf(cardFace)) !== (await faceOf(page.locator('[data-device="alpha"] [data-face]')))) {
    throw new Error("alpha wears one face in the list and another on its card");
  }

  //     The readings are drawn, and **how many of them there are is not
  //     a number written here**. It follows what the backend could read
  //     on this machine, which is the entire point of the GPU being a
  //     field that can be absent: on a machine khor cannot ask, both
  //     faces show one row fewer and this still holds. So the count comes
  //     from `khor devices` printing the same reading — an independent
  //     witness, with the age line counted on both sides because the CLI
  //     prints it inside the same run of cells.
  if ((await page.locator("[data-vitals-bar]").count()) === 0) {
    throw new Error("no reading drew a bar");
  }
  await sameNumberOfReadings("alpha", envB);

  //     **The offline axis reaches the screen.** alpha's reading was
  //     taken on alpha and carried here, so it has an age; beta samples
  //     itself to answer the very call that painted this, so it has
  //     none. Both halves are asserted because either alone passes for
  //     the wrong reason — always showing an age, or never showing one.
  if ((await page.locator("[data-vitals-age]").count()) !== 1) {
    throw new Error("a reading that travelled here shows no age");
  }
  await page.locator('[data-device="beta"] [data-row-open]').click();
  await until("beta's card", 10_000, async () =>
    (await page.locator("[data-machine-id]").innerText()).startsWith(
      await page.locator('[data-device="beta"]').evaluate((el) => el.dataset.row.slice(0, 12)),
    ),
  );
  if ((await page.locator("[data-vitals-unit]").count()) === 0) {
    throw new Error("this machine's own card is missing readings");
  }
  if ((await page.locator("[data-vitals-age]").count()) !== 0) {
    throw new Error("this machine's own reading is dressed as something remembered");
  }
  await sameNumberOfReadings("beta", envB);

  //     **The GPU is one of those readings, and it is the same sentence
  //     on both faces.** The counts above already tie how many rows there
  //     are to what the CLI printed; what is left is that this particular
  //     row says what the CLI says. Digits are stripped before comparing
  //     because the utilisation is alive and the two faces sample a
  //     moment apart — what has to match is the sentence (which is the
  //     catalog key, so a word drifting on one face reddens this) and the
  //     card count, which is not alive.
  //
  //     Required, not skipped: every Mac has an accelerator, so a missing
  //     row here is a real finding. On a machine khor cannot ask this
  //     goes red and names the reason, which beats a silent skip that
  //     would pass by not running.
  const gpuRow = page.locator('[data-vitals-unit="gpu"]');
  if ((await gpuRow.count()) !== 1) {
    throw new Error("no GPU reading on this machine's card, and every Mac has one to read");
  }
  const sentence = (s) => s.replace(/\d+/g, "#").replace(/\s+/g, " ").trim();
  const cardCount = (s) => s.match(/\d+/g)?.[1];
  const onCard = await gpuRow.innerText();
  const inTerminal = cliReadings("beta", envB).find((p) => sentence(p) === sentence(onCard));
  if (!inTerminal) {
    throw new Error(
      `the card says "${onCard}" and \`khor devices\` prints nothing shaped like it — ` +
        `${cliReadings("beta", envB).join(" | ")}`,
    );
  }
  if (cardCount(onCard) !== cardCount(inTerminal)) {
    throw new Error(`the card counts ${cardCount(onCard)} cards, the terminal ${cardCount(inTerminal)}`);
  }

  // 24) only the rows with somewhere to go open anything.
  //
  //     The positive half is above (alpha's row was clicked and it
  //     opened). Here is the other half, and the count on the devices
  //     pane is re-read first so that "zero on files" means zero rather
  //     than a selector that stopped naming anything.
  const openersOn = async (tab) => {
    await openLanding(tab);
    await until(`machines on the ${tab} pane`, 10_000, async () =>
      (await page.locator("[data-device]").count()) > 0,
    );
    return page.locator("[data-device] [data-row-open]").count();
  };
  const openersOnDevices = await openersOn("devices");
  if (openersOnDevices < 2) {
    throw new Error(`probe dead: ${openersOnDevices} machine rows open anything on the devices pane`);
  }
  for (const tab of ["files", "browser"]) {
    const n = await openersOn(tab);
    if (n !== 0) {
      throw new Error(`${n} machine rows on the ${tab} pane offer to open something that is not there`);
    }
  }
  // Back where the next item expects to be: it pins the first row it
  // finds, and which pane is open decides which row that is.
  await openLanding("devices");

  // 25) the hook button is a pair, and the button *is* the report.
  //
  //     The state it edits lives in a file, so the CLI is an independent
  //     witness: `khor hooks` reading beta's own home says the same
  //     thing the button does, before and after each press. Both faces,
  //     one fact — and neither word is spelled here, they are read off
  //     the two and required to move together.
  //
  //     **It writes into beta's smoke home, never the developer's.**
  //     `adaptor::vendor_home` roots at KHOR_HOME, which this run set,
  //     so `~/.claude` is not reachable from here at all.
  const hookWords = () =>
    cli(envB, "hooks")
      .split("\n")
      .filter((l) => l.includes("\t"))
      .map((l) => l.split("\t")[1]);
  const hookToggle = page.locator("[data-hooks-toggle]");

  //     The control: this control belongs to this machine, so alpha's
  //     card must not carry it. Read first, because "absent on alpha"
  //     would also pass if the selector named nothing anywhere.
  await openLanding("devices");
  await page.locator('[data-device="beta"] [data-row-open]').click();
  await until("the hook button on this machine's card", 10_000, async () =>
    (await hookToggle.count()) === 1,
  );
  await page.locator('[data-device="alpha"] [data-row-open]').click();
  await until("alpha's card", 10_000, async () =>
    (await page.locator("[data-machine-id]").count()) === 1,
  );
  if ((await hookToggle.count()) !== 0) {
    throw new Error("another machine's card offers to edit this machine's hooks");
  }
  await page.locator('[data-device="beta"] [data-row-open]').click();
  await until("back on this machine's card", 10_000, async () => (await hookToggle.count()) === 1);

  //     Not installed to begin with, said by both faces.
  const offWord = await hookToggle.innerText();
  const offCli = hookWords();
  if (offCli.length === 0) throw new Error("probe dead: `khor hooks` listed no events");
  if (await page.locator('[data-hooks][data-installed="true"]').count()) {
    throw new Error("hooks report themselves installed in a home that has never had any");
  }

  //     Pressed the way a person presses it.
  await hookToggle.click();
  await until("the hooks to go on", 10_000, async () =>
    (await page.locator('[data-hooks][data-installed="true"]').count()) === 1,
  );
  const onWord = await hookToggle.innerText();
  if (!onWord || onWord === offWord) {
    throw new Error(`the button kept its old name after installing: ${onWord}`);
  }
  const onCli = hookWords();
  if (onCli.some((w) => offCli.includes(w))) {
    throw new Error(`the CLI says the same thing before and after installing: ${onCli}`);
  }
  if (new Set(onCli).size !== 1) {
    throw new Error(`installing left the events disagreeing: ${onCli}`);
  }

  //     …and back off again, all the way to the words it started with.
  //     A one-way button would pass everything above.
  await hookToggle.click();
  await until("the hooks to come back off", 10_000, async () =>
    (await page.locator('[data-hooks][data-installed="false"]').count()) === 1,
  );
  if ((await hookToggle.innerText()) !== offWord) {
    throw new Error("the button did not go back to offering the install");
  }
  if (hookWords().join(",") !== offCli.join(",")) {
    throw new Error(`the CLI does not agree the hooks came out: ${hookWords()}`);
  }

  // 26) a pin that does not take says so, on the button that was
  //     pressed.
  //
  //     **A real failure, through the real path.** The backend is taken
  //     away and the pin is pressed in the app the way a person presses
  //     it; nothing is stubbed and no call is made behind the UI's back.
  //     This is last because it ends the bridge.
  //
  //     The control comes first: the same button, pressed while the
  //     backend is there, must come back *not* wearing the face. A mark
  //     that is always on is not a failure report.
  const failTarget = (await page.locator("[data-row]").evaluateAll((els) => els.map((e) => e.dataset.row)))[0];
  const failPin = page.locator(`[data-row="${failTarget}"] [data-row-pin]`);
  // **The colour is read after the button's own transitions finish**,
  // not the instant the attribute flips. The button carries
  // `transition-colors`, and a reading taken at t=0 of a transition is
  // bit-for-bit the colour it is *leaving* — which is what one observed
  // failure here reported, with the failure attribute already set and
  // the built stylesheet putting the failure rule last. Waiting on the
  // element's own animations is the "wait for the thing itself" rule; a
  // fixed sleep would measure how busy the machine is, and two equal
  // consecutive reads can both land before the transition starts moving.
  const pinState = () =>
    failPin.evaluate(async (el) => {
      await Promise.all(el.getAnimations().map((a) => a.finished.catch(() => {})));
      return {
        failed: el.dataset.pinFailed,
        name: el.getAttribute("aria-label"),
        color: getComputedStyle(el).color,
      };
    });

  await failPin.click();
  await until("the pin to take while the backend is there", 10_000, async () =>
    (await page.locator(`[data-row="${failTarget}"][data-pinned=true]`).count()) === 1,
  );
  const okState = await pinState();
  if (okState.failed !== "false") {
    throw new Error(`a pin that worked is wearing the failure face: ${JSON.stringify(okState)}`);
  }

  // What the failure colour actually computes to, measured off a probe
  // wearing the token rather than written down here — a hex compared
  // against a browser's `rgb()` never matches, and hard-coding either
  // side would stop tracking the theme.
  const failedColor = await page.evaluate(() => {
    const el = document.createElement("span");
    el.style.color = "var(--state-failed)";
    document.body.appendChild(el);
    const c = getComputedStyle(el).color;
    el.remove();
    return c;
  });

  process.kill(-bridge.pid, "SIGKILL");
  await until("the bridge to be gone", 15_000, async () => {
    const r = await fetch(`http://127.0.0.1:${BRIDGE_PORT}/devices`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "{}",
    }).catch(() => null);
    return r === null;
  });

  await failPin.click();
  await until("the failure face on the button that was pressed", 10_000, async () =>
    (await pinState()).failed === "true",
  );
  const badState = await pinState();
  // The name changes with it, so the report is not colour alone —
  // compared against the working button's name rather than spelled
  // here, because the words belong to the catalog.
  if (!badState.name || badState.name === okState.name) {
    throw new Error(`the failed pin kept its old name: ${JSON.stringify(badState)}`);
  }
  if (badState.color !== failedColor) {
    throw new Error(
      `the failed pin is not wearing the failure colour: ${badState.color} vs ${failedColor}`,
    );
  }
  // …and it is that row's report, not the whole list going red.
  const others = await page
    .locator("[data-row-pin]")
    .evaluateAll((els) => els.filter((e) => e.dataset.pinFailed === "true").length);
  if (others !== 1) throw new Error(`${others} buttons wear the failure face; exactly one was pressed`);

  // 27) the page never threw.
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
