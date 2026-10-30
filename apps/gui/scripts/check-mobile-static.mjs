// Source contracts run on any host. This module never spawns a compiler.
import { readFileSync } from "node:fs";
import { join } from "node:path";

export function checkMobileSources(gui) {
  const repo = join(gui, "..", "..");
  const errors = [];
  const fail = message => errors.push(message);
  const probeDead = message => fail(`probe dead: ${message}`);
  // ── 2) the entry point ─────────────────────────────────────────────
  const lib = readFileSync(join(gui, "src-tauri", "src", "lib.rs"), "utf8");
  const admitted = new Set();
  if (lib.includes("native_io::dispatch(invoke)")) {
    const native = readFileSync(join(gui, "src-tauri", "src", "native_io.rs"), "utf8");
    const table = /const COMMANDS:\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/.exec(native)?.[1];
    if (table === undefined || !/COMMANDS\s*\.iter\(\)/.test(native)) {
      probeDead("ordered native dispatcher must use its COMMANDS table for admission");
    } else {
      for (const entry of table.split(",").map(s => s.trim()).filter(Boolean)) {
        const name = /^"([a-z_0-9]+)"$/.exec(entry)?.[1];
        if (!name || admitted.has(name)) probeDead(`invalid or duplicate native command: ${entry}`);
        else admitted.add(name);
      }
      if (admitted.size === 0) probeDead("ordered native command table is empty");
      if (!native.includes("respond_async")) probeDead("ordered native commands lost asynchronous completion");
    }
  }
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

  // ── 3c) a plain-http page loads, as in Safari ──────────────────────
  // **App Transport Security refused every `http://` page by name** (UX-38
  // §6, measured on the simulator: 「被拦截」, NSURLErrorDomain -1022, for
  // any host that is not an IP address or `localhost`). A browser opens the
  // web as it is; the exception is for web content only
  // (`NSAllowsArbitraryLoadsInWebContent`), khor's own connections keep ATS.
  {
    const plist = readFileSync(join(gui, "src-tauri", "gen", "apple", "khor-gui_iOS", "Info.plist"), "utf8");
    if (!/<key>NSAppTransportSecurity<\/key>\s*<dict>[\s\S]*?<key>NSAllowsArbitraryLoadsInWebContent<\/key>\s*<true\/>[\s\S]*?<\/dict>/.test(plist)) {
      fail("src-tauri/gen/apple/khor-gui_iOS/Info.plist does not allow web content over plain http (NSAppTransportSecurity › NSAllowsArbitraryLoadsInWebContent) — every http:// page by name says 被拦截");
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
    "workspaces", "tabs_list", "tabs_open", "tabs_go", "tabs_focus",
    "tabs_close", "tabs_pin", "tabs_save", "tabs_reorder", "ws_rm", "ws_pin", "pin_account", "rename_account", "seen_snapshot",
    "update_account", "register_account", "remove_account", "seen", "rename_session",
    "rename_machine", "relays", "set_relays", "pin_session", "pin_device", "forget_device",
    "set_watch", "set_receive_dir", "face_choices", "restyle", "recents", "recent_visit", "forget_recent", "find",
    "machine_log", "import_candidates", "import_sessions", "agents",
    "agents_known", "agents_known_taken", "register_known_agent", "agent_seen", "invite",
    "pending_list", "root_words", "set_paused", "set_machine_paused", "config", "set_config",
    "about", "web_key", "rotate_web_key", "reset_identity", "page_icons",
    // This machine's own settings files.
    "hooks_state", "install_hooks", "uninstall_hooks", "lock_state", "set_lock", "set_lock_pin",
    "set_lock_timeout", "unlock", "doctor_fix",
    // Attachment reads and leave only touch memory or interrupt a socket;
    // ordered writes are admitted by native_io and complete asynchronously.
    "attachment_begin", "chat_poll", "chat_leave", "term_attach", "term_stream", "term_leave", "term_drop_progress",
    // Answers the build knows, or plain memory.
    "console_catalog", // static command descriptors; no node/network work
    "merge_text", "trash_here", "pdf_here", "scanner_here", "traffic_room", "menu_rows",
    "reveal_here", "file_handoff_supported", "file_preview_supported",
    // The system's: windows, menus, the theme, a link handed to the OS.
    "set_window_theme", "set_window_appearance", "window_appearance_support", "menu_state", "open_link", "size_menu", "menu_pick",
    "hide_menu", "widen", "unwiden", "move_page", "hide_page", "show_page", "close_page",
    // The phone's native deck over its page, and the ground under its folded capsule (UIKit, queued).
    "page_deck", "page_sheet", "page_ground", "page_state",
    "window_minimize", "window_maximize_toggle", "window_close", "name_settings",
    // A line to the app's log; a notification handed to the OS; a haptic
    // handed to UIKit's thread (`bell_haptic` queues it and returns).
    "face_note", "notify_finished", "bell_haptic", "joy_haptic", "haptic",
    // What the next edit menu over a selection adds (item 54): memory.
    "edit_menu",
    // The system's edit menu at a selection khor drew (item 73): queued on UIKit's thread.
    "edit_menu_at", "edit_menu_hide",
    // 添加机器 / 安装 khor (`khor_gui_core::add`): a start registers the run
    // in memory and hands the ssh work to a thread of its own; a read is
    // that memory. The ssh config's Host names: one local file, read only.
    "add_start", "promote_start", "add_read", "ssh_config_hosts",
    // Handoff workers own the filesystem and network work; these only register/read/cancel jobs.
    "file_handoff_start", "file_handoff_state", "file_handoff_cancel",
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
    // 2026-10-05: the session list read every document and swept the
    // process table on the main thread, every 2 s (the owner's Mac app at
    // 70–260 % CPU, typing that lagged).
    "sessions",
    // 2026-10-11: and the day strips were carrying the same read on the
    // main thread from their own 10 s beat — a strip asks `sessions` for
    // its own snapshot by design (`khor_node::Node::day_strips`). Measured
    // at 31 ms held on a scratch app, with key→echo max 35 ms in the same
    // window against a p95 of 12 ms.
    "day_strips", "session_content", "history_begin", "history_cancel", "history_page", "history_record", "history_projection",
    "dirs_begin", "dirs_wait", "dirs_leave",
  ];
  {
    const decl = /#\[tauri::command(\([^)]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(async\s+)?fn\s+([a-z_0-9]+)/g;
    const sync = new Set();
    const all = new Set();
    for (const m of lib.matchAll(decl)) {
      all.add(m[3]);
      if (!m[2] && !/async/.test(m[1] ?? "")) sync.add(m[3]);
    }
    for (const name of admitted) {
      if (all.has(name)) fail(`ordered native command \`${name}\` is also declared as a generated command`);
      all.add(name);
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

  return errors;
}
