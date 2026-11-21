// 「继续」 — which rows offer it, and what the read-only row says.
//
// Two facts pinned here, both from the owner's 2026-11-18 report
// (「当前只有只读对话，为什么？」):
//
//   1. A recorded conversation can be continued in its own tool, and the
//      gate must name **every vendor whose resume line exists** — it named
//      codex alone, so a DSH record offered nothing.
//   2. A read-only record must say who holds it and where the way forward
//      is, in the detail view itself, not only in a menu.
//
// Run: node apps/gui/scripts/check-continue-actions.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const read = (...parts) => readFileSync(join(here, "..", ...parts), "utf8");
const repo = (...parts) => readFileSync(join(here, "..", "..", "..", ...parts), "utf8");

const list = read("src", "views", "SessionsList.tsx");
const detail = read("src", "views", "DetailPane.tsx");
const driver = repo("crates", "node", "src", "agent_driver.rs");
const zh = repo("crates", "catalog", "zh.toml");
const en = repo("crates", "catalog", "en.toml");

// ── 1. The gate names every vendor khor can actually resume ──────────
const resume = driver.match(/pub fn resume_argv[\s\S]*?\n}/);
assert.ok(resume, "resume_argv exists in agent_driver");
const vendors = [...resume[0].matchAll(/"([a-z]+)" => format!/g)].map((m) => m[1]);
assert.deepEqual(
  vendors.sort(),
  ["codex", "dsh"],
  "the node knows exactly these resume lines; the gate must be read against the same set",
);
const gate = list.match(/const RESUMABLE_VENDORS = \[([^\]]+)\] as const;/);
assert.ok(gate, "the GUI keeps its own list beside the gate");
const named = [...gate[1].matchAll(/"([a-z]+)"/g)].map((m) => m[1]);
assert.deepEqual(
  named.sort(),
  vendors.sort(),
  "the GUI's list and the node's resume table cannot drift",
);
assert.match(
  list,
  /r\.kind === "observe" && !r\.source && isResumableVendor\(r\.category\)/,
  "the action is offered for any resumable vendor's record",
);
assert.ok(
  !/r\.category === "codex"/.test(list),
  "no vendor is named in the gate any more — that was the bug",
);

// ── 2. A read-only record says who holds it, where it is read ─────────
assert.match(
  detail,
  /row\.kind === "observe" \? \(/,
  "the note is drawn for observed rows",
);
assert.match(detail, /gui\.session_observation_note/, "and it is the catalog's sentence");
assert.match(detail, /data-readonly-note/, "with a hook a check can find");
assert.match(
  detail,
  /if \(!content\.capture\) return <>\{readonlyNote\}\{history\}<\/>;/,
  "the note rides above the transcript too, not only above the failure",
);
assert.match(zh, /^session_observation_note = "这段记录由厂商自己的工具持有/m, "zh text");
assert.match(en, /^session_observation_note = "This record is held by the vendor/m, "en text");

// ── 3. The list dispatches; it does not merely report ────────────────
assert.match(zh, /^empty-sessions = "还没有派发过任务"$/m, "empty means 'nothing dispatched yet'");
assert.match(zh, /^no-matches = "没有匹配项"$/m, "and a filter that hides rows says so instead");
assert.match(en, /^empty-sessions = "No work dispatched yet"$/m, "en empty state");
assert.match(en, /^no-matches = "No matches"$/m, "en filtered state");

console.log("Continue actions: every resumable vendor, and the read-only note — passed");
