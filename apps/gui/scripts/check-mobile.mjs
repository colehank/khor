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
//   3. the camera can be asked for (加入网络's 扫码, docs/IOS.md 第三步):
//      the iOS `Info.plist` carries `NSCameraUsageDescription`, and it is
//      the catalog's `camera-usage`, word for word. Without the key the
//      scanner refuses before it asks (iOS would kill an app that asked
//      without it); a key that drifted from the catalog is a sentence the
//      catalog no longer owns (docs/UX.md 文案 — words live there).
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

// ── 3) the camera's sentence ───────────────────────────────────────
{
  const plist = readFileSync(join(gui, "src-tauri", "gen", "apple", "khor-gui_iOS", "Info.plist"), "utf8");
  const said = /<key>NSCameraUsageDescription<\/key>\s*<string>([^<]*)<\/string>/.exec(plist)?.[1];
  const zh = readFileSync(join(repo, "crates", "catalog", "zh.toml"), "utf8");
  const owned = /^camera-usage = "([^"]*)"$/m.exec(zh)?.[1];
  if (owned === undefined) {
    probeDead("crates/catalog/zh.toml has no `camera-usage` — the sentence the Info.plist must carry");
  } else if (said === undefined) {
    fail(
      "src-tauri/gen/apple/khor-gui_iOS/Info.plist has no NSCameraUsageDescription — the scanner " +
        "refuses before it asks, and 加入网络's 扫码 can never open the camera",
    );
  } else if (said !== owned) {
    fail(`Info.plist's NSCameraUsageDescription is 「${said}」, the catalog's camera-usage 「${owned}」 — one sentence, two copies that drifted`);
  }
}

// ── 3b) the local network's sentence ───────────────────────────────
// **Without `NSLocalNetworkUsageDescription` iOS never asks** — it drops
// UDP to a LAN address without a word, so a phone on the same Wi-Fi as
// the Mac reached it only through a relay, for ever (the owner,
// 2026-10-03). The key must be there, and it must be the catalog's
// `local-network-usage`, word for word (the Mac's Info.plist too).
{
  const zh = readFileSync(join(repo, "crates", "catalog", "zh.toml"), "utf8");
  const owned = /^local-network-usage = "([^"]*)"$/m.exec(zh)?.[1];
  if (owned === undefined) {
    probeDead("crates/catalog/zh.toml has no `local-network-usage` — the sentence the Info.plists must carry");
  }
  for (const rel of [["gen", "apple", "khor-gui_iOS", "Info.plist"], ["Info.plist"]]) {
    const plist = readFileSync(join(gui, "src-tauri", ...rel), "utf8");
    const said = /<key>NSLocalNetworkUsageDescription<\/key>\s*<string>([^<]*)<\/string>/.exec(plist)?.[1];
    if (said === undefined) {
      fail(`src-tauri/${rel.join("/")} has no NSLocalNetworkUsageDescription — LAN packets are dropped and every machine on the same Wi-Fi reads 中继`);
    } else if (owned !== undefined && said !== owned) {
      fail(`src-tauri/${rel.join("/")}'s NSLocalNetworkUsageDescription is 「${said}」, the catalog's local-network-usage 「${owned}」`);
    }
  }
}

