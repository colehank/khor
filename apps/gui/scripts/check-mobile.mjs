// The phone gate: the app crate still compiles for iOS.
//
// **Why this exists.** The shell has desktop halves and mobile halves of
// its commands (`src-tauri/src/lib.rs`, `#[cfg(desktop)]` /
// `#[cfg(not(desktop))]`), and nothing ever built the mobile half. By
// 2026-10-02 it had rotted without anybody noticing: four commands
// (`traffic_room`, `window_minimize`, `window_maximize_toggle`,
// `window_close`) were desktop-only but registered unconditionally, and
// `run` called `menu::speak_at_launch()` from a module that exists on
// desktop alone — nine errors under `cargo check --target
// aarch64-apple-ios -p khor-gui`, every dependency fine. A desktop build
// cannot see any of it: `desktop` is what tauri-build sets there, so the
// other half is never type-checked.
//
// Claims:
//   1. `cargo check --target aarch64-apple-ios -p khor-gui` passes. One
//      target is enough: the simulator triple has the same `mobile` /
//      `target_os = "ios"` cfgs, and the shell branches on nothing finer.
//   2. `run` still carries `tauri::mobile_entry_point`. Without it the
//      library type-checks and links, and the Xcode project's `start_app`
//      has nothing to call — a phone app that opens onto nothing.
//
// **What it cannot block:** anything that compiles and misbehaves on the
// phone — a command that answers `Ok` for a thing a phone cannot do, a
// path under `$HOME` the sandbox refuses. That is read in the simulator
// (docs/IOS.md), not here.
//
// macOS only (the iOS SDK is Xcode's). Needs the iOS target
// (`rustup target add aarch64-apple-ios`) and a built frontend — the web
// face's crate embeds `dist` (`npm run build`). Takes the cargo lock, and
// the first run compiles every dependency for iOS once (minutes); after
// that it is the shell alone.
//
// Run: `npm run mobile`.
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform !== "darwin") {
  console.log("mobile gate: not macOS, there is no iOS SDK here to check against");
  process.exit(0);
}

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "..", "..");
const TARGET = "aarch64-apple-ios";
let bad = 0;
let dead = false;
const fail = (m) => {
  console.error(m);
  bad += 1;
};
const probeDead = (m) => {
  fail(`probe dead: ${m}`);
  dead = true;
};

// ── probes: a missing prerequisite is a red with its fix, not a pass ──
const installed = spawnSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" });
if (installed.status !== 0) {
  probeDead(`\`rustup target list --installed\` did not answer (${installed.stderr || installed.error})`);
} else if (!installed.stdout.split("\n").includes(TARGET)) {
  probeDead(`the ${TARGET} target is not installed — \`rustup target add ${TARGET}\``);
}
if (!existsSync(join(gui, "dist", "index.html"))) {
  probeDead("apps/gui/dist/index.html is missing, and khor-web embeds it — `npm run build` first");
}

// ── 2) the entry point ─────────────────────────────────────────────
const lib = readFileSync(join(gui, "src-tauri", "src", "lib.rs"), "utf8");
if (!/#\[cfg_attr\(mobile,\s*tauri::mobile_entry_point\)\]\s*pub fn run\(\)/.test(lib)) {
  fail(
    "no entry point: `pub fn run()` in src-tauri/src/lib.rs lost " +
      "`#[cfg_attr(mobile, tauri::mobile_entry_point)]` — the iOS app would link and never start",
  );
}

// ── 1) the type-check — skipped only when a probe says it cannot run ─
if (!dead) {
  const check = spawnSync("cargo", ["check", "--target", TARGET, "-p", "khor-gui"], {
    cwd: repo,
    stdio: "inherit",
  });
  if (check.status !== 0) {
    fail(
      `khor-gui does not compile for ${TARGET}. Most often a command or module gated ` +
        "`#[cfg(desktop)]` that `run` or `generate_handler!` still names unconditionally: give it a " +
        "`#[cfg(not(desktop))]` half beside it — an honest no-op where the phone's answer really is " +
        "「nothing to do」, a refusal in words from `khor_catalog::msg` where the phone cannot do it.",
    );
  }
}

if (bad) process.exit(1);
console.log(`mobile: khor-gui type-checks for ${TARGET}, and run is the mobile entry point`);
