// The native gate: what AppKit draws, measured on the built app.
//
// Two things broke on 2026-09-27 that no browser gate could see, because
// the browser is not where they are drawn:
//
//   - **The left block went light under a dark page.** The block is the
//     macOS sidebar material, which paints in the window's appearance;
//     switching the theme set that appearance off the main thread, and
//     later a window's own appearance was found reset to nil with NSApp
//     never set (ebacbe2, c0be9a4).
//   - **The three window buttons sat 8 above the first row's centre**,
//     off the rail's glyph box: every appearance change makes AppKit lay
//     the title bar out again (560546c).
//
// So this gate runs the real binary, on a store of its own (`KHOR_HOME`
// in a temporary directory — never the owner's store, settings or
// sessions), and switches the theme the way the owner's other surfaces
// do: `khor config set theme …`, then bringing the window back to the
// front, which re-reads the store (`use-settings`). Nothing is pressed
// or typed in any window. At each step it screenshots the window and
// measures two things:
//
//   1. the close button's left edge is `traffic::X` and its centre is
//      the first row's centre (`traffic::PANE_HEAD` / 2), within 1.5;
//   2. the left block's ground and the board agree with the theme —
//      both dark in 深色, both light in 浅色.
//
// **And, since 2026-09-28, the half of 「像一个 Mac app」 that only the
// real WKWebView can answer** (the keys phase, `keysPhase` below): the
// menu bar khor builds (`src-tauri/src/menu.rs`) — its five menus, the
// Edit menu WKWebView needs for ⌘C/⌘V in a text field, the shortcuts it
// shows — and that a key pressed in the window runs its act **once**:
// the page takes it first (`src/lib/shortcuts.ts`) and the menu's own
// accelerator must not run it a second time. Counted on the scratch
// store's browsing tabs (`.khor/tabs.json`), which is a number a double
// press cannot hide in. Then the page's right-click: none of WebKit's own
// menu (Reload / Inspect Element) over the chrome, the system's over a
// text field. Last, ⌘W with no tab open must leave the window standing —
// the default menu's ⌘W was 「close window」, and closing khor's window
// stops the node it runs.
//
// **The keys phase presses keys**, which the theme phase never did. Each
// press is posted to the scratch app's pid (`native-probe key`), never to
// the system's input stream, so a press that arrives while somebody else's
// window is in front is dropped rather than typed into it — and the gate
// then reads it as a red, not as a keystroke in the owner's khor.
// `NATIVE_ONLY=keys` or `NATIVE_ONLY=theme` runs one phase.
//
// macOS only (everywhere else there is no material and no AppKit title
// bar). Needs the built binaries: `npm run build`, then
// `cargo build --profile local -p khor-gui -p khor-cli` (or the debug
// profile). It moves no pointer, but it does bring a window to the
// front for a few seconds and hands focus to Finder in between.
import { spawn, execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform !== "darwin") {
  console.log("native gate: not macOS, nothing AppKit draws here to measure");
  process.exit(0);
}

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "..", "..");
let bad = 0;
const fail = (m) => {
  console.error(m);
  bad += 1;
};
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const pick = (name) =>
  [join(repo, "target", "local", name), join(repo, "target", "debug", name)].find((p) => existsSync(p));
const guiBin = process.env.KHOR_GUI_BIN ?? pick("khor-gui");
const cliBin = process.env.KHOR_BIN ?? pick("khor");
if (!guiBin || !cliBin) {
  console.error("probe dead: no built khor-gui / khor under target/local or target/debug — build them first");
  process.exit(1);
}
const dist = join(gui, "dist", "index.html");
if (existsSync(dist) && statSync(dist).mtimeMs > statSync(guiBin).mtimeMs) {
  console.log(`note: ${guiBin} is older than dist/ — this measures the face the binary carries, not the newest build`);
}

// The numbers the window was built with, read where they live.
const traffic = readFileSync(join(gui, "src-tauri", "src", "traffic.rs"), "utf8");
const X = Number(traffic.match(/pub const X: f64 = ([\d.]+);/)?.[1]);
const HEAD = Number(traffic.match(/pub const PANE_HEAD: f64 = ([\d.]+);/)?.[1]);
if (!Number.isFinite(X) || !Number.isFinite(HEAD)) {
  console.error("probe dead: traffic.rs no longer spells X / PANE_HEAD the way this gate reads them");
  process.exit(1);
}

