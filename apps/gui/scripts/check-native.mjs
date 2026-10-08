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
// The last theme step relaunches the app on the theme **opposite to the
// desktop's** (2026-10-01): a window that opened that way kept the
// desktop's sidebar material under a page in the other theme.
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
// The same phase reads **the menu bar's name and language**
// (`appKitSpeaks`): the app menu's bold title is 「khor」, not the process
// name, and no item under 编辑 / 窗口 — AppKit's own included — is in the
// other language; then the app is relaunched with `lang en` and read
// again, so the English case is held on a Chinese desktop too. Red on the
// code before `src-tauri/Info.plist`: 「khor-gui」 and eleven English items.
//
// **The keys phase presses keys**, which the theme phase never did. Each
// press is posted to the scratch app's pid (`native-probe key`), never to
// the system's input stream, so a press that arrives while somebody else's
// window is in front is dropped rather than typed into it — and the gate
// then reads it as a red, not as a keystroke in the owner's khor.
// `NATIVE_ONLY=keys` or `NATIVE_ONLY=theme` runs one phase; `NATIVE_ONLY=room`
// runs the window's room alone (UX-46, `roomPhase`), which the others skip.
//
// macOS only (everywhere else there is no material and no AppKit title
// bar). Needs the built binaries: `npm run build`, then
// `cargo build --profile local -p khor-gui -p khor-cli` (or the debug
// profile). It moves no pointer, but it does bring a window to the
// front for a few seconds and hands focus to Finder in between.
import { spawn, execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
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

// **The newer of the two, not the first that exists** (2026-09-28): the
// owner's installed app lives at target/local, so 「local first」 measured
// yesterday's binary whenever today's was built under debug — a whole
// run red about code that was no longer the code, with only a note.
const pick = (name) =>
  [join(repo, "target", "local", name), join(repo, "target", "debug", name)]
    .filter((p) => existsSync(p))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
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
// Off the segment, like every scratch node (`fake-home.mjs`).
const env = { ...process.env, KHOR_HOME: home, KHOR_NO_MDNS: "1", TMUX_TMPDIR: scratch };
delete env.TMUX;
const setTheme = (t) => execFileSync(cliBin, ["config", "set", "theme", t], { env, encoding: "utf8" });

/** The page store this run's `KHOR_HOME` gets (src-tauri `lib.rs`: the
    FNV-1a pair over the path's bytes, little-endian, as the WebKit data
    store's UUID). Named exactly, so the cleanup takes this run's store
    and never a neighbour's — another gate may be up on another scratch
    home at the same time. It used to be left behind, one per run. */
const pageStore = (() => {
  const fnv = (seed) => {
    let h = seed;
    for (const b of Buffer.from(home)) h = ((h ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn;
    return h;
  };
  const bytes = Buffer.alloc(16);
  bytes.writeBigUInt64LE(fnv(0xcbf29ce484222325n), 0);
  bytes.writeBigUInt64LE(fnv(0x6c62272e07bb0142n), 8);
  const x = bytes.toString("hex");
  const uuid = `${x.slice(0, 8)}-${x.slice(8, 12)}-${x.slice(12, 16)}-${x.slice(16, 20)}-${x.slice(20)}`;
  return join(homedir(), "Library", "WebKit", "khor-gui", "WebsiteDataStore", uuid);
})();

let child = null;
const done = () => {
  if (child && child.exitCode === null) child.kill("SIGTERM");
  if (process.env.KEEP_NATIVE) console.log(`kept ${scratch} and ${pageStore}`);
  else {
    rmSync(scratch, { recursive: true, force: true });
    rmSync(pageStore, { recursive: true, force: true });
  }
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
    a catalog's `[menu]` and `[gui]`, as `section.key`. Chinese is the one
    the scratch store is set to before launch; English is the relaunch at
    the end of the keys phase. A second copy here would be the one that
    drifts. */
const catalog = (lang) => {
  const text = readFileSync(join(repo, "crates", "catalog", `${lang}.toml`), "utf8");
  const out = {};
  let section = "";
  for (const line of text.split("\n")) {
    const head = line.match(/^\[([a-z]+)\]$/);
    if (head) section = head[1];
    const m = line.match(/^([a-z0-9-]+) = "((?:[^"\\]|\\.)*)"$/);
    if (m) out[`${section}.${m[1]}`] = JSON.parse(`"${m[2]}"`);
  }
  return out;
};
const words = catalog("zh");
const wordsEn = catalog("en");
/** **The strip's order, as the page ships it** (`src/lib/nav-order.ts`
    `NAV_DEFAULT`): ⌘1…⌘5 are the strip's seats, not fixed landings
    (docs/design-system/guidelines/30-behavior.md 键盘), and the scratch
    store has no order of its own, so the default is the one on screen.
    Read where it is written: this gate expected 会话 on ⌘1 for a round
    after the redesign put 机器 first. */
const navDefault = (() => {
  const text = readFileSync(join(repo, "apps", "gui", "src", "lib", "nav-order.ts"), "utf8");
  const m = text.match(/export const NAV_DEFAULT: NavKey\[\] = \[([^\]]*)\]/);
  if (!m) throw new Error("probe dead: nav-order.ts has no NAV_DEFAULT — the gate reads the strip's order there");
  return [...m[1].matchAll(/"([a-z]+)"/g)].map((x) => x[1]);
})();
/** Each landing's name on the View menu (`menu.rs` `name`). */
const LANDING_WORD = {
  sessions: "gui.sessions-tab",
  devices: "gui.devices-tab",
  workspaces: "gui.workspaces-tab",
  quota: "gui.accounts-tab",
  browser: "gui.browser-tab",
};
/** The digit ⌘ takes to reach a landing: its seat on the strip. */
const seat = (landing) => {
  const i = navDefault.indexOf(landing);
  if (i < 0) throw new Error(`probe dead: NAV_DEFAULT has no ${landing}`);
  return String(i + 1);
};
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
/** **The whole menu bar speaks one language, under khor's name**
    (2026-09-28). The owner runs the bare executable, not a `.app`, and a
    process with no bundle gets two things from AppKit that khor never
    wrote: the app menu's bold title is the process name (「khor-gui」),
    and the items AppKit adds by itself — 自动填充 / 开始听写… / 表情与符号
    under 编辑, 填充 / 居中 / 移动与调整大小 … under 窗口 — come out in
    English, because a process that declares no localization is spoken to
    in the development language. `src-tauri/Info.plist`, embedded in the
    binary, names the app and declares both; `menu::speak_at_launch` puts
    khor's own language first for AppKit when the system says another.

    The app menu is the one carrying 设置…; its accessibility title is the
    name AppKit draws in bold (not the submenu's own title, which AppKit
    ignores — menu.rs says 「khor」 there and the bar said 「khor-gui」).
    「Foreign」 is any Latin letter in a Chinese menu, any Han character in
    an English one, after taking out 「khor」 itself — the name, and the
    window's own title in the window list. */
const appKitSpeaks = (bar, lang) => {
  const w = (k) => (lang === "zh" ? words : wordsEn)[`menu.${k}`];
  const app = bar.find((m) => m.items.some((i) => i.name === w("settings")));
  console.log(`${lang}: app menu 「${app?.title ?? "(none)"}」: ${app?.items.map((i) => i.name || "—").join(" · ")}`);
  if (!app) fail(`${lang}: no menu carries 「${w("settings")}」 — the menu bar is not khor's, or not in ${lang}`);
  else if (app.title !== "khor") {
    fail(`${lang}: the app menu's bold title is 「${app.title}」, not 「khor」 — the process name, because the binary declares no bundle name (src-tauri/Info.plist)`);
  }
  const foreign = lang === "zh" ? /[A-Za-z]/ : /[\u3400-\u9fff]/;
  for (const title of [w("edit"), w("window")]) {
    const m = bar.find((x) => x.title === title);
    if (!m) {
      fail(`${lang}: the menu bar has no 「${title}」 menu`);
      continue;
    }
    console.log(`${lang}: ${title}: ${m.items.map((i) => i.name || "—").join(" · ")}`);
    const off = m.items.filter((i) => foreign.test(i.name.replace(/khor/gi, "")));
    if (off.length) {
      fail(
        `${lang}: 「${title}」 has ${off.length} item(s) in another language — ${off.map((i) => `「${i.name}」`).join(" ")} ` +
          "(AppKit's own, spoken in a language the process did not declare or did not put first)",
      );
    }
  }
};
/** Presses one item of one menu, the way a person picks it — by pid. */
const choose = (pid, menu, item) => {
  const said = run("pick", String(pid), menu, item);
  if (said !== "pressed") throw new Error(`keys: 「${menu} › ${item}」 could not be pressed (${said})`);
};

/** macOS virtual key codes for the keys the phase presses. */
const KEY = { 1: 18, 2: 19, 3: 20, 4: 21, 5: 23, f: 3, n: 45, w: 13, k: 40, l: 37, comma: 43, tab: 48, esc: 53, "]": 30, "[": 33 };

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
  // The table in 30-behavior.md 键盘 is the truth; each row here is one
  // of its keys as the menu bar shows it.
  const shows = [
    ...navDefault.map((l, i) => [said("view"), said(LANDING_WORD[l]), String(i + 1), 0]),
    [said("view"), said("find"), "K", 0],
    [said("view"), said("hide-sidebar"), "S", 4],
    [said("view"), said("reload-term"), "R", 0],
    [said("view"), said("command-history"), ";", 1],
    [said("edit"), said("find-list"), "F", 0],
    [said("file"), said("new-tab"), "T", 0],
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
  // Under khor's name, and AppKit's own items in Chinese too.
  appKitSpeaks(bar, "zh");

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
  // **Opened first**: since the redesign the list's search is a circle
  // that becomes a box when pressed (渐进式披露), so the page has no field
  // until ⌘F (this column's search, 30-behavior.md 键盘) opens one.
  await press("f", "cmd");
  await wait(600);
  const field = await menuOn("AXTextField");
  console.log(`keys: right-click on the search box: ${field.found ? `${field.menus} menu(s)` : "not found"}`);
  if (!field.found) fail("probe dead: no text field in the scratch app's page to right-click");
  else if (field.menus === 0) fail("keys: right-click on the search box brought up no menu — a text field keeps the system's (cut, copy, paste)");

  // ── 3. a key runs its act once ──
  // Put focus back on the page: the search box took it on the right-click.
  await press("esc");
  await press(seat("browser"), "cmd");
  // The landing, not a tab: a fresh store's 浏览器 opens on its list
  // with no page yet (the redesign), and ⌘N below makes the first two.
  const browsing = await until(4000, () => newItem() === said("gui.new-tab"));
  console.log(`keys: ⌘${seat("browser")} → 「${newItem()}」 first in 「${said("file")}」, ${pageTabs().tabs.length} tab(s)`);
  if (!browsing) fail(`keys: ⌘${seat("browser")} did not open 「${said("gui.browser-tab")}」 (the menu's first item reads 「${newItem()}」, not 「${said("gui.new-tab")}」)`);
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
  await press(seat("sessions"), "cmd");
  const sessions = await until(4000, () => newItem() === said("gui.new-session-ask"));
  console.log(`keys: ⌘${seat("sessions")} → 「${newItem()}」 first in 「${said("file")}」`);
  if (!sessions) fail(`keys: ⌘${seat("sessions")} did not open 「${said("gui.sessions-tab")}」 (the menu's first item reads 「${newItem()}」)`);
  const closeTab = item(said("file"), said("close-tab"));
  if (closeTab?.enabled) fail(`keys: 「${said("close-tab")}」 is enabled with no session tab open`);
  await press("w", "cmd");
  await wait(2000);
  if (!alive()) fail("keys: ⌘W with no tab open closed khor's window — that stops the node it runs");
  else console.log("keys: ⌘W with no tab open: the window stays");
}

// ── the room phase (UX-46 §2) ─────────────────────────────────────────

/**
 * **The window comes back to its size, exactly** (UX-46, the owner
 * 2026-10-07: 「打开多大，关闭了不影响原来的大小是要做好的…本来的窗口还变
 * 了」). The session's 附栏 grows the window by its width (UX-39 §9) and
 * every road that closes it must put the frame back: the front tab
 * pressed again, another landing, the tab closed. A window the person
 * only moved while it was open keeps their move and gets its pressed
 * size back; one they resized keeps their size (the lead's ruling,
 * 2026-10-07); a window against the screen's right edge grows leftward and
 * comes back the same way. Read as the window's frame, by pid, before
 * and after each road (`native-probe window`, points); the presses are
 * accessibility's (`axpress`), posted to the scratch app, and the moves
 * and sizes are what a person dragging it does (`move`, `resize`).
 *
 * The session is a tmux session on the scratch server, which the scratch
 * node lists like any other. `NATIVE_ONLY=room` runs this phase alone.
 */
async function roomPhase(pid) {
  const P = String(pid);
  const tokens = readFileSync(join(gui, "src", "styles", "tokens.css"), "utf8");
  const col = Number(tokens.match(/--session-side-w: (\d+)px;/)?.[1]);
  if (!Number.isFinite(col)) throw new Error("probe dead: tokens.css no longer spells --session-side-w the way this gate reads it");
  const frame = () => {
    const [, x, y, w, h] = run("window", P).split(" ").map(Number);
    return { x, y, w, h };
  };
  const fmt = (f) => `${f.x},${f.y} ${f.w}×${f.h}`;
  const press = async (name, ...how) => {
    const ok = run("axpress", P, name, ...how) === "1";
    await wait(1500);
    return ok;
  };
  const moveTo = async (x, y) => {
    run("move", P, String(x), String(y));
    await wait(900);
  };
  const sizeTo = async (w, h) => {
    run("resize", P, String(w), String(h));
    await wait(900);
  };
  const [sx, sy, sw] = run("screen").split(" ").map(Number);
  const name = "room";
  // **Opened through khor, not tmux** (fix-c, 2026-10-08): the scratch
  // node lists the sessions khor made, and a bare `tmux new-session` on
  // its server was never a row — the other half of 「no row 「room」」.
  const opened = execFileSync(cliBin, ["open", "-d", "--title", name, "--", "cat"], { env, encoding: "utf8" }).trim();
  front(pid);
  // The row, listed by the scratch node's next poll; pressed, its tab.
  let listed = false;
  for (let i = 0; i < 30 && !listed; i++) {
    listed = run("axpress", P, name) === "1";
    if (!listed) await wait(1000);
  }
  if (!listed) throw new Error(`probe dead: room · no row 「${name}」 to open — the scratch node did not list the tmux session`);
  await wait(1500);
  const tab = () => press(name, "expanded");
  if (!(await tab())) throw new Error(`probe dead: room · no front tab 「${name}」 that opens the 附栏`);
  const near = (a, b) => Math.abs(a - b) <= 1;
  const same = (a, b) => near(a.x, b.x) && near(a.y, b.y) && near(a.w, b.w) && near(a.h, b.h);
  const home = async () => {
    await sizeTo(1080, 720);
    await moveTo(sx + 120, sy + 60);
  };
  // The press above opened it; a second closes it. Start from closed.
  await tab();
  await home();
  const rows = [];
  const road = async (what, close, want) => {
    const f0 = frame();
    if (!(await tab())) {
      fail(`room · ${what}: the front tab could not be pressed`);
      return;
    }
    const f1 = frame();
    const was = await close(f0, f1);
    const f2 = frame();
    // A road answers the frame it measures from, or nothing (a press's
    // true is not a frame).
    const expected = want(was && typeof was === "object" ? was : f0, f1);
    rows.push(`${what}: ${fmt(f0)} → open ${fmt(f1)} → closed ${fmt(f2)}`);
    console.log(`room · ${what}: before ${fmt(f0)}, open ${fmt(f1)}, closed ${fmt(f2)}${same(f2, expected) ? "" : ` — want ${fmt(expected)}`}`);
    if (!near(f1.w, f0.w + col) || !near(f1.h, f0.h) || !near(f1.y, f0.y)) {
      fail(`room · ${what}: opening made the window ${fmt(f1)} from ${fmt(f0)} — ${col} wider, nothing else (UX-39 §9)`);
    }
    if (!same(f2, expected)) fail(`room · ${what}: closed, the window is ${fmt(f2)}, want ${fmt(expected)} (UX-46 §2)`);
  };
  // 1. the front tab pressed again
  await road("the tab again", () => tab(), (f0) => f0);
  // 2. another landing, then back to the session (closed there)
  await road(
    "another landing",
    async () => {
      await press(said("gui.devices-tab"));
      await press(said("gui.sessions-tab"));
    },
    (f0) => f0,
  );
  // 3. the tab closed, then the session opened again from its row
  await road(
    "the tab closed",
    async () => {
      await press(said("gui.close"));
    },
    (f0) => f0,
  );
  await press(name);
  // 4. moved while open: their move kept, the column off
  await road(
    "moved while open",
    async (f0) => {
      await moveTo(f0.x + 60, f0.y + 40);
      await tab();
    },
    (f0) => ({ x: f0.x + 60, y: f0.y + 40, w: f0.w, h: f0.h }),
  );
  await home();
  // 5. resized while open: their size stands, the close leaves it alone
  await road(
    "resized while open",
    async (f0, f1) => {
      await sizeTo(f1.w + 100, f0.h - 60);
      await tab();
    },
    (f0, f1) => ({ x: f1.x, y: f1.y, w: f1.w + 100, h: f0.h - 60 }),
  );
  await home();
  // 6. against the screen's right edge: 100 to the right, the rest leftward
  await moveTo(sx + sw - 1080 - 100, sy + 60);
  {
    const f0 = frame();
    await tab();
    const f1 = frame();
    await tab();
    const f2 = frame();
    rows.push(`at the right edge: ${fmt(f0)} → open ${fmt(f1)} → closed ${fmt(f2)}`);
    console.log(`room · at the right edge: before ${fmt(f0)}, open ${fmt(f1)}, closed ${fmt(f2)}`);
    if (!near(f1.x + f1.w, sx + sw) || !near(f1.w, f0.w + col)) {
      fail(`room · at the right edge the window opened to ${fmt(f1)} from ${fmt(f0)} — to the screen's edge, the rest leftward (UX-46 §2)`);
    }
    if (!same(f2, f0)) fail(`room · at the right edge, closed, the window is ${fmt(f2)}, want ${fmt(f0)} (UX-46 §2)`);
  }
  await home();
  execFileSync(cliBin, ["close", opened.split(/\s+/).pop()], { env, stdio: "ignore" });
  return rows;
}

/** Launches the scratch app and waits for its window; the window's
    `[id, x, y, w, h]`. */
const launch = async () => {
  child = spawn(guiBin, [], { env, stdio: "ignore" });
  for (let i = 0; i < 60; i++) {
    await wait(500);
    const w = run("window", String(child.pid));
    if (w) return w.split(" ").map(Number);
  }
  throw new Error("probe dead: the app opened no window within 30 s");
};

/** **The menu follows khor's language, not only the system's**: the
    scratch app again, relaunched after `khor config set lang en`. On a
    Chinese desktop (this machine's) this is the case the Info.plist alone
    cannot answer — AppKit would speak the system's Chinese under khor's
    English — and `menu::speak_at_launch` must put English first. Read
    only, by pid: nothing is pressed and the window is not brought to the
    front. */
/** **The Mac face at its tokens' size on a fresh store** (UX-9): 界面字号
    follows the system by default, and it divided the Mac's 13 by iOS's 17,
    so every Mac came up at 76 % — row names at 9.9 px, the list's title at
    16.8. Measured where it is drawn: the list's title (R0's heading, 会话
    on a fresh store), whose text box accessibility reports. Not a row
    name: a row is a button, and accessibility folds its text into the
    button's title with no box of its own; the scale is one number for the
    whole ramp, so the title answers for the rows.

    **The box is the glyphs', not the line's**: WebKit reports a static
    text's ascent-to-descent run — 26 pt for the 22 px title, 21 for the
    16.8 px one it was. So the check is a band read off tokens.css: at its
    tokens' size the title's box is at least `--fs-title-lg` and at most
    `--lh-title-lg`; at 76 % it is below the font size. The scratch run is
    at the system's default text size. */
function typeAtLaunch(pid) {
  const tokens = readFileSync(join(gui, "src", "styles", "tokens.css"), "utf8");
  const px = (name) => Number(tokens.match(new RegExp(`--${name}: calc\\((\\d+)px \\* var\\(--ui-scale\\)\\)`))?.[1]);
  const [fs, lh] = [px("fs-title-lg"), px("lh-title-lg")];
  if (!Number.isFinite(fs) || !Number.isFinite(lh)) {
    throw new Error("probe dead: tokens.css no longer spells --fs-title-lg / --lh-title-lg the way this gate reads them");
  }
  const title = said("gui.sessions-tab");
  const frames = JSON.parse(run("axtext", String(pid), title));
  if (!frames.length) {
    fail(`type: no 「${title}」 title in the page — a fresh store opens on the sessions list`);
    return;
  }
  const h = Math.max(...frames.map((f) => f[3]));
  console.log(`type: the list title 「${title}」, text box ${h.toFixed(1)} pt (tokens: ${fs} px on a ${lh} px line)`);
  if (h < fs || h > lh) {
    fail(`type: the list title's text box is ${h.toFixed(1)} pt, outside ${fs}–${lh} — the Mac face is not at its tokens' size (lib/type-size.ts SYSTEM_BODY)`);
  }
}

async function langPhase() {
  const was = child;
  was.kill("SIGTERM");
  for (let i = 0; i < 40 && was.exitCode === null && was.signalCode === null; i++) await wait(250);
  if (was.exitCode === null && was.signalCode === null) was.kill("SIGKILL");
  execFileSync(cliBin, ["config", "set", "lang", "en"], { env, encoding: "utf8" });
  await launch();
  await wait(1500);
  appKitSpeaks(menuBar(child.pid), "en");
}

try {
  execFileSync(cliBin, ["config", "set", "lang", "zh"], { env, encoding: "utf8" });
  setTheme("dark");
  let [id, , , w, h] = await launch();
  console.log(`window ${id}: ${w}×${h}`);

  /** Front, settle, shoot, judge. */
  const step = async (theme, how, refocus = true) => {
    // Focus away and back — the page re-reads the store on coming to the
    // front. **Finder put in front by System Events, not asked to activate
    // itself** (2026-10-01): `tell application "Finder" to activate` is an
    // Apple Event Finder has to answer, and a Finder busy elsewhere (a
    // trash operation stuck the same morning) times it out (-1712) and the
    // whole gate with it; System Events answers for Finder.
    if (refocus) {
      execFileSync("osascript", [
        "-e",
        'tell application "System Events" to set frontmost of (first process whose name is "Finder") to true',
      ]);
      await wait(900);
    }
    front(child.pid);
    await wait(refocus ? 1800 : 3000);
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
  if (only === "room") {
    const rows = await roomPhase(child.pid);
    if (!bad) console.log(`native gate (room): the window came back on every road —\n  ${rows.join("\n  ")}`);
    process.exit(bad ? 1 : 0);
  }
  if (only !== "keys") {
    await step("dark", "at launch");
    typeAtLaunch(child.pid);
    setTheme("light");
    await step("light", "after khor config + refocus");
    setTheme("dark");
    await step("dark", "switched back");
    setTheme("auto");
    await step("auto", "follows the desktop");
    // **Resized from outside** (UX-4, measured on a scratch app: the trio's
    // centre at 15.5 after an accessibility resize, until 1f8289b6). The
    // green button and tiling set the size the same way, and AppKit lays
    // the title bar out again; narrow and back, the trio must stand on
    // the first row each time. Only the buttons are judged — the narrow
    // face has no left block to sample.
    for (const [rw, how] of [[560, "resized narrow"], [w, "resized back"]]) {
      if (run("resize", String(child.pid), String(rw), String(h)).trim() !== "0") {
        fail(`${how}: accessibility refused to resize the scratch window (System Settings › Privacy › Accessibility for the terminal)`);
        break;
      }
      await wait(1500);
      const png = join(scratch, `resize-${rw}.png`);
      execFileSync("screencapture", ["-x", "-o", "-l", String(id), png]);
      const m = JSON.parse(run("measure", png, String(rw)));
      const centre = m.close ? (m.close[2] + m.close[3]) / 2 : null;
      console.log(`${how} (${rw}): close button centre y ${centre === null ? "—" : centre.toFixed(1)}`);
      if (centre === null) fail(`${how}: no close button found in the window's corner`);
      else if (Math.abs(centre - HEAD / 2) > 1.5) {
        fail(`${how}: the window buttons are centred at y ${centre.toFixed(1)}, not the first row's ${HEAD / 2} — AppKit put them back after a resize (traffic::place on Resized)`);
      }
    }
    // **Launched against the desktop** (2026-10-01, the owner's bug lane:
    // two lanes saw the scratch app light with the left block dark — dark
    // material, dark ink). The switches above all start from a window
    // that already wore a theme; a window that *opens* on the store's
    // light while the desktop is dark (or the other way round) wore the
    // desktop's material until something made the window redraw it — a
    // trip to the background and back, or switching the theme twice by
    // hand. So this one is read **without** the trip the steps above
    // take: brought to the front once, the way a person opens it.
    // Measured before the fix: board rgb(255,255,255), left block
    // rgb(67,67,67).
    const against = desktopDark() ? "light" : "dark";
    child.kill("SIGTERM");
    for (let i = 0; i < 40 && child.exitCode === null && child.signalCode === null; i++) await wait(250);
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
    setTheme(against);
    [id, , , w, h] = await launch();
    await step(against, "launched against the desktop, never sent back", false);
  }
  if (only !== "theme") {
    await keysPhase(child.pid);
    await langPhase();
  }
} catch (e) {
  fail(String(e?.message ?? e));
}

if (!bad) {
  console.log(
    "native gate: the window buttons sit on the first row and the left block wears the theme, launched and switched; " +
      "the menu bar is khor's, under khor's name and in khor's language (AppKit's own items too, zh and en), " +
      "a key runs its act once, the chrome has no WebKit menu and ⌘W never closes the window",
  );
}
// **Exit on purpose**: the app this gate started keeps the event loop
// alive, and `done` (which stops it) only runs on exit — without this the
// gate finishes its checks and then waits for ever.
process.exit(bad ? 1 : 0);