// ── 4) nothing that waits runs on the main thread ──────────────────
// **The phone froze when a machine dropped off** (2026-10-02, the owner
// on a real iPhone). Tauri runs a command declared without `async` on
// the main thread — the one that draws the window and runs the page —
// and holds it there until the command returns. `term_open` reached the
// far machine from there, and an unreachable machine held the whole app
// still for the reach's deadline. So every sync command is on this list
// of ones that only touch memory, one local file or document, or AppKit
// (which wants the main thread); anything that waits on the network, a
// far machine, a process coming up or a cold disk read is `async` and
// goes through `off_main` in `src-tauri/src/lib.rs`.
//
// A ratchet, both ways: a sync command not on the list is red (it runs
// on the main thread and nobody said it is quick), and a name on the
// list that is no longer a sync command is red (delete the line). It is
// a static census — it reads the declaration, not what the body does —
// so the list is only as true as the person who adds to it; the
// simulator measures the rest (docs/IOS.md, handoff 001).
const QUICK = new Set([
  // Lists and documents on this machine's own disk.
  "sessions", "day_strips", "workspaces", "tabs_list", "tabs_open", "tabs_go", "tabs_focus",
  "tabs_close", "tabs_pin", "tabs_save", "ws_rm", "ws_pin", "pin_account", "rename_account",
  "update_account", "register_account", "remove_account", "seen", "rename_session",
  "rename_machine", "relays", "set_relays", "tell", "pin_session", "pin_device", "forget_device",
  "set_watch", "face_choices", "restyle", "recents", "recent_visit", "forget_recent", "find",
  "history", "history_follow", "machine_log", "import_candidates", "import_sessions", "agents",
  "agents_known", "agents_known_taken", "register_known_agent", "agent_seen", "invite",
  "pending_list", "root_words", "set_paused", "set_machine_paused", "config", "set_config",
  "about", "web_key", "rotate_web_key", "reset_identity", "page_icons",
  // This machine's own settings files.
  "hooks_state", "install_hooks", "uninstall_hooks", "lock_state", "set_lock", "set_lock_pin",
  "set_lock_timeout", "unlock", "doctor_fix",
  // An attachment already open: memory, and a write to its socket.
  "chat_poll", "chat_answer", "chat_stop", "chat_set_mode", "chat_set_option",
  "chat_replay", "chat_leave", "term_attach", "term_stream", "term_key", "term_resize", "term_leave",
  // Answers the build knows, or plain memory.
  "merge_text", "trash_here", "pdf_here", "scanner_here", "traffic_room", "menu_rows",
  "reveal_here",
  // The system's: windows, menus, the theme, a link handed to the OS.
  "set_window_theme", "menu_state", "open_link", "reveal", "show_menu", "size_menu", "menu_pick",
  "hide_menu", "widen", "move_page", "hide_page", "show_page", "close_page", "page_step",
  "window_minimize", "window_maximize_toggle", "window_close",
  // A line to the app's log; a notification handed to the OS; a haptic
  // handed to UIKit's thread (`bell_haptic` queues it and returns).
  "face_note", "notify_finished", "bell_haptic",
]);
// The ones that waited on the main thread on 2026-10-02, named so the
// red says which: each must be `async`.
const WAITS = [
  "term_open", "chat_open", "term_drop", "close_session", "usage", "agent_ask",
  "takeover", "takeover_term", "resume_session",
  // 2026-10-04: a turn carrying photos to a far machine held the main
  // thread for minutes — a write to its socket is quick only while the
  // road keeps up.
  "chat_say",
];
{
  const decl = /#\[tauri::command(\([^)]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(async\s+)?fn\s+([a-z_0-9]+)/g;
  const sync = new Set();
  const all = new Set();
  for (const m of lib.matchAll(decl)) {
    all.add(m[3]);
    if (!m[2] && !/async/.test(m[1] ?? "")) sync.add(m[3]);
  }
  if (all.size < 100) {
    probeDead(`read only ${all.size} commands out of src-tauri/src/lib.rs — the declaration pattern no longer matches`);
  }
  for (const name of WAITS) {
    if (!all.has(name)) probeDead(`no command \`${name}\` in src-tauri/src/lib.rs — renamed? update this gate`);
    else if (sync.has(name)) {
      fail(
        `\`${name}\` is a sync command, so tauri runs it on the main thread — and it waits (the network, a far ` +
          "machine, a process coming up). Make it `async` and run its body through `off_main`.",
      );
    }
  }
  for (const name of [...sync].sort()) {
    if (!QUICK.has(name) && !WAITS.includes(name)) {
      fail(
        `\`${name}\` is a sync command (runs on the main thread) and is not on check-mobile's QUICK list. If it only ` +
          "touches memory, a local file or AppKit, add it there; if it can wait, make it `async` through `off_main`.",
      );
    }
  }
  for (const name of [...QUICK].sort()) {
    if (!sync.has(name)) fail(`census stale: \`${name}\` is on QUICK but is not a sync command any more — delete the line`);
  }
}