const scratch = mkdtempSync(join(tmpdir(), "khor-native-"));
const home = join(scratch, "khor");
const probe = join(scratch, "probe");
execFileSync("swiftc", ["-O", "-o", probe, join(gui, "scripts", "native-probe.swift")], { stdio: "inherit" });
const run = (...a) => execFileSync(probe, a, { encoding: "utf8" }).trim();
/** To the front. **System Events, not `NSRunningApplication.activate`**:
    macOS 14 refuses activation asked for by a background process, and
    the window stays behind with flat grey buttons (first run of this
    gate). */
const front = (pid) =>
  execFileSync("osascript", [
    "-e",
    `tell application "System Events" to set frontmost of (first process whose unix id is ${pid}) to true`,
  ]);
// A tmux server of its own too: without it the app finds the owner's
// tmux on the default socket and opens their live terminal sessions as
// tabs (found on the first run).
const env = { ...process.env, KHOR_HOME: home, TMUX_TMPDIR: scratch };
delete env.TMUX;
const setTheme = (t) => execFileSync(cliBin, ["config", "set", "theme", t], { env, encoding: "utf8" });

let child = null;
const done = () => {
  if (child && child.exitCode === null) child.kill("SIGTERM");
  if (process.env.KEEP_NATIVE) console.log(`kept ${scratch}`);
  else rmSync(scratch, { recursive: true, force: true });
};
process.on("exit", done);
process.on("SIGINT", () => process.exit(130));

/** What the desktop wears, for the `auto` step. */
const desktopDark = () => {
  try {
    return execFileSync("defaults", ["read", "-g", "AppleInterfaceStyle"], { encoding: "utf8" }).trim() === "Dark";
  } catch {
    return false; // the key is absent in light mode
  }
};

const lum = ([r, g, b]) => {
  const c = (v) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b);
};

// ── the keys phase ────────────────────────────────────────────────────

/** The words the menu is expected to say, read where they are written:
    the Chinese catalog's `[menu]` and `[gui]` (the scratch store is set to
    中文 before launch), as `section.key`. A second copy here would be the
    one that drifts. */
const words = (() => {
  const zh = readFileSync(join(repo, "crates", "catalog", "zh.toml"), "utf8");
  const out = {};
  let section = "";
  for (const line of zh.split("\n")) {
    const head = line.match(/^\[([a-z]+)\]$/);
    if (head) section = head[1];
    const m = line.match(/^([a-z0-9-]+) = "((?:[^"\\]|\\.)*)"$/);
    if (m) out[`${section}.${m[1]}`] = JSON.parse(`"${m[2]}"`);
  }
  return out;
})();
const said = (key) => {
  const at = key.includes(".") ? key : `menu.${key}`;
  if (!(at in words)) throw new Error(`probe dead: zh.toml has no ${at} — the gate reads the menu's words there`);
  return words[at];
};

/** The whole menu bar of one process, **read by pid** through the
    accessibility API (`native-probe menubar`): each menu's title, then
    each item's name, enabled, command key and modifier mask (AppKit's:
    0 is ⌘ alone, +1 ⇧, +2 ⌥, +4 ⌃, +8 no ⌘).

    ⚠️ **Not System Events.** Its `set p to first process whose unix id
    is …` keeps a reference *by process name*, and every khor-gui on this
    Mac is called khor-gui: the first run of this phase read the owner's
    running app's menus (English, tauri's default) and reported them as
    the scratch app's. A press through such a reference would land in the
    owner's app — the one thing this gate must never do. */
const menuBar = (pid) => {
  const out = JSON.parse(run("menubar", String(pid)));
  if (!Array.isArray(out)) throw new Error(`probe dead: no menu bar read for ${pid} (${JSON.stringify(out)})`);
  return out;
};
/** Presses one item of one menu, the way a person picks it — by pid. */
const choose = (pid, menu, item) => {
  const said = run("pick", String(pid), menu, item);
  if (said !== "pressed") throw new Error(`keys: 「${menu} › ${item}」 could not be pressed (${said})`);
};

/** macOS virtual key codes for the keys the phase presses. */
const KEY = { 1: 18, 2: 19, 3: 20, 4: 21, 5: 23, n: 45, w: 13, k: 40, l: 37, comma: 43, tab: 48, esc: 53, "]": 30, "[": 33 };

