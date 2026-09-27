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
// macOS only (everywhere else there is no material and no AppKit title
// bar). Needs the built binaries: `npm run build`, then
// `cargo build --profile local -p khor-gui -p khor-cli` (or the debug
// profile). It moves no pointer, but it does bring a window to the
// front for a few seconds and hands focus to Finder in between.
import { spawn, execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
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

try {
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

  await step("dark", "at launch");
  setTheme("light");
  await step("light", "after khor config + refocus");
  setTheme("dark");
  await step("dark", "switched back");
  setTheme("auto");
  await step("auto", "follows the desktop");
} catch (e) {
  fail(String(e?.message ?? e));
}

if (!bad) {
  console.log("native gate: the window buttons sit on the first row and the left block wears the theme, launched and switched");
}
// **Exit on purpose**: the app this gate started keeps the event loop
// alive, and `done` (which stops it) only runs on exit — without this the
// gate finishes its checks and then waits for ever.
process.exit(bad ? 1 : 0);