// ── 5) the status bar wears the app's theme ─────────────────────────
// **The status bar's ink followed the phone, not the app** (2026-10-02,
// the owner). It is UIKit's: its default style is dark ink on a light
// window and light ink on a dark one, and the window followed the phone
// because `set_window_theme` did nothing on iOS. The iOS half has to set
// the windows' interface style. Static — what it looks like is read in
// the simulator (docs/IOS.md, `/Volumes/disk0/khor-wt-fix-shots/`).
{
  const body = /fn set_window_theme\([^)]*\)\s*\{([\s\S]*?)\n\}\n/.exec(lib)?.[1];
  if (body === undefined) {
    probeDead("no `fn set_window_theme` in src-tauri/src/lib.rs");
  } else if (!/#\[cfg\(target_os = "ios"\)\][\s\S]*wear_on_the_windows/.test(body)) {
    fail(
      "set_window_theme has no iOS half: the app's theme never reaches UIKit, so the status bar's ink follows the " +
        "phone — dark ink over a dark app on a light phone",
    );
  } else if (!/setOverrideUserInterfaceStyle/.test(lib)) {
    fail("wear_on_the_windows no longer sets overrideUserInterfaceStyle — the status bar would follow the phone again");
  }
}

// ── 6) a command reads its arguments by the names the face sends ────
// **Tauri reads a command's arguments as camelCase unless told
// otherwise** (tauri-macros: `ArgumentCase::Camel` by default). The face
// and the bridge speak snake_case for `only_mine` and `base_url`, so in
// the app `set_relays` refused for a missing argument and the account
// commands dropped `base_url` without a word (round NET-5). A census, both
// ways: a command with a multi-word argument is either declared
// `rename_all = "snake_case"`, or every call the face makes to it with a
// literal object uses camelCase keys.
{
  const api = readFileSync(join(gui, "src", "api.ts"), "utf8");
  const decl = /#\[tauri::command(\([^)]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+([a-z_0-9]+)\s*\(([^)]*)\)/g;
  for (const m of lib.matchAll(decl)) {
    const [, attr = "", name, args] = m;
    const multi = args
      .split(",")
      .map((a) => a.split(":")[0].trim())
      .filter((a) => a.includes("_") && !a.startsWith("_"));
    if (multi.length === 0) continue;
    const snake = /rename_all\s*=\s*"snake_case"/.test(attr);
    const literal = new RegExp(`call<[^>]*>\\("${name}",\\s*\\{([^}]*)\\}`).exec(api)?.[1] ?? "";
    for (const arg of multi) {
      const camel = arg.replace(/_([a-z])/g, (_, c) => c.toUpperCase());
      const keyed = (k) => new RegExp(`(^|[\\s,{])${k}\\s*[:,}]`).test(literal);
      if (!snake && keyed(arg)) {
        fail(
          `\`${name}\` takes \`${arg}\` and the face sends it as \`${arg}\`, but tauri reads \`${camel}\`: ` +
            'declare it `#[tauri::command(rename_all = "snake_case")]` (the bridge reads snake_case too)',
        );
      }
      if (snake && keyed(camel)) {
        fail(`\`${name}\` is declared snake_case and the face sends \`${camel}\`: send \`${arg}\``);
      }
    }
  }
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
console.log(
  `mobile: khor-gui type-checks for ${TARGET}, run is the mobile entry point, the camera's and the local network's sentences are the catalog's, ` +
    "every sync command is a quick one (nothing that waits runs on the main thread), the iOS windows wear the app's theme, " +
    "and every command reads its arguments by the names the face sends",
);
