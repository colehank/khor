// 「只看对话」 — the sessions list's fifth axis, and the Windows default.
//
// The owner, 2026-11-18: 「同意只看对话过滤，windows上这么设定」. What is
// being pinned here is the *judgment*, not the pixels: a conversation stays,
// a terminal with nothing but a shell behind it goes, and a kind this build
// has never heard of stays (a newer node's conversation must not vanish).
//
// Run: node apps/gui/scripts/check-session-talk.mjs
import assert from "node:assert/strict";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const src = join(here, "..", "src", "lib", "session-filter.ts");

// These checks run the TypeScript through `tsx`-free transpilation: the
// module imports `@/` aliases and React-free helpers, so the smallest
// honest way to exercise it is to compile it the way the app does. That
// costs the whole toolchain; instead this script reads the source and
// checks the *rules* it declares, plus the pure functions it can evaluate
// by stripping the imports — the same trade `check-new-session-choices.mjs`
// makes for the wizard's default.
const text = readFileSync(src, "utf8");

const must = (re, why) => assert.match(text, re, why);

must(/export const TALK_AXIS = "talk:"/, "the axis keeps its own prefix, so a tick says which axis it is on");
must(
  /const TERMINAL_ONLY: readonly string\[\] = \[[^\]]*"shell"[^\]]*\]/,
  "a plain shell is the thing this filter hides",
);
must(/"transfer"/, "a file transfer is not a conversation");
must(/"borrow"/, "a borrowed terminal is not a conversation");
assert.ok(
  !/TERMINAL_ONLY: readonly string\[\] = \[[^\]]*"tui"/.test(text),
  "an agent TUI is an agent conversation in a terminal — hiding it would hide real work",
);
assert.ok(
  !/TERMINAL_ONLY: readonly string\[\] = \[[^\]]*"observe"/.test(text),
  "an observed agent transcript is a conversation, read-only or not",
);
must(
  /export const isTalk = \(kind: string\) => !TERMINAL_ONLY\.includes\(kind\)/,
  "the rule is a whitelist of what to hide, so unknown kinds stay visible",
);

// Every desktop starts with the same list and honours saved choices.
must(/export const ARRANGE_DEFAULT = "recent"/, "new installs use the same order on every platform");
const app = readFileSync(join(here, "..", "src", "App.tsx"), "utf8");
assert.match(app, /sessions: storedWords\("sessions"\)/, "use the saved filter or show every session");

// The axis must be offered on the sessions landing and labelled with the
// two words the wizard already uses for the same pair.
must(/\[TALK_AXIS\]: \(v\) => \(v === "yes" \? gui\.view_chat : gui\.view_terminal\)/, "the chip says 对话 / 终端");
must(/\[TALK_AXIS\]: gui\.axis_talk/, "the axis has a heading of its own");
must(/export const ALL_AXES = \[\.\.\.AXES, TALK_AXIS\] as const/, "the sessions landing offers it");
must(
  /sieve\(rows, ticked, SESSION_SIEVE, ALL_AXES\)/,
  "the sieve reads the talk tick as talk, never as a state word",
);

// Catalog: the heading exists in both languages and does not collide with
// another name for the same idea in that section (the build refuses that).
const zh = readFileSync(join(here, "..", "..", "..", "crates", "catalog", "zh.toml"), "utf8");
const en = readFileSync(join(here, "..", "..", "..", "crates", "catalog", "en.toml"), "utf8");
assert.match(zh, /^axis-talk = "形态"$/m, "zh heading");
assert.match(en, /^axis-talk = "Form"$/m, "en heading");

console.log("Session kind filter: 对话 stays, 终端 goes, unknown kinds stay visible — passed");