async function keysPhase(pid) {
  const press = async (key, ...mods) => {
    run("key", String(pid), String(KEY[key]), ...mods);
    await wait(250);
  };
  /** Waits for a condition, up to `ms`; true when it held. */
  const until = async (ms, ok) => {
    const end = Date.now() + ms;
    for (;;) {
      if (ok()) return true;
      if (Date.now() > end) return false;
      await wait(150);
    }
  };
  const alive = () => child.exitCode === null && run("window", String(pid)) !== "";

  // The store's browsing tabs — the number a double press would show in.
  const tabsAt = () => {
    const find = (dir) => {
      for (const e of readdirSync(dir, { withFileTypes: true })) {
        const p = join(dir, e.name);
        if (e.isFile() && e.name === "tabs.json") return p;
        if (e.isDirectory() && e.name !== "tmux") {
          const hit = find(p);
          if (hit) return hit;
        }
      }
      return null;
    };
    return find(home);
  };
  const pageTabs = () => {
    const at = tabsAt();
    if (!at) return { tabs: [], active: "" };
    try {
      const all = JSON.parse(readFileSync(at, "utf8"));
      const page = all.page ?? { tabs: [], active: "" };
      return { tabs: (page.tabs ?? []).map((t) => t.id), active: page.active ?? "" };
    } catch {
      return { tabs: [], active: "" };
    }
  };
  const menu = (title) => menuBar(pid).find((m) => m.title === title);
  const item = (title, name) => menu(title)?.items.find((i) => i.name === name);
  /** What ⌘N would make here, read off the menu: its first item. */
  const newItem = () => menu(said("file"))?.items[0]?.name ?? null;

  front(pid);
  await wait(1500);

  // ── 1. the menu bar, as a person reads it ──
  const bar = menuBar(pid);
  console.log(`keys: menu bar ${bar.map((m) => m.title).join(" · ")}`);
  for (const t of ["file", "edit", "view", "window"]) {
    if (!bar.some((m) => m.title === said(t))) fail(`keys: the menu bar has no 「${said(t)}」 menu`);
  }
  // The Edit menu is not decoration: without an item whose action is
  // `copy:` / `paste:`, WKWebView gets no ⌘C / ⌘V in a text field.
  for (const [k, ch] of [["undo", "Z"], ["redo", "Z"], ["cut", "X"], ["copy", "C"], ["paste", "V"], ["select-all", "A"]]) {
    const it = item(said("edit"), said(k));
    if (!it) fail(`keys: 「${said("edit")}」 has no 「${said(k)}」`);
    else if (it.key.toUpperCase() !== ch) fail(`keys: 「${said(k)}」 is on ${it.key || "no key"}, not ⌘${ch}`);
  }
  const shows = [
    [said("view"), said("gui.sessions-tab"), "1", 0],
    [said("view"), said("gui.devices-tab"), "2", 0],
    [said("view"), said("gui.workspaces-tab"), "3", 0],
    [said("view"), said("gui.accounts-tab"), "4", 0],
    [said("view"), said("gui.browser-tab"), "5", 0],
    [said("view"), said("find"), "K", 0],
    [said("file"), said("close-tab"), "W", 0],
    [said("file"), said("close-window"), "W", 1],
    [said("file"), said("import"), "", null],
  ];
  const app = bar.find((m) => m.items.some((i) => i.name === said("settings")));
  if (!app) fail(`keys: no menu carries 「${said("settings")}」`);
  else {
    shows.push([app.title, said("settings"), ",", 0], [app.title, said("lock"), "L", 0]);
    if (!app.items.some((i) => i.name === said("quit") && i.key.toUpperCase() === "Q")) {
      fail(`keys: 「${app.title}」 has no 「${said("quit")}」 on ⌘Q`);
    }
  }
  for (const [m, name, key, mods] of shows) {
    const it = item(m, name);
    if (!it) fail(`keys: 「${m}」 has no 「${name}」`);
    else if (it.key.toUpperCase() !== key || (mods !== null && it.mods !== mods)) {
      fail(`keys: 「${m} › ${name}」 shows ${it.key || "no key"} (mask ${it.mods}), not ${key} (mask ${mods})`);
    }
  }

  // ── 2. the page's right-click ──
  // Asked through accessibility (`native-probe axmenu`: AXShowMenu, which
  // WebKit answers by sending the page a real `contextmenu` at the
  // element). **Not a posted mouse click**: a secondary click posted to
  // a pid never reached the page (the first version of this read 0 menus
  // on the old code, where WebKit's menu was certainly there), and one
  // sent through the system's input stream lands on whatever window is
  // under that point — possibly the owner's.
  const menuOn = async (role) => {
    const [found, menus] = run("axmenu", String(pid), role).split(" ").map(Number);
    // Esc until it is gone: a menu left up would eat the next key pressed.
    for (let i = 0; i < 4 && Number(run("menus", String(pid))) > 0; i++) {
      await press("esc");
      await wait(300);
    }
    return { found: found === 1, menus };
  };
  // Chrome: the page itself (its middle is the board) and R0's title.
  for (const [role, what] of [["AXWebArea", "the page"], ["AXHeading", "R0's title"]]) {
    const m = await menuOn(role);
    console.log(`keys: right-click on ${what}: ${m.found ? `${m.menus} menu(s)` : "not found"}`);
    if (!m.found) fail(`probe dead: no ${role} in the scratch app's page to right-click`);
    else if (m.menus > 0) fail(`keys: right-click on ${what} brought up WebKit's own menu (Reload / Inspect Element)`);
  }
  // A text field keeps the system's menu (cut, copy, paste, spelling) —
  // the positive that says the reading above can see a menu at all.
  const field = await menuOn("AXTextField");
  console.log(`keys: right-click on the search box: ${field.found ? `${field.menus} menu(s)` : "not found"}`);
  if (!field.found) fail("probe dead: no text field in the scratch app's page to right-click");
  else if (field.menus === 0) fail("keys: right-click on the search box brought up no menu — a text field keeps the system's (cut, copy, paste)");

  // ── 3. a key runs its act once ──
  // Put focus back on the page: the search box took it on the right-click.
  await press("esc");
  await press("5", "cmd");
  const browsing = await until(4000, () => newItem() === said("gui.new-tab") && pageTabs().tabs.length > 0);
  console.log(`keys: ⌘5 → 「${newItem()}」 first in 「${said("file")}」, ${pageTabs().tabs.length} tab(s)`);
  if (!browsing) fail(`keys: ⌘5 did not open 「${said("gui.browser-tab")}」 (the menu's first item reads 「${newItem()}」, not 「${said("gui.new-tab")}」)`);
  const n0 = pageTabs().tabs.length;
  await press("n", "cmd");
  await wait(600);
  await press("n", "cmd");
  await until(4000, () => pageTabs().tabs.length >= n0 + 2);
  await wait(1200);
  const n1 = pageTabs().tabs.length;
  console.log(`keys: ⌘N twice → ${n0} → ${n1} tab(s)`);
  if (n1 !== n0 + 2) fail(`keys: ⌘N pressed twice made ${n1 - n0} tab(s) — ${n1 - n0 > 2 ? "the page and the menu both ran it" : "the press did not arrive"}`);
  // ⌃⇥ moves one tab, wrapping: twice would skip one.
  const before = pageTabs();
  await press("tab", "ctrl");
  const at = before.tabs.indexOf(before.active);
  const want = before.tabs[(at + 1) % before.tabs.length];
  await until(3000, () => pageTabs().active === want);
  await wait(600);
  console.log(`keys: ⌃⇥ → active ${before.active} → ${pageTabs().active} (want ${want})`);
  if (pageTabs().active !== want) fail(`keys: ⌃⇥ put ${pageTabs().active} in front, not the next tab ${want}`);
  await press("[", "cmd", "shift");
  await until(3000, () => pageTabs().active === before.active);
  if (pageTabs().active !== before.active) fail(`keys: ⇧⌘[ did not step back to ${before.active}`);
  const n2 = pageTabs().tabs.length;
  await press("w", "cmd");
  await until(4000, () => pageTabs().tabs.length <= n2 - 1 || !alive());
  await wait(1200);
  if (!alive()) throw new Error("keys: ⌘W on a browsing tab closed khor's window — the default menu's 「close window」 is still on ⌘W");
  const n3 = pageTabs().tabs.length;
  console.log(`keys: ⌘W → ${n2} → ${n3} tab(s)`);
  if (n3 !== n2 - 1) fail(`keys: ⌘W closed ${n2 - n3} tab(s), not one`);
  // The menu's own road: picked in the menu bar, it runs once too.
  choose(pid, said("file"), said("gui.new-tab"));
  await until(4000, () => pageTabs().tabs.length >= n3 + 1);
  await wait(1200);
  const n4 = pageTabs().tabs.length;
  console.log(`keys: 「${said("file")} › ${said("gui.new-tab")}」 picked → ${n3} → ${n4} tab(s)`);
  if (n4 !== n3 + 1) fail(`keys: picking 「${said("gui.new-tab")}」 in the menu bar made ${n4 - n3} tab(s), not one`);

  // ── 4. ⌘W with nothing to close leaves the window standing ──
  await press("1", "cmd");
  const sessions = await until(4000, () => newItem() === said("gui.new-session-ask"));
  console.log(`keys: ⌘1 → 「${newItem()}」 first in 「${said("file")}」`);
  if (!sessions) fail(`keys: ⌘1 did not open 「${said("gui.sessions-tab")}」 (the menu's first item reads 「${newItem()}」)`);
  const closeTab = item(said("file"), said("close-tab"));
  if (closeTab?.enabled) fail(`keys: 「${said("close-tab")}」 is enabled with no session tab open`);
  await press("w", "cmd");
  await wait(2000);
  if (!alive()) fail("keys: ⌘W with no tab open closed khor's window — that stops the node it runs");
  else console.log("keys: ⌘W with no tab open: the window stays");
}

try {
  execFileSync(cliBin, ["config", "set", "lang", "zh"], { env, encoding: "utf8" });
  setTheme("dark");
  child = spawn(guiBin, [], { env, stdio: "ignore" });
  let win = null;
  for (let i = 0; i < 60 && !win; i++) {
    await wait(500);
    const w = run("window", String(child.pid));
    if (w) win = w.split(" ").map(Number);
  }
  if (!win) throw new Error("probe dead: the app opened no window within 30 s");
  const [id, , , w, h] = win;
  console.log(`window ${id}: ${w}×${h}`);

  /** Front, settle, shoot, judge. */
  const step = async (theme, how) => {
    execFileSync("osascript", ["-e", 'tell application "Finder" to activate']);
    await wait(900);
    front(child.pid);
    await wait(1800);
    const png = join(scratch, `${theme}-${how}.png`);
    execFileSync("screencapture", ["-x", "-o", "-l", String(id), png]);
    // The left block's blank rail (below the landings, above 「…」) and
    // the board's lower right corner.
    const railPt = `32,${Math.max(330, h - 200)}`;
    const boardPt = `${w - 40},${h - 40}`;
    const m = JSON.parse(run("measure", png, String(w), railPt, boardPt));
    if (m.error) throw new Error(`probe dead: could not read ${png}`);
    const where = `${theme} (${how})`;
    if (!m.close) {
      fail(`${where}: no close button found in the window's corner — nothing to measure`);
    } else {
      const [left, right, top, bottom] = m.close;
      const centre = (top + bottom) / 2;
      console.log(`${where}: close button x ${left.toFixed(1)}–${right.toFixed(1)}, centre y ${centre.toFixed(1)}`);
      if (Math.abs(left - X) > 1.5) {
        fail(`${where}: the close button starts at x ${left.toFixed(1)}, not the rail's glyph box at ${X} (traffic.rs X)`);
      }
      if (Math.abs(centre - HEAD / 2) > 1.5) {
        fail(`${where}: the window buttons are centred at y ${centre.toFixed(1)}, not the first row's ${HEAD / 2} — AppKit put them back after an appearance change (traffic::place)`);
      }
    }
    const [rail, board] = m.samples;
    const dark = theme === "dark" || (theme === "auto" && desktopDark());
    console.log(`${where}: left block rgb(${rail}) L ${lum(rail).toFixed(2)}, board rgb(${board}) L ${lum(board).toFixed(2)}`);
    for (const [name, c] of [["left block", rail], ["board", board]]) {
      const L = lum(c);
      if (dark ? L > 0.25 : L < 0.45) {
        fail(`${where}: the ${name} is rgb(${c}) (L ${L.toFixed(2)}) — ${dark ? "light under a dark theme" : "dark under a light theme"}`);
      }
    }
  };

  const only = process.env.NATIVE_ONLY ?? "";
  if (only !== "keys") {
    await step("dark", "at launch");
    setTheme("light");
    await step("light", "after khor config + refocus");
    setTheme("dark");
    await step("dark", "switched back");
    setTheme("auto");
    await step("auto", "follows the desktop");
  }
  if (only !== "theme") await keysPhase(child.pid);
} catch (e) {
  fail(String(e?.message ?? e));
}

if (!bad) {
  console.log(
    "native gate: the window buttons sit on the first row and the left block wears the theme, launched and switched; " +
      "the menu bar is khor's, a key runs its act once, the chrome has no WebKit menu and ⌘W never closes the window",
  );
}
// **Exit on purpose**: the app this gate started keeps the event loop
// alive, and `done` (which stops it) only runs on exit — without this the
// gate finishes its checks and then waits for ever.
process.exit(bad ? 1 : 0);
